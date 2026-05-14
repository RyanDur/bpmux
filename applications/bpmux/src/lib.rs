#![deny(unsafe_code)]

pub mod board;
pub mod bridge;
pub mod ci;
#[cfg(test)]
mod ci_tests;
pub mod github;
pub mod mcp;
pub mod orchestrator;
pub mod reload;
pub mod roadmap;
pub mod server;
pub mod session;
pub mod transport;

use std::sync::Arc;

use clap::{Parser, Subcommand};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

#[derive(Parser)]
#[command(
    name = "bpmux",
    version,
    about = "P2P group chat for AI pair programming"
)]
struct Cli {
    /// Your name in the session
    #[arg(long, default_value = "dev")]
    name: String,

    /// MCP server port
    #[arg(long, default_value = "4117")]
    mcp_port: u16,

    /// Session base directory (defaults to ~/.bpmux/sessions)
    #[arg(long)]
    session_base: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run as a channel bridge for Claude Code integration
    Bridge {
        /// bpmux server port to connect to
        #[arg(long, default_value = "4117")]
        port: u16,

        /// Base identity name for this bridge (a unique suffix is appended)
        #[arg(long, default_value = "claude-code")]
        name: String,

        /// Use fake story tracker (no gh calls). Test infrastructure only.
        #[arg(long)]
        fake_tracker: bool,

        /// Path to a JSON fixture file for list_stories. Test infrastructure only.
        #[arg(long)]
        board_fixture: Option<String>,
    },
    Server,
    /// Print an agent's recent lessons to stdout for session-start auto-surface (#157)
    Recall {
        /// Role to surface lessons for (e.g. dev, pm, designer).
        /// Defaults to BPMUX_ROLE env var if unset.
        #[arg(long)]
        role: Option<String>,
    },
    /// Emit a pre-commit-gates-passed activity broadcast (#424). Invoked
    /// by the git post-commit hook with the SHA of the just-created
    /// commit. Fires only on gate-passed state because the commit
    /// wouldn't exist if pre-commit had failed.
    EmitPrecommitPassed {
        /// SHA of the commit whose pre-commit gates just passed.
        #[arg(long)]
        sha: String,
        /// Role to attribute the broadcast to. Defaults to BPMUX_ROLE
        /// env var; falls back to "dev".
        #[arg(long)]
        role: Option<String>,
    },
}

fn resolve_session_base(cli_base: &Option<String>) -> anyhow::Result<std::path::PathBuf> {
    match cli_base {
        Some(path) => Ok(std::path::PathBuf::from(path)),
        None => match std::env::var("BPMUX_SESSION_BASE") {
            Ok(path) => Ok(std::path::PathBuf::from(path)),
            Err(_) => Ok(session::default_sessions_base()?),
        },
    }
}

fn run_recall(cli: &Cli, role_arg: &Option<String>) -> anyhow::Result<()> {
    let role = role_arg
        .clone()
        .or_else(|| std::env::var("BPMUX_ROLE").ok())
        .unwrap_or_default();
    if role.is_empty() {
        eprintln!("bpmux recall: no role specified (pass --role or set BPMUX_ROLE)");
        return Ok(());
    }
    let base = resolve_session_base(&cli.session_base)?;
    let session_dir = match session::resume_or_create_session(&base) {
        Ok(dir) => dir,
        Err(_) => return Ok(()),
    };
    let lessons = session::recall::surface_for_role(&session_dir, &role);
    print!("{}", session::recall::format_for_surface(&role, &lessons));
    // Emit a user-visible activity line into the bpmux channel
    // so Ryan can observe that recall fired without asking the agent
    // to quote its private system-reminder back. Best-effort — if the
    // bpmux TUI isn't running, the POST fails silently so the hook
    // never blocks session start.
    emit_recall_activity(&role, lessons.len());
    Ok(())
}

fn resolve_bpmux_port() -> u16 {
    std::env::var("BPMUX_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4117)
}

fn emit_recall_activity(role: &str, count: usize) {
    let body = session::recall::build_recall_activity_payload(role, count);
    post_activity(&body);
}

fn resolve_role(role_arg: &Option<String>) -> String {
    role_arg
        .clone()
        .or_else(|| std::env::var("BPMUX_ROLE").ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "dev".to_string())
}

fn run_emit_precommit_passed(sha: &str, role_arg: &Option<String>) -> anyhow::Result<()> {
    let role = resolve_role(role_arg);
    let body = session::precommit::build_precommit_activity_payload(&role, sha);
    post_activity(&body);
    Ok(())
}

fn post_activity(body: &serde_json::Value) {
    let port = resolve_bpmux_port();
    let url = format!("http://127.0.0.1:{port}/api/messages");
    // Fire the POST from a one-shot tokio runtime. reqwest is async
    // and this entry point is sync; the runtime is cheap and local.
    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return;
    };
    rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(500))
            .build();
        let Ok(client) = client else { return };
        let _ = client.post(&url).json(&body).send().await;
    });
}

/// #1258 α-substrate boot migration. Idempotent: runs once at process
/// init; subsequent boots see populated `icebox_unified` and no-op.
/// Failures are logged via diag — migration is best-effort and must
/// not block server boot (priority.json is recoverable from columns
/// on next session). On successful migration, emits an `IceboxMigration`
/// state-delta so any bridges that connect during boot can refetch
/// (AC 26).
fn migrate_icebox_unified_at_boot(session: &session::Session) {
    let Ok(path) = session::priority::default_priority_path() else {
        return;
    };
    let mut index = session::priority::load_priority(&path);
    let migrated = session::priority::migrate_icebox_unified(&mut index);
    if !migrated {
        return;
    }
    if let Err(e) = session::priority::save_priority(&path, &index) {
        session::diag::log_diag(&format!(
            "[#1258] icebox_unified migration: save failed: {e}"
        ));
        return;
    }
    let entries = index.icebox_unified.len();
    session::diag::log_diag(&format!(
        "[#1258] icebox_unified migration: populated {entries} entries"
    ));
    let payload = session::message::StateDeltaPayload::IceboxMigration { entries };
    let msg = session::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::StateDelta,
        state_delta: Some(payload),
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        session::diag::log_diag(&format!(
            "[#1258] icebox_unified migration: state-delta broadcast failed: {e}"
        ));
    }
}

/// #1268 backward-resilience boot-time auto-heal. Forces a one-shot
/// `refresh_all` so the canonical workflow_state is available, then
/// evicts Story entries from `icebox_unified` whose state ≠ Icebox.
/// Pairs with #1266 forward cascade-clean at boot axis. Failures are
/// logged via diag — heal is best-effort and must not block server
/// boot. On successful eviction, emits an `IceboxMigration` state-
/// delta so renderers refetch (sister-pattern to #1258 migration).
pub async fn auto_heal_icebox_at_boot(
    session: Arc<session::Session>,
    refresh: Arc<dyn board::RefreshDriver>,
) {
    let Ok(path) = session::priority::default_priority_path() else {
        return;
    };
    auto_heal_icebox_at_path(&path, session, refresh).await;
}

pub async fn auto_heal_icebox_at_path(
    path: &std::path::Path,
    session: Arc<session::Session>,
    refresh: Arc<dyn board::RefreshDriver>,
) {
    if let Err(e) = refresh.refresh_all().await {
        session::diag::log_diag(&format!(
            "[#1268] auto-heal: refresh_all failed: {e}; skipping heal"
        ));
        return;
    }
    let mut index = session::priority::load_priority(path);
    let canonical_states: std::collections::HashMap<u64, board::domain::StoryState> = refresh
        .all_stories()
        .into_iter()
        .map(|s| (s.number, s.state))
        .collect();
    let evicted = session::priority::auto_heal_icebox_unified_via_workflow_state(
        &mut index,
        &canonical_states,
    );
    if evicted == 0 {
        return;
    }
    if let Err(e) = session::priority::save_priority(path, &index) {
        session::diag::log_diag(&format!("[#1268] auto-heal: save failed: {e}"));
        return;
    }
    session::diag::log_diag(&format!(
        "[#1268] auto-heal: evicted {evicted} stale story entries from icebox_unified"
    ));
    let payload = session::message::StateDeltaPayload::IceboxMigration {
        entries: index.icebox_unified.len(),
    };
    let msg = session::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::StateDelta,
        state_delta: Some(payload),
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        session::diag::log_diag(&format!(
            "[#1268] auto-heal: state-delta broadcast failed: {e}"
        ));
    }
}

/// #1272 backward-resilience boot-time auto-heal at ready-column-axis.
/// Sister of `auto_heal_icebox_at_boot` (#1268) at columns["ready"]-
/// axis. Forces a one-shot `refresh_all` so canonical workflow_state
/// is available, then evicts entries from `columns["ready"]` whose
/// state ≠ Ready (catches stale state from missing forward-cascade-
/// clean on accept/close + duplicates from pre-#1272 close_iteration
/// appends).
///
/// Best-effort: save-failures log via diag and don't block boot. No
/// state-delta broadcast — the legacy ready-column is a backward-
/// compat surface; renderers read the canonical state via
/// list_ready/refresh.all_stories().
pub async fn auto_heal_ready_column_at_boot(
    session: Arc<session::Session>,
    refresh: Arc<dyn board::RefreshDriver>,
) {
    let Ok(priority_path) = session::priority::default_priority_path() else {
        return;
    };
    let _ = session;
    auto_heal_ready_column_at_path(&priority_path, refresh).await;
}

pub async fn auto_heal_ready_column_at_path(
    priority_path: &std::path::Path,
    refresh: Arc<dyn board::RefreshDriver>,
) {
    if let Err(e) = refresh.refresh_all().await {
        session::diag::log_diag(&format!(
            "[#1272] auto-heal ready column: refresh_all failed: {e}; skipping heal"
        ));
        return;
    }
    let mut index = session::priority::load_priority(priority_path);
    let canonical_states: std::collections::HashMap<u64, board::domain::StoryState> = refresh
        .all_stories()
        .into_iter()
        .map(|s| (s.number, s.state))
        .collect();
    let evicted =
        session::priority::auto_heal_ready_column_via_workflow_state(&mut index, &canonical_states);
    if evicted == 0 {
        return;
    }
    if let Err(e) = session::priority::save_priority(priority_path, &index) {
        session::diag::log_diag(&format!("[#1272] auto-heal ready column: save failed: {e}"));
        return;
    }
    session::diag::log_diag(&format!(
        "[#1272] auto-heal ready column: evicted {evicted} stale/duplicate entries"
    ));
}

fn run_server_only(cli: &Cli) -> anyhow::Result<()> {
    let base = resolve_session_base(&cli.session_base)?;
    std::fs::create_dir_all(&base)?;
    let session_dir = session::resume_or_create_session(&base)?;
    let (wake_tx, _wake_rx) =
        broadcast::channel::<session::Message>(session::BROADCAST_CHANNEL_CAPACITY);
    let session = Arc::new(session::Session::new(
        session_dir,
        cli.name.clone(),
        wake_tx,
    ));
    migrate_icebox_unified_at_boot(&session);
    let port = cli.mcp_port;
    let ct = CancellationToken::new();
    let restart_signal = transport::self_update::RestartSignal::new();
    let screen_capture = Arc::new(std::sync::Mutex::new(String::new()));
    let rt = tokio::runtime::Runtime::new()?;
    let server_result = rt.block_on({
        let ct = ct.clone();
        let restart_signal = restart_signal.clone();
        async move {
            session::awareness::spawn(Arc::clone(&session), session.owner_name.clone());
            transport::self_update::spawn_binary_watcher(ct.clone(), restart_signal);
            // #1291: SIGTERM/SIGINT handler triggers ct.cancel() so the
            // server's graceful shutdown actually fires when Electron's
            // killBackend() sends SIGTERM. Without this, axum kept
            // serving WS long-polls until SIGKILL (2s after SIGTERM),
            // and Playwright observed 4-10s per-test close — over 50
            // tests/shard, cumulative cost exceeded the 30s worker-
            // teardown budget on macOS arm64 self-hosted CI. Sister to
            // server::run_server's hard-deadline shutdown path.
            spawn_shutdown_signal_listener(ct.clone());
            server::run_server(session, port, ct, screen_capture)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))
        }
    });
    if restart_signal.was_fired() {
        eprintln!(
            "[bpmux] binary changed on disk; exiting with code {} so the supervisor respawns",
            transport::self_update::RESTART_EXIT_CODE
        );
        std::process::exit(transport::self_update::RESTART_EXIT_CODE);
    }
    server_result
}

fn spawn_shutdown_signal_listener(ct: CancellationToken) {
    tokio::spawn(async move {
        let sigterm = async {
            #[cfg(unix)]
            {
                if let Ok(mut stream) =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                {
                    stream.recv().await;
                }
            }
            #[cfg(not(unix))]
            std::future::pending::<()>().await;
        };
        let sigint = tokio::signal::ctrl_c();
        tokio::select! {
            _ = sigterm => {
                crate::session::diag::log_diag("server: SIGTERM received; cancelling CT");
            }
            _ = sigint => {
                crate::session::diag::log_diag("server: SIGINT received; cancelling CT");
            }
        }
        ct.cancel();
    });
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(Command::Bridge {
        port,
        name,
        fake_tracker,
        board_fixture,
    }) = cli.command
    {
        return bridge::run_bridge(port, &name, fake_tracker, board_fixture);
    }

    if let Some(Command::Recall { role }) = &cli.command {
        return run_recall(&cli, role);
    }

    if let Some(Command::EmitPrecommitPassed { sha, role }) = &cli.command {
        return run_emit_precommit_passed(sha, role);
    }

    run_server_only(&cli)
}
