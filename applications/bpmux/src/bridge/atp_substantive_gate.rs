//! #1362 desk-check substrate-gate: validates the body's named ATP
//! path(s) contain substantive assertions, not empty stubs.
//!
//! Fires from `handle_pass_desk_check` before the label-mutation
//! chain. Today's #1306 + #1335 catch surfaced the body-vs-impl ATP-
//! path-stub class: a story body cites an ATP file path; the file
//! exists but contains zero assertions. Designer-desk-check passes
//! manually; PM-accept audit catches the gap at acceptance instead.
//!
//! Substrate-enforce > discipline-only canon: the gate cheaply
//! prevents the class at desk-check fire, not at accept-audit.

use std::path::PathBuf;

/// Source-file extensions the gate inspects for assertion-content.
/// Other ATP path types (markdown audit-logs, docs, etc.) are
/// implicitly accepted — the gate doesn't gate on non-source.
const SOURCE_EXTENSIONS: &[&str] = &["rs", "tsx", "ts", "js", "jsx"];

/// Substring markers that count as "this file has assertions."
/// At least one of these must appear in a source-ATP file for the
/// gate to consider it substantive.
const ASSERTION_MARKERS: &[&str] = &[
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

/// Parse ATP paths from a story body. The body convention is a
/// section header like `## Acceptance test path` followed by one or
/// more file paths (in backticks, plus-separated, or newline-
/// separated). The parser is intentionally lenient — extracts any
/// path-shaped string with a source-file extension.
pub(super) fn extract_atp_paths(body: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for line in body.lines() {
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

fn has_source_extension(path: &str) -> bool {
    SOURCE_EXTENSIONS
        .iter()
        .any(|ext| path.ends_with(&format!(".{ext}")) || path.ends_with(&format!(".test.{ext}")))
}

/// Verdict per ATP path inspection.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum AtpVerdict {
    /// File found and contains at least one assertion marker.
    Substantive,
    /// File found but contains zero assertion markers — stub.
    Stub,
    /// File not found at any candidate root — gate is lenient and
    /// treats path-resolution failure as not-blocking. The caller
    /// can choose to surface this as a warning.
    NotFound,
}

/// Resolve a body-named ATP path against candidate workspace roots
/// + return the verdict on assertion-content.
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

/// Default candidate roots — walks up from `current_dir` looking for
/// a workspace `Cargo.toml`. Falls back to `current_dir` itself.
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

/// Top-level gate API. Returns `Err` with a clear message when the
/// body cites a source-file ATP that exists but contains no
/// assertions. Lenient on NotFound (path-resolution failures don't
/// block — separate concern).
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

/// #1412 preventive sister to `verify_atp_substantive`: fires at
/// `create_story` / `edit_story` time (before pickup), stricter than
/// the desk-check gate. Both `NotFound` and `Stub` block — the goal is
/// to refuse ATP-divergence at filing-time so it never reaches
/// pickup. Lenient `verify_atp_substantive` (desk-check time) treats
/// NotFound as not-blocking because the file might appear between
/// filing and pickup; the pre-filing gate has no such grace.
///
/// Returns a structured error so the MCP handler can serialise
/// `{ kind, path, message }` for the caller — distinct from the
/// human-readable desk-check error string.
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
mod tests {
    use super::*;

    #[test]
    fn extract_atp_path_from_backticked_body_section() {
        let body =
            "## Acceptance test path\n`applications/electron/src/renderer/src/foo.test.tsx`\n";
        let paths = extract_atp_paths(body);
        assert_eq!(
            paths,
            vec!["applications/electron/src/renderer/src/foo.test.tsx"]
        );
    }

    #[test]
    fn extract_multiple_atp_paths_plus_separated() {
        let body = "## ATP\n`foo/bar.test.tsx + baz/qux.rs`\n";
        let paths = extract_atp_paths(body);
        assert!(paths.contains(&"foo/bar.test.tsx".to_string()));
        assert!(paths.contains(&"baz/qux.rs".to_string()));
    }

    #[test]
    fn extract_skips_non_source_paths() {
        let body = "ATP: docs/audit-logs/2026-05-16.md + applications/foo.test.tsx";
        let paths = extract_atp_paths(body);
        assert_eq!(paths, vec!["applications/foo.test.tsx"]);
    }

    #[test]
    fn classify_substantive_when_file_has_expect_call() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("foo.test.tsx");
        std::fs::write(&file, "it('x', () => { expect(1).toBe(1) })").unwrap();
        let verdict = classify_atp_path("foo.test.tsx", &[dir.path().to_path_buf()]);
        assert_eq!(verdict, AtpVerdict::Substantive);
    }

    #[test]
    fn classify_stub_when_file_has_no_assertion_markers() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("stub.test.tsx");
        std::fs::write(&file, "describe('x', () => { it('y', () => {}) })").unwrap();
        let verdict = classify_atp_path("stub.test.tsx", &[dir.path().to_path_buf()]);
        assert_eq!(verdict, AtpVerdict::Stub);
    }

    #[test]
    fn classify_not_found_when_no_candidate_resolves() {
        let dir = tempfile::tempdir().unwrap();
        let verdict = classify_atp_path("never/exists.test.tsx", &[dir.path().to_path_buf()]);
        assert_eq!(verdict, AtpVerdict::NotFound);
    }

    #[test]
    fn classify_substantive_for_rust_assert_macro() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("foo.rs");
        std::fs::write(&file, "#[test] fn t() { assert_eq!(1, 1); }").unwrap();
        let verdict = classify_atp_path("foo.rs", &[dir.path().to_path_buf()]);
        assert_eq!(verdict, AtpVerdict::Substantive);
    }

    #[test]
    fn verify_gate_blocks_when_named_path_is_stub() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("nested")).unwrap();
        let stub_path = dir.path().join("nested/stub.test.tsx");
        std::fs::write(&stub_path, "describe('x', () => { it('y', () => {}) })").unwrap();
        let body = "## Acceptance test path\n`nested/stub.test.tsx`";
        let result = verify_atp_substantive_with_roots(body, &[dir.path().to_path_buf()]);
        let err = result.expect_err("stub must block");
        assert!(err.contains("ATP-substantive gate blocks"));
        assert!(err.contains("nested/stub.test.tsx"));
    }

    #[test]
    fn verify_gate_passes_when_named_path_substantive() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("nested")).unwrap();
        let path = dir.path().join("nested/good.test.tsx");
        std::fs::write(&path, "it('x', () => { expect(1).toBe(1) })").unwrap();
        let body = "## Acceptance test path\n`nested/good.test.tsx`";
        let result = verify_atp_substantive_with_roots(body, &[dir.path().to_path_buf()]);
        assert!(result.is_ok(), "substantive path passes; got {result:?}");
    }

    #[test]
    fn verify_gate_passes_when_no_source_atp_in_body() {
        let body = "## ATP\n`docs/audit-logs/2026-05-16.md`";
        let result = verify_atp_substantive_with_roots(body, &[]);
        assert!(result.is_ok(), "non-source ATP is lenient; got {result:?}");
    }

    #[test]
    fn verify_gate_passes_when_file_not_found() {
        let body = "## ATP\n`never/exists.test.tsx`";
        let dir = tempfile::tempdir().unwrap();
        let result = verify_atp_substantive_with_roots(body, &[dir.path().to_path_buf()]);
        assert!(
            result.is_ok(),
            "NotFound is lenient (separate concern); got {result:?}"
        );
    }

    #[test]
    fn pre_filing_passes_when_atp_arg_is_substantive() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("good.test.tsx");
        std::fs::write(&path, "expect(1).toBe(1)").unwrap();
        let result = verify_atp_pre_filing_with_roots(
            "",
            Some("good.test.tsx"),
            &[dir.path().to_path_buf()],
        );
        assert!(result.is_ok(), "substantive ATP passes; got {result:?}");
    }

    #[test]
    fn pre_filing_blocks_when_atp_arg_is_stub() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stub.test.tsx");
        std::fs::write(&path, "describe('x', () => { it('y', () => {}) })").unwrap();
        let result = verify_atp_pre_filing_with_roots(
            "",
            Some("stub.test.tsx"),
            &[dir.path().to_path_buf()],
        );
        let err = result.expect_err("stub must block at filing-time");
        assert_eq!(err.kind, "atp-not-substantive");
        assert_eq!(err.path, "stub.test.tsx");
        assert!(err.message.contains("zero assertion markers"));
    }

    #[test]
    fn pre_filing_blocks_when_atp_arg_path_missing() {
        let dir = tempfile::tempdir().unwrap();
        let result = verify_atp_pre_filing_with_roots(
            "",
            Some("never/exists.rs"),
            &[dir.path().to_path_buf()],
        );
        let err = result.expect_err("missing ATP must block at filing-time");
        assert_eq!(err.kind, "atp-not-substantive");
        assert_eq!(err.path, "never/exists.rs");
        assert!(err.message.contains("missing"));
    }

    #[test]
    fn pre_filing_passes_when_no_atp_arg_and_body_has_none() {
        let result = verify_atp_pre_filing_with_roots("body without ATP section", None, &[]);
        assert!(
            result.is_ok(),
            "no ATP cited → exempt (Chore-class shape); got {result:?}"
        );
    }

    #[test]
    fn pre_filing_falls_back_to_body_extraction_when_atp_arg_absent() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("nested")).unwrap();
        let stub_path = dir.path().join("nested/stub.rs");
        std::fs::write(&stub_path, "fn placeholder() {}").unwrap();
        let body = "## Acceptance test path\n`nested/stub.rs`";
        let result = verify_atp_pre_filing_with_roots(body, None, &[dir.path().to_path_buf()]);
        let err = result.expect_err("body-cited stub blocks at filing");
        assert_eq!(err.path, "nested/stub.rs");
    }

    #[test]
    fn pre_filing_error_serializes_to_json_with_kind_path_message() {
        let err = AtpPreFilingError {
            kind: "atp-not-substantive",
            path: "foo.rs".into(),
            message: "test".into(),
        };
        let json = err.as_json_string();
        assert!(json.contains("\"kind\":\"atp-not-substantive\""));
        assert!(json.contains("\"path\":\"foo.rs\""));
        assert!(json.contains("\"message\":\"test\""));
    }

    #[test]
    fn pre_filing_planted_violation_seed_substring_matcher_accepts_commented_assertion() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("commented_out.rs");
        std::fs::write(
            &path,
            "fn t() { /* assert_eq!(1, 1); */ println!(\"not asserting\") }",
        )
        .unwrap();
        let result = verify_atp_pre_filing_with_roots(
            "",
            Some("commented_out.rs"),
            &[dir.path().to_path_buf()],
        );
        assert!(
            result.is_ok(),
            "substring-matcher accepts commented `assert_eq!` — planted-violation seed for #1359-style discipline (future AST-tightening would tighten this); got {result:?}"
        );
    }
}
