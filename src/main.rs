use anyhow::Result;
use axum::{
    Router,
    extract::State,
    http::{HeaderValue, StatusCode, header},
    response::Response,
    routing::get,
};
use queue_aware_vgpu_controller::{
    config::Config,
    controller::{self, ControllerContext},
    metrics::Metrics,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

/// Controller 的程式入口。
///
/// 這個 binary 同時做兩件事：
///
/// 1. 啟動 Kubernetes controller，監看 Pod 和 VGPUQueue。
/// 2. 啟動 HTTP `/metrics` endpoint，讓 Prometheus 可以抓取 metrics。
///
/// 真正的 queue / fragmentation / overcommit 決策不在這裡，
/// 而是在 `controller.rs` 和 `policy/` 裡。`main.rs` 主要負責組裝元件。
// `tokio::main` 會先建立 Tokio async runtime，然後才執行這個 async main。
// 可以把它想成：替 Rust 程式準備一個能同時等待 Kubernetes API、HTTP 和 signal
// 的工作環境；沒有它，async fn main 裡的 `.await` 沒有 runtime 可以執行。
#[tokio::main]
async fn main() -> Result<()> {
    // 從 RUST_LOG 讀取 log level，並輸出 JSON structured logs。
    // 例如 Deployment 裡設定 RUST_LOG=info，便會看到 admission decision。
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();

    // Config 來自 CONFIG_PATH 指向的 YAML；在 Kubernetes 裡是 ConfigMap mount。
    // 這裡包含 mode、lookahead、overcommit 等 system-level 設定。
    let config = Config::load()?;

    // Metrics 是 controller 共用的 Prometheus counter/gauge registry。
    // controller 更新它，HTTP handler 負責把目前數值輸出給 Prometheus。
    let metrics = Metrics::new()?;

    // 讀取 metrics_bind_address，例如 0.0.0.0:8080。
    // 這只決定 metrics HTTP server 綁在哪個位址，不是 Kubernetes API 位址。
    let address = config.metrics_addr()?;

    // 建立 Kubernetes API client。
    // kube::Client::try_default() 會使用 Pod 裡的 ServiceAccount；
    // 在本機執行時則通常使用 ~/.kube/config。
    let client = kube::Client::try_default().await?;

    // ControllerContext 是 controller reconcile 共用的狀態容器，包含：
    // - Kubernetes client
    // - system config
    // - Prometheus metrics
    // - adaptive overcommit 的 runtime state
    // Arc 讓 controller 裡的多個 async task 可以共用同一份 context，
    // 而不需要複製 Kubernetes client 或違反 Rust 的 ownership 規則。
    // Metrics server 使用的是前面 metrics.clone() 傳入的 Metrics registry，
    // 不是直接共用這個 Arc<ControllerContext>。
    let context = Arc::new(ControllerContext::new(client, config, metrics.clone()));

    // 建立很小的 Axum HTTP server，目前只提供 /metrics。
    // State(metrics) 會把共用的 Metrics registry 傳給 metrics_handler。
    let metrics_router = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(metrics);
    let listener = TcpListener::bind(address).await?;
    info!(%address, "metrics endpoint listening");

    // Metrics server 在背景 task 中執行，這樣 main 可以同時啟動 controller。
    let metrics_server = tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, metrics_router).await {
            error!(%error, "metrics server stopped");
        }
    });

    // 同時等待兩種事件：
    // - controller::run：持續 watch Kubernetes objects 並做 admission decision。
    // - ctrl_c：使用者停止程式時，結束 controller。
    // 任一個結束，main 都會離開，並停止 metrics server。
    tokio::select! {
        result = controller::run(context) => result?,
        result = tokio::signal::ctrl_c() => result?,
    }

    // controller 結束後，metrics server 也不需要繼續存在。
    metrics_server.abort();
    Ok(())
}

/// Prometheus scrape `/metrics` 時執行的 HTTP handler。
///
/// 成功時回傳 Prometheus text exposition format；
/// render 失敗時回傳 HTTP 500，讓 scrape failure 可以被監控到。
async fn metrics_handler(State(metrics): State<Metrics>) -> Response<String> {
    match metrics.render() {
        Ok(body) => Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; version=0.0.4"),
            )
            .body(body)
            .expect("valid metrics response"),
        Err(error) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(error.to_string())
            .expect("valid error response"),
    }
}
