# Archivist Monorepo

This repository uses a monorepo layout with Turborepo orchestration.

## Apps

- `apps/backend`: Rust API/backend (Slack ingest, backfill, record APIs)
- `apps/frontend`: Next.js + shadcn frontend

## Packages

- `packages/contracts`: shared contracts/types (reserved)

## Tooling

- Turborepo for task orchestration
- pnpm workspaces for JS package management
- Cargo for Rust workspace/package management

## Release Prep

- Env template: `.env.example`
- Phase 5 runbook: `docs/phase5-deploy-cutover.md`
- Preflight check: `pnpm run release:preflight`
- Smoke test: `pnpm run release:smoke -- <backend_url> <frontend_url>`
