use std::sync::Arc;

const HYDRATION_MAX_PER_TICK: usize = 3;

pub(crate) fn select_stories_to_hydrate(
    stories: &[product_management::story::Story],
    limit: usize,
    iterations: &[product_management::iteration::Iteration],
) -> Vec<u64> {
    use product_management::scheduling::ProjectedState;
    stories
        .iter()
        .filter(|s| {
            matches!(
                product_management::scheduling::projected_state(s, iterations),
                ProjectedState::Ready | ProjectedState::InProgress | ProjectedState::Acceptance
            ) && s.body.is_none()
        })
        .take(limit)
        .map(|s| u64::from(s.id))
        .collect()
}

pub(crate) fn select_stories_to_revalidate(
    stories: &[product_management::story::Story],
    limit: usize,
    iterations: &[product_management::iteration::Iteration],
) -> Vec<(u64, Option<String>)> {
    use product_management::scheduling::ProjectedState;
    stories
        .iter()
        .filter(|s| {
            matches!(
                product_management::scheduling::projected_state(s, iterations),
                ProjectedState::Ready | ProjectedState::InProgress | ProjectedState::Acceptance
            ) && s.body.is_some()
        })
        .take(limit)
        .map(|s| (u64::from(s.id), s.body_etag.clone()))
        .collect()
}

pub async fn run_hydration_loop_cancellable(
    refresh: Arc<dyn crate::sync::RefreshDriver>,
    interval: std::time::Duration,
    observer: crate::rate_limit::RateLimitObserver,
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
        let decision = crate::rate_limit::decide_backoff(
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
        let iterations = refresh.iterations();
        let to_hydrate = select_stories_to_hydrate(&stories, HYDRATION_MAX_PER_TICK, &iterations);
        let to_revalidate =
            select_stories_to_revalidate(&stories, HYDRATION_MAX_PER_TICK, &iterations);
        if to_hydrate.is_empty() && to_revalidate.is_empty() {
            diag::log_diag("bridge: hydration tick: nothing to hydrate or revalidate");
            continue;
        }
        diag::log_diag(&format!(
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
                    diag::log_diag(&format!("bridge: hydrate body #{number} failed: {e}"));
                    break;
                }
            }
        }
        diag::log_diag(&format!(
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
