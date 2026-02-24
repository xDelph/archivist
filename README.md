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
GET  /record/threads?sort=&period=&channel=&user=&search=
  → filter + sort in Rust, return HTML fragment (HTMX swaps #threads)
GET  /record/thread?channel_id=&ts=[&search=]
  → fetch messages + files for thread
  → return HTML fragment (loaded lazily by HTMX on first expand)

POST /api/admin/backfill  (bearer-token protected)
  → conversations.list → conversations.history → conversations.replies
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
| `POST /api/admin/backfill` | Trigger channel backfill (bearer-token protected) |
| `GET /record` | **SSR thread viewer** — renders server-side with maud + HTMX |
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
| `ADMIN_TOKEN` | Shared secret for `POST /api/admin/backfill` |
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

Vercel cron is not available on the free tier. Backfill is triggered by a GitHub Actions workflow (`.github/workflows/backfill.yml`) on a nightly schedule (2am UTC). It can also be triggered manually from the GitHub Actions UI.

Add these two secrets to the GitHub repository (`Settings → Secrets and variables → Actions`):

| Secret | Value |
|---|---|
| `APP_URL` | Your deployed Vercel URL, e.g. `https://archivist.vercel.app` |
| `ADMIN_TOKEN` | Same value as the `ADMIN_TOKEN` env var in Vercel |

## Slack App configuration

**Bot Token Scopes:** `channels:read`, `channels:history`, `groups:read`, `groups:history`, `reactions:read`

**Event Subscriptions → Request URL:** `https://<your-project>.vercel.app/api/slack/events`

**Subscribe to bot events:** `message.channels`, `message.groups`, `reaction_added`

Private channels are only archived if the bot is added as a member.
