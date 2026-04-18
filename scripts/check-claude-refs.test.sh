#!/usr/bin/env bash
# Test coverage for check-claude-refs.sh (#283).
#
# Each case builds a synthetic CLAUDE.md in a tempdir, invokes the
# audit script against that file only, and asserts the expected
# output shape + exit code. Tests are independent and self-cleaning.

set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd)"
audit="$script_dir/check-claude-refs.sh"

if [ ! -x "$audit" ]; then
  printf 'ERROR: audit script not executable: %s\n' "$audit" >&2
  exit 1
fi

pass=0
fail=0

run_case() {
  local name="$1"
  local claude_md="$2"
  local expect_exit="$3"
  local expect_contains="$4"

  local actual_out actual_exit
  actual_out="$("$audit" "$claude_md" 2>&1 || true)"
  actual_exit=$("$audit" "$claude_md" >/dev/null 2>&1; echo "$?" || echo "?")

  if [ "$actual_exit" != "$expect_exit" ]; then
    printf 'FAIL %s: expected exit %s, got %s\n' "$name" "$expect_exit" "$actual_exit"
    printf '  output:\n%s\n' "$actual_out"
    fail=$((fail + 1))
    return
  fi
  if [ -n "$expect_contains" ] && ! grep -q -- "$expect_contains" <<<"$actual_out"; then
    printf 'FAIL %s: expected output to contain %q\n' "$name" "$expect_contains"
    printf '  output:\n%s\n' "$actual_out"
    fail=$((fail + 1))
    return
  fi
  printf 'PASS %s\n' "$name"
  pass=$((pass + 1))
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# Case 1: resolvable markdown-link reference → clean pass
mkdir -p "$tmp/case1"
touch "$tmp/case1/real.md"
cat > "$tmp/case1/CLAUDE.md" <<'EOF'
# case1
See [real](./real.md) for details.
EOF
run_case "resolvable markdown link exits clean" \
  "$tmp/case1/CLAUDE.md" "0" "0 stale"

# Case 2: stale markdown link → exit 1, reports STALE
mkdir -p "$tmp/case2"
cat > "$tmp/case2/CLAUDE.md" <<'EOF'
# case2
See [ghost](./ghost.md) which was deleted.
EOF
run_case "stale markdown link is detected" \
  "$tmp/case2/CLAUDE.md" "1" "STALE"

# Case 3: resolvable inline-backtick path → clean pass
mkdir -p "$tmp/case3/sub"
touch "$tmp/case3/sub/file.md"
cat > "$tmp/case3/CLAUDE.md" <<'EOF'
# case3
Refers to `./sub/file.md` which exists.
EOF
run_case "resolvable inline-backtick path exits clean" \
  "$tmp/case3/CLAUDE.md" "0" "0 stale"

# Case 4: stale inline-backtick path → detected
mkdir -p "$tmp/case4"
cat > "$tmp/case4/CLAUDE.md" <<'EOF'
# case4
Refers to `./missing/file.md` which was deleted.
EOF
run_case "stale inline-backtick path is detected" \
  "$tmp/case4/CLAUDE.md" "1" "STALE"

# Case 5: fenced code block with shell commands → NOT flagged
mkdir -p "$tmp/case5"
cat > "$tmp/case5/CLAUDE.md" <<'EOF'
# case5
```bash
cd ../elsewhere
git -C ../elsewhere status
```
EOF
run_case "fenced shell commands do not count as refs" \
  "$tmp/case5/CLAUDE.md" "0" "0 stale"

# Case 6: prose backticks without ./ or ../ prefix → NOT flagged
mkdir -p "$tmp/case6"
cat > "$tmp/case6/CLAUDE.md" <<'EOF'
# case6
The `claude-plugin/` directory holds hook scripts.
EOF
run_case "prose backticks without path prefix are skipped" \
  "$tmp/case6/CLAUDE.md" "0" "0 stale"

# Case 7: URLs in markdown links → not flagged as missing paths
mkdir -p "$tmp/case7"
cat > "$tmp/case7/CLAUDE.md" <<'EOF'
# case7
See [docs](https://example.com/docs) for more.
EOF
run_case "URL-destination links are skipped" \
  "$tmp/case7/CLAUDE.md" "0" "0 stale"

# Case 8: mixed valid + stale → reports only the stale one
mkdir -p "$tmp/case8"
touch "$tmp/case8/live.md"
cat > "$tmp/case8/CLAUDE.md" <<'EOF'
# case8
See [live](./live.md) (resolves) and [ghost](./ghost.md) (stale).
EOF
out8="$("$audit" "$tmp/case8/CLAUDE.md" 2>&1 || true)"
if ! grep -q 'ghost.md' <<<"$out8"; then
  printf 'FAIL mixed case did not report ghost.md\n  output:\n%s\n' "$out8"
  fail=$((fail + 1))
elif grep -q 'live.md' <<<"$out8"; then
  printf 'FAIL mixed case wrongly flagged live.md\n  output:\n%s\n' "$out8"
  fail=$((fail + 1))
else
  printf 'PASS mixed case flags only stale refs\n'
  pass=$((pass + 1))
fi

printf '\n%d passed, %d failed\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
