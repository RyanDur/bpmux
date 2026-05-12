use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::broadcast;

use super::domain::{FreeComment, GateComment, Story, StoryState, Tag};
use super::events::BoardEvent;

#[derive(Debug)]
pub enum PersistError {
    Io(std::io::Error),
    Serde(serde_json::Error),
}

impl std::fmt::Display for PersistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistError::Io(e) => write!(f, "i/o: {e}"),
            PersistError::Serde(e) => write!(f, "serde: {e}"),
        }
    }
}

impl std::error::Error for PersistError {}

impl From<std::io::Error> for PersistError {
    fn from(e: std::io::Error) -> Self {
        PersistError::Io(e)
    }
}

impl From<serde_json::Error> for PersistError {
    fn from(e: serde_json::Error) -> Self {
        PersistError::Serde(e)
    }
}

pub fn default_store_path() -> Result<PathBuf, std::io::Error> {
    if let Ok(p) = std::env::var("BPMUX_BOARD_STORE_PATH")
        && !p.is_empty()
    {
        return Ok(PathBuf::from(p));
    }
    let base = dirs::home_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "home directory not found")
    })?;
    Ok(base.join(".bpmux").join("board-store.json"))
}

fn tag_from_label_string(s: &str) -> Option<Tag> {
    if s.eq_ignore_ascii_case("blocked") {
        Some(Tag::Blocked)
    } else {
        None
    }
}

const SUBSCRIBE_BUFFER: usize = 1_000;

pub struct BoardStore {
    stories: RwLock<HashMap<u64, Story>>,
    event_tx: broadcast::Sender<BoardEvent>,
    dirty: AtomicBool,
    #[cfg(test)]
    test_post_send_hook: std::sync::Mutex<Option<Box<dyn Fn() + Send + Sync>>>,
}

impl BoardStore {
    pub fn new() -> Self {
        let (event_tx, _rx) = broadcast::channel(SUBSCRIBE_BUFFER);
        Self {
            stories: RwLock::new(HashMap::new()),
            event_tx,
            dirty: AtomicBool::new(false),
            #[cfg(test)]
            test_post_send_hook: std::sync::Mutex::new(None),
        }
    }

    #[cfg(test)]
    pub fn set_test_post_send_hook<F>(&self, f: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        *self
            .test_post_send_hook
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(Box::new(f));
    }

    pub fn load_from_disk(path: &Path) -> Result<Self, PersistError> {
        let store = Self::new();
        if !path.exists() {
            return Ok(store);
        }
        let bytes = std::fs::read(path)?;
        if bytes.is_empty() {
            return Ok(store);
        }
        let stories: HashMap<u64, Story> = serde_json::from_slice(&bytes)?;
        let mut w = store.stories.write().unwrap_or_else(|e| e.into_inner());
        *w = stories;
        drop(w);
        Ok(store)
    }

    pub fn persist_to_disk(&self, path: &Path) -> Result<(), PersistError> {
        let snapshot: HashMap<u64, Story> = {
            let r = self.stories.read().unwrap_or_else(|e| e.into_inner());
            r.clone()
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec(&snapshot)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, path)?;
        self.dirty.store(false, Ordering::Release);
        Ok(())
    }

    /// #1289 Phase-1 step 2: load via Storage trait instead of path.
    /// Sister of `load_from_disk` but routes through the Storage
    /// abstraction so tests can inject `MemoryStorage` (zero disk I/O).
    /// Production behavior preserved: `FileStorage::load_board_snapshot`
    /// resolves the same path as `default_store_path()` at call-time.
    pub fn load_from_storage(
        storage: &dyn crate::session::storage::Storage,
    ) -> Result<Self, PersistError> {
        let store = Self::new();
        let snapshot = storage.load_board_snapshot()?;
        let mut w = store.stories.write().unwrap_or_else(|e| e.into_inner());
        *w = snapshot;
        drop(w);
        Ok(store)
    }

    /// #1289 Phase-1 step 2: persist via Storage trait instead of path.
    /// Sister of `persist_to_disk`.
    pub fn persist_to_storage(
        &self,
        storage: &dyn crate::session::storage::Storage,
    ) -> Result<(), PersistError> {
        let snapshot: HashMap<u64, Story> = {
            let r = self.stories.read().unwrap_or_else(|e| e.into_inner());
            r.clone()
        };
        storage.save_board_snapshot(&snapshot)?;
        self.dirty.store(false, Ordering::Release);
        Ok(())
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Acquire)
    }

    pub fn seed_all(&self, stories: Vec<Story>) {
        let mut w = self.stories.write().unwrap_or_else(|e| e.into_inner());
        for story in stories {
            w.insert(story.number, story);
        }
        drop(w);
        self.dirty.store(true, Ordering::Release);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<BoardEvent> {
        self.event_tx.subscribe()
    }

    pub fn receiver_count(&self) -> usize {
        self.event_tx.receiver_count()
    }

    /// Apply a `BoardEvent` to the canonical store.
    ///
    /// **Fire-ordering invariant (#1282)**: `event_tx.send` fires BEFORE
    /// the write-lock acquires and mutates the store, in this exact
    /// order within this function. Today's #1274 root cause class: a
    /// state-mutation path that committed without firing the event,
    /// leaving downstream listeners (close_iteration promote-batch
    /// scan, icebox_unified cascade-clean, iteration_id membership scan)
    /// unable to detect the transition. The pre-commit ordering ensures
    /// that any observer running synchronously between the send and the
    /// write-lock acquire (e.g., the `#[cfg(test)] test_post_send_hook`)
    /// sees the pre-mutation store-state; the deterministic-ordering
    /// regression test installs such a hook to pin the invariant.
    ///
    /// `BoardStore::apply` is the single chokepoint for every mutation
    /// in the canonical mutation chain (bridge handlers →
    /// `RefreshDriver::mutate_sync` → `SyncOrchestrator::mutate` →
    /// `BoardStore::apply`). Any mutation that goes through this fn
    /// inherits the invariant; any mutation that bypasses this fn
    /// violates the invariant and must be routed through here per
    /// `feedback_canonical_mutation_chain_through_orchestrator`.
    ///
    /// Sister-invariant for non-state mutations (`TagAdded`, `TagRemoved`,
    /// `ParentEpicSet`/`Unset`, `StoryCreated`, `StoryClosed`/`Reopened`,
    /// `LabelsRefreshed`, etc.): same pre-commit fire-ordering applies
    /// since the same code path serves all events.
    pub fn apply(&self, event: &BoardEvent) {
        let _ = self.event_tx.send(event.clone());
        #[cfg(test)]
        {
            let hook = self
                .test_post_send_hook
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(f) = hook.as_ref() {
                f();
            }
        }
        let mut w = self.stories.write().unwrap_or_else(|e| e.into_inner());
        let number = event.story_number();
        match event {
            BoardEvent::StoryCreated {
                meta,
                number: n,
                title,
                body,
                initial_state,
                story_type,
            } => {
                let story = Story {
                    number: *n,
                    title: title.clone(),
                    state: *initial_state,
                    story_type: *story_type,
                    tags: Vec::new(),
                    filed_at: meta.ts,
                    filed_by: meta.actor.clone(),
                    body: Some(body.clone()),
                    free_comments: Some(Vec::new()),
                    gate_comments: Some(Vec::new()),
                    is_closed: false,
                    parent_epic: None,
                    iteration_id: None,
                    acceptance_test_path: crate::bridge::parse_acceptance_test_path(body),
                    raw_labels: Vec::new(),
                    body_etag: None,
                };
                w.insert(*n, story);
            }
            BoardEvent::StateTransitioned { new_state, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.state = *new_state;
                }
            }
            BoardEvent::TagAdded { label, .. } => {
                if let Some(tag) = tag_from_label_string(label)
                    && let Some(story) = w.get_mut(&number)
                    && !story.tags.contains(&tag)
                {
                    story.tags.push(tag);
                }
            }
            BoardEvent::TagRemoved { label, .. } => {
                if let Some(tag) = tag_from_label_string(label)
                    && let Some(story) = w.get_mut(&number)
                {
                    story.tags.retain(|t| *t != tag);
                }
            }
            BoardEvent::GateCommentAppended {
                meta,
                gate,
                payload,
                ..
            } => {
                if let Some(story) = w.get_mut(&number) {
                    let entry = GateComment::from_event(*gate, meta, payload.clone());
                    match &mut story.gate_comments {
                        Some(list) => list.push(entry),
                        None => story.gate_comments = Some(vec![entry]),
                    }
                }
            }
            BoardEvent::FreeCommentAppended { meta, body, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    let entry = FreeComment::from_event(body.clone(), meta);
                    match &mut story.free_comments {
                        Some(list) => list.push(entry),
                        None => story.free_comments = Some(vec![entry]),
                    }
                }
            }
            BoardEvent::BodyFetched { body, etag, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.body = Some(body.clone());
                    story.acceptance_test_path = crate::bridge::parse_acceptance_test_path(body);
                    story.body_etag = etag.clone();
                }
            }
            BoardEvent::FreeCommentsFetched { comments, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.free_comments = Some(comments.clone());
                }
            }
            BoardEvent::GateCommentsFetched { comments, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.gate_comments = Some(comments.clone());
                }
            }
            BoardEvent::StoryClosed { .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.is_closed = true;
                }
            }
            BoardEvent::StoryReopened { .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.is_closed = false;
                }
            }
            BoardEvent::ParentEpicSet { epic_number, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.parent_epic = Some(*epic_number);
                }
            }
            BoardEvent::ParentEpicUnset { .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.parent_epic = None;
                }
            }
            BoardEvent::StoryRefreshed { story, .. } => {
                if let Some(existing) = w.get_mut(&number) {
                    existing.title = story.title.clone();
                    existing.state = story.state;
                    existing.story_type = story.story_type;
                    existing.tags = story.tags.clone();
                    existing.is_closed = story.is_closed;
                    existing.parent_epic = story.parent_epic;
                    existing.iteration_id = story.iteration_id;
                    if story.body.is_some() {
                        existing.body = story.body.clone();
                        existing.acceptance_test_path = story.acceptance_test_path.clone();
                        existing.body_etag = None;
                    }
                } else {
                    w.insert(number, (**story).clone());
                }
            }
            BoardEvent::LabelsRefreshed { labels, .. } => {
                if labels.is_empty() {
                } else if let Some(existing) = w.get_mut(&number) {
                    existing.state = crate::board::adapters::github::state_from_labels(
                        labels,
                        existing.is_closed,
                    );
                    existing.tags = labels
                        .iter()
                        .filter_map(|l| tag_from_label_string(l))
                        .collect();
                    existing.parent_epic =
                        crate::board::adapters::github::parent_epic_from_labels(labels);
                    existing.iteration_id =
                        crate::board::adapters::github::iteration_id_from_labels(labels);
                    existing.raw_labels = labels.clone();
                }
            }
            // #1195: ConflictDetected/ConflictResolved are
            // audit-trail-only events. The conflict-pending count
            // lives on SyncStatus (drain_once writes it); no
            // canonical-store-level Story-field updates needed.
            // Subscribers (TUI, renderer) observe these via the
            // event-broadcast channel + can derive UI hints.
            BoardEvent::ConflictDetected { .. } | BoardEvent::ConflictResolved { .. } => {}
        }
        drop(w);
        self.dirty.store(true, Ordering::Release);
    }

    pub fn story(&self, number: u64) -> Option<Story> {
        self.stories
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&number)
            .cloned()
    }

    pub fn all_stories(&self) -> Vec<Story> {
        self.stories
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect()
    }

    pub fn stories_in_state(&self, state: StoryState) -> Vec<Story> {
        self.stories
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .filter(|s| s.state == state)
            .cloned()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.stories.read().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.stories
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .is_empty()
    }
}

impl Default for BoardStore {
    fn default() -> Self {
        Self::new()
    }
}
