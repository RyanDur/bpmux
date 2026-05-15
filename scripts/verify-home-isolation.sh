#!/bin/bash
set -e

test "$CARGO_HOME" = "$RUNNER_TEMP/cargo-home" || { echo "CARGO_HOME not redirected"; exit 1; }
test "$RUSTUP_HOME" = "$RUNNER_TEMP/rustup-home" || { echo "RUSTUP_HOME not redirected"; exit 1; }
test -d "$RUNNER_TEMP/cargo-home" || { echo "isolated cargo-home not populated"; exit 1; }
test -d "$RUNNER_TEMP/rustup-home" || { echo "isolated rustup-home not populated"; exit 1; }

if ! find "$RUNNER_TEMP/cargo-home/bin" -maxdepth 1 -type f 2>/dev/null | grep -q .; then
    echo "REGRESSION: \$RUNNER_TEMP/cargo-home/bin is empty — dtolnay/rust-toolchain did NOT write binaries to the isolated path."
    echo "This means CARGO_HOME redirect via \$GITHUB_ENV was not honored by the action, OR the Isolate-step was reverted/skipped."
    echo "Falsifier-fidelity per feedback_falsifier_load_bearing_test_at_e2e_layer: this regression gate empirically fails when isolation broken."
    exit 1
fi
if ! find "$RUNNER_TEMP/rustup-home" -maxdepth 2 -type d 2>/dev/null | grep -q "toolchains"; then
    echo "REGRESSION: \$RUNNER_TEMP/rustup-home/toolchains missing — rustup state did NOT populate the isolated path."
    echo "Falsifier-fidelity: RUSTUP_HOME isolation broken."
    exit 1
fi
echo "\$HOME-isolation gate passed: env-vars redirected + binaries actually written to isolated CARGO_HOME + rustup toolchains in isolated RUSTUP_HOME"
