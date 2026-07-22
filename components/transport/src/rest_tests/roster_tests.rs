use store::{Presence, RosterMember};

use crate::rest::roster_entries;

#[test]
fn a_roster_entry_carries_the_member_routing_id_so_an_agent_can_target_it_with_a_direct_send() {
    let members = vec![RosterMember {
        routing_id: "6b3054e9-dir-keyed".to_string(),
        identity: "dev:claude-code".to_string(),
        presence: Presence::Online,
    }];

    let entries = roster_entries(members);

    let json = serde_json::to_value(&entries).expect("roster entries serialize");
    assert_eq!(json[0]["id"], "6b3054e9-dir-keyed");
    assert_eq!(json[0]["identity"], "dev:claude-code");
    assert_eq!(json[0]["role"], "dev");
    assert_eq!(json[0]["presence"], "Online");
}
