# Weekly / Monthly Ranking — Design Notes

## Core idea: event-driven weekly scores (Option B)

Scores are written **lazily** — only when a thread receives an event (new message or reaction). The backfill (already running hourly) also upserts weekly scores for every thread it processes during its normal channel sweep. No dedicated snapshot job, no extra DB round-trips beyond what already happens.

Each row stores two scores for a given (thread, week):

- `score_week` — delta score: only reactions and replies received **within this week**
- `score_total` — cumulative all-time score at the time of the last update

## Schema

```sql
CREATE TABLE thread_weekly_scores (
  week_start   DATE      NOT NULL,  -- ISO Monday, e.g. 2026-02-23
  channel_id   TEXT      NOT NULL,
  thread_ts    TEXT      NOT NULL,
  score_week   INT       NOT NULL,  -- score earned only within this week
  score_total  INT       NOT NULL,  -- all-time score as of last update
  updated_at   TIMESTAMP NOT NULL DEFAULT NOW(),
  PRIMARY KEY (week_start, channel_id, thread_ts)
);
CREATE INDEX ON thread_weekly_scores (week_start, score_week DESC);
```

## When rows are written

Two triggers — both upsert on `(week_start, channel_id, thread_ts)`:

1. **Ingest event** (message or reaction arrives for a thread): compute both scores and upsert immediately.
2. **Hourly backfill**: as it processes each channel's threads, upsert a row for any thread it touches. Threads reached but with no activity this week get `score_week = 0` with a fresh `score_total`.

Threads with **zero activity this week** and not reached by the backfill have no row for the current week. They do not appear in the weekly or monthly ranking. The all-time ranking is unaffected.

## Score computation (both triggers)

```sql
-- score_week: events within current week only
-- (reactions created this week + replies created this week, same weighted formula as get_top_threads)

-- score_total: all-time (same formula as get_top_threads)
```

## Queries

**All-time top threads** (existing, unchanged):
```sql
-- existing get_top_threads query, no change
-- optional: LEFT JOIN thread_weekly_scores for current week to show score_week badge
```

**Weekly ranking** (hot this week):
```sql
SELECT channel_id, thread_ts, score_week, score_total
FROM thread_weekly_scores
WHERE week_start = date_trunc('week', CURRENT_DATE)
ORDER BY score_week DESC
LIMIT 50;
```

**Position change** (this week vs last week):
```sql
WITH cur AS (
  SELECT *, RANK() OVER (ORDER BY score_week DESC) AS rank
  FROM thread_weekly_scores
  WHERE week_start = date_trunc('week', CURRENT_DATE)
),
prev AS (
  SELECT channel_id, thread_ts,
         RANK() OVER (ORDER BY score_week DESC) AS rank
  FROM thread_weekly_scores
  WHERE week_start = date_trunc('week', CURRENT_DATE) - INTERVAL '7 days'
)
SELECT cur.*, prev.rank AS prev_rank
FROM cur LEFT JOIN prev USING (channel_id, thread_ts)
ORDER BY cur.rank;
```

- No `prev_rank` → **NEW** badge
- Not in `cur` → dropped out (not shown)

**Monthly ranking** (sum of weekly deltas within the month):
```sql
SELECT channel_id, thread_ts, SUM(score_week) AS score_month
FROM thread_weekly_scores
WHERE week_start >= date_trunc('month', CURRENT_DATE)
  AND week_start <  date_trunc('month', CURRENT_DATE) + INTERVAL '1 month'
GROUP BY channel_id, thread_ts
ORDER BY score_month DESC
LIMIT 50;
```

## Display — three tabs

A new `/record/weekly` page with three tabs:

| Tab | Ranked by | Position change |
|---|---|---|
| **Top threads** | all-time `score` (existing) | optional `score_week` badge |
| **This week** | `score_week` for current week | vs previous week rank |
| **This month** | `SUM(score_week)` for current month | vs previous month rank |

Same thread card design as `/record`. Badges: **↑3** (green), **↓2** (red), **NEW** (neutral).

## Backfill behaviour (DB cost)

One `INSERT ... ON CONFLICT DO UPDATE` per thread already fetched by the backfill. No additional SELECT, no extra API call. Cost is proportional to threads the backfill processes, not the total thread count.

## One-shot historical backfill script

A dedicated Rust binary (`src/bin/compute_weekly_scores.rs`) to seed `thread_weekly_scores` from existing data. Run once after the migration.

**Algorithm:**
1. Fetch all distinct `(channel_id, thread_ts)` pairs from the `messages` table
2. For each thread, fetch all its messages and reactions with their `created_at` timestamps
3. Group events by ISO week (`date_trunc('week', created_at)`)
4. For each (thread, week): compute `score_week` (events in that week) and `score_total` (all events up to end of that week)
5. Bulk insert into `thread_weekly_scores`

**Notes:**
- Can be run with `cargo run --bin compute_weekly_scores`
- Loads `.env` automatically like `backfill_local`
- Uses `DATABASE_URL_UNPOOLED` (same as migrations — avoids Neon pooler)
- Safe to re-run: all inserts are `ON CONFLICT DO UPDATE`
- Expected runtime: a few seconds to a few minutes depending on archive size
- Progress logging per channel so it can be interrupted and resumed (idempotent)

## Edge cases

- **Reaction removed**: next ingest event for that thread updates `score_week` downward.
- **Week boundary**: `week_start = date_trunc('week', NOW())` (ISO Monday). Always consistent.
- **Partial weeks** in monthly view: first and last week of a month may start/end outside the month. Included in full — acceptable approximation.
- **Score formula change**: old weeks remain consistent (computed with the formula in effect at the time).

## Storage estimate

~200 active threads/week × 52 weeks = ~10k rows/year. Negligible.
