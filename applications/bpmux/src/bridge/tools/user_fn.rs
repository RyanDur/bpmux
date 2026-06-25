use std::path::PathBuf;

use super::user_fn_parse::{parse_definition, substitute_args};

fn functions_dir() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("resolve cwd: {e}"))?;
    Ok(cwd.join(".bpmux").join("functions"))
}

pub(super) fn definition_path(name: &str) -> Result<PathBuf, String> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(format!(
            "user_fn name '{name}' must be a single path segment (no '/', '\\\\', or '..')"
        ));
    }
    Ok(functions_dir()?.join(format!("{name}.md")))
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
