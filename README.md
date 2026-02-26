# Archivist Monorepo

This repository uses a monorepo layout with Turborepo orchestration.

## Apps

- `apps/backend-rust`: Rust API/backend (Slack ingest, backfill, record APIs)
- `apps/frontend-v0`: Next.js + shadcn frontend

## Packages

- `packages/contracts`: shared contracts/types (reserved)

## Tooling

- Turborepo for task orchestration
- pnpm workspaces for JS package management
- Cargo for Rust workspace/package management
