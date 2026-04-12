use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Internal interface types (shared between DAP adapter and debugger)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StackFrame {
    pub id: u32,
    pub name: String,
    pub line: u32,
    pub column: u32,
    pub source: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub r#type: Option<String>,
}

// ---------------------------------------------------------------------------
// Base Protocol
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProtocolMessage {
    #[serde(rename = "request")]
    Request {
        seq: i64,
        command: String,
        #[serde(default)]
        arguments: Option<serde_json::Value>,
    },
    #[serde(rename = "response")]
    Response {
        seq: i64,
        request_seq: i64,
        success: bool,
        command: String,
        #[serde(default)]
        message: Option<String>,
        #[serde(default)]
        body: Option<serde_json::Value>,
    },
    #[serde(rename = "event")]
    Event {
        seq: i64,
        event: String,
        #[serde(default)]
        body: Option<serde_json::Value>,
    },
}
