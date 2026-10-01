mod support;

use serde_json::Value;

#[test]
fn validation_is_json_only_and_non_executing() {
    let output = support::gateway_command()
        .arg("validate-config")
        .arg("--config")
        .arg(support::fixture("examples/local.toml"))
        .output()
        .expect("run validation");
    assert!(output.status.success());
    let result: Value = serde_json::from_slice(&output.stdout).expect("one JSON result");
    assert_eq!(
        result["schema"],
        "zixcel://mcp/public-catalog-validation/v1"
    );
    assert_eq!(result["server_id"], "example-public-mcp");
    assert_eq!(result["resource_count"], 1);
    assert_eq!(result["network_access"], false);
    assert_eq!(result["external_actions"], false);
    assert_eq!(result.as_object().map(serde_json::Map::len), Some(7));
}
