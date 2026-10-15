#![cfg(feature = "http")]
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use zixcel_mcp_gateway::http::{HttpOptions, run_http};
use zixcel_mcp_gateway::{Config, Gateway};

const LIST: &str = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#;
const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"fixture","version":"1"}}}"#;
fn gateway() -> Gateway {
    Gateway::new(Config::from_toml(include_str!("../examples/local.toml")).unwrap())
}

async fn server() -> (SocketAddr, oneshot::Sender<()>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let options = HttpOptions::new("https://catalog.example.org")
        .unwrap()
        .allow_browser_origin("https://portfolio.example.org")
        .unwrap();
    let (send, receive) = oneshot::channel();
    let task = tokio::spawn(async move {
        run_http(gateway(), listener, options, async {
            let _ = receive.await;
        })
        .await
        .unwrap();
    });
    (address, send, task)
}

fn request(address: SocketAddr, method: &str, body: &str, extra: &str) -> String {
    format!(
        "{method} /mcp HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nMCP-Protocol-Version: 2025-06-18\r\nContent-Length: {}\r\n{extra}\r\n{body}",
        body.len()
    )
}
async fn exchange(address: SocketAddr, request: &str) -> String {
    tokio::time::timeout(Duration::from_secs(3), async {
        let mut socket = TcpStream::connect(address).await.unwrap();
        socket.write_all(request.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        socket.read_to_end(&mut response).await.unwrap();
        String::from_utf8(response).unwrap()
    })
    .await
    .unwrap()
}
fn body(response: &str) -> &str {
    response.split_once("\r\n\r\n").unwrap().1
}

#[tokio::test]
async fn initialization_lists_calls_and_reads_have_the_identical_stdio_semantics() {
    let (address, stop, task) = server().await;
    let requests = [
        INIT,
        LIST,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"zixcel_public_resource","arguments":{"resource":"example-service"}}}"#,
        r#"{"jsonrpc":"2.0","id":4,"method":"resources/list","params":{}}"#,
        r#"{"jsonrpc":"2.0","id":5,"method":"resources/read","params":{"uri":"zixcel://public/example-service"}}"#,
        r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"process_execute","arguments":{}}}"#,
        r#"{"jsonrpc":"2.0","id":7,"method":"resources/read","params":{"uri":"file:///private"}}"#,
        r#"{"jsonrpc":"2.0","id":184467440737095516160,"method":"tools/list","params":{}}"#,
        r#"{"jsonrpc":"2.0","id":1e+1000,"method":"tools/list","params":{}}"#,
    ];
    for input in requests {
        let response = exchange(address, &request(address, "POST", input, "")).await;
        assert!(response.starts_with("HTTP/1.1 200"));
        assert_eq!(body(&response), gateway().handle_line(input).unwrap());
        assert!(!response.to_ascii_lowercase().contains("mcp-session-id:"));
    }
    stop.send(()).unwrap();
    task.await.unwrap();
}

#[tokio::test]
async fn notification_is_accepted_without_a_body_and_get_does_not_open_an_sse_stream() {
    let (address, stop, task) = server().await;
    let input = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    let response = exchange(address, &request(address, "POST", input, "")).await;
    assert!(response.starts_with("HTTP/1.1 202"));
    assert_eq!(body(&response), "");
    for method in ["GET", "DELETE", "PUT"] {
        let response = exchange(address, &request(address, method, "", "")).await;
        assert!(response.starts_with("HTTP/1.1 405"));
    }
    stop.send(()).unwrap();
    task.await.unwrap();
}

#[tokio::test]
async fn origins_host_versions_session_and_credentials_are_checked_before_dispatch() {
    let (address, stop, task) = server().await;
    for (header, status) in [
        ("Origin: https://untrusted.example\r\n", 403),
        ("Origin: null\r\n", 403),
        (
            "Origin: https://catalog.example.org\r\nOrigin: https://catalog.example.org\r\n",
            400,
        ),
        ("MCP-Protocol-Version: 2024-11-05\r\n", 400),
        ("Mcp-Session-Id: synthetic-not-issued\r\n", 400),
        ("Authorization: Bearer synthetic-never-issued\r\n", 400),
        ("Cookie: synthetic=never-issued\r\n", 400),
    ] {
        let response = exchange(address, &request(address, "POST", LIST, header)).await;
        assert!(
            response.starts_with(&format!("HTTP/1.1 {status}")),
            "{response}"
        );
        assert!(!response.contains("synthetic-never-issued"));
    }
    let untrusted = request(address, "POST", LIST, "")
        .replace(&format!("Host: {address}"), "Host: rebinding.example");
    assert!(
        exchange(address, &untrusted)
            .await
            .starts_with("HTTP/1.1 403")
    );
    let valid = exchange(
        address,
        &request(
            address,
            "POST",
            LIST,
            "Origin: https://portfolio.example.org\r\n",
        ),
    )
    .await;
    assert!(valid.starts_with("HTTP/1.1 200"));
    assert!(valid.contains("access-control-allow-origin: https://portfolio.example.org"));
    stop.send(()).unwrap();
    task.await.unwrap();
}

#[tokio::test]
async fn version_is_required_after_initialization_and_framing_is_bounded() {
    let (address, stop, task) = server().await;
    let no_version = |text: String| text.replace("MCP-Protocol-Version: 2025-06-18\r\n", "");
    assert!(
        exchange(address, &no_version(request(address, "POST", INIT, "")))
            .await
            .starts_with("HTTP/1.1 200")
    );
    assert!(
        exchange(address, &no_version(request(address, "POST", LIST, "")))
            .await
            .starts_with("HTTP/1.1 400")
    );
    for input in ["[]", "{", "true", r#"{"jsonrpc":"2.0","id":1,"result":{}}"#] {
        assert!(
            exchange(address, &request(address, "POST", input, ""))
                .await
                .starts_with("HTTP/1.1 400")
        );
    }
    let wrong_media = request(address, "POST", LIST, "")
        .replace("Content-Type: application/json", "Content-Type: text/plain");
    assert!(
        exchange(address, &wrong_media)
            .await
            .starts_with("HTTP/1.1 415")
    );
    let wrong_accept = request(address, "POST", LIST, "")
        .replace("application/json, text/event-stream", "application/json");
    assert!(
        exchange(address, &wrong_accept)
            .await
            .starts_with("HTTP/1.1 406")
    );
    let large = format!("\"{}\"", "a".repeat(64 * 1024));
    assert!(
        exchange(address, &request(address, "POST", &large, ""))
            .await
            .starts_with("HTTP/1.1 413")
    );
    stop.send(()).unwrap();
    task.await.unwrap();
}

#[tokio::test]
async fn explicit_preflight_allows_only_public_post_headers() {
    let (address, stop, task) = server().await;
    let headers = "Origin: https://portfolio.example.org\r\nAccess-Control-Request-Method: POST\r\nAccess-Control-Request-Headers: content-type,mcp-protocol-version\r\n";
    assert!(
        exchange(address, &request(address, "OPTIONS", "", headers))
            .await
            .starts_with("HTTP/1.1 204")
    );
    let denied = headers.replace("content-type,mcp-protocol-version", "authorization");
    assert!(
        exchange(address, &request(address, "OPTIONS", "", &denied))
            .await
            .starts_with("HTTP/1.1 400")
    );
    stop.send(()).unwrap();
    task.await.unwrap();
}

#[test]
fn origins_are_canonical_https_and_cannot_contain_credentials_paths_or_wildcards() {
    for value in [
        "http://example.org",
        "https://example.org/",
        "https://user@example.org",
        "https://example.org/path",
        "https://example.org?x=1",
        "https://*.example.org",
        "https://EXAMPLE.org",
        "https://example.org:0",
    ] {
        assert!(HttpOptions::new(value).is_err(), "{value}");
    }
    assert!(
        HttpOptions::new("https://example.org")
            .unwrap()
            .allow_browser_origin("https://example.org")
            .is_err()
    );
}

#[tokio::test]
async fn public_interface_binding_is_never_accepted() {
    let listener = TcpListener::bind("0.0.0.0:0").await.unwrap();
    assert!(
        run_http(
            gateway(),
            listener,
            HttpOptions::new("https://example.org").unwrap(),
            async {}
        )
        .await
        .is_err()
    );
}

#[tokio::test(start_paused = true)]
async fn slow_header_connections_are_released_at_the_connection_deadline() {
    let (address, stop, task) = server().await;
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket.write_all(b"POST /mcp HTTP/1.1\r\n").await.unwrap();
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(11)).await;
    let mut bytes = Vec::new();
    tokio::time::timeout(Duration::from_secs(1), socket.read_to_end(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("\"result\""));
    stop.send(()).unwrap();
    task.await.unwrap();
}

#[tokio::test]
async fn connection_budget_applies_before_headers_and_dropping_one_releases_a_slot() {
    let (address, stop, task) = server().await;
    let mut held = Vec::new();
    for _ in 0..8 {
        let mut socket = TcpStream::connect(address).await.unwrap();
        socket.write_all(b"POST /mcp HTTP/1.1\r\n").await.unwrap();
        held.push(socket);
    }
    let mut ninth = TcpStream::connect(address).await.unwrap();
    ninth
        .write_all(request(address, "POST", LIST, "").as_bytes())
        .await
        .unwrap();
    let mut bytes = Vec::new();
    assert!(
        tokio::time::timeout(Duration::from_millis(50), ninth.read_to_end(&mut bytes))
            .await
            .is_err()
    );
    drop(held.remove(0));
    tokio::time::timeout(Duration::from_secs(3), ninth.read_to_end(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert!(
        String::from_utf8(bytes)
            .unwrap()
            .starts_with("HTTP/1.1 200")
    );
    drop(held);
    stop.send(()).unwrap();
    task.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn an_incomplete_body_times_out_without_a_result_or_payload_echo() {
    let (address, stop, task) = server().await;
    let mut socket = TcpStream::connect(address).await.unwrap();
    let frame = request(address, "POST", LIST, "");
    let headers = frame.split_once("\r\n\r\n").unwrap().0;
    socket
        .write_all(format!("{headers}\r\n\r\n{{").as_bytes())
        .await
        .unwrap();
    tokio::task::yield_now().await;
    let mut output = Vec::new();
    tokio::time::timeout(Duration::from_secs(4), socket.read_to_end(&mut output))
        .await
        .unwrap()
        .unwrap();
    assert!(
        String::from_utf8(output)
            .unwrap()
            .starts_with("HTTP/1.1 408")
    );
    stop.send(()).unwrap();
    task.await.unwrap();
}
