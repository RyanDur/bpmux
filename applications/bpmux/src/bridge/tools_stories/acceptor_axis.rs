use crate::product_management::story::Acceptor;

pub(crate) fn derive_acceptor(user_job: &str) -> Option<Acceptor> {
    let trimmed = user_job.trim_start();
    if !trimmed
        .as_bytes()
        .get(..3)
        .is_some_and(|p| p.eq_ignore_ascii_case(b"as "))
    {
        return None;
    }
    let use_axis = trimmed.split(',').next().unwrap_or(trimmed);
    let words: Vec<&str> = use_axis
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let has = |needle: &str| words.iter().any(|w| w.eq_ignore_ascii_case(needle));

    if has("owner") || has("ryan") {
        return Some(Acceptor::Owner);
    }
    let team_axis =
        has("agent") || has("agents") || has("team") || has("everyone") || has("anyone");
    let dev_axis = has("dev") || has("developer") || has("developers");
    let pm_axis = has("pm");
    if team_axis || (dev_axis && pm_axis) {
        return Some(Acceptor::Team);
    }
    if dev_axis {
        return Some(Acceptor::Dev);
    }
    if pm_axis {
        return Some(Acceptor::Team);
    }
    None
}
