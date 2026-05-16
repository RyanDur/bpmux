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

# Piped-stdin: rustdoc /// → pass.
RUSTDOC_OK=$(printf '+/// a rustdoc comment\n+pub fn foo() {}')
run_case "piped-rustdoc-ok" "$RUSTDOC_OK" "pass" "piped"

# Piped-stdin: module-doc //! → pass.
MODULE_DOC_OK=$(printf '+//! a module doc\n+pub fn foo() {}')
run_case "piped-module-doc-ok" "$MODULE_DOC_OK" "pass" "piped"

# Piped-stdin: empty diff → pass (sanity).
run_case "piped-empty-diff" "" "pass" "piped"

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
