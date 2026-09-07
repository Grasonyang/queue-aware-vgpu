#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
CLUSTER_NAME="${K3D_CLUSTER_NAME:-gx10}"
TIMEOUT="${SMOKE_TIMEOUT_SECONDS:-120}"

usage() {
  cat <<'EOF'
Usage: scripts/smoke-test.sh [--cluster NAME] [--timeout SECONDS]

Validates CRD, controller, Scheduling Gate admission, HAMi scheduling, and
GPU execution. Prometheus scraping is checked separately by the deployment
verification because it requires a monitoring port-forward.
EOF
}
while (($# > 0)); do
  case "$1" in
    --cluster) CLUSTER_NAME="$2"; shift 2 ;;
    --timeout) TIMEOUT="$2"; shift 2 ;;
    --help) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

KUBE_CONTEXT="k3d-${CLUSTER_NAME}"
kubectl --context "$KUBE_CONTEXT" get crd vgpuqueues.queue-aware-vgpu.io >/dev/null
kubectl --context "$KUBE_CONTEXT" -n queue-aware-vgpu rollout status deployment/queue-aware-vgpu-controller --timeout=30s >/dev/null
kubectl --context "$KUBE_CONTEXT" apply -f "$ROOT_DIR/experiments/queues/research-queues.yaml" >/dev/null
kubectl --context "$KUBE_CONTEXT" -n research delete pod queue-gated-gpu --ignore-not-found --wait=true >/dev/null
kubectl --context "$KUBE_CONTEXT" apply -f "$ROOT_DIR/experiments/workloads/queue-gated-gpu.yaml" >/dev/null

deadline=$((SECONDS + TIMEOUT))
while ((SECONDS < deadline)); do
  phase="$(kubectl --context "$KUBE_CONTEXT" -n research get pod queue-gated-gpu -o jsonpath='{.status.phase}')"
  gates="$(kubectl --context "$KUBE_CONTEXT" -n research get pod queue-gated-gpu -o jsonpath='{.spec.schedulingGates}')"
  if [[ -z "$gates" && "$phase" == "Running" ]]; then
    break
  fi
  sleep 2
done

phase="$(kubectl --context "$KUBE_CONTEXT" -n research get pod queue-gated-gpu -o jsonpath='{.status.phase}')"
gates="$(kubectl --context "$KUBE_CONTEXT" -n research get pod queue-gated-gpu -o jsonpath='{.spec.schedulingGates}')"
[[ -z "$gates" ]] || { echo "Scheduling Gate was not removed" >&2; exit 1; }
[[ "$phase" == "Running" || "$phase" == "Succeeded" ]] || { echo "GPU smoke phase: $phase" >&2; exit 1; }
kubectl --context "$KUBE_CONTEXT" -n research logs queue-gated-gpu | grep -F 'NVIDIA GB10' >/dev/null

kubectl --context "$KUBE_CONTEXT" get vgpuqueue -n research >/dev/null
echo "controller,crd,gate,hami,gpu smoke: PASS"
