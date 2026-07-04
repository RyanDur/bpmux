const MARKER_PREFIX: &str = "<!-- bpmux-relationships:";
const MARKER_SUFFIX: &str = "-->";

pub fn extract(body: &str) -> Option<serde_json::Value> {
    let (start, end) = marker_span(body)?;
    let inner = body[start + MARKER_PREFIX.len()..end - MARKER_SUFFIX.len()].trim();
    serde_json::from_str(inner).ok()
}

pub fn embed(body: &str, block: &serde_json::Value) -> String {
    let marker = format!("{MARKER_PREFIX} {block} {MARKER_SUFFIX}");
    match marker_span(body) {
        Some((start, end)) => format!("{}{marker}{}", &body[..start], &body[end..]),
        None if body.is_empty() => marker,
        None => format!("{body}\n\n{marker}"),
    }
}

fn marker_span(body: &str) -> Option<(usize, usize)> {
    let start = body.find(MARKER_PREFIX)?;
    let after_prefix = start + MARKER_PREFIX.len();
    let suffix_rel = body[after_prefix..].find(MARKER_SUFFIX)?;
    Some((start, after_prefix + suffix_rel + MARKER_SUFFIX.len()))
}
