#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GATE="$SCRIPT_DIR/verify-home-isolation.sh"

pass() { echo "  PASS: $1"; }
fail() { echo "FAIL: $1" >&2; exit 1; }

setup_sandbox() {
    local sandbox="$1"
    mkdir -p "$sandbox/cargo-home/bin"
    mkdir -p "$sandbox/rustup-home/toolchains/stable"
    echo "#!/bin/sh" > "$sandbox/cargo-home/bin/rustup"
    chmod +x "$sandbox/cargo-home/bin/rustup"
}

run_gate() {
    local sandbox="$1"
    RUNNER_TEMP="$sandbox" \
    CARGO_HOME="$sandbox/cargo-home" \
    RUSTUP_HOME="$sandbox/rustup-home" \
    "$GATE"
}

SANDBOX=$(mktemp -d)
trap "rm -rf $SANDBOX" EXIT

echo "Running test-verify-home-isolation..."

setup_sandbox "$SANDBOX"
if ! run_gate "$SANDBOX" >/dev/null 2>&1; then
    fail "isolation-honored scenario (expected zero-exit; got non-zero)"
fi
pass "isolation-honored: env-vars + isolated-bin populated + rustup toolchains present → gate PASS"

REVERTED_SANDBOX=$(mktemp -d)
mkdir -p "$REVERTED_SANDBOX/cargo-home"
mkdir -p "$REVERTED_SANDBOX/rustup-home"
if run_gate "$REVERTED_SANDBOX" >/dev/null 2>&1; then
    fail "isolation-broken-bin-empty scenario (expected non-zero; got zero)"
fi
pass "isolation-broken (bin empty): gate empirically FAILS — falsifier-fidelity proven"
rm -rf "$REVERTED_SANDBOX"

PARTIAL_SANDBOX=$(mktemp -d)
mkdir -p "$PARTIAL_SANDBOX/cargo-home/bin"
mkdir -p "$PARTIAL_SANDBOX/rustup-home"
echo "#!/bin/sh" > "$PARTIAL_SANDBOX/cargo-home/bin/rustup"
chmod +x "$PARTIAL_SANDBOX/cargo-home/bin/rustup"
if run_gate "$PARTIAL_SANDBOX" >/dev/null 2>&1; then
    fail "rustup-toolchains-missing scenario (expected non-zero; got zero)"
fi
pass "rustup-toolchains missing: gate empirically FAILS — sister falsifier-fidelity"
rm -rf "$PARTIAL_SANDBOX"

WRONG_ENV_SANDBOX=$(mktemp -d)
setup_sandbox "$WRONG_ENV_SANDBOX"
if RUNNER_TEMP="$WRONG_ENV_SANDBOX" CARGO_HOME="/wrong-path" RUSTUP_HOME="$WRONG_ENV_SANDBOX/rustup-home" "$GATE" >/dev/null 2>&1; then
    fail "CARGO_HOME-misdirected scenario (expected non-zero; got zero)"
fi
pass "CARGO_HOME misdirected: gate empirically FAILS — env-var-check still active"
rm -rf "$WRONG_ENV_SANDBOX"

echo "All falsifier-fidelity tests passed."
