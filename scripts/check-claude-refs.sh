#!/usr/bin/env bash
# check-claude-refs.sh — audit CLAUDE.md path references (#283).
#
# Walks the workspace + repo CLAUDE.md files, extracts inline-backtick
# and markdown-link path references, and checks each referenced path
# resolves relative to the CLAUDE.md file's directory. Reports stale
# references with file / line / referenced path; exits non-zero if
# any stale ref is found.
#
# Usage:
#   scripts/check-claude-refs.sh                    # audit default set
#   scripts/check-claude-refs.sh path/to/CLAUDE.md  # audit specific file
#
# Why: 2026-04-17 found three of four references in each workspace
# CLAUDE.md were stale — welcome.md / roles/{role}.md / AGENT_WORKFLOW.md
# deleted 2026-04-08 but workspace references not updated, silently
# broken for 9 days. This tool catches similar decay before it hides
# again.
#
# What counts as a reference:
# - Markdown-link destinations that start with `./`, `../`, or `/`.
# - Inline-backticked strings that start with `./`, `../`, or `/`.
# Plain backtick-wrapped identifiers (e.g. `claude-plugin/`) are prose
# mentions, not path references, and are skipped. This reduces false
# positives on documentation that names directories-as-concepts rather
# than as filesystem references.
#
# Fenced code blocks (```...```) are skipped entirely so shell example
# commands (e.g. `cd ../bpmux`) aren't mistaken for path references.

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
workspaces_root="$(cd "$repo_root/.." && pwd)"

default_files=(
  "$repo_root/CLAUDE.md"
  "$workspaces_root/bpmux-dev/CLAUDE.md"
  "$workspaces_root/bpmux-pm/CLAUDE.md"
  "$workspaces_root/bpmux-designer/CLAUDE.md"
)

if [ "$#" -gt 0 ]; then
  files=("$@")
else
  files=("${default_files[@]}")
fi

stale_count=0
audited_refs=0

# Extract path references from a single non-fence line.
# Paths must begin with ./, ../, or / so prose mentions of
# backtick-wrapped directory names are not matched.
extract_paths() {
  local line="$1"
  # Markdown links: [text](dest). We only accept dest with a
  # leading path-prefix to avoid matching URLs or plain words.
  printf '%s\n' "$line" \
    | grep -oE '\[[^]]+\]\((\.{1,2}/|/)[^)]+\)' \
    | sed -E 's/^\[[^]]+\]\(([^)]+)\)$/\1/' || true
  # Inline-backticked paths: require ./, ../, or /
  printf '%s\n' "$line" \
    | grep -oE '`(\.{1,2}/|/)[^`]+`' \
    | sed -E 's/^`(.*)`$/\1/' || true
}

for claude_md in "${files[@]}"; do
  if [ ! -f "$claude_md" ]; then
    printf 'WARN: CLAUDE.md not found, skipping: %s\n' "$claude_md" >&2
    continue
  fi
  file_dir="$(dirname "$claude_md")"
  line_no=0
  in_fence=0
  while IFS= read -r line; do
    line_no=$((line_no + 1))
    # Toggle fenced-block state on ``` markers (optionally followed
    # by a language hint like ```bash). State flip happens AT the
    # fence line; the fence line itself is never parsed for paths.
    if [[ "$line" =~ ^\`\`\` ]]; then
      if [ "$in_fence" -eq 0 ]; then
        in_fence=1
      else
        in_fence=0
      fi
      continue
    fi
    [ "$in_fence" -eq 1 ] && continue

    while IFS= read -r ref; do
      [ -z "$ref" ] && continue
      # Trim whitespace
      ref="${ref#"${ref%%[![:space:]]*}"}"
      ref="${ref%"${ref##*[![:space:]]}"}"
      # Resolve relative to the CLAUDE.md's directory. Absolute
      # paths are taken as-is (pattern already ensures they start
      # with /, ./, or ../).
      if [[ "$ref" = /* ]]; then
        resolved="$ref"
      else
        resolved="$file_dir/$ref"
      fi
      audited_refs=$((audited_refs + 1))
      if [ ! -e "$resolved" ]; then
        printf 'STALE %s:%d → %s (resolves to %s)\n' \
          "$claude_md" "$line_no" "$ref" "$resolved"
        stale_count=$((stale_count + 1))
      fi
    done < <(extract_paths "$line")
  done < "$claude_md"
done

printf 'Audited %d references across %d CLAUDE.md files; %d stale.\n' \
  "$audited_refs" "${#files[@]}" "$stale_count" >&2

if [ "$stale_count" -gt 0 ]; then
  exit 1
fi
exit 0
