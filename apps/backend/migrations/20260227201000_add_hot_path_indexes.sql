-- Hot-path indexes for dashboard and thread rendering.
-- Safe additive migration: no behavior change, only planner improvements.

CREATE INDEX IF NOT EXISTS idx_messages_channel_thread_ts
    ON messages (channel_id, thread_ts, ts)
    WHERE thread_ts IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_reactions_channel_message_ts
    ON reactions (channel_id, message_ts);
