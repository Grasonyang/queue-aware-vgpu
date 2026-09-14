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

Copies the repository Markdown architecture pages into an existing GitHub Wiki checkout.
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

managed=("_Sidebar.md")

page_name() {
  local relative="$1"
  case "$relative" in
    arch/v1.md) echo "Home.md" ;;
    arch/domain/README.md) echo "domain.md" ;;
    arch/domain/*.md) echo "domain-$(basename "$relative")" ;;
    *) echo "$(basename "$relative")" ;;
  esac
}

rewrite_links() {
  sed \
    -e 's#](v1\.md)#](Home)#g' \
    -e 's#](domain/README\.md)#](domain)#g' \
    -e 's#](\.\./dev/deep-module-design\.md)#](deep-module-design)#g' \
    -e 's#](\.\./dev/research-scope-and-boundaries\.md)#](research-scope-and-boundaries)#g'
}

mapfile -d '' markdown_files < <(find "$SOURCE_DIR" -type f -name '*.md' -print0 | sort -z)
for source in "${markdown_files[@]}"; do
  relative="${source#"$SOURCE_DIR/"}"
  page="$(page_name "$relative")"
  rewrite_links < "$source" > "$WIKI_DIR/$page"
  managed+=("$page")
done

cat > "$WIKI_DIR/_Sidebar.md" <<'EOF'
## Queue-aware vGPU

- [V1 架構](Home)
- [V1 架構圖](v1-architecture-image)
- [V1 Domain contracts](v1-domain-contracts)
- [Domain 工具地圖](domain)
- [Identity](domain-ids)
- [GPU 記憶體](domain-memory)
- [Cluster 治理](domain-cluster)
- [租戶 Queue](domain-queue)
- [Job 需求](domain-job)
- [排程決策](domain-decision)
- [Workload lifecycle](domain-lifecycle)
- [Reservation](domain-reservation)
- [Deep Module Design](deep-module-design)
- [研究範圍與邊界](research-scope-and-boundaries)
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
