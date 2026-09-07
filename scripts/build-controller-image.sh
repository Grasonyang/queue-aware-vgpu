#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="queue-aware-vgpu/controller:dev"

usage() {
  cat <<'EOF'
Build the queue-aware-vGPU controller image for the local k3d cluster.

Options:
  --image NAME   Image tag (default: queue-aware-vgpu/controller:dev)
  --help         Show this help
EOF
}

while (($# > 0)); do
  case "$1" in
    --image)
      IMAGE="$2"
      shift 2
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

docker build --tag "$IMAGE" --file "$ROOT_DIR/deploy/controller/Dockerfile" "$ROOT_DIR"
echo "Built $IMAGE"
