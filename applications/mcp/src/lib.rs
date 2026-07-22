pub fn run(
    port: u16,
    name: &str,
    fake_tracker: bool,
    board_fixture: Option<String>,
) -> anyhow::Result<()> {
    let user_id = std::env::current_dir()
        .ok()
        .and_then(|cwd| location::resolve_registered_user_id(&cwd));
    bridge::run_bridge(port, name, user_id, fake_tracker, board_fixture)
}
