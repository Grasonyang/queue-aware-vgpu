#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
CLUSTER_NAME="${K3D_CLUSTER_NAME:-gx10}"
IMAGE="queue-aware-vgpu/controller:dev"
BUILD_IMAGE=false

usage() {
  cat <<'EOF'
Usage: scripts/deploy-controller.sh [options]

Options:
  --cluster NAME  k3d cluster name (default: gx10)
  --image NAME    controller image (default: queue-aware-vgpu/controller:dev)
  --build         build and import the image before deploying
  --help          show this help
EOF
}

while (($# > 0)); do
  case "$1" in
    --cluster) CLUSTER_NAME="$2"; shift 2 ;;
    --image) IMAGE="$2"; shift 2 ;;
    --build) BUILD_IMAGE=true; shift ;;
    --help) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

command -v kubectl >/dev/null || { echo "kubectl is required" >&2; exit 1; }
KUBE_CONTEXT="k3d-${CLUSTER_NAME}"
kubectl config get-contexts -o name | grep -Fxq "$KUBE_CONTEXT" || {
  echo "Kubernetes context $KUBE_CONTEXT was not found; refusing to create a cluster." >&2
  exit 1
}

if [[ "$BUILD_IMAGE" == true ]]; then
  command -v docker >/dev/null || { echo "docker is required for --build" >&2; exit 1; }
  "$ROOT_DIR/scripts/build-controller-image.sh" --image "$IMAGE"
  command -v k3d >/dev/null || { echo "k3d is required for --build" >&2; exit 1; }
  k3d image import "$IMAGE" --cluster "$CLUSTER_NAME"
fi

kubectl --context "$KUBE_CONTEXT" apply \
  -f "$ROOT_DIR/deploy/controller/namespace.yaml" \
  -f "$ROOT_DIR/deploy/controller/crd.yaml" \
  -f "$ROOT_DIR/deploy/controller/service-account.yaml" \
  -f "$ROOT_DIR/deploy/controller/rbac.yaml" \
  -f "$ROOT_DIR/deploy/controller/configmap.yaml" \
  -f "$ROOT_DIR/deploy/controller/deployment.yaml" \
  -f "$ROOT_DIR/deploy/controller/service.yaml"
if kubectl --context "$KUBE_CONTEXT" get crd servicemonitors.monitoring.coreos.com >/dev/null 2>&1; then
  kubectl --context "$KUBE_CONTEXT" apply -f "$ROOT_DIR/deploy/controller/servicemonitor.yaml"
fi
kubectl --context "$KUBE_CONTEXT" -n queue-aware-vgpu rollout restart deployment/queue-aware-vgpu-controller >/dev/null
kubectl --context "$KUBE_CONTEXT" -n queue-aware-vgpu rollout status deployment/queue-aware-vgpu-controller --timeout=180s
