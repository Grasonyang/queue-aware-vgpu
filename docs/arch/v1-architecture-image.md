# V1 架構圖

主文件：[@v1.md](v1.md)。本檔只保存 Mermaid 圖與讀圖說明，詳細規則以主文件為準。
以下是目標設計，不是現有部署能力宣告；不引入額外的服務或資料庫。

## 1. 研究全景與第一版範圍

實線表示決策與執行方向，虛線表示觀測回饋。Rust 負責 Cluster 與 Job；
第一版 Pod 層使用既有 HAMi，HAMi-Core 開發僅在後續研究必要時進行。

```mermaid
flowchart TD
    clusterLayer["Cluster：固定額度與加權輪詢；後續 DQA"]
    jobLayer["Job：FIFO 與可行放置；後續 FWA／GFS／topology"]
    podLayer["Pod：HAMi 排程與配置；必要時整合 HAMi-Core 保護"]
    observation["跨層觀測：Prometheus 與執行紀錄"]

    clusterLayer -->|"額度與租戶調度次序"| jobLayer
    jobLayer -->|"GPU 放置限制與需求"| podLayer
    clusterLayer -.->|"治理結果"| observation
    jobLayer -.->|"等待與放置結果"| observation
    podLayer -.->|"實際配置與執行結果"| observation
    observation -.->|"後續研究的歷史輸入"| clusterLayer
    observation -.->|"資源觀測與後續研究輸入"| jobLayer
```

## 2. MVP 執行流程

圖中的 Rust 節點是同一程序內的 module，不是獨立部署的服務。
箭頭表示資料與決策流，不代表 domain module 直接呼叫 Kubernetes 或 HAMi。

```mermaid
flowchart TD
    ui["UI／實驗腳本"]
    resources["Kubernetes：DesiredGpuPod、租戶 Queue、Pod 與 GPU 觀測"]
    hami["HAMi：實際排程與資源配置"]
    gpuWork["GB10：固定記憶體配置、佔用與釋放"]

    subgraph rustBackend ["單一 Rust 後端"]
        application["application：預覽、提交與 reconcile"]
        clusterPolicy["cluster：固定額度與加權輪詢"]
        jobPolicy["job：租戶 FIFO、可行性與 GPU 放置"]
        workloadState["workload：等待、預留與生命週期"]
        adapter["Kubernetes／HAMi adapter：觀測與執行動作"]

        application -->|"租戶快照與輪詢進度"| clusterPolicy
        clusterPolicy -->|"額度與租戶次序"| jobPolicy
        jobPolicy -->|"放置決策或等待原因"| workloadState
        workloadState -->|"下一狀態與待執行動作"| application
        application -->|"讀取觀測或執行動作"| adapter
        adapter -.->|"正規化觀測"| application
    end

    ui -->|"預覽、提交、查詢"| application
    application -.->|"高亮結果與需求狀態"| ui
    adapter -->|"寫入需求與狀態；安排時建立 Pod"| resources
    resources -.->|"watch 與結果觀測"| adapter
    resources -->|"交付帶有 Rust 放置限制的 Pod"| hami
    hami -->|"配置 GPU 執行資源"| gpuWork
    gpuWork -.->|"配置回報與 Pod 結果"| resources
```

Cluster 治理 queue 決定租戶間的機會；租戶 queue 排列自己的 `DesiredGpuPod`。
兩者不重複保存需求。預覽只取決策結果，正式提交後的 reconcile 才能預留並建立執行 Pod。

## 3. DesiredGpuPod 生命週期

`Waiting` 包含排隊中與分配中；排隊中尚未預留，分配中已預留並追蹤同一次嘗試。
圖只呈現 UI 主狀態，內部分類與資源帳務見主文件。
不合法需求在正式接受前拒絕，不進入本狀態圖。

```mermaid
stateDiagram-v2
    direction TB

    state "Waiting：排隊中或分配中" as waiting
    state "Running：固定記憶體已配置成功" as running
    state "Succeeded：保留結果" as succeeded
    state "Failed：保留原因，不重跑" as failed
    state "TimedOut：保留逾時結果，不重跑" as timedOut

    [*] --> waiting: 合法需求正式接受
    waiting --> waiting: 暫時不足或等待配置結果
    waiting --> running: 確認配置成功
    waiting --> failed: 本次配置或工作明確失敗
    waiting --> timedOut: 限時到期且尚未完成配置
    running --> succeeded: 工作正常完成
    running --> failed: 工作明確失敗
    succeeded --> [*]
    failed --> [*]
    timedOut --> [*]
```

- 永久等待沒有逾時轉移；限時模式從正式接受起算，重啟或轉入分配中不重設期限。
- `Running` 以 workload 實際配置成功為準，不只是 Pod phase；之後另計固定佔用時間。
- API 結果不明先核對同一次嘗試，不能直接當成失敗或另建一個 Pod。
- 終止不等於立刻釋放額度；必須確認實際工作已停止或不再可能取得該份資源。
- 結束符號表示該次執行結束，不表示刪除 CR。手動重跑建立新 CR，不把終止狀態改回等待。

## 4. UI 預覽與正式提交

本圖只畫合法提交到成功執行的路徑；等待、失敗與逾時分支見上面的生命週期。
`rustBackend` 是前圖整個後端，不是另一個服務；配置與工作結果由後端觀測後更新 CR。

```mermaid
sequenceDiagram
    participant ui
    participant rustBackend
    participant kubernetes
    participant hami

    ui->>rustBackend: 預覽需求
    rustBackend->>kubernetes: 取得最新需求與資源觀測
    kubernetes-->>rustBackend: 快照
    rustBackend-->>ui: 可行性與原因，不寫入或預留
    ui->>rustBackend: 正式拖入執行區提交
    rustBackend->>kubernetes: 重新驗證後建立 DesiredGpuPod
    rustBackend-->>ui: 已接受，Waiting
    kubernetes-->>rustBackend: 需求與資源變化
    rustBackend->>rustBackend: 加權輪詢、FIFO 與放置評估
    rustBackend->>kubernetes: 可行時記錄預留並建立唯一執行 Pod
    kubernetes->>hami: 待排程 Pod 與 Rust 放置限制
    hami-->>kubernetes: 排程與資源配置狀態
    kubernetes-->>rustBackend: workload 配置回報與 Pod 狀態
    rustBackend->>kubernetes: 確認配置成功後更新 Running
    rustBackend-->>ui: 回傳需求最新狀態
```

高亮不是資源承諾，提交也不繞過 queue。需求合理但當下不足，仍由 Rust 在 `Waiting` 中安排；
UI 回彈只是顯示行為，不刪除需求或驅逐 Pod。
