use super::{RosterMember, RosterRole, display_name, role_of, roster_role_for_identity_prefix};

fn roster_role_for_user(assigned: Option<&str>, identity: &str) -> RosterRole {
    match assigned {
        Some(role) => roster_role_for_identity_prefix(role),
        None => role_of(identity),
    }
}

pub(crate) fn members_wire(
    members: &[RosterMember],
    assigned_role: impl Fn(&str) -> Option<String>,
) -> Vec<message::RosterMemberWire> {
    members
        .iter()
        .map(|m| {
            let assigned = assigned_role(&m.routing_id);
            message::RosterMemberWire {
                routing_id: m.routing_id.clone(),
                name: display_name(&m.identity).to_string(),
                role: roster_role_for_user(assigned.as_deref(), &m.identity).into(),
                identity: m.identity.clone(),
                presence: m.presence.into(),
            }
        })
        .collect()
}
