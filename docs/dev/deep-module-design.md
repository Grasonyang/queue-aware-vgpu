## How to build with Deep Modules

設計優先採用 Deep Module：

> small, stable interface; substantial hidden implementation.

模組的價值不在於檔案小或 function 少，而在於是否能用簡單 interface
隱藏足夠多的 domain complexity。

### 1. Organize by responsibility, not by code size

每個 module 應擁有一個完整且可描述的 responsibility。

例如：

- `dqa` 擁有 tenant scoring、quota decision 與 governance policy
- `fwa` 擁有 fragmentation scoring 與 placement decision
- `topology` 擁有 topology representation、cost evaluation 與 prediction boundary
- `overcommit` 擁有 runtime pressure state、state transition 與 protection decision

不要因為 function 或 file 變長，就機械式拆成更多 module。
只有當 responsibility 已經分裂時才拆分。

LOC limit 是 refactor signal，不是 architecture goal。

### 2. Prefer deep interfaces

呼叫端應只需要知道完成工作所需的最少資訊。

好的 module：

`input -> domain decision -> output`

呼叫端不應知道 module 內部用了幾個 scoring step、cache、threshold、
fallback 或 intermediate representation。

避免建立只有轉呼叫、欄位搬運或單一條件判斷的 shallow abstraction。

若一個 abstraction：

- interface 很複雜；
- 但內部只隱藏極少邏輯；

則優先合併回真正擁有該 responsibility 的 module。

### 3. Hide implementation decisions

以下內容原則上應留在 module 內部：

- intermediate score
- normalization method
- fallback rule
- cache representation
- internal state transition
- temporary research heuristic
- external format conversion

只有跨 boundary 真正需要的概念才成為 public API。

Rust 預設使用 private 或 `pub(crate)`；
不要為未出現的需求提前建立 public API、generic trait、plugin system
或 extensibility framework。

### 4. Keep domain core pure

研究邏輯優先實作成 pure function、small state machine
或具有明確 invariant 的 domain struct。

例如：

`metrics -> DQA score`

`snapshot + workload -> FWA score`

`runtime signal + current state -> overcommit transition`

這些 domain computation 不應自行：

- 呼叫 Kubernetes API
- query Prometheus
- patch resource
- 寫檔
- sleep / retry
- 操作 network

I/O 由 adapter 負責，controller / orchestrator 只組合流程。

### 5. Dependencies point inward

依賴方向應盡量維持：

`external system -> adapter -> domain module`

而不是：

`domain module -> Kubernetes / HAMi / Prometheus`

外部系統的 object 應先轉換成研究 domain 所需要的最小 representation，
再交給 policy module。

例如不要讓 FWA 接收完整 `Pod` 或 `Node` object；
應轉成它真正需要的 workload demand、available VRAM、topology features
或其他 domain input。

### 6. One source of truth

一項研究概念只能有一個主要 owner。

不要在 controller、adapter、experiment script 與 domain module
重複實作同一套：

- scoring
- threshold
- state transition
- quota calculation
- fragmentation calculation

experiment 可以選擇 policy、配置參數與記錄結果，
但不得偷偷複製或修改 production research logic。

### 7. Make invalid states difficult to represent

對有明確 domain invariant 的資料，優先使用 enum、newtype 或 struct 表達，
而不是散落的 string、bool 與 magic number。

尤其適用於：

- research mode
- overcommit state
- decision reason
- queue / tenant identity
- score / ratio boundary
- admission result

module 應自行保護自己的 invariant，
不要要求每個 caller 都記得相同的 validation rule。

### 8. Refactor when complexity leaks

出現以下情況時，優先重新設計 module boundary，而不是繼續加 function：

- caller 必須理解大量 module internals
- 同一組參數在很多 function 間反覆傳遞
- controller 開始包含 research policy
- adapter 開始決定研究結果
- 同一 domain rule 出現在多個檔案
- 修改一個 policy 需要同時修改很多無關 module
- public API 持續增加但實際 capability 沒有增加

Deep Module 的目標是：
讓上層看到更少概念，而不是把同樣的複雜度拆散到更多檔案。
