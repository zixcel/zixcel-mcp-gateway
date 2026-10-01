mod support;

use serde_json::Value;
use std::io::Write;
use std::process::Stdio;

#[test]
fn prohibited_io_and_execution_requests_are_rejected_without_disclosure() {
    let input = [
        rpc(1, "resources/read", r#"{"uri":"file:///etc/passwd"}"#),
        rpc(
            2,
            "tools/call",
            r#"{"name":"zixcel_public_resource","arguments":{"resource":"example-service","path":"/etc/passwd"}}"#,
        ),
        rpc(3, "filesystem/read", r#"{"path":"/etc/passwd"}"#),
        rpc(
            4,
            "tools/call",
            r#"{"name":"external_exec","arguments":{"command":"id"}}"#,
        ),
    ]
    .join("\n");
    let config = support::fixture("examples/local.toml");
    let output = run(
        config.to_str().expect("UTF-8 config path"),
        &format!("{input}\n"),
    );
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    assert!(!stdout.contains("root:"));
    for line in stdout.lines() {
        let response: Value = serde_json::from_str(line).expect("JSON response");
        assert!(response.get("error").is_some(), "{response}");
    }
}

#[test]
fn secret_like_config_is_rejected_without_echoing_its_value() {
    let source =
        std::fs::read_to_string(support::fixture("examples/local.toml")).expect("example config");
    let sensitive = source.replace(
        "schema_version = 1",
        "schema_version = 1\naccess_token = \"sensitive-value-123\"",
    );
    let path = unique_temp_path();
    std::fs::write(&path, sensitive).expect("write temporary config");
    let output = run(path.to_str().expect("UTF-8 path"), "");
    std::fs::remove_file(path).expect("remove temporary config");
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    assert!(stderr.contains("prohibited secret-like marker"));
    assert!(!stderr.contains("sensitive-value-123"));
}

#[test]
fn runtime_sources_do_not_link_network_or_command_execution_apis() {
    let sources = [
        include_str!("../src/config.rs"),
        include_str!("../src/file_input.rs"),
        include_str!("../src/protocol.rs"),
        include_str!("../src/server.rs"),
        include_str!("../src/transport.rs"),
    ]
    .join("\n");
    for prohibited in ["std::net::", "TcpStream", "UdpSocket", "Command::new"] {
        assert!(!sources.contains(prohibited), "found {prohibited}");
    }
    assert!(sources.contains("O_NOFOLLOW"));
}

fn rpc(id: u8, method: &str, params: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{params}}}"#)
}

fn run(config: &str, input: &str) -> std::process::Output {
    let mut child = support::gateway_command()
        .args(["--config", config])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write");
    child.wait_with_output().expect("gateway output")
}

fn unique_temp_path() -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "zixcel-mcp-security-{}-{nonce}.toml",
        std::process::id()
    ))
}
