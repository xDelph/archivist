# Archivist Query Audit (2026-02-27)

Scope:
- `/api/record/threads` (top/week/month)
- `/api/record/thread`
- files/reactions lookups used by dashboard rendering

## Hot Query Inventory

1. `Repository::get_top_threads`
- Current behavior: full-thread aggregate recomputation on each request.
- Cost drivers:
  - repeated CTE over `messages`
  - lateral JSON reaction sums per thread
  - join/group for replies/participants
  - sort on computed score

2. `Repository::get_weekly_ranked_threads`
3. `Repository::get_monthly_ranked_threads`
- Current behavior: same heavy aggregate pattern with period bounds and rank.
- Cost drivers:
  - same expensive base CTE as top threads
  - timestamp casts and ranking over recomputed dataset

4. `Repository::get_thread_messages`
- Access pattern: `WHERE channel_id = ? AND (thread_ts = ? OR ts = ?)` with `ORDER BY ts`.
- Needs index support for `(channel_id, thread_ts, ts)`.

5. `Repository::get_files_for_messages`
- Access pattern: `WHERE channel_id = ? AND message_ts = ANY(?)`.
- Already indexed by `files(channel_id, message_ts)`.

6. Reactions aggregation by message:
- Access pattern in thread rendering and score computations:
  - `WHERE channel_id = ? AND message_ts = ?`
- Needed index added on `reactions(channel_id, message_ts)`.

## Index Changes (Phase 1)

Migration:
- `apps/backend/migrations/20260227201000_add_hot_path_indexes.sql`

Added:
- `idx_messages_channel_thread_ts` on `messages(channel_id, thread_ts, ts)` (partial where `thread_ts IS NOT NULL`)
- `idx_reactions_channel_message_ts` on `reactions(channel_id, message_ts)`

Expected impact:
- Faster thread message scans by thread root.
- Better planner selectivity for reaction fanout lookups.
- Reduced CPU on repeated join/filter paths before read-model cutover.

## EXPLAIN Checklist (Run on Preview/Prod DB)

Run:

```sql
EXPLAIN (ANALYZE, BUFFERS)
SELECT m.ts
FROM messages m
WHERE m.channel_id = 'C123'
  AND m.thread_ts = '1700000000.000000'
ORDER BY m.ts;
```

```sql
EXPLAIN (ANALYZE, BUFFERS)
SELECT reaction_name, COUNT(*)::int
FROM reactions
WHERE channel_id = 'C123'
  AND message_ts = '1700000000.000000'
GROUP BY reaction_name;
```

Validation targets:
- Index Scan or Bitmap Index Scan used for both checks.
- Reduced shared buffer reads compared to pre-index baseline.
- No sequential scans on full `messages` or `reactions` for selective keys.

## Notes

- This phase intentionally does not change API behavior.
- Major latency reduction comes in next phases from read-model precomputation.
