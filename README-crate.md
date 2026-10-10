# zixcel-mcp-gateway

## Problem
Applications need to expose approved public metadata to MCP clients without adding product authentication, network access, commands, or arbitrary file resources.

## Solution
This Rust library and stdio executable project a validated, closed TOML catalog through a fixed read-only MCP surface. Configuration files are bounded to 64 KiB, catalogs to 64 resources, and stdio input lines to 1 MiB. The declared protocol version is `2025-06-18`; compatibility with other protocol versions is not established.

## Usage
Install the library with `cargo add zixcel-mcp-gateway`. Run the downloaded source with:

```sh
cargo run --locked -- validate-config --config examples/local.toml
cargo run --locked -- --config examples/local.toml
```

The example contains fictional public metadata. MCP clients communicate with the second command over newline-delimited JSON on stdin/stdout. See [Rust usage](docs/rust.md) and [catalog schema](schema/public-catalog.schema.json).

## Result
Clients can enumerate public tools and resources and read approved summaries. Product identity, authorization and private data stay in the consuming application; they are not dependencies of this crate.

## License
Apache-2.0. Retained prior license text and attribution are included in LICENSE-MIT and NOTICE.

## Protocol contract

The gateway checks the declared `2025-06-18` input shapes:
required initialize fields and client identity/capabilities, object `_meta`
on implemented methods, and each tool's declared object arguments. 
Request `_meta.progressToken`, when present, is a string or JSON number;
fractional numbers are allowed independently of integer request IDs. Other
metadata values remain opaque, and notification metadata has no request-token
type restriction. This type check does not add progress notifications.

Unsupported client versions receive the supported `2025-06-18` fallback;
this does not establish compatibility with every other version. The gateway
is stateless. Compliant clients initialize first and send
`notifications/initialized` before normal operations. This component does not
enforce a connection phase machine or authenticate a session.

Sampling, prompts, logging, subscriptions, HTTP authorization and live deployment
are outside its implemented surface. The documented method checks are not a
claim of complete MCP lifecycle or all optional features.
