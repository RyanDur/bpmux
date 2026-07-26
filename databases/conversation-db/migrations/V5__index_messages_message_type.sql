-- chat is a fraction of the log; search candidates are chat-only by contract
CREATE INDEX IF NOT EXISTS idx_messages_message_type ON messages(message_type);
