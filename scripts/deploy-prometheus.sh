#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
CLUSTER_NAME="${K3D_CLUSTER_NAME:-gx10}"
RELEASE_NAME="${PROMETHEUS_RELEASE:-monitoring}"
NAMESPACE="${PROMETHEUS_NAMESPACE:-monitoring}"
CHART_VERSION="${PROMETHEUS_CHART_VERSION:-}"

usage() {
  cat <<'EOF'
Usage: scripts/deploy-prometheus.sh [options]

Install or upgrade kube-prometheus-stack on the existing k3d cluster.

Options:
  --cluster NAME       k3d cluster name (default: gx10)
  --namespace NAME     monitoring namespace (default: monitoring)
  --version VERSION    optional chart version
  --help               show this help
EOF
}

while (($# > 0)); do
  case "$1" in
    --cluster) CLUSTER_NAME="$2"; shift 2 ;;
    --namespace) NAMESPACE="$2"; shift 2 ;;
    --version) CHART_VERSION="$2"; shift 2 ;;
    --help) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

command -v kubectl >/dev/null || { echo "kubectl is required" >&2; exit 1; }
command -v helm >/dev/null || { echo "helm is required" >&2; exit 1; }
KUBE_CONTEXT="k3d-${CLUSTER_NAME}"
kubectl config get-contexts -o name | grep -Fxq "$KUBE_CONTEXT" || {
  echo "Kubernetes context $KUBE_CONTEXT was not found; refusing to create a cluster." >&2
  exit 1
}

helm repo add prometheus-community https://prometheus-community.github.io/helm-charts --force-update >/dev/null
helm repo update prometheus-community >/dev/null

args=(
  upgrade --install "$RELEASE_NAME" prometheus-community/kube-prometheus-stack
  --kube-context "$KUBE_CONTEXT"
  --namespace "$NAMESPACE"
  --create-namespace
  --values "$ROOT_DIR/scripts/monitoring-values.yaml"
  --wait
  --timeout 15m
)
if [[ -n "$CHART_VERSION" ]]; then
  args+=(--version "$CHART_VERSION")
fi

helm "${args[@]}"
kubectl --context "$KUBE_CONTEXT" -n "$NAMESPACE" rollout status "deployment/${RELEASE_NAME}-kube-prometheus-operator" --timeout=10m
kubectl --context "$KUBE_CONTEXT" -n "$NAMESPACE" get pods -o wide
