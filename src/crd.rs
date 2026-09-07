use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OvercommitSpec {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_min_ratio")]
    pub min_ratio: f64,
    #[serde(default = "default_max_ratio")]
    pub max_ratio: f64,
}

impl Default for OvercommitSpec {
    fn default() -> Self {
        Self {
            enabled: true,
            min_ratio: default_min_ratio(),
            max_ratio: default_max_ratio(),
        }
    }
}

#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "queue-aware-vgpu.io",
    version = "v1alpha1",
    kind = "VGPUQueue",
    namespaced,
    shortname = "vgpuq",
    status = "VGPUQueueStatus",
    printcolumn = r#"{"name":"Pending","type":"integer","jsonPath":".status.pendingJobs"}"#,
    printcolumn = r#"{"name":"Running","type":"integer","jsonPath":".status.runningJobs"}"#,
    printcolumn = r#"{"name":"Weight","type":"integer","jsonPath":".spec.weight"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct VGPUQueueSpec {
    #[serde(default = "default_weight")]
    pub weight: u32,
    pub max_running: Option<u32>,
    pub max_memory_mib: Option<u64>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub overcommit: OvercommitSpec,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VGPUQueueStatus {
    #[serde(default)]
    pub pending_jobs: u32,
    #[serde(default)]
    pub running_jobs: u32,
    #[serde(default)]
    pub allocated_memory_mib: u64,
    #[serde(default)]
    pub overcommit_ratio: f64,
    pub observed_at: Option<String>,
    #[serde(default)]
    pub conditions: Vec<QueueCondition>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct QueueCondition {
    pub r#type: String,
    pub status: String,
    pub reason: Option<String>,
    pub message: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_min_ratio() -> f64 {
    1.0
}

fn default_max_ratio() -> f64 {
    1.2
}

fn default_weight() -> u32 {
    1
}
