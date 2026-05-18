use console_error_panic_hook;
use futures::channel::oneshot;
use std::cell::RefCell;
use std::path::PathBuf;
use wasm_bindgen::prelude::*;
use wasmer_wasix::virtual_fs::{AsyncWriteExt, FileSystem, create_dir_all, mem_fs};
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

use crate::debug::instrument::{InstrumenterResult, instrument_wasm};
use crate::types::{FsNode, WorkerOut, WorkerPrepare};

mod debuggee;
mod execution;
mod io;
mod runtime;

use debuggee::Debuggee;
use execution::Execution;

// ╭──────────────────────────────────────────────────────────────────────────╮
// │ Helpers                                                                  │
// ╰──────────────────────────────────────────────────────────────────────────╯

async fn create_user_fs(node: FsNode) -> Result<mem_fs::FileSystem, std::io::Error> {
    let fs = mem_fs::FileSystem::default();
    create_user_fs_rec(&fs, &PathBuf::from("/"), &node).await?;
    Ok(fs)
}

async fn create_user_fs_rec(
    fs: &mem_fs::FileSystem,
    base_path: &PathBuf,
    node: &FsNode,
) -> Result<(), std::io::Error> {
    match node {
        FsNode::File(contents) => {
            let mut file = fs
                .new_open_options()
                .create(true)
                .write(true)
                .open(base_path)?;
            file.write_all(contents.as_bytes())
                .await
                .expect("Failed to write injected file");
            file.flush().await.expect("Flushed file")
        }
        FsNode::Dir(children) => {
            create_dir_all(fs, base_path)?;
            for (name, child_node) in children {
                let mut child_path = base_path.clone();
                child_path.push(name);
                Box::pin(create_user_fs_rec(fs, &child_path, child_node)).await?;
            }
        }
    }
    Ok(())
}

fn collect_sources(node: &FsNode, base_path: &PathBuf, sources: &mut Vec<String>) {
    match node {
        FsNode::File(_) => {
            if let Some(ext) = base_path.extension().and_then(|ext| ext.to_str()) {
                let is_source = matches!(
                    ext.to_ascii_lowercase().as_str(),
                    "c" | "cc" | "cp" | "cpp" | "cxx" | "c++"
                );
                if is_source {
                    sources.push(base_path.to_string_lossy().to_string());
                }
            }
        }
        FsNode::Dir(children) => {
            collect_dir_sources(children, base_path, sources);
        }
    }
}

fn collect_dir_sources(
    children: &std::collections::HashMap<String, FsNode>,
    base_path: &PathBuf,
    sources: &mut Vec<String>,
) {
    for (name, child_node) in children {
        let mut child_path = base_path.clone();
        child_path.push(name);
        collect_sources(child_node, &child_path, sources);
    }
}

// ╭──────────────────────────────────────────────────────────────────────────╮
// │ Worker                                                                   │
// ╰──────────────────────────────────────────────────────────────────────────╯

/// The worker's full lifecycle: compile and link during the prepare phase,
/// suspend on `run_rx` until the main thread sends `run`, then execute.
async fn lifecycle(prepare: WorkerPrepare, run_rx: oneshot::Receiver<()>) {
    let WorkerPrepare {
        fs,
        is_debug,
        stdin_buffer,
    } = prepare;

    let mut sources = Vec::new();
    collect_dir_sources(&fs, &PathBuf::from("/"), &mut sources);
    sources.sort();

    assert!(
        !sources.is_empty(),
        "No C/C++ source files found in provided filesystem"
    );

    let user_fs = create_user_fs(FsNode::Dir(fs))
        .await
        .expect("created user files filesystem");

    let exec = Execution::new();

    // Build clang args, conditional on is_debug
    let mut clang_args = vec![
        "-cc1",
        "-triple",
        "wasm32-wasip1",
        "-Werror",
        "-emit-obj",
        "-disable-free",
        "-isysroot",
        "/",
        "-internal-isystem",
        "/include/c++/v1",
        "-internal-isystem",
        "/include",
        "-internal-isystem",
        "/include/wasm32-wasip1",
        "-ferror-limit",
        "4",
        "-fcolor-diagnostics",
        "-x",
        "c++",
        "-std=c++23",
        "-o",
        "/main.o",
    ];

    if is_debug {
        clang_args.push("-O0");
        // because of the -cc1 flag
        clang_args.push("-debug-info-kind=standalone");
        clang_args.push("-dwarf-version=5");
    }

    for source in &sources {
        clang_args.push(source);
    }

    let exit = exec
        .step("clang")
        // from @yowasp
        .binary("https://fabioibanez.github.io/website/llvm.core.wasm")
        .sysroot("https://fabioibanez.github.io/website/llvm-resources.tar.gz")
        .fs(Box::new(user_fs))
        .args(&clang_args)
        .run()
        .await
        .expect("Compilation succeeded");

    if !exit.is_success() {
        return WorkerOut::Stop {
            exit_code: exit.raw(),
        }
        .send();
    }

    let exit = exec
        .step("wasm-ld")
        .binary("https://fabioibanez.github.io/website/llvm.core.wasm")
        .args(&[
            "--export-dynamic",
            "-z",
            "stack-size=1048576",
            "-L/lib/wasm32-wasip1",
            "/lib/wasm32-wasip1/crt1.o",
            "/main.o",
            "-lc++",
            "-lc++abi",
            "/lib/wasm32-unknown-wasip1/libclang_rt.builtins.a",
            "-lc",
            "-o",
            "/main.wasm",
        ])
        .run()
        .await
        .expect("Linking succeeded");

    if !exit.is_success() {
        return WorkerOut::Stop {
            exit_code: exit.raw(),
        }
        .send();
    }

    // In debug mode, instrument /main.wasm in place and build the Debuggee.
    let debuggee = if is_debug {
        let wasm = exec
            .read_bytes("/main.wasm")
            .await
            .expect("read /main.wasm");
        WorkerOut::Artifact {
            data: &wasm,
            name: "pre.wasm".into(),
        }
        .send();

        let InstrumenterResult {
            info,
            wasm: instrumented,
        } = instrument_wasm(&wasm).expect("instrument /main.wasm");

        WorkerOut::Artifact {
            data: &instrumented,
            name: "post.wasm".into(),
        }
        .send();

        exec.write_bytes("/main.wasm", &instrumented)
            .await
            .expect("write instrumented /main.wasm");

        Some(Debuggee::new(info))
    } else {
        None
    };

    // Wait for the main thread to give us the go-ahead.
    run_rx.await.expect("run signal");

    let mut main_step = exec
        .step("main")
        .binary("/main.wasm")
        .stdin_buffer(stdin_buffer);
    if let Some(d) = debuggee {
        main_step = main_step.debuggee(d);
    }

    let exit = main_step.run().await.expect("Running succeeded");

    WorkerOut::Stop {
        exit_code: exit.raw(),
    }
    .send();
}

#[wasm_bindgen]
pub fn main() {
    console_error_panic_hook::set_once();
    let scope = DedicatedWorkerGlobalScope::from(JsValue::from(js_sys::global()));

    // Holds the run-signal sender once `prepare` has spawned the lifecycle.
    let run_tx: RefCell<Option<oneshot::Sender<()>>> = RefCell::new(None);

    // Function that gets called when the worker receives a message.
    // We dispatch on the `type` discriminator manually because internally‑tagged
    // serde enums are incompatible with `serde_wasm_bindgen::preserve` (used
    // for the `SharedArrayBuffer` inside `WorkerPrepare`).
    let onmessage = Closure::wrap(Box::new(move |msg: MessageEvent| {
        let data = msg.data();
        let ty = js_sys::Reflect::get(&data, &JsValue::from_str("type"))
            .expect("WorkerIn.type")
            .as_string()
            .expect("WorkerIn.type is a string");
        match ty.as_str() {
            "prepare" => {
                let p: WorkerPrepare =
                    serde_wasm_bindgen::from_value(data).expect("deserialize WorkerPrepare");
                let (tx, rx) = oneshot::channel();
                *run_tx.borrow_mut() = Some(tx);
                wasm_bindgen_futures::spawn_local(lifecycle(p, rx));
            }
            "run" => {
                let tx = run_tx.borrow_mut().take().expect("Run before Prepare");
                // Receiver may already be gone if the prepare phase failed
                // and the lifecycle exited early; ignore the SendError.
                let _ = tx.send(());
            }
            other => panic!("unknown WorkerIn type: {other}"),
        }
    }) as Box<dyn Fn(MessageEvent)>);
    scope.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // The worker must send a message to indicate that it's ready to receive messages.
    WorkerOut::Ready.send();
}
