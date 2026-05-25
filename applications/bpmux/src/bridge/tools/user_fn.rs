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
#[path = "user_fn_tests.rs"]
mod tests;
