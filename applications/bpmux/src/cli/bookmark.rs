use crate::bookmark::{Bookmark, load_bookmark, save_bookmark};

fn current_data_dir() -> anyhow::Result<std::path::PathBuf> {
    let base = crate::location::resolve_default_base()?;
    Ok(crate::location::resume_or_create_data_dir(&base)?)
}

pub(crate) fn run_bookmark_save(
    current_task: &str,
    next_step: &Option<String>,
    notes: &Option<String>,
    decisions: &[String],
    open_questions: &[String],
) -> anyhow::Result<()> {
    let role = crate::resolve_role(&None);
    let data_dir = current_data_dir()?;
    let bookmark = Bookmark {
        current_task: current_task.to_string(),
        decisions: decisions.to_vec(),
        open_questions: open_questions.to_vec(),
        next_step: next_step.clone().unwrap_or_default(),
        notes: notes.clone().unwrap_or_default(),
        last_updated: String::new(),
    };
    save_bookmark(&data_dir, &role, &bookmark)?;
    println!("bookmark saved for {role}");
    Ok(())
}

pub(crate) fn run_bookmark_get(role: &Option<String>) -> anyhow::Result<()> {
    let target = role.clone().unwrap_or_else(|| crate::resolve_role(&None));
    let data_dir = current_data_dir()?;
    let bookmark = load_bookmark(&data_dir, &target);
    println!("{}", serde_json::to_string_pretty(&bookmark)?);
    Ok(())
}
