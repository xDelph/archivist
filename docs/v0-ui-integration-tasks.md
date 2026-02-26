# Archivist Monorepo Migration Tasks (Rust backend + v0 frontend)

Goal: move Archivist to a monorepo where Rust is the backend app and v0/shadcn is the frontend app.

Selected decisions:
- Tooling: Turborepo
- Architecture: Option A (`apps/` + `packages/`)

## Mandatory Inputs (Requested)

- [ ] Start from branch `feat/monorepo`, created from `develop`.
- [ ] Store a full pre-migration snapshot under `/backup` before moving files.
- [ ] Rust codebase becomes the backend app.
- [ ] v0 codebase becomes the frontend app.

## Monorepo Architecture Options

### Option A (Recommended): `apps/` + `packages/`

```text
/
  apps/
    backend-rust/
    frontend-v0/
  packages/
    contracts/          # shared API schemas/types (optional)
  backup/
```

- Best long-term separation of concerns.
- Easy to scale with new apps/workers.
- Clean CI split per app.

### Option B: `services/` layout

```text
/
  services/
    api-rust/
    web-v0/
  shared/
  backup/
```

- Similar to Option A, better naming if you think in deployable services.
- Slightly less standard in JS toolchains than `apps/`.

### Option C: Minimal move (fastest)

```text
/
  backend/              # move current root backend here
  frontend/             # move v0 here
  backup/
```

- Fast migration, fewer paths to touch.
- Less explicit if you later add many packages/tools.

## Ordered Migration Plan

## Phase 0 - Branch + Backup

- [x] 0.1 Checkout `develop` and create `feat/monorepo`.
- [x] 0.2 Create `/backup` at repo root.
- [x] 0.3 Copy current backend tree and current `v0/` into `/backup` before any move.
- [x] 0.4 Verify backup integrity (spot check important files: `Cargo.toml`, `src/`, `api/`, `migrations/`, `v0/`).

## Phase 1 - Choose and Create Monorepo Layout

- [x] 1.1 Pick Option A/B/C.
- [x] 1.2 Create final folders for backend app and frontend app.
- [x] 1.3 Move backend files into backend app folder.
- [x] 1.4 Move `v0/` into frontend app folder.
- [x] 1.5 Update `.gitignore` to stop ignoring frontend app files.

## Phase 2 - Workspace Tooling

- [ ] 2.1 Configure Rust workspace (`Cargo.toml`) if backend app is no longer at root.
- [ ] 2.2 Configure Node workspace for frontend (`pnpm-workspace.yaml` or npm workspaces).
- [ ] 2.3 Add root scripts for `build`, `test`, `lint`, `dev` per app.
- [ ] 2.4 Decide CI strategy: split backend/frontend pipelines with independent checks.

## Phase 3 - Frontend Integration

- [ ] 3.1 Remove mock-data dependency from frontend (`lib/mock-data.ts`).
- [ ] 3.2 Add typed API client in frontend for Archivist endpoints.
- [ ] 3.3 Connect tabs, filters, sort, and thread expansion to backend data.
- [ ] 3.4 Implement real file preview/download behavior from archived files.
- [ ] 3.5 Keep local preferences (theme/density) in frontend.

## Phase 4 - Backend API for Frontend

- [ ] 4.1 Add JSON endpoints in Rust for overview, thread list, filters, thread details.
- [ ] 4.2 Keep existing HTML endpoints during transition for rollback safety.
- [ ] 4.3 Reuse current cache strategy (5-minute TTL where applicable).
- [ ] 4.4 Add tests for new request/response contracts.

## Phase 5 - Deploy + Cutover

- [ ] 5.1 Create two Vercel projects from the monorepo:
  - backend app (Rust)
  - frontend app (v0 Next.js)
- [ ] 5.2 Wire frontend to backend base URL via env vars.
- [ ] 5.3 Validate parity against current `/record` behavior.
- [ ] 5.4 Cut over traffic to the new frontend.
- [ ] 5.5 Keep old UI path available temporarily as rollback.

## Current Gaps to Address During Migration

- `v0` is mock-data driven and needs backend wiring.
- `v0/next.config.mjs` currently sets `typescript.ignoreBuildErrors: true`.
- Root `.gitignore` currently contains `/v0`.
