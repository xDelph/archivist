# Archivist

Slack archiver for public and private channels (no DMs), deployed as Rust serverless functions on Vercel. Receives real-time events via the Slack Events API and stores messages, threads, and reactions in Neon (Postgres serverless).

## Architecture

```
Slack → POST /api/slack/events
  → verify HMAC-SHA256 signature
  → url_verification: return challenge
  → event_callback: dedup → upsert to DB → 200 OK

POST /api/admin/backfill  (bearer-token protected)
  → conversations.list → conversations.history → conversations.replies
GET  /api/health
```

Source layout:

```
src/
  api/          HTTP handler logic (events, health, admin)
  db/           PgPool setup, repository functions, migrations
  slack/        signature verification, event types, ingest, backfill
    tests/      all tests in separate files (signature, ingest, backfill)
api/            Vercel function entry points (thin wrappers over src/api/)
migrations/     sqlx migrations
```

## Requirements

- Rust (stable)
- [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli): `cargo install sqlx-cli --no-default-features --features postgres`
- [Vercel CLI](https://vercel.com/docs/cli): `npm i -g vercel`
- A [Neon](https://neon.tech) Postgres database
- A Slack App with Events API configured (see below)

## Local setup

```bash
cp .env.example .env
# fill in .env values

sqlx database create
sqlx migrate run

vercel dev
```

## Environment variables

| Variable | Description |
|---|---|
| `DATABASE_URL` | Neon connection string (`postgres://...?sslmode=require`) |
| `SLACK_SIGNING_SECRET` | From Slack App → Basic Information |
| `SLACK_BOT_TOKEN` | `xoxb-...` from Slack App → OAuth & Permissions |
| `ADMIN_TOKEN` | Shared secret for `POST /api/admin/backfill` |

## Development workflow

After every feature or fix, run the full check sequence:

```bash
rtk cargo fmt && rtk cargo check && rtk cargo clippy && rtk cargo build && rtk cargo test
```

Run a single test: `cargo test test_name`
Run a module's tests: `cargo test slack::`

## Deployment

```bash
vercel deploy
```

Set all environment variables in the Vercel dashboard or via `vercel env add`.

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
