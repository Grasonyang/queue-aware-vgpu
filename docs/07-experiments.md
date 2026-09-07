# 07｜實驗：如何比較四種 mode

研究問題不是「controller 有沒有跑」，而是：

```text
queue awareness 改善什麼？
fragmentation awareness 改善什麼？
adaptive overcommit 是否增加吞吐而不增加 OOM？
```

## 相同 workload、只改 mode

```bash
scripts/run-experiment.sh --cluster gx10 --mode passthrough
scripts/run-experiment.sh --cluster gx10 --mode queue
scripts/run-experiment.sh --cluster gx10 --mode fragmentation
scripts/run-experiment.sh --cluster gx10 --mode adaptive \
  --prometheus-url http://monitoring-kube-prometheus-prometheus.monitoring.svc:9090
```

burst scenario 的順序是：

```text
small → large → small → medium → large → small
```

## 每次輸出的資料

`results/` 會產生 CSV 和 JSON，包含：

- submit time
- creation time
- admission time
- scheduled / start time
- completion time
- queue
- requested memory
- final phase

## 可以計算的研究指標

```text
admission wait = admission time - creation time
JCT             = completion time - creation time
throughput      = 完成 workload 數 / 時間
fairness        = 不同 queue 的等待和 allocation 分布
```

Prometheus 額外提供：

- pending / running jobs
- allocated memory
- admission count / wait
- fragmentation score
- overcommit ratio
- error count

## 解讀原則

- 不要用單次 smoke test 宣稱 policy 優越。
- 四種 mode 必須使用同一 cluster、同一 scenario、相同提交順序。
- 至少重複多次，再比較 mean / P95 wait、throughput、OOM、restart。
- `adaptive` 的 overcommit 是 logical admission，不等於 HAMi 硬體 limit 被改變。
