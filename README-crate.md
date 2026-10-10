# zixcel-mcp-gateway

## Problem / 課題
Applications need to expose approved public metadata to MCP clients without adding product authentication, network access, commands, or arbitrary file resources.
アプリケーションの承認済み公開メタデータを、製品認証や外部通信、コマンド実行、任意ファイル参照を追加せず MCP クライアントへ提供します。

## Solution / 解決
This Rust library and stdio executable project a validated, closed TOML catalog through a fixed read-only MCP surface. Configuration files are bounded to 64 KiB, catalogs to 64 resources, and stdio input lines to 1 MiB. The declared protocol version is `2025-06-18`; compatibility with other protocol versions is not established.
閉じた TOML スキーマを検証し、読み取り専用の MCP インターフェースへ投影します。設定ファイルは 64 KiB、リソースは 64 件、標準入力の各行は 1 MiB までです。対応プロトコルは `2025-06-18` として宣言され、他の版との互換性は未確認です。

## Usage / 使用方法
Version 0.1.0 is available from crates.io. Run the downloaded source with:
0.1.0 は crates.io で公開済みです。取得した配布ソースから実行できます。

```sh
cargo run --locked -- validate-config --config examples/local.toml
cargo run --locked -- --config examples/local.toml
```

The example contains fictional public metadata. MCP clients communicate with the second command over newline-delimited JSON on stdin/stdout. See [Rust usage](docs/rust.md) and [catalog schema](schema/public-catalog.schema.json).
例は架空の公開メタデータです。2 番目のコマンドへ標準入出力で改行区切り JSON を送信します。Rust の使用方法とカタログスキーマも同梱しています。

## Result / 結果
Clients can enumerate public tools and resources and read approved summaries. Product identity, authorization and private data stay in the consuming application; they are not dependencies of this crate.
クライアントは公開ツールとリソースを列挙し、承認済みの概要を取得できます。製品の ID・認可・非公開データは利用側の責務です。

## License / ライセンス
Apache-2.0. Retained prior license text and attribution are included in LICENSE-MIT and NOTICE.
Apache-2.0。過去のライセンス文と帰属表示は LICENSE-MIT と NOTICE に保持しています。

## Implemented contract and pending verification

This follow-up candidate checks the declared `2025-06-18` input shapes:
required initialize fields and client identity/capabilities, object `_meta`
on implemented methods, and each tool's declared object arguments. Validation
must be tied to this candidate's exact source tree. Results for earlier source
bytes do not verify this change.

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
