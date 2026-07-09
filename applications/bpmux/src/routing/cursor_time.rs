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

pub(super) fn to_millis(value: &str) -> Option<i64> {
    parse_ts(value).map(|dt| dt.timestamp_millis())
}

pub(super) fn from_millis(ms: i64) -> Option<String> {
    chrono::DateTime::from_timestamp_millis(ms).map(|dt| dt.to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::later_cursor;

    #[test]
    fn later_cursor_parses_rather_than_lexically_compares_so_subsecond_precision_is_correct() {
        assert_eq!(
            later_cursor(Some("2026-06-11T20:00:01Z"), Some("2026-06-11T20:00:01.5Z")).as_deref(),
            Some("2026-06-11T20:00:01.5Z"),
            "the later instant wins by PARSED time — a lexical string compare would wrongly pick the no-fraction form ('Z' > '.5') and lose half a second of events"
        );
    }
}
