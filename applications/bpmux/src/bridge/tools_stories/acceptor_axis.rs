use super::helpers::{LABEL_AWAITING_DEV, LABEL_AWAITING_OWNER, LABEL_AWAITING_TEAM};

const ALL_ACCEPTOR_LABELS: [&str; 3] = [
    LABEL_AWAITING_TEAM,
    LABEL_AWAITING_OWNER,
    LABEL_AWAITING_DEV,
];

#[derive(Clone, Copy)]
pub(crate) enum AcceptorClass {
    Team,
    Owner,
    Dev,
}

impl AcceptorClass {
    pub(crate) fn label(self) -> &'static str {
        match self {
            AcceptorClass::Team => LABEL_AWAITING_TEAM,
            AcceptorClass::Owner => LABEL_AWAITING_OWNER,
            AcceptorClass::Dev => LABEL_AWAITING_DEV,
        }
    }

    pub(crate) fn stale_labels(self) -> impl Iterator<Item = &'static str> {
        let keep = self.label();
        ALL_ACCEPTOR_LABELS.into_iter().filter(move |l| *l != keep)
    }

    pub(crate) fn parse(raw: &str) -> Option<Self> {
        match raw {
            "team" => Some(AcceptorClass::Team),
            "owner" => Some(AcceptorClass::Owner),
            "dev" => Some(AcceptorClass::Dev),
            _ => None,
        }
    }
}

pub(crate) fn acceptor_label_events(
    acceptor: AcceptorClass,
    current_labels: &[String],
    meta: &crate::product_management::board::events::EventMeta,
    number: u64,
) -> Vec<crate::product_management::board::BoardEvent> {
    let mut events = Vec::new();
    for stale in acceptor.stale_labels() {
        if current_labels.iter().any(|l| l == stale) {
            events.push(crate::product_management::board::BoardEvent::TagRemoved {
                meta: meta.clone(),
                number,
                label: stale.to_string(),
            });
        }
    }
    let keep = acceptor.label();
    if !current_labels.iter().any(|l| l == keep) {
        events.push(crate::product_management::board::BoardEvent::TagAdded {
            meta: meta.clone(),
            number,
            label: keep.to_string(),
        });
    }
    events
}

pub(crate) fn derive_acceptor_label(user_job: &str) -> Option<&'static str> {
    let trimmed = user_job.trim_start();
    if !trimmed
        .as_bytes()
        .get(..3)
        .is_some_and(|p| p.eq_ignore_ascii_case(b"as "))
    {
        return None;
    }
    let use_axis = trimmed.split(',').next().unwrap_or(trimmed);
    let words: Vec<&str> = use_axis
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let has = |needle: &str| words.iter().any(|w| w.eq_ignore_ascii_case(needle));

    if has("owner") || has("ryan") {
        return Some(LABEL_AWAITING_OWNER);
    }
    let team_axis =
        has("agent") || has("agents") || has("team") || has("everyone") || has("anyone");
    let dev_axis = has("dev") || has("developer") || has("developers");
    let pm_axis = has("pm");
    if team_axis || (dev_axis && pm_axis) {
        return Some(LABEL_AWAITING_TEAM);
    }
    if dev_axis {
        return Some(LABEL_AWAITING_DEV);
    }
    if pm_axis {
        return Some(LABEL_AWAITING_TEAM);
    }
    None
}
