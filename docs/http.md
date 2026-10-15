# Public metadata over Streamable HTTP

The optional `http` feature exposes the same validated in-memory public catalog
as stdio at `/mcp`. It does not fetch the catalog URLs, read additional files,
launch processes, connect accounts or introduce private/write tools. Configuration
is loaded and validated once. The default build remains stdio-only.

```sh
cargo run --locked --features http -- serve-http \
  --config examples/local.toml \
  --listen 127.0.0.1:8787 \
  --public-origin https://catalog.example.org
```

The example origin is a placeholder. No DNS, public deployment or ChatGPT
registration is performed. Configure your reviewed HTTPS endpoint in front of
the loopback service, preserve its fixed Host, and apply TLS, ingress rate limits,
request/header budgets and access logging without bodies/credentials at that
boundary. Untrusted forwarded headers cannot change the service's origin policy.

The server accepts only loopback IP listeners and exact Host values for the
listener or configured public authority. Any supplied Origin must be in the exact
HTTPS allowlist; `null`, wildcard origins, paths, credentials and duplicates are
rejected. Add a separate portfolio browser origin only with an explicit
`--allow-origin https://portfolio.example.org`. Requests without Origin are
allowed for server-to-server clients reading this intentionally public metadata.
Origin and CORS are not authentication. No authentication claims are made here;
private data or actions require a separately reviewed authorization design.

POST accepts JSON and advertises JSON responses. Clients must list
`application/json, text/event-stream` in Accept. Initialization may omit the
protocol header; subsequent requests require `MCP-Protocol-Version: 2025-06-18`.
Other versions are rejected, including the historical implicit-version path.
GET/DELETE return 405: this stateless endpoint has no SSE stream, replay history,
server-initiated requests or issued session IDs. Accepted notifications return
202 with an empty body. Unknown/prohibited tools and resources use the same RPC
rejection as stdio. Authentication/session/cookie inputs are refused rather than
being silently adopted. No UI resource is advertised; the public metadata use
case does not need embedded MCP Apps UI or host-message permissions.

At most eight connections are accepted, including slow headers. A connection
expires after ten seconds; request bodies have a two-second deadline and 64 KiB
limit; responses are limited to one MiB. A dropped/expired connection releases
its capacity. This is a bounded local adapter, not a claim that the public TLS
edge or a hostile network has been accepted. Validate the reverse proxy and its
resource/recovery policy before exposure.

## Verification

`cargo test --locked --all-features` covers the shared RPC semantics, exact large
numeric IDs, initialized notification, origin/host/version/credential refusals,
bounded payloads, preflight and the connection budget/deadline with real loopback
sockets. `cargo clippy --locked --all-features --all-targets -- -D warnings`
checks both the default CLI and optional adapter.

Managed Crowsi transport was evaluated first. Its loopback reader is a bounded
OAuth GET callback, not a JSON-RPC POST server. Its authority/envelope contracts
do not match this anonymous public catalog. This adapter therefore uses Axum's
HTTP framing and reuses this repository's exact Gateway validation/dispatch;
it does not copy a parser or introduce another authorization authority.

Public HTTPS/ingress configuration, ChatGPT workspace/connection scope,
optional directory submission and live connection acceptance are tracked in
coela-oss/coela-release#110. No API key is required to test this local server.

References: [MCP transport specification](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports),
[OpenAI MCP server guidance](https://developers.openai.com/plugins/build/mcp-server),
[ChatGPT connection acceptance](https://developers.openai.com/plugins/deploy/connect-chatgpt).
