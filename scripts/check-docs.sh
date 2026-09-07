#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
DOCS_DIR="$ROOT_DIR/docs"
MAX_LINES=100
failed=false

[[ -d "$DOCS_DIR" ]] || { echo "docs directory is missing" >&2; exit 1; }

for file in "$DOCS_DIR"/*.md; do
  line_count="$(wc -l < "$file")"
  if (( line_count > MAX_LINES )); then
    echo "too long: $file has $line_count lines (limit $MAX_LINES)" >&2
    failed=true
  fi

  fence_count="$(grep -c '^```' "$file" || true)"
  if (( fence_count % 2 != 0 )); then
    echo "unclosed code fence: $file" >&2
    failed=true
  fi

  if grep -nE '(/home/|file://)' "$file" >/dev/null; then
    echo "machine-local link found: $file" >&2
    failed=true
  fi
done

required=(
  "$DOCS_DIR/README.md"
  "$DOCS_DIR/00-overview.md"
  "$DOCS_DIR/01-platform.md"
  "$DOCS_DIR/02-gate-queue-vgpu.md"
  "$DOCS_DIR/03-controller-loop.md"
  "$DOCS_DIR/04-modes.md"
  "$DOCS_DIR/05-code-map.md"
  "$DOCS_DIR/06-reproduce.md"
  "$DOCS_DIR/07-experiments.md"
  "$DOCS_DIR/08-teacher-talk.md"
  "$DOCS_DIR/09-rust-primer.md"
)
for file in "${required[@]}"; do
  [[ -f "$file" ]] || {
    echo "required document is missing: $file" >&2
    failed=true
  }
done

if [[ "$failed" == true ]]; then
  exit 1
fi

echo "docs check: PASS"
