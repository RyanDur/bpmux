use std::path::PathBuf;

mod classify;

use classify::{ASSERTION_MARKERS, AtpVerdict, classify_atp_path, default_candidate_roots};

const SOURCE_EXTENSIONS: &[&str] = &["rs", "tsx", "ts", "js", "jsx"];

pub(super) fn extract_atp_paths(body: &str) -> Vec<String> {
    let Some(section) = extract_atp_section(body) else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    for line in section.lines() {
        for token in line.split(|c: char| c.is_whitespace() || c == '`' || c == '+' || c == ',') {
            let trimmed = token.trim_matches(|c: char| {
                c == '.' || c == ':' || c == ';' || c == '(' || c == ')' || c == '"'
            });
            if has_source_extension(trimmed) && trimmed.contains('/') {
                paths.push(trimmed.to_string());
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn extract_atp_section(body: &str) -> Option<&str> {
    const HEADER: &str = "## Acceptance test path";
    let header_start = body.rfind(HEADER)?;
    let after_header = header_start + HEADER.len();
    let rest = &body[after_header..];
    let end_offset = rest
        .find("\n## ")
        .or_else(|| rest.find("\n---"))
        .unwrap_or(rest.len());
    Some(&rest[..end_offset])
}

fn has_source_extension(path: &str) -> bool {
    SOURCE_EXTENSIONS
        .iter()
        .any(|ext| path.ends_with(&format!(".{ext}")) || path.ends_with(&format!(".test.{ext}")))
}

pub(super) fn verify_atp_substantive(body: &str) -> Result<(), String> {
    verify_atp_substantive_with_roots(body, &default_candidate_roots())
}

pub(super) fn verify_atp_substantive_with_roots(
    body: &str,
    roots: &[PathBuf],
) -> Result<(), String> {
    let paths = extract_atp_paths(body);
    if paths.is_empty() {
        return Ok(());
    }
    let mut stubs = Vec::new();
    for path in &paths {
        if matches!(classify_atp_path(path, roots), AtpVerdict::Stub) {
            stubs.push(path.clone());
        }
    }
    if stubs.is_empty() {
        return Ok(());
    }
    Err(format!(
        "#1362 ATP-substantive gate blocks pass_desk_check: body cites ATP path(s) {stubs:?} but file(s) contain zero assertion markers ({assertions}). Sister to #1306/#1335 stub-file class. Either add assertions OR edit_story body to point at the sibling test file(s) holding the assertions.",
        assertions = ASSERTION_MARKERS.join(", "),
    ))
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct AtpPreFilingError {
    pub kind: &'static str,
    pub path: String,
    pub message: String,
}

impl AtpPreFilingError {
    pub(super) fn as_json_string(&self) -> String {
        serde_json::json!({
            "kind": self.kind,
            "path": self.path,
            "message": self.message,
        })
        .to_string()
    }
}

pub(super) fn verify_atp_pre_filing(
    body: &str,
    atp_arg: Option<&str>,
) -> Result<(), AtpPreFilingError> {
    verify_atp_pre_filing_with_roots(body, atp_arg, &default_candidate_roots())
}

pub(super) fn verify_atp_pre_filing_with_roots(
    body: &str,
    atp_arg: Option<&str>,
    roots: &[PathBuf],
) -> Result<(), AtpPreFilingError> {
    let paths: Vec<String> = match atp_arg {
        Some(p) if !p.trim().is_empty() => {
            let trimmed = p.trim();
            if has_source_extension(trimmed) {
                vec![trimmed.to_string()]
            } else {
                Vec::new()
            }
        }
        _ => extract_atp_paths(body),
    };
    if paths.is_empty() {
        return Ok(());
    }
    for path in &paths {
        match classify_atp_path(path, roots) {
            AtpVerdict::Substantive => continue,
            AtpVerdict::Stub => {
                return Err(AtpPreFilingError {
                    kind: "atp-not-substantive",
                    path: path.clone(),
                    message: format!(
                        "ATP-path file exists but contains zero assertion markers ({})",
                        ASSERTION_MARKERS.join(", ")
                    ),
                });
            }
            AtpVerdict::NotFound => {
                return Err(AtpPreFilingError {
                    kind: "atp-not-substantive",
                    path: path.clone(),
                    message: "ATP-path file missing — create the test file at this path \
                              with at least one assertion marker, OR fix the path."
                        .to_string(),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "tests.rs"]
mod tests;
