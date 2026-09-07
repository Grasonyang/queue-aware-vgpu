use crate::hami::{allocated_memory_mib, pod_gpu_request, pod_memory_request_mib};
use chrono::{DateTime, Utc};
use k8s_openapi::api::core::v1::{Node, Pod};
use kube::ResourceExt;
use std::collections::BTreeMap;

pub const QUEUE_ANNOTATION: &str = "queue-aware-vgpu.io/queue";
pub const ADMISSION_GATE: &str = "queue-aware-vgpu.io/admission";

#[derive(Clone, Debug)]
pub struct WorkloadView {
    pub namespace: String,
    pub name: String,
    pub queue: Option<String>,
    pub requested_memory_mib: u64,
    pub gpu_request: u64,
    pub running: bool,
    pub pending: bool,
    pub waiting_since: DateTime<Utc>,
    pub hami_allocated_memory_mib: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ClusterView {
    pub total_memory_mib: u64,
    pub free_memory_mib: u64,
    pub total_replicas: u64,
    pub workloads: Vec<WorkloadView>,
    pub allocated_by_queue: BTreeMap<String, u64>,
    pub running_by_queue: BTreeMap<String, u32>,
    pub pending_by_queue: BTreeMap<String, u32>,
}

impl ClusterView {
    pub fn from_objects(nodes: &[Node], pods: &[Pod], default_request_memory_mib: u64) -> Self {
        /*
        讀取 Node
         → 加總 GPU memory / GPU replica

         讀取 Pod
         → 判斷 queue
         → 判斷 pending / running
         → 讀取 GPU memory request
         → 讀取 HAMi allocation

         最後算出：
         → total capacity
         → allocated capacity
         → free capacity
         → 每個 queue 的 workload 數量
        */
        let mut view = Self::default();
        for node in nodes {
            let capacity = crate::hami::node_capacity(node);
            view.total_memory_mib = view.total_memory_mib.saturating_add(capacity.memory_mib);
            view.total_replicas = view.total_replicas.saturating_add(capacity.replicas);
        }

        for pod in pods {
            let workload = workload_from_pod(pod, default_request_memory_mib);
            if workload.gpu_request == 0 && workload.queue.is_none() {
                continue;
            }
            let queue = workload
                .queue
                .as_deref()
                .map(|queue| {
                    if queue.contains('/') {
                        queue.to_string()
                    } else {
                        format!("{}/{}", workload.namespace, queue)
                    }
                })
                .unwrap_or_else(|| format!("{}/default", workload.namespace));
            if workload.running {
                *view.running_by_queue.entry(queue.clone()).or_default() += 1;
                *view.allocated_by_queue.entry(queue).or_default() += workload
                    .hami_allocated_memory_mib
                    .max(workload.requested_memory_mib);
            } else if workload.pending {
                *view.pending_by_queue.entry(queue).or_default() += 1;
            }
            view.workloads.push(workload);
        }

        let allocated: u64 = view.allocated_by_queue.values().sum();
        view.free_memory_mib = view.total_memory_mib.saturating_sub(allocated);
        view
    }
}

pub fn workload_from_pod(pod: &Pod, default_request_memory_mib: u64) -> WorkloadView {
    let phase = pod
        .status
        .as_ref()
        .and_then(|status| status.phase.as_deref())
        .unwrap_or("Pending");
    let namespace = pod.namespace().unwrap_or_else(|| "default".to_string());
    let name = pod.name_any();
    let queue = pod
        .metadata
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get(QUEUE_ANNOTATION))
        .cloned();
    let waiting_since = pod
        .metadata
        .creation_timestamp
        .as_ref()
        .map(|time| time.0)
        .unwrap_or_else(Utc::now);

    WorkloadView {
        namespace,
        name,
        queue,
        requested_memory_mib: pod_memory_request_mib(pod, default_request_memory_mib),
        gpu_request: pod_gpu_request(pod),
        running: phase == "Running",
        pending: phase == "Pending",
        waiting_since,
        hami_allocated_memory_mib: allocated_memory_mib(pod),
    }
}

pub fn has_admission_gate(pod: &Pod) -> bool {
    pod.spec
        .as_ref()
        .and_then(|spec| spec.scheduling_gates.as_ref())
        .is_some_and(|gates| gates.iter().any(|gate| gate.name == ADMISSION_GATE))
}
