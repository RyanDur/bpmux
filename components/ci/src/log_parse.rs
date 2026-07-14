use crate::{CiRunFailures, CiTestFailure};

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
