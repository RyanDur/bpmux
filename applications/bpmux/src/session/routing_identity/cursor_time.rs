fn parse_ts(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

pub(super) fn later_cursor(a: Option<&str>, b: Option<&str>) -> Option<String> {
    match (a, b) {
        (Some(x), Some(y)) => match (parse_ts(x), parse_ts(y)) {
            (Some(px), Some(py)) => Some(if px >= py {
                x.to_string()
            } else {
                y.to_string()
            }),
            (Some(_), None) => Some(x.to_string()),
            (None, _) => Some(y.to_string()),
        },
        (Some(x), None) => Some(x.to_string()),
        (None, Some(y)) => Some(y.to_string()),
        (None, None) => None,
    }
}

pub(super) fn cursor_after(stored: Option<&str>, job_start: &str) -> bool {
    match (stored.and_then(parse_ts), parse_ts(job_start)) {
        (Some(stored), Some(job_start)) => stored > job_start,
        _ => false,
    }
}

pub(super) fn is_future(ts: &str) -> bool {
    parse_ts(ts).is_some_and(|ts| ts > chrono::Utc::now())
}
