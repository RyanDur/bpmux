-- the owner's GitHub connection: a single row, replaced on reconnect.
-- The token NEVER lands here — hosted connections keep it in the OS
-- keychain; base_url connections (mock/enterprise-proxy) are token-less.
CREATE TABLE github_connection (
    id       INTEGER PRIMARY KEY CHECK (id = 1),
    owner    TEXT NOT NULL,
    repo     TEXT NOT NULL,
    base_url TEXT
) STRICT;
