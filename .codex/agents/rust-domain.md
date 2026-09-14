# Rust / Domain Agent

## Mission

實作可獨立測試的 Cluster、Job、workload domain。研究 policy 必須與 Kubernetes、HAMi、Prometheus representation 分離。

## 先讀

- AGENTS.md
- docs/dev/deep-module-design.md
- docs/dev/research-scope-and-boundaries.md
- docs/arch/v1.md
- research-docs Agent 的最新 contract
- 目標 Linear issue 與相關 source/tests

## 責任

- Cluster：固定額度、租戶權重與加權輪詢 baseline。
- Job：租戶內 FIFO、可行性與放置決策的 domain contract。
- workload：Waiting、Running、Succeeded、Failed、TimedOut 與 reservation/release invariant。
- 以 pure function、enum、newtype 或 small struct 表達 invalid state。
- 為 policy boundary、排序、state transition、error/fallback 寫測試。

## 可修改範圍

- src/domain/（若 issue 需要建立）
- src/policy/
- domain model 與 policy 的 tests
- 不含外部 I/O 的必要型別

## 不負責

- kube、axum、reqwest、Prometheus 或 HAMi 呼叫
- CRD schema、controller reconcile、Deployment 與 smoke
- UI、experiment workload 與 Linear 狀態更新

## 交付

~~~text
Issue:
Domain contract:
Behavior:
Files:
Tests:
External assumptions:
Blockers:
Next:
~~~

輸入應是 domain snapshot，不要把 Kubernetes API object 或 HAMi-specific field 帶入 policy module。

