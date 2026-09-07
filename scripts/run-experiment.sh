#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
CLUSTER_NAME="${K3D_CLUSTER_NAME:-gx10}"
NAMESPACE="research"
MODE="queue"
SCENARIO="burst"
TIMEOUT="240"
RESULTS_DIR="${ROOT_DIR}/results"
PROMETHEUS_URL="${PROMETHEUS_URL:-}"

usage() {
  cat <<'EOF'
Usage: scripts/run-experiment.sh [options]

Options:
  --cluster NAME       k3d cluster name (default: gx10)
  --mode MODE          passthrough|queue|fragmentation|adaptive (default: queue)
  --scenario NAME      scenario manifest name (default: burst)
  --timeout SECONDS    completion timeout (default: 240)
  --results-dir PATH   output directory (default: results/)
  --prometheus-url URL optional Prometheus API URL for query export
  --help               show this help
EOF
}

while (($# > 0)); do
  case "$1" in
    --cluster) CLUSTER_NAME="$2"; shift 2 ;;
    --mode) MODE="$2"; shift 2 ;;
    --scenario) SCENARIO="$2"; shift 2 ;;
    --timeout) TIMEOUT="$2"; shift 2 ;;
    --results-dir) RESULTS_DIR="$2"; shift 2 ;;
    --prometheus-url) PROMETHEUS_URL="$2"; shift 2 ;;
    --help) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

case "$MODE" in
  passthrough|queue|fragmentation|adaptive) ;;
  *) echo "invalid mode: $MODE" >&2; exit 2 ;;
esac
command -v jq >/dev/null || { echo "jq is required to export experiment results" >&2; exit 1; }
KUBE_CONTEXT="k3d-${CLUSTER_NAME}"
SCENARIO_FILE="$ROOT_DIR/experiments/scenarios/${SCENARIO}.yaml"
[[ -f "$SCENARIO_FILE" ]] || { echo "scenario file not found: $SCENARIO_FILE" >&2; exit 1; }
mkdir -p "$RESULTS_DIR"

kubectl --context "$KUBE_CONTEXT" apply -f "$ROOT_DIR/experiments/queues/research-queues.yaml" >/dev/null
kubectl --context "$KUBE_CONTEXT" -n "$NAMESPACE" delete pod \
  -l "queue-aware-vgpu.io/scenario=${SCENARIO}" --ignore-not-found --wait=true >/dev/null

kubectl --context "$KUBE_CONTEXT" apply -f - <<EOF >/dev/null
apiVersion: v1
kind: ConfigMap
metadata:
  name: queue-aware-vgpu-controller
  namespace: queue-aware-vgpu
data:
  config.yaml: |
    mode: ${MODE}
    lookahead: 8
    starvation_timeout_secs: 120
    reconcile_interval_secs: 5
    safety_margin_mib: 512
    default_request_memory_mib: 4096
    overcommit_step: 0.05
    overcommit_cooldown_secs: 45
    min_overcommit: 1.0
    max_overcommit: 1.2
    metrics_bind_address: 0.0.0.0:8080
    prometheus_url: ${PROMETHEUS_URL:-null}
EOF
kubectl --context "$KUBE_CONTEXT" -n queue-aware-vgpu rollout restart deployment/queue-aware-vgpu-controller >/dev/null
kubectl --context "$KUBE_CONTEXT" -n queue-aware-vgpu rollout status deployment/queue-aware-vgpu-controller --timeout=180s >/dev/null

submit_epoch="$(date -u +%s)"
run_id="$(date -u +%Y%m%dT%H%M%SZ)-${MODE}-${SCENARIO}"
json_file="$RESULTS_DIR/${run_id}.json"
csv_file="$RESULTS_DIR/${run_id}.csv"
kubectl --context "$KUBE_CONTEXT" apply -f "$SCENARIO_FILE" >/dev/null

deadline=$((SECONDS + TIMEOUT))
while ((SECONDS < deadline)); do
  pod_json="$(kubectl --context "$KUBE_CONTEXT" -n "$NAMESPACE" get pods -l "queue-aware-vgpu.io/scenario=${SCENARIO}" -o json)"
  count="$(jq '.items | length' <<<"$pod_json")"
  finished="$(jq '[.items[].status.phase | select(. == "Succeeded" or . == "Failed")] | length' <<<"$pod_json")"
  if [[ "$count" -gt 0 && "$finished" -eq "$count" ]]; then
    break
  fi
  sleep 3
done

kubectl --context "$KUBE_CONTEXT" -n "$NAMESPACE" get pods \
  -l "queue-aware-vgpu.io/scenario=${SCENARIO}" -o json > "$json_file"
{
  echo 'name,queue,requested_memory_mib,submit_epoch,creation_time,admitted_at,start_time,completion_time,phase'
  jq -r --arg submit "$submit_epoch" '.items[] | [
    .metadata.name,
    (.metadata.annotations["queue-aware-vgpu.io/queue"] // ""),
    (.spec.containers[0].resources.limits["nvidia.com/gpumem"] // ""),
    $submit,
    (.metadata.creationTimestamp // ""),
    (.metadata.annotations["queue-aware-vgpu.io/admitted-at"] // ""),
    (.status.startTime // ""),
    (.status.containerStatuses[0].state.terminated.finishedAt // ""),
    (.status.phase // "")
  ] | @csv' "$json_file"
} > "$csv_file"

if [[ -n "$PROMETHEUS_URL" ]] && command -v curl >/dev/null; then
  if ! curl --fail --silent --get "$PROMETHEUS_URL/api/v1/query" \
    --data-urlencode 'query=queue_vgpu_pending_jobs' \
    > "$RESULTS_DIR/${run_id}-prometheus.json"; then
    rm -f "$RESULTS_DIR/${run_id}-prometheus.json"
  fi
fi
echo "experiment complete: $csv_file"
