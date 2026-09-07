# 00｜先建立全貌

## 一句話

本專案不是重寫 GPU scheduler，而是在 Kubernetes scheduler 之前加入 queue-aware admission。

```mermaid
flowchart LR
    A[研究 Pod] --> B[Scheduling Gate]
    B --> C[Rust controller]
    C -->|保留 gate| D[繼續等待]
    C -->|移除 gate| E[Kubernetes / HAMi scheduler]
    E --> F[GB10 vGPU workload]
```

## 四個名詞分工

| 名詞 | 它是什麼 | 它不做什麼 |
|---|---|---|
| `VGPUQueue` | queue policy 的資料 | 不直接分配 GPU |
| Scheduling Gate | 暫停 Pod 進入 scheduler 的開關 | 不決定 GPU placement |
| Rust controller | 決定何時移除 gate | 不取代 scheduler |
| HAMi | 實際 vGPU allocation / isolation | 不做 queue fairness |

## 一個 Pod 的生命週期

```mermaid
sequenceDiagram
    participant U as User
    participant K as Kubernetes API
    participant Q as Queue controller
    participant H as HAMi scheduler
    U->>K: 建立帶 queue + gate 的 Pod
    K-->>Q: Pod watch event
    Q->>Q: 讀取 Queue / Node / Pod state
    alt 資源或政策不允許
        Q-->>K: 保留 gate
    else 可以 admission
        Q->>K: 移除 admission gate
        K->>H: 進入 GPU scheduling path
        H-->>K: 配置 vGPU
    end
```

## 目前已完成

- gx10：k3d + k3s + GB10 GPU path。
- HAMi：`nvidia.com/gpu`、`gpumem`、`gpucores`。
- Rust controller：watch、admission、metrics。
- 一個 `VGPUQueue` CRD。
- queue、fragmentation、adaptive 四種研究 mode。
- Prometheus scrape、smoke test、CSV/JSON experiment output。

## 目前沒有做

- custom kube-scheduler
- admission webhook
- database / Redis / message queue
- HAMi-Core fork
- GPU workload eviction
- 真正修改 HAMi runtime hard limit 的 overcommit
