# AGENT_WORKFLOW.md

這份文件只說明如何使用多 Agent。每個 Agent 的責任、可修改範圍與回報格式，分別放在 agents/ 目錄。

## 設定檔

- `.codex/config.toml` 是 Codex runtime 讀取的多 Agent 設定，包含執行上限與角色 profile 對應。
- `.codex/agents/*.toml` 是 Codex 可載入的角色設定；同名 `.md` 是給人閱讀與 code review 的完整責任說明。
- `.codex/agent-registry.toml` 保存專案的檔案 ownership、Linear routing 與 handoff 欄位；它是 workflow registry，不是 Codex runtime 設定。

## 使用流程

1. Lead 先讀取 Linear issue、milestone、AGENTS.md、相關架構文件與最小 source context。
2. 從 agents/ 選一個主要 Agent；需要時再選一至兩個協作 Agent。
3. 主要 Agent 只處理自己的責任範圍；協作 Agent 只提供 findings、文件或測試結果。
4. 不同 Agent 不得同時修改同一檔案。Lead 負責整合衝突與最後 wiring。
5. 每個 Agent 完成後使用自己的回報格式交接給 Lead。
6. Lead 執行完整驗證，確認 acceptance criteria 後才回寫 Linear。
7. 只有 Lead 更新 Linear status、comment、milestone 或 issue 描述。

## Agent 選擇

| 工作內容 | 主要 Agent |
| --- | --- |
| 需求釐清、架構決策、研究範圍、README、docs、Mermaid | [research-docs](agents/research-docs.md) |
| Cluster／Job／workload 的 pure domain policy 與測試 | [rust-domain](agents/rust-domain.md) |
| CRD、controller、Kubernetes、HAMi、部署與 smoke | [platform-kubernetes](agents/platform-kubernetes.md) |
| UI、固定記憶體 workload、experiment harness、evidence | [ui-experiment](agents/ui-experiment.md) |
| 跨 Agent 分派、整合、wiring、完整驗證、Linear | [lead-integrator](agents/lead-integrator.md) |

## 目前 Linear ticket 的預設 owner

| Issue | Owner |
| --- | --- |
| GRA-5 | rust-domain，research-docs 協作 |
| GRA-6 | rust-domain |
| GRA-9 | platform-kubernetes，rust-domain 協作 |
| GRA-8 | platform-kubernetes，rust-domain 與 research-docs 協作 |
| GRA-7 | platform-kubernetes，rust-domain 協作 |
| GRA-10 | ui-experiment，platform-kubernetes 協作 |
| GRA-11 | ui-experiment |
| GRA-12 | ui-experiment，platform-kubernetes 協作 |

這是預設分工；若 issue 實際 scope 不同，由 Lead 在開始前重新指定。

## 最小交接格式

~~~text
Issue:
Role:
Changed:
Files:
Validation:
Blockers:
Next:
~~~

沒有修改檔案時，回報 findings 與證據即可。遇到未確認的研究決策、跨層 scope 或外部能力限制，回報 Blockers，不自行擴大工作。

## Lead 完成前檢查

- 所有修改都有明確 owner，沒有未解決的同檔案衝突。
- domain policy 與 Kubernetes／HAMi I/O 分離。
- 相關測試與 AGENTS.md 要求的驗證已完成。
- 必要的 smoke、experiment 或 evidence 已保存。
- Linear comment 已記錄 Changed、Validation、Limitations、Next。
