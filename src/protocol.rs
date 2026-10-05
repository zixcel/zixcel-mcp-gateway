//! JSON-RPC request handling with a fixed MCP method surface.

use crate::config::Config;
use crate::server;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::json;
use serde_json::value::RawValue;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    jsonrpc: String,
    #[serde(default)]
    id: RawField,
    method: String,
    #[serde(default)]
    params: RawField,
}

#[derive(Debug, Default)]
pub(crate) struct RawField(pub(crate) Option<Box<RawValue>>);

impl<'de> Deserialize<'de> for RawField {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Box::<RawValue>::deserialize(deserializer).map(|id| Self(Some(id)))
    }
}

#[derive(Serialize)]
struct Reply<'a> {
    jsonrpc: &'static str,
    id: &'a RawValue,
    #[serde(flatten)]
    payload: Value,
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
        let Ok(root) = serde_json::from_str::<Box<RawValue>>(line) else {
            return Some(error(&Value::Null, -32700, "Parse error").to_string());
        };
        if !root.get().starts_with('{') {
            return Some(error(&Value::Null, -32600, "Invalid Request").to_string());
        }
        let Ok(request) = serde_json::from_str::<Request>(line) else {
            return Some(error(&Value::Null, -32600, "Invalid Request").to_string());
        };
        if request.jsonrpc != "2.0" || request.id.0.as_deref().is_some_and(|id| !valid_mcp_id(id)) {
            return Some(error(&Value::Null, -32600, "Invalid Request").to_string());
        }
        let params = request.params.0.as_deref();
        let validation = params.map_or(Ok(()), |raw| {
            if raw.get().starts_with('{') {
                validate_params_depth(raw, 0)
                    .and_then(|()| validate_common_params(raw, request.id.0.is_some()))
            } else {
                Err(())
            }
        });
        // Notifications never dispatch or respond, including invalid params.
        let id = request.id.0?;
        let result = validation
            .map_err(|()| (-32602, "Invalid params"))
            .and_then(|()| self.dispatch(&request.method, params));
        let payload = match result {
            Ok(value) => json!({"result":value}),
            Err((code, message)) => json!({"error":{"code":code,"message":message}}),
        };
        serde_json::to_string(&Reply {
            jsonrpc: "2.0",
            id: &id,
            payload,
        })
        .ok()
    }

    fn dispatch(
        &self,
        method: &str,
        params: Option<&RawValue>,
    ) -> Result<Value, (i32, &'static str)> {
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

#[derive(Deserialize)]
struct CommonParams {
    #[serde(default, rename = "_meta")]
    metadata: RawField,
}

#[derive(Deserialize)]
struct RequestMetadata {
    #[serde(default, rename = "progressToken")]
    progress_token: RawField,
}

fn validate_common_params(raw: &RawValue, is_request: bool) -> Result<(), ()> {
    let common: CommonParams = serde_json::from_str(raw.get()).map_err(|_| ())?;
    let Some(metadata) = common.metadata.0.as_deref() else {
        return Ok(());
    };
    if !metadata.get().starts_with('{') {
        return Err(());
    }
    // Notification metadata has unknown-valued keys. Only requests reserve the
    // string-or-number progressToken type; this does not implement progress.
    if is_request {
        let metadata: RequestMetadata = serde_json::from_str(metadata.get()).map_err(|_| ())?;
        if metadata.progress_token.0.as_deref().is_some_and(|token| {
            !matches!(
                token.get().as_bytes().first(),
                Some(b'"' | b'-' | b'0'..=b'9')
            )
        }) {
            return Err(());
        }
    }
    Ok(())
}

#[cfg(test)]
mod metadata_contract_tests {
    use super::validate_common_params;
    use serde_json::value::RawValue;

    #[test]
    fn reserved_request_token_does_not_restrict_notification_metadata() {
        for token in ["null", "true", "false", "[]", "{}"] {
            let source = format!(r#"{{"_meta":{{"progressToken":{token}}}}}"#);
            let raw: Box<RawValue> = serde_json::from_str(&source).expect("valid JSON");
            assert!(validate_common_params(&raw, true).is_err());
            assert!(validate_common_params(&raw, false).is_ok());
        }
    }
}

// RawValue validates JSON and preserves the original type and numeric spelling.
// Only the numeric ID is parsed as decimal; params remain ordinary JSON values.
fn valid_mcp_id(id: &RawValue) -> bool {
    let source = id.get();
    match source.as_bytes().first() {
        Some(b'"') => true,
        Some(b'-' | b'0'..=b'9') => source
            .parse::<bigdecimal::BigDecimal>()
            .is_ok_and(|number| {
                number.fractional_digit_count() <= 0
                    || number.normalized().fractional_digit_count() <= 0
            }),
        _ => false,
    }
}

// Inspect every occurrence before any map aggregation. serde_json parses JSON.
struct DepthVisitor {
    depth: usize,
}

impl<'de> serde::de::Visitor<'de> for DepthVisitor {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON container within the nesting budget")
    }

    fn visit_map<M: serde::de::MapAccess<'de>>(self, mut map: M) -> Result<(), M::Error> {
        while let Some((_, value)) = map.next_entry::<serde::de::IgnoredAny, Box<RawValue>>()? {
            validate_params_depth(&value, self.depth + 1)
                .map_err(|()| serde::de::Error::custom("params exceed nesting budget"))?;
        }
        Ok(())
    }

    fn visit_seq<S: serde::de::SeqAccess<'de>>(self, mut sequence: S) -> Result<(), S::Error> {
        while let Some(value) = sequence.next_element::<Box<RawValue>>()? {
            validate_params_depth(&value, self.depth + 1)
                .map_err(|()| serde::de::Error::custom("params exceed nesting budget"))?;
        }
        Ok(())
    }
}

fn validate_params_depth(raw: &RawValue, depth: usize) -> Result<(), ()> {
    if depth >= 128 {
        return Err(());
    }
    let mut decoder = serde_json::Deserializer::from_str(raw.get());
    let visitor = DepthVisitor { depth };
    match raw.get().as_bytes().first() {
        Some(b'{') => serde::Deserializer::deserialize_map(&mut decoder, visitor),
        Some(b'[') => serde::Deserializer::deserialize_seq(&mut decoder, visitor),
        _ => return Ok(()),
    }
    .map_err(|_| ())
}

fn error(id: &Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}

#[cfg(test)]
mod raw_parameter_tests {
    use super::validate_params_depth;
    use serde_json::value::RawValue;

    #[test]
    fn nesting_budget_matches_ordinary_serde_json() {
        for levels in [126, 127, 128, 129] {
            let source = format!("{}0{}", "[".repeat(levels), "]".repeat(levels));
            let expected = serde_json::from_str::<serde_json::Value>(&source).is_ok();
            let raw = RawValue::from_string(source).expect("valid JSON grammar");
            assert_eq!(validate_params_depth(&raw, 0).is_ok(), expected, "{levels}");
        }
    }
}
