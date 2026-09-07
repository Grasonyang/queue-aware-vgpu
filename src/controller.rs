use crate::{
    config::{Config, PolicyMode},
    crd::{VGPUQueue, VGPUQueueStatus},
    metrics::Metrics,
    policy::{
        fragmentation::{Candidate, choose_best_fit},
        overcommit::{Decision, DecisionInput, PressureSignal, decide},
        queue::{QueueLoad, choose_queue, within_queue_limits},
    },
    state::{ADMISSION_GATE, ClusterView, WorkloadView, has_admission_gate},
};
use anyhow::{Context as AnyhowContext, Result, anyhow};
use chrono::Utc;
use futures::StreamExt;
use k8s_openapi::api::core::v1::{Node, Pod, PodSchedulingGate};
use kube::{
    Client, ResourceExt,
    api::{Api, ListParams, Patch, PatchParams},
    runtime::{
        controller::{Action, Controller},
        watcher,
    },
};
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};

#[derive(Debug, thiserror::Error)]
pub enum ControllerError {
    #[error(transparent)]
    Kubernetes(#[from] kube::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub struct ControllerContext {
    pub client: Client,
    pub config: Arc<RwLock<Config>>,
    pub metrics: Metrics,
    pub overcommit: Mutex<OvercommitRuntime>,
    reconcile_lock: Mutex<()>,
}

pub struct OvercommitRuntime {
    pub ratio: f64,
    pub last_change: Instant,
}

impl ControllerContext {
    pub fn new(client: Client, config: Config, metrics: Metrics) -> Self {
        let ratio = config.min_overcommit.clamp(1.0, config.max_overcommit);
        Self {
            client,
            config: Arc::new(RwLock::new(config)),
            metrics,
            overcommit: Mutex::new(OvercommitRuntime {
                ratio,
                last_change: Instant::now(),
            }),
            reconcile_lock: Mutex::new(()),
        }
    }
}

pub async fn run(ctx: Arc<ControllerContext>) -> Result<()> {
    let pods = Api::<Pod>::all(ctx.client.clone());
    let queues = Api::<VGPUQueue>::all(ctx.client.clone());
    let pod_ctx = ctx.clone();
    let queue_ctx = ctx.clone();

    tokio::spawn(async move {
        Controller::new(pods, watcher::Config::default())
            .run(reconcile_pod, error_policy, pod_ctx)
            .for_each(|result| async move {
                if let Err(error) = result {
                    warn!(%error, "pod controller stream error");
                }
            })
            .await;
    });

    tokio::spawn(async move {
        Controller::new(queues, watcher::Config::default())
            .run(reconcile_queue, error_policy, queue_ctx)
            .for_each(|result| async move {
                if let Err(error) = result {
                    warn!(%error, "queue controller stream error");
                }
            })
            .await;
    });

    let mut ticker = tokio::time::interval(Duration::from_secs(
        ctx.config.read().await.reconcile_interval_secs.max(1),
    ));
    loop {
        ticker.tick().await;
        if let Err(error) = reconcile_all(ctx.clone()).await {
            ctx.metrics.error("reconcile");
            error!(%error, "periodic reconcile failed");
        }
    }
}

async fn reconcile_pod(
    _pod: Arc<Pod>,
    ctx: Arc<ControllerContext>,
) -> Result<Action, ControllerError> {
    reconcile_all(ctx.clone()).await?;
    Ok(Action::requeue(Duration::from_secs(
        ctx.config.read().await.reconcile_interval_secs.max(1),
    )))
}

async fn reconcile_queue(
    _queue: Arc<VGPUQueue>,
    ctx: Arc<ControllerContext>,
) -> Result<Action, ControllerError> {
    reconcile_all(ctx.clone()).await?;
    Ok(Action::requeue(Duration::from_secs(
        ctx.config.read().await.reconcile_interval_secs.max(1),
    )))
}

fn error_policy<T: ResourceExt>(
    object: Arc<T>,
    error: &ControllerError,
    ctx: Arc<ControllerContext>,
) -> Action {
    ctx.metrics.error("watch");
    warn!(name = %object.name_any(), %error, "controller reconcile error");
    Action::requeue(Duration::from_secs(10))
}

pub async fn reconcile_all(ctx: Arc<ControllerContext>) -> Result<()> {
    let _guard = ctx.reconcile_lock.lock().await;
    let config = ctx.config.read().await.clone();
    let queues_api = Api::<VGPUQueue>::all(ctx.client.clone());
    let pods_api = Api::<Pod>::all(ctx.client.clone());
    let nodes_api = Api::<Node>::all(ctx.client.clone());
    let queues = queues_api.list(&ListParams::default()).await?.items;
    let pods = pods_api.list(&ListParams::default()).await?.items;
    let nodes = nodes_api.list(&ListParams::default()).await?.items;
    let view = ClusterView::from_objects(&nodes, &pods, config.default_request_memory_mib);

    let queue_by_key: BTreeMap<String, VGPUQueue> = queues
        .iter()
        .map(|queue| {
            (
                queue_key(
                    queue.namespace().as_deref().unwrap_or("default"),
                    &queue.name_any(),
                ),
                queue.clone(),
            )
        })
        .collect();
    let mut loads = Vec::new();
    for (key, queue) in &queue_by_key {
        let pending_jobs = *view.pending_by_queue.get(key).unwrap_or(&0);
        let running_jobs = *view.running_by_queue.get(key).unwrap_or(&0);
        let allocated_memory_mib = *view.allocated_by_queue.get(key).unwrap_or(&0);
        let oldest_pending_since = view
            .workloads
            .iter()
            .filter(|workload| {
                workload.pending && workload_queue_key(workload, &queue_by_key) == Some(key.clone())
            })
            .map(|workload| workload.waiting_since)
            .min();
        let weight = queue.spec.weight;
        loads.push(QueueLoad {
            name: key.clone(),
            weight,
            allocated_memory_mib,
            running_jobs,
            pending_jobs,
            oldest_pending_since,
        });
        ctx.metrics.set_queue(
            key,
            pending_jobs as i64,
            running_jobs as i64,
            allocated_memory_mib as i64,
        );
        ctx.metrics.set_overcommit(key, config.min_overcommit);
        ctx.metrics
            .set_fragmentation(key, fragmentation_score(&view, key, &queue_by_key));
    }

    let pending_total = view.pending_by_queue.values().copied().sum();
    let (ratio, pressure) = current_overcommit(&ctx, &config, pending_total).await;
    if matches!(
        pressure,
        PressureSignal::Oom | PressureSignal::MemoryPressure | PressureSignal::AbnormalRestart
    ) {
        ctx.metrics.admission("__system__", "blocked-pressure");
        warn!(
            ?pressure,
            "holding new vGPU admissions under resource pressure"
        );
        update_statuses(&ctx.client, &queue_by_key, &view, ratio).await?;
        return Ok(());
    }
    for key in queue_by_key.keys() {
        ctx.metrics.set_overcommit(key, ratio);
    }
    update_statuses(&ctx.client, &queue_by_key, &view, ratio).await?;
    let queue_choice = match config.mode {
        PolicyMode::Passthrough => loads
            .iter()
            .find(|load| load.pending_jobs > 0)
            .map(|load| load.name.clone()),
        _ => choose_queue(&loads, Utc::now(), config.starvation_timeout_secs).map(|choice| {
            ctx.metrics.decision(&choice.queue, "weighted-fair");
            choice.queue
        }),
    };

    let Some(queue_key) = queue_choice else {
        return Ok(());
    };
    let Some(queue) = queue_by_key.get(&queue_key) else {
        return Ok(());
    };
    let queue_ratio = if queue.spec.overcommit.enabled {
        ratio.clamp(
            queue.spec.overcommit.min_ratio,
            queue.spec.overcommit.max_ratio,
        )
    } else {
        1.0
    };
    let mut candidates: Vec<_> = view
        .workloads
        .iter()
        .filter(|workload| workload.pending && has_admission_gate_for_view(workload, &pods))
        .filter(|workload| {
            workload_queue_key(workload, &queue_by_key).as_deref() == Some(queue_key.as_str())
        })
        .collect();
    candidates.sort_by_key(|workload| workload.waiting_since);
    if candidates.is_empty() {
        return Ok(());
    }

    let selected = if matches!(
        config.mode,
        PolicyMode::Fragmentation | PolicyMode::Adaptive
    ) {
        let fragmentation_candidates: Vec<_> = candidates
            .iter()
            .map(|workload| Candidate {
                id: format!("{}/{}", workload.namespace, workload.name),
                requested_memory_mib: workload.requested_memory_mib,
                waiting_since: workload.waiting_since,
            })
            .collect();
        choose_best_fit(
            effective_free_memory(
                view.total_memory_mib,
                view.allocated_by_queue.values().sum(),
                queue_ratio,
            ),
            &fragmentation_candidates,
            config.lookahead,
            config.safety_margin_mib,
            config.starvation_timeout_secs,
            Utc::now(),
        )
        .and_then(|selection| {
            candidates.into_iter().find(|candidate| {
                format!("{}/{}", candidate.namespace, candidate.name) == selection.id
            })
        })
    } else {
        candidates.into_iter().next()
    };

    let Some(selected) = selected else {
        ctx.metrics.admission(&queue_key, "blocked-fragmentation");
        return Ok(());
    };
    let allocated = *view.allocated_by_queue.get(&queue_key).unwrap_or(&0);
    let running = *view.running_by_queue.get(&queue_key).unwrap_or(&0);
    let free_memory = effective_free_memory(
        view.total_memory_mib,
        view.allocated_by_queue.values().sum(),
        queue_ratio,
    );
    let free_replicas = view.total_replicas.saturating_sub(
        view.workloads
            .iter()
            .filter(|workload| workload.running)
            .map(|workload| workload.gpu_request)
            .sum(),
    );
    let within_queue_limits = within_queue_limits(
        &QueueLoad {
            name: queue_key.clone(),
            weight: queue.spec.weight,
            allocated_memory_mib: allocated,
            running_jobs: running,
            pending_jobs: 0,
            oldest_pending_since: None,
        },
        queue.spec.max_running,
        queue.spec.max_memory_mib,
        selected.requested_memory_mib,
    );
    let fits = selected.requested_memory_mib <= free_memory
        && pod_request_or_one(selected.gpu_request) <= free_replicas;

    if !within_queue_limits || !fits {
        ctx.metrics.admission(&queue_key, "blocked-capacity");
        debug!(queue = %queue_key, pod = %selected.name, "pod remains gated");
        return Ok(());
    }

    let admitted_at = Utc::now();
    remove_admission_gate(
        &ctx.client,
        &selected.namespace,
        &selected.name,
        &pods,
        &admitted_at.to_rfc3339(),
    )
    .await?;
    let wait = (admitted_at - selected.waiting_since).num_seconds().max(0) as f64;
    ctx.metrics.admission(&queue_key, "admitted");
    ctx.metrics.observe_wait(&queue_key, wait);
    info!(queue = %queue_key, pod = %selected.name, memory_mib = selected.requested_memory_mib, "admitted gated vGPU pod");
    Ok(())
}

async fn remove_admission_gate(
    client: &Client,
    namespace: &str,
    name: &str,
    pods: &[Pod],
    admitted_at: &str,
) -> Result<()> {
    let pod = pods
        .iter()
        .find(|pod| pod.namespace().as_deref() == Some(namespace) && pod.name_any() == name)
        .ok_or_else(|| anyhow!("pod {namespace}/{name} disappeared before patch"))?;
    let gates: Vec<PodSchedulingGate> = pod
        .spec
        .as_ref()
        .and_then(|spec| spec.scheduling_gates.clone())
        .unwrap_or_default()
        .into_iter()
        .filter(|gate| gate.name != ADMISSION_GATE)
        .collect();
    let api = Api::<Pod>::namespaced(client.clone(), namespace);
    api.patch(
        name,
        &PatchParams::default(),
        &Patch::Merge(json!({
            "metadata": {"annotations": {"queue-aware-vgpu.io/admitted-at": admitted_at}},
            "spec": {"schedulingGates": gates}
        })),
    )
    .await
    .with_context(|| format!("remove admission gate from {namespace}/{name}"))?;
    Ok(())
}

async fn update_statuses(
    client: &Client,
    queues: &BTreeMap<String, VGPUQueue>,
    view: &ClusterView,
    ratio: f64,
) -> Result<()> {
    for (key, queue) in queues {
        let pending_jobs = *view.pending_by_queue.get(key).unwrap_or(&0);
        let running_jobs = *view.running_by_queue.get(key).unwrap_or(&0);
        let allocated_memory_mib = *view.allocated_by_queue.get(key).unwrap_or(&0);
        let unchanged = queue.status.as_ref().is_some_and(|current| {
            current.pending_jobs == pending_jobs
                && current.running_jobs == running_jobs
                && current.allocated_memory_mib == allocated_memory_mib
                && (current.overcommit_ratio - ratio).abs() < f64::EPSILON
        });
        if unchanged {
            continue;
        }
        let status = VGPUQueueStatus {
            pending_jobs,
            running_jobs,
            allocated_memory_mib,
            overcommit_ratio: ratio,
            observed_at: Some(Utc::now().to_rfc3339()),
            conditions: Vec::new(),
        };
        let namespace = queue.namespace().unwrap_or_else(|| "default".to_string());
        Api::<VGPUQueue>::namespaced(client.clone(), &namespace)
            .patch_status(
                &queue.name_any(),
                &PatchParams::default(),
                &Patch::Merge(json!({"status": status})),
            )
            .await?;
    }
    Ok(())
}

async fn current_overcommit(
    ctx: &ControllerContext,
    config: &Config,
    pending_jobs: u32,
) -> (f64, PressureSignal) {
    let mut state = ctx.overcommit.lock().await;
    if !matches!(config.mode, PolicyMode::Adaptive) {
        state.ratio = 1.0;
        return (state.ratio, PressureSignal::Safe);
    }
    let Some(prometheus_url) = config.prometheus_url.as_deref() else {
        state.ratio = 1.0;
        return (state.ratio, PressureSignal::Unknown);
    };
    let signal = prometheus_pressure_signal(prometheus_url).await;
    if signal == PressureSignal::Unknown {
        ctx.metrics.error("prometheus");
        state.ratio = 1.0;
        return (state.ratio, signal);
    }
    let cooldown_elapsed =
        state.last_change.elapsed() >= Duration::from_secs(config.overcommit_cooldown_secs);
    let decision = decide(DecisionInput {
        current_ratio: state.ratio,
        min_ratio: config.min_overcommit,
        max_ratio: config.max_overcommit,
        step: config.overcommit_step,
        pending_jobs,
        signal,
        cooldown_elapsed,
    });
    match decision {
        Decision::Increase { ratio } | Decision::Decrease { ratio } => {
            state.ratio = ratio;
            state.last_change = Instant::now();
        }
        Decision::Hold { ratio } => state.ratio = ratio,
    }
    (state.ratio, signal)
}

#[derive(Debug, Deserialize)]
struct PrometheusResponse {
    status: String,
    data: PrometheusData,
}

#[derive(Debug, Deserialize)]
struct PrometheusData {
    result: Vec<PrometheusResult>,
}

#[derive(Debug, Deserialize)]
struct PrometheusResult {
    value: Option<Vec<serde_json::Value>>,
}

async fn prometheus_pressure_signal(base_url: &str) -> PressureSignal {
    let queries = [
        (
            PressureSignal::Oom,
            "sum(max_over_time(kube_pod_container_status_last_terminated_reason{reason=\"OOMKilled\"}[5m]))",
        ),
        (
            PressureSignal::MemoryPressure,
            "sum(kube_node_status_condition{condition=\"MemoryPressure\",status=\"true\"})",
        ),
        (
            PressureSignal::AbnormalRestart,
            "sum(increase(kube_pod_container_status_restarts_total[5m]))",
        ),
    ];
    let client = reqwest::Client::new();
    let mut successful_queries = 0;
    for (signal, query) in queries {
        let endpoint = format!("{}/api/v1/query", base_url.trim_end_matches('/'));
        let response = client
            .get(endpoint)
            .query(&[("query", query)])
            .timeout(Duration::from_secs(3))
            .send()
            .await;
        let Ok(response) = response else {
            continue;
        };
        let Ok(payload) = response.json::<PrometheusResponse>().await else {
            continue;
        };
        if payload.status != "success" {
            continue;
        }
        successful_queries += 1;
        if payload
            .data
            .result
            .iter()
            .filter_map(|result| result.value.as_ref())
            .filter_map(|value| value.get(1))
            .filter_map(|value| value.as_str().and_then(|value| value.parse::<f64>().ok()))
            .any(|value| value > 0.0)
        {
            return signal;
        }
    }
    if successful_queries == queries.len() {
        PressureSignal::Safe
    } else {
        PressureSignal::Unknown
    }
}

fn effective_free_memory(total: u64, allocated: u64, ratio: f64) -> u64 {
    ((total as f64 * ratio).max(0.0) as u64).saturating_sub(allocated)
}

fn fragmentation_score(
    view: &ClusterView,
    queue_key: &str,
    queues: &BTreeMap<String, VGPUQueue>,
) -> f64 {
    let free = view
        .total_memory_mib
        .saturating_sub(view.allocated_by_queue.values().sum());
    if free == 0 {
        return 1.0;
    }
    let largest_fit = view
        .workloads
        .iter()
        .filter(|workload| workload.pending)
        .filter(|workload| workload_queue_key(workload, queues).as_deref() == Some(queue_key))
        .map(|workload| workload.requested_memory_mib)
        .filter(|request| *request <= free)
        .max()
        .unwrap_or(0);
    free.saturating_sub(largest_fit) as f64 / free as f64
}

fn queue_key(namespace: &str, name: &str) -> String {
    format!("{namespace}/{name}")
}

fn workload_queue_key(
    workload: &WorkloadView,
    queues: &BTreeMap<String, VGPUQueue>,
) -> Option<String> {
    let queue = workload.queue.as_deref()?;
    if queue.contains('/') {
        return queues.contains_key(queue).then(|| queue.to_string());
    }
    let key = queue_key(&workload.namespace, queue);
    queues.contains_key(&key).then_some(key)
}

fn has_admission_gate_for_view(workload: &WorkloadView, pods: &[Pod]) -> bool {
    pods.iter()
        .find(|pod| {
            pod.namespace().as_deref() == Some(workload.namespace.as_str())
                && pod.name_any() == workload.name
        })
        .is_some_and(has_admission_gate)
}

fn pod_request_or_one(request: u64) -> u64 {
    request.max(1)
}
