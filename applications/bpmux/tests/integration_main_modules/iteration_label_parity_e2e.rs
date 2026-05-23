#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use bpmux::board::adapters::gh_label_codec::GhLabelCodec;
use bpmux::board::iteration_label::theme_name_from_title;
use bpmux::board::label_codec::LabelCodec;

fn renderer_theme_name_from_title(title: &str) -> String {
    let raw = match title.strip_prefix("Iteration ") {
        Some(rest) if !rest.is_empty() => rest.trim(),
        _ => title.trim(),
    };
    raw.split_whitespace().collect::<Vec<_>>().join("-")
}

fn renderer_parse_iteration_label(label: &str) -> Option<(String, u32)> {
    let rest = label.strip_prefix("iteration-")?;
    let dot = rest.rfind('.')?;
    let (name, dotted_pos) = rest.split_at(dot);
    if name.is_empty() {
        return None;
    }
    let position: u32 = dotted_pos[1..].parse().ok()?;
    Some((name.to_string(), position))
}

#[test]
fn theme_name_from_title_agrees_with_renderer_mirror() {
    let titles = [
        "Iteration Octans",
        "Iteration Microscopium",
        "Iteration Foo Bar",
        "Iteration  Double  Space",
        "Octans",
        "Iteration Unscheduled",
    ];
    for title in titles {
        assert_eq!(
            theme_name_from_title(title),
            renderer_theme_name_from_title(title),
            "cross-surface drift: title={title:?} — Rust theme_name_from_title disagrees with the \
             renderer labelSchema.ts themeNameFromTitle mirror; update both sides together"
        );
    }
}

#[test]
fn decode_iteration_agrees_with_renderer_parse_mirror() {
    let codec = GhLabelCodec;
    let labels = [
        "iteration-Octans.100",
        "iteration-Foo-Bar.250",
        "iteration-Foo.Bar.300",
        "iteration-Unscheduled.100",
        "iteration-Octans.abc",
        "iteration-.100",
        "iteration-Octans",
        "ready-order.5",
        "parent-epic-#1234",
    ];
    for label in labels {
        let rust = codec
            .decode_iteration(label)
            .map(|p| (p.theme_name, p.position));
        let renderer = renderer_parse_iteration_label(label);
        assert_eq!(
            rust, renderer,
            "cross-surface drift: label={label:?} — Rust decode_iteration disagrees with the \
             renderer labelSchema.ts parseIterationLabel mirror; update both sides together"
        );
    }
}
