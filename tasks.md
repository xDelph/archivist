# tasks.md — Archivist Full Implementation

Stack: Rust · Vercel (serverless functions) · Neon (Postgres serverless) · sqlx · Direct sync write

---

## Phase 0 — Project Scaffold

- [ ] **0.1** Init Cargo workspace (`cargo init --name archivist`) with `Cargo.toml` dependencies:
  - `axum` (or `vercel_runtime`) for HTTP handlers
  - `sqlx` with features `postgres`, `runtime-tokio-native-tls`, `macros`, `migrate`
  - `serde` / `serde_json`
  - `hmac` + `sha2` (signature verification)
  - `tokio` (async runtime)
  - `anyhow` / `thiserror` (error handling)

- [ ] **0.2** Configure Vercel Rust runtime:
  - `vercel.json` with `builds` using `vercel-rust` (or `@vercel/rust`)
  - Route mapping: `POST /api/slack/events`, `GET /api/health`, `POST /api/admin/backfill`
  - Cron entry for nightly backfill (M2)

- [ ] **0.3** Set up module structure:
  ```
  src/
    lib.rs
    db/
      mod.rs
      pool.rs
      migrations/
    slack/
      mod.rs
      signature.rs
      types.rs
      ingest.rs
      backfill.rs       ← M2
      tests/
        signature_tests.rs
        ingest_tests.rs
        backfill_tests.rs ← M2
    api/
      mod.rs
      events.rs
      health.rs
      admin.rs
  api/
    slack/events.rs     ← Vercel function entry points
    health.rs
    admin/backfill.rs
  ```

- [ ] **0.4** Configure `DATABASE_URL` in Vercel env + local `.env` (gitignored):
  - `DATABASE_URL` (Neon connection string, SSL mode required: `?sslmode=require`)
  - `SLACK_SIGNING_SECRET`
  - `SLACK_BOT_TOKEN`
  - `ADMIN_TOKEN` (shared secret for `/api/admin/backfill`)

- [ ] **0.5** Install `sqlx-cli` and initialize migrations directory:
  ```bash
  cargo install sqlx-cli --no-default-features --features postgres
  sqlx migrate add init
  ```

---

## Phase 1 — Database (M1)

- [ ] **1.1** Write migration `001_init.sql` — create tables:
  - `slack_events(event_id PK, team_id, event_time, payload_json jsonb, received_at)`
  - `messages(id, team_id, channel_id, ts, thread_ts, user_id, text, subtype, edited_ts, deleted, raw_json jsonb, created_at, updated_at)` — unique on `(channel_id, ts)`
  - `reactions(id, team_id, channel_id, message_ts, user_id, reaction_name, event_ts)` — unique on `(team_id, channel_id, message_ts, user_id, reaction_name)`
  - Add indexes: `messages(channel_id, ts)`, `messages(thread_ts)`, `slack_events(event_id)`

- [ ] **1.2** `db/pool.rs` — connection pool:
  - `PgPool` via `sqlx::postgres::PgPoolOptions`
  - Max connections tuned for Neon serverless (keep low, e.g. 5)
  - Pool stored as shared state (e.g. `Arc<PgPool>` or Vercel function-level lazy init)

- [ ] **1.3** `db/mod.rs` — repository functions:
  - `event_exists(pool, event_id) -> bool`
  - `insert_slack_event(pool, event_id, team_id, event_time, payload_json)`
  - `upsert_message(pool, msg: &MessageRecord)`
  - `insert_reaction(pool, reaction: &ReactionRecord)` (ON CONFLICT DO NOTHING)
  - ✅ Tests in `db/tests.rs` using a test DB or `sqlx::test` macro

---

## Phase 2 — Slack Signature Verification (M1)

- [ ] **2.1** `slack/signature.rs` — `verify_signature(secret, timestamp, raw_body, signature) -> Result<(), SignatureError>`:
  - Reject if `|now - timestamp| > 300s`
  - Compute `"v0:{timestamp}:{raw_body}"`
  - HMAC-SHA256 with `signing_secret`
  - Constant-time compare with `signature` header value

- [ ] **2.2** Tests in `slack/tests/signature_tests.rs`:
  - Valid signature passes
  - Invalid signature returns error
  - Replayed timestamp (>300s) returns error
  - Tampered body returns error

---

## Phase 3 — Slack Event Types (M1)

- [ ] **3.1** `slack/types.rs` — serde structs:
  - `SlackEnvelope` (top-level, `type` field dispatch)
  - `UrlVerification { challenge }`
  - `EventCallback { team_id, api_app_id, event_id, event_time, event: SlackEvent }`
  - `SlackEvent` enum: `Message(MessageEvent)`, `ReactionAdded(ReactionEvent)`
  - `MessageEvent { channel, user, text, ts, thread_ts, subtype }`
  - `ReactionEvent { reaction, user, item: ReactionItem, event_ts }`
  - ✅ Tests: round-trip serde for each type using fixture JSON payloads

---

## Phase 4 — Ingest Logic (M1)

- [ ] **4.1** `slack/ingest.rs` — `handle_event(pool, envelope: EventCallback) -> Result<()>`:
  - Step 1: `event_exists` → return early if duplicate
  - Step 2: `insert_slack_event` (raw dedup store)
  - Step 3: match `event.type`:
    - `message` → skip unwanted subtypes (`bot_message`, `channel_join`, etc.) → `upsert_message`
    - `reaction_added` → `insert_reaction`

- [ ] **4.2** Tests in `slack/tests/ingest_tests.rs`:
  - First occurrence of event_id is processed
  - Duplicate event_id is ignored (idempotent)
  - Message upsert on edit (`edited_ts` updated)
  - Bot message subtype is ignored
  - Reaction insert succeeds; duplicate is silently ignored

---

## Phase 5 — API Handlers (M1)

- [ ] **5.1** `api/events.rs` — `POST /api/slack/events`:
  - Extract raw body bytes (before deserialization — needed for signature check)
  - Extract `X-Slack-Signature` and `X-Slack-Request-Timestamp` headers
  - Call `verify_signature` → 401 on failure
  - Deserialize envelope
  - If `url_verification` → return `{"challenge": "..."}` immediately
  - If `event_callback` → call `handle_event` → return 200

- [ ] **5.2** `api/health.rs` — `GET /api/health`:
  - Return `{"ok": true}` with 200

- [ ] **5.3** `api/admin.rs` — `POST /api/admin/backfill`:
  - Verify `Authorization: Bearer <ADMIN_TOKEN>` → 401 if missing/wrong
  - Trigger backfill (M2)
  - ✅ Tests: missing token → 401, valid token → accepted

- [ ] **5.4** Tests in `api/tests/events_tests.rs`:
  - url_verification challenge round-trip
  - Missing signature header → 401
  - Invalid signature → 401
  - Valid event → 200

---

## Phase 6 — Vercel Wiring (M1)

- [ ] **6.1** Wire each Vercel function entry point in `api/` to call the corresponding handler from `src/api/`

- [ ] **6.2** Handle `PgPool` initialization once per function cold start (lazy static or Vercel init hook)

- [ ] **6.3** Deploy to Vercel preview → configure Slack App Request URL → complete `url_verification` handshake

- [ ] **6.4** Manual smoke test: send a test message in a channel → verify row appears in `messages` table

---

## Phase 7 — Slack Web API Client (M2)

- [ ] **7.1** `slack/backfill.rs` — HTTP client (use `reqwest` with `rustls`):
  - `SlackClient { bot_token, http_client }`
  - `conversations_list(cursor) -> Result<(Vec<Channel>, Option<String>)>` (paginated)
  - `conversations_history(channel_id, oldest, cursor) -> Result<(Vec<Message>, Option<String>)>` (paginated)
  - `conversations_replies(channel_id, ts, cursor) -> Result<(Vec<Message>, Option<String>)>` (paginated)
  - Respect `retry_after` header on 429 (rate limit)

- [ ] **7.2** Tests in `slack/tests/backfill_tests.rs`:
  - Pagination: `has_more=true` triggers next page fetch
  - 429 rate limit is surfaced as error (no panic)
  - Empty channel history returns empty vec

---

## Phase 8 — Backfill Logic (M2)

- [ ] **8.1** Add DB helper: `get_last_archived_ts(pool, channel_id) -> Option<String>` (max `ts` in messages for that channel)

- [ ] **8.2** `slack/backfill.rs` — `run_backfill(pool, slack_client)`:
  - `conversations_list` (all pages) → for each channel:
    - `get_last_archived_ts` → use as `oldest` param
    - `conversations_history` (all pages) → `upsert_message` for each
    - For each message with `thread_ts == ts` (thread parent): `conversations_replies` → `upsert_message` for replies
  - ✅ Tests: full backfill flow with mocked Slack client

- [ ] **8.3** Wire `run_backfill` into `api/admin/backfill.rs` handler
  - Scheduling via GitHub Actions (`.github/workflows/backfill.yml`) — Vercel cron not available on free tier
  - Add `APP_URL` and `ADMIN_TOKEN` to GitHub repo secrets

---

## Phase 9 — Hardening & Documentation

- [ ] **9.1** Add structured logging (`tracing` crate) with `tracing-subscriber` JSON output (Vercel captures stdout)

- [ ] **9.2** Review all `unwrap()`/`expect()` — replace with proper error propagation

- [ ] **9.3** Document all public functions and modules with `///` doc comments

- [ ] **9.4** Final full test run:
  ```bash
  rtk cargo fmt && rtk cargo check && rtk cargo clippy && rtk cargo build && rtk cargo test
  ```

---

## Backlog (post M2)

- M3: Message importance scoring (reactions, unique participants, length, keywords)
- M3: Weekly digest per channel
- M3: Markdown/HTML export for blogging
- Admin UI for channel management / bot membership
- Retention policy (delete messages older than N days)
