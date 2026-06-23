CREATE TABLE agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    role TEXT NOT NULL
);

CREATE TABLE messages (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    ts TEXT NOT NULL,
    sender TEXT NOT NULL,
    role TEXT,
    recipient TEXT,
    message_type TEXT NOT NULL,
    payload TEXT NOT NULL
);

CREATE INDEX idx_messages_ts ON messages (ts);
CREATE INDEX idx_messages_sender ON messages (sender);
CREATE INDEX idx_messages_recipient ON messages (recipient);
CREATE INDEX idx_messages_type ON messages (message_type);

CREATE TABLE documents (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
