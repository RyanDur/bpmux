use std::path::PathBuf;

pub(super) const ASSERTION_MARKERS: &[&str] = &[
    "expect(",
    "assert_eq!",
    "assert_ne!",
    "assert!(",
    "assert_matches!",
    "assert_throws",
    ".toBe(",
    ".toEqual(",
    ".toMatch",
    ".toContain",
    ".toHaveBeenCalled",
];

#[derive(Debug, Clone, PartialEq)]
pub(super) enum AtpVerdict {
    Substantive,
    Stub,
    NotFound,
}

pub(super) fn classify_atp_path(path: &str, candidate_roots: &[PathBuf]) -> AtpVerdict {
    for root in candidate_roots {
        let resolved = root.join(path);
        if resolved.exists()
            && let Ok(contents) = std::fs::read_to_string(&resolved)
        {
            return if has_assertion(&contents) {
                AtpVerdict::Substantive
            } else {
                AtpVerdict::Stub
            };
        }
    }
    AtpVerdict::NotFound
}

fn has_assertion(contents: &str) -> bool {
    ASSERTION_MARKERS.iter().any(|m| contents.contains(m))
}

pub(super) fn default_candidate_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        let mut walker = cwd.as_path();
        loop {
            if walker.join("Cargo.toml").exists()
                && std::fs::read_to_string(walker.join("Cargo.toml"))
                    .map(|c| c.contains("[workspace]"))
                    .unwrap_or(false)
            {
                roots.push(walker.to_path_buf());
                break;
            }
            match walker.parent() {
                Some(p) => walker = p,
                None => break,
            }
        }
        roots.push(cwd);
    }
    roots
}
