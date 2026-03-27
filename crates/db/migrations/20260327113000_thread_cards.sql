CREATE TABLE IF NOT EXISTS thread_cards (
    channel_id TEXT NOT NULL,
    root_ts TEXT NOT NULL,
    author_user_id TEXT,
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

CREATE INDEX IF NOT EXISTS thread_cards_last_activity_idx
ON thread_cards (last_activity_at DESC);

CREATE INDEX IF NOT EXISTS thread_cards_channel_activity_idx
ON thread_cards (channel_id, last_activity_at DESC);

INSERT INTO thread_cards (
    channel_id,
    root_ts,
    author_user_id,
    title,
    preview,
    reply_count,
    participant_count,
    reaction_count,
    file_count,
    root_message_at,
    last_activity_at
)
SELECT
    thread_summaries.channel_id,
    thread_summaries.root_ts,
    messages.user_id,
    CASE
        WHEN btrim(replace(messages.text, E'\r\n', E'\n')) = '' THEN '(no text)'
        WHEN char_length(btrim(replace(messages.text, E'\r\n', E'\n'))) > 700
            THEN left(btrim(replace(messages.text, E'\r\n', E'\n')), 700) || '…'
        ELSE btrim(replace(messages.text, E'\r\n', E'\n'))
    END AS title,
    CASE
        WHEN btrim(replace(messages.text, E'\r\n', E'\n')) = '' THEN '(no text)'
        WHEN char_length(btrim(replace(messages.text, E'\r\n', E'\n'))) > 700
            THEN left(btrim(replace(messages.text, E'\r\n', E'\n')), 700) || '…'
        ELSE btrim(replace(messages.text, E'\r\n', E'\n'))
    END AS preview,
    thread_summaries.reply_count,
    thread_summaries.participant_count,
    thread_summaries.reaction_count,
    thread_summaries.file_count,
    thread_summaries.root_message_at,
    thread_summaries.last_activity_at
FROM thread_summaries
JOIN messages
    ON messages.channel_id = thread_summaries.channel_id
   AND messages.ts = thread_summaries.root_ts
ON CONFLICT (channel_id, root_ts) DO UPDATE
SET author_user_id = EXCLUDED.author_user_id,
    title = EXCLUDED.title,
    preview = EXCLUDED.preview,
    reply_count = EXCLUDED.reply_count,
    participant_count = EXCLUDED.participant_count,
    reaction_count = EXCLUDED.reaction_count,
    file_count = EXCLUDED.file_count,
    root_message_at = EXCLUDED.root_message_at,
    last_activity_at = EXCLUDED.last_activity_at,
    updated_at = now();
