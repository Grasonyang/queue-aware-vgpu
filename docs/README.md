# Queue-aware vGPU 文件導覽

這套文件寫給第一次接觸本專案的人。每一頁只回答一個問題，先理解目的，再看實作。

## 建議閱讀順序

1. [00-overview](00-overview.md)：這個專案到底在解決什麼問題？
2. [01-platform](01-platform.md)：k3d、k3s、HAMi、Prometheus 各自做什麼？
3. [02-gate-queue-vgpu](02-gate-queue-vgpu.md)：Pod 的 gate 和 vGPU request 如何合作？
4. [03-controller-loop](03-controller-loop.md)：Rust controller 如何做一次決策？
5. [04-modes](04-modes.md)：四種 mode 有什麼差別？
6. [05-code-map](05-code-map.md)：概念如何對應到 `src/`？
7. [06-reproduce](06-reproduce.md)：如何重新部署和驗證？
8. [07-experiments](07-experiments.md)：如何做可比較的實驗？
9. [08-teacher-talk](08-teacher-talk.md)：明天如何用 5–7 分鐘報告？
10. [09-rust-primer](09-rust-primer.md)：C++ 開發者需要的 Rust 最小背景。

## 閱讀規則

- 先看 Mermaid 圖，再看文字。
- 每一頁的 code link 只用來定位，不要求一次讀完。
- 不理解某個 Rust 語法時，先回到 `09-rust-primer`，不要停在細節裡。
- 研究核心是 admission policy；k3d GPU image 是基礎設施 workaround。

## 最短版本

```text
Pod 帶著 gate 建立
→ controller 判斷現在能不能進 scheduler
→ 可以就移除 gate
→ Kubernetes + HAMi 負責真正的 GPU placement / isolation
```
