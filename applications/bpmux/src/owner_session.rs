use crate::store::RosterRole;

#[derive(Default, Clone)]
pub struct OwnerSessionState {
    pub claimed_nonce: Option<String>,
}

#[derive(Clone, Debug)]
pub enum OwnerSessionAction {
    Claimed { nonce: String },
}

pub fn reduce(state: &OwnerSessionState, action: &OwnerSessionAction) -> Option<OwnerSessionState> {
    match action {
        OwnerSessionAction::Claimed { nonce } => {
            if state.claimed_nonce.as_deref() == Some(nonce.as_str()) {
                return None;
            }
            Some(OwnerSessionState {
                claimed_nonce: Some(nonce.clone()),
            })
        }
    }
}

pub fn claimed_nonce(state: &OwnerSessionState) -> Option<String> {
    state.claimed_nonce.clone()
}

pub fn resolve_owner_mark(claimed: Option<&str>, presented: Option<&str>, identity: &str) -> bool {
    match claimed {
        Some(claim) => presented == Some(claim),
        None => matches!(crate::roster::role_of(identity), RosterRole::Owner),
    }
}

pub fn owner_walk_authorized(claimed: Option<&str>, presented: Option<&str>) -> bool {
    match claimed {
        Some(claim) => presented == Some(claim),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn a_claim_records_the_nonce_and_a_repeat_claim_is_a_no_op() {
        let claimed = reduce(
            &OwnerSessionState::default(),
            &OwnerSessionAction::Claimed {
                nonce: "n1".to_string(),
            },
        )
        .expect("a first claim records the nonce");
        assert_eq!(claimed.claimed_nonce.as_deref(), Some("n1"));
        assert!(
            reduce(
                &claimed,
                &OwnerSessionAction::Claimed {
                    nonce: "n1".to_string()
                }
            )
            .is_none(),
            "re-claiming the same nonce is a no-op (no needless state churn)"
        );
    }

    #[test]
    fn a_fresh_claim_supersedes_the_prior_last_claim_wins() {
        let first = reduce(
            &OwnerSessionState::default(),
            &OwnerSessionAction::Claimed {
                nonce: "n1".to_string(),
            },
        )
        .expect("first claim");
        let second = reduce(
            &first,
            &OwnerSessionAction::Claimed {
                nonce: "n2".to_string(),
            },
        )
        .expect("a fresh launch claims a new nonce and supersedes the old");
        assert_eq!(
            second.claimed_nonce.as_deref(),
            Some("n2"),
            "last-claim-wins: the constant is whoever started the CURRENT app"
        );
    }

    #[test]
    fn before_any_claim_the_display_string_owner_is_the_owner_fallback() {
        assert!(
            resolve_owner_mark(None, None, "owner:electron"),
            "bootstrap fallback: before a nonce is claimed this process, a string-owner session keeps owner-ness"
        );
        assert!(
            !resolve_owner_mark(None, None, "dev:claude-code"),
            "a non-owner display is never the owner, claim or no claim"
        );
    }

    #[test]
    fn a_claimed_nonce_denies_a_string_owner_that_presents_no_nonce() {
        assert!(
            !resolve_owner_mark(Some("n1"), None, "owner:electron"),
            "THE centerpiece: once a nonce is claimed, a session identifying as name=owner WITHOUT the nonce is NOT the owner — the structural fix for self-declared owner-ness"
        );
    }

    #[test]
    fn a_claimed_nonce_grants_any_name_that_presents_it() {
        assert!(
            resolve_owner_mark(Some("n1"), Some("n1"), "anything:cli"),
            "the twin: the session presenting the claimed nonce is the owner regardless of the name it sent — owner-ness anchors to the launch, not the string"
        );
        assert!(
            !resolve_owner_mark(Some("n1"), Some("n2"), "owner:electron"),
            "a stale/foreign nonce grants nothing"
        );
    }

    #[test]
    fn owner_walk_before_any_claim_proceeds_as_the_bootstrap_fallback() {
        assert!(
            owner_walk_authorized(None, None),
            "before any nonce is claimed this process, the board walk proceeds (bootstrap)"
        );
        assert!(
            owner_walk_authorized(None, Some("stray")),
            "a stray presented nonce is harmless before any claim exists"
        );
    }

    #[test]
    fn owner_walk_once_claimed_requires_the_launch_nonce_both_directions() {
        assert!(
            owner_walk_authorized(Some("n1"), Some("n1")),
            "the owner session presenting its launch nonce is authorized to walk the board"
        );
        assert!(
            !owner_walk_authorized(Some("n1"), None),
            "once a nonce is claimed, a board walk WITHOUT it is refused — the session-authority fix"
        );
        assert!(
            !owner_walk_authorized(Some("n1"), Some("n2")),
            "a stale/foreign nonce is refused once a claim exists"
        );
    }
}
