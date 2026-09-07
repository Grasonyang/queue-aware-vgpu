#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
SOURCE_DIR="$ROOT_DIR/docs"
WIKI_DIR=""
PUSH=false

usage() {
  cat <<'EOF'
Usage: scripts/sync-wiki.sh --wiki-dir PATH [--source PATH] [--push]

Copies the short, numbered docs pages into an existing GitHub Wiki checkout.
Unmanaged Wiki pages are preserved.

Options:
  --wiki-dir PATH  existing checkout of REPO.wiki.git
  --source PATH    docs directory (default: repo/docs)
  --push           commit and push changed Wiki pages
  --help           show this help
EOF
}

while (($# > 0)); do
  case "$1" in
    --wiki-dir) WIKI_DIR="$2"; shift 2 ;;
    --source) SOURCE_DIR="$2"; shift 2 ;;
    --push) PUSH=true; shift ;;
    --help) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

[[ -n "$WIKI_DIR" ]] || { echo "--wiki-dir is required" >&2; exit 2; }
[[ -d "$WIKI_DIR/.git" ]] || { echo "not a Git repository: $WIKI_DIR" >&2; exit 1; }
[[ -d "$SOURCE_DIR" ]] || { echo "docs directory is missing: $SOURCE_DIR" >&2; exit 1; }

managed_manifest="$WIKI_DIR/.queue-aware-vgpu-managed-pages"
previous_managed=()
if [[ -f "$managed_manifest" ]]; then
  while IFS= read -r page; do
    if [[ -n "$page" ]]; then
      previous_managed+=("$page")
      rm -f "$WIKI_DIR/$page"
    fi
  done < "$managed_manifest"
fi

managed=("Home.md" "Index.md" "_Sidebar.md")
cp "$SOURCE_DIR/00-overview.md" "$WIKI_DIR/Home.md"
cp "$SOURCE_DIR/README.md" "$WIKI_DIR/Index.md"

shopt -s nullglob
for source in "$SOURCE_DIR"/[0-9][0-9]-*.md; do
  page="$(basename "$source")"
  cp "$source" "$WIKI_DIR/$page"
  managed+=("$page")
done
shopt -u nullglob

cat > "$WIKI_DIR/_Sidebar.md" <<'EOF'
## Queue-aware vGPU

- [總覽](Home)
- [平台](01-platform)
- [Gate / Queue / vGPU](02-gate-queue-vgpu)
- [Controller loop](03-controller-loop)
- [四種 mode](04-modes)
- [Code map](05-code-map)
- [復現步驟](06-reproduce)
- [實驗](07-experiments)
- [明天報告稿](08-teacher-talk)
- [Rust primer](09-rust-primer)
EOF
managed+=(".queue-aware-vgpu-managed-pages")
printf '%s\n' "${managed[@]}" > "$managed_manifest"

git -C "$WIKI_DIR" add -A -- "${managed[@]}" "${previous_managed[@]}"

if git -C "$WIKI_DIR" diff --cached --quiet; then
  echo "wiki sync: no changes"
  exit 0
fi

if [[ "$PUSH" != true ]]; then
  echo "wiki sync: files prepared (not pushed)"
  exit 0
fi

git -C "$WIKI_DIR" config user.name "github-actions[bot]"
git -C "$WIKI_DIR" config user.email "41898282+github-actions[bot]@users.noreply.github.com"
git -C "$WIKI_DIR" commit -m "docs: sync repository guide"
git -C "$WIKI_DIR" push origin HEAD
echo "wiki sync: pushed"
