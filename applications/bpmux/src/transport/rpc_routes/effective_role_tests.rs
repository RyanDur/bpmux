#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::effective_role;
use crate::store::SeededStore;
use crate::store::routing::RoutingAction;

fn seeded(registrations: &[(&str, &str, Option<&str>)]) -> SeededStore {
    let store = SeededStore::new(Vec::new(), Vec::new(), Vec::new());
    for (user_id, display, role) in registrations {
        store.dispatch(
            RoutingAction::Registered {
                user_id: (*user_id).to_string(),
                display: (*display).to_string(),
                role: role.map(str::to_string),
            }
            .into(),
        );
    }
    store
}

fn label_args() -> serde_json::Map<String, serde_json::Value> {
    let mut args = serde_json::Map::new();
    args.insert("id".to_string(), serde_json::json!(7));
    args.insert("label".to_string(), serde_json::json!("needs-love"));
    args
}

#[tokio::test]
async fn a_dev_registrant_presenting_pm_is_refused_a_pm_gated_tool() {
    let store = seeded(&[("sess-imposter", "pm", Some("dev"))]);
    let select = store.select();

    let role = effective_role(&select, "pm", Some("sess-imposter"));
    assert_eq!(
        role, "dev",
        "the REGISTERED role wins over the presented display string"
    );

    let err = crate::bridge::tools_stories::handle_label_story(
        &label_args(),
        "pm",
        &role,
        Some("sess-imposter"),
        None,
        &select,
    )
    .await
    .expect_err("a dev registrant presenting 'pm' must be refused the pm-gated label tool");
    assert!(
        err.contains("restricted to pm"),
        "expected a pm-gate refusal; got {err}"
    );
}

#[tokio::test]
async fn a_pm_registrant_presenting_a_nonrole_display_passes_pm_gates() {
    let store = seeded(&[("sess-pm", "aria", Some("pm"))]);
    let select = store.select();

    let role = effective_role(&select, "aria", Some("sess-pm"));
    assert_eq!(
        role, "pm",
        "a registered pm resolves to pm regardless of the non-role display it presents"
    );

    let err = crate::bridge::tools_stories::handle_label_story(
        &label_args(),
        "aria",
        &role,
        Some("sess-pm"),
        None,
        &select,
    )
    .await
    .expect_err("the pm gate passes, then the handler reaches the refresh-absent path");
    assert!(
        !err.contains("restricted to"),
        "a registered pm must clear the pm gate; got {err}"
    );
    assert!(
        err.contains("requires refresh"),
        "clearing the gate reaches the refresh-required path; got {err}"
    );
}

#[test]
fn without_an_actor_id_the_effective_role_falls_back_to_the_display_derivation() {
    let store = seeded(&[]);
    let select = store.select();
    assert_eq!(effective_role(&select, "pm:claude-code", None), "pm");
    assert_eq!(effective_role(&select, "dev", None), "dev");
}

#[test]
fn an_unassigned_or_unknown_actor_id_falls_back_to_the_display_derivation() {
    let store = seeded(&[("sess-legacy", "dev", None)]);
    let select = store.select();
    assert_eq!(
        effective_role(&select, "dev", Some("sess-legacy")),
        "dev",
        "a registrant with no assigned role keeps today's display derivation"
    );
    assert_eq!(
        effective_role(&select, "designer", Some("ghost-id")),
        "designer",
        "an unknown actor_id keeps today's display derivation"
    );
}
