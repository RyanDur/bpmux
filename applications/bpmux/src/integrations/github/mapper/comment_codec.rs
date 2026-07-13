use chrono::{DateTime, Utc};

use product_management::story::events::EventMeta;
use product_management::story::{CastVerdict, FreeComment, Verdict};

pub(crate) fn decode_free_comment(v: &serde_json::Value) -> Option<FreeComment> {
    let body = v.get("body").and_then(|x| x.as_str())?.to_string();
    let author = v
        .get("author")
        .and_then(|x| x.as_str())
        .unwrap_or("unknown")
        .to_string();
    let ts = v
        .get("created_at")
        .and_then(|x| x.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    Some(FreeComment { body, author, ts })
}

pub(crate) fn encode_verdict(verdict: &Verdict, meta: &EventMeta, payload: Option<&str>) -> String {
    let (marker, tag) = match verdict {
        Verdict::Succeed { step } => match step.as_str() {
            "ready" => ("pickup_paraphrase", "dev pickup-gate"),
            "in-progress" => ("verified_observation", "dev deliver-gate"),
            "desk-check" => ("pass_desk_check", "desk-check-pass"),
            _ => ("accepted", "accept-gate"),
        },
        Verdict::Fail { step } => match step.as_str() {
            "desk-check" => ("rejection_reason", "desk-check-fail"),
            _ => ("rejection_reason", "reject-gate"),
        },
    };
    match payload {
        Some(body) => format!("**{marker}** ({tag} by {}):\n\n{body}", meta.actor),
        None => format!("**{marker}** ({tag} by {})", meta.actor),
    }
}

pub fn decode_verdicts(comments: &[serde_json::Value]) -> Vec<CastVerdict> {
    comments
        .iter()
        .filter_map(|c| {
            let body = c.get("body").and_then(|x| x.as_str())?;
            let author = c
                .get("author")
                .and_then(|x| x.as_str())
                .unwrap_or("unknown")
                .to_string();
            let ts = c
                .get("created_at")
                .and_then(|x| x.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))?;
            classify_verdict(body).map(|(verdict, payload)| CastVerdict {
                verdict,
                ts,
                actor: author,
                payload,
            })
        })
        .collect()
}

pub fn classify_verdict(body: &str) -> Option<(Verdict, Option<String>)> {
    if body.contains("**pickup_paraphrase**") {
        return Some((
            Verdict::Succeed {
                step: "ready".into(),
            },
            extract_marker_payload(body),
        ));
    }
    if body.contains("**verified_observation**") {
        return Some((
            Verdict::Succeed {
                step: "in-progress".into(),
            },
            extract_marker_payload(body),
        ));
    }
    if body.contains("**pass_desk_check**") {
        return Some((
            Verdict::Succeed {
                step: "desk-check".into(),
            },
            None,
        ));
    }
    if body.contains("**accepted**") {
        return Some((
            Verdict::Succeed {
                step: "acceptance".into(),
            },
            None,
        ));
    }
    if body.contains("**rejection_reason**") {
        let verdict = if body.contains("desk-check-fail") {
            Verdict::Fail {
                step: "desk-check".into(),
            }
        } else {
            Verdict::Fail {
                step: "acceptance".into(),
            }
        };
        return Some((verdict, extract_marker_payload(body)));
    }
    None
}

fn extract_marker_payload(body: &str) -> Option<String> {
    body.split_once(":\n\n").map(|(_, rest)| rest.to_string())
}
