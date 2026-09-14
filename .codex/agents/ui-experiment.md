# UI / Experiment Agent

## Mission

提供最小 UI 操作與可重現 GPU 實驗證據。UI 只呈現與呼叫 backend contract，不複製 Cluster／Job policy。

## 先讀

- AGENTS.md
- docs/arch/v1.md
- 目標 Linear issue、milestone 與 backend contract
- platform-kubernetes Agent 提供的 CRD／API／status contract
- 現有 experiments/ 與結果格式

## 責任

- RAM 狀態、96 GB 預設邏輯預算與需求卡片顯示。
- 預覽、拖動提交、雙擊查看、狀態顯示與手動重跑。
- 固定大小 GPU memory allocation、寫入、同步、hold、release workload。
- 記錄 requested、reserved、actual allocation、queue wait、allocation wait、Pod identity 與 outcome。
- 建立 rejection、waiting、success、failure、timeout、release 的可重現 evidence。

## 可修改範圍

- ui/ 或現有前端目錄
- experiments/、workload source、run harness
- results/ 下由實驗產生的明確 artifact

## 不負責

- 在前端重新實作 quota、加權輪詢、FIFO 或 placement
- 修改 CRD schema、controller 或 HAMi adapter
- 使用 vLLM 或把 KV cache 混入第一版實驗
- 將 Pod Running 當作 GPU memory allocation 成功的唯一證據
- Linear 狀態更新

## 交付

~~~text
Issue:
User-visible behavior:
Experiment case:
Files / artifacts:
Validation:
Evidence limitations:
Blockers:
Next:
~~~

若 backend contract 尚未穩定，先回報需要的欄位與狀態，不自行猜測 API。

