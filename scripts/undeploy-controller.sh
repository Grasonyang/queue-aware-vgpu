#!/usr/bin/env bash
set -euo pipefail

CLUSTER_NAME="${K3D_CLUSTER_NAME:-gx10}"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
usage() {
  cat <<'EOF'
Usage: scripts/undeploy-controller.sh [--cluster NAME] [--help]

Deletes only controller resources. The k3d cluster, HAMi, Prometheus, queues,
and research workloads are left untouched.
EOF
}
while (($# > 0)); do
  case "$1" in
    --cluster) CLUSTER_NAME="$2"; shift 2 ;;
    --help) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done
KUBE_CONTEXT="k3d-${CLUSTER_NAME}"
kubectl config get-contexts -o name | grep -Fxq "$KUBE_CONTEXT" || {
  echo "Kubernetes context $KUBE_CONTEXT was not found" >&2
  exit 1
}
kubectl --context "$KUBE_CONTEXT" delete -f "$ROOT_DIR/deploy/controller-monitor.yaml" --ignore-not-found
kubectl --context "$KUBE_CONTEXT" delete -f "$ROOT_DIR/deploy/controller.yaml" --ignore-not-found
kubectl --context "$KUBE_CONTEXT" delete -f "$ROOT_DIR/deploy/controller-config.yaml" --ignore-not-found
kubectl --context "$KUBE_CONTEXT" delete -f "$ROOT_DIR/deploy/controller-rbac.yaml" --ignore-not-found
kubectl --context "$KUBE_CONTEXT" delete -f "$ROOT_DIR/deploy/controller-crd.yaml" --ignore-not-found
