use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{env, fs, net::SocketAddr, path::Path};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PolicyMode {
    Passthrough,
    #[default]
    Queue,
    Fragmentation,
    Adaptive,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub mode: PolicyMode,
    pub lookahead: usize,
    pub starvation_timeout_secs: u64,
    pub reconcile_interval_secs: u64,
    pub safety_margin_mib: u64,
    pub default_request_memory_mib: u64,
    pub overcommit_step: f64,
    pub overcommit_cooldown_secs: u64,
    pub min_overcommit: f64,
    pub max_overcommit: f64,
    pub metrics_bind_address: String,
    pub prometheus_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: PolicyMode::Queue,
            lookahead: 8,
            starvation_timeout_secs: 120,
            reconcile_interval_secs: 5,
            safety_margin_mib: 512,
            default_request_memory_mib: 4096,
            overcommit_step: 0.05,
            overcommit_cooldown_secs: 45,
            min_overcommit: 1.0,
            max_overcommit: 1.2,
            metrics_bind_address: "0.0.0.0:8080".to_string(),
            prometheus_url: None,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = env::var("CONFIG_PATH")
            .unwrap_or_else(|_| "/etc/queue-aware-vgpu/config.yaml".to_string());
        if !Path::new(&path).exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(&path).with_context(|| format!("read config {path}"))?;
        serde_yaml::from_str(&raw).with_context(|| format!("parse config {path}"))
    }

    pub fn metrics_addr(&self) -> Result<SocketAddr> {
        self.metrics_bind_address
            .parse()
            .with_context(|| format!("invalid metrics bind address {}", self.metrics_bind_address))
    }
}
