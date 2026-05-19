#!/usr/bin/env bash
# Test suite for scripts/check-inline-comments.sh (#1373).
# Pins both invocation modes: piped-stdin (CI shape) + closed-stdin
# (pre-commit hook shape). Earlier bug: closed-stdin path read `cat`
# which returned empty → 0 violations → false-pass.
set -u

SCRIPT="$(cd "$(dirname "$0")/.." && pwd)/check-inline-comments.sh"
PASS=0
FAIL=0

run_case() {
  local label="$1"
  local input="$2"
  local expect="$3"  # "pass" | "fail"
  local mode="$4"    # "piped" | "closed"

  local got
  if [ "$mode" = "piped" ]; then
    if echo "$input" | "$SCRIPT" >/dev/null 2>&1; then
      got="pass"
    else
      got="fail"
    fi
  else
    # Closed-stdin: redirect /dev/null. Forces the script's empty-
    # stdin fallback to git diff --cached. Since this is a test, the
    # cached diff is whatever the user has staged — neutral environment
    # is assumed. Don't run "closed" cases in this mode that assume
    # specific content from cache; we test the FALLBACK shape only.
    if "$SCRIPT" </dev/null >/dev/null 2>&1; then
      got="pass"
    else
      got="fail"
    fi
  fi
  if [ "$got" = "$expect" ]; then
    echo "PASS  [$label]"
    PASS=$((PASS+1))
  else
    echo "FAIL  [$label] expected=$expect got=$got mode=$mode"
    FAIL=$((FAIL+1))
  fi
}

# Piped-stdin: clean diff → pass.
run_case "piped-clean" "+pub fn foo() {}" "pass" "piped"

# Piped-stdin: diff with added inline-// → fail.
INLINE_VIOLATION=$(printf '+// dropping a comment here\n+pub fn foo() {}')
run_case "piped-inline-violation" "$INLINE_VIOLATION" "fail" "piped"

# Piped-stdin: rustdoc /// → fail (#1422 — Ryan-canon "build understanding into code").
RUSTDOC_VIOLATION=$(printf '+/// a rustdoc comment\n+pub fn foo() {}')
run_case "piped-rustdoc-violation" "$RUSTDOC_VIOLATION" "fail" "piped"

# Piped-stdin: module-doc //! → fail (#1422).
MODULE_DOC_VIOLATION=$(printf '+//! a module doc\n+pub fn foo() {}')
run_case "piped-module-doc-violation" "$MODULE_DOC_VIOLATION" "fail" "piped"

# Piped-stdin: block-comment /* */ → fail (#1422).
BLOCK_VIOLATION=$(printf '+/* a block comment */\n+pub fn foo() {}')
run_case "piped-block-comment-violation" "$BLOCK_VIOLATION" "fail" "piped"

# Piped-stdin: URL in string-literal → pass (#1422 false-positive guard).
URL_OK=$(printf '+pub fn foo() { let url = "https://example.com"; }')
run_case "piped-url-in-string-ok" "$URL_OK" "pass" "piped"

# Piped-stdin: /* */ inside string-literal → pass (#1422 false-positive guard).
BLOCK_IN_STRING=$(printf '+pub fn foo() { let s = "this /\052 not a comment \052/ either"; }')
run_case "piped-block-in-string-ok" "$BLOCK_IN_STRING" "pass" "piped"

# Piped-stdin: empty diff → pass (sanity).
run_case "piped-empty-diff" "" "pass" "piped"

# Piped-stdin: structured-error-shape — verify file:line + class + canon-cite (#1422 falsifier-g).
STRUCTURED_INPUT=$(printf '+++ b/foo.rs\n@@ -1,0 +1,1 @@\n+/// docstring on a fn')
STRUCTURED_ERR=$(echo "$STRUCTURED_INPUT" | "$SCRIPT" 2>&1 || true)
if echo "$STRUCTURED_ERR" | grep -q "foo.rs:1:" \
   && echo "$STRUCTURED_ERR" | grep -q "comment class '///'" \
   && echo "$STRUCTURED_ERR" | grep -q "Ryan-canon 2026-05-18"; then
  echo "PASS  [structured-error-shape]"
  PASS=$((PASS+1))
else
  echo "FAIL  [structured-error-shape] expected file:line + class + canon-cite; got: $STRUCTURED_ERR"
  FAIL=$((FAIL+1))
fi

# Closed-stdin: triggers git diff --cached fallback. We can't
# deterministically assert pass/fail (depends on user's staged state),
# but we CAN confirm the script doesn't blow up on the closed-stdin
# path. Hermetic shape: cd into a clean tempdir + init a fresh git repo.
TMP=$(mktemp -d)
pushd "$TMP" >/dev/null
git init -q
git config user.email "test@example.com"
git config user.name "test"
echo "fn main() {}" > clean.rs
git add clean.rs
if "$SCRIPT" </dev/null >/dev/null 2>&1; then
  echo "PASS  [closed-stdin-fallback-clean]"
  PASS=$((PASS+1))
else
  echo "FAIL  [closed-stdin-fallback-clean] expected pass on clean staged file"
  FAIL=$((FAIL+1))
fi

# Hermetic-closed-stdin with inline-// violation → must FAIL.
cat > dirty.rs <<'EOF'
// dropping a comment here
fn main() {}
EOF
git add dirty.rs
if "$SCRIPT" </dev/null >/dev/null 2>&1; then
  echo "FAIL  [closed-stdin-fallback-violation] expected fail on staged inline-//"
  FAIL=$((FAIL+1))
else
  echo "PASS  [closed-stdin-fallback-violation]"
  PASS=$((PASS+1))
fi
popd >/dev/null
rm -rf "$TMP"

echo
echo "==================================="
echo "PASS: $PASS  FAIL: $FAIL"
echo "==================================="
exit $FAIL
