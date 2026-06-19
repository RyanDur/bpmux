#!/bin/bash
set -e

test "$CARGO_HOME" = "$RUNNER_TEMP/cargo-home" || { echo "CARGO_HOME not redirected"; exit 1; }
test "$RUSTUP_HOME" = "$RUNNER_TEMP/rustup-home" || { echo "RUSTUP_HOME not redirected"; exit 1; }
test -d "$RUNNER_TEMP/cargo-home" || { echo "isolated cargo-home not populated"; exit 1; }
test -d "$RUNNER_TEMP/rustup-home" || { echo "isolated rustup-home not populated"; exit 1; }

if ! find "$RUNNER_TEMP/cargo-home/registry" -type f 2>/dev/null | grep -q .; then
    echo "REGRESSION: \$RUNNER_TEMP/cargo-home/registry is empty — cargo did NOT write its registry to the isolated CARGO_HOME."
    echo "This means the CARGO_HOME redirect via \$GITHUB_ENV was not honored, so dependency downloads would land in owner's ~/.cargo."
    echo "Falsifier-fidelity: cargo populates the isolated registry on every build/test when isolation holds, on warm and cold runners alike; an empty registry empirically means the redirect broke."
    exit 1
fi
if ! find "$RUNNER_TEMP/rustup-home" -maxdepth 2 -type d 2>/dev/null | grep -q "toolchains"; then
    echo "REGRESSION: \$RUNNER_TEMP/rustup-home/toolchains missing — rustup state did NOT populate the isolated path."
    echo "Falsifier-fidelity: RUSTUP_HOME isolation broken."
    exit 1
fi
echo "\$HOME-isolation gate passed: env-vars redirected + cargo registry written to isolated CARGO_HOME + rustup toolchains in isolated RUSTUP_HOME"
