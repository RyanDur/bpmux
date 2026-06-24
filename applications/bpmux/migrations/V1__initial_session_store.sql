-- uuid shape: 8-4-4-4-12 lowercase hex, as produced by Uuid::new_v4().to_string()
CREATE TABLE users (
    id         TEXT PRIMARY KEY
               CHECK (id GLOB '[0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]-[0-9a-f][0-9a-f][0-9a-f][0-9a-f]-[0-9a-f][0-9a-f][0-9a-f][0-9a-f]-[0-9a-f][0-9a-f][0-9a-f][0-9a-f]-[0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]'),
    session_id TEXT NOT NULL UNIQUE,
    name       TEXT NOT NULL,
    role       TEXT NOT NULL,
    kind       TEXT NOT NULL DEFAULT 'agent' CHECK (kind IN ('agent', 'owner', 'system')),
    last_delivered_at INTEGER,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
) STRICT;

CREATE TABLE messages (
    seq          INTEGER PRIMARY KEY AUTOINCREMENT,
    id           TEXT NOT NULL UNIQUE
                 CHECK (id GLOB '[0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]-[0-9a-f][0-9a-f][0-9a-f][0-9a-f]-[0-9a-f][0-9a-f][0-9a-f][0-9a-f]-[0-9a-f][0-9a-f][0-9a-f][0-9a-f]-[0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]'),
    created_at   INTEGER NOT NULL,
    sender       TEXT REFERENCES users(id),
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
    user_id  TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    bookmark TEXT NOT NULL
) STRICT;

CREATE TABLE surfaced_topics (
    topic TEXT PRIMARY KEY
) STRICT;

CREATE TABLE meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;
