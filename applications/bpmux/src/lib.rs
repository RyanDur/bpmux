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
    #[arg(long, default_value_t = transport::local_url::DEFAULT_API_PORT)]
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
        #[arg(long, default_value_t = transport::local_url::DEFAULT_API_PORT)]
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
        .unwrap_or(transport::local_url::DEFAULT_API_PORT)
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
    let url = transport::local_url::api_url(port, "/api/messages");
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

/// #1301 slice 4D — one-shot boot migration: translate pre-existing
/// `priority.json` `columns["ready"]` entries into canonical-via-GH
/// `ready-order.{position}` labels via the canonical mutation chain.
///
/// Idempotent: if any Ready story already carries a `ready-order.*`
/// label, the migration treats the canonical substrate as already in
/// place and is a no-op (matches #1301 Scenario 6).
///
/// Positions: emitted as `(idx + 1) * 100` so the resulting label
/// space matches the sparse-numeric algebra used by drag-reorder
/// (default_first_position=100, default_step=100 — see
/// [`board::ready_order_label`]).
///
/// Returns the count of stories migrated (0 on no-op).
pub async fn migrate_legacy_ready_to_labels(
    refresh: Arc<dyn board::RefreshDriver>,
) -> Result<usize, String> {
    let Ok(priority_path) = session::priority::default_priority_path() else {
        return Ok(0);
    };
    migrate_legacy_ready_to_labels_at_path(&priority_path, refresh).await
}

pub async fn migrate_legacy_ready_to_labels_at_path(
    priority_path: &std::path::Path,
    refresh: Arc<dyn board::RefreshDriver>,
) -> Result<usize, String> {
    let any_labeled = refresh.all_stories().iter().any(|s| {
        s.state == board::domain::StoryState::Ready
            && s.raw_labels
                .iter()
                .any(|l| board::ready_order_label::is_ready_order_label(l))
    });
    if any_labeled {
        return Ok(0);
    }
    let legacy_ready = match std::fs::read_to_string(priority_path) {
        Ok(contents) => serde_json::from_str::<serde_json::Value>(&contents)
            .ok()
            .and_then(|v| v.get("ready").cloned())
            .and_then(|v| v.as_array().cloned())
            .map(|arr| arr.iter().filter_map(|n| n.as_u64()).collect::<Vec<u64>>())
            .unwrap_or_default(),
        Err(_) => return Ok(0),
    };
    if legacy_ready.is_empty() {
        return Ok(0);
    }
    let canonical_ready: std::collections::HashSet<u64> = refresh
        .all_stories()
        .iter()
        .filter(|s| s.state == board::domain::StoryState::Ready && !s.is_closed)
        .map(|s| s.number)
        .collect();
    let mut migrated = 0usize;
    for (idx, number) in legacy_ready.into_iter().enumerate() {
        if !canonical_ready.contains(&number) {
            continue;
        }
        let position = ((idx + 1) as u32) * 100;
        let label = format!("ready-order.{position}");
        let meta = board::events::EventMeta {
            actor: "system".to_string(),
            ts: chrono::Utc::now(),
            peer_id: board::PeerId::local(),
        };
        refresh
            .mutate_sync(board::BoardEvent::TagAdded {
                meta,
                number,
                label,
            })
            .map_err(|e| format!("migrate ready-order #{number}: {e}"))?;
        migrated += 1;
    }
    Ok(migrated)
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod migration_legacy_ready_to_labels_tests {
    use super::*;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use crate::board::orchestrator::SyncOrchestrator;
    use crate::board::store::BoardStore;
    use chrono::{TimeZone, Utc};

    fn seed_ready(number: u64, extra_labels: Vec<String>) -> Story {
        let mut s = Story::new_minimal(
            number,
            format!("ready-{number}"),
            StoryState::Ready,
            Utc.with_ymd_and_hms(2026, 5, 16, 0, 0, 0).unwrap(),
            "pm".to_string(),
        );
        s.raw_labels = std::iter::once("ready".to_string())
            .chain(extra_labels)
            .collect();
        s
    }

    async fn build_refresh(stories: Vec<Story>) -> Arc<dyn board::RefreshDriver> {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(stories);
        let orch = SyncOrchestrator::new(store, adapter);
        let orch: Arc<dyn board::RefreshDriver> = Arc::new(orch);
        orch.refresh_all().await.unwrap();
        orch
    }

    fn seed_priority(path: &std::path::Path, ready: &[u64]) {
        let body = serde_json::json!({ "ready": ready });
        std::fs::write(path, serde_json::to_string(&body).unwrap()).unwrap();
    }

    #[tokio::test]
    async fn empty_legacy_ready_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("priority.json");
        seed_priority(&path, &[]);
        let refresh = build_refresh(vec![]).await;
        let migrated = migrate_legacy_ready_to_labels_at_path(&path, refresh)
            .await
            .unwrap();
        assert_eq!(migrated, 0);
    }

    #[tokio::test]
    async fn absent_priority_file_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("priority.json");
        let refresh = build_refresh(vec![seed_ready(100, vec![])]).await;
        let migrated = migrate_legacy_ready_to_labels_at_path(&path, refresh)
            .await
            .unwrap();
        assert_eq!(migrated, 0);
    }

    #[tokio::test]
    async fn three_legacy_stories_get_labels_at_100_200_300() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("priority.json");
        seed_priority(&path, &[100, 200, 300]);
        let stories = vec![
            seed_ready(100, vec![]),
            seed_ready(200, vec![]),
            seed_ready(300, vec![]),
        ];
        let refresh = build_refresh(stories).await;
        let migrated = migrate_legacy_ready_to_labels_at_path(&path, refresh.clone())
            .await
            .unwrap();
        assert_eq!(migrated, 3);
        let s100 = refresh.story(100).unwrap();
        assert!(s100.raw_labels.iter().any(|l| l == "ready-order.100"));
        let s200 = refresh.story(200).unwrap();
        assert!(s200.raw_labels.iter().any(|l| l == "ready-order.200"));
        let s300 = refresh.story(300).unwrap();
        assert!(s300.raw_labels.iter().any(|l| l == "ready-order.300"));
    }

    #[tokio::test]
    async fn idempotent_when_any_story_already_labeled() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("priority.json");
        seed_priority(&path, &[100, 200, 300]);
        let stories = vec![
            seed_ready(100, vec!["ready-order.50".to_string()]),
            seed_ready(200, vec![]),
            seed_ready(300, vec![]),
        ];
        let refresh = build_refresh(stories).await;
        let migrated = migrate_legacy_ready_to_labels_at_path(&path, refresh.clone())
            .await
            .unwrap();
        assert_eq!(migrated, 0, "any pre-existing label → migration is no-op");
        let s200 = refresh.story(200).unwrap();
        assert!(
            !s200
                .raw_labels
                .iter()
                .any(|l| l.starts_with("ready-order.")),
            "untouched story stays unlabeled"
        );
    }

    #[tokio::test]
    async fn migration_second_run_is_noop_after_first_run_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("priority.json");
        seed_priority(&path, &[100, 200]);
        let refresh = build_refresh(vec![seed_ready(100, vec![]), seed_ready(200, vec![])]).await;
        let first = migrate_legacy_ready_to_labels_at_path(&path, refresh.clone())
            .await
            .unwrap();
        let second = migrate_legacy_ready_to_labels_at_path(&path, refresh.clone())
            .await
            .unwrap();
        assert_eq!(first, 2, "first run migrates 2 stories");
        assert_eq!(
            second, 0,
            "second run is no-op (#1301 Scenario 6 idempotency)"
        );
    }

    #[tokio::test]
    async fn skips_legacy_entries_not_in_store_ready_state() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("priority.json");
        seed_priority(&path, &[100, 200, 999]);
        let refresh = build_refresh(vec![seed_ready(100, vec![]), seed_ready(200, vec![])]).await;
        let migrated = migrate_legacy_ready_to_labels_at_path(&path, refresh.clone())
            .await
            .unwrap();
        assert_eq!(
            migrated, 2,
            "story #999 in legacy but absent from store is skipped"
        );
    }
}
