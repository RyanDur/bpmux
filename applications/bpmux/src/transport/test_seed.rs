//! Test-mode-only seed endpoints (#1290 Part A).
//!
//! Playwright e2e tests previously seeded session state by writing
//! JSONL/JSON directly to the session directory before bpmux started.
//! That seeding mechanism is bypassed when `BPMUX_STORAGE_BACKEND=memory`
//! is active (#1289) because MemoryStorage never reads those files.
//!
//! These endpoints accept seed payloads over HTTP after the backend is
//! healthy, route them through the active `Storage` trait, and preserve
//! caller-supplied timestamps exactly — which the production
//! `post_message` handler does NOT, because it overrides `ts` with
//! `chrono::Utc::now()` to prevent timestamp spoofing.
//!
//! ## Boundary discipline (#1290 AC 1, 6)
//!
//! These routes are mounted only when `BPMUX_TEST_SEED_API=1` is set in
//! the bpmux server's environment. The mount happens at router-
//! construction time in `server.rs`, so a production server (without
//! the env-var) literally does not register these paths — the test-mode
//! boundary is structural, not runtime-conditional.
//!
//! The gate is separate from `BPMUX_TEST_MODE` because that flag also
//! switches the orchestrator to an in-memory variant; seed endpoints
//! should be enableable without changing the orchestrator (specifically
//! so `launchWithMockGh` tests can seed state while keeping the prod
//! orchestrator pointed at the mock-GH boundary).

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::session;

/// Returns true when seed-API endpoints should be mounted. Read at
/// router-construction time only — runtime toggling is not supported.
pub fn seed_api_enabled() -> bool {
    std::env::var("BPMUX_TEST_SEED_API")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Seed-messages payload — array of fully-formed `Message` rows. Each
/// row's `ts` is preserved exactly (no `Utc::now()` override) so tests
/// asserting timestamp-derived state (ordering, before/after filters,
/// channel-key derivation) get deterministic input.
#[derive(Deserialize)]
pub struct SeedMessagesBody {
    pub messages: Vec<session::Message>,
}

pub async fn post_seed_messages(
    State(session): State<Arc<session::Session>>,
    Json(body): Json<SeedMessagesBody>,
) -> impl IntoResponse {
    for msg in &body.messages {
        if let Err(e) = session.storage.append_message(msg) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("seed_messages append failed: {e}"),
            );
        }
    }
    (
        StatusCode::CREATED,
        format!("seeded {} messages", body.messages.len()),
    )
}

/// Seed-pins payload — full `PinsStore` shape (channel-key → Pin
/// list). Replaces the entire pins state (matches the existing
/// `seedPins` filesystem-write semantic in launch.ts).
#[derive(Deserialize)]
pub struct SeedPinsBody {
    pub store: session::pins::PinsStore,
}

pub async fn post_seed_pins(
    State(session): State<Arc<session::Session>>,
    Json(body): Json<SeedPinsBody>,
) -> impl IntoResponse {
    if let Err(e) = session.storage.save_pins(&body.store) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("seed_pins save failed: {e}"),
        );
    }
    (StatusCode::CREATED, "seeded pins".to_string())
}

/// Seed-reactions payload — full `ReactionsStore` shape. Replaces the
/// entire reactions state. Currently unused by launch.ts but the API
/// surface is added now per #1290 AC 5 so future test fixtures can
/// seed reactions without another round-trip through this story.
#[derive(Deserialize)]
pub struct SeedReactionsBody {
    pub store: session::reactions::ReactionsStore,
}

pub async fn post_seed_reactions(
    State(session): State<Arc<session::Session>>,
    Json(body): Json<SeedReactionsBody>,
) -> impl IntoResponse {
    if let Err(e) = session.storage.save_reactions(&body.store) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("seed_reactions save failed: {e}"),
        );
    }
    (StatusCode::CREATED, "seeded reactions".to_string())
}

/// #1306 slice 4.1: clear the message store. Lets RTL+real-backend
/// tests reset state between cases that share the per-run backend
/// (vitest globalSetup) — without this, seeded messages leak across
/// tests and counts/scope-based assertions fail. Gated by
/// `BPMUX_TEST_SEED_API` at the REST boundary.
pub async fn post_reset_messages(
    State(session): State<Arc<session::Session>>,
) -> impl IntoResponse {
    if let Err(e) = session.storage.clear_messages() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("reset_messages failed: {e}"),
        );
    }
    (StatusCode::OK, "reset messages".to_string())
}

/// #1335 slice 4.1.B: seed the orchestrator's RosterStore via the
/// canonical OrchestratorAction::Roster dispatch path. Sister to the
/// production WS-hello roster-upsert flow — same action shape, same
/// broadcast semantic (mirror.roster propagates to the renderer's
/// state-snapshot + state-delta channels). Each entry is a full
/// identity string like "dev:claude-code"; the role is derived as the
/// prefix before ':'. Gated by `BPMUX_TEST_SEED_API`.
#[derive(Deserialize)]
pub struct SeedRosterBody {
    pub identities: Vec<String>,
}

pub async fn post_seed_roster(
    axum::Extension(orchestrator): axum::Extension<
        Arc<crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>>,
    >,
    Json(body): Json<SeedRosterBody>,
) -> impl IntoResponse {
    for identity in &body.identities {
        let name = identity
            .split(':')
            .next()
            .unwrap_or(identity.as_str())
            .to_string();
        let role = match name.as_str() {
            "dev" => crate::orchestrator::RosterRole::Dev,
            "pm" => crate::orchestrator::RosterRole::Pm,
            "designer" => crate::orchestrator::RosterRole::Designer,
            "ryan" => crate::orchestrator::RosterRole::Owner,
            _ => crate::orchestrator::RosterRole::Other,
        };
        orchestrator
            .dispatch(crate::orchestrator::OrchestratorAction::Roster(
                crate::orchestrator::RosterAction::RosterMemberUpserted {
                    name,
                    identity: identity.clone(),
                    role,
                    presence: crate::orchestrator::Presence::Online,
                },
            ))
            .await;
    }
    (
        StatusCode::OK,
        format!("seeded {} roster identities", body.identities.len()),
    )
}
