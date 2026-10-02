# Using zixcel-mcp-gateway

Expose reviewed metadata through a validated, bounded MCP interface.

## Before you start

Applications own disclosure policy and resource content. Configuration must be supplied before the gateway is started.

## First steps

Run from the repository root:

```sh
cargo test --locked
```

## How to assess the result

- Validate a configured resource projection.
- Serve declared resources over stdio.

A passing source-level check establishes only what that check observes. Keep missing configuration, unavailable services and unverified deployment paths visible.

## Continue reading

[Repository overview](../README.md)
