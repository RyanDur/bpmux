-- addressing is data: one row per recipient, role-predicates and user-references alike
CREATE TABLE message_recipients (
    message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    kind       TEXT NOT NULL CHECK (kind IN ('role', 'user')),
    value      TEXT NOT NULL
) STRICT;
CREATE INDEX idx_message_recipients_message ON message_recipients(message_id);
CREATE INDEX idx_message_recipients_addressee ON message_recipients(kind, value);

INSERT INTO message_recipients (message_id, kind, value)
SELECT m.id, 'role', j.value
FROM messages m, json_each(m.payload, '$.to') j;

INSERT INTO message_recipients (message_id, kind, value)
SELECT m.id, 'user', j.value
FROM messages m, json_each(m.payload, '$.to_ids') j;
