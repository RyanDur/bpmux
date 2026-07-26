-- chat is 15% of the log; search candidates are chat-only by contract
CREATE INDEX idx_messages_message_type ON messages(message_type);
