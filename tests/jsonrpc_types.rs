use serde_json::{Value, json};
use zixcel_mcp_gateway::{Config, Gateway};

#[derive(serde::Deserialize)]
struct RawReply {
    id: Box<serde_json::value::RawValue>,
}

fn gateway() -> Gateway {
    Gateway::new(Config::from_toml(include_str!("../examples/local.toml")).expect("fixture"))
}
fn response(source: &str) -> Value {
    serde_json::from_str(&gateway().handle_line(source).expect("response")).expect("JSON")
}
#[test]
fn valid_json_invalid_requests_are_not_parse_errors() {
    for source in ["{}", "true", "7", "\"scalar\"", "null", "[]"] {
        let reply = response(source);
        assert_eq!(reply["error"]["code"], -32600, "{source}: {reply}");
        assert_eq!(reply["id"], Value::Null);
    }
}
#[test]
fn malformed_json_is_a_parse_error() {
    let reply = response("{");
    assert_eq!(reply["error"]["code"], -32700);
    assert_eq!(reply["id"], Value::Null);
}
#[test]
fn structured_and_boolean_ids_are_invalid() {
    for id in [json!({}), json!([]), json!(true)] {
        let reply =
            response(&json!({"jsonrpc":"2.0","id":id,"method":"resources/list"}).to_string());
        assert_eq!(reply["error"]["code"], -32600, "{reply}");
        assert_eq!(reply["id"], Value::Null);
    }
}
#[test]
fn mcp_string_and_integer_ids_are_echoed() {
    for id in [json!("request-a"), json!(4), json!(-2), json!(4.0)] {
        let reply =
            response(&json!({"jsonrpc":"2.0","id":id,"method":"resources/list"}).to_string());
        assert!(reply.get("result").is_some(), "{reply}");
        assert_eq!(reply["id"], id);
    }
}
#[test]
fn valid_notifications_produce_no_response() {
    for method in ["notifications/initialized", "resources/list", "missing"] {
        let request = json!({"jsonrpc":"2.0","method":method}).to_string();
        assert!(gateway().handle_line(&request).is_none());
    }
}

#[test]
fn mcp_rejects_explicit_null_id() {
    let reply = response(r#"{"jsonrpc":"2.0","id":null,"method":"resources/list"}"#);
    assert_eq!(reply["error"]["code"], -32600);
    assert_eq!(reply["id"], Value::Null);
    assert!(reply.get("result").is_none());
}
#[test]
fn mcp_rejects_fractional_id() {
    let reply = response(r#"{"jsonrpc":"2.0","id":1.5,"method":"resources/list"}"#);
    assert_eq!(reply["error"]["code"], -32600);
    assert_eq!(reply["id"], Value::Null);
    assert!(reply.get("result").is_none());
}

#[test]
fn exact_fractional_ids_are_rejected_without_float_rounding() {
    for id in [
        "1.0000000000000000001",
        "-1.0000000000000000001",
        "9007199254740992.5",
        "-9007199254740992.5",
        "1e-400",
        "-1e-400",
        "1e-1000000000",
        "18446744073709551616.1",
        "1.23e1",
    ] {
        let reply = response(&format!(
            r#"{{"jsonrpc":"2.0","id":{id},"method":"resources/list"}}"#
        ));
        assert_eq!(reply["error"]["code"], -32600, "{id}: {reply}");
        assert_eq!(reply["id"], Value::Null);
    }
}
#[test]
fn exact_integral_ids_preserve_decimal_exponent_and_large_values() {
    for id in [
        "-2",
        "-0",
        "1.0",
        "-2.00",
        "1e3",
        "1.20e1",
        "100e-2",
        "9007199254740993",
        "18446744073709551616",
        "-18446744073709551616",
        "1e400",
        "0e-400",
        "1e1000000000",
        "0e-1000000000",
    ] {
        let source = format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"resources/list"}}"#);
        let reply = gateway().handle_line(&source).expect("response");
        let parsed: RawReply = serde_json::from_str(&reply).expect("raw response");
        assert!(reply.contains("\"result\""), "{id}: {reply}");
        assert_eq!(parsed.id.get(), id, "{id}");
    }
}

#[test]
fn internal_number_tag_objects_remain_objects() {
    for tag in ["1", "bad"] {
        let source = format!(
            r#"{{"jsonrpc":"2.0","id":{{"$serde_json::private::Number":"{tag}"}},"method":"resources/list"}}"#
        );
        let reply = response(&source);
        assert_eq!(reply["error"]["code"], -32600);
        assert_eq!(reply["id"], Value::Null);
    }
}
#[test]
fn parameter_objects_are_not_reinterpreted_as_numbers() {
    for tag in ["1", "bad"] {
        let source = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"synthetic","version":"1"}},"$serde_json::private::Number":"{tag}"}}}}"#
        );
        let reply = response(&source);
        assert!(reply.get("result").is_some(), "{reply}");
        assert_eq!(reply["id"], 1);
    }
}
