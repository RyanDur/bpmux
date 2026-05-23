use std::sync::OnceLock;

pub fn ensure_crypto_provider_installed() {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    INSTALLED.get_or_init(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

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
