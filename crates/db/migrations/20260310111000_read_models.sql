CREATE TABLE IF NOT EXISTS search_documents (
    channel_id TEXT NOT NULL,
    root_ts TEXT NOT NULL,
    message_ts TEXT NOT NULL,
    title TEXT,
    body TEXT NOT NULL,
    document TSVECTOR NOT NULL,
    message_occurred_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (channel_id, message_ts),
    FOREIGN KEY (channel_id, message_ts)
        REFERENCES messages (channel_id, ts)
        ON DELETE CASCADE,
    FOREIGN KEY (channel_id, root_ts)
        REFERENCES messages (channel_id, ts)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS search_documents_document_idx
ON search_documents
USING GIN (document);

CREATE INDEX IF NOT EXISTS search_documents_channel_ts_idx
ON search_documents (channel_id, root_ts, message_occurred_at DESC);

CREATE TABLE IF NOT EXISTS thread_summaries (
    channel_id TEXT NOT NULL,
    root_ts TEXT NOT NULL,
    title TEXT NOT NULL,
    preview TEXT NOT NULL,
    reply_count BIGINT NOT NULL,
    participant_count BIGINT NOT NULL,
    reaction_count BIGINT NOT NULL,
    file_count BIGINT NOT NULL,
    root_message_at TIMESTAMPTZ NOT NULL,
    last_activity_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (channel_id, root_ts),
    FOREIGN KEY (channel_id, root_ts)
        REFERENCES messages (channel_id, ts)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS thread_summaries_last_activity_idx
ON thread_summaries (last_activity_at DESC);

CREATE INDEX IF NOT EXISTS thread_summaries_channel_activity_idx
ON thread_summaries (channel_id, last_activity_at DESC);
