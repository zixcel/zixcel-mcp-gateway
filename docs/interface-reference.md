# zixcel-mcp-gateway interface reference

Use the [usage guide](getting-started.md) for the first steps. This reference preserves the current interface details and operational limits. Run command examples from the repository root, after preparing the exact declared dependencies and registered configuration.

## Responsibility boundary

- Zixcel owns validation, bounded file input, JSON-RPC dispatch, and stdio.
- A consuming service owns its server identity, public resources, deployment,
  release, and disclosure review.
- The runtime has no network client, command execution, writes, arbitrary file
  resources, credential fields, or product-specific infrastructure definition.
- Configuration is one explicit regular TOML file, limited to 64 KiB and opened
  without following links.
- Each JSON-RPC frame is limited to 1 MiB.

This repository is not an aggregate ecosystem catalog. A service must not add
another organization's resources to its configuration without an explicit
cross-organization publication contract.

## MCP surface

| Method | Result |
| --- | --- |
| `initialize` | Configured server identity and the fixed safety instructions |
| `tools/list` | Generic catalog and resource tools |
| `tools/call` | Configured public catalog or one configured resource |
| `resources/list` | Only resources in the validated configuration |
| `resources/read` | Only `zixcel://public/{resource-id}` |

Notifications without an ID produce no response. Unknown methods, extra
arguments, unknown resource IDs, and non-Zixcel URIs are rejected.

## Configuration

[`schema/public-catalog.schema.json`](../schema/public-catalog.schema.json) defines
the product-neutral contract. [`examples/local.toml`](../examples/local.toml) uses
fictional data. Real product names and descriptions belong in the consuming
service repository.

The configuration accepts 1 to 64 unique lowercase kebab-case resource IDs.
Text is bounded and control characters and secret-like markers are rejected.
There are deliberately no URL, path, command, token, or credential fields.

## Run

```bash
cargo run --offline --locked -- \
  validate-config --config examples/local.toml

cargo run --offline --locked -- \
  --config examples/local.toml
```

Example request:

```json
{"jsonrpc":"2.0","id":1,"method":"resources/read","params":{"uri":"zixcel://public/example-service"}}
```

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

## Declared method contract and verification scope

The server declares protocol version `2025-06-18`. This follow-up candidate
checks required initialize fields, client identity/capability object types,
object `_meta` on implemented methods, and the tools' declared object arguments.
Metadata is opaque and does not alter the catalog or its disclosure boundary.
The reserved request `_meta.progressToken` is a string or JSON number, including
fractional values. Other metadata keys and notification metadata stay opaque.
No progress reporting feature is added by this input-type validation.
An unissued pagination cursor remains invalid; this bounded catalog does not
issue `nextCursor` values or introduce pagination state.

The stateless gateway does not enforce a connection phase machine. Compliant
clients must initialize first, send `notifications/initialized`, and respect the
selected version. A different client version may receive the supported
`2025-06-18` fallback. Missing initialize fields and mistyped inputs are rejected;
this is distinct from implementing new optional client/server features.

Record compilation and full regression review against the exact source tree.
Tests for earlier candidate bytes are not a verification of this change.
HTTP authentication, deployment, sampling, prompts, logging and subscriptions
are outside the implemented surface; no full MCP conformance claim is made.
