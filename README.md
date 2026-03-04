# Archivist Monorepo

This repository uses a monorepo layout with Turborepo orchestration.

## Apps

- `apps/backend`: Rust API/backend (Slack ingest, backfill, record APIs)
- `apps/frontend`: Next.js + shadcn frontend

## Packages

- `packages/contracts`: shared contracts/types (reserved)

## Tooling

- Turborepo for task orchestration
- Bun workspaces for JS package management
- Cargo for Rust workspace/package management

## Local Dev

- Backend: `bun run dev:backend` (Vercel dev on `http://localhost:3100`)
- Frontend: `bun run dev:frontend` (Next.js on `http://localhost:3001`)
- Keep port `3000` free when running backend locally: current `vercel_runtime` binds function workers to `127.0.0.1:3000`.
- Backend regression smoke check (API + logs): `bun run check:backend:dev` (or `bun run check:backend:dev -- http://localhost:3100`)

## Release Prep

- Env template: `.env.example`
- Phase 5 runbook: `docs/phase5-deploy-cutover.md`
- Preflight check: `bun run release:preflight`
- Smoke test: `bun run release:smoke -- <backend_url> <frontend_url>`
