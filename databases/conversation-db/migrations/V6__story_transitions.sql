-- the board's lifecycle ledger: every observed state change, local or remote-sourced
CREATE TABLE story_transitions (
    seq         INTEGER PRIMARY KEY AUTOINCREMENT,
    story_id    INTEGER NOT NULL,
    new_state   TEXT NOT NULL CHECK (new_state IN ('ready', 'in-progress', 'acceptance', 'accepted', 'retired')),
    observed_at INTEGER NOT NULL
) STRICT;
CREATE INDEX idx_story_transitions_observed_at ON story_transitions(observed_at);
