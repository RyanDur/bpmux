#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserFnDefinition {
    pub name: String,
    pub description: String,
    pub args: Vec<String>,
    pub body: String,
}

const FRONTMATTER_FENCE: &str = "---";

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
