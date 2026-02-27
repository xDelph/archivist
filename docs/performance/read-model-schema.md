# Read-Model Schema (Phase 2)

Date: 2026-02-27

Migration:
- `apps/backend/migrations/20260227203000_add_read_model_tables.sql`

## Tables

1. `thread_rollups`
- Grain: one row per thread root (`channel_id`, `thread_ts`)
- Contains counters required by dashboard cards and ranking:
  - `reaction_count_total`
  - `reply_count_total`
  - `participant_count_total`
  - `file_count_total`
  - `has_files`
  - `score_total`
- Stores root metadata for rendering/filtering:
  - `root_user_id`, `root_text`, `root_created_at`

2. `thread_period_scores`
- Grain: one row per (`period_kind`, `period_start`, `channel_id`, `thread_ts`)
- Supports fast week/month ranking with `score_period`
- `period_kind`: `week` or `month`

3. `channel_daily_rollups`
- Grain: one row per (`day`, `channel_id`)
- Supports activity graph:
  - `thread_count`
  - `message_count`

4. `workspace_overview_rollups`
- Snapshot table for overview cards.
- Current usage key: `snapshot_key = 'latest'`.

5. `aggregation_jobs`
- Queue for async recomputation.
- Dedupe key prevents duplicate queued work for same thread.
- Retries controlled by `attempts` and `max_attempts`.

## Rebuild and Validation Story

- Source-of-truth remains `messages`, `reactions`, `files`, `users`, `channels`.
- Read-model rows are derivable and disposable.
- A full rebuild can truncate read-model tables and recompute deterministically.

## Rollback/Forward

- Forward: apply migration, enqueue/recompute jobs, switch reads to rollups.
- Rollback: keep migration in place, switch reads back to legacy SQL path.
- No source data mutation is required for rollback.
