# Rollup Runbook

Date: 2026-02-27

## Prerequisites

- Backend migrations applied, including:
  - `20260227201000_add_hot_path_indexes.sql`
  - `20260227203000_add_read_model_tables.sql`
- Environment variables set:
  - `DATABASE_URL` (or `DATABASE_URL_UNPOOLED` for scripts)
  - `READ_MODEL_V2=1` to activate read-model query path

## Worker Runtime

- Aggregation jobs are enqueued by:
  - Slack webhook ingest (`/api/slack/events`)
  - Sync pipeline trigger (`/api/admin/sync`)
- Aggregation jobs are consumed by:
  - `/api/admin/sync/run` worker

## Rebuild (full)

Command:

```bash
cargo run --manifest-path apps/backend/Cargo.toml --bin rebuild_rollups
```

Options:
- `--enqueue-only`: only queue jobs, do not process
- `--no-reset`: keep existing read-model rows before queueing
- `ROLLUP_BATCH_SIZE=200` (env): worker batch size for local processing

## Validate

Command:

```bash
cargo run --manifest-path apps/backend/Cargo.toml --bin validate_rollups -- --sample 300 --strict
```

Options:
- `--sample N`: number of threads to validate
- `--strict`: exit non-zero if mismatches are found

## Repair (single thread)

Command:

```bash
cargo run --manifest-path apps/backend/Cargo.toml --bin repair_rollup -- --channel C123 --ts 1700000000.000000
```

Options:
- `ROLLUP_REPAIR_BATCH_SIZE` (env): per-round processing batch
- `ROLLUP_REPAIR_MAX_ROUNDS` (env): max polling rounds before failing

## Operational checks

1. Queue depth:

```sql
SELECT status, COUNT(*)
FROM aggregation_jobs
GROUP BY status
ORDER BY status;
```

2. Failed jobs:

```sql
SELECT id, channel_id, thread_ts, attempts, last_error, updated_at
FROM aggregation_jobs
WHERE status = 'failed'
ORDER BY updated_at DESC
LIMIT 50;
```

3. Read-model freshness:

```sql
SELECT MAX(computed_at) AS latest_rollup_compute
FROM thread_rollups;
```

## Rollback strategy

- Disable read-model reads immediately by setting `READ_MODEL_V2=0`.
- Keep ingestion + aggregation queue running (safe).
- Investigate and repair with `validate_rollups` + `repair_rollup` / `rebuild_rollups`.
