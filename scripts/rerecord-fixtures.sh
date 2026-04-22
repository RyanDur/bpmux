#!/bin/bash
# Re-record contract-test fixtures from the live GitHub API + gh CLI.
#
# Run when:
# - The feature-gated integration suite (cargo test --features
#   integration_tests) surfaces drift from recorded fixtures
# - bpmux starts calling a new gh endpoint
# - GitHub changes a response field name or adds required fields
#
# Requires GH_TOKEN + BPMUX_REPO env vars to be set. Writes to:
# - applications/bpmux/tests/fixtures/gh/*.json  (gh CLI subprocess output)
#
# The httpmock-based octocrab suite does NOT record to disk — it
# builds responses inline from typed helpers. Re-recording that
# path means updating the `issue()` / `comment()` / `label()` helpers
# in applications/bpmux/src/bridge/tracker_contract_tests.rs if
# octocrab's model definitions change (rare; bump on octocrab upgrades).
#
# Exits non-zero if prerequisites are missing so CI doesn't silently
# no-op when the operator intended a refresh.

set -euo pipefail

if [ -z "${BPMUX_REPO:-}" ]; then
    echo "ERROR: BPMUX_REPO not set (expected owner/name)." >&2
    exit 2
fi
if ! command -v gh >/dev/null 2>&1; then
    echo "ERROR: gh CLI not found on PATH." >&2
    exit 2
fi
if ! gh auth status >/dev/null 2>&1; then
    echo "ERROR: gh CLI not authenticated. Run 'gh auth login' first." >&2
    exit 2
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FIXTURE_DIR="$REPO_ROOT/applications/bpmux/tests/fixtures/gh"
mkdir -p "$FIXTURE_DIR"

echo "Re-recording gh fixtures against $BPMUX_REPO..."

# Issue list — the shape handle_view_board parses
echo "  issue_list_open.json"
gh -R "$BPMUX_REPO" issue list --state all --limit 3 --json number,title,state,labels \
    > "$FIXTURE_DIR/issue_list_open.json.tmp"
mv "$FIXTURE_DIR/issue_list_open.json.tmp" "$FIXTURE_DIR/issue_list_open.json"

# Recent CI run — the shape terminal::ci::poll_once parses
echo "  run_list_*.json (picks most recent run; may be success or failure)"
gh -R "$BPMUX_REPO" run list -L 1 --branch main \
    --json status,conclusion,headSha,displayTitle,databaseId \
    > "$FIXTURE_DIR/run_list_current.json.tmp"
mv "$FIXTURE_DIR/run_list_current.json.tmp" "$FIXTURE_DIR/run_list_current.json"

echo ""
echo "Re-record complete. Note: run_list_success.json / run_list_failure.json"
echo "are curated exemplars — do NOT overwrite them with whatever the live"
echo "repo happens to be in right now. Edit run_list_current.json into the"
echo "right exemplar if the contract shape actually changed."
echo ""
echo "Review the diff:  git diff $FIXTURE_DIR"
echo "Then re-run:      cargo test"
