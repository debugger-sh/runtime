use serde::Serialize;
use tsify::Tsify;

#[derive(Debug, Tsify, Serialize)]
pub struct StackFrame {
    pub id: u32,
    pub name: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Tsify, Serialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub r#type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
    pub seq: i64,

    #[serde(flatten)]
    pub body: MessageBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageBody {
    #[serde(rename = "request")]
    Request {
        #[serde(flatten)]
        command: RequestCommand,
    },
    #[serde(rename = "response")]
    Response {
        request_seq: i64,
        success: bool,
        command: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<ResponseBody>,
    },
    #[serde(rename = "event")]
    Event {
        #[serde(flatten)]
        body: EventBody,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", content = "arguments")]
pub enum RequestCommand {
    #[serde(rename = "initialize")]
    Initialize,
    #[serde(rename = "launch")]
    Launch,
    #[serde(rename = "configurationDone")]
    ConfigurationDone,
    #[serde(rename = "setBreakpoints")]
    SetBreakpoints(SetBreakpointsArguments),
    #[serde(rename = "threads")]
    Threads,
    #[serde(rename = "stackTrace")]
    StackTrace(StackTraceArguments),
    #[serde(rename = "scopes")]
    Scopes(ScopesArguments),
    #[serde(rename = "variables")]
    Variables(VariablesArguments),
    #[serde(rename = "continue")]
    Continue(ContinueArguments),
    #[serde(rename = "disconnect")]
    Disconnect,
}