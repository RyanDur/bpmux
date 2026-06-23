CREATE TABLE agents (
    id     TEXT PRIMARY KEY,
    name   TEXT NOT NULL,
    role   TEXT NOT NULL,
    kind   TEXT NOT NULL DEFAULT 'agent',
    cursor TEXT
) STRICT;

CREATE TABLE messages (
    seq          INTEGER PRIMARY KEY AUTOINCREMENT,
    ts           TEXT NOT NULL,
    sender       TEXT NOT NULL,
    message_type TEXT NOT NULL,
    payload      TEXT NOT NULL
) STRICT;

CREATE TABLE documents (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;
