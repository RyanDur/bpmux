#!/bin/bash
# Lint gate over staged files only — Slice 1 of Epic #1477 frontend
# canon-cluster (#1478). New code can't introduce eslint/stylelint
# violations; pre-existing baseline (190 eslint + 166 stylelint
# errors as of 2026-05-20) is tolerated. Slices 2-N retire baseline
# per-component-cluster.
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT/applications/electron"

STAGED_TS=$(git diff --name-only --cached --diff-filter=ACMR \
  | grep -E '^applications/electron/(src|tests)/.*\.(ts|tsx)$' \
  | sed 's|^applications/electron/||' || true)

STAGED_CSS=$(git diff --name-only --cached --diff-filter=ACMR \
  | grep -E '^applications/electron/src/.*\.css$' \
  | sed 's|^applications/electron/||' || true)

ts_exit=0
css_exit=0

if [ -n "$STAGED_TS" ]; then
  echo "$STAGED_TS" | xargs npx eslint --no-warn-ignored || ts_exit=$?
fi

if [ -n "$STAGED_CSS" ]; then
  echo "$STAGED_CSS" | xargs npx stylelint || css_exit=$?
fi

[ $ts_exit -ne 0 ] && exit $ts_exit
[ $css_exit -ne 0 ] && exit $css_exit
exit 0
