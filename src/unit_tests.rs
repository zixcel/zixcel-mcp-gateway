use crate::Config;
use crate::Gateway;
use serde_json::Value;
use serde_json::json;

const VALID: &str = include_str!("../examples/local.toml");

#[test]
fn published_schema_is_valid_json() {
    serde_json::from_str::<Value>(include_str!("../schema/public-catalog.schema.json"))
        .expect("valid JSON schema document");
}

#[test]
fn closed_config_accepts_generic_public_resources() {
    let config = Config::from_toml(VALID).expect("valid public config");
    assert_eq!(config.resources.len(), 1);
    assert_eq!(config.resources[0].id.as_str(), "example-service");
}

#[test]
fn closed_config_rejects_duplicate_and_invalid_ids() {
    let resource = VALID
        .split("[[resource]]")
        .nth(1)
        .expect("resource fixture");
    let duplicate = format!("{VALID}\n[[resource]]{resource}");
    let error = Config::from_toml(&duplicate).expect_err("duplicate id");
    assert!(error.contains("unique public resources"));
    let invalid = VALID.replace("example-service", "Example_Service");
    assert!(Config::from_toml(&invalid).is_err());
}

#[test]
fn closed_config_rejects_unknown_fields_and_secret_markers() {
    let unknown = format!("{VALID}\nendpoint = \"local\"\n");
    assert!(Config::from_toml(&unknown).is_err());
    let secret = VALID.replace(
        "schema_version = 1",
        "schema_version = 1\npassword = \"do-not-store-this\"",
    );
    let error = Config::from_toml(&secret).expect_err("secret-like config must fail");
    assert!(error.contains("prohibited"));
    assert!(!error.contains("do-not-store-this"));
}

#[test]
fn notifications_do_not_produce_protocol_output() {
    let gateway = gateway();
    assert_eq!(
        gateway.handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#),
        None
    );
}

#[test]
fn catalog_tool_returns_only_configured_resources() {
    let gateway = gateway();
    let request = json!({
        "jsonrpc":"2.0",
        "id":7,
        "method":"tools/call",
        "params":{"name":"zixcel_public_catalog","arguments":{}}
    });
    let response = response(&gateway, &request);
    assert_eq!(response["id"], 7);
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .expect("text result");
    assert!(text.contains("\"example-service\""));
    assert!(!text.contains("customer-product"));
}

#[test]
fn file_resource_and_extra_tool_arguments_are_rejected() {
    let gateway = gateway();
    for request in [
        json!({
            "jsonrpc":"2.0","id":1,"method":"resources/read",
            "params":{"uri":"file:///etc/passwd"}
        }),
        json!({
            "jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"zixcel_public_resource","arguments":{
                "resource":"example-service","path":"/etc/passwd"
            }}
        }),
    ] {
        let value = response(&gateway, &request);
        assert_eq!(value["error"]["code"], -32602);
    }
}

fn gateway() -> Gateway {
    Gateway::new(Config::from_toml(VALID).expect("valid config"))
}

fn response(gateway: &Gateway, request: &Value) -> Value {
    let line = gateway
        .handle_line(&request.to_string())
        .expect("request response");
    serde_json::from_str(&line).expect("JSON response")
}
