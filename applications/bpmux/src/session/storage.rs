//! Storage abstraction — Strategy pattern (#1289).
//!
//! Two traits define the persistence boundary:
//!
//! - [`Storage`] for session-scoped state (messages, lessons, board
//!   snapshot, outbox, bookmarks, pins, reactions, iteration markers,
//!   generic key-value). State lives under a session directory and
//!   is short-lived relative to the user's overall bpmux history.
//! - [`GlobalStore`] for cross-session state living at user-global
//!   paths under `~/.bpmux/` (priority initially; identity /
//!   briefings / tokens / usage / auth-token migration deferred to
//!   #1290).
//!
//! Each trait has a production impl ([`FileStorage`] / [`FileGlobalStore`])
//! and an in-memory impl ([`MemoryStorage`] / [`MemoryGlobalStore`]) for
//! tests. Tests opt into in-memory mode via the
//! `BPMUX_STORAGE_BACKEND=memory` env-var at `Session::new`
//! construction time (see `session/log.rs:Session::new`).
//!
//! **Trait contract**:
//! - Implementations satisfy semantic-parity verified by the
//!   `contract!` macro at `session/storage_tests.rs` — every behavior
//!   asserted runs against both `FileStorage` and `MemoryStorage`;
//!   divergence is caught at test-write time.
//! - Filesystem idiosyncrasies (atomic-rename via tmpfile, fsync
//!   semantics, mode-0600 for sensitive paths, lock-file behavior)
//!   stay inside `FileStorage`. The trait surface returns
//!   `Result<T, std::io::Error>` / `Result<T, PersistError>`; callers
//!   never reach into FS quirks. Per
//!   `feedback_external_system_idiosyncrasies_at_boundary`.
//! - `MemoryStorage` is per-instance: tests get isolated state by
//!   constructing a fresh `MemoryStorage` per test fixture.
//!
//! **Path-injection builders**: each FileStorage method that derives
//! a non-session-dir path (board store + outbox so far) has a
//! corresponding `with_<thing>_path(p)` builder. Production
//! constructs with `None` so the default `default_<thing>_path()`
//! helper resolves at call-time (preserving the env-overridable +
//! home-default semantic for `BPMUX_BOARD_STORE_PATH`,
//! `BPMUX_BOARD_OUTBOX_PATH`, `BPMUX_PRIORITY_PATH`-equivalent).
//! Tests construct with explicit per-test tempdirs to avoid
//! polluting Ryan's actual `~/.bpmux/*` files.
//!
//! **Storage vs GlobalStore separation**: motivated by lifetime —
//! `Storage` is per-session and tests get fresh state per launch;
//! `GlobalStore` is per-user and persists across sessions
//! (production reads + writes the same `~/.bpmux/priority.json`
//! across many bpmux startups). Mixing them on a single trait would
//! conflate two different reset-points + boundary-semantics. The
//! separation is explicit per designer's 12:32 UTC trait-shape
//! review.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::board::domain::Story;
use crate::board::orchestrator::QueuedEvent;
use crate::board::store::PersistError;

use super::lessons::LessonIndex;
use super::log::LogError;
use super::message::Message;
use super::search::{self, SearchFilters, SearchResult};

/// The persistence interface. Core code depends on this trait,
/// never on a concrete storage mechanism.
pub trait Storage: Send + Sync {
    // Messages
    fn append_message(&self, msg: &Message) -> Result<(), LogError>;
    fn read_messages(&self) -> Result<Vec<Message>, LogError>;
    fn search_messages(&self, filters: &SearchFilters) -> Vec<SearchResult>;
    /// #1306 slice 4.1: test-only message-store wipe so RTL+real-backend
    /// tests can reset state between cases sharing a per-run backend.
    /// Production-callers never reach here — gated by
    /// `BPMUX_TEST_SEED_API` at the REST boundary.
    fn clear_messages(&self) -> Result<(), std::io::Error>;

    // Lessons
    fn load_lessons(&self) -> LessonIndex;
    fn save_lessons(&self, index: &LessonIndex) -> Result<(), std::io::Error>;
    fn add_lesson(
        &self,
        topic: &str,
        lesson: &str,
        author_role: &str,
        evidence: Vec<String>,
    ) -> Result<(), std::io::Error>;

    // Peer lessons
    fn load_peer_lessons(&self, peer: &str) -> LessonIndex;
    fn save_peer_lessons(&self, peer: &str, index: &LessonIndex) -> Result<(), std::io::Error>;
    fn list_peers(&self) -> Vec<String>;

    // Session state (generic key-value)
    fn load_state(&self, key: &str) -> Option<String>;
    fn save_state(&self, key: &str, value: &str);

    // Session directory (for backward compatibility during migration)
    fn dir(&self) -> &Path;

    // #1289 Phase-1 extension: board snapshot (board-store.json).
    // FileStorage uses the configured `board_store_path` (env-overridable
    // via BPMUX_BOARD_STORE_PATH at call-time; injectable per-instance
    // via `with_board_store_path` for test isolation). MemoryStorage uses
    // per-instance in-memory state.
    fn load_board_snapshot(&self) -> Result<HashMap<u64, Story>, PersistError>;
    fn save_board_snapshot(&self, snapshot: &HashMap<u64, Story>) -> Result<(), PersistError>;

    // #1289 Phase-1 step 3: outbox queue (board-outbox.json).
    // FileStorage uses the configured `outbox_path` (env-overridable
    // via BPMUX_BOARD_OUTBOX_PATH at call-time; injectable per-instance
    // via `with_outbox_path` for test isolation). MemoryStorage uses
    // per-instance in-memory state. Disk-shape supports backward-compat
    // legacy-Vec<BoardEvent> parse via OutboxEntryOnDisk; new writes use
    // the QueuedEvent format directly.
    fn load_outbox(&self) -> Result<VecDeque<QueuedEvent>, PersistError>;
    fn save_outbox(&self, events: &[QueuedEvent]) -> Result<(), PersistError>;

    // #1289 Phase-2: session-scoped per-role bookmark
    // (bookmark-{role}.json). FileStorage derives path from self.dir +
    // role. MemoryStorage uses HashMap<role, Bookmark> per-instance.
    fn load_bookmark(&self, role: &str) -> super::bookmark::Bookmark;
    fn save_bookmark(
        &self,
        role: &str,
        bookmark: &super::bookmark::Bookmark,
    ) -> Result<(), std::io::Error>;

    // #1289 Phase-2: session-scoped pins (pins.json).
    fn load_pins(&self) -> super::pins::PinsStore;
    fn save_pins(&self, store: &super::pins::PinsStore) -> Result<(), std::io::Error>;

    // #1289 Phase-2: session-scoped reactions (reactions.json).
    fn load_reactions(&self) -> super::reactions::ReactionsStore;
    fn save_reactions(
        &self,
        store: &super::reactions::ReactionsStore,
    ) -> Result<(), std::io::Error>;
}

/// #1289 Phase-3: cross-session global state. Distinct from `Storage`
/// (which is session-scoped) because these files live at user-global
/// paths (e.g., `~/.bpmux/priority.json`, `~/.bpmux/identity/{role}.json`)
/// and survive across multiple bpmux sessions.
///
/// FileGlobalStore production impl resolves paths via existing
/// `default_*_path()` helpers (each preserves their respective env-
/// override + home-default semantics). MemoryGlobalStore tests-only
/// impl uses per-instance Mutex<T> state.
pub trait GlobalStore: Send + Sync {
    // Priority (~/.bpmux/priority.json)
    fn load_priority(&self) -> super::priority::PriorityIndex;
    fn save_priority(&self, index: &super::priority::PriorityIndex) -> Result<(), std::io::Error>;

    // #1290 Phase-3 step 2: per-role identity (~/.bpmux/identity/{role}.json)
    fn load_identity(&self, role: &str) -> super::identity::IdentityIndex;
    fn save_identity(
        &self,
        role: &str,
        index: &super::identity::IdentityIndex,
    ) -> Result<(), std::io::Error>;

    // #1290 Phase-3 step 3: per-role briefing (~/.bpmux/briefings/{role}.md).
    // Raw markdown string; 1MB cap enforced in production save path.
    fn load_briefing(&self, role: &str) -> String;
    fn save_briefing(&self, role: &str, content: &str) -> Result<(), std::io::Error>;

    // #1290 Phase-3 step 4: token log (~/.bpmux/tokens/log.jsonl).
    // Append-only; FileStore preserves POSIX append atomicity.
    fn append_token_row(&self, row: &super::tokens::TokenRow) -> Result<(), super::log::LogError>;
    fn read_token_rows(&self) -> Result<Vec<super::tokens::TokenRow>, super::log::LogError>;

    // #1290 Phase-3 step 5: feature-usage counters (~/.bpmux/usage.json).
    // Whole-map read/replace; the higher-level `usage::record()` helper
    // composes load-mutate-save against the default path on FileStore.
    fn load_usage(&self) -> std::collections::HashMap<String, super::usage::Record>;
    fn save_usage(
        &self,
        map: &std::collections::HashMap<String, super::usage::Record>,
    ) -> Result<(), std::io::Error>;

    // #1290 Phase-3 step 6 (AC 18): per-run auth token (~/.bpmux/auth_token).
    // Process-owned secret with mode-0600 filesystem semantics. The
    // MemoryGlobalStore impl returns NotSupported by boundary-discipline
    // (`feedback_external_system_idiosyncrasies_at_boundary`) — tests
    // requiring auth-token isolation use FileGlobalStore with
    // `BPMUX_AUTH_TOKEN_PATH` per-test tempdir (the established pattern
    // at `applications/electron/tests/helpers/launch.ts`).
    fn ensure_auth_token(&self) -> std::io::Result<String>;
    fn read_auth_token(&self) -> std::io::Result<String>;
}

/// File-based global store — production impl.
pub struct FileGlobalStore {
    /// #1289: per-instance priority-path override. Tests inject
    /// per-test paths to avoid sharing `~/.bpmux/priority.json`.
    priority_path: Option<PathBuf>,
    /// #1290: per-instance identity-dir override. Tests inject
    /// tempdirs to avoid sharing `~/.bpmux/identity/`.
    identity_dir: Option<PathBuf>,
    /// #1290: per-instance briefings-dir override. Tests inject
    /// tempdirs to avoid sharing `~/.bpmux/briefings/`.
    briefings_dir: Option<PathBuf>,
    /// #1290: per-instance tokens-log path override. Tests inject
    /// per-test paths to avoid sharing `~/.bpmux/tokens/log.jsonl`.
    tokens_log_path: Option<PathBuf>,
    /// #1290: per-instance usage-path override. Tests inject per-test
    /// paths to avoid sharing `~/.bpmux/usage.json`.
    usage_path: Option<PathBuf>,
    /// #1290: per-instance auth-token-path override. Tests inject
    /// per-test paths to avoid sharing `~/.bpmux/auth_token`.
    auth_token_path: Option<PathBuf>,
}

impl Default for FileGlobalStore {
    fn default() -> Self {
        Self::new()
    }
}

impl FileGlobalStore {
    pub fn new() -> Self {
        Self {
            priority_path: None,
            identity_dir: None,
            briefings_dir: None,
            tokens_log_path: None,
            usage_path: None,
            auth_token_path: None,
        }
    }

    /// Opt-in path-injection for the priority index. Tests use this
    /// to point at a unique tempdir; production leaves it `None` so
    /// `default_priority_path()` resolves at call-time.
    pub fn with_priority_path(mut self, path: PathBuf) -> Self {
        self.priority_path = Some(path);
        self
    }

    /// #1290: opt-in identity-dir override (per-role files live under
    /// this dir as `{role}.json`).
    pub fn with_identity_dir(mut self, dir: PathBuf) -> Self {
        self.identity_dir = Some(dir);
        self
    }

    /// #1290: opt-in briefings-dir override (per-role files live under
    /// this dir as `{role}.md`).
    pub fn with_briefings_dir(mut self, dir: PathBuf) -> Self {
        self.briefings_dir = Some(dir);
        self
    }

    /// #1290: opt-in tokens-log path override.
    pub fn with_tokens_log_path(mut self, path: PathBuf) -> Self {
        self.tokens_log_path = Some(path);
        self
    }

    /// #1290: opt-in usage-path override.
    pub fn with_usage_path(mut self, path: PathBuf) -> Self {
        self.usage_path = Some(path);
        self
    }

    /// #1290: opt-in auth-token-path override.
    pub fn with_auth_token_path(mut self, path: PathBuf) -> Self {
        self.auth_token_path = Some(path);
        self
    }

    fn resolve_priority_path(&self) -> Result<PathBuf, String> {
        if let Some(p) = &self.priority_path {
            return Ok(p.clone());
        }
        super::priority::default_priority_path()
    }

    fn resolve_identity_dir(&self) -> Result<PathBuf, std::io::Error> {
        if let Some(d) = &self.identity_dir {
            return Ok(d.clone());
        }
        super::identity::default_identity_dir()
    }

    fn resolve_briefings_dir(&self) -> PathBuf {
        if let Some(d) = &self.briefings_dir {
            return d.clone();
        }
        super::briefing::briefings_dir()
    }

    fn resolve_tokens_log_path(&self) -> Result<PathBuf, super::log::LogError> {
        if let Some(p) = &self.tokens_log_path {
            return Ok(p.clone());
        }
        super::tokens::default_tokens_log_path()
    }

    fn resolve_usage_path(&self) -> Option<PathBuf> {
        if let Some(p) = &self.usage_path {
            return Some(p.clone());
        }
        super::usage::default_usage_path()
    }

    fn resolve_auth_token_path(&self) -> std::io::Result<PathBuf> {
        if let Some(p) = &self.auth_token_path {
            return Ok(p.clone());
        }
        super::auth_token::default_auth_token_path().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "home directory not found for auth_token",
            )
        })
    }
}

impl GlobalStore for FileGlobalStore {
    fn load_priority(&self) -> super::priority::PriorityIndex {
        match self.resolve_priority_path() {
            Ok(path) => super::priority::load_priority(&path),
            Err(_) => super::priority::PriorityIndex::default(),
        }
    }

    fn save_priority(&self, index: &super::priority::PriorityIndex) -> Result<(), std::io::Error> {
        let path = self
            .resolve_priority_path()
            .map_err(std::io::Error::other)?;
        super::priority::save_priority(&path, index)
    }

    fn load_identity(&self, role: &str) -> super::identity::IdentityIndex {
        match self.resolve_identity_dir() {
            Ok(dir) => super::identity::load_identity(&dir, role),
            Err(_) => super::identity::IdentityIndex::default(),
        }
    }

    fn save_identity(
        &self,
        role: &str,
        index: &super::identity::IdentityIndex,
    ) -> Result<(), std::io::Error> {
        let dir = self.resolve_identity_dir()?;
        super::identity::save_identity(&dir, role, index)
    }

    fn load_briefing(&self, role: &str) -> String {
        super::briefing::load_briefing_in(&self.resolve_briefings_dir(), role)
    }

    fn save_briefing(&self, role: &str, content: &str) -> Result<(), std::io::Error> {
        super::briefing::save_briefing_in(&self.resolve_briefings_dir(), role, content)
    }

    fn append_token_row(&self, row: &super::tokens::TokenRow) -> Result<(), super::log::LogError> {
        let path = self.resolve_tokens_log_path()?;
        super::tokens::append_token_row(&path, row)
    }

    fn read_token_rows(&self) -> Result<Vec<super::tokens::TokenRow>, super::log::LogError> {
        let path = self.resolve_tokens_log_path()?;
        super::tokens::read_token_rows(&path)
    }

    fn load_usage(&self) -> std::collections::HashMap<String, super::usage::Record> {
        let Some(path) = self.resolve_usage_path() else {
            return std::collections::HashMap::new();
        };
        super::usage::load_from(&path)
    }

    fn save_usage(
        &self,
        map: &std::collections::HashMap<String, super::usage::Record>,
    ) -> Result<(), std::io::Error> {
        let Some(path) = self.resolve_usage_path() else {
            return Ok(());
        };
        super::usage::save_to(&path, map)
    }

    fn ensure_auth_token(&self) -> std::io::Result<String> {
        let path = self.resolve_auth_token_path()?;
        super::auth_token::ensure(&path)
    }

    fn read_auth_token(&self) -> std::io::Result<String> {
        let path = self.resolve_auth_token_path()?;
        super::auth_token::read(&path)
    }
}

/// In-memory global store — tests-only impl.
pub struct MemoryGlobalStore {
    priority: Mutex<super::priority::PriorityIndex>,
    identity: Mutex<HashMap<String, super::identity::IdentityIndex>>,
    briefings: Mutex<HashMap<String, String>>,
    tokens: Mutex<Vec<super::tokens::TokenRow>>,
    usage: Mutex<HashMap<String, super::usage::Record>>,
}

impl Default for MemoryGlobalStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryGlobalStore {
    pub fn new() -> Self {
        Self {
            priority: Mutex::new(super::priority::PriorityIndex::default()),
            identity: Mutex::new(HashMap::new()),
            briefings: Mutex::new(HashMap::new()),
            tokens: Mutex::new(Vec::new()),
            usage: Mutex::new(HashMap::new()),
        }
    }
}

impl GlobalStore for MemoryGlobalStore {
    fn load_priority(&self) -> super::priority::PriorityIndex {
        self.priority
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn save_priority(&self, index: &super::priority::PriorityIndex) -> Result<(), std::io::Error> {
        *self.priority.lock().unwrap_or_else(|e| e.into_inner()) = index.clone();
        Ok(())
    }

    fn load_identity(&self, role: &str) -> super::identity::IdentityIndex {
        self.identity
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(role)
            .cloned()
            .unwrap_or_default()
    }

    fn save_identity(
        &self,
        role: &str,
        index: &super::identity::IdentityIndex,
    ) -> Result<(), std::io::Error> {
        self.identity
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(role.to_string(), index.clone());
        Ok(())
    }

    fn load_briefing(&self, role: &str) -> String {
        self.briefings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(role)
            .cloned()
            .unwrap_or_default()
    }

    fn save_briefing(&self, role: &str, content: &str) -> Result<(), std::io::Error> {
        self.briefings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(role.to_string(), content.to_string());
        Ok(())
    }

    fn append_token_row(&self, row: &super::tokens::TokenRow) -> Result<(), super::log::LogError> {
        self.tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(row.clone());
        Ok(())
    }

    fn read_token_rows(&self) -> Result<Vec<super::tokens::TokenRow>, super::log::LogError> {
        Ok(self
            .tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone())
    }

    fn load_usage(&self) -> HashMap<String, super::usage::Record> {
        self.usage.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn save_usage(
        &self,
        map: &HashMap<String, super::usage::Record>,
    ) -> Result<(), std::io::Error> {
        *self.usage.lock().unwrap_or_else(|e| e.into_inner()) = map.clone();
        Ok(())
    }

    // #1290 AC 18 boundary-discipline: auth_token is a process-owned
    // secret requiring mode-0600 filesystem semantics. MemoryGlobalStore
    // refuses the operation rather than fake non-FS in-memory storage —
    // tests requiring auth-token isolation use FileGlobalStore with a
    // per-test BPMUX_AUTH_TOKEN_PATH tempdir (the established pattern).
    fn ensure_auth_token(&self) -> std::io::Result<String> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "MemoryGlobalStore does not support auth_token (process-owned secret \
             requires mode-0600 filesystem semantics); use FileGlobalStore with \
             BPMUX_AUTH_TOKEN_PATH for test isolation",
        ))
    }

    fn read_auth_token(&self) -> std::io::Result<String> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "MemoryGlobalStore does not support auth_token (process-owned secret \
             requires mode-0600 filesystem semantics); use FileGlobalStore with \
             BPMUX_AUTH_TOKEN_PATH for test isolation",
        ))
    }
}

/// File-based storage — the current implementation.
pub struct FileStorage {
    dir: PathBuf,
    /// #1289: per-instance board-store path override. `None` →
    /// `crate::board::store::default_store_path()` (env-or-home).
    /// `Some(p)` → use `p` directly. Tests inject per-test paths to
    /// avoid sharing `~/.bpmux/board-store.json`; production uses
    /// the default-path resolution.
    board_store_path: Option<PathBuf>,
    /// #1289: per-instance outbox-path override. Same shape as
    /// `board_store_path`; `None` → `default_outbox_path()` at
    /// call-time (BPMUX_BOARD_OUTBOX_PATH env-or-home).
    outbox_path: Option<PathBuf>,
}

impl FileStorage {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            board_store_path: None,
            outbox_path: None,
        }
    }

    /// #1289: opt-in path-injection for the board-snapshot store.
    /// Tests use this to point at a unique tempdir; production
    /// leaves it `None` so `default_store_path()` resolves at
    /// call-time (preserves env-override + home-default semantic).
    pub fn with_board_store_path(mut self, path: PathBuf) -> Self {
        self.board_store_path = Some(path);
        self
    }

    /// #1289 Phase-1 step 3: opt-in path-injection for the outbox.
    /// Sister of `with_board_store_path` at outbox-axis.
    pub fn with_outbox_path(mut self, path: PathBuf) -> Self {
        self.outbox_path = Some(path);
        self
    }

    fn resolve_board_store_path(&self) -> Result<PathBuf, std::io::Error> {
        if let Some(p) = &self.board_store_path {
            return Ok(p.clone());
        }
        crate::board::store::default_store_path()
    }

    fn resolve_outbox_path(&self) -> Result<PathBuf, std::io::Error> {
        if let Some(p) = &self.outbox_path {
            return Ok(p.clone());
        }
        crate::board::orchestrator::default_outbox_path()
    }
}

impl Storage for FileStorage {
    fn append_message(&self, msg: &Message) -> Result<(), LogError> {
        super::log::append_message(&self.dir, msg)
    }

    fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        super::log::read_messages(&self.dir)
    }

    fn search_messages(&self, filters: &SearchFilters) -> Vec<SearchResult> {
        match self.read_messages() {
            Ok(messages) => super::search::search(&messages, filters),
            Err(_) => Vec::new(),
        }
    }

    fn clear_messages(&self) -> Result<(), std::io::Error> {
        let log_path = self.dir.join("log.jsonl");
        match std::fs::remove_file(&log_path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn load_lessons(&self) -> LessonIndex {
        super::lessons::load_index(&self.dir)
    }

    fn save_lessons(&self, index: &LessonIndex) -> Result<(), std::io::Error> {
        super::lessons::save_index(&self.dir, index)
    }

    fn add_lesson(
        &self,
        topic: &str,
        lesson: &str,
        author_role: &str,
        evidence: Vec<String>,
    ) -> Result<(), std::io::Error> {
        super::lessons::add_lesson(&self.dir, topic, lesson, author_role, evidence)
    }

    fn load_peer_lessons(&self, peer: &str) -> LessonIndex {
        super::lessons::load_peer_index(&self.dir, peer)
    }

    fn save_peer_lessons(&self, peer: &str, index: &LessonIndex) -> Result<(), std::io::Error> {
        super::lessons::save_peer_index(&self.dir, peer, index)
    }

    fn list_peers(&self) -> Vec<String> {
        super::lessons::list_peer_names(&self.dir)
    }

    fn load_state(&self, key: &str) -> Option<String> {
        let safe_key = super::identity::sanitize_name(key);
        let path = self.dir.join(format!("{safe_key}.json"));
        std::fs::read_to_string(&path).ok()
    }

    fn save_state(&self, key: &str, value: &str) {
        let safe_key = super::identity::sanitize_name(key);
        let path = self.dir.join(format!("{safe_key}.json"));
        let tmp = self.dir.join(format!("{safe_key}.json.tmp"));
        if let Err(e) = std::fs::write(&tmp, value) {
            super::diag::log_diag(&format!("save_state write failed for {safe_key}: {e}"));
            return;
        }
        if let Err(e) = std::fs::rename(&tmp, &path) {
            super::diag::log_diag(&format!("save_state rename failed for {safe_key}: {e}"));
        }
    }

    fn dir(&self) -> &Path {
        &self.dir
    }

    fn load_board_snapshot(&self) -> Result<HashMap<u64, Story>, PersistError> {
        let path = self.resolve_board_store_path()?;
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let bytes = std::fs::read(&path)?;
        if bytes.is_empty() {
            return Ok(HashMap::new());
        }
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn save_board_snapshot(&self, snapshot: &HashMap<u64, Story>) -> Result<(), PersistError> {
        let path = self.resolve_board_store_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec(snapshot)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    fn load_outbox(&self) -> Result<VecDeque<QueuedEvent>, PersistError> {
        let path = self.resolve_outbox_path()?;
        crate::board::orchestrator::load_outbox_from_disk(&path)
    }

    fn save_outbox(&self, events: &[QueuedEvent]) -> Result<(), PersistError> {
        let path = self.resolve_outbox_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec(events)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    fn load_bookmark(&self, role: &str) -> super::bookmark::Bookmark {
        super::bookmark::load_bookmark(&self.dir, role)
    }

    fn save_bookmark(
        &self,
        role: &str,
        bookmark: &super::bookmark::Bookmark,
    ) -> Result<(), std::io::Error> {
        super::bookmark::save_bookmark(&self.dir, role, bookmark)
    }

    fn load_pins(&self) -> super::pins::PinsStore {
        super::pins::load(&self.dir)
    }

    fn save_pins(&self, store: &super::pins::PinsStore) -> Result<(), std::io::Error> {
        super::pins::save(&self.dir, store)
    }

    fn load_reactions(&self) -> super::reactions::ReactionsStore {
        super::reactions::load(&self.dir)
    }

    fn save_reactions(
        &self,
        store: &super::reactions::ReactionsStore,
    ) -> Result<(), std::io::Error> {
        super::reactions::save(&self.dir, store)
    }
}

/// In-memory storage — #1289 substrate-foundation for the Store-trait
/// DI refactor. Provides parity with `FileStorage` semantics on every
/// existing trait method without disk I/O.
///
/// Tests inject this via the launch helper so per-test backend startup
/// skips filesystem I/O (the macOS-WindowServer-class slowness root
/// cause from f9ebe47 / c8ba878 / 5bdb235 empirical evidence today).
///
/// `dir()` returns the path passed at construction — it's a marker
/// path tests use for any code that derives derived-state from
/// session-dir, but no I/O is performed against it.
pub struct MemoryStorage {
    dir: PathBuf,
    messages: Mutex<Vec<Message>>,
    lessons: Mutex<LessonIndex>,
    peer_lessons: Mutex<HashMap<String, LessonIndex>>,
    state: Mutex<HashMap<String, String>>,
    board_snapshot: Mutex<HashMap<u64, Story>>,
    outbox: Mutex<VecDeque<QueuedEvent>>,
    bookmarks: Mutex<HashMap<String, super::bookmark::Bookmark>>,
    pins: Mutex<super::pins::PinsStore>,
    reactions: Mutex<super::reactions::ReactionsStore>,
}

impl MemoryStorage {
    /// `dir` is a marker path returned by `dir()`. No I/O is performed
    /// against it. Tests typically pass a `tempfile::tempdir()` path
    /// so the marker is unique per test.
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            messages: Mutex::new(Vec::new()),
            lessons: Mutex::new(LessonIndex::default()),
            peer_lessons: Mutex::new(HashMap::new()),
            state: Mutex::new(HashMap::new()),
            board_snapshot: Mutex::new(HashMap::new()),
            outbox: Mutex::new(VecDeque::new()),
            bookmarks: Mutex::new(HashMap::new()),
            pins: Mutex::new(super::pins::PinsStore::default()),
            reactions: Mutex::new(super::reactions::ReactionsStore::default()),
        }
    }
}

impl Storage for MemoryStorage {
    fn append_message(&self, msg: &Message) -> Result<(), LogError> {
        self.messages
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(msg.clone());
        Ok(())
    }

    fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        Ok(self
            .messages
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone())
    }

    fn search_messages(&self, filters: &SearchFilters) -> Vec<SearchResult> {
        let messages = self
            .messages
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        search::search(&messages, filters)
    }

    fn clear_messages(&self) -> Result<(), std::io::Error> {
        self.messages
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        Ok(())
    }

    fn load_lessons(&self) -> LessonIndex {
        self.lessons
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn save_lessons(&self, index: &LessonIndex) -> Result<(), std::io::Error> {
        *self.lessons.lock().unwrap_or_else(|e| e.into_inner()) = index.clone();
        Ok(())
    }

    fn add_lesson(
        &self,
        topic: &str,
        lesson: &str,
        author_role: &str,
        evidence: Vec<String>,
    ) -> Result<(), std::io::Error> {
        let new_entry = super::lessons::AgentLesson {
            topic: topic.to_string(),
            lesson: lesson.to_string(),
            author_role: author_role.to_string(),
            evidence,
            established: chrono::Utc::now().to_rfc3339(),
            last_reinforced: None,
            reinforcement_count: 0,
            reinforcement_history: Vec::new(),
        };
        self.lessons
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .lessons
            .push(new_entry);
        Ok(())
    }

    fn load_peer_lessons(&self, peer: &str) -> LessonIndex {
        self.peer_lessons
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(peer)
            .cloned()
            .unwrap_or_default()
    }

    fn save_peer_lessons(&self, peer: &str, index: &LessonIndex) -> Result<(), std::io::Error> {
        self.peer_lessons
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(peer.to_string(), index.clone());
        Ok(())
    }

    fn list_peers(&self) -> Vec<String> {
        self.peer_lessons
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect()
    }

    fn load_state(&self, key: &str) -> Option<String> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(key)
            .cloned()
    }

    fn save_state(&self, key: &str, value: &str) {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key.to_string(), value.to_string());
    }

    fn dir(&self) -> &Path {
        &self.dir
    }

    fn load_board_snapshot(&self) -> Result<HashMap<u64, Story>, PersistError> {
        Ok(self
            .board_snapshot
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone())
    }

    fn save_board_snapshot(&self, snapshot: &HashMap<u64, Story>) -> Result<(), PersistError> {
        *self
            .board_snapshot
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = snapshot.clone();
        Ok(())
    }

    fn load_outbox(&self) -> Result<VecDeque<QueuedEvent>, PersistError> {
        Ok(self
            .outbox
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone())
    }

    fn save_outbox(&self, events: &[QueuedEvent]) -> Result<(), PersistError> {
        let mut guard = self.outbox.lock().unwrap_or_else(|e| e.into_inner());
        *guard = events.iter().cloned().collect();
        Ok(())
    }

    fn load_bookmark(&self, role: &str) -> super::bookmark::Bookmark {
        self.bookmarks
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(role)
            .cloned()
            .unwrap_or_default()
    }

    fn save_bookmark(
        &self,
        role: &str,
        bookmark: &super::bookmark::Bookmark,
    ) -> Result<(), std::io::Error> {
        self.bookmarks
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(role.to_string(), bookmark.clone());
        Ok(())
    }

    fn load_pins(&self) -> super::pins::PinsStore {
        self.pins.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn save_pins(&self, store: &super::pins::PinsStore) -> Result<(), std::io::Error> {
        *self.pins.lock().unwrap_or_else(|e| e.into_inner()) = store.clone();
        Ok(())
    }

    fn load_reactions(&self) -> super::reactions::ReactionsStore {
        self.reactions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn save_reactions(
        &self,
        store: &super::reactions::ReactionsStore,
    ) -> Result<(), std::io::Error> {
        *self.reactions.lock().unwrap_or_else(|e| e.into_inner()) = store.clone();
        Ok(())
    }
}
