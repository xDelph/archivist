# Arkivist Monorepo

Arkivist archive les conversations Slack (messages, threads, reactions, fichiers), calcule des classements hebdo/mensuels, et expose une UI moderne avec authentification.

Ce README documente le monorepo racine.  
La documentation backend détaillée reste dans `apps/backend/README.md`.

## Sommaire

- [Vue d'ensemble](#vue-densemble)
- [Architecture](#architecture)
- [Structure du repo](#structure-du-repo)
- [Prérequis](#prérequis)
- [Installation locale](#installation-locale)
- [Variables d'environnement](#variables-denvironnement)
- [Commandes utiles](#commandes-utiles)
- [API et sécurité](#api-et-sécurité)
- [CI/CD et release](#cicd-et-release)
- [Maintenance et debug](#maintenance-et-debug)
- [Licence](#licence)

## Vue d'ensemble

### Fonctions principales

- Ingestion en temps réel des événements Slack (`/api/slack/events`)
- Backfill asynchrone en 3 phases (messages, fichiers, agrégations)
- Stockage SQL (Neon/Postgres) + archivage fichiers Cloudflare R2
- Dashboard threads (`recent`, `week`, `month`, `top`)
- Auth complète (register/login/logout/me, reset password, préférences anonymat)
- UI Next.js avec proxy BFF vers le backend Rust

### Stack

- Monorepo: `bun` workspaces + `turbo`
- Backend: Rust (`axum`, `sqlx`, `vercel_runtime`)
- Frontend: Next.js 16 + React 19 + Tailwind v4 + composants shadcn/ui
- Infra cible: Vercel (frontend + backend), Slack Events API, Neon, Cloudflare R2, Upstash QStash

## Architecture

```text
Slack Events API
   -> Backend Rust (Vercel Functions)
      -> Postgres (messages, rollups, auth, jobs)
      -> R2 (fichiers archivés)
      -> QStash (orchestration workers sync)

Utilisateur navigateur
   -> Frontend Next.js (port 3001 en local)
      -> /api/auth/* proxy (cookie session + secret interne)
      -> /api/record/* proxy (cookie session + secret interne)
      -> Backend Rust
```

Points importants:

- Les routes backend `api/auth/*` et `api/record/*` sont protégées par un header interne `x-arkivist-internal-secret`.
- Le frontend joue le rôle de BFF: il ajoute ce header, relaie la session, et gère les cookies HTTP-only.
- Les pages applicatives sont protégées par middleware (`apps/frontend/proxy.ts`) et redirigent vers `/login` sans session.

## Structure du repo

```text
.
|- apps/
|  |- backend/         # API Rust, workers sync, migrations SQL, assets SSR legacy
|  `- frontend/        # App Next.js + routes proxy /api/auth et /api/record
|- packages/
|  `- contracts/       # Réservé aux contrats partagés
|- scripts/
|  |- db/              # Probes SQL read-model
|  |- dev/             # Checks de régression locale backend
|  |- perf/            # Probes perf endpoints
|  `- release/         # Preflight env + smoke test déploiement
`- docs/
   |- phase5-deploy-cutover.md
   `- performance/*
```

## Prérequis

- Node.js 22.x (aligné CI)
- `bun` (version stable récente)
- Rust stable (+ `cargo`)
- `sqlx-cli` pour migrations locales
- Optionnel mais recommandé: `vercel` CLI

Installation `sqlx-cli`:

```bash
cargo install sqlx-cli --no-default-features --features postgres,rustls
```

## Installation locale

### 1) Installer les dépendances

```bash
bun install
```

### 2) Préparer les environnements

```bash
cp .env.example .env
cp apps/frontend/.env.example apps/frontend/.env.local
```

Puis renseigner les valeurs nécessaires (voir section [Variables d'environnement](#variables-denvironnement)).

### 3) Appliquer les migrations backend

Depuis `apps/backend`:

```bash
source ../../.env
DATABASE_URL="$DATABASE_URL_UNPOOLED" sqlx migrate run
```

### 4) Lancer le backend

Mode Rust direct (script racine):

```bash
bun run dev:backend
```

Mode Vercel local (utile pour simuler le runtime Vercel):

```bash
cd apps/backend
bun run dev:vercel
```

### 5) Lancer le frontend

```bash
bun run dev:frontend
```

URLs locales habituelles:

- Frontend: `http://localhost:3001`
- Backend direct (cargo): `http://localhost:3000`
- Backend via vercel dev: `http://localhost:3100`

## Variables d'environnement

Source de vérité: `.env.example`.

### Variables critiques (dev + prod)

- `DATABASE_URL`
- `DATABASE_URL_UNPOOLED`
- `SLACK_SIGNING_SECRET`
- `SLACK_BOT_TOKEN` (ou `SLACK_USER_TOKEN` selon le mode de backfill)
- `ADMIN_TOKEN`
- `FRONTEND_BACKEND_SHARED_SECRET`
- `BACKEND_API_BASE_URL`
- `NEXT_PUBLIC_API_BASE_URL` (optionnelle en preflight, mais utile en split frontend/backend)
- `EMAIL_ENCRYPTION_KEY`
- `EMAIL_LOOKUP_KEY`

### Variables d'archivage fichiers (si activé)

- `CLOUDFLARED_R2_ACCOUNT_ID`
- `CLOUDFLARED_R2_ACCESS_KEY`
- `CLOUDFLARED_R2_SECRET_KEY`
- `CLOUDFLARED_R2_BUCKET`
- `CLOUDFLARED_R2_PUBLIC_URL`

### Variables sync workers / QStash

- `UPSTASH_QSTASH_TOKEN`
- `UPSTASH_QSTASH_URL`
- `UPSTASH_QSTASH_CURRENT_SIGNING_KEY`
- `UPSTASH_QSTASH_NEXT_SIGNING_KEY`
- `BACKFILL_WORKER_TOKEN`
- Optionnelles: `BACKFILL_WORKER_URL`, `BACKFILL_FILES_WORKER_URL`, `BACKFILL_AGGREGATE_WORKER_URL`

### Notes auth

- `AUTH_DEV_EXPOSE_RESET_TOKEN=true` peut exposer le token de reset en dev uniquement.
- Les comptes sont liés aux utilisateurs Slack existants en DB (email eligible).

## Commandes utiles

### Commandes monorepo

```bash
bun run build
bun run lint
bun run typecheck
bun run test
bun run check
```

### Développement ciblé

```bash
bun run dev:backend
bun run dev:frontend
```

### Vérification backend locale (API + logs)

```bash
bun run check:backend:dev
bun run check:backend:dev -- http://localhost:3100
bun run check:backend:dev -- http://localhost:3100 ./logs/app.log
```

### Preflight + smoke release

```bash
bun run release:preflight
bun run release:smoke -- https://<backend-domain> https://<frontend-domain>
```

### Probes performance / SQL

```bash
scripts/perf/probe_backend_endpoints.sh https://arkivist-backend.vercel.app 5
scripts/db/probe_read_model_queries.sh --tab all --limit 50
scripts/db/probe_read_model_queries.sh --tab all --limit 50 --api-url http://localhost:3100
```

## API et sécurité

### Endpoints backend exposés (protégés selon le cas)

- `POST /api/slack/events`
- `GET /api/health`
- `POST /api/admin/sync` (Bearer `ADMIN_TOKEN`)
- `POST /api/admin/sync/run` (worker token et/ou signature QStash)
- `POST /api/admin/sync/files` (worker token et/ou signature QStash)
- `POST /api/admin/sync/aggregate` (worker token et/ou signature QStash)

### Endpoints backend internes (via frontend proxy)

- `POST /api/auth/register`
- `POST /api/auth/login`
- `POST /api/auth/logout`
- `GET /api/auth/me`
- `POST /api/auth/change-password`
- `PATCH|POST /api/auth/preferences`
- `POST /api/auth/password/forgot`
- `POST /api/auth/password/reset`
- `GET /api/record/threads?...`
- `GET /api/record/thread?channel_id=...&ts=...`

Les routes frontend correspondantes (`apps/frontend/app/api/auth/[...path]/route.ts` et `apps/frontend/app/api/record/[...path]/route.ts`) injectent:

- `x-arkivist-internal-secret`
- `x-arkivist-session` (si cookie session présent)

## CI/CD et release

Workflows GitHub:

- `.github/workflows/ci-backend.yml`: fmt + check + test + clippy Rust
- `.github/workflows/ci-frontend.yml`: install + typecheck + build frontend
- `.github/workflows/release-smoke.yml`: smoke test manuel backend/frontend
- `.github/workflows/sync.yml`: sync cron toutes les 10 min (`POST /api/admin/sync`)

Runbook de cutover:

- `docs/phase5-deploy-cutover.md`

## Maintenance et debug

### Binaires backend utiles

```bash
# Backfill local + sync users + archivage fichiers batch
cargo run --manifest-path apps/backend/Cargo.toml --bin backfill_local

# Archivage one-shot de fichiers existants
cargo run --manifest-path apps/backend/Cargo.toml --bin archive_files_local -- --purge

# Recalcul historique des scores hebdo
cargo run --manifest-path apps/backend/Cargo.toml --bin compute_weekly_scores

# Read-model rollups
cargo run --manifest-path apps/backend/Cargo.toml --bin rebuild_rollups -- --enqueue-only
cargo run --manifest-path apps/backend/Cargo.toml --bin repair_rollup -- --channel C123 --ts 1700000000.000000
cargo run --manifest-path apps/backend/Cargo.toml --bin validate_rollups -- --strict --sample 300
```

### Problèmes fréquents

- `missing internal secret configuration`: variable `FRONTEND_BACKEND_SHARED_SECRET` absente côté backend et/ou frontend.
- `401 unauthorized` sur `/api/record/*`: requête directe backend sans session/proxy frontend.
- Port déjà pris: garder `3000` libre si backend en runtime direct.
- `release:preflight` en échec: clé manquante ou vide dans `.env`.

---

Si vous cherchez les détails d'implémentation backend (architecture interne, endpoints SSR historiques, SQLX, tests), voir `apps/backend/README.md`.

## Licence

Ce projet est en double licence:

- Open source: `AGPL-3.0-or-later` (voir `LICENSE`)
- Commercial: autorisation commerciale écrite requise hors conformité AGPL
  (voir `LICENSE-COMMERCIAL.md`)
