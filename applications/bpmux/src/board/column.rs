//! Single-source-of-truth column projection for the board (#520).
//!
//! Two observation surfaces — the Electron renderer's `columnFor`
//! (TypeScript) and the bridge `view_board` MCP tool's text-output
//! (Rust) — used to derive their column-bucket from the (state,
//! is_closed) tuple via separate implementations. #522 brought the
//! Rust side into output-parity with the renderer; this module makes
//! the projection itself shared: the Rust function lives here, and
//! the parity-with-the-renderer is pinned by an end-to-end test
//! (`tests/board_column_parity_e2e_test.rs`) that drives the
//! renderer's columnFor truth-table against this function and
//! asserts identical buckets. Future bucketing changes happen in one
//! place; the test catches any TS-vs-Rust drift.
//!
//! Renderer mirror: applications/electron/src/renderer/src/board/types.ts
//!   `columnFor(state, isClosed)`.

use super::domain::StoryState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnBucket {
    Icebox,
    Ready,
    InProgress,
    DeskCheck,
    Delivered,
    Accepted,
}

impl ColumnBucket {
    pub fn header(self) -> &'static str {
        match self {
            ColumnBucket::Icebox => "ICEBOX",
            ColumnBucket::Ready => "READY",
            ColumnBucket::InProgress => "IN PROGRESS",
            ColumnBucket::DeskCheck => "DESK CHECK",
            ColumnBucket::Delivered => "DELIVERED",
            ColumnBucket::Accepted => "ACCEPTED",
        }
    }
}

pub fn column_for(state: StoryState, is_closed: bool) -> ColumnBucket {
    if is_closed {
        return match state {
            StoryState::Icebox => ColumnBucket::Icebox,
            _ => ColumnBucket::Accepted,
        };
    }
    match state {
        StoryState::Icebox => ColumnBucket::Icebox,
        StoryState::Ready | StoryState::Rejected => ColumnBucket::Ready,
        StoryState::InProgress | StoryState::NeedsFixes => ColumnBucket::InProgress,
        StoryState::DeskCheck => ColumnBucket::DeskCheck,
        StoryState::Delivered | StoryState::ReadyForAccept => ColumnBucket::Delivered,
        StoryState::Accepted => ColumnBucket::Accepted,
    }
}
