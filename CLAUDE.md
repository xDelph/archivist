# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

**Archivist** — a Slack archiver for public and private channels (no DMs) deployed as Rust serverless functions on Vercel. It receives real-time events via Slack Events API and stores messages/threads/reactions in **Neon (Postgres serverless)** via **sqlx**.

## Git commits

**Never** add `Co-Authored-By` trailers to commit messages.

## Mandatory Workflow

After every feature or fix, update `README.md` to reflect any changes, then run in order:

```bash
rtk cargo fmt && rtk cargo check && rtk cargo clippy && rtk cargo build && rtk cargo test
```

Clippy is configured to deny all warnings via `[lints.clippy] all = "deny"` in `Cargo.toml` — no need for `-- -D warnings`.

## Testing

- Tests live in **separate files** (e.g., `src/slack/tests/mod.rs`), never inline in the module.
- Every feature or bug fix **must** include a new test for non-regression.
- All unit tests run **fully offline** — no DB or network. Use `InMemoryRepository` for DB-dependent logic.
- Run a single test: `cargo test test_name`
- Run tests for a module: `cargo test db::`

## sqlx offline cache

`SQLX_OFFLINE=true` is set in `.cargo/config.toml` — builds never need a live DB.
Only regenerate `.sqlx/` after changing a SQL query:
```bash
source .env && DATABASE_URL="$DATABASE_URL_UNPOOLED" cargo sqlx prepare
```
`DATABASE_URL_UNPOOLED` is required because Neon's pooler (`DATABASE_URL`) drops prepared statements and breaks `cargo sqlx prepare`.

## Architecture

The app is **modular** — each concern lives in its own module:

| Module | Responsibility |
|---|---|
| `api/slack/events` | Vercel handler: `POST /api/slack/events` — ack 200 immediately |
| `api/health` | `GET /api/health` |
| `api/admin/backfill` | `POST /api/admin/backfill` (protected) |
| `slack::signature` | HMAC-SHA256 signature verification (anti-replay: 300s window) |
| `slack::events` | Event dispatch: `url_verification` challenge, `event_callback` routing |
| `slack::ingest` | Dedup via `event_id`, upsert messages, insert reactions |
| `slack::backfill` | Slack Web API calls: `conversations.list/history/replies` |
| `db` | DB access layer (schema: `slack_events`, `messages`, `reactions`) |

### Request flow

```
Slack → POST /api/slack/events
  → verify signature (401 if invalid)
  → if url_verification → return {challenge}
  → if event_callback → dedup event_id → upsert to DB → 200 OK
```

### Key constraints

- **Ack must be immediate** (200 before any heavy processing) — offload to queue or write fast.
- Slack retries if no 200 within a few seconds.
- Private channels: only archived if the bot is a member.
- Message uniqueness key: `(channel_id, ts)`.
- Reaction uniqueness: `(team_id, channel_id, message_ts, user_id, reaction_name)`.

## Environment Variables

```
DATABASE_URL              # Neon pooled — used at runtime
DATABASE_URL_UNPOOLED     # Neon direct — used only for cargo sqlx prepare / migrations
SLACK_SIGNING_SECRET
SLACK_BOT_TOKEN
ADMIN_TOKEN
```

## DB Schema (minimal)

Three tables: `slack_events` (dedup store), `messages`, `reactions` — see `project.md §8` for full field list.

## Vercel Deployment

Rust on Vercel uses the community Rust runtime. Functions are compiled to WASM or native depending on runtime choice. Keep handler cold-start weight minimal.
