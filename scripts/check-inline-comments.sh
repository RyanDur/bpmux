#!/bin/bash
set -euo pipefail

if [ -t 0 ]; then
    REPO_ROOT="$(git rev-parse --show-toplevel)"
    cd "$REPO_ROOT"
    DIFF_OUTPUT=$(git diff --cached --unified=0 -- '*.rs' '*.ts' '*.tsx' '*.js' '*.jsx' 2>/dev/null)
else
    DIFF_OUTPUT=$(cat)
fi

violations=$(
    echo "$DIFF_OUTPUT" \
    | grep -E '^\+[^+]' \
    | grep -E '^\+[[:space:]]*//' \
    | grep -vE '^\+[[:space:]]*///' \
    | grep -vE '^\+[[:space:]]*//!' \
    || true
)

if [ -n "$violations" ]; then
    echo "ERROR: inline-// absolute-rule violations in staged diff." >&2
    echo "Per feedback_no_comments_absolute + feedback_absolute_rules_no_carveouts." >&2
    echo "" >&2
    echo "Offending added lines:" >&2
    echo "$violations" >&2
    echo "" >&2
    echo "Delete the inline // blocks before committing." >&2
    echo "Rustdoc /// and module-doc //! and JSDoc /** */ pass." >&2
    exit 1
fi

exit 0
