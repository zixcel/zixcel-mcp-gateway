use serde_json::{Value, json};
use zixcel_mcp_gateway::{Config, Gateway};

fn gateway() -> Gateway {
    Gateway::new(
        Config::from_toml(
            r#"schema_version = 1
[server]
id = "synthetic-mcp"
title = "Synthetic metadata"
instructions = "Read-only synthetic metadata."
[[resource]]
id = "example"
title = "Example"
summary = "Synthetic resource for protocol boundary checks."
capabilities = ["read-product-overview"]
"#,
        )
        .expect("synthetic config"),
    )
}

fn reply(method: &str, params: Option<Value>) -> Value {
    let mut request = json!({"jsonrpc":"2.0","id":"contract","method":method});
    if let Some(params) = params {
        request["params"] = params;
    }
    serde_json::from_str(
        &gateway()
            .handle_line(&request.to_string())
            .expect("request reply"),
    )
    .expect("JSON reply")
}

#[test]
fn all_methods_reject_nonobject_params_in_common_boundary() {
    for method in [
        "initialize",
        "tools/list",
        "tools/call",
        "resources/list",
        "resources/read",
        "synthetic/unknown",
    ] {
        for params in [
            Value::Null,
            json!([]),
            json!([{}]),
            json!(true),
            json!(false),
            json!(0),
            json!(1.5),
            json!(""),
            json!("{}"),
        ] {
            let response = reply(method, Some(params.clone()));
            assert_eq!(response["error"]["code"], -32602, "{method}: {params}");
            assert_eq!(response["id"], "contract");
        }
    }
}

#[test]
fn omitted_and_legal_objects_keep_method_contracts() {
    for method in ["tools/list", "resources/list"] {
        for params in [None, Some(json!({}))] {
            assert!(reply(method, params).get("result").is_some(), "{method}");
        }
    }
    for (method, params) in [
        (
            "initialize",
            json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"synthetic","version":"1"}}),
        ),
        ("tools/call", json!({"name":"zixcel_public_catalog"})),
        (
            "tools/call",
            json!({"name":"zixcel_public_catalog","arguments":{}}),
        ),
        (
            "tools/call",
            json!({"name":"zixcel_public_resource","arguments":{"resource":"example"}}),
        ),
        ("resources/read", json!({"uri":"zixcel://public/example"})),
    ] {
        assert!(
            reply(method, Some(params)).get("result").is_some(),
            "{method}"
        );
    }
    for method in ["initialize", "tools/call", "resources/read"] {
        assert_eq!(reply(method, None)["error"]["code"], -32602, "{method}");
    }
    assert_eq!(reply("synthetic/unknown", None)["error"]["code"], -32601);
    assert_eq!(
        reply("synthetic/unknown", Some(json!({})))["error"]["code"],
        -32601
    );
}

#[test]
fn notifications_never_reply_for_any_params_shape_or_method() {
    for method in [
        "notifications/initialized",
        "initialize",
        "tools/list",
        "tools/call",
        "resources/list",
        "resources/read",
        "synthetic/unknown",
    ] {
        for params in [
            None,
            Some(json!({})),
            Some(Value::Null),
            Some(json!([])),
            Some(json!(true)),
            Some(json!(1)),
            Some(json!("text")),
        ] {
            let mut message = json!({"jsonrpc":"2.0","method":method});
            if let Some(params) = params {
                message["params"] = params;
            }
            assert!(
                gateway().handle_line(&message.to_string()).is_none(),
                "{message}"
            );
        }
    }
}

#[test]
fn common_guard_preserves_nested_json_and_duplicate_rejection() {
    let nested = json!({"protocolVersion":"2025-06-18","capabilities":{"experimental":{"synthetic":{"array":[null,true,1,"text",{}]}}},"clientInfo":{"name":"synthetic","version":"1"}});
    assert!(reply("initialize", Some(nested)).get("result").is_some());
    let duplicate = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{},"params":null}"#;
    let response: Value =
        serde_json::from_str(&gateway().handle_line(duplicate).expect("reply")).expect("JSON");
    assert_eq!(response["error"]["code"], -32600);
}
