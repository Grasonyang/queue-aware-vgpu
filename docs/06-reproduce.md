# 06｜從零復現與驗證

本頁是操作手冊，不是理論說明。

## 0. 先確認工具

需要 Docker、k3d、kubectl、Helm、jq、Rust。

```bash
docker version
k3d version
kubectl version --short
helm version
jq --version
```

## 1. 建立 GPU-enabled gx10

只在沒有 gx10 時使用 `--create`：

```bash
scripts/build-k3d-gpu-image.sh
scripts/deploy-k3d.sh --create --cluster gx10
```

不要在有重要資料時使用 `--recreate`。

## 2. 安裝 HAMi 和 Prometheus

```bash
scripts/deploy-hami.sh --cluster gx10 --node k3d-gx10-agent-0
scripts/deploy-prometheus.sh --cluster gx10
```

## 3. 建立 controller

```bash
scripts/deploy-controller.sh --cluster gx10 --build
```

這會建立 CRD、RBAC、ConfigMap、Deployment、Service 和 ServiceMonitor。

## 4. 執行端到端 smoke test

```bash
kubectl --context k3d-gx10 apply \
  -f experiments/queues/research-queues.yaml
scripts/smoke-test.sh --cluster gx10
```

預期輸出：

```text
controller,crd,gate,hami,gpu smoke: PASS
```

這五個詞各自代表：controller running、CRD 存在、gate 被移除、HAMi 完成配置、GPU container 成功執行。

## 5. 查詢基本狀態

```bash
kubectl --context k3d-gx10 get nodes
kubectl --context k3d-gx10 -n queue-aware-vgpu get pods
kubectl --context k3d-gx10 -n kube-system get pods
```

若 smoke test 成功，代表 gate、controller、HAMi 和 GPU execution 已形成閉環。

## 6. Prometheus

```bash
kubectl --context k3d-gx10 -n monitoring \
  port-forward svc/monitoring-kube-prometheus-prometheus 19090:9090
```

另一個 terminal：

```bash
curl -sG http://127.0.0.1:19090/api/v1/query \
  --data-urlencode 'query=queue_vgpu_admissions_total' | jq
```

## 7. Wiki 同步

先在 GitHub repository 啟用 Wiki。若 Actions 的預設 token 無法寫入 Wiki，新增一個具有該 repository Wiki write access 的 `WIKI_TOKEN` secret；否則 workflow 會嘗試使用 `GITHUB_TOKEN`。

之後 push 到 `main`，或在 Actions 手動執行 `Sync docs to Wiki`。workflow 只管理編號頁、`Home`、`Index` 和 `_Sidebar`，其他 Wiki 頁面會保留。
