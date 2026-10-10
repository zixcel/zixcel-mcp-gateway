# Rust usage

## Problem
Embed the same validated catalog without spawning the executable.

## Solution and usage
The library exports `Config`, `Gateway` and `run_stdio`.

```rust
use zixcel_mcp_gateway::{Config, Gateway};
let config = Config::from_toml(include_str!("../examples/local.toml"))?;
let gateway = Gateway::new(config);
let response = gateway.handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"resources/list"}"#);
assert!(response.is_some());
# Ok::<(), String>(())
```

## Result
Responses contain the approved public catalog. Notifications produce no response. A consuming application must enforce its own identity, authorization and product policies.

`Config::load` applies bounded regular-file checks. `Config::from_toml` validates a supplied string; the embedding caller controls that string's allocation. `run_stdio` supplies the bounded line transport. These limits are application policy, not a claim of complete MCP SDK coverage.

## Request IDs and bounds
String IDs and exact integral JSON numbers are accepted; `1.0`, `1e3`, and integers beyond 64 bits retain their original numeric representation in replies. Fractional numbers, objects, arrays, booleans and explicit null IDs are rejected. Notifications omit the ID and produce no reply. Numeric decimal exponents must fit the decimal library's signed 64-bit scale representation; unsupported values are rejected rather than rounded.

`run_stdio` limits each complete input frame, including its newline, to 1 MiB. Direct `Gateway::handle_line` callers must apply their own frame admission limit. Parameter admission keeps a 128-level nesting budget; methods decode original JSON text directly into their final models. Very long decimal coefficients require substantially more CPU than ordinary IDs; the frame limit bounds input bytes and does not promise constant processing time. These bounds do not add a smaller numeric-ID length limit.

The official rmcp 3.5.0 ID model uses signed 64-bit integer or string variants. This gateway's exact decimal/exponent and larger numeric-ID support therefore does not imply that every rmcp peer accepts those forms. Ordinary string IDs avoid that numeric representation mismatch.
