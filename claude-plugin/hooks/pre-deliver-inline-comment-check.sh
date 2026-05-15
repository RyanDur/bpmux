#!/usr/bin/env bash
set -euo pipefail

role="${BPMUX_ROLE:-}"
case "$role" in
  dev) ;;
  *) exit 0 ;;
esac

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [ -z "$repo_root" ]; then
  exit 0
fi

check_script="$repo_root/scripts/check-inline-comments.sh"
if [ ! -x "$check_script" ]; then
  exit 0
fi

set +e
diff_output=$(git -C "$repo_root" diff origin/main..HEAD --unified=0 -- '*.rs' '*.ts' '*.tsx' '*.js' '*.jsx' 2>/dev/null)
violations_output=$(printf '%s' "$diff_output" | "$check_script" 2>&1 1>/dev/null)
violations_exit=$?
set -e

if [ "$violations_exit" -eq 0 ]; then
  exit 0
fi

advisory="[deliver_story advisory] inline-// absolute-rule violations detected in this branch vs main. Per feedback_no_comments_absolute. Pre-commit hook should have caught at commit-time; if not enabled, install scripts/git-hooks/pre-commit as .git/hooks/pre-commit. Violations:\\n$(printf '%s' "$violations_output" | head -20)"

escaped="${advisory//\\/\\\\}"
escaped="${escaped//\"/\\\"}"
escaped="${escaped//$'\n'/\\n}"

printf '{"systemMessage":"%s"}\n' "$escaped"
