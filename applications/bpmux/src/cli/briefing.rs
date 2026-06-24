pub(crate) fn run_briefing_save(content: &str) -> anyhow::Result<()> {
    let role = crate::resolve_role(&None);
    crate::briefing::save_briefing(&role, content)
        .map_err(|e| anyhow::anyhow!("save briefing for {role}: {e}"))?;
    println!("briefing saved for {role} ({} bytes)", content.len());
    Ok(())
}

pub(crate) fn run_briefing_get(role: &Option<String>) -> anyhow::Result<()> {
    let target = role.clone().unwrap_or_else(|| crate::resolve_role(&None));
    let briefing = crate::briefing::load_briefing(&target);
    println!("{briefing}");
    Ok(())
}
