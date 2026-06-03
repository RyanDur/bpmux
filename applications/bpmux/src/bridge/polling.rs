use std::sync::Arc;

#[cfg(test)]
pub(crate) async fn run_ingest_poll_loop(
    refresh: Arc<dyn crate::board::RefreshDriver>,
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
    refresh: Arc<dyn crate::board::RefreshDriver>,
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
                    crate::session::diag::log_diag(&format!(
                        "bridge: ingest rate-limited; backoff signal={} wait={}s",
                        crate::github::signal_label(decision.signal),
                        decision.wait.as_secs()
                    ));
                    tokio::time::sleep(decision.wait).await;
                    backoff = (backoff * 2).min(max_backoff);
                } else {
                    crate::session::diag::log_diag(&format!(
                        "bridge: background ingest poll failed: {e}"
                    ));
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

const HYDRATION_MAX_PER_TICK: usize = 3;

pub(crate) fn select_stories_to_hydrate(stories: &[crate::board::Story], limit: usize) -> Vec<u64> {
    use crate::board::domain::StoryState;
    stories
        .iter()
        .filter(|s| {
            matches!(
                s.state,
                StoryState::Ready
                    | StoryState::InProgress
                    | StoryState::DeskCheck
                    | StoryState::Delivered
            ) && s.body.is_none()
        })
        .take(limit)
        .map(|s| s.number)
        .collect()
}

pub(crate) fn select_stories_to_revalidate(
    stories: &[crate::board::Story],
    limit: usize,
) -> Vec<(u64, Option<String>)> {
    use crate::board::domain::StoryState;
    stories
        .iter()
        .filter(|s| {
            matches!(
                s.state,
                StoryState::Ready
                    | StoryState::InProgress
                    | StoryState::DeskCheck
                    | StoryState::Delivered
            ) && s.body.is_some()
        })
        .take(limit)
        .map(|s| (s.number, s.body_etag.clone()))
        .collect()
}

pub async fn run_hydration_loop_cancellable(
    refresh: Arc<dyn crate::board::RefreshDriver>,
    interval: std::time::Duration,
    observer: crate::github::RateLimitObserver,
    ct: tokio_util::sync::CancellationToken,
) {
    let start = tokio::time::Instant::now() + interval;
    let mut ticker = tokio::time::interval_at(start, interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = ct.cancelled() => break,
            _ = ticker.tick() => {}
        }
        let snapshot = observer.snapshot();
        let decision = crate::github::decide_backoff(
            snapshot,
            std::time::Duration::ZERO,
            std::time::SystemTime::now(),
        );
        if decision.wait > std::time::Duration::ZERO {
            tokio::select! {
                _ = ct.cancelled() => break,
                _ = tokio::time::sleep(decision.wait) => {}
            }
            continue;
        }
        let stories = refresh.all_stories();
        let to_hydrate = select_stories_to_hydrate(&stories, HYDRATION_MAX_PER_TICK);
        let to_revalidate = select_stories_to_revalidate(&stories, HYDRATION_MAX_PER_TICK);
        if to_hydrate.is_empty() && to_revalidate.is_empty() {
            crate::session::diag::log_diag(
                "bridge: hydration tick: nothing to hydrate or revalidate",
            );
            continue;
        }
        crate::session::diag::log_diag(&format!(
            "bridge: hydration tick: hydrate={:?} revalidate={:?}",
            to_hydrate,
            to_revalidate.iter().map(|(n, _)| *n).collect::<Vec<_>>()
        ));
        let mut hydrated_ok = 0u32;
        let mut revalidated_ok = 0u32;
        let to_hydrate_len = to_hydrate.len();
        for (idx, (number, prior_etag)) in to_hydrate
            .into_iter()
            .map(|n| (n, None::<String>))
            .chain(to_revalidate)
            .enumerate()
        {
            match refresh.fetch_body_conditional(number, prior_etag).await {
                Ok(_) => {
                    if idx < to_hydrate_len {
                        hydrated_ok += 1;
                    } else {
                        revalidated_ok += 1;
                    }
                }
                Err(e) => {
                    crate::session::diag::log_diag(&format!(
                        "bridge: hydrate body #{number} failed: {e}"
                    ));
                    break;
                }
            }
        }
        crate::session::diag::log_diag(&format!(
            "bridge: hydration tick complete: hydrated={hydrated_ok} revalidated={revalidated_ok}"
        ));
    }
}

pub(crate) fn hydration_interval_from_env() -> std::time::Duration {
    let secs = std::env::var("BPMUX_HYDRATION_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(15);
    std::time::Duration::from_secs(secs.max(1))
}

pub(crate) async fn run_drain_loop(
    refresh: Arc<dyn crate::board::RefreshDriver>,
    session: Arc<crate::session::Session>,
    interval: std::time::Duration,
) {
    let start = tokio::time::Instant::now() + interval;
    let mut ticker = tokio::time::interval_at(start, interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        let state_changed = match refresh.drain_pending().await {
            Ok(0) => false,
            Ok(n) => {
                crate::session::diag::log_diag(&format!(
                    "bridge: background drain pushed {n} event(s)"
                ));
                true
            }
            Err(e) => {
                crate::session::diag::log_diag(&format!("bridge: background drain: {e}"));
                true
            }
        };
        if state_changed {
            broadcast_sync_status_update(&session, &refresh.snapshot_status());
        }
    }
}

pub(crate) fn broadcast_sync_status_update(
    session: &crate::session::Session,
    snapshot: &crate::board::SyncStatus,
) {
    let msg = crate::session::Message {
        from: "bpmux".to_string(),
        text: super::SYNC_STATUS_UPDATE_MARKER.to_string(),
        ts: chrono::Utc::now(),
        message_type: crate::session::MessageType::System,
        ..crate::session::Message::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        crate::session::diag::log_diag(&format!(
            "run_drain_loop: sync-status marker broadcast failed: {e}"
        ));
    }
    if let Some(transition) = crate::board::sync_transition(snapshot) {
        let delta = crate::session::Message {
            from: "bpmux".to_string(),
            text: String::new(),
            ts: chrono::Utc::now(),
            message_type: crate::session::MessageType::StateDelta,
            state_delta: Some(crate::session::message::StateDeltaPayload::Sync {
                transition: crate::session::message::SyncTransitionWire {
                    status: transition.status.to_string(),
                    why: transition.why,
                    pending_events: transition.pending_events,
                    conflicts_pending: transition.conflicts_pending,
                },
            }),
            ..crate::session::Message::default()
        };
        if let Err(e) = session.broadcast(&delta) {
            crate::session::diag::log_diag(&format!(
                "run_drain_loop: sync-status state-delta broadcast failed: {e}"
            ));
        }
    }
}

pub(crate) fn drain_interval_from_env() -> std::time::Duration {
    let secs = std::env::var("BPMUX_DRAIN_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(5);
    std::time::Duration::from_secs(secs.max(1))
}
