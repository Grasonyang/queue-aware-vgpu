## Research scope & Boundaries

本 repo 的研究架構固定為三個研究層，以及一個跨層 observability / feedback plane：

- Cluster layer：DQA / Governance
- Job layer：FWA / GFS / topology-aware scheduling
- Pod layer：HAMi-Core dynamic overcommit / isolation
- Cross-layer observability：Prometheus / feedback / experiment evidence

Prometheus 不視為獨立研究層；它只負責蒐集、聚合與提供跨層 feedback、
metrics 與 experiment evidence，不在其中實作研究 policy。

每項變更必須標記唯一主要研究層，以及變更類型：

`baseline` / `research implementation` / `refactor` / `experiment` / `bugfix`

研究範圍以研究計畫與已確認的研究設計為 authority。
無法對應研究主線的功能預設不做；不得因 TODO、README、現有 prototype
或實作方便而自行擴張研究問題。

baseline 只能作為比較基準，不得描述為最終研究方法。
不得自行發明研究公式、metric、policy、threshold 或新的研究目標；
若既有設計不足，先明確記錄 gap，再進行設計決策。

架構邊界固定如下：

- Controller 只負責 watch、reconcile、snapshot、resource/status patch、
  dependency orchestration 與 metrics exposure。
- Research policy 必須與 Kubernetes、HAMi、Prometheus 等 external I/O 分離。
- DQA、FWA/GFS、topology 與 runtime overcommit 必須維持獨立 domain boundary。
- Kubernetes object parsing、Prometheus query、HAMi integration 等外部格式處理，
  必須留在 adapter / infrastructure boundary。
- Domain policy 不得直接依賴 Kubernetes API object、Prometheus response
  或 HAMi-specific transport representation。
- 不新增第二個 persistent state store，除非研究需求明確要求且有設計依據。
