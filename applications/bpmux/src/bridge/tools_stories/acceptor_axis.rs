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
    meta: &crate::board::events::EventMeta,
    number: u64,
) -> Vec<crate::board::BoardEvent> {
    let mut events = Vec::new();
    for stale in acceptor.stale_labels() {
        if current_labels.iter().any(|l| l == stale) {
            events.push(crate::board::BoardEvent::TagRemoved {
                meta: meta.clone(),
                number,
                label: stale.to_string(),
            });
        }
    }
    let keep = acceptor.label();
    if !current_labels.iter().any(|l| l == keep) {
        events.push(crate::board::BoardEvent::TagAdded {
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn owner_use_axis_derives_owner() {
        assert_eq!(
            derive_acceptor_label("As the owner, I want the roadmap legible"),
            Some(LABEL_AWAITING_OWNER)
        );
        assert_eq!(
            derive_acceptor_label("As Ryan, I want a clear board"),
            Some(LABEL_AWAITING_OWNER)
        );
    }

    #[test]
    fn single_dev_use_axis_derives_dev() {
        assert_eq!(
            derive_acceptor_label("As a dev, I want the module decomposed"),
            Some(LABEL_AWAITING_DEV)
        );
        assert_eq!(
            derive_acceptor_label("As a developer, I want a faster build"),
            Some(LABEL_AWAITING_DEV)
        );
    }

    #[test]
    fn agent_or_team_use_axis_derives_team() {
        assert_eq!(
            derive_acceptor_label("As an agent on the team, I want shared introspection"),
            Some(LABEL_AWAITING_TEAM)
        );
        assert_eq!(
            derive_acceptor_label("As a PM, I want the queue ordered"),
            Some(LABEL_AWAITING_TEAM)
        );
    }

    #[test]
    fn multi_role_use_axis_derives_team_not_dev() {
        assert_eq!(
            derive_acceptor_label(
                "As an agent (PM or dev) filing a story, I want the acceptor set"
            ),
            Some(LABEL_AWAITING_TEAM),
            "a PM-or-dev multi-role axis is a TEAM feature, not dev — team-axis beats the bare dev token"
        );
    }

    #[test]
    fn owner_axis_is_not_a_false_match_on_the_noun_ownership() {
        assert_eq!(
            derive_acceptor_label("As a dev, I want input-ownership preserved across reloads"),
            Some(LABEL_AWAITING_DEV),
            "the NOUN 'ownership' must not match the owner ROLE — word-exact, not substring"
        );
    }

    #[test]
    fn an_unparseable_axis_derives_nothing_so_the_acceptor_flag_can_override() {
        assert_eq!(
            derive_acceptor_label("As a designer, I want the palette tokenized"),
            None,
            "an axis with no team/owner/dev role yields no derivation — the --acceptor override (interim) sets it explicitly rather than deriving a wrong acceptor"
        );
    }

    #[test]
    fn an_incidental_role_token_in_the_situation_clause_is_not_the_use_axis() {
        assert_eq!(
            derive_acceptor_label(
                "When owner accepts a delivered story, the human watching the board needs to see ACCEPTED"
            ),
            None,
            "'owner' here is scenario-prose, not the use-axis subject — the derivation must read the 'As a [role]' use-axis, NOT body-scan incidental role tokens (#1787 falsifier: the derived acceptor must not diverge from the use-axis)"
        );
        assert_eq!(
            derive_acceptor_label("As a dev, I want the owner notified when CI breaks"),
            Some(LABEL_AWAITING_DEV),
            "the use-axis is 'As a dev'; 'owner' after the clause is incidental and must not flip the acceptor to owner"
        );
    }

    #[test]
    fn a_user_job_without_a_clean_as_a_use_axis_falls_back_to_none() {
        assert_eq!(
            derive_acceptor_label("When a pm files, dev needs the bug verified"),
            None,
            "no leading 'As a [role]' clause → no derivation (sane fallback); the explicit --acceptor or label sets it"
        );
    }
}
