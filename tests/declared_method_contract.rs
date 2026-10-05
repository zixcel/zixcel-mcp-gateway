use serde_json::{Value, json};
use zixcel_mcp_gateway::{Config, Gateway};

fn gateway() -> Gateway {
    Gateway::new(
        Config::from_toml(include_str!("../examples/local.toml")).expect("synthetic config"),
    )
}

fn reply(method: &str, params: Value) -> Value {
    let mut input = json!({"jsonrpc":"2.0","id":"contract","method":method});
    input["params"] = params;
    serde_json::from_str(&gateway().handle_line(&input.to_string()).expect("reply")).expect("JSON")
}

fn initialize() -> Value {
    json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"synthetic","version":"1"}})
}

fn legal_params(method: &str) -> Value {
    match method {
        "initialize" => initialize(),
        "tools/list" | "resources/list" => json!({}),
        "tools/call" => json!({"name":"zixcel_public_catalog","arguments":{}}),
        "resources/read" => json!({"uri":"zixcel://public/example-service"}),
        _ => panic!("synthetic method"),
    }
}

#[test]
fn legal_metadata_is_opaque_on_every_implemented_method() {
    let metadata = json!({"example.test/value":[null,true,1,"text",{"nested":{}}],"progressToken":"synthetic"});
    for (method, mut params) in [
        ("initialize", initialize()),
        ("tools/list", json!({})),
        (
            "tools/call",
            json!({"name":"zixcel_public_catalog","arguments":{}}),
        ),
        (
            "tools/call",
            json!({"name":"zixcel_public_resource","arguments":{"resource":"example-service"}}),
        ),
        ("resources/list", json!({})),
        (
            "resources/read",
            json!({"uri":"zixcel://public/example-service"}),
        ),
    ] {
        params["_meta"] = metadata.clone();
        let output = reply(method, params);
        assert!(output.get("result").is_some(), "{method}: {output}");
        assert!(!output.to_string().contains("example.test/value"));
    }
}

#[test]
fn metadata_must_be_an_object_without_reinterpreting_internal_tags() {
    for method in [
        "initialize",
        "tools/list",
        "tools/call",
        "resources/list",
        "resources/read",
    ] {
        for metadata in [Value::Null, json!([]), json!(true), json!(1), json!("{}")] {
            let mut params = legal_params(method);
            params["_meta"] = metadata;
            let output = reply(method, params);
            assert_eq!(output["error"]["code"], -32602, "{method}: {output}");
            assert_eq!(output["id"], "contract");
        }
    }
    for tag in [
        "$serde_json::private::Number",
        "$serde_json::private::RawValue",
    ] {
        let mut metadata = serde_json::Map::new();
        metadata.insert(tag.into(), json!("synthetic"));
        assert!(
            reply("tools/list", json!({"_meta":metadata}))
                .get("result")
                .is_some()
        );
    }
    let duplicate =
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{"_meta":{},"\u005fmeta":{}}}"#;
    let output: Value =
        serde_json::from_str(&gateway().handle_line(duplicate).expect("reply")).expect("JSON");
    assert_eq!(output["error"]["code"], -32602);
}

#[test]
fn progress_tokens_accept_strings_and_all_json_numbers_not_only_integer_ids() {
    for method in [
        "initialize",
        "tools/list",
        "tools/call",
        "resources/list",
        "resources/read",
    ] {
        for token in [
            r#""""#,
            r#""synthetic-token""#,
            r#""1.5""#,
            r#""{\"$serde_json::private::Number\":\"1.5\"}""#,
            "0",
            "-0",
            "1.5",
            "-0.125",
            "4.0",
            "1e3",
            "1e-400",
            "-1e-1000000000",
            "18446744073709551616.1",
            "1.0000000000000000001",
            "1e1000000000",
        ] {
            let mut params = legal_params(method);
            params["_meta"] = json!({
                "progressToken":"PROGRESS_TOKEN_SENTINEL",
                "example.test/null":null,
                "example.test/bool":true,
                "example.test/array":[null,true,1,"text",{}],
                "example.test/object":{"$serde_json::private::Number":"opaque"},
            });
            let params = params
                .to_string()
                .replace(r#""PROGRESS_TOKEN_SENTINEL""#, token);
            let input = format!(
                r#"{{"jsonrpc":"2.0","id":"progress","method":"{method}","params":{params}}}"#
            );
            let output: Value =
                serde_json::from_str(&gateway().handle_line(&input).expect("reply")).expect("JSON");
            assert!(
                output.get("result").is_some(),
                "{method}, {token}: {output}"
            );
            assert_eq!(output["id"], "progress");
        }
    }
}

#[test]
fn progress_tokens_reject_null_containers_booleans_and_private_tag_objects() {
    for method in [
        "initialize",
        "tools/list",
        "tools/call",
        "resources/list",
        "resources/read",
    ] {
        for token in [
            Value::Null,
            json!(true),
            json!(false),
            json!([]),
            json!({}),
            json!({"$serde_json::private::Number":"1.5"}),
            json!({"$serde_json::private::RawValue":"1.5"}),
        ] {
            let mut params = legal_params(method);
            params["_meta"] = json!({"progressToken":token});
            let output = reply(method, params);
            assert_eq!(output["error"]["code"], -32602, "{method}: {output}");
            assert_eq!(output["id"], "contract");
        }
    }
    for metadata in [
        r#"{"progressToken":1,"progressToken":2}"#,
        r#"{"progressToken":null,"progressToken":1}"#,
        r#"{"progressToken":1,"progressToken":null}"#,
        r#"{"progressToken":1,"progress\u0054oken":2}"#,
    ] {
        let input = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{{"_meta":{metadata}}}}}"#
        );
        let output: Value =
            serde_json::from_str(&gateway().handle_line(&input).expect("reply")).expect("JSON");
        assert_eq!(output["error"]["code"], -32602);
    }
    let escaped = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{"_meta":{"progress\u0054oken":null}}}"#;
    let output: Value =
        serde_json::from_str(&gateway().handle_line(escaped).expect("reply")).expect("JSON");
    assert_eq!(output["error"]["code"], -32602);
}

#[test]
fn notifications_keep_progress_token_values_opaque_without_progress_feature() {
    for token in [
        Value::Null,
        json!(true),
        json!(false),
        json!([]),
        json!({}),
        json!(1.5),
        json!("text"),
    ] {
        let input = json!({"jsonrpc":"2.0","method":"notifications/initialized","params":{"_meta":{"progressToken":token}}});
        assert!(gateway().handle_line(&input.to_string()).is_none());
    }
}

#[test]
fn nested_arguments_keep_omission_separate_from_null_and_strict_schema() {
    assert!(
        reply("tools/call", json!({"name":"zixcel_public_catalog"}))
            .get("result")
            .is_some()
    );
    assert!(
        reply(
            "tools/call",
            json!({"name":"zixcel_public_catalog","arguments":{}})
        )
        .get("result")
        .is_some()
    );
    for name in ["zixcel_public_catalog", "zixcel_public_resource"] {
        for arguments in [
            Value::Null,
            json!([]),
            json!(true),
            json!(1),
            json!("{}"),
            json!({"_meta":{}}),
        ] {
            assert_eq!(
                reply("tools/call", json!({"name":name,"arguments":arguments}))["error"]["code"],
                -32602
            );
        }
    }
    assert_eq!(
        reply("tools/call", json!({"name":"zixcel_public_resource"}))["error"]["code"],
        -32602
    );
}

#[test]
fn initialize_requires_declared_fields_and_object_client_contracts() {
    for field in ["protocolVersion", "capabilities", "clientInfo"] {
        let mut params = initialize();
        params.as_object_mut().expect("object").remove(field);
        assert_eq!(reply("initialize", params)["error"]["code"], -32602);
    }
    for field in ["capabilities", "clientInfo"] {
        for invalid in [Value::Null, json!([]), json!(true), json!(1), json!("{}")] {
            let mut params = initialize();
            params[field] = invalid;
            assert_eq!(reply("initialize", params)["error"]["code"], -32602);
        }
    }
    for field in ["name", "version"] {
        for invalid in [Value::Null, json!(true), json!(1), json!([])] {
            let mut params = initialize();
            params["clientInfo"][field] = invalid;
            assert_eq!(reply("initialize", params)["error"]["code"], -32602);
        }
        let mut params = initialize();
        params["clientInfo"]
            .as_object_mut()
            .expect("object")
            .remove(field);
        assert_eq!(reply("initialize", params)["error"]["code"], -32602);
    }
    for invalid in [Value::Null, json!(true), json!(1), json!([])] {
        let mut params = initialize();
        params["protocolVersion"] = invalid;
        assert_eq!(reply("initialize", params)["error"]["code"], -32602);
    }
    for capability in ["experimental", "roots", "sampling", "elicitation"] {
        for invalid in [Value::Null, json!([]), json!(true), json!(1)] {
            let mut params = initialize();
            params["capabilities"][capability] = invalid;
            assert_eq!(reply("initialize", params)["error"]["code"], -32602);
        }
    }
    for invalid in [Value::Null, json!(true), json!(1), json!([])] {
        let mut params = initialize();
        params["clientInfo"]["title"] = invalid.clone();
        assert_eq!(reply("initialize", params)["error"]["code"], -32602);
    }
    for invalid in [Value::Null, json!(1), json!([]), json!("true")] {
        let mut params = initialize();
        params["capabilities"]["roots"] = json!({"listChanged":invalid});
        assert_eq!(reply("initialize", params)["error"]["code"], -32602);
    }
    for invalid in [Value::Null, json!(true), json!(1), json!([]), json!("text")] {
        let mut params = initialize();
        params["capabilities"]["experimental"] = json!({"synthetic":invalid});
        assert_eq!(reply("initialize", params)["error"]["code"], -32602);
    }
    let mut legal = initialize();
    legal["capabilities"] = json!({"experimental":{"synthetic":{}},"roots":{"listChanged":true},"sampling":{},"elicitation":{},"example.test/extension":{}});
    legal["clientInfo"]["title"] = json!("Synthetic title");
    assert!(reply("initialize", legal.clone()).get("result").is_some());
    legal["protocolVersion"] = json!("2025-11-25");
    assert_eq!(
        reply("initialize", legal)["result"]["protocolVersion"],
        "2025-06-18"
    );
}

#[test]
fn notifications_never_dispatch_or_reply_for_metadata_or_bad_init() {
    for method in [
        "notifications/initialized",
        "initialize",
        "tools/list",
        "tools/call",
        "resources/list",
        "resources/read",
    ] {
        for metadata in [json!({}), Value::Null, json!([]), json!(true)] {
            let input = json!({"jsonrpc":"2.0","method":method,"params":{"_meta":metadata}});
            assert!(gateway().handle_line(&input.to_string()).is_none());
        }
    }
}
