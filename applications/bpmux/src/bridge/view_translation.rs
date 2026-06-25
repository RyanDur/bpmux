#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::view_model::*;
use crate::memory;

#[test]
fn identity_value_view_mirrors_source_fields() {
    let src = memory::identity::IdentityValue {
        name: "iteration over prediction".to_string(),
        description: "ship and learn".to_string(),
        principles: vec!["small steps".to_string()],
    };
    let view = IdentityValueView::from(&src);
    assert_eq!(view.name, "iteration over prediction");
    assert_eq!(view.description, "ship and learn");
    assert_eq!(view.principles, vec!["small steps".to_string()]);
}

#[test]
fn identity_index_view_translates_full_collection() {
    let src = memory::identity::IdentityIndex {
        values: vec![
            memory::identity::IdentityValue {
                name: "alpha".to_string(),
                description: "a".to_string(),
                principles: vec![],
            },
            memory::identity::IdentityValue {
                name: "beta".to_string(),
                description: "b".to_string(),
                principles: vec!["p1".to_string(), "p2".to_string()],
            },
        ],
    };
    let view = IdentityIndexView::from(&src);
    assert_eq!(view.values.len(), 2);
    assert_eq!(view.values[0].name, "alpha");
    assert_eq!(view.values[1].principles.len(), 2);
}

#[test]
fn identity_index_view_serializes_to_stable_json_shape() {
    let src = memory::identity::IdentityIndex {
        values: vec![memory::identity::IdentityValue {
            name: "iteration".to_string(),
            description: "ship".to_string(),
            principles: vec!["small".to_string()],
        }],
    };
    let view = IdentityIndexView::from(&src);
    let json = serde_json::to_string(&view).unwrap_or_default();
    assert!(!json.is_empty(), "view must serialize");
    assert!(json.contains("\"values\""));
    assert!(json.contains("\"name\":\"iteration\""));
    assert!(json.contains("\"description\":\"ship\""));
    assert!(json.contains("\"principles\":[\"small\"]"));
}
