#!/usr/bin/env bash

set -euo pipefail

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
readonly DOCKERFILE="${REPO_ROOT}/deploy/k3d-gpu/Dockerfile"
readonly BUILD_CONTEXT="${REPO_ROOT}/deploy/k3d-gpu"

K3S_TAG="${K3S_TAG:-v1.35.5-k3s1}"
IMAGE="${K3D_GPU_IMAGE:-queue-aware-vgpu/k3s-gpu:${K3S_TAG}}"

usage() {
    cat <<'EOF'
Usage: scripts/build-k3d-gpu-image.sh [options]

Build the native ARM64 K3s image used by k3d for GPU workloads.

Options:
  --k3s-tag TAG  K3s image tag (default: v1.35.5-k3s1)
  --image NAME    output image name
  --help          show this help

The image is built locally and is not pushed to a registry.
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --k3s-tag)
            [[ $# -ge 2 ]] || { echo "Error: --k3s-tag requires a value" >&2; exit 1; }
            K3S_TAG="$2"
            shift 2
            ;;
        --image)
            [[ $# -ge 2 ]] || { echo "Error: --image requires a value" >&2; exit 1; }
            IMAGE="$2"
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

command -v docker >/dev/null || { echo "Error: docker is required" >&2; exit 1; }
[[ -f "${DOCKERFILE}" ]] || { echo "Error: missing ${DOCKERFILE}" >&2; exit 1; }

echo "Building ${IMAGE} for linux/arm64"
docker build \
    --platform linux/arm64 \
    --build-arg "K3S_TAG=${K3S_TAG}" \
    --tag "${IMAGE}" \
    --file "${DOCKERFILE}" \
    "${BUILD_CONTEXT}"

echo "Built ${IMAGE}"
