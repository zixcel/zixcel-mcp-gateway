//! MCP results are generated only from validated in-memory public metadata.

use crate::config::Config;
use crate::config::PublicResource;
use crate::protocol::RawField;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use serde_json::value::RawValue;

type RpcResult = Result<Value, (i32, &'static str)>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolCall {
    name: String,
    #[serde(default)]
    arguments: RawField,
    #[serde(default)]
    _meta: RawField,
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
    #[serde(default)]
    _meta: RawField,
}

pub fn initialize(config: &Config, params: Option<&RawValue>) -> RpcResult {
    validate_initialize(params)?;
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

pub fn tools_list(params: Option<&RawValue>) -> RpcResult {
    require_empty::<EmptyParams>(params)?;
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

pub fn tools_call(config: &Config, params: Option<&RawValue>) -> RpcResult {
    let call: ToolCall = decode(params)?;
    let value = match call.name.as_str() {
        "zixcel_public_catalog" => {
            require_empty::<EmptyObject>(call.arguments.0.as_deref())?;
            catalog(config)
        }
        "zixcel_public_resource" => {
            let argument: ResourceArgument = decode(call.arguments.0.as_deref())?;
            let resource = find_resource(config, &argument.resource)?;
            serde_json::to_value(resource).map_err(internal)?
        }
        _ => return Err((-32602, "Unknown or prohibited tool")),
    };
    let text = serde_json::to_string(&value).map_err(internal)?;
    Ok(json!({"content":[{"type":"text","text":text}],"isError":false}))
}

pub fn resources_list(config: &Config, params: Option<&RawValue>) -> RpcResult {
    require_empty::<EmptyParams>(params)?;
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

pub fn resources_read(config: &Config, params: Option<&RawValue>) -> RpcResult {
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

fn decode<T: for<'de> Deserialize<'de>>(raw: Option<&RawValue>) -> Result<T, (i32, &'static str)> {
    let source = raw
        .filter(|value| value.get().starts_with('{'))
        .ok_or((-32602, "Invalid params"))?;
    serde_json::from_str(source.get()).map_err(|_| (-32602, "Invalid params"))
}

#[derive(Deserialize)]
struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    _protocol_version: String,
    capabilities: Box<RawValue>,
    #[serde(rename = "clientInfo")]
    client_info: Box<RawValue>,
}

#[derive(Deserialize)]
struct ClientInfo {
    #[serde(rename = "name")]
    _name: String,
    #[serde(rename = "version")]
    _version: String,
    #[serde(default)]
    title: RawField,
}

#[derive(Deserialize)]
struct ClientCapabilities {
    #[serde(default)]
    experimental: RawField,
    #[serde(default)]
    roots: RawField,
    #[serde(default)]
    sampling: RawField,
    #[serde(default)]
    elicitation: RawField,
}

#[derive(Deserialize)]
struct RootsCapabilities {
    #[serde(default, rename = "listChanged")]
    list_changed: RawField,
}

fn validate_initialize(raw: Option<&RawValue>) -> RpcResult {
    let request: InitializeParams = decode(raw)?;
    let client: ClientInfo = decode(Some(&request.client_info))?;
    if let Some(title) = client.title.0.as_deref() {
        serde_json::from_str::<String>(title.get()).map_err(|_| (-32602, "Invalid params"))?;
    }
    let capabilities: ClientCapabilities = decode(Some(&request.capabilities))?;
    for capability in [
        &capabilities.experimental,
        &capabilities.roots,
        &capabilities.sampling,
        &capabilities.elicitation,
    ] {
        if capability
            .0
            .as_deref()
            .is_some_and(|value| !value.get().starts_with('{'))
        {
            return Err((-32602, "Invalid params"));
        }
    }
    if let Some(experimental) = capabilities.experimental.0.as_deref() {
        let values: std::collections::BTreeMap<String, Box<RawValue>> = decode(Some(experimental))?;
        if values.values().any(|value| !value.get().starts_with('{')) {
            return Err((-32602, "Invalid params"));
        }
    }
    if let Some(roots) = capabilities.roots.0.as_deref() {
        let roots: RootsCapabilities = decode(Some(roots))?;
        if let Some(changed) = roots.list_changed.0.as_deref() {
            serde_json::from_str::<bool>(changed.get()).map_err(|_| (-32602, "Invalid params"))?;
        }
    }
    Ok(Value::Null)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyObject {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyParams {
    #[serde(default)]
    _meta: RawField,
}

fn require_empty<T: for<'de> Deserialize<'de>>(raw: Option<&RawValue>) -> RpcResult {
    if raw.is_none() {
        return Ok(Value::Null);
    }
    let _: T = decode(raw)?;
    Ok(Value::Null)
}

fn internal(_: serde_json::Error) -> (i32, &'static str) {
    (-32603, "Internal error")
}
