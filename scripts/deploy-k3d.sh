#!/usr/bin/env bash

set -euo pipefail

CLUSTER_NAME="${K3D_CLUSTER_NAME:-gx10}"
IMAGE="${K3D_GPU_IMAGE:-queue-aware-vgpu/k3s-gpu:v1.35.5-k3s1}"
AGENTS="${K3D_AGENTS:-1}"
INSTALL=false
CREATE=false
RECREATE=false

usage() {
    cat <<'EOF'
Usage: scripts/deploy-k3d.sh [options]

Install k3d or create the GPU-enabled research cluster.

Options:
  --install       install k3d
  --create        create the cluster; fail if it already exists
  --recreate      delete and recreate the named cluster (destructive)
  --cluster NAME  cluster name (default: gx10)
  --image NAME    GPU-enabled K3s image
  --agents COUNT  number of agent nodes (default: 1)
  --help          show this help

The create path passes all host GPUs to node containers and uses the custom
ARM64 K3s image built by scripts/build-k3d-gpu-image.sh.
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --install)
            INSTALL=true
            shift
            ;;
        --create)
            CREATE=true
            shift
            ;;
        --recreate)
            RECREATE=true
            shift
            ;;
        --cluster)
            [[ $# -ge 2 ]] || { echo "Error: --cluster requires a name" >&2; exit 1; }
            CLUSTER_NAME="$2"
            shift 2
            ;;
        --image)
            [[ $# -ge 2 ]] || { echo "Error: --image requires a value" >&2; exit 1; }
            IMAGE="$2"
            shift 2
            ;;
        --agents)
            [[ $# -ge 2 ]] || { echo "Error: --agents requires a count" >&2; exit 1; }
            AGENTS="$2"
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

if [[ "${INSTALL}" == true ]]; then
    command -v curl >/dev/null || { echo "Error: curl is required" >&2; exit 1; }
    curl --fail --silent --show-error https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | bash
fi

if [[ "${CREATE}" == true && "${RECREATE}" == true ]]; then
    echo "Error: choose only one of --create or --recreate" >&2
    exit 1
fi

if [[ "${CREATE}" == false && "${RECREATE}" == false ]]; then
    [[ "${INSTALL}" == true ]] && exit 0
    echo "Error: --create or --recreate is required" >&2
    usage >&2
    exit 1
fi

command -v k3d >/dev/null || { echo "Error: k3d is required" >&2; exit 1; }
command -v docker >/dev/null || { echo "Error: docker is required" >&2; exit 1; }

if ! [[ "${AGENTS}" =~ ^[0-9]+$ ]] || [[ "${AGENTS}" -lt 1 ]]; then
    echo "Error: --agents must be a positive integer" >&2
    exit 1
fi

cluster_exists=false
if k3d cluster list --no-headers | awk '{print $1}' | grep -Fxq "${CLUSTER_NAME}"; then
    cluster_exists=true
fi

if [[ "${RECREATE}" == true ]]; then
    if [[ "${cluster_exists}" == true ]]; then
        echo "Deleting existing cluster ${CLUSTER_NAME} because --recreate was explicit"
        k3d cluster delete "${CLUSTER_NAME}"
    fi
elif [[ "${cluster_exists}" == true ]]; then
    echo "Error: cluster ${CLUSTER_NAME} already exists; use --recreate explicitly" >&2
    exit 1
fi

if ! docker image inspect "${IMAGE}" >/dev/null 2>&1; then
    echo "Error: image ${IMAGE} is not available locally" >&2
    echo "       Run scripts/build-k3d-gpu-image.sh first." >&2
    exit 1
fi

echo "Creating ${CLUSTER_NAME} with ${AGENTS} GPU agent(s)"
k3d cluster create "${CLUSTER_NAME}" \
    --image "${IMAGE}" \
    --gpus all \
    --agents "${AGENTS}" \
    --wait
