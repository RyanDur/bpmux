-- metrics reads only the recent velocity window; an index on created_at
-- keeps that query proportional to the window, not to history
CREATE INDEX idx_messages_created_at ON messages(created_at);
