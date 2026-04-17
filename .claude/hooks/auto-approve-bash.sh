#!/usr/bin/env bash
#
# PreToolUse hook for Bash — auto-approves bash commands based on
# (cwd in whitelisted project dirs) AND (no match in a small denylist).
# Replaces the prefix-wildcard allowlist that breaks on pipes, heredocs,
# and common tools.
#
# Dry-run mode (DRY_RUN=1): log what would be approved but output empty JSON,
# letting Claude Code's normal prompting fire. Review the log for a session,
# then flip DRY_RUN=0 to enforce.
#
# Enforce mode (DRY_RUN=0): on match, output permissionDecision=allow. On
# dangerous pattern, non-project cwd, or non-Bash tool, output empty JSON
# (fall through to normal prompting).
#
# Known regex limits (not fixed by this script, surfaced as prompts instead
# of allows — the fail-safe direction): shell var expansion (`x=rm; $x -rf`),
# base64-decoded pipes, command substitution masking a dangerous token,
# relative-path traversal (`cat ../../other-project/secret`). For true
# containment of these cases, use an OS-level sandbox (nono / macOS sandbox
# profiles) outside this hook.

set -euo pipefail

DRY_RUN="${DRY_RUN:-0}"
# Default log path is inside /tmp (already in the path whitelist), so running
# `tail /tmp/bpmux-hook.log` while reviewing dry-run data doesn't itself
# trigger a prompt. Override with BPMUX_HOOK_LOG=/path if persistence needed.
LOG_FILE="${BPMUX_HOOK_LOG:-/tmp/bpmux-hook.log}"

mkdir -p "$(dirname "$LOG_FILE")"

# Read PreToolUse JSON from stdin.
input=$(cat)

# Extract tool_name and tool_input.command via jq. If jq isn't available or
# parsing fails, fall through to normal prompting.
tool_name=$(printf '%s' "$input" | jq -r '.tool_name // ""' 2>/dev/null || echo "")
command=$(printf '%s' "$input" | jq -r '.tool_input.command // ""' 2>/dev/null || echo "")

ts=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

log_decision() {
    local decision="$1"
    local reason="$2"
    local cmd_escaped
    cmd_escaped=$(printf '%s' "$command" | tr '\n' ' ' | head -c 500)
    printf '%s dry_run=%s cwd=%s tool=%s decision=%s reason=%s cmd=%q\n' \
        "$ts" "$DRY_RUN" "$PWD" "$tool_name" "$decision" "$reason" "$cmd_escaped" \
        >>"$LOG_FILE"
}

# Only act on Bash. Fall through for anything else.
if [[ "$tool_name" != "Bash" ]]; then
    log_decision "pass" "not-bash"
    echo '{}'
    exit 0
fi

# Cwd gate: auto-approval is scoped to known bpmux project workspaces.
# If we're running from anywhere else, fall through to normal prompting.
# The trailing /* glob means subdirectories are also in scope.
cwd_allowed=0
for allowed_prefix in \
    "$HOME/IdeaProjects/bpmux" \
    "$HOME/IdeaProjects/bpmux-dev" \
    "$HOME/IdeaProjects/bpmux-pm" \
    "$HOME/IdeaProjects/bpmux-designer"
do
    if [[ "$PWD" == "$allowed_prefix" || "$PWD" == "$allowed_prefix"/* ]]; then
        cwd_allowed=1
        break
    fi
done

if [[ "$cwd_allowed" != "1" ]]; then
    log_decision "pass" "cwd-not-in-whitelist"
    echo '{}'
    exit 0
fi

# Denylist: explicit, high-confidence dangerous patterns.
# If the command matches ANY of these, fall through to normal prompting so
# Ryan sees it.
deny_patterns=(
    # Filesystem destruction — short flags (allow extra letters in cluster)
    'rm[[:space:]]+-[a-zA-Z]*[rRf][a-zA-Z]*([[:space:]]|$)'
    # Filesystem destruction — long-form flags
    'rm[[:space:]]+(.*[[:space:]])?--recursive'
    'rm[[:space:]]+(.*[[:space:]])?--force'
    # Privilege escalation
    '(^|[[:space:]])sudo([[:space:]]|$)'
    '(^|[[:space:]])doas([[:space:]]|$)'
    # `sed` has wrecked files in this repo; Ryan's rule is Edit only.
    # Denylist here forces a prompt on every sed invocation so stray
    # uses surface before running. (Team discipline lives in the
    # `feedback_no_sed_use_edit` memory; this hook is the enforcement.)
    '(^|[[:space:]])sed([[:space:]]|$)'
    # Piping into an interpreter from a fetch — broad set of shells + scripting
    'curl[^|]*\|[[:space:]]*(sh|bash|zsh|fish|ksh|dash|python|python3|ruby|perl|node|php)([[:space:]]|$)'
    'wget[^|]*\|[[:space:]]*(sh|bash|zsh|fish|ksh|dash|python|python3|ruby|perl|node|php)([[:space:]]|$)'
    # Force-push destructive git (short, long, and refspec forms)
    'git[[:space:]]+push[^;&|]*--force'
    'git[[:space:]]+push[^;&|]*-f([[:space:]]|$)'
    'git[[:space:]]+push[^;&|]*[[:space:]]\+'
    'git[[:space:]]+reset[[:space:]]+--hard'
    'git[[:space:]]+clean[[:space:]]+-[fdx]'
    # Filesystem tools that can nuke things fast
    'dd[[:space:]]+if='
    'mkfs\.'
    # Permission nukes
    'chmod[[:space:]]+(-R[[:space:]]+)?777'
    'chmod[[:space:]]+(-R[[:space:]]+)?000'
    # Sensitive paths (writes)
    '>[[:space:]]*/etc/'
    '>[[:space:]]*~?/\.ssh/'
    '>[[:space:]]*~?/\.gnupg/'
    # Shell config writes (persistence risk)
    '>[[:space:]]*~?/\.zshrc'
    '>[[:space:]]*~?/\.bashrc'
    '>[[:space:]]*~?/\.profile'
    '>[[:space:]]*~?/\.zshenv'
    '>[[:space:]]*~?/\.bash_profile'
    # Fork bomb (classic)
    ':\(\)\{'
    # Process napalm
    'pkill[[:space:]]+-9'
    'kill[[:space:]]+-9[[:space:]]+1([[:space:]]|$)'
)

for pattern in "${deny_patterns[@]}"; do
    if [[ "$command" =~ $pattern ]]; then
        log_decision "pass" "denylist:$pattern"
        echo '{}'
        exit 0
    fi
done

# Path whitelist: any absolute path or ~/-relative path in the command must
# resolve to one of the allowed prefixes. Commands that touch paths outside
# bpmux workspaces or /tmp fall through to normal prompting.
#
# Extraction strategy: grep for tokens that look like absolute paths or
# tilde paths in the command string. For each candidate, check if it starts
# with an allowed prefix. If any candidate is outside, fall through.
#
# This is a heuristic — it can over-prompt (false positive from a path-like
# token inside a string literal, e.g. an awk script) but it cannot under-
# prompt, because any path that doesn't match the whitelist triggers a
# prompt. The fail-safe direction is the right one for a permission gate.

allowed_abs_prefixes=(
    "/tmp"
    "/private/tmp"
    "/Users/ryandurling/IdeaProjects/bpmux"
    "/Users/ryandurling/IdeaProjects/bpmux-dev"
    "/Users/ryandurling/IdeaProjects/bpmux-pm"
    "/Users/ryandurling/IdeaProjects/bpmux-designer"
    "/dev/null"
    "/dev/stdout"
    "/dev/stderr"
    "/dev/zero"
    "/dev/urandom"
    "/dev/random"
    "/dev/tty"
    "/dev/fd"
    "/var/folders"                                # macOS tempdir root
)

allowed_tilde_prefixes=(
    "~/IdeaProjects/bpmux"
    "~/IdeaProjects/bpmux-dev"
    "~/IdeaProjects/bpmux-pm"
    "~/IdeaProjects/bpmux-designer"
)

is_allowed_abs_path() {
    local p="$1"
    for prefix in "${allowed_abs_prefixes[@]}"; do
        if [[ "$p" == "$prefix" || "$p" == "$prefix"/* ]]; then
            return 0
        fi
    done
    return 1
}

is_allowed_tilde_path() {
    local p="$1"
    for prefix in "${allowed_tilde_prefixes[@]}"; do
        if [[ "$p" == "$prefix" || "$p" == "$prefix"/* ]]; then
            return 0
        fi
    done
    return 1
}

# Pre-process: strip URL tokens so `https://host/path` doesn't get parsed as
# an absolute path `/path`. Replace with a placeholder so the rest of the
# pattern matching still sees a word token where the URL used to be.
command_for_path_check=$(
    printf '%s\n' "$command" |
        sed -E 's#([a-zA-Z]+://)[^[:space:]"'\''`)]+#URL#g'
)

# Pull absolute-path tokens. The regex matches strings that start with / and
# contain path characters. It'll pick up false positives inside quoted args,
# which is fine because those will just over-prompt.
while IFS= read -r token; do
    [[ -z "$token" ]] && continue
    if ! is_allowed_abs_path "$token"; then
        log_decision "pass" "abs-path-outside-whitelist:$token"
        echo '{}'
        exit 0
    fi
done < <(printf '%s\n' "$command_for_path_check" | grep -oE '(^|[^[:alnum:]_./~-])(/[a-zA-Z0-9._/-]+)' | sed -E 's#^[^/]*##')

# Pull tilde-path tokens (~/something).
while IFS= read -r token; do
    [[ -z "$token" ]] && continue
    if ! is_allowed_tilde_path "$token"; then
        log_decision "pass" "tilde-path-outside-whitelist:$token"
        echo '{}'
        exit 0
    fi
done < <(printf '%s\n' "$command_for_path_check" | grep -oE '~/[a-zA-Z0-9._/-]+')

# Pull $HOME/ tokens and treat them like tilde paths.
while IFS= read -r token; do
    [[ -z "$token" ]] && continue
    # Strip $HOME/ prefix and compare against allowed tilde tails.
    rel="${token#\$HOME/}"
    if ! is_allowed_tilde_path "~/$rel"; then
        log_decision "pass" "home-path-outside-whitelist:$token"
        echo '{}'
        exit 0
    fi
done < <(printf '%s\n' "$command_for_path_check" | grep -oE '\$HOME/[a-zA-Z0-9._/-]+')

# Passed the cwd gate, the denylist, and the path whitelist. In dry-run,
# log "would allow" and fall through.
if [[ "$DRY_RUN" == "1" ]]; then
    log_decision "would-allow" "dry-run"
    echo '{}'
    exit 0
fi

# Enforce mode: allow.
log_decision "allow" "enforce"
cat <<'JSON'
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow",
    "permissionDecisionReason": "auto-approved by .claude/hooks/auto-approve-bash.sh (cwd in bpmux workspace, not in denylist)"
  }
}
JSON
