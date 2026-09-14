# AGENTS.md

本檔只記錄 `queue-aware-vgpu` 的長期開發規則；

## Dev notes(開發注意事項)
- 具體研究與邊界：[研究範圍與邊界](docs/dev/research-scope-and-boundaries.md)
- 模組設計原則：[Deep Module Design](docs/dev/deep-module-design.md)

## Design limits

- function 目標不超過 30 行；超過 50 行必須拆分或說明原因。
- domain source file 目標不超過 120 行；超過 180 行必須拆分。
- adapter / orchestrator source file 不得超過 220 行。
- 一個檔案只處理一個主要概念；不要建立無明確責任的 `utils` / `helpers`。
- source 超過 hard limit 時，先切責任再加功能。

## Rust rules

- 優先用 module 組織 code；只有需要獨立編譯、依賴隔離、重用或穩定 API 時才拆 crate。
- `main` 保持薄，只負責 startup、config 與 dependency wiring。
- 研究邏輯優先寫成可單獨測試的 pure function 或 small struct。
- 不為未使用的需求加入 generic trait、plugin system 或 framework。
- 預設使用 `pub(crate)` 或 private。

## Verification

Rust 變更依序執行：

```text
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
```

只有 deployment / Kubernetes behavior 改變時，才額外執行 smoke test、deployment
或 experiment。測試優先覆蓋 policy boundary、scoring、state transition 與
error/fallback behavior。
