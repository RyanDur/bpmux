use super::http::{authed_get, authed_post_empty};

pub fn run_ci_state() -> anyhow::Result<()> {
    let (status, body) = authed_get("/api/ci")?;
    if !status.is_success() {
        anyhow::bail!("ci state failed ({status}): {body}");
    }
    println!("{body}");
    Ok(())
}

pub fn run_ci_log(run: &str) -> anyhow::Result<()> {
    let (status, body) = authed_get(&format!("/api/ci/log/{run}"))?;
    if !status.is_success() {
        anyhow::bail!("ci log failed ({status}): {body}");
    }
    println!("{body}");
    Ok(())
}

pub fn run_ci_jobs(run: &str) -> anyhow::Result<()> {
    let (status, body) = authed_get(&format!("/api/ci/jobs/{run}"))?;
    if !status.is_success() {
        anyhow::bail!("ci jobs failed ({status}): {body}");
    }
    println!("{body}");
    Ok(())
}

pub fn run_ci_rerun(run: &str) -> anyhow::Result<()> {
    let (status, body) = authed_post_empty(&format!("/api/ci/rerun/{run}"))?;
    if !status.is_success() {
        anyhow::bail!("ci rerun failed ({status}): {body}");
    }
    println!("{body}");
    Ok(())
}
