# zixcel-mcp-gateway

Expose reviewed metadata through a validated, bounded MCP interface.

## What you can do

- Validate a configured resource projection.
- Serve declared resources over stdio.

## Current scope

Applications own disclosure policy and resource content. Configuration must be supplied before the gateway is started.

The Rust library is published on crates.io. Standalone source builds use the locked public dependencies; each release records its source SHA and distribution checksum.

## Getting started

Install the Rust toolchain declared by this repository. Cargo resolves this package's dependencies through crates.io. Run from this repository:

```sh
cargo test --locked
```

## Examples and interface details

## Configuration

[`schema/public-catalog.schema.json`](schema/public-catalog.schema.json) defines
the product-neutral contract. [`examples/local.toml`](examples/local.toml) uses
fictional data. Real product names and descriptions belong in the consuming
service repository.

The configuration accepts 1 to 64 unique lowercase kebab-case resource IDs.
Text is bounded and control characters and secret-like markers are rejected.
There are deliberately no URL, path, command, token, or credential fields.

## Consumer pattern

Each service-owned MCP repository should:

1. depend on a reviewed release of `zixcel-mcp-gateway`;
2. keep its public catalog beside its service code;
3. embed or explicitly resolve only that catalog;
4. declare its own infrastructure and loopback/process boundary;
5. test that no resource owned by another service is exposed.

The local workspace may use a versioned path dependency while repositories are
being separated. A remote release should resolve the same declared version from
the approved package source.

## Documentation and source

[Interface reference](docs/interface-reference.md)

[Usage guide](docs/getting-started.md)

[Examples](examples) · [Schemas](schema) · [Implementation and public interfaces](src) · [Verification cases](tests) · [Contributing](CONTRIBUTING.md) · [Security reporting](SECURITY.md) · [License](LICENSE) · [Attribution notices](NOTICE)
