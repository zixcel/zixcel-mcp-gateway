# Explicit public documents (catalog v2)

Catalog v1 remains metadata-only with its existing marker checks and JSON resource
contents. Select `schema_version = 2` and an explicit `resource.document` to serve
complete `text/plain` or `text/markdown` bytes. See `examples/documents.toml`.
The native closed model and published schema reject unknown fields; v1 rejects
any document, and v2 without documents can still serve metadata resources.

Every document requires operator-selected public classification, raw UTF-8 SHA256,
source URL, media type, license, attribution and literal text. The gateway verifies
content bytes against the declared digest. Classification and hash agreement do
not prove confidentiality, truth, copyright permission or publisher trust. Admit
and review public sources before supplying configuration; this is not a DLP tool.
Public security prose may discuss passwords/authorization, while credential fields
remain absent from the closed input contract. The existing metadata marker policy
still applies to v1 and v2 metadata; it is not a guarantee that text contains no secret.

The entire supplied TOML is limited to 64 KiB, each document to 16 KiB and aggregate
document text to 32 KiB, measured as UTF-8 bytes. Hashing and validation operate on
that bounded literal data. Catalog/tools retain document provenance but omit bodies;
`resources/list` reports its actual media type, and `resources/read` returns exact
text with source/digest/license/attribution/classification in `_meta`.

No document path, URL loader, network request, command, credential or new tool is
introduced. The source URL is attribution only and is never dereferenced. Resource
requests still select only configured `zixcel://public/<id>` identities; arbitrary
file/HTTP URLs, query variants and missing IDs fail. Reading content grants no
installation, semantic adoption, execution or provider access. Existing HTTP host,
origin and transport policies apply unchanged; publishing this crate deploys no
HTTP service or external connector.

The 0.2.0 API/config extension preserves existing v1 wire behavior. Actual consumers
must adopt the released runtime before generating v2 configuration; an old validator
must not be bypassed by squeezing full text into the summary field. Release and
independent stdio consumer acceptance are required before issue #24 closes.

Protocol reference: https://modelcontextprotocol.io/specification/2025-06-18/server/resources
