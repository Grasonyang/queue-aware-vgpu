# 09｜C++ 開發者的最小 Rust 背景

這頁只解釋閱讀本專案需要的概念。

| Rust | C++ 暫時類比 |
|---|---|
| `struct` | `struct` / data class |
| `impl Config` | `Config::method()` 的實作區塊 |
| `trait` | interface / concept |
| `Option<T>` | `std::optional<T>` |
| `Result<T, E>` | `std::expected<T, E>` |
| `pub mod` | 公開 module / namespace |
| `String` | `std::string` |
| `&str` | 借用的字串 view |
| `BTreeMap` | `std::map` |

## 目前常見的幾個寫法

```rust
let value = maybe_value.unwrap_or(default);
```

沒有值就用預設值。

```rust
let value = maybe_value.unwrap_or_else(|| build_default());
```

需要時才建立預設值。

```rust
let value = fallible_operation()?;
```

成功取值；失敗就把錯誤往上一層傳。

```rust
items.iter().map(|item| transform(item)).collect()
```

逐項轉換，再收集成新的容器。

```rust
impl Default for Config { ... }
```

宣告 `Config` 提供標準的預設值建立方式。

先把這些當成閱讀工具，不需要先完整學完 Rust 才能理解 controller 的系統流程。
