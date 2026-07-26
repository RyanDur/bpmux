-- addressing is data: one row per recipient, role-predicates and user-references alike
CREATE TABLE IF NOT EXISTS message_recipients (
    message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    kind       TEXT NOT NULL CHECK (kind IN ('role', 'user')),
    value      TEXT NOT NULL
) STRICT;
CREATE INDEX IF NOT EXISTS idx_message_recipients_message ON message_recipients(message_id);
CREATE INDEX IF NOT EXISTS idx_message_recipients_addressee ON message_recipients(kind, value);
