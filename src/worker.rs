use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

#[derive(Tsify, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkerIn {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "stdin")]
    Stdin { data: String },
}

#[derive(Tsify, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkerOut {
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "stdout")]
    Stdout { data: String },
}

#[wasm_bindgen]
pub fn main() {
    web_sys::console::log_1(&"worker starting".into());

    let scope = DedicatedWorkerGlobalScope::from(JsValue::from(js_sys::global()));

    // Function that gets called when the worker receives a message
    let onmessage = Closure::wrap(Box::new(move |msg: MessageEvent| {
        web_sys::console::log_1(&"got message".into());
        let message: WorkerIn = serde_wasm_bindgen::from_value(msg.data()).expect("");

        match message {
            WorkerIn::Start => web_sys::console::log_1(&"Started!".into()),
            WorkerIn::Stdin { data } => web_sys::console::log_1(&format!("Stdin: {}", data).into()),
        }
    }) as Box<dyn Fn(MessageEvent)>);
    scope.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // The worker must send a message to indicate that it's ready to receive messages.
    scope
        .post_message(
            &serde_wasm_bindgen::to_value(&WorkerOut::Ready)
                .expect("serialization worked")
                .into(),
        )
        .expect("posting ready message succeeds!");
}

// #[wasm_bindgen]
// pub fn main() {
//     web_sys::console::log_1(&"hello world".into());
// }
