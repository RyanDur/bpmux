#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::*;
use chrono::{DateTime, Utc};

#[test]
fn ensure_id_mints_a_valid_uuid_when_absent() {
    let mut msg = Message::default();
    assert!(msg.id.is_empty());
    msg.ensure_id();
    assert!(uuid::Uuid::parse_str(&msg.id).is_ok());
}

#[test]
fn ensure_id_preserves_an_id_that_is_already_set() {
    let mut msg = Message {
        id: "already-minted".to_string(),
        ..Message::default()
    };
    msg.ensure_id();
    assert_eq!(msg.id, "already-minted");
}

#[test]
fn chat_message_is_chat() {
    let msg = Message {
        from: "dev:human".to_string(),
        text: "hello".to_string(),
        ts: Utc::now(),
        ..Message::default()
    };
    assert!(msg.is_chat());
}

#[test]
fn system_message_is_not_chat() {
    let msg = Message {
        from: "owner:claude-code".to_string(),
        text: "[turn complete]".to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        ..Message::default()
    };
    assert!(!msg.is_chat());
}

#[test]
fn explicit_chat_type_is_chat() {
    let msg = Message {
        from: "dev:human".to_string(),
        text: "hello".to_string(),
        ts: Utc::now(),
        message_type: MessageType::Chat,
        ..Message::default()
    };
    assert!(msg.is_chat());
}

#[test]
fn serializes_message_to_json() {
    let msg = Message {
        from: "owner:human".to_string(),
        text: "hello world".to_string(),
        ts: DateTime::parse_from_rfc3339("2026-04-02T12:00:00Z")
            .expect("test timestamp")
            .with_timezone(&Utc),
        ..Message::default()
    };

    let json = serde_json::to_string(&msg).expect("serialize");
    let parsed: Message = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(parsed, msg);
}

#[test]
fn addressee_omitted_by_default_in_json() {
    let msg = Message {
        from: "owner:human".to_string(),
        text: "hi".to_string(),
        ts: Utc::now(),
        ..Message::default()
    };
    let json = serde_json::to_string(&msg).expect("serialize");
    assert!(!json.contains("addressee"));
}

#[test]
fn addressee_roundtrips_through_json() {
    let msg = Message {
        from: "owner:human".to_string(),
        text: "what about X?".to_string(),
        ts: Utc::now(),
        addressee: Some("pm".to_string()),
        ..Message::default()
    };
    let json = serde_json::to_string(&msg).expect("serialize");
    assert!(json.contains(r#""addressee":"pm""#));
    let parsed: Message = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.addressee.as_deref(), Some("pm"));
}

#[test]
fn addressee_deserializes_from_legacy_json() {
    let json = r#"{"from":"owner:human","text":"hi","ts":"2026-04-02T12:00:00Z","to":null}"#;
    let parsed: Message = serde_json::from_str(json).expect("deserialize");
    assert!(parsed.addressee.is_none());
}

#[test]
fn addressed_to_user_none_is_not_addressed_to_anyone_the_bridge_live_semantic() {
    let msg = Message {
        addressee: None,
        ..Message::default()
    };
    assert!(
        !msg.addressed_to_user(None, "dev"),
        "the dead is_addressed_to's None->true semantic died with it — the bridge's live semantic (connection.rs addressed_to_me) treats an unaddressed message as NOT specifically addressed"
    );
    assert!(!msg.addressed_to_user(None, "pm"));
    assert!(!msg.addressed_to_user(None, "designer"));
}

#[test]
fn addressed_to_user_all_is_not_addressed_to_anyone_specifically() {
    let msg = Message {
        addressee: Some("all".to_string()),
        ..Message::default()
    };
    assert!(!msg.addressed_to_user(None, "dev"));
    assert!(!msg.addressed_to_user(None, "pm"));
    assert!(!msg.addressed_to_user(None, "designer"));
}

#[test]
fn addressed_to_user_specific_role_excludes_others_by_display() {
    let msg = Message {
        addressee: Some("pm".to_string()),
        ..Message::default()
    };
    assert!(msg.addressed_to_user(None, "pm"));
    assert!(!msg.addressed_to_user(None, "dev"));
    assert!(!msg.addressed_to_user(None, "designer"));
}

#[test]
fn addressed_to_user_matches_by_id_and_excludes_a_display_sibling() {
    let msg = Message {
        addressee: Some("pm".to_string()),
        addressee_ids: Some(vec!["id-pm-1".to_string()]),
        ..Message::default()
    };
    assert!(msg.addressed_to_user(Some("id-pm-1"), "pm"));
    assert!(
        !msg.addressed_to_user(Some("id-pm-2"), "pm"),
        "id-pm-2 shares the display 'pm' with id-pm-1 but is a DIFFERENT user — id precision must not deliver to it just because the display matches"
    );
}

#[test]
fn reaches_user_matches_by_id_and_excludes_a_display_sibling() {
    let msg = Message {
        to: Some(vec!["dev".to_string()]),
        to_ids: Some(vec!["id-dev-1".to_string()]),
        ..Message::default()
    };
    assert!(msg.reaches_user(Some("id-dev-1"), "dev"));
    assert!(
        !msg.reaches_user(Some("id-dev-2"), "dev"),
        "id-dev-2 shares the display 'dev' with id-dev-1 but is a DIFFERENT user — a frozen to_ids list must not deliver to it just because the display matches"
    );
}

#[test]
fn reaches_user_falls_back_to_display_when_to_ids_is_none_a_legacy_row() {
    let msg = Message {
        to: Some(vec!["dev".to_string()]),
        to_ids: None,
        ..Message::default()
    };
    assert!(msg.reaches_user(Some("id-dev-1"), "dev"));
    assert!(msg.reaches_user(Some("id-dev-2"), "dev"));
    assert!(!msg.reaches_user(Some("id-pm-1"), "pm"));
}

#[test]
fn reaches_user_treats_to_ids_absence_of_the_caller_id_as_no_match() {
    let msg = Message {
        to: Some(vec!["dev".to_string()]),
        to_ids: Some(vec!["id-dev-1".to_string()]),
        ..Message::default()
    };
    assert!(
        !msg.reaches_user(None, "dev"),
        "to_ids present but the caller's own id is unknown — false, never guess by display once ids are frozen"
    );
}

#[test]
fn reaches_user_broadcast_all_reaches_everyone_even_though_all_never_resolves_to_an_id() {
    let msg = Message {
        to: Some(vec![BROADCAST_ALL.to_string()]),
        to_ids: Some(vec![]),
        ..Message::default()
    };
    assert!(
        msg.reaches_user(Some("id-dev-1"), "dev"),
        "\"all\" never resolves to a registered id — our ingress never stamps Some(empty), but a peer relay or hand-built frame can still carry it, so reaches_user must special-case broadcast before consulting to_ids"
    );
    assert!(msg.reaches_user(None, "designer"));
}

#[test]
fn is_from_user_matches_by_id_and_does_not_treat_a_display_sibling_as_the_sender() {
    let msg = Message {
        from: "dev".to_string(),
        sender_id: Some("id-dev-1".to_string()),
        ..Message::default()
    };
    assert!(msg.is_from_user(Some("id-dev-1"), "dev"));
    assert!(
        !msg.is_from_user(Some("id-dev-2"), "dev"),
        "id-dev-2 shares the display 'dev' with the true sender id-dev-1 but is a DIFFERENT user — sender-sees-own must not fire for a display sibling"
    );
}

#[test]
fn is_from_user_falls_back_to_display_when_sender_id_is_none_a_legacy_row() {
    let msg = Message {
        from: "dev".to_string(),
        sender_id: None,
        ..Message::default()
    };
    assert!(msg.is_from_user(Some("id-dev-1"), "dev"));
    assert!(msg.is_from_user(None, "dev"));
    assert!(!msg.is_from_user(Some("id-dev-1"), "pm"));
}

#[test]
fn message_without_multi_depth_serializes_without_the_field() {
    let msg = Message {
        from: "dev:claude-code".to_string(),
        text: "plain".to_string(),
        ts: Utc::now(),
        ..Message::default()
    };
    let json = serde_json::to_string(&msg).expect("serialize");
    assert!(
        !json.contains("multi_depth"),
        "absent field should be omitted from wire format: {json}"
    );
}

#[test]
fn message_deserializes_from_legacy_json_without_multi_depth() {
    let json = r#"{"from":"owner:human","text":"hi","ts":"2026-04-02T12:00:00Z","to":null}"#;
    let parsed: Message = serde_json::from_str(json).expect("deserialize");
    assert!(parsed.multi_depth.is_none());
}

#[test]
fn multi_depth_payload_roundtrips_preloaded_deep_detail() {
    let mut role_default = std::collections::HashMap::new();
    role_default.insert("pm".to_string(), "CI ✗".to_string());
    role_default.insert(
        "dev".to_string(),
        "CI ✗ — run #12345 failed at cargo clippy".to_string(),
    );
    let payload = MultiDepthPayload {
        summary: "CI ✗".to_string(),
        role_default,
        deep: DeepDetail::Preloaded {
            text: "full build log contents here".to_string(),
        },
    };
    let msg = Message {
        from: "bpmux".to_string(),
        text: "CI ✗".to_string(),
        ts: Utc::now(),
        multi_depth: Some(payload.clone()),
        ..Message::default()
    };
    let json = serde_json::to_string(&msg).expect("serialize");
    let parsed: Message = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.multi_depth, Some(payload));
}

#[test]
fn multi_depth_payload_roundtrips_on_demand_deep_detail() {
    let payload = MultiDepthPayload {
        summary: "CI ✗".to_string(),
        role_default: std::collections::HashMap::new(),
        deep: DeepDetail::OnDemand {
            command: "gh run view 12345".to_string(),
            description: "click to load the build log".to_string(),
        },
    };
    let msg = Message {
        from: "bpmux".to_string(),
        text: "CI ✗".to_string(),
        ts: Utc::now(),
        multi_depth: Some(payload.clone()),
        ..Message::default()
    };
    let json = serde_json::to_string(&msg).expect("serialize");
    assert!(json.contains(r#""kind":"on_demand""#), "got: {json}");
    assert!(json.contains(r#""command":"gh run view 12345""#));
    let parsed: Message = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.multi_depth, Some(payload));
}

#[test]
fn multi_depth_with_empty_role_default_still_roundtrips() {
    let payload = MultiDepthPayload {
        summary: "ok".to_string(),
        role_default: std::collections::HashMap::new(),
        deep: DeepDetail::Preloaded {
            text: "detail".to_string(),
        },
    };
    let json = serde_json::to_string(&payload).expect("serialize");
    let parsed: MultiDepthPayload = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed, payload);
}

#[test]
fn addressed_to_user_default_message_is_not_addressed_to_anyone() {
    let msg = Message::default();
    assert!(!msg.addressed_to_user(None, "dev"));
}

#[test]
fn addressed_to_user_is_independent_of_to() {
    let broadcast_with_addressee = Message {
        to: None,
        addressee: Some("pm".to_string()),
        ..Message::default()
    };
    assert!(broadcast_with_addressee.addressed_to_user(None, "pm"));
    assert!(!broadcast_with_addressee.addressed_to_user(None, "dev"));

    let dm_no_addressee = Message {
        to: Some(vec!["pm".to_string()]),
        addressee: None,
        ..Message::default()
    };
    assert!(
        !dm_no_addressee.addressed_to_user(None, "pm"),
        "addressed_to_user reads ONLY the addressee field — a `to` DM with no addressee is NOT addressed_to_user (the bridge :150 live semantic); is_addressed_to's None->true died with the dead helper"
    );
    assert!(!dm_no_addressee.addressed_to_user(None, "dev"));
}

#[test]
fn nudge_message_is_not_chat() {
    let msg = Message {
        message_type: MessageType::Nudge,
        ..Message::default()
    };
    assert!(
        !msg.is_chat(),
        "Nudge is a system-authored operational signal, not chat — it must classify non-chat so the noise-filter hides it"
    );
}

#[test]
fn chat_omits_message_type_but_non_chat_serializes_it() {
    let chat = Message {
        message_type: MessageType::Chat,
        ..Message::default()
    };
    let chat_json = serde_json::to_string(&chat).expect("serialize");
    assert!(
        !chat_json.contains("message_type"),
        "Chat must omit message_type on the wire (default-chat): {chat_json}"
    );

    let insight = Message {
        message_type: MessageType::Insight,
        ..Message::default()
    };
    let insight_json = serde_json::to_string(&insight).expect("serialize");
    assert!(
        insight_json.contains(r#""message_type":"insight""#),
        "non-Chat types must serialize their wire-type so the renderer keeps the class-glyph: {insight_json}"
    );
}

#[test]
fn message_with_unknown_state_delta_variant_parses_as_none() {
    let raw = r#"{
          "from": "bpmux",
          "text": "",
          "ts": "2026-05-15T01:23:45Z",
          "message_type": "state-delta",
          "state_delta": {"domain": "icebox_migration", "details": "retired"}
        }"#;
    let m: Message = serde_json::from_str(raw).expect("retired variant must parse tolerantly");
    assert_eq!(m.from, "bpmux");
    assert!(m.state_delta.is_none());
}

#[test]
fn message_with_known_state_delta_variant_still_parses_canonically() {
    let raw = r#"{
          "from": "bpmux",
          "text": "",
          "ts": "2026-05-16T01:23:45Z",
          "message_type": "state-delta",
          "state_delta": {"domain": "roster", "members": []}
        }"#;
    let m: Message = serde_json::from_str(raw).expect("known variant parses");
    assert!(matches!(
        m.state_delta,
        Some(StateDeltaPayload::Roster { members: _ })
    ));
}

#[test]
fn ci_delta_wire_json_is_byte_identical_for_every_ci_state_string() {
    for state in ["unknown", "green", "red", "running", "quota_exhausted"] {
        let wire = serde_json::json!({
            "domain": "ci",
            "snapshot": {
                "state": state,
                "run_id": null,
                "head_sha": null,
                "display_title": null,
                "last_successful_poll_at": null,
                "url": null
            }
        });
        let payload: StateDeltaPayload =
            serde_json::from_value(wire.clone()).expect("ci delta parses");
        assert_eq!(
            serde_json::to_value(&payload).expect("ci delta serializes"),
            wire,
            "the renderer pins these CI-state strings — the wire must round-trip byte-identical for {state}"
        );
    }
}

#[test]
fn roster_delta_wire_json_is_byte_identical_for_every_role_and_presence_string() {
    for role in ["Designer", "Dev", "Pm", "Owner", "Other"] {
        for presence in ["Online", "Busy", "Idle", "Offline", "Blocked", "Help"] {
            let wire = serde_json::json!({
                "domain": "roster",
                "members": [{
                    "routing_id": "r1",
                    "name": "n",
                    "identity": "i",
                    "role": role,
                    "presence": presence
                }]
            });
            let payload: StateDeltaPayload =
                serde_json::from_value(wire.clone()).expect("roster delta parses");
            assert_eq!(
                serde_json::to_value(&payload).expect("roster delta serializes"),
                wire,
                "roster role/presence ride PascalCase on the wire — must round-trip identical for {role}/{presence}"
            );
        }
    }
}

#[test]
fn persisted_ci_delta_without_url_or_poll_time_survives_and_keeps_typed_values() {
    let raw = r#"{
        "from": "bpmux",
        "text": "",
        "ts": "2026-05-15T01:23:45Z",
        "message_type": "state-delta",
        "state_delta": {"domain": "ci", "snapshot": {"state": "red", "run_id": 7}}
    }"#;
    let m: Message = serde_json::from_str(raw).expect("a historical ci delta must parse");
    let payload = m
        .state_delta
        .expect("the ci delta must survive the tolerant deserializer, not silently vanish");
    let reserialized = serde_json::to_value(&payload).expect("serialize");
    assert_eq!(reserialized["domain"], "ci");
    assert_eq!(reserialized["snapshot"]["state"], "red");
    assert_eq!(reserialized["snapshot"]["run_id"], 7);
    assert_eq!(
        reserialized["snapshot"]["url"],
        serde_json::Value::Null,
        "a delta persisted before the url field existed defaults url to null — the payload is never dropped"
    );
}

#[test]
fn persisted_roster_delta_with_pascalcase_survives_and_keeps_typed_values() {
    let raw = r#"{
        "from": "bpmux",
        "text": "",
        "ts": "2026-05-16T01:23:45Z",
        "message_type": "state-delta",
        "state_delta": {"domain": "roster", "members": [
            {"routing_id": "r1", "name": "n", "identity": "i", "role": "Dev", "presence": "Blocked"}
        ]}
    }"#;
    let m: Message = serde_json::from_str(raw).expect("a historical roster delta must parse");
    let payload = m
        .state_delta
        .expect("the roster delta must survive the tolerant deserializer");
    let reserialized = serde_json::to_value(&payload).expect("serialize");
    assert_eq!(reserialized["domain"], "roster");
    assert_eq!(reserialized["members"][0]["role"], "Dev");
    assert_eq!(reserialized["members"][0]["presence"], "Blocked");
}

#[test]
fn ci_state_changed_subscription_event_carries_the_snapshot_url() {
    let payload: StateDeltaPayload = serde_json::from_value(serde_json::json!({
        "domain": "ci",
        "snapshot": {"state": "green", "run_id": 99, "head_sha": "sha1", "display_title": "build",
                     "url": "https://github.com/acme/bpmux/actions"}
    }))
    .expect("ci delta parses");
    let (event_class, body) = payload
        .to_subscription_event("2026-07-03T00:00:00Z")
        .expect("a ci delta maps to a subscription event");
    assert_eq!(event_class, "ci_state_changed");
    assert_eq!(body["conclusion"], "green");
    assert_eq!(body["run_id"], 99);
    assert_eq!(body["head_sha"], "sha1");
    assert_eq!(body["display_title"], "build");
    assert_eq!(
        body["url"],
        serde_json::Value::String("https://github.com/acme/bpmux/actions".to_string()),
        "the wire snapshot is the one source of the actions url — the server stamps it at birth from the stored connection, and every reader (renderer, bridge, wait-api) relays it"
    );
}

#[test]
fn ci_state_changed_subscription_event_without_a_url_falls_back_to_the_github_base() {
    let payload: StateDeltaPayload = serde_json::from_value(serde_json::json!({
        "domain": "ci",
        "snapshot": {"state": "green", "run_id": 99, "head_sha": "sha1", "display_title": "build"}
    }))
    .expect("ci delta parses");
    let (_, body) = payload
        .to_subscription_event("2026-07-03T00:00:00Z")
        .expect("a ci delta maps to a subscription event");
    assert_eq!(
        body["url"],
        serde_json::Value::String(crate::GITHUB_BASE_URL.to_string()),
        "a delta from a sender that predates url-stamping still yields a usable link"
    );
}

#[test]
fn granular_mutation_deltas_are_nameable_agent_events_with_the_wire_shape_as_payload() {
    let cases: Vec<(StateDeltaPayload, &str, serde_json::Value)> = vec![
        (
            StateDeltaPayload::StoryUpserted {
                story: serde_json::json!({"id": 7, "title": "a story"}),
            },
            "story_upserted",
            serde_json::json!({"story": {"id": 7, "title": "a story"}}),
        ),
        (
            StateDeltaPayload::StoryRemoved { id: 7 },
            "story_removed",
            serde_json::json!({"id": 7}),
        ),
        (
            StateDeltaPayload::StoryOrder { ids: vec![3, 1, 2] },
            "story_order",
            serde_json::json!({"ids": [3, 1, 2]}),
        ),
        (
            StateDeltaPayload::EpicUpserted {
                epic: serde_json::json!({"id": 9}),
            },
            "epic_upserted",
            serde_json::json!({"epic": {"id": 9}}),
        ),
        (
            StateDeltaPayload::OutcomeRemoved { id: 4 },
            "outcome_removed",
            serde_json::json!({"id": 4}),
        ),
        (
            StateDeltaPayload::IterationOrder { ids: vec![5] },
            "iteration_order",
            serde_json::json!({"ids": [5]}),
        ),
        (
            StateDeltaPayload::RoadmapCardUpserted {
                card: serde_json::json!({"id": 2}),
            },
            "roadmap_card_upserted",
            serde_json::json!({"card": {"id": 2}}),
        ),
        (
            StateDeltaPayload::ReactionUpdated {
                channel: "board".to_string(),
                message_key: "m1".to_string(),
                emoji: "🎉".to_string(),
                count: 0,
                users: vec![],
            },
            "reaction_updated",
            serde_json::json!({
                "channel": "board", "message_key": "m1", "emoji": "🎉",
                "count": 0, "users": []
            }),
        ),
        (
            StateDeltaPayload::RosterMemberRemoved {
                routing_id: "dev-a".to_string(),
            },
            "roster_member_removed",
            serde_json::json!({"routing_id": "dev-a"}),
        ),
        (
            StateDeltaPayload::IterationClose {
                promoted: vec![11],
                deleted_marker_id: None,
            },
            "iteration_close",
            serde_json::json!({"promoted": [11], "deleted_marker_id": null}),
        ),
    ];
    for (payload, expected_class, expected_body) in cases {
        assert_eq!(
            payload.to_subscription_event("2026-07-28T00:00:00Z"),
            Some((expected_class, expected_body)),
            "agent event for {expected_class}"
        );
    }
}

#[test]
fn hydration_deltas_stay_gui_only_and_never_wake_agents() {
    let hydration: Vec<StateDeltaPayload> = vec![
        StateDeltaPayload::Stories {
            stories: serde_json::json!([]),
            columns: None,
        },
        StateDeltaPayload::Epics {
            epics: serde_json::json!([]),
        },
        StateDeltaPayload::Roster { members: vec![] },
        StateDeltaPayload::Reactions {
            channel: "board".to_string(),
            aggregated: vec![],
        },
        StateDeltaPayload::SyncPoll {
            status: serde_json::json!({}),
        },
    ];
    for payload in hydration {
        assert!(
            payload
                .to_subscription_event("2026-07-28T00:00:00Z")
                .is_none(),
            "hydration payloads are addressed catch-up, not events"
        );
    }
}

#[test]
fn reaches_user_an_explicit_id_send_is_direct_even_with_no_role_list() {
    let msg = Message {
        from: "dev".to_string(),
        to: None,
        to_ids: Some(vec!["pm-a-id".to_string()]),
        ..Message::default()
    };
    assert!(
        msg.reaches_user(Some("pm-a-id"), "pm"),
        "the targeted id is reached"
    );
    assert!(
        !msg.reaches_user(Some("pm-b-id"), "pm"),
        "a SAME-ROLE sibling is NOT reached — explicit ids are surgical, the role hat is irrelevant"
    );
    assert!(
        !msg.reaches_user(Some("dev-id"), "dev"),
        "explicit ids with no role list is a DIRECT send, not a broadcast — nobody outside the id list sees it"
    );
}
