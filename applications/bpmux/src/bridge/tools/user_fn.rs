use std::path::PathBuf;

const FRONTMATTER_FENCE: &str = "---";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserFnDefinition {
    pub name: String,
    pub description: String,
    pub args: Vec<String>,
    pub body: String,
}

fn functions_dir() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("resolve cwd: {e}"))?;
    Ok(cwd.join(".bpmux").join("functions"))
}

fn definition_path(name: &str) -> Result<PathBuf, String> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(format!(
            "user_fn name '{name}' must be a single path segment (no '/', '\\\\', or '..')"
        ));
    }
    Ok(functions_dir()?.join(format!("{name}.md")))
}

pub fn parse_definition(file_text: &str) -> Result<UserFnDefinition, String> {
    let trimmed = file_text.trim_start_matches('\u{feff}');
    let after_open = trimmed
        .strip_prefix(FRONTMATTER_FENCE)
        .ok_or_else(|| format!("expected leading '{FRONTMATTER_FENCE}' frontmatter fence"))?;
    let after_open = after_open.strip_prefix('\n').unwrap_or(after_open);
    let close_idx = after_open
        .find("\n---\n")
        .or_else(|| after_open.find("\n---"))
        .ok_or_else(|| {
            format!("expected closing '{FRONTMATTER_FENCE}' fence after frontmatter block")
        })?;
    let frontmatter = &after_open[..close_idx];
    let after_close = &after_open[close_idx + 1..];
    let body_start = after_close
        .strip_prefix(FRONTMATTER_FENCE)
        .map(|rest| rest.strip_prefix('\n').unwrap_or(rest))
        .unwrap_or(after_close);
    let mut name = String::new();
    let mut description = String::new();
    let mut type_value = String::new();
    let mut args: Vec<String> = Vec::new();
    for raw in frontmatter.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once(':')
            .ok_or_else(|| format!("frontmatter line missing ':' separator: {line:?}"))?;
        let key = key.trim();
        let value = value.trim();
        match key {
            "name" => name = strip_quotes(value).to_string(),
            "description" => description = strip_quotes(value).to_string(),
            "type" => type_value = strip_quotes(value).to_string(),
            "args" => args = parse_args_list(value)?,
            _ => {}
        }
    }
    if name.is_empty() {
        return Err("frontmatter missing required 'name'".to_string());
    }
    if description.is_empty() {
        return Err("frontmatter missing required 'description'".to_string());
    }
    if type_value != "user_fn" {
        return Err(format!(
            "frontmatter 'type' must be 'user_fn'; got {type_value:?}"
        ));
    }
    Ok(UserFnDefinition {
        name,
        description,
        args,
        body: body_start.to_string(),
    })
}

fn strip_quotes(value: &str) -> &str {
    let v = value.trim();
    if (v.starts_with('"') && v.ends_with('"') && v.len() >= 2)
        || (v.starts_with('\'') && v.ends_with('\'') && v.len() >= 2)
    {
        &v[1..v.len() - 1]
    } else {
        v
    }
}

fn parse_args_list(value: &str) -> Result<Vec<String>, String> {
    let v = value.trim();
    if v.is_empty() {
        return Ok(Vec::new());
    }
    let inner = v
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .ok_or_else(|| {
            format!("frontmatter 'args' must be a bracketed list e.g. [foo, bar]; got {v:?}")
        })?;
    Ok(inner
        .split(',')
        .map(|s| strip_quotes(s.trim()).to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

pub fn substitute_args(
    body: &str,
    args: &serde_json::Map<String, serde_json::Value>,
) -> Result<String, String> {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        rest = &rest[open + 2..];
        let close = rest.find("}}").ok_or_else(|| {
            "unterminated '{{' substitution slot in body (missing closing '}}')".to_string()
        })?;
        let key = rest[..close].trim().to_string();
        rest = &rest[close + 2..];
        let value = args.get(&key).ok_or(key.clone())?;
        let rendered = match value {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        out.push_str(&rendered);
    }
    out.push_str(rest);
    Ok(out)
}

pub(in crate::bridge) async fn handle_user_fn(
    args: &serde_json::Map<String, serde_json::Value>,
    role: &str,
) -> Result<String, String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("missing 'name' argument (string expected)")?;
    let user_args = args
        .get("args")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let path = definition_path(name)?;
    let file_text = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return serde_json::to_string_pretty(&serde_json::json!({
                "status": "not_defined",
                "message": format!(
                    "user_fn '{name}' not defined for role {role}; create at bpmux-{role}/.bpmux/functions/{name}.md"
                )
            }))
            .map_err(|e| format!("serialize: {e}"));
        }
        Err(e) => return Err(format!("read {}: {e}", path.display())),
    };
    let def = parse_definition(&file_text).map_err(|e| format!("parse {}: {e}", path.display()))?;
    let body_out = match substitute_args(&def.body, &user_args) {
        Ok(s) => s,
        Err(missing_key) => {
            return serde_json::to_string_pretty(&serde_json::json!({
                "status": "missing_arg",
                "message": format!(
                    "user_fn '{name}' references arg '{missing_key}' but caller did not provide it"
                )
            }))
            .map_err(|e| format!("serialize: {e}"));
        }
    };
    serde_json::to_string_pretty(&serde_json::json!({
        "status": "ok",
        "name": def.name,
        "description": def.description,
        "body": body_out,
    }))
    .map_err(|e| format!("serialize: {e}"))
}

pub(in crate::bridge) async fn handle_list_user_fns(
    _args: &serde_json::Map<String, serde_json::Value>,
    _role: &str,
) -> Result<String, String> {
    let dir = functions_dir()?;
    if !dir.exists() {
        return serde_json::to_string_pretty(&serde_json::json!({ "functions": [] }))
            .map_err(|e| format!("serialize: {e}"));
    }
    let mut entries: Vec<serde_json::Value> = Vec::new();
    let read_dir =
        std::fs::read_dir(&dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
    for raw in read_dir {
        let raw = raw.map_err(|e| format!("read_dir entry: {e}"))?;
        let path: PathBuf = raw.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let file_text =
            std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        match parse_definition(&file_text) {
            Ok(def) => {
                let mut entry = serde_json::Map::new();
                entry.insert("name".into(), serde_json::Value::String(def.name));
                entry.insert(
                    "description".into(),
                    serde_json::Value::String(def.description),
                );
                if !def.args.is_empty() {
                    entry.insert(
                        "args".into(),
                        serde_json::Value::Array(
                            def.args
                                .into_iter()
                                .map(serde_json::Value::String)
                                .collect(),
                        ),
                    );
                }
                entries.push(serde_json::Value::Object(entry));
            }
            Err(_) => continue,
        }
    }
    entries.sort_by(|a, b| {
        let an = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let bn = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        an.cmp(bn)
    });
    serde_json::to_string_pretty(&serde_json::json!({ "functions": entries }))
        .map_err(|e| format!("serialize: {e}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static CWD_LOCK: Mutex<()> = Mutex::new(());

    struct CwdGuard {
        saved: PathBuf,
        _guard: std::sync::MutexGuard<'static, ()>,
    }

    impl CwdGuard {
        fn enter(target: &std::path::Path) -> Self {
            let guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let saved = std::env::current_dir().unwrap();
            std::env::set_current_dir(target).unwrap();
            Self {
                saved,
                _guard: guard,
            }
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.saved);
        }
    }

    fn workspace_functions_dir_for_test(cwd: &std::path::Path) -> PathBuf {
        cwd.join(".bpmux").join("functions")
    }

    #[test]
    fn parses_minimal_frontmatter_and_body() {
        let text = "---\nname: walk_state_summary\ndescription: Walk the team state.\ntype: user_fn\n---\nBody here with {{topic}}.\n";
        let def = parse_definition(text).unwrap();
        assert_eq!(def.name, "walk_state_summary");
        assert_eq!(def.description, "Walk the team state.");
        assert!(def.args.is_empty());
        assert_eq!(def.body, "Body here with {{topic}}.\n");
    }

    #[test]
    fn parses_args_list() {
        let text = "---\nname: greet\ndescription: Greet someone\ntype: user_fn\nargs: [name, mood]\n---\nHi {{name}}, you seem {{mood}}.\n";
        let def = parse_definition(text).unwrap();
        assert_eq!(def.args, vec!["name".to_string(), "mood".to_string()]);
    }

    #[test]
    fn rejects_missing_name() {
        let text = "---\ndescription: x\ntype: user_fn\n---\nbody";
        assert!(parse_definition(text).unwrap_err().contains("name"));
    }

    #[test]
    fn rejects_wrong_type() {
        let text = "---\nname: x\ndescription: x\ntype: lesson\n---\nbody";
        assert!(parse_definition(text).unwrap_err().contains("user_fn"));
    }

    #[test]
    fn rejects_missing_frontmatter() {
        let text = "no frontmatter here";
        assert!(parse_definition(text).unwrap_err().contains("fence"));
    }

    #[test]
    fn substitutes_known_args() {
        let body = "Hello {{name}}, ratio is {{ratio}}.";
        let mut args = serde_json::Map::new();
        args.insert("name".into(), serde_json::json!("dev"));
        args.insert("ratio".into(), serde_json::json!(4.5));
        let out = substitute_args(body, &args).unwrap();
        assert_eq!(out, "Hello dev, ratio is 4.5.");
    }

    #[test]
    fn substitution_returns_err_with_missing_key() {
        let body = "Hello {{name}}, you forgot {{topic}}.";
        let mut args = serde_json::Map::new();
        args.insert("name".into(), serde_json::json!("dev"));
        let err = substitute_args(body, &args).unwrap_err();
        assert_eq!(err, "topic");
    }

    #[test]
    fn substitution_handles_repeated_slots() {
        let body = "{{x}} and {{x}} again";
        let mut args = serde_json::Map::new();
        args.insert("x".into(), serde_json::json!("ok"));
        assert_eq!(substitute_args(body, &args).unwrap(), "ok and ok again");
    }

    #[test]
    fn substitution_no_slots_passes_through() {
        let body = "no slots here";
        let args = serde_json::Map::new();
        assert_eq!(substitute_args(body, &args).unwrap(), body);
    }

    #[test]
    fn rejects_path_traversal_in_name() {
        assert!(definition_path("../escape").is_err());
        assert!(definition_path("nested/sub").is_err());
        assert!(definition_path("").is_err());
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

    #[test]
    fn workspace_functions_dir_for_test_helper_returns_relative_path() {
        let p = std::path::Path::new("/foo/bpmux-dev");
        assert_eq!(
            workspace_functions_dir_for_test(p),
            std::path::Path::new("/foo/bpmux-dev/.bpmux/functions")
        );
    }
}
