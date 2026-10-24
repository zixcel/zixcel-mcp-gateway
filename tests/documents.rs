mod support;
use serde_json::{Value, json};
use zixcel_mcp_gateway::{Config, Gateway};
const SOURCE: &str = include_str!("../examples/documents.toml");
fn response(gateway: &Gateway, method: &str, params: &Value) -> Value {
    serde_json::from_str(
        &gateway
            .handle_line(
                &json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}).to_string(),
            )
            .unwrap(),
    )
    .unwrap()
}
#[test]
fn full_public_text_retains_exact_bytes_digest_media_and_attribution() {
    let gateway = Gateway::new(Config::from_toml(SOURCE).unwrap());
    let out = response(
        &gateway,
        "resources/read",
        &json!({"uri":"zixcel://public/guidance"}),
    );
    let document = &out["result"]["contents"][0];
    assert_eq!(document["mimeType"], "text/markdown");
    assert_eq!(
        document["text"],
        "# Public guidance\nAuthorization and secret handling are separate responsibilities.\n"
    );
    assert_eq!(document["_meta"]["zixcel/license"], "Apache-2.0");
    assert_eq!(document["_meta"]["zixcel/attribution"], "Zixcel");
    let listed = response(&gateway, "resources/list", &json!({}));
    assert_eq!(
        listed["result"]["resources"][0]["mimeType"],
        "text/markdown"
    );
    let catalog = response(
        &gateway,
        "tools/call",
        &json!({"name":"zixcel_public_catalog","arguments":{}}),
    );
    let text = catalog["result"]["content"][0]["text"].as_str().unwrap();
    assert!(!text.contains("Authorization and secret handling"));
    assert!(text.contains("source_url"));
}
#[test]
fn bad_hash_unknown_fields_nonpublic_media_control_and_v1_documents_are_rejected() {
    for value in [
        SOURCE.replace("schema_version = 2", "schema_version = 1"),
        SOURCE.replace(
            "classification = \"public\"",
            "classification = \"internal\"",
        ),
        SOURCE.replace("text/markdown", "text/html"),
        SOURCE.replace("# Public guidance", "# Changed guidance"),
        SOURCE.replace("sha256 = \"", "sha256 = \"0"),
        format!("{SOURCE}\napi_key = \"not-admitted\""),
        SOURCE.replace(
            "https://example.org/guidance/revision-1",
            "file:///not-admitted",
        ),
    ] {
        assert!(Config::from_toml(&value).is_err());
    }
}
#[test]
fn document_limits_apply_before_parsing_and_unknown_uris_never_fetch() {
    assert!(Config::from_toml(&" ".repeat(65537)).is_err());
    let gateway = Gateway::new(Config::from_toml(SOURCE).unwrap());
    for uri in [
        "file:///not-admitted",
        "https://example.org/",
        "zixcel://public/guidance?path=elsewhere",
        "zixcel://public/missing",
    ] {
        assert_eq!(
            response(&gateway, "resources/read", &json!({"uri":uri}))["error"]["code"],
            -32602
        );
    }
}
#[test]
fn real_stdio_returns_complete_document_and_leaves_legacy_metadata_available() {
    use std::{io::Write, process::Stdio};
    let mut child = support::gateway_command()
        .args(["--config"])
        .arg(support::fixture("examples/documents.toml"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let input = json!({"jsonrpc":"2.0","id":"document","method":"resources/read","params":{"uri":"zixcel://public/guidance"}}).to_string()+"\n";
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let out: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(out["id"], "document");
    assert!(
        out["result"]["contents"][0]["text"]
            .as_str()
            .unwrap()
            .ends_with("responsibilities.\n")
    );
    let legacy = Gateway::new(Config::from_toml(include_str!("../examples/local.toml")).unwrap());
    assert_eq!(
        response(
            &legacy,
            "resources/read",
            &json!({"uri":"zixcel://public/example-service"})
        )["result"]["contents"][0]["mimeType"],
        "application/json"
    );
}
