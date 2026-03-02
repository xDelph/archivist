# Archivist

Slack archiver for public channels (no DMs or private channels), deployed as Rust serverless functions on Vercel. Receives real-time events via the Slack Events API and stores messages, threads, reactions, and file attachments in Neon (Postgres serverless). Files and images are downloaded from Slack and permanently stored in Cloudflare R2 to survive Slack's 90-day free-tier deletion.

## Architecture

```
Slack → POST /api/slack/events
  → verify HMAC-SHA256 signature
  → url_verification: return challenge
  → event_callback: dedup → upsert to DB → 200 OK

GET  /record
  → fetch top-200 threads from DB (first 50 displayed, rest populate filter options)
  → render full HTML page (maud + HTMX) with filter/sort/search bar
GET  /record/weekly?tab=top|week|month
  → render weekly ranking page with tabs:
    top threads (all-time + current-week badge), top week (thread date in current ISO week),
    top month (thread date in current calendar month)
  → fetch top-200 for the selected tab (first 50 visible initially; filters/search use the full 200 client-side)
GET  /record/threads?sort=&period=&channel=&user=&search=
  → filter + sort in Rust, return HTML fragment (HTMX swaps #threads)
GET  /record/thread?channel_id=&ts=[&search=]
  → fetch messages + files for thread
  → return HTML fragment (loaded lazily by HTMX on first expand)

POST /api/admin/sync  (bearer-token protected)
  → triggers a QStash publish to run the worker asynchronously
POST /api/admin/sync/run (bearer-token protected)
  → worker phase 1: backfill messages + enqueue file jobs + enqueue aggregation jobs
POST /api/admin/sync/files (bearer-token protected)
  → worker phase 2: archive queued files to R2
POST /api/admin/sync/aggregate (bearer-token protected)
  → worker phase 3: drain aggregation jobs
GET  /api/health
```

Source layout:

```
src/
  api/          HTTP handler logic (events, health, admin, report, record)
  db/           PgPool setup, repository functions
  render/       Server-side HTML rendering with maud + HTMX
    text.rs     Slack mrkdwn → HTML, demojify (emoji shortcode → Unicode)
    components  Thread cards, avatars, score badges, header
    thread.rs   HTMX fragment: rendered thread messages
    page.rs     Full HTML page for GET /record
  slack/        signature verification, event types, ingest, backfill
    tests/      all tests in separate files (signature, ingest, backfill)
  storage/      Cloudflare R2 client (S3-compatible)
api/            Vercel function entry points (thin wrappers over src/api/)
migrations/     sqlx migrations
public/         Static assets (record.css, modal.js, og.png)
```

## Endpoints

| Endpoint | Description |
|---|---|
| `POST /api/slack/events` | Receive Slack Events API payloads |
| `GET /api/health` | Health check |
| `POST /api/admin/sync` | Trigger endpoint: publishes one worker execution to QStash |
| `POST /api/admin/sync/run` | Worker phase 1: backfill messages + queue files + queue aggregation |
| `POST /api/admin/sync/files` | Worker phase 2: process queued file archival jobs |
| `POST /api/admin/sync/aggregate` | Worker phase 3: drain aggregation jobs |
| `GET /record` | **SSR thread viewer** — renders server-side with maud + HTMX |
| `GET /record/weekly?tab=top|week|month` | SSR ranking tabs (all-time, weekly, monthly) with rank-change badges |
| `GET /record/threads?sort=&period=&channel=&user=&search=` | HTMX fragment: filtered/sorted thread list |
| `GET /record/thread?channel_id=&ts=` | HTMX fragment: lazy-load thread messages |

## Requirements

- Rust (stable)
- [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli): `cargo install sqlx-cli --no-default-features --features postgres,rustls`
- [Vercel CLI](https://vercel.com/docs/cli): `npm i -g vercel`
- A [Neon](https://neon.tech) Postgres database
- A [Cloudflare R2](https://www.cloudflare.com/developer-platform/r2/) bucket with public access enabled
- A Slack App with Events API configured (see below)

## Local setup

```bash
cp .env.example .env
# fill in .env values (DATABASE_URL and DATABASE_URL_UNPOOLED required)

# Run migrations (uses DATABASE_URL_UNPOOLED — Neon's pooler drops prepared statements)
source .env && DATABASE_URL="$DATABASE_URL_UNPOOLED" sqlx migrate run

vercel dev
```

**Regenerate sqlx offline query cache** (only needed after changing SQL queries):
```bash
source .env && DATABASE_URL="$DATABASE_URL_UNPOOLED" cargo sqlx prepare
```

**Unit tests** run fully offline — no database or network needed:
```bash
cargo test
```

## Environment variables

| Variable | Description |
|---|---|
| `DATABASE_URL` | Neon connection string (`postgres://...?sslmode=require`) |
| `DATABASE_URL_UNPOOLED` | Neon direct connection — only for migrations / sqlx prepare |
| `SLACK_SIGNING_SECRET` | From Slack App → Basic Information |
| `SLACK_BOT_TOKEN` | `xoxb-...` from Slack App → OAuth & Permissions |
| `SLACK_USER_TOKEN` | `xoxp-...` — for backfill (full channel history) |
| `ADMIN_TOKEN` | Shared secret for `POST /api/admin/sync` |
| `UPSTASH_QSTASH_TOKEN` | Upstash QStash token used to publish worker calls |
| `UPSTASH_QSTASH_URL` | Upstash QStash base URL (example: `https://qstash.upstash.io`) |
| `UPSTASH_QSTASH_CURRENT_SIGNING_KEY` | Upstash current signing key used to verify request signatures |
| `UPSTASH_QSTASH_NEXT_SIGNING_KEY` | Upstash next signing key used during key rotation |
| `BACKFILL_WORKER_TOKEN` | Bearer token forwarded by QStash to authenticate `/api/admin/sync/run` |
| `BACKFILL_WORKER_URL` | Optional absolute worker URL override (defaults to `<base>/api/admin/sync/run`) |
| `BACKFILL_FILES_WORKER_URL` | Optional absolute URL override for phase-2 worker (`/api/admin/sync/files`) |
| `BACKFILL_AGGREGATE_WORKER_URL` | Optional absolute URL override for phase-3 worker (`/api/admin/sync/aggregate`) |
| `QSTASH_TIMEOUT_SECONDS` | Optional timeout for publish HTTP call to QStash (default: `10`) |
| `AGGREGATION_JOBS_PER_BATCH` | Optional max aggregation jobs per batch while draining (default: `200`) |
| `AGGREGATION_MAX_BATCHES_PER_RUN` | Optional max aggregation batches drained per worker run (default: `200`) |
| `AGGREGATION_RUNNING_LEASE_MINUTES` | Optional timeout to recover stale `running` aggregation jobs (default: `15`) |
| `FILE_BACKFILL_RUNNING_LEASE_MINUTES` | Optional timeout to recover stale `running` file jobs (default: `15`) |
| `CLOUDFLARED_R2_ACCOUNT_ID` | Cloudflare account ID |
| `CLOUDFLARED_R2_ACCESS_KEY` | R2 API token access key |
| `CLOUDFLARED_R2_SECRET_KEY` | R2 API token secret key |
| `CLOUDFLARED_R2_BUCKET` | R2 bucket name (e.g. `archivist-files`) |
| `CLOUDFLARED_R2_PUBLIC_URL` | Public R2 URL (e.g. `https://pub-xxx.r2.dev`) |
| `SLACK_WORKSPACE_URL` | Workspace URL (e.g. `devwithai.slack.com`) — shown as "Open Slack" link |

## Development workflow

After every feature or fix, run the full check sequence:

```bash
rtk cargo fmt && rtk cargo check && rtk cargo clippy && rtk cargo build && rtk cargo test
```

Run a single test: `cargo test test_name`
Run a module's tests: `cargo test slack::`

## SQL query probe (read-model)

Use this script to validate read-model SQL query behaviour/results directly on the DB:

```bash
scripts/db/probe_read_model_queries.sh --tab all --limit 50
```

With optional API JSON validation (detects runtime mapping issues like non-JSON error bodies):

```bash
scripts/db/probe_read_model_queries.sh --tab all --limit 50 --api-url http://localhost:3100
```

Useful variants:

```bash
scripts/db/probe_read_model_queries.sh --tab recent --limit 20
scripts/db/probe_read_model_queries.sh --tab week --limit 100 --explain
```

## Backend dev smoke check

Use this script to validate local backend behavior end-to-end (health + each tab + one thread payload + new log lines):

```bash
pnpm run check:backend:dev
```

Optional arguments:

```bash
pnpm run check:backend:dev -- http://localhost:3100
pnpm run check:backend:dev -- http://localhost:3100 ./logs/app.log
```

## Deployment

**First deploy:**
```bash
vercel deploy --prod
```

Set all environment variables (Vercel dashboard or CLI) before the first request hits the function:
```bash
vercel env add DATABASE_URL
vercel env add SLACK_SIGNING_SECRET
vercel env add SLACK_BOT_TOKEN
vercel env add ADMIN_TOKEN
vercel env add UPSTASH_QSTASH_TOKEN
vercel env add UPSTASH_QSTASH_URL
vercel env add UPSTASH_QSTASH_CURRENT_SIGNING_KEY
vercel env add UPSTASH_QSTASH_NEXT_SIGNING_KEY
vercel env add BACKFILL_WORKER_TOKEN
vercel env add SLACK_WORKSPACE_URL   # optional — enables "Open Slack" link
```

**Subsequent deploys** (after code changes):
```bash
vercel deploy --prod
```

**Verify the deployment** is live:
```bash
curl https://<your-project>.vercel.app/api/health
# → {"ok":true}
```

**Complete the Slack URL verification** — after deploying, paste the URL into the Slack App's Event Subscriptions page. Slack will send a `url_verification` challenge; the handler echoes it back automatically.

**Pool behaviour:** the `PgPool` is initialised once per Lambda instance via `OnceCell` and reused across warm invocations. Cold starts create a new pool (≤5 connections, Neon pooler).

## Backfill scheduling

Backfill scheduling uses QStash:

- `POST /api/admin/sync` is a trigger-only endpoint.
- It publishes one call to `POST /api/admin/sync/run` via QStash.
- `POST /api/admin/sync/run` runs message backfill then publishes phase 2.
- `POST /api/admin/sync/files` processes queued file archival jobs then publishes phase 3.
- `POST /api/admin/sync/aggregate` drains aggregation jobs.
- QStash handles async delivery and retries.

## Weekly ranking data

`thread_weekly_scores` is updated in two paths:

- Real-time ingest: message and reaction events refresh the current week row for the touched thread.
- Scheduled backfill runs: each processed touched thread root gets one weekly-score upsert.

One-shot historical seeding:

```bash
cargo run --bin compute_weekly_scores
```

This script loads `.env`, uses `DATABASE_URL_UNPOOLED`, and can be re-run safely (`ON CONFLICT DO UPDATE`).

## Record viewer (`GET /record`)

The viewer is a server-rendered HTMX app. Behaviour notes:

- **Header controls** — includes ranking tabs plus local UI preferences:
  - Theme: `Dark` / `Light`
  - Card density: `Normal` / `Compact`
  Preferences are saved in `localStorage` and reused on next visit.
- **Filter bar** wraps gracefully at narrow widths; fully responsive on mobile.
- **Search** — live with 300 ms debounce; pressing Enter also triggers the search (handled via HTMX, no full-page navigation). All filtering, sorting and search run in Rust against a cached dataset — zero DB round-trips per keystroke.
- **User mentions** in thread previews are resolved to display names (loaded server-side alongside threads).
- **In-process cache** — `get_top_threads`, `get_all_users` and `get_all_channels` are cached in-process for 5 minutes (TTL). On warm Lambda instances, search and filter requests hit zero DB queries.
- **Slack-encoded entities** (`&amp;`, `&lt;`, `&gt;`) are decoded before re-escaping to prevent double-encoding.

## Slack App configuration

**Bot Token Scopes:** `channels:read`, `channels:history`, `groups:read`, `groups:history`, `reactions:read`

**Event Subscriptions → Request URL:** `https://<your-project>.vercel.app/api/slack/events`

**Subscribe to bot events:** `message.channels`, `message.groups`, `reaction_added`

Private channels are only archived if the bot is added as a member.
