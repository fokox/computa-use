use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

fn main() {
    let req = r#"{"jsonrpc": "2.0", "method": "notifications/initialized"}"#;
    let r: Result<JsonRpcRequest, _> = serde_json::from_str(req);
    println!("{:?}", r);
}
