//! Bridge-owned response shapes — `bridge::view_model::*` types are
//! the public-API shapes returned by bridge tools to MCP callers.
//!
//! Per architecture.md Friction #2 (#1363): bridge tools previously
//! serialized `session::*` types directly via their `Serialize`
//! derive, leaking the session schema across the bridge↔MCP wire. A
//! field rename in `session/identity.rs` would silently change the
//! MCP JSON shape — making bridge a downstream of session refactors.
//!
//! The view-model boundary inverts that dependency: bridge owns its
//! response shapes, and per-class `From<&session::Foo>` translations
//! decide what to expose. Session schema changes are absorbed by the
//! translation step, not propagated to MCP consumers.
//!
//! This is the foundation slice — slice 1 of the multi-slice
//! migration. Subsequent slices populate the module with views for
//! each bridge-tool class (identity write-path / briefing / tokens /
//! bookmark / messages / etc.).

use serde::Serialize;

use crate::session;

/// Mirror of `session::identity::IdentityValue` exposed at the
/// bridge↔MCP boundary. Field-for-field today; the boundary's value
/// lands when session::IdentityValue gains/renames fields — the
/// translation in `From` controls what flows to MCP.
#[derive(Debug, Clone, Serialize)]
pub struct IdentityValueView {
    pub name: String,
    pub description: String,
    pub principles: Vec<String>,
}

impl From<&session::identity::IdentityValue> for IdentityValueView {
    fn from(value: &session::identity::IdentityValue) -> Self {
        Self {
            name: value.name.clone(),
            description: value.description.clone(),
            principles: value.principles.clone(),
        }
    }
}

/// Mirror of `session::identity::IdentityIndex` — the collection
/// of identity values for one role.
#[derive(Debug, Clone, Serialize, Default)]
pub struct IdentityIndexView {
    pub values: Vec<IdentityValueView>,
}

impl From<&session::identity::IdentityIndex> for IdentityIndexView {
    fn from(index: &session::identity::IdentityIndex) -> Self {
        Self {
            values: index.values.iter().map(IdentityValueView::from).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_value_view_mirrors_source_fields() {
        let src = session::identity::IdentityValue {
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
        let src = session::identity::IdentityIndex {
            values: vec![
                session::identity::IdentityValue {
                    name: "alpha".to_string(),
                    description: "a".to_string(),
                    principles: vec![],
                },
                session::identity::IdentityValue {
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
        let src = session::identity::IdentityIndex {
            values: vec![session::identity::IdentityValue {
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
}
