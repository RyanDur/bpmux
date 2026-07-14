#![allow(clippy::unwrap_used)]

use super::{Presence, RosterRole};
use message::{PresenceWire, RosterRoleWire};

#[test]
fn roster_role_and_presence_wire_mirrors_serialize_identically_to_domain() {
    for role in [
        RosterRole::Designer,
        RosterRole::Dev,
        RosterRole::Pm,
        RosterRole::Owner,
        RosterRole::Other,
    ] {
        assert_eq!(
            serde_json::to_value(role).unwrap(),
            serde_json::to_value(RosterRoleWire::from(role)).unwrap(),
            "RosterRoleWire must serialize byte-identically (PascalCase) to the domain RosterRole — the renderer decodes these strings"
        );
    }

    for presence in [
        Presence::Online,
        Presence::Busy,
        Presence::Idle,
        Presence::Offline,
        Presence::Blocked,
        Presence::Help,
    ] {
        assert_eq!(
            serde_json::to_value(presence).unwrap(),
            serde_json::to_value(PresenceWire::from(presence)).unwrap(),
            "PresenceWire must serialize byte-identically to the domain Presence"
        );
    }
}
