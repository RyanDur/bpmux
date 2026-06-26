#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::user_fn::handle_user_fn;
use super::user_fn_test_support::CwdGuard;

#[tokio::test]
async fn rejects_path_traversal_in_name() {
    let tmp = tempfile::tempdir().unwrap();
    let _cwd = CwdGuard::enter(tmp.path());
    for evil in ["../escape", "nested/sub", ""] {
        let mut args = serde_json::Map::new();
        args.insert("name".into(), serde_json::json!(evil));
        let outcome = handle_user_fn(&args, "dev").await;
        assert!(
            outcome.is_err(),
            "a name that is not a single path segment ('/', '..', or empty) is rejected before any file read — the workspace boundary cannot be escaped; name {evil:?} produced {outcome:?}"
        );
    }
}

#[tokio::test]
async fn handle_user_fn_returns_not_defined_when_file_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let _cwd = CwdGuard::enter(tmp.path());
    let mut args = serde_json::Map::new();
    args.insert("name".into(), serde_json::json!("does_not_exist"));
    let out = handle_user_fn(&args, "dev").await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["status"], "not_defined");
    let message = parsed["message"].as_str().unwrap();
    assert!(message.contains("does_not_exist"));
    assert!(
        message.contains("for role dev"),
        "message must name role per falsifier 12; got {message:?}"
    );
    assert!(
        message.contains("bpmux-dev/.bpmux/functions/does_not_exist.md"),
        "message must use canonical bpmux-<role>/.bpmux/... shape per falsifier 12; got {message:?}"
    );
    assert!(
        !message.contains("relative to workspace cwd"),
        "message must not leak bridge-internal vocabulary; got {message:?}"
    );
}

#[tokio::test]
async fn handle_user_fn_returns_substituted_body() {
    let tmp = tempfile::tempdir().unwrap();
    let fns_dir = tmp.path().join(".bpmux").join("functions");
    std::fs::create_dir_all(&fns_dir).unwrap();
    std::fs::write(
            fns_dir.join("greet.md"),
            "---\nname: greet\ndescription: Greet someone\ntype: user_fn\nargs: [who]\n---\nHello {{who}}!\n",
        )
        .unwrap();
    let _cwd = CwdGuard::enter(tmp.path());
    let mut args = serde_json::Map::new();
    args.insert("name".into(), serde_json::json!("greet"));
    args.insert("args".into(), serde_json::json!({ "who": "team" }));
    let out = handle_user_fn(&args, "dev").await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["body"], "Hello team!\n");
}

#[tokio::test]
async fn handle_user_fn_returns_missing_arg_when_slot_unfilled() {
    let tmp = tempfile::tempdir().unwrap();
    let fns_dir = tmp.path().join(".bpmux").join("functions");
    std::fs::create_dir_all(&fns_dir).unwrap();
    std::fs::write(
            fns_dir.join("greet.md"),
            "---\nname: greet\ndescription: Greet someone\ntype: user_fn\nargs: [who]\n---\nHello {{who}}!\n",
        )
        .unwrap();
    let _cwd = CwdGuard::enter(tmp.path());
    let mut args = serde_json::Map::new();
    args.insert("name".into(), serde_json::json!("greet"));
    let out = handle_user_fn(&args, "dev").await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["status"], "missing_arg");
    assert!(parsed["message"].as_str().unwrap().contains("who"));
}

#[tokio::test]
async fn handle_user_fn_hot_reloads_on_re_invoke() {
    let tmp = tempfile::tempdir().unwrap();
    let fns_dir = tmp.path().join(".bpmux").join("functions");
    std::fs::create_dir_all(&fns_dir).unwrap();
    let path = fns_dir.join("v.md");
    std::fs::write(
        &path,
        "---\nname: v\ndescription: First\ntype: user_fn\n---\nv1\n",
    )
    .unwrap();
    let _cwd = CwdGuard::enter(tmp.path());
    let mut args = serde_json::Map::new();
    args.insert("name".into(), serde_json::json!("v"));
    let first = handle_user_fn(&args, "dev").await.unwrap();
    std::fs::write(
        &path,
        "---\nname: v\ndescription: Second\ntype: user_fn\n---\nv2\n",
    )
    .unwrap();
    let second = handle_user_fn(&args, "dev").await.unwrap();
    let p1: serde_json::Value = serde_json::from_str(&first).unwrap();
    let p2: serde_json::Value = serde_json::from_str(&second).unwrap();
    assert_eq!(p1["body"], "v1\n");
    assert_eq!(p2["body"], "v2\n");
}

#[tokio::test]
async fn handle_user_fn_role_isolation_same_name_different_role_resolves_distinctly() {
    let pm_workspace = tempfile::tempdir().unwrap();
    let dev_workspace = tempfile::tempdir().unwrap();
    let pm_fns = pm_workspace.path().join(".bpmux").join("functions");
    let dev_fns = dev_workspace.path().join(".bpmux").join("functions");
    std::fs::create_dir_all(&pm_fns).unwrap();
    std::fs::create_dir_all(&dev_fns).unwrap();
    std::fs::write(
        pm_fns.join("walk.md"),
        "---\nname: walk\ndescription: PM walk\ntype: user_fn\n---\nPM body.\n",
    )
    .unwrap();
    std::fs::write(
        dev_fns.join("walk.md"),
        "---\nname: walk\ndescription: dev walk\ntype: user_fn\n---\nDev body.\n",
    )
    .unwrap();
    let mut args = serde_json::Map::new();
    args.insert("name".into(), serde_json::json!("walk"));
    let pm_out = {
        let _cwd = CwdGuard::enter(pm_workspace.path());
        handle_user_fn(&args, "pm").await.unwrap()
    };
    let dev_out = {
        let _cwd = CwdGuard::enter(dev_workspace.path());
        handle_user_fn(&args, "dev").await.unwrap()
    };
    let pm_parsed: serde_json::Value = serde_json::from_str(&pm_out).unwrap();
    let dev_parsed: serde_json::Value = serde_json::from_str(&dev_out).unwrap();
    assert_eq!(pm_parsed["body"], "PM body.\n");
    assert_eq!(dev_parsed["body"], "Dev body.\n");
    assert_ne!(pm_parsed["body"], dev_parsed["body"]);
    assert_eq!(pm_parsed["description"], "PM walk");
    assert_eq!(dev_parsed["description"], "dev walk");
}
