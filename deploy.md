# Deploying a Rust Slack Archiveur on Vercel (without `@vercel/rust`)

`@vercel/rust` (the old community runtime/builder) is no longer the recommended path.  
You can deploy Rust on Vercel using the **official native Rust runtime for Vercel Functions**, with **no custom runtime**.  
Docs: https://vercel.com/docs/functions/runtimes/rust

---

## 1) What changes conceptually

### ✅ Supported on Vercel
- **HTTP serverless functions** written in Rust (handlers), e.g. `POST /api/slack/events`

### ❌ Not supported as-is
- A long-running Rust web server (Axum/Actix listening on a port) as a persistent process

So your Slack “archiveur” should be implemented as Vercel Functions (HTTP handlers), not a daemon.

---

## 2) Minimal repo layout (official)

Create this structure at the repo root:

```
/
  Cargo.toml
  api/
    slack_events.rs
    health.rs
```

Vercel maps `api/<name>.rs` to the route `/api/<name>`.

Examples:
- `api/slack_events.rs` → `POST https://<project>.vercel.app/api/slack_events`
- `api/health.rs` → `GET  https://<project>.vercel.app/api/health`

> Note: If you want `/api/slack/events`, you’ll typically use an extra routing layer (see §7).

---

## 3) `Cargo.toml` (one `[[bin]]` per function)

Vercel compiles each Rust function as a separate binary.  
Define a `[[bin]]` entry for each file in `api/`.

Example:

```toml
[package]
name = "slack-archiver"
version = "0.1.0"
edition = "2021"

[dependencies]
vercel_runtime = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[[bin]]
name = "slack_events"
path = "api/slack_events.rs"

[[bin]]
name = "health"
path = "api/health.rs"
```

---

## 4) Rust function skeleton (`api/health.rs`)

```rust
use vercel_runtime::{run, service_fn, Body, Error, Request, Response, StatusCode};

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(handler)).await
}

async fn handler(_req: Request) -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("ok"))?)
}
```

---

## 5) Slack Events endpoint skeleton (`api/slack_events.rs`)

This file will handle:
- Slack URL verification (`type = "url_verification"`)
- Event callbacks (`type = "event_callback"`)
- (Recommended) Slack request signature verification via headers:
  - `X-Slack-Signature`
  - `X-Slack-Request-Timestamp`

High-level flow:
1. Validate Slack signature (HMAC)
2. If `url_verification` → return `{ "challenge": "..." }`
3. If `event_callback` → **ACK 200 immediately**, then store/process asynchronously

> Slack signature verification algorithm:
> - Reject if timestamp too old (anti-replay)
> - Build `basestring = "v0:{timestamp}:{raw_body}"`
> - Compute `HMAC_SHA256(signing_secret, basestring)`
> - Compare with `X-Slack-Signature`

Slack requires fast ACK to avoid retries.

---

## 6) Environment variables on Vercel

Set these in **Vercel → Project → Settings → Environment Variables**:

- `SLACK_SIGNING_SECRET` — from Slack App credentials
- `SLACK_BOT_TOKEN` — `xoxb-...` from “OAuth & Permissions” after install
- `DATABASE_URL` — your DB connection string
- (optional) queue/caching:
  - `UPSTASH_REDIS_REST_URL`
  - `UPSTASH_REDIS_REST_TOKEN`

---

## 7) Routing: getting `/api/slack/events`

By default, `api/slack_events.rs` maps to `/api/slack_events`.

If you *must* expose `/api/slack/events`, common options are:

### Option A — keep the route simple
Use `/api/slack_events` as your Slack Request URL.

### Option B — add a small framework router (Next.js) just for rewrites
If your repo already uses Next.js, you can add a rewrite to map:
- `/api/slack/events` → `/api/slack_events`

(Exact configuration depends on your framework setup.)

---

## 8) Deploy

### Recommended: Git deployment
1. Push repo to GitHub
2. Import into Vercel
3. Deploy (Preview + Production)

### CLI
```bash
vercel login
vercel deploy
```

Vercel will detect the Rust functions in `api/*.rs` and build them using the native runtime.

---

## 9) Common gotchas

### 9.1 Function not triggered
- Make sure `api/` is at the repo root (not `src/api/`, not `pages/api/`).

### 9.2 `vercel dev` routing oddities (when using a framework)
Some framework dev servers may intercept `/api/*` locally.  
If this happens, test on a deployed Preview URL to confirm behavior.

### 9.3 Slow Slack ACK
Slack retries if you don’t respond quickly.  
Store the event fast (DB/queue) and process later.

---

## 10) Next steps for the Slack Archiveur
- Implement request signature verification
- Store events/messages/reactions in DB
- Add periodic backfill using Slack Web API (`conversations.history`, `conversations.replies`)
- Add “importance scoring” for highlights (reactions, participant count, etc.)
