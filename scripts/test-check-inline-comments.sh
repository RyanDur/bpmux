#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CHECK="$SCRIPT_DIR/check-inline-comments.sh"

fail() { echo "FAIL: $1" >&2; exit 1; }
pass() { echo "  PASS: $1"; }

assert_violation() {
    local name="$1"
    local input="$2"
    if echo "$input" | "$CHECK" >/dev/null 2>&1; then
        fail "$name (expected non-zero exit; got zero)"
    fi
    pass "$name"
}

assert_no_violation() {
    local name="$1"
    local input="$2"
    if ! echo "$input" | "$CHECK" >/dev/null 2>&1; then
        fail "$name (expected zero exit; got non-zero)"
    fi
    pass "$name"
}

echo "Running test-check-inline-comments..."

assert_violation "added inline // in .rs" "$(cat <<'EOF'
diff --git a/foo.rs b/foo.rs
--- a/foo.rs
+++ b/foo.rs
@@ -1,1 +1,2 @@
+    // file-purpose explanation
fn main() {}
EOF
)"

assert_violation "rustdoc /// rejected (#1422)" "$(cat <<'EOF'
diff --git a/foo.rs b/foo.rs
--- a/foo.rs
+++ b/foo.rs
@@ -1,1 +1,2 @@
+/// public-fn doc
pub fn x() {}
EOF
)"

assert_violation "module-doc //! rejected (#1422)" "$(cat <<'EOF'
diff --git a/foo.rs b/foo.rs
--- a/foo.rs
+++ b/foo.rs
@@ -1,1 +1,2 @@
+//! module-level docs
use std::path;
EOF
)"

assert_violation "added inline // in .ts" "$(cat <<'EOF'
diff --git a/app.ts b/app.ts
--- a/app.ts
+++ b/app.ts
@@ -1,1 +1,2 @@
+    // explanatory inline
export function x() {}
EOF
)"

assert_violation "added inline // in .tsx" "$(cat <<'EOF'
diff --git a/App.tsx b/App.tsx
--- a/App.tsx
+++ b/App.tsx
@@ -1,1 +1,2 @@
+// component file-purpose
export default function App() {}
EOF
)"

assert_violation "JSDoc /** */ block rejected (#1422)" "$(cat <<'EOF'
diff --git a/app.ts b/app.ts
--- a/app.ts
+++ b/app.ts
@@ -1,1 +1,3 @@
+/** jsdoc block */
+export function x() {}
fn main() {}
EOF
)"

assert_violation "block-comment /* */ rejected (#1422)" "$(cat <<'EOF'
diff --git a/foo.rs b/foo.rs
--- a/foo.rs
+++ b/foo.rs
@@ -1,1 +1,2 @@
+    /* block comment */
fn main() {}
EOF
)"

assert_no_violation "URL in string-literal passes (false-positive guard #1422)" "$(cat <<'EOF'
diff --git a/foo.rs b/foo.rs
--- a/foo.rs
+++ b/foo.rs
@@ -1,1 +1,2 @@
+    let url = "https://example.com";
fn main() {}
EOF
)"

assert_no_violation "block-token in string-literal passes (false-positive guard #1422)" "$(cat <<'EOF'
diff --git a/foo.rs b/foo.rs
--- a/foo.rs
+++ b/foo.rs
@@ -1,1 +1,2 @@
+    let s = "this isn't a /\052 comment \052/ either";
fn main() {}
EOF
)"

assert_no_violation "removed inline-// is fine" "$(cat <<'EOF'
diff --git a/foo.rs b/foo.rs
--- a/foo.rs
+++ b/foo.rs
@@ -1,3 +1,2 @@
fn main() {
-    // existing comment getting removed
let x = 1;
EOF
)"

assert_no_violation "pre-existing // in context is fine" "$(cat <<'EOF'
diff --git a/foo.rs b/foo.rs
--- a/foo.rs
+++ b/foo.rs
@@ -1,2 +1,3 @@
// pre-existing
fn main() {
+    let x = 1;
EOF
)"

assert_no_violation "diff header +++ lines excluded" "$(cat <<'EOF'
diff --git a/foo.rs b/foo.rs
--- a/foo.rs
+++ b/foo.rs
@@ -1,1 +1,2 @@
fn main() {}
EOF
)"

assert_violation "multiple violations detected" "$(cat <<'EOF'
diff --git a/foo.rs b/foo.rs
--- a/foo.rs
+++ b/foo.rs
@@ -1,1 +1,4 @@
fn main() {
+// first violation
+// second violation
+let x = 1;
EOF
)"

assert_no_violation "empty diff passes" ""

echo "All tests passed."
