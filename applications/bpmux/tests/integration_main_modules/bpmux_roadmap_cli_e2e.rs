use crate::harness::pty::TestBpmux;

async fn run_roadmap(port: u16, args: &[&str]) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    tokio::process::Command::new(bin)
        .arg("roadmap")
        .args(args)
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "dev")
        .output()
        .await
        .expect("run bpmux roadmap")
}

#[tokio::test]
async fn roadmap_prints_the_substrate_view_not_the_retired_markdown() {
    let bpmux = TestBpmux::start();
    let out = run_roadmap(bpmux.port(), &[]).await;
    assert!(
        out.status.success(),
        "bpmux roadmap exits 0 against the live server (CLI -> /api/roadmap -> derived view); stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.starts_with("ROADMAP"),
        "the repurposed verb prints the substrate-derived view header; stdout: {stdout}"
    );
    assert!(
        stdout.contains("(no outcomes yet)"),
        "an empty substrate prints the derived-view's OWN marker — definitively NOT ROADMAP.md (the falsifier: bpmux roadmap prints the substrate-view, not the retired markdown); stdout: {stdout}"
    );
}
