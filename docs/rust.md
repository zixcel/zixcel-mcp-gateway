# Rust usage / Rust からの使用

## Problem / 課題
Embed the same validated catalog without spawning the executable.
実行ファイルを起動せず、検証済みカタログを利用します。

## Solution and usage / 解決と使用方法
The library exports `Config`, `Gateway` and `run_stdio`.
ライブラリは `Config`、`Gateway`、`run_stdio` を公開しています。

```rust
use zixcel_mcp_gateway::{Config, Gateway};
let config = Config::from_toml(include_str!("../examples/local.toml"))?;
let gateway = Gateway::new(config);
let response = gateway.handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"resources/list"}"#);
assert!(response.is_some());
# Ok::<(), String>(())
```

## Result / 結果
Responses contain the approved public catalog. Notifications produce no response. A consuming application must enforce its own identity, authorization and product policies.
承認済み公開カタログが応答され、通知には応答しません。利用アプリケーション自身が ID・認可・製品ポリシーを適用します。

`Config::load` applies bounded regular-file checks. `Config::from_toml` validates a supplied string; the embedding caller controls that string's allocation. `run_stdio` supplies the bounded line transport. These limits are application policy, not a claim of complete MCP SDK coverage.
`Config::load` はファイルサイズと通常ファイルの検査を適用します。`Config::from_toml` の文字列確保は呼び出し側の責務です。`run_stdio` は行長制限を適用します。

## Request IDs and bounds / 要求 ID と制限
String IDs and exact integral JSON numbers are accepted; `1.0`, `1e3`, and integers beyond 64 bits retain their original numeric representation in replies. Fractional numbers, objects, arrays, booleans and explicit null IDs are rejected. Notifications omit the ID and produce no reply. Numeric decimal exponents must fit the decimal library's signed 64-bit scale representation; unsupported values are rejected rather than rounded.
文字列 ID と厳密に整数値の JSON 数値を受理します。`1.0`、`1e3`、64 bit を超える整数も元の数値表記で応答します。小数・object・array・boolean・明示 null は拒否します。通知は ID を省略し、応答しません。十進指数は数値ライブラリの符号付き64 bit scale で表現可能な範囲に限られ、範囲外は丸めず拒否します。

`run_stdio` limits each complete input frame, including its newline, to 1 MiB. Direct `Gateway::handle_line` callers must apply their own frame admission limit. Parameter admission keeps a 128-level nesting budget; methods decode original JSON text directly into their final models. Very long decimal coefficients require substantially more CPU than ordinary IDs; the frame limit bounds input bytes and does not promise constant processing time. These bounds do not add a smaller numeric-ID length limit.
`run_stdio` は改行を含む入力フレーム全体を 1 MiB に制限します。`Gateway::handle_line` の直接呼び出し側はフレームの受入制限を適用してください。params の受入検査は128段のネスト上限を保持し、method は元の JSON 文字列から最終モデルへ直接デコードします。非常に長い十進係数は通常の ID より CPU 時間を要し、バイト上限は一定処理時間を保証しません。これより小さい数値 ID 長の制限は追加していません。

The official rmcp 3.5.0 ID model uses signed 64-bit integer or string variants. This gateway's exact decimal/exponent and larger numeric-ID support therefore does not imply that every rmcp peer accepts those forms. Ordinary string IDs avoid that numeric representation mismatch.
公式 rmcp 3.5.0 の ID モデルは符号付き64 bit 整数か文字列です。本 gateway の十進・指数表記や大きい数値 ID を全 rmcp peer が受理するとは限りません。通常の文字列 ID はその数値表現の差を避けられます。
