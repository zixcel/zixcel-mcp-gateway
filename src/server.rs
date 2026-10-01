//! MCP results are generated only from validated in-memory public metadata.

use crate::config::Config;
use crate::config::PublicResource;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;

type RpcResult = Result<Value, (i32, &'static str)>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolCall {
    name: String,
    #[serde(default)]
    arguments: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResourceArgument {
    resource: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResourceRead {
    uri: String,
}

pub fn initialize(config: &Config, params: &Value) -> RpcResult {
    require_object(params)?;
    Ok(json!({
        "protocolVersion":"2025-06-18",
        "capabilities":{"tools":{},"resources":{}},
        "serverInfo":{
            "name":config.server.id.as_str(),
            "title":config.server.title,
            "version":env!("CARGO_PKG_VERSION")
        },
        "instructions":config.server.instructions
    }))
}

pub fn tools_list(params: &Value) -> RpcResult {
    require_empty(params)?;
    Ok(json!({"tools":[
        {
            "name":"zixcel_public_catalog",
            "description":"Return the configured read-only public resource catalog.",
            "inputSchema":{"type":"object","additionalProperties":false}
        },
        {
            "name":"zixcel_public_resource",
            "description":"Return one configured public resource.",
            "inputSchema":{
                "type":"object",
                "properties":{"resource":{"type":"string","pattern":"^[a-z][a-z0-9-]{0,62}$"}},
                "required":["resource"],
                "additionalProperties":false
            }
        }
    ]}))
}

pub fn tools_call(config: &Config, params: &Value) -> RpcResult {
    let call: ToolCall = decode(params)?;
    let value = match call.name.as_str() {
        "zixcel_public_catalog" if empty_arguments(&call.arguments) => catalog(config),
        "zixcel_public_resource" => {
            let argument: ResourceArgument = decode(&call.arguments)?;
            let resource = find_resource(config, &argument.resource)?;
            serde_json::to_value(resource).map_err(internal)?
        }
        "zixcel_public_catalog" => return Err((-32602, "Invalid params")),
        _ => return Err((-32602, "Unknown or prohibited tool")),
    };
    let text = serde_json::to_string(&value).map_err(internal)?;
    Ok(json!({"content":[{"type":"text","text":text}],"isError":false}))
}

pub fn resources_list(config: &Config, params: &Value) -> RpcResult {
    require_empty(params)?;
    let resources = config.resources.iter().map(|resource| {
        json!({
            "uri":format!("zixcel://public/{}", resource.id.as_str()),
            "name":resource.title,
            "description":resource.summary,
            "mimeType":"application/json"
        })
    });
    Ok(json!({"resources":resources.collect::<Vec<_>>()}))
}

pub fn resources_read(config: &Config, params: &Value) -> RpcResult {
    let read: ResourceRead = decode(params)?;
    let Some(slug) = read.uri.strip_prefix("zixcel://public/") else {
        return Err((-32602, "Only approved zixcel public resources are readable"));
    };
    let resource = find_resource(config, slug)?;
    let text = serde_json::to_string(resource).map_err(internal)?;
    Ok(json!({"contents":[{"uri":read.uri,"mimeType":"application/json","text":text}]}))
}

fn catalog(config: &Config) -> Value {
    json!({"schemaVersion":config.schema_version,"resources":config.resources})
}

fn find_resource<'a>(
    config: &'a Config,
    slug: &str,
) -> Result<&'a PublicResource, (i32, &'static str)> {
    config
        .resources
        .iter()
        .find(|resource| resource.id.as_str() == slug)
        .ok_or((-32602, "Unknown or prohibited resource"))
}

fn decode<T: for<'de> Deserialize<'de>>(value: &Value) -> Result<T, (i32, &'static str)> {
    serde_json::from_value(value.clone()).map_err(|_| (-32602, "Invalid params"))
}

fn require_object(value: &Value) -> RpcResult {
    value
        .as_object()
        .map(|_| Value::Null)
        .ok_or((-32602, "Invalid params"))
}

fn require_empty(value: &Value) -> RpcResult {
    if value.is_null() || value.as_object().is_some_and(serde_json::Map::is_empty) {
        Ok(Value::Null)
    } else {
        Err((-32602, "Invalid params"))
    }
}

fn empty_arguments(value: &Value) -> bool {
    value.is_null() || value.as_object().is_some_and(serde_json::Map::is_empty)
}

fn internal(_: serde_json::Error) -> (i32, &'static str) {
    (-32603, "Internal error")
}
