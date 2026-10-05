use serde_json::Value;
use zixcel_mcp_gateway::{Config, Gateway};

fn gateway() -> Gateway {
    Gateway::new(Config::from_toml(include_str!("../examples/local.toml")).expect("fixture"))
}
fn initialize(params: &str) {
    // Preserve every unknown/duplicate occurrence while supplying the newly
    // validated required initialize fields. This does not change nesting depth.
    let params = format!(
        r#"{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"synthetic","version":"1"}},{}}}"#,
        &params[1..params.len() - 1]
    );
    // Match the already reviewed ordinary serde_json recursion budget.
    let accepted = serde_json::from_str::<Value>(&params).is_ok();
    let input = format!(r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{params}}}"#);
    let output = gateway().handle_line(&input).expect("response");
    let response: Value = serde_json::from_str(&output).expect("response JSON");
    assert_eq!(
        response.get("result").is_some(),
        accepted,
        "{input}: {response}"
    );
    if !accepted {
        assert_eq!(response["error"]["code"], -32602);
        assert_eq!(response["id"], 1);
    }
}
fn arrays(levels: usize) -> String {
    format!("{}0{}", "[".repeat(levels), "]".repeat(levels))
}
#[test]
fn every_duplicate_occurrence_counts_at_127_128_129_container_boundaries() {
    for containers in [127, 128, 129] {
        let deep = arrays(containers - 1);
        for params in [
            format!(r#"{{"same":{deep},"same":0}}"#),
            format!(r#"{{"same":0,"same":{deep}}}"#),
            format!(r#"{{"same":0,"same":{deep},"same":1}}"#),
            format!(r#"{{"same":{deep},"same":{deep},"same":0}}"#),
            format!(r#"{{"same":{deep},"\u0073ame":0}}"#),
            format!(r#"{{"\u0073ame":0,"same":{deep}}}"#),
        ] {
            initialize(&params);
        }
    }
}
#[test]
fn arrays_and_unknown_nested_objects_check_every_occurrence() {
    for containers in [127, 128, 129] {
        let deep = arrays(containers - 3);
        for params in [
            format!(r#"{{"outer":[{{"unknown":{deep},"unknown":0}}]}}"#),
            format!(r#"{{"outer":[{{"unknown":0,"unknown":{deep}}}]}}"#),
            format!(r#"{{"outer":[{{"unknown":{deep},"unknown":0}}],"outer":[]}}"#),
            format!(r#"{{"outer":[],"outer":[{{"unknown":0,"unknown":{deep}}}]}}"#),
            format!(r#"{{"outer":[{{"unknown":{deep},"unknown":0}},{{"unknown":0}}]}}"#),
        ] {
            initialize(&params);
        }
    }
}
#[test]
fn closed_models_still_reject_unknown_fields_with_duplicate_depth_values() {
    let deep = arrays(128);
    for method in ["tools/list", "resources/list", "resources/read"] {
        let input = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"{method}","params":{{"unknown":{deep},"unknown":0}}}}"#
        );
        let output = gateway().handle_line(&input).expect("response");
        let response: Value = serde_json::from_str(&output).expect("JSON");
        assert_eq!(response["error"]["code"], -32602);
    }
    for tool in ["zixcel_public_catalog", "zixcel_public_resource"] {
        let input = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{tool}","arguments":{{"unknown":{deep},"unknown":0}}}}}}"#
        );
        let output = gateway().handle_line(&input).expect("response");
        let response: Value = serde_json::from_str(&output).expect("JSON");
        assert_eq!(response["error"]["code"], -32602);
    }
}
