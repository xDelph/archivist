# Phase 5 Deploy + Cutover Runbook

This runbook completes the remaining Phase 5 items for the monorepo migration.

Prepared assets in repo:
- Env template: `.env.example`
- Local preflight: `scripts/release/preflight-check.sh`
- Local/CI smoke test: `scripts/release/smoke-test.sh`
- Manual smoke workflow: `.github/workflows/release-smoke.yml`

## 0. Prerequisites

- Access to Vercel team/project settings
- Access to Slack App configuration
- Access to GitHub repo secrets
- Local tools: `pnpm`, `cargo`, `vercel` CLI (optional but recommended)

## 1. Create Two Vercel Projects

Create two projects in Vercel:

1. Backend project
- Name: `archivist-backend` (or your preferred stable name)
- Root Directory: `apps/backend`
- Framework preset: `Other`

2. Frontend project
- Name: `archivist-frontend`
- Root Directory: `apps/frontend`
- Framework preset: `Next.js`

## 2. Configure Environment Variables

Use `.env.example` as source of truth.

Backend project required vars:
- `DATABASE_URL`
- `DATABASE_URL_UNPOOLED`
- `SLACK_SIGNING_SECRET`
- `SLACK_BOT_TOKEN` (or `SLACK_USER_TOKEN` fallback if required by your ops model)
- `SLACK_WORKSPACE_URL`
- `ADMIN_TOKEN`

Backend project file archival vars (if archival is enabled):
- `CLOUDFLARED_R2_ACCOUNT_ID`
- `CLOUDFLARED_R2_ACCESS_KEY`
- `CLOUDFLARED_R2_SECRET_KEY`
- `CLOUDFLARED_R2_BUCKET`
- `CLOUDFLARED_R2_PUBLIC_URL`

Frontend project required vars:
- `NEXT_PUBLIC_API_BASE_URL=https://<your-backend-domain>`

## 3. Update GitHub Secrets

Set these repo secrets:
- `ADMIN_TOKEN`
- `BACKEND_APP_URL` (preferred) or `APP_URL` fallback

Optional for future deploy automation:
- `VERCEL_TOKEN`
- `VERCEL_ORG_ID`
- `VERCEL_PROJECT_ID_BACKEND`
- `VERCEL_PROJECT_ID_FRONTEND`

## 4. Deploy Order

Deploy backend first, then frontend.

Suggested flow:
1. Deploy backend to preview
2. Validate backend health and record API
3. Deploy frontend preview with `NEXT_PUBLIC_API_BASE_URL` targeting backend preview
4. Validate full UI parity
5. Promote backend to production
6. Update frontend production env if needed
7. Promote frontend to production

## 5. Run Verification

Local preflight (checks required env keys):

```bash
pnpm run release:preflight
```

Smoke test deployed pair:

```bash
pnpm run release:smoke -- https://<backend-domain> https://<frontend-domain>
```

Or run from GitHub Actions:
- Workflow: `Release Smoke Test`
- Inputs: `backend_url`, `frontend_url`

## 6. Slack Request URL Cutover

After backend production is validated:

1. Slack App -> Event Subscriptions
2. Update Request URL to:
- `https://<backend-domain>/api/slack/events`
3. Save and confirm challenge handshake succeeds
4. Send a test Slack message and confirm ingestion in Archivist

Important behavior:
- Changing only the Request URL does **not** invalidate the app install.
- Reinstall/admin reapproval is typically **not** required unless scopes/permissions changed.

## 7. Frontend Cutover + Rollback

Cutover:
1. Point production frontend domain to new `archivist-frontend` deployment
2. Validate:
- dashboard loads
- tabs and filters work
- thread expansion works
- file modal preview works

Rollback:
1. Restore previous frontend domain target/deployment
2. Keep backend as-is (or roll back backend deployment if regression is backend-related)
3. Re-run smoke tests to confirm restored behavior

## 8. Definition of Done (Phase 5)

- [ ] `5.1` Two Vercel projects created (backend + frontend)
- [ ] `5.2` Frontend env wired to backend base URL
- [ ] `5.3` Parity validated against `/record`
- [ ] `5.4` Production traffic switched to new frontend
- [ ] `5.5` Legacy UI path retained temporarily for rollback
