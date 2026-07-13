pub fn derive_iteration_member_section(members: &[(u64, &str)]) -> String {
    if members.is_empty() {
        return "_(no members yet)_\n".to_string();
    }
    let mut out = String::new();
    for (number, title) in members {
        out.push_str(&format!("- #{number} — {title}\n"));
    }
    out
}

pub fn substitute_member_stories_section(body: &str, derived_content: &str) -> String {
    let header_marker = "## Member stories\n";
    let Some(header_pos) = body.find(header_marker) else {
        return body.to_string();
    };
    let section_start = header_pos + header_marker.len();
    let next_h2 = body[section_start..]
        .find("\n## ")
        .map(|i| section_start + i + 1)
        .unwrap_or(body.len());
    let trimmed = derived_content.trim_matches('\n');
    let mut out = String::with_capacity(body.len() + trimmed.len());
    out.push_str(&body[..section_start]);
    out.push('\n');
    out.push_str(trimmed);
    out.push_str("\n\n");
    out.push_str(&body[next_h2..]);
    out
}
