CREATE TABLE users (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    role       TEXT NOT NULL,
    kind       TEXT NOT NULL DEFAULT 'agent' CHECK (kind IN ('agent', 'owner', 'system')),
    cursor     TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
) STRICT;

CREATE TABLE messages (
    seq          INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at   INTEGER NOT NULL,
    sender       TEXT NOT NULL,
    message_type TEXT NOT NULL,
    payload      TEXT NOT NULL
) STRICT;

CREATE TABLE outbox (
    seq   INTEGER PRIMARY KEY AUTOINCREMENT,
    event TEXT NOT NULL
) STRICT;

CREATE TABLE reactions (
    channel   TEXT PRIMARY KEY,
    reactions TEXT NOT NULL
) STRICT;

CREATE TABLE bookmarks (
    role     TEXT PRIMARY KEY,
    bookmark TEXT NOT NULL
) STRICT;

CREATE TABLE meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;
