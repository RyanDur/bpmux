//! Per-test CI failure breakdown for a given GH Actions run (#1174).
//! Sister-substrate to #1145's get_ci_state, but keyed by run_id with
//! TTL-forever cache for completed-run logs (append-only-immutable).
//! In-progress runs bypass cache and return a named error-shape so
//! callers don't get stale empty results from incomplete runs.

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CiTestFailure {
    pub shard_id: Option<String>,
    pub test_name: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub error_message: String,
    #[serde(default)]
    pub retry_count: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CiRunFailures {
    pub run_id: u64,
    pub conclusion: String,
    pub failures: Vec<CiTestFailure>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CiLogError {
    RunNotFound { run_id: u64 },
    RunInProgress { run_id: u64, conclusion: String },
    RateLimited { wait_secs: u64 },
    UpstreamError { detail: String },
}

pub(in crate::bridge) async fn handle_get_ci_log_for_run(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let run_id = args
        .get("run_id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'run_id' argument")?;
    let token_path = crate::session::auth_token::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    let token = crate::session::auth_token::read(&token_path)
        .map_err(|e| format!("read auth-token from {}: {e}", token_path.display()))?;
    let url = format!("http://127.0.0.1:{port}/api/ci/log/{run_id}");
    let res = reqwest::Client::new()
        .get(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("get_ci_log_for_run: electron-server unreachable at {url}: {e}"))?;
    let status = res.status();
    let body = res
        .text()
        .await
        .map_err(|e| format!("get_ci_log_for_run: read body: {e}"))?;
    if !status.is_success() {
        return Err(format!("get_ci_log_for_run: {status} — {body}"));
    }
    Ok(body)
}

pub fn parse_playwright_failures_from_log(run_id: u64, raw_log: &str) -> CiRunFailures {
    let mut failures = Vec::new();
    let mut lines = raw_log.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start_matches([' ', '\t']);
        let after_index: &str = if let Some(rest) = trimmed
            .strip_prefix("✘ ")
            .or_else(|| trimmed.strip_prefix("✗ "))
            .or_else(|| trimmed.strip_prefix("× "))
        {
            rest.split_once(char::is_whitespace)
                .map(|x| x.1)
                .unwrap_or(rest)
                .trim()
        } else if let Some(after_paren) = strip_indexed_failure_prefix(trimmed) {
            after_paren.trim()
        } else {
            continue;
        };
        let (location_part, name_part) = match after_index.find(" › ") {
            Some(idx) => (&after_index[..idx], &after_index[idx + 5..]),
            None => (after_index, after_index),
        };
        let test_name = name_part
            .rsplit(" › ")
            .next()
            .unwrap_or(name_part)
            .trim()
            .trim_end_matches(|c: char| c == '(' || c.is_ascii_digit() || c == '.')
            .trim_end_matches(['s', '(', ')'])
            .trim()
            .to_string();
        let (file, line) = parse_file_and_line(location_part);
        let error_message = scan_error_message(&mut lines);
        let retry_count = if line_indicates_retry(name_part) {
            1
        } else {
            0
        };
        failures.push(CiTestFailure {
            shard_id: None,
            test_name,
            file,
            line,
            error_message,
            retry_count,
        });
    }
    CiRunFailures {
        run_id,
        conclusion: if failures.is_empty() {
            "success".to_string()
        } else {
            "failure".to_string()
        },
        failures,
    }
}

fn strip_indexed_failure_prefix(line: &str) -> Option<&str> {
    let mut chars = line.char_indices();
    let first = chars.next()?;
    if !first.1.is_ascii_digit() {
        return None;
    }
    while let Some(&(idx, c)) = chars.clone().peekable().peek() {
        if c.is_ascii_digit() {
            chars.next();
        } else if c == ')' {
            chars.next();
            let rest = &line[idx + 1..];
            return Some(rest.strip_prefix(' ').unwrap_or(rest));
        } else {
            return None;
        }
    }
    None
}

fn parse_file_and_line(loc: &str) -> (Option<String>, Option<u32>) {
    let mut parts = loc.rsplitn(3, ':');
    let _col = parts.next();
    let line_str = parts.next();
    let file = parts.next();
    let line = line_str.and_then(|s| s.parse::<u32>().ok());
    let file_owned = file.map(|s| s.to_string());
    if line.is_some() {
        (file_owned, line)
    } else {
        (Some(loc.to_string()), None)
    }
}

fn line_indicates_retry(name_part: &str) -> bool {
    name_part.contains("(retry #")
}

fn scan_error_message<'a>(
    lines: &mut std::iter::Peekable<impl Iterator<Item = &'a str>>,
) -> String {
    while let Some(peek) = lines.peek() {
        let trimmed = peek.trim();
        if trimmed.is_empty() {
            lines.next();
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("Error: ") {
            lines.next();
            return rest.to_string();
        }
        if trimmed.starts_with("Test timeout") {
            let captured = trimmed.to_string();
            lines.next();
            return captured;
        }
        break;
    }
    "(no error detail in log)".to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn empty_log_yields_zero_failures_and_success_conclusion() {
        let report = parse_playwright_failures_from_log(42, "");
        assert_eq!(report.run_id, 42);
        assert_eq!(report.conclusion, "success");
        assert!(report.failures.is_empty());
    }

    #[test]
    fn parses_single_failure_with_test_timeout_message() {
        let log = "\
Running 47 tests using 4 workers, shard 6 of 6
··············································×T

  1) tests/playwright/offline-board.spec.ts:121:7 › #1158 — offline-first board › F8 stale — glyph surfaces stale state when ingest exceeds threshold

    Test timeout of 30000ms exceeded.
";
        let report = parse_playwright_failures_from_log(42, log);
        assert_eq!(report.conclusion, "failure");
        assert_eq!(report.failures.len(), 1);
        let f = &report.failures[0];
        assert_eq!(
            f.file.as_deref(),
            Some("tests/playwright/offline-board.spec.ts")
        );
        assert_eq!(f.line, Some(121));
        assert!(f.test_name.contains("F8 stale"));
        assert!(f.error_message.contains("Test timeout"));
    }

    #[test]
    fn retry_marker_increments_retry_count() {
        let log = "\
  ✘  1 tests/foo.spec.ts:10:5 › suite › my flaky test
  ✘  2 tests/foo.spec.ts:10:5 › suite › my flaky test (retry #1)

    Error: assertion failed
";
        let report = parse_playwright_failures_from_log(7, log);
        assert!(report.failures.iter().any(|f| f.retry_count > 0));
    }

    #[test]
    fn run_not_found_serializes_with_run_id_payload() {
        let err = CiLogError::RunNotFound { run_id: 9999 };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["kind"], "run_not_found");
        assert_eq!(json["run_id"], 9999);
    }

    #[test]
    fn run_in_progress_carries_current_conclusion() {
        let err = CiLogError::RunInProgress {
            run_id: 100,
            conclusion: "in_progress".to_string(),
        };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["kind"], "run_in_progress");
        assert_eq!(json["conclusion"], "in_progress");
    }

    #[test]
    fn rate_limited_carries_wait_secs() {
        let err = CiLogError::RateLimited { wait_secs: 60 };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["kind"], "rate_limited");
        assert_eq!(json["wait_secs"], 60);
    }
}
