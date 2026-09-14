#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
DOCS_DIR="$ROOT_DIR/docs"
MAX_LINES=220
failed=false

[[ -d "$DOCS_DIR" ]] || { echo "docs directory is missing" >&2; exit 1; }

mapfile -d '' markdown_files < <(find "$DOCS_DIR" -type f -name '*.md' -print0 | sort -z)
if ((${#markdown_files[@]} == 0)); then
  echo "no Markdown documents found under: $DOCS_DIR" >&2
  exit 1
fi

for file in "${markdown_files[@]}"; do
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
  "$DOCS_DIR/arch/v1.md"
  "$DOCS_DIR/arch/v1-architecture-image.md"
  "$DOCS_DIR/arch/v1-domain-contracts.md"
  "$DOCS_DIR/arch/domain/README.md"
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
