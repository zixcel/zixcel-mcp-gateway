//! Optional stateless HTTP projection of the same approved public metadata as stdio.
//! TLS termination, public routing and `ChatGPT` registration remain operator-owned.
use axum::body::{Body, to_bytes};
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, Method, Request, StatusCode, Uri};
use axum::response::Response;
use axum::routing::any;
use axum::{Router, serve};
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::{Sleep, sleep, timeout};

use crate::Gateway;
const REQUEST_BYTES: usize = 64 * 1024;
const RESPONSE_BYTES: usize = 1024 * 1024;
const CONNECTIONS: usize = 8;
const CONNECTION_LIFETIME: Duration = Duration::from_secs(10);

/// A fixed public HTTPS origin and optional browser origins. No credentials or proxies.
#[derive(Clone, Debug)]
pub struct HttpOptions {
    authority: String,
    origins: Vec<String>,
}

impl HttpOptions {
    /// Validates one canonical HTTPS origin, without paths, credentials or wildcards.
    /// # Errors
    /// Rejects a non-origin URI or a non-HTTPS public target.
    pub fn new(public_origin: &str) -> Result<Self, String> {
        let authority = origin_authority(public_origin)?;
        Ok(Self {
            authority,
            origins: vec![public_origin.into()],
        })
    }

    /// Adds an explicit browser origin; server-to-server clients may omit Origin.
    /// # Errors
    /// Rejects invalid, duplicate or more than eight origins.
    pub fn allow_browser_origin(mut self, origin: &str) -> Result<Self, String> {
        origin_authority(origin)?;
        if self.origins.len() >= 8 || self.origins.iter().any(|value| value == origin) {
            return Err("Invalid browser origin allowlist".into());
        }
        self.origins.push(origin.into());
        Ok(self)
    }
}

fn origin_authority(value: &str) -> Result<String, String> {
    let uri: Uri = value.parse().map_err(|_| "Invalid HTTPS origin")?;
    let authority = uri.authority().ok_or("HTTPS origin has no authority")?;
    if uri.scheme_str() != Some("https")
        || authority.as_str().contains('@')
        || value != format!("https://{authority}")
        || authority.host().contains('*')
        || authority.as_str() != authority.as_str().to_ascii_lowercase()
        || authority.port_u16() == Some(0)
    {
        return Err("A canonical HTTPS origin is required".into());
    }
    Ok(authority.as_str().into())
}

struct Endpoint {
    gateway: Gateway,
    options: HttpOptions,
    local_authority: String,
}

fn unique_header<'a>(headers: &'a HeaderMap, name: &str) -> Result<Option<&'a str>, ()> {
    let mut values = headers.get_all(name).iter();
    let first = values.next();
    if values.next().is_some() {
        return Err(());
    }
    first
        .map(|value| value.to_str().map_err(|_| ()))
        .transpose()
}

fn response(status: StatusCode, text: &str, content_type: &'static str) -> Response {
    let mut result = Response::new(Body::from(text.to_owned()));
    *result.status_mut() = status;
    result
        .headers_mut()
        .insert("content-type", HeaderValue::from_static(content_type));
    result
        .headers_mut()
        .insert("cache-control", HeaderValue::from_static("no-store"));
    result.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    result
}

fn rejected(status: StatusCode) -> Response {
    response(status, "MCP request refused", "text/plain; charset=utf-8")
}

fn check_headers(headers: &HeaderMap, endpoint: &Endpoint) -> Result<Option<String>, StatusCode> {
    let host = unique_header(headers, "host").map_err(|()| StatusCode::BAD_REQUEST)?;
    if !matches!(host, Some(value) if value == endpoint.options.authority || value == endpoint.local_authority)
    {
        return Err(StatusCode::FORBIDDEN);
    }
    let origin = unique_header(headers, "origin").map_err(|()| StatusCode::BAD_REQUEST)?;
    if origin.is_some_and(|value| {
        !endpoint
            .options
            .origins
            .iter()
            .any(|allowed| allowed == value)
    }) {
        return Err(StatusCode::FORBIDDEN);
    }
    for name in ["authorization", "cookie", "mcp-session-id", "last-event-id"] {
        if headers.contains_key(name) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let version =
        unique_header(headers, "mcp-protocol-version").map_err(|()| StatusCode::BAD_REQUEST)?;
    if version.is_some_and(|value| value != "2025-06-18") {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(origin.map(str::to_owned))
}

async fn endpoint(State(state): State<Arc<Endpoint>>, request: Request<Body>) -> Response {
    let origin = match check_headers(request.headers(), &state) {
        Ok(origin) => origin,
        Err(status) => return rejected(status),
    };
    let mut result = handle(request, &state).await;
    if let Some(origin) = origin
        && let Ok(value) = HeaderValue::from_str(&origin)
    {
        result
            .headers_mut()
            .insert("access-control-allow-origin", value);
        result
            .headers_mut()
            .insert("vary", HeaderValue::from_static("Origin"));
    }
    result
}

fn discovery(request: &Request<Body>, state: &Endpoint) -> Response {
    if !matches!(*request.method(), Method::GET | Method::HEAD) {
        let mut result = rejected(StatusCode::METHOD_NOT_ALLOWED);
        result
            .headers_mut()
            .insert("allow", HeaderValue::from_static("GET, HEAD"));
        return result;
    }
    let card = state
        .gateway
        .server_card(&format!("https://{}/mcp", state.options.authority));
    let body = if request.method() == Method::HEAD {
        String::new()
    } else {
        card.to_string()
    };
    response(StatusCode::OK, &body, "application/json")
}

async fn handle(request: Request<Body>, state: &Endpoint) -> Response {
    if request.uri().query().is_some() {
        return rejected(StatusCode::BAD_REQUEST);
    }
    if request.uri().path() == "/.well-known/mcp/server-card.json" {
        return discovery(&request, state);
    }
    if request.method() == Method::OPTIONS {
        let allowed_headers = ["content-type", "accept", "mcp-protocol-version"];
        let headers = request.headers();
        let method = unique_header(headers, "access-control-request-method");
        let requested = unique_header(headers, "access-control-request-headers");
        if !headers.contains_key("origin")
            || !matches!(method, Ok(Some("POST")))
            || !matches!(requested, Ok(None | Some(_)))
        {
            return rejected(StatusCode::BAD_REQUEST);
        }
        if requested.ok().flatten().is_some_and(|value| {
            value
                .split(',')
                .any(|part| !allowed_headers.contains(&part.trim().to_ascii_lowercase().as_str()))
        }) {
            return rejected(StatusCode::BAD_REQUEST);
        }
        let mut result = response(StatusCode::NO_CONTENT, "", "text/plain");
        result.headers_mut().insert(
            "access-control-allow-methods",
            HeaderValue::from_static("POST"),
        );
        result.headers_mut().insert(
            "access-control-allow-headers",
            HeaderValue::from_static("content-type, accept, mcp-protocol-version"),
        );
        return result;
    }
    if request.method() != Method::POST {
        let mut result = rejected(StatusCode::METHOD_NOT_ALLOWED);
        result
            .headers_mut()
            .insert("allow", HeaderValue::from_static("POST, OPTIONS"));
        return result;
    }
    let headers = request.headers();
    let content_type = unique_header(headers, "content-type");
    if !matches!(content_type, Ok(Some(value)) if value.split(';').next().is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json")))
    {
        return rejected(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
    let Ok(Some(accept)) = unique_header(headers, "accept") else {
        return rejected(StatusCode::NOT_ACCEPTABLE);
    };
    let accepted: Vec<_> = accept
        .split(',')
        .map(|value| value.trim().to_ascii_lowercase())
        .collect();
    if !["application/json", "text/event-stream"]
        .iter()
        .all(|required| accepted.iter().any(|value| value == required))
    {
        return rejected(StatusCode::NOT_ACCEPTABLE);
    }
    let has_version = headers.contains_key("mcp-protocol-version");
    let bytes = match timeout(
        Duration::from_secs(2),
        to_bytes(request.into_body(), REQUEST_BYTES),
    )
    .await
    {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(_)) => return rejected(StatusCode::PAYLOAD_TOO_LARGE),
        Err(_) => return rejected(StatusCode::REQUEST_TIMEOUT),
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return rejected(StatusCode::BAD_REQUEST);
    };
    // HTTP framing does not reimplement RPC validation or grant an additional method surface.
    // Only initialization may omit the negotiated version in this deliberately modern-only transport.
    let value = serde_json::from_str::<Box<serde_json::value::RawValue>>(text);
    if !matches!(value, Ok(ref value) if value.get().starts_with('{')) {
        return rejected(StatusCode::BAD_REQUEST);
    }
    let Ok(method) = crate::protocol::http_method(text) else {
        return rejected(StatusCode::BAD_REQUEST);
    };
    if !has_version && method != "initialize" {
        return rejected(StatusCode::BAD_REQUEST);
    }
    match state.gateway.handle_line(text) {
        Some(reply) if reply.len() <= RESPONSE_BYTES => {
            response(StatusCode::OK, &reply, "application/json")
        }
        Some(_) => rejected(StatusCode::INSUFFICIENT_STORAGE),
        None => response(StatusCode::ACCEPTED, "", "application/json"),
    }
}

struct Connection {
    socket: TcpStream,
    deadline: Pin<Box<Sleep>>,
    _permit: OwnedSemaphorePermit,
}

impl Connection {
    fn expired(&mut self, cx: &mut Context<'_>) -> bool {
        self.deadline.as_mut().poll(cx).is_ready()
    }
}
impl AsyncRead for Connection {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if this.expired(cx) {
            return Poll::Ready(Err(io::ErrorKind::TimedOut.into()));
        }
        Pin::new(&mut this.socket).poll_read(cx, buf)
    }
}
impl AsyncWrite for Connection {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        if this.expired(cx) {
            return Poll::Ready(Err(io::ErrorKind::TimedOut.into()));
        }
        Pin::new(&mut this.socket).poll_write(cx, bytes)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().socket).poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().socket).poll_shutdown(cx)
    }
}
struct BoundedListener {
    socket: TcpListener,
    slots: Arc<Semaphore>,
}
impl axum::serve::Listener for BoundedListener {
    type Io = Connection;
    type Addr = SocketAddr;
    async fn accept(&mut self) -> (Connection, SocketAddr) {
        loop {
            if let Ok(permit) = self.slots.clone().acquire_owned().await
                && let Ok((socket, peer)) = self.socket.accept().await
            {
                return (
                    Connection {
                        socket,
                        deadline: Box::pin(sleep(CONNECTION_LIFETIME)),
                        _permit: permit,
                    },
                    peer,
                );
            }
            sleep(Duration::from_millis(100)).await;
        }
    }
    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }
}

/// Serves `/mcp` on an already bound loopback socket until shutdown.
/// Eight accepted connections, 10s connection lifetime, 2s bodies, 64KiB input, 1MiB output.
/// # Errors
/// Rejects public-interface listeners; TLS and external exposure must be reviewed separately.
pub async fn run_http(
    gateway: Gateway,
    listener: TcpListener,
    options: HttpOptions,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), String> {
    let address = listener
        .local_addr()
        .map_err(|_| "HTTP listener unavailable")?;
    if !address.ip().is_loopback() {
        return Err("MCP HTTP must bind a loopback IP".into());
    }
    let state = Arc::new(Endpoint {
        gateway,
        options,
        local_authority: address.to_string(),
    });
    let app = Router::new()
        .route("/mcp", any(endpoint))
        .route("/.well-known/mcp/server-card.json", any(endpoint))
        .with_state(state);
    let listener = BoundedListener {
        socket: listener,
        slots: Arc::new(Semaphore::new(CONNECTIONS)),
    };
    serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(|_| "HTTP server stopped".into())
}
