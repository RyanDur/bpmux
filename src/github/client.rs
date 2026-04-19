//! Shared octocrab client construction for terminal-side callers
//! (board poller, CI poller, DORA/project/board-state fetchers).
//!
//! Mirrors the env-decisioning already done in
//! `bridge::tracker::select_from_env`, but lives here so terminal
//! modules don't have to reach across the bridge layer for the same
//! auth substrate. Kept small and pure — the GitHub error taxonomy
//! and the octocrab client build are the only knobs.
//!
//! Before #320's gh→octocrab port, these helpers lived inline inside
//! each terminal/board/* fetcher. Extracting them (a) removes the
//! duplicate env+token+crypto-init dance from four callsites, and
//! (b) unifies the failure vocabulary so "GH_TOKEN missing" reads
//! the same wherever it surfaces.

use std::sync::OnceLock;

/// Install rustls's ring crypto provider once per process.
/// rustls-0.23 panics on first TLS handshake without an explicit
/// provider when multiple backends are compiled in. Called from
/// every octocrab builder — idempotent via `OnceLock`.
pub fn ensure_crypto_provider_installed() {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    INSTALLED.get_or_init(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Build an authenticated octocrab client from env, along with the
/// owner/repo parsed from `BPMUX_REPO`. Returns a short error string
/// naming the missing/malformed input when env isn't set up — the
/// caller absorbs it (poll-once callers log + keep-last-state; board
/// fetchers surface a "board unavailable" note). No panics.
///
/// Env contract (matches `bridge::tracker::select_from_env`):
/// - `BPMUX_REPO` = "owner/name" (required, non-empty halves)
/// - `GH_TOKEN` OR `GITHUB_TOKEN` (either works; GH_TOKEN preferred)
pub fn octocrab_client() -> Result<(octocrab::Octocrab, String, String), String> {
    let repo_spec =
        std::env::var("BPMUX_REPO").map_err(|_| "BPMUX_REPO env var not set".to_string())?;
    let (owner, repo) = repo_spec
        .split_once('/')
        .ok_or_else(|| format!("BPMUX_REPO malformed (expected owner/name): {repo_spec}"))?;
    if owner.is_empty() || repo.is_empty() {
        return Err(format!(
            "BPMUX_REPO malformed (empty owner or repo): {repo_spec}"
        ));
    }
    let token = std::env::var("GH_TOKEN")
        .or_else(|_| std::env::var("GITHUB_TOKEN"))
        .map_err(|_| "GH_TOKEN/GITHUB_TOKEN env var not set".to_string())?;
    ensure_crypto_provider_installed();
    let client = octocrab::OctocrabBuilder::new()
        .personal_token(token)
        .build()
        .map_err(|e| format!("octocrab build failed: {e}"))?;
    Ok((client, owner.to_string(), repo.to_string()))
}
