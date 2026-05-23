#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CiTestFailure {
    pub shard_id: Option<String>,
    pub test_name: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub error_message: String,
    #[serde(default)]
    pub retry_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
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
    let url = crate::transport::local_url::api_url(port, &format!("/api/ci/log/{run_id}"));
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
    let stripped_log: String = raw_log
        .lines()
        .map(strip_gh_actions_timestamp_prefix)
        .collect::<Vec<&str>>()
        .join("\n");
    let mut lines = stripped_log.lines().peekable();
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
            stage: None,
            class: Some("playwright".to_string()),
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

pub fn build_non_test_failure_record(job_name: &str, raw_log: &str) -> CiTestFailure {
    const MAX_TAIL_LINES: usize = 50;
    let stripped_log: String = raw_log
        .lines()
        .map(strip_gh_actions_timestamp_prefix)
        .collect::<Vec<&str>>()
        .join("\n");
    let lines: Vec<&str> = stripped_log
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    let tail_start = lines.len().saturating_sub(MAX_TAIL_LINES);
    let tail: String = lines[tail_start..].join("\n");
    let error_message = if tail.is_empty() {
        "(job log empty or unparseable — workflow conclusion=failure)".to_string()
    } else {
        tail
    };
    CiTestFailure {
        shard_id: Some(job_name.to_string()),
        test_name: format!("(non-test failure in job: {job_name})"),
        file: None,
        line: None,
        error_message,
        retry_count: 0,
        stage: Some(job_name.to_string()),
        class: Some("non_test_failure".to_string()),
    }
}

fn strip_gh_actions_timestamp_prefix(line: &str) -> &str {
    let mut chars = line.char_indices();
    for _ in 0..30 {
        let Some((idx, c)) = chars.next() else {
            return line;
        };
        if c == 'Z' && idx > 0 {
            let after = &line[idx + 1..];
            return after.strip_prefix(' ').unwrap_or(after);
        }
        if !(c.is_ascii_digit() || matches!(c, '-' | ':' | '.' | 'T')) {
            return line;
        }
    }
    line
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
    fn parses_failures_from_gh_actions_timestamp_prefixed_log() {
        let log = "\
2026-05-04T16:17:35.1721141Z Running 47 tests using 4 workers, shard 6 of 6
2026-05-04T16:17:36.0000000Z
2026-05-04T16:26:55.1234567Z   1) tests/playwright/offline-board.spec.ts:121:7 \u{203A} #1158 \u{2014} offline-first board \u{203A} F8 stale
2026-05-04T16:26:55.1234568Z
2026-05-04T16:26:55.1234569Z     Test timeout of 30000ms exceeded.
";
        let report = parse_playwright_failures_from_log(42, log);
        assert_eq!(report.conclusion, "failure");
        assert_eq!(
            report.failures.len(),
            1,
            "GH-Actions timestamp-prefixed log must parse"
        );
        let f = &report.failures[0];
        assert_eq!(
            f.file.as_deref(),
            Some("tests/playwright/offline-board.spec.ts"),
            "parser strips GH timestamps before extracting file/line"
        );
        assert_eq!(f.line, Some(121));
    }

    #[test]
    fn rate_limited_carries_wait_secs() {
        let err = CiLogError::RateLimited { wait_secs: 60 };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["kind"], "rate_limited");
        assert_eq!(json["wait_secs"], 60);
    }

    #[test]
    fn fallback_record_for_clippy_failure_carries_tail_of_log() {
        let log = "\
2026-05-11T14:00:00Z Compiling bpmux v0.1.0
2026-05-11T14:00:01Z error[E0308]: mismatched types
2026-05-11T14:00:01Z   --> applications/bpmux/src/lib.rs:42:15
2026-05-11T14:00:01Z    |
2026-05-11T14:00:02Z error: aborting due to previous error
2026-05-11T14:00:02Z Process completed with exit code 101.
";
        let record = build_non_test_failure_record("build-bpmux", log);
        assert_eq!(record.class.as_deref(), Some("non_test_failure"));
        assert_eq!(record.stage.as_deref(), Some("build-bpmux"));
        assert_eq!(record.shard_id.as_deref(), Some("build-bpmux"));
        assert!(
            record.error_message.contains("error[E0308]"),
            "tail-of-log carries the compiler error: {}",
            record.error_message
        );
        assert!(
            record.error_message.contains("exit code 101"),
            "tail captures step exit code: {}",
            record.error_message
        );
    }

    #[test]
    fn fallback_record_for_empty_log_returns_explicit_placeholder() {
        let record = build_non_test_failure_record("test", "");
        assert_eq!(record.class.as_deref(), Some("non_test_failure"));
        assert!(
            record.error_message.contains("empty or unparseable"),
            "explicit placeholder beats empty failures[]: {}",
            record.error_message
        );
    }

    #[test]
    fn fallback_record_bounded_by_max_tail_lines() {
        let mut log = String::new();
        for i in 0..200 {
            log.push_str(&format!("line {i}\n"));
        }
        let record = build_non_test_failure_record("audit", &log);
        let line_count = record.error_message.lines().count();
        assert!(
            line_count <= 50,
            "MAX_TAIL_LINES bounds payload: {line_count}"
        );
        assert!(
            record.error_message.contains("line 199"),
            "tail captures the LAST lines (not first): {}",
            record.error_message
        );
    }

    #[test]
    fn playwright_failure_carries_playwright_class() {
        let log = "\
  ✘  1 tests/foo.spec.ts:10:5 › suite › my test

    Error: assertion failed
";
        let report = parse_playwright_failures_from_log(42, log);
        assert_eq!(report.failures.len(), 1);
        assert_eq!(
            report.failures[0].class.as_deref(),
            Some("playwright"),
            "test-level failures carry class=\"playwright\" for caller-side routing"
        );
    }
}
