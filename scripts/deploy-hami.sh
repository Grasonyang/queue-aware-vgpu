#!/usr/bin/env bash

set -euo pipefail

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
readonly VALUES_FILE="${REPO_ROOT}/deploy/hami-values.yaml"

CLUSTER_NAME="${K3D_CLUSTER_NAME:-gx10}"
HAMI_VERSION="${HAMI_VERSION:-2.10.0}"
GPU_NODE="${GPU_NODE:-}"

usage() {
    cat <<'EOF'
Usage: scripts/deploy-hami.sh [options]

Install or upgrade HAMi on an existing k3d cluster without recreating it.

Options:
  --cluster NAME  k3d cluster name (default: gx10)
  --node NAME     node to label and manage with HAMi
  --version VER   HAMi chart version (default: 2.10.0)
  --help          show this help

Environment:
  K3D_CLUSTER_NAME, GPU_NODE, HAMI_VERSION can provide the same defaults.
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --cluster)
            [[ $# -ge 2 ]] || { echo "Error: --cluster requires a name" >&2; exit 1; }
            CLUSTER_NAME="$2"
            shift 2
            ;;
        --node)
            [[ $# -ge 2 ]] || { echo "Error: --node requires a name" >&2; exit 1; }
            GPU_NODE="$2"
            shift 2
            ;;
        --version)
            [[ $# -ge 2 ]] || { echo "Error: --version requires a value" >&2; exit 1; }
            HAMI_VERSION="$2"
            shift 2
            ;;
        --help)
            usage
            exit 0
            ;;
        *)
            echo "Error: unknown option: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

command -v kubectl >/dev/null || { echo "Error: kubectl is required" >&2; exit 1; }
command -v helm >/dev/null || { echo "Error: helm is required" >&2; exit 1; }
[[ -f "${VALUES_FILE}" ]] || { echo "Error: missing ${VALUES_FILE}" >&2; exit 1; }

KUBE_CONTEXT="k3d-${CLUSTER_NAME}"
if ! kubectl config get-contexts -o name | grep -Fxq "${KUBE_CONTEXT}"; then
    echo "Error: Kubernetes context ${KUBE_CONTEXT} was not found" >&2
    echo "       Refusing to create or recreate a cluster." >&2
    exit 1
fi

kubectl --context "${KUBE_CONTEXT}" cluster-info >/dev/null

if [[ -z "${GPU_NODE}" ]]; then
    mapfile -t nodes < <(kubectl --context "${KUBE_CONTEXT}" get nodes \
        -o jsonpath='{range .items[*]}{.metadata.name}{"\n"}{end}')
    if [[ "${#nodes[@]}" -ne 1 ]]; then
        echo "Error: multiple Kubernetes nodes found; pass --node explicitly" >&2
        printf '       %s\n' "${nodes[@]}" >&2
        exit 1
    fi
    GPU_NODE="${nodes[0]}"
fi

if ! kubectl --context "${KUBE_CONTEXT}" get node "${GPU_NODE}" >/dev/null; then
    echo "Error: node ${GPU_NODE} was not found in ${KUBE_CONTEXT}" >&2
    exit 1
fi

echo "Labeling ${GPU_NODE} for HAMi"
kubectl --context "${KUBE_CONTEXT}" label node "${GPU_NODE}" gpu=on --overwrite

echo "Refreshing HAMi Helm repository"
helm repo add hami-charts https://project-hami.github.io/HAMi/ --force-update >/dev/null
helm repo update hami-charts >/dev/null

echo "Installing HAMi ${HAMI_VERSION} without replacing kube-scheduler"
helm upgrade --install hami hami-charts/hami \
    --kube-context "${KUBE_CONTEXT}" \
    --namespace kube-system \
    --version "${HAMI_VERSION}" \
    --values "${VALUES_FILE}" \
    --set scheduler.kubeScheduler.enabled=true \
    --set scheduler.kubeScheduler.image.registry=registry.k8s.io \
    --set scheduler.kubeScheduler.image.repository=kube-scheduler \
    --set scheduler.kubeScheduler.image.tag=v1.35.5 \
    --set scheduler.forceOverwriteDefaultScheduler=true \
    --set scheduler.leaderElect=false \
    --set scheduler.replicas=1 \
    --wait \
    --timeout 10m

echo "Waiting for HAMi components"
kubectl --context "${KUBE_CONTEXT}" -n kube-system rollout status deployment/hami-scheduler --timeout=5m
kubectl --context "${KUBE_CONTEXT}" -n kube-system rollout status daemonset/hami-device-plugin --timeout=5m

gpu_resource="$(kubectl --context "${KUBE_CONTEXT}" get node "${GPU_NODE}" \
    -o jsonpath='{.status.allocatable.nvidia\.com/gpu}')"
if [[ -z "${gpu_resource}" ]]; then
    echo "Error: HAMi is running but nvidia.com/gpu is not allocatable on ${GPU_NODE}" >&2
    kubectl --context "${KUBE_CONTEXT}" -n kube-system get pods -l app.kubernetes.io/component=hami-device-plugin -o wide >&2
    exit 1
fi

echo "HAMi is ready: ${GPU_NODE} allocatable nvidia.com/gpu=${gpu_resource}"
kubectl --context "${KUBE_CONTEXT}" get node "${GPU_NODE}" \
    -o custom-columns='NAME:.metadata.name,ARCH:.status.nodeInfo.architecture,GPU:.status.allocatable.nvidia\.com/gpu'
