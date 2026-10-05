mod support;

use serde_json::Value;
use std::io::Write;
use std::process::Stdio;

#[test]
fn stdio_supports_the_minimal_mcp_surface() {
    let requests = [
        request(
            1,
            "initialize",
            r#"{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"synthetic","version":"1"}}"#,
        ),
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#.into(),
        request(2, "tools/list", "{}"),
        request(
            3,
            "tools/call",
            r#"{"name":"zixcel_public_resource","arguments":{"resource":"example-service"}}"#,
        ),
        request(4, "resources/list", "{}"),
        request(
            5,
            "resources/read",
            r#"{"uri":"zixcel://public/example-service"}"#,
        ),
    ]
    .join("\n");
    let output = run(&format!("{requests}\n"));
    assert!(output.status.success(), "{output:?}");
    let responses = output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice::<Value>(line).expect("JSON response"))
        .collect::<Vec<_>>();
    assert_eq!(responses.len(), 5);
    assert_eq!(
        responses[0]["result"]["serverInfo"]["name"],
        "example-public-mcp"
    );
    assert_eq!(
        responses[1]["result"]["tools"].as_array().map(Vec::len),
        Some(2)
    );
    assert!(
        responses[2]["result"]["content"][0]["text"]
            .as_str()
            .is_some_and(|text| text.contains("example-service"))
    );
    assert_eq!(
        responses[3]["result"]["resources"].as_array().map(Vec::len),
        Some(1)
    );
    assert!(
        responses[4]["result"]["contents"][0]["text"]
            .as_str()
            .is_some_and(|text| text.contains("example-service"))
    );
}

fn request(id: u8, method: &str, params: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{params}}}"#)
}

fn run(input: &str) -> std::process::Output {
    let config = support::fixture("examples/local.toml");
    let mut child = support::gateway_command()
        .arg("--config")
        .arg(config)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    child
        .stdin
        .take()
        .expect("child stdin")
        .write_all(input.as_bytes())
        .expect("write requests");
    child.wait_with_output().expect("gateway output")
}
