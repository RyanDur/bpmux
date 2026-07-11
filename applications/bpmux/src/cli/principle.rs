use memory::identity::{
    add_principle, add_value, default_identity_dir, load_identity, remove_principle, save_identity,
    update_principle,
};

fn role_and_base() -> anyhow::Result<(String, std::path::PathBuf)> {
    let role = crate::resolve_role(&None);
    let base = default_identity_dir()?;
    Ok((role, base))
}

pub(crate) fn run_value_add(name: &str, description: &str) -> anyhow::Result<()> {
    let (role, base) = role_and_base()?;
    let mut index = load_identity(&base, &role);
    add_value(&mut index, name, description).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    save_identity(&base, &role, &index)?;
    println!("value '{name}' added");
    Ok(())
}

pub(crate) fn run_principle_add(value: &str, principle: &str) -> anyhow::Result<()> {
    let (role, base) = role_and_base()?;
    let mut index = load_identity(&base, &role);
    add_principle(&mut index, value, principle).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    save_identity(&base, &role, &index)?;
    println!("principle added under value '{value}'");
    Ok(())
}

pub(crate) fn run_principle_update(value: &str, old: &str, new: &str) -> anyhow::Result<()> {
    let (role, base) = role_and_base()?;
    let mut index = load_identity(&base, &role);
    update_principle(&mut index, value, old, new).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    save_identity(&base, &role, &index)?;
    println!("principle updated under value '{value}'");
    Ok(())
}

pub(crate) fn run_principle_remove(value: &str, principle: &str) -> anyhow::Result<()> {
    let (role, base) = role_and_base()?;
    let mut index = load_identity(&base, &role);
    remove_principle(&mut index, value, principle).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    save_identity(&base, &role, &index)?;
    println!("principle removed under value '{value}'");
    Ok(())
}

pub(crate) fn run_identity_get() -> anyhow::Result<()> {
    let (role, base) = role_and_base()?;
    let index = load_identity(&base, &role);
    println!("{}", serde_json::to_string_pretty(&index)?);
    Ok(())
}
