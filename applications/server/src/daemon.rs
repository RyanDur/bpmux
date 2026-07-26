use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

pub fn run(name: String, base: std::path::PathBuf, port: u16) -> anyhow::Result<()> {
    let workflow = workflow::load_workflow_from_process_env()
        .map_err(|e| anyhow::anyhow!("refusing to start on a broken workflow config: {e}"))?;
    workflow::install_workflow(workflow);
    std::fs::create_dir_all(&base)?;
    let data_dir = location::durable_data_dir(&base)?;
    let (wake_tx, _wake_rx) =
        broadcast::channel::<message::Message>(hub::BROADCAST_CHANNEL_CAPACITY);
    let data_gateways = storage::open(&data_dir);
    let hub = Arc::new(hub::Hub::with_conversation(
        data_dir,
        name,
        wake_tx,
        data_gateways.conversation.clone(),
    ));
    let ct = CancellationToken::new();
    let restart_signal = self_update::RestartSignal::new();
    let rt = tokio::runtime::Runtime::new()?;
    let server_result = rt.block_on({
        let ct = ct.clone();
        let restart_signal = restart_signal.clone();
        async move {
            awareness::spawn(
                Arc::new(crate::awareness_host::HubAwarenessHost {
                    hub: hub.clone(),
                    topics: data_gateways.topics.clone(),
                    lessons: data_gateways.lessons.clone(),
                }),
                hub.owner_name.clone(),
            );
            self_update::spawn_binary_watcher(ct.clone(), restart_signal);
            spawn_shutdown_signal_listener(ct.clone());
            crate::run_server(hub, data_gateways, port, ct)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))
        }
    });
    if restart_signal.was_fired() {
        eprintln!(
            "[bpmux] binary changed on disk; exiting with code {} so the supervisor respawns",
            self_update::RESTART_EXIT_CODE
        );
        std::process::exit(self_update::RESTART_EXIT_CODE);
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
                diag::log_diag("server: SIGTERM received; cancelling CT");
            }
            _ = sigint => {
                diag::log_diag("server: SIGINT received; cancelling CT");
            }
        }
        ct.cancel();
    });
}
