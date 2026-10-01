//! JSON-RPC request handling with a fixed MCP method surface.

use crate::config::Config;
use crate::server;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

pub struct Gateway {
    config: Config,
}

impl Gateway {
    #[must_use]
    pub const fn new(config: Config) -> Self {
        Self { config }
    }

    #[must_use]
    pub fn handle_line(&self, line: &str) -> Option<String> {
        let Ok(request) = serde_json::from_str::<Request>(line) else {
            return Some(error(&Value::Null, -32700, "Parse error").to_string());
        };
        let id = request.id?;
        if request.jsonrpc != "2.0" {
            return Some(error(&id, -32600, "Invalid Request").to_string());
        }
        let result = self.dispatch(&request.method, &request.params);
        Some(match result {
            Ok(value) => json!({"jsonrpc":"2.0","id":id,"result":value}).to_string(),
            Err((code, message)) => error(&id, code, message).to_string(),
        })
    }

    fn dispatch(&self, method: &str, params: &Value) -> Result<Value, (i32, &'static str)> {
        match method {
            "initialize" => server::initialize(&self.config, params),
            "tools/list" => server::tools_list(params),
            "tools/call" => server::tools_call(&self.config, params),
            "resources/list" => server::resources_list(&self.config, params),
            "resources/read" => server::resources_read(&self.config, params),
            _ => Err((-32601, "Method not found")),
        }
    }
}

fn error(id: &Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
