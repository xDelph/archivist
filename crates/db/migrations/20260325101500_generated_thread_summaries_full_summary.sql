ALTER TABLE generated_thread_summaries
ADD COLUMN IF NOT EXISTS full_summary TEXT;

UPDATE generated_thread_summaries
SET full_summary = summary
WHERE full_summary IS NULL;
