use std::sync::Arc;

#[cfg(test)]
pub(crate) async fn run_ingest_poll_loop(
    refresh: Arc<dyn crate::sync::RefreshDriver>,
    interval: std::time::Duration,
    observer: Option<crate::github::RateLimitObserver>,
) {
    run_ingest_poll_loop_cancellable(
        refresh,
        interval,
        observer,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
}

pub async fn run_ingest_poll_loop_cancellable(
    refresh: Arc<dyn crate::sync::RefreshDriver>,
    interval: std::time::Duration,
    observer: Option<crate::github::RateLimitObserver>,
    ct: tokio_util::sync::CancellationToken,
) {
    let start = tokio::time::Instant::now() + interval;
    let mut ticker = tokio::time::interval_at(start, interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let base_backoff = std::time::Duration::from_secs(60);
    let max_backoff = std::time::Duration::from_secs(3600);
    let mut backoff = base_backoff;
    let mut last_poll_since: Option<chrono::DateTime<chrono::Utc>> = None;
    loop {
        tokio::select! {
            _ = ct.cancelled() => break,
            _ = ticker.tick() => {}
        }
        let poll_started_at = chrono::Utc::now();
        match refresh.refresh_all_since(last_poll_since).await {
            Ok(()) => {
                backoff = base_backoff;
                last_poll_since = Some(poll_started_at);
            }
            Err(e) => {
                let msg = e.to_string();
                let snapshot = observer.as_ref().map(|o| o.snapshot()).unwrap_or_default();
                if let Some(decision) = crate::github::classify_and_decide(
                    &msg,
                    snapshot,
                    backoff,
                    std::time::SystemTime::now(),
                ) {
                    crate::diag::log_diag(&format!(
                        "bridge: ingest rate-limited; backoff signal={} wait={}s",
                        crate::github::signal_label(decision.signal),
                        decision.wait.as_secs()
                    ));
                    tokio::time::sleep(decision.wait).await;
                    backoff = (backoff * 2).min(max_backoff);
                } else {
                    crate::diag::log_diag(&format!("bridge: background ingest poll failed: {e}"));
                }
            }
        }
    }
}

pub(crate) fn poll_interval_from_env() -> std::time::Duration {
    let secs = std::env::var("BPMUX_POLL_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(60);
    std::time::Duration::from_secs(secs.max(1))
}
