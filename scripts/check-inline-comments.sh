#!/bin/bash
set -euo pipefail

DIFF_OUTPUT=""
if [ ! -t 0 ]; then
    DIFF_OUTPUT=$(cat)
fi
if [ -z "$DIFF_OUTPUT" ]; then
    REPO_ROOT="$(git rev-parse --show-toplevel)"
    cd "$REPO_ROOT"
    DIFF_OUTPUT=$(git diff --cached --unified=0 -- '*.rs' '*.ts' '*.tsx' '*.js' '*.jsx' '*.css' 2>/dev/null)
fi

violations=$(
    echo "$DIFF_OUTPUT" | awk '
    BEGIN { file = "<unknown>"; line = 0 }
    /^\+\+\+ b\// {
        f = $0
        sub(/^\+\+\+ b\//, "", f)
        file = f
        next
    }
    /^@@ / {
        if (match($0, /\+[0-9]+/)) {
            line = substr($0, RSTART + 1, RLENGTH - 1) + 0
        }
        next
    }
    /^\+\+\+/ { next }
    /^---/ { next }
    /^\+/ {
        text = substr($0, 2)
        stripped = text
        sub(/^[[:space:]]+/, "", stripped)
        if (file ~ /\.css$/) {
            if (file !~ /reset\.css$/ && stripped ~ /^\/\*/ && stripped !~ /^\/\* keep[+]flag\((a11y|motion|placeholder|layout-edge)(, (interim|until: #[0-9]+))?\): /) {
                printf "%s:%d: css comment without keep+flag(<axis>) marker — retire it, or mark per kb/documentation-honest-homes (axis = a11y|motion|placeholder|layout-edge)\n", file, line
            }
            line++
            next
        }
        cls = ""
        if (stripped ~ /^\/\/!/) cls = "//!"
        else if (stripped ~ /^\/\/\//) cls = "///"
        else if (stripped ~ /^\/\//) cls = "//"
        else if (stripped ~ /^\/\*/) cls = "/* */"
        if (cls != "") {
            printf "%s:%d: comment class \047%s\047 detected — bpmux is application not library; build understanding into code per Ryan-canon 2026-05-18\n", file, line, cls
        }
        line++
        next
    }
    /^-/ { next }
    /^ / { line++; next }
    '
)

if [ -n "$violations" ]; then
    echo "ERROR: comment-class violations in staged diff." >&2
    echo "Per feedback_no_comments_absolute + feedback_no_rustdoc_bpmux_not_library + feedback_absolute_rules_no_carveouts." >&2
    echo "" >&2
    echo "$violations" >&2
    echo "" >&2
    echo "bpmux is application not library — build understanding into code via names + types + structure, not comments." >&2
    exit 1
fi

exit 0
