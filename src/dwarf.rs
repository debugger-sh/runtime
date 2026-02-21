use crate::types::LocationInfo;
use wasmer_wasix::virtual_fs::{AsyncReadExt, FileSystem, mem_fs};

pub use wasm_instrument::{parse_dwarf_info as parse_dwarf_info_inner, instrument_binary as instrument_binary_inner};

/// Get the WASM bytes from the filesystem.
pub async fn get_wasm_bytes(
    fs: &mem_fs::FileSystem,
    path: &str,
) -> Result<Vec<u8>, std::io::Error> {
    let mut file = fs
        .new_open_options()
        .read(true)
        .open(path)
        .expect(&format!("{} exists", path));

    let mut wasm_bytes = Vec::new();
    file.read_to_end(&mut wasm_bytes)
        .await
        .expect("Read main.wasm");

    Ok(wasm_bytes)
}

/// Parse DWARF debug info, logging errors to the browser console.
pub fn parse_dwarf_info(wasm_bytes: &[u8]) -> (Vec<LocationInfo>, Vec<String>) {
    match parse_dwarf_info_inner(wasm_bytes) {
        Ok((locs, files)) => {
            let locs = locs.into_iter().map(LocationInfo::from).collect();
            (locs, files)
        }
        Err(e) => {
            web_sys::console::error_1(&format!("DWARF parsing error: {}", e).into());
            (vec![], vec![])
        }
    }
}

/// Instrument a WASM binary, logging progress to the browser console.
pub fn instrument_binary(wasm_bytes: &[u8], locations: &[LocationInfo]) -> Result<Vec<u8>, String> {
    web_sys::console::log_1(
        &format!(
            "instrument_binary: {} locations, input {} bytes",
            locations.len(),
            wasm_bytes.len()
        )
        .into(),
    );

    let lib_locs: Vec<wasm_instrument::LocationInfo> =
        locations.iter().map(|l| l.into()).collect();

    let result = instrument_binary_inner(wasm_bytes, &lib_locs)?;

    web_sys::console::log_1(
        &format!("instrument_binary: output {} bytes", result.len()).into(),
    );
    Ok(result)
}
