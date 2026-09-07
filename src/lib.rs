//! Queue-aware vGPU controller 的 library crate 根目錄。
//!
//! 這個檔案本身幾乎沒有執行邏輯，主要工作是把各個 Rust module
//! 宣告出來，讓 `main.rs` 和其他 module 可以使用它們。
//!
//! 對 C++ 來說，可以先把它想成「整個 library 對外公開的模組索引」：
//! - `src/config.rs` 實作 config module
//! - `src/controller.rs` 實作 controller module
//! - 其他 `pub mod` 依此類推
//!
//! `pub` 代表這個 module 可以被 library crate 外部使用。
//! 因此 binary `main.rs` 才能寫：
//! `use queue_aware_vgpu_controller::config::Config;`

// 系統設定：讀取 ConfigMap，包含 mode、lookahead、overcommit 等設定。
pub mod config;

// Kubernetes controller 的主控制流程：watch object、reconcile、admission。
pub mod controller;

// VGPUQueue CRD 的 Rust 型別，以及 Kubernetes CRD schema 相關定義。
pub mod crd;

// HAMi 專用的 resource name、annotation 和 GPU 註冊/分配格式解析。
pub mod hami;

// Prometheus counters、gauges 和 metrics 輸出。
pub mod metrics;

// 純 policy 函式：queue fairness、fragmentation、adaptive overcommit。
pub mod policy;

// 從 Kubernetes Pod、Node 和 HAMi annotation 重建目前 cluster state。
pub mod state;
