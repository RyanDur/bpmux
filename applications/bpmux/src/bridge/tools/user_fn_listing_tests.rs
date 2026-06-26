#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::user_fn::handle_list_user_fns;
use super::user_fn_test_support::CwdGuard;

#[tokio::test]
async fn handle_list_user_fns_returns_empty_when_dir_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let _cwd = CwdGuard::enter(tmp.path());
    let out = handle_list_user_fns(&serde_json::Map::new(), "dev")
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["functions"], serde_json::json!([]));
}

#[tokio::test]
async fn handle_list_user_fns_enumerates_definitions_with_shape() {
    let tmp = tempfile::tempdir().unwrap();
    let fns_dir = tmp.path().join(".bpmux").join("functions");
    std::fs::create_dir_all(&fns_dir).unwrap();
    std::fs::write(
        fns_dir.join("greet.md"),
        "---\nname: greet\ndescription: Greet someone\ntype: user_fn\nargs: [who]\n---\nHi.",
    )
    .unwrap();
    std::fs::write(
        fns_dir.join("audit.md"),
        "---\nname: audit\ndescription: Run an audit\ntype: user_fn\n---\nAudit body.",
    )
    .unwrap();
    std::fs::write(fns_dir.join("README.txt"), "ignored").unwrap();
    let _cwd = CwdGuard::enter(tmp.path());
    let out = handle_list_user_fns(&serde_json::Map::new(), "dev")
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    let fns = parsed["functions"].as_array().unwrap();
    assert_eq!(fns.len(), 2);
    assert_eq!(fns[0]["name"], "audit");
    assert_eq!(fns[0]["description"], "Run an audit");
    assert!(fns[0].get("args").is_none());
    assert_eq!(fns[1]["name"], "greet");
    assert_eq!(fns[1]["args"], serde_json::json!(["who"]));
}
