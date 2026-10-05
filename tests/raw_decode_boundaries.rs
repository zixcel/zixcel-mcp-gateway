use serde_json::Value;
use zixcel_mcp_gateway::{Config, Gateway};

const TAG_KEYS: [&str; 4] = [
    r#""$serde_json::private::RawValue""#,
    r#""\u0024serde_json::private::RawValue""#,
    r#""$serde_json::private::Number""#,
    r#""\u0024serde_json::private::Number""#,
];
const TAG_VALUES: [&str; 5] = [
    r#""{}""#,
    r#""1""#,
    r#""bad""#,
    r#""{\"resource\":\"example-service\"}""#,
    r#"{"nested":"1"}"#,
];
fn tag_objects() -> Vec<String> {
    TAG_KEYS
        .into_iter()
        .flat_map(|key| {
            TAG_VALUES
                .into_iter()
                .map(move |value| format!("{{{key}:{value}}}"))
        })
        .collect()
}

fn gateway() -> Gateway {
    Gateway::new(Config::from_toml(include_str!("../examples/local.toml")).expect("fixture"))
}
fn reply(input: &str) -> Value {
    let raw = gateway().handle_line(input).expect("request response");
    serde_json::from_str(&raw).expect("valid response")
}
fn error(input: &str, expected: i32) {
    let value = reply(input);
    assert_eq!(value["error"]["code"], expected, "{input}: {value}");
    assert!(value.get("result").is_none(), "{input}: {value}");
}
fn call(tool: &str, arguments: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
    )
}
#[test]
fn root_and_id_objects_never_become_internal_numbers_or_raw_values() {
    for object in tag_objects() {
        error(&object, -32600);
        error(
            &format!(r#"{{"jsonrpc":"2.0","id":{object},"method":"resources/list"}}"#),
            -32600,
        );
    }
    for key in TAG_KEYS {
        error(
            &format!(r#"{{"jsonrpc":"2.0","id":1,"method":"resources/list",{key}:"{{}}"}}"#),
            -32600,
        );
    }
}
#[test]
fn params_are_decoded_from_original_text_for_every_method() {
    for object in tag_objects() {
        for method in [
            "initialize",
            "tools/list",
            "tools/call",
            "resources/list",
            "resources/read",
        ] {
            let input =
                format!(r#"{{"jsonrpc":"2.0","id":1,"method":"{method}","params":{object}}}"#);
            if method == "initialize" {
                // The tag object is ordinary JSON, but lacks required fields.
                error(&input, -32602);
                let valid = format!(
                    r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"synthetic","version":"1"}},{}}}}}"#,
                    &object[1..object.len() - 1]
                );
                assert!(reply(&valid).get("result").is_some(), "{valid}");
            } else {
                error(&input, -32602);
            }
        }
    }
}
#[test]
fn catalog_and_resource_arguments_do_not_redecode_internal_tags() {
    for object in tag_objects() {
        for tool in ["zixcel_public_catalog", "zixcel_public_resource"] {
            error(&call(tool, &object), -32602);
            error(&call(tool, &format!(r#"{{"nested":{object}}}"#)), -32602);
        }
        error(
            &call(
                "zixcel_public_resource",
                &format!(r#"{{"resource":{object}}}"#),
            ),
            -32602,
        );
        let input = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":{object},"arguments":{{}}}}}}"#
        );
        error(&input, -32602);
    }
}
#[test]
fn unknown_fields_remain_unknown_even_when_keys_are_escaped() {
    for key in TAG_KEYS {
        let argument = format!(r#"{{"resource":"example-service",{key}:"{{}}"}}"#);
        error(&call("zixcel_public_resource", &argument), -32602);
        let input = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"zixcel_public_catalog","arguments":{{}},{key}:"{{}}"}}}}"#
        );
        error(&input, -32602);
        let input = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"resources/read","params":{{"uri":"zixcel://public/example-service",{key}:"{{}}"}}}}"#
        );
        error(&input, -32602);
    }
}
#[test]
fn normal_payloads_keep_omitted_empty_params_and_arguments() {
    assert!(
        reply(&call("zixcel_public_catalog", "{}"))
            .get("result")
            .is_some()
    );
    for input in [
        r#"{"jsonrpc":"2.0","id":"catalog","method":"tools/call","params":{"name":"zixcel_public_catalog"}}"#,
        r#"{"jsonrpc":"2.0","id":-2,"method":"tools/call","params":{"name":"zixcel_public_resource","arguments":{"resource":"example-service"}}}"#,
        r#"{"jsonrpc":"2.0","id":1,"method":"resources/read","params":{"uri":"zixcel://public/example-service"}}"#,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"synthetic","version":"1"}}}"#,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#,
        r#"{"jsonrpc":"2.0","id":1,"method":"resources/list"}"#,
    ] {
        assert!(reply(input).get("result").is_some(), "{input}");
    }
    error(&call("zixcel_public_catalog", "null"), -32602);
    for method in ["tools/list", "resources/list"] {
        error(
            &format!(r#"{{"jsonrpc":"2.0","id":1,"method":"{method}","params":null}}"#),
            -32602,
        );
    }
}
#[test]
fn notifications_and_error_classes_remain_distinct() {
    for object in tag_objects() {
        let input = format!(r#"{{"jsonrpc":"2.0","method":"tools/call","params":{object}}}"#);
        assert!(gateway().handle_line(&input).is_none());
    }
    error("{", -32700);
    error(
        r#"{"jsonrpc":"2.0","id":null,"method":"resources/list"}"#,
        -32600,
    );
    error(
        r#"{"jsonrpc":"2.0","id":1,"method":"unknown","params":{}}"#,
        -32601,
    );
    error(
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"unknown","arguments":{}}}"#,
        -32602,
    );
}

#[test]
fn positional_arrays_are_not_json_rpc_objects_or_tool_arguments() {
    error(r#"["2.0",1,"resources/list",null]"#, -32600);
    for method in ["initialize", "tools/list", "resources/list"] {
        error(
            &format!(r#"{{"jsonrpc":"2.0","id":1,"method":"{method}","params":[]}}"#),
            -32602,
        );
    }
    error(&call("zixcel_public_catalog", "[]"), -32602);
    error(
        &call("zixcel_public_resource", r#"["example-service"]"#),
        -32602,
    );
    error(
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":["zixcel_public_catalog",{}]}"#,
        -32602,
    );
    error(
        r#"{"jsonrpc":"2.0","id":1,"method":"resources/read","params":["zixcel://public/example-service"]}"#,
        -32602,
    );
}
