#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TrackerSelection {
    Octocrab {
        owner: String,
        repo: String,
        token: String,
    },
    OctocrabWithBaseUrl {
        owner: String,
        repo: String,
        base_url: String,
    },
    Fallback(FallbackReason),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FallbackReason {
    NoRepo,
    MalformedRepo,
    NoToken,
}

pub(crate) fn select_from_env<F>(get: F) -> TrackerSelection
where
    F: Fn(&str) -> Option<String>,
{
    let Some(repo_spec) = get("BPMUX_REPO") else {
        return TrackerSelection::Fallback(FallbackReason::NoRepo);
    };
    let Some((owner, repo)) = repo_spec.split_once('/') else {
        return TrackerSelection::Fallback(FallbackReason::MalformedRepo);
    };
    if owner.is_empty() || repo.is_empty() {
        return TrackerSelection::Fallback(FallbackReason::MalformedRepo);
    }
    if let Some(base_url) = get("BPMUX_GH_BASE_URL") {
        return TrackerSelection::OctocrabWithBaseUrl {
            owner: owner.to_string(),
            repo: repo.to_string(),
            base_url,
        };
    }
    let Some(token) = get("GH_TOKEN").or_else(|| get("GITHUB_TOKEN")) else {
        return TrackerSelection::Fallback(FallbackReason::NoToken);
    };
    TrackerSelection::Octocrab {
        owner: owner.to_string(),
        repo: repo.to_string(),
        token,
    }
}
