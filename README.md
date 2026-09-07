# Queue-aware vGPU

研究型 prototype：在 Kubernetes Scheduling Gate 之前做 queue-aware admission，讓 Kubernetes scheduler 與 HAMi 繼續負責 Pod placement、vGPU allocation 與 isolation。

## Start Here

如果你第一次看這個專案，請先讀 [分層文件導覽](docs/README.md)，不要直接從 Rust 原始碼開始。文件按「平台 → Pod gate → controller → policy → code → 復現 → 實驗」分層，每頁控制在 100 行內並包含 Mermaid 圖。

GitHub Actions 會檢查文件長度、code fence 和本機路徑；`main` 更新時會把 `docs/` 同步到 GitHub Wiki。Wiki workflow 預設使用 `GITHUB_TOKEN`，若 repository 的 Wiki 需要額外權限，請設定 `WIKI_TOKEN` secret。

## Goal

比較四種可重現 policy：`passthrough`、`queue`、`fragmentation`、`adaptive`。本版只建立一個 Rust crate、一個 controller binary、一個 `VGPUQueue` CRD；沒有 custom scheduler、admission webhook、資料庫或第二個 persistent state store。

## Architecture

```text
DGX Spark / NVIDIA GB10
└── k3d: gx10
    ├── k3s default scheduler
    ├── HAMi 2.10.0 (hami-scheduler + device plugin + HAMi-core)
    ├── queue-aware-vgpu-controller (Rust)
    │   ├── queue fairness
    │   ├── fragmentation-aware admission
    │   ├── conservative adaptive policy
    │   └── Prometheus metrics
    ├── Prometheus Operator stack
    └── experiments/

Pod with gate → controller admission → gate removed → hami-scheduler → GPU Pod
```

The controller does not replace Kubernetes' default scheduler or HAMi's scheduler. The HAMi chart's auxiliary `hami-scheduler` profile handles GPU Pods; the cluster default scheduler remains unchanged.

## Environment

The verified target is an arm64 DGX Spark / ASUS GX10 with NVIDIA GB10, Docker, k3d and Kubernetes 1.35.5. GB10 uses unified memory, so HAMi is configured with `preConfiguredDeviceMemory: 131072` and `deviceSplitCount: 10`; the observed node allocatable is `nvidia.com/gpu=10`.

The actual HAMi interface used by this repo is:

- GPU replica: `nvidia.com/gpu`
- GPU memory: `nvidia.com/gpumem`
- GPU core: `nvidia.com/gpucores`
- registration: `hami.io/node-nvidia-register`
- allocation: `hami.io/vgpu-devices-allocated`

HAMi-specific parsing is isolated in [`src/hami.rs`](/home/grason/Documents/queue-aware-vgpu/src/hami.rs).

## Repository Structure

- `src/`: Rust controller and pure policy modules.
- `deploy/`: k3d GPU image, HAMi, controller and monitoring manifests.
- `scripts/`: idempotent build, deploy, smoke and experiment scripts.
- `experiments/`: queues, gated workloads and burst scenarios.
- `results/`: ignored local CSV/JSON experiment output.

## Prerequisites

Native arm64 build is preferred on DGX Spark. Install Docker, k3d, kubectl, Helm, jq and Rust. Scripts refuse to create a cluster unless explicitly asked; `--recreate` is the only k3d destructive path.

## Deploy

For a new local cluster:

```bash
scripts/build-k3d-gpu-image.sh
scripts/deploy-k3d.sh --create --cluster gx10 \
  --image queue-aware-vgpu/k3s-gpu:v1.35.5-k3s1
scripts/deploy-hami.sh --cluster gx10 --node k3d-gx10-agent-0
scripts/deploy-prometheus.sh --cluster gx10
scripts/deploy-controller.sh --cluster gx10 --build
```

For the existing `gx10`, omit `--create`; use `scripts/deploy-controller.sh --build` when the Rust image changes. Prometheus is installed without Grafana, and the controller `ServiceMonitor` is enabled when Prometheus Operator CRDs exist.

## Create Queues and Run Workload

```bash
kubectl --context k3d-gx10 apply -f experiments/queues/research-queues.yaml
scripts/smoke-test.sh --cluster gx10
```

A workload selects a queue with `queue-aware-vgpu.io/queue: interactive` or `batch` and opts into admission with:

```yaml
spec:
  schedulingGates:
    - name: queue-aware-vgpu.io/admission
```

The smoke test verifies the CRD, controller, gate removal, `hami-scheduler`, HAMi allocation and `NVIDIA GB10` output.

## Experiments

The burst scenario submits small/large/small/medium/large/small jobs. Requests are 8192/16384/32768 MiB, or 6.25%/12.5%/25% of the configured 131072 MiB GB10 reference capacity.

```bash
scripts/run-experiment.sh --mode passthrough
scripts/run-experiment.sh --mode queue
scripts/run-experiment.sh --mode fragmentation
scripts/run-experiment.sh --mode adaptive \
  --prometheus-url http://monitoring-kube-prometheus-prometheus.monitoring.svc:9090
```

Each run writes JSON and CSV containing submit, creation, admission, start and completion timestamps, queue, request and phase. `results/` is intentionally git-ignored. Use the same scenario and cluster for baseline/ablation comparisons.

## Metrics

The controller serves `/metrics` on port 8080 and exports `queue_vgpu_pending_jobs`, `queue_vgpu_running_jobs`, `queue_vgpu_allocated_memory_mib`, `queue_vgpu_admissions_total`, `queue_vgpu_admission_wait_seconds`, `queue_vgpu_policy_decisions_total`, `queue_vgpu_fragmentation_score`, `queue_vgpu_overcommit_ratio`, and `queue_vgpu_errors_total`.

The Prometheus scrape was verified through the generated ServiceMonitor: the target was `up` and `queue_vgpu_running_jobs` was queryable.

## Design and Limitations

Weighted fairness uses allocated GPU memory divided by queue weight, with waiting-age starvation protection. Fragmentation uses a configurable lookahead (default 8), safety margin (default 512 MiB), and smallest legal remainder. A candidate waiting more than 120 seconds cannot be bypassed by fragmentation.

Adaptive mode changes only logical admission capacity. It moves in 0.05 steps between 1.0 and 1.2 with a 45-second cooldown. Prometheus feedback checks Kubernetes memory pressure, OOMKilled termination and recent restart signals. A missing/unavailable Prometheus endpoint resets the ratio to 1.0; pressure blocks new admissions and decreases the ratio. The tested HAMi version does not expose a verified runtime knob for changing CUDA/HAMi enforcement, so this prototype never patches or restarts HAMi components.

Governance is admission-first: pressure holds new Pods and never deletes running Pods. There is no eviction manager in this milestone. GB10 memory is configured rather than read from NVML because the tested path does not report it as ordinary discrete GPU memory. No DCGM exporter is assumed; metric names must be verified before adding GPU telemetry.

State is reconstructed from Kubernetes and HAMi annotations after restart. The controller is intentionally single-replica without leader election, and queue objects are namespaced.

## Research Roadmap

1. Repeat baseline/ablation scenarios and calculate mean/P95 wait, throughput, utilization, fairness, OOM and restart rate from CSV/Prometheus data.
2. Add a verified GB10 telemetry source without hardcoding unavailable DCGM metrics.
3. Validate logical overcommit against a HAMi release with a safe runtime control.
4. Study opt-in eviction only for explicitly marked experimental workloads.
