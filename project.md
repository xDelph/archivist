# Slack Archiveur — Project

## 1) Objectif
Construire un **archiveur Slack** (canaux **publics + privés**, **sans DM**) sur **Slack Free**, afin de :
1. **Archiver** en temps réel les messages/threads/réactions dans une base
2. Permettre ensuite des usages **résumé / highlights / blogging** sur des threads/débats importants

---

## 2) Périmètre

### Inclus
- Canaux **publics** : archivage des messages + threads + réactions
- Canaux **privés** : archivage **uniquement** si l’app est **membre** du canal privé
- Gestion des **threads** (reconstruction via `thread_ts`)
- Gestion des **réactions** (événement + enrichissement optionnel via Web API)
- Déduplication des événements (`event_id`)
- Stockage durable (DB + éventuellement objet storage)

### Exclus
- DM / IM / MPIM
- Export natif Slack (limité sur Free) comme source de vérité
- Conformité/eDiscovery “Enterprise” (non disponible sur Free)

---

## 3) Contraintes Slack Free (à connaître)
- L’archivage fiable doit être **en flux** via Events API.
- L’app ne voit un **canal privé** que si elle y est **ajoutée**.
- Les événements doivent être **ack** rapidement (réponse HTTP 200), sinon Slack réessaie.

---

## 4) Architecture (MVP)

### Vue d’ensemble
1. Slack → **Events API** → Endpoint Vercel `POST /api/slack/events`
2. Endpoint :
   - Vérifie la signature Slack
   - Répond **200** immédiatement
   - Envoie le payload vers un traitement asynchrone (queue) OU écrit en DB rapidement
3. Worker/cron :
   - Dédup via `event_id`
   - Normalise et stocke
   - Backfill / enrichissement via Slack Web API (optionnel)

### Recommandation Vercel
- **Route API** pour la réception des events
- **Queue** (Upstash Redis) ou traitement léger direct en DB
- **Cron** Vercel (optionnel) pour backfill périodique

---

## 5) Configuration Slack App — Étapes

### 5.1 Créer l’app
- Slack API → Create an app → From scratch

### 5.2 Secrets
- Récupérer :
  - `SLACK_SIGNING_SECRET`
  - (après installation) `SLACK_BOT_TOKEN` (`xoxb-...`)

### 5.3 Scopes (Bot Token Scopes)
Minimum recommandé :
- `channels:read`
- `channels:history`
- `groups:read`
- `groups:history`
- `reactions:read` (recommandé)

### 5.4 Events API (Event Subscriptions)
- Enable Events
- Request URL = `https://<ton-projet>.vercel.app/api/slack/events`
- Subscribe to bot events :
  - `message.channels`
  - `message.groups`
  - `reaction_added`

### 5.5 Installation
- Install to Workspace
- Ajouter l’app aux canaux privés à archiver (manuel / via gouvernance interne)

---

## 6) Endpoint Vercel — Contrats Entrée/Sortie

### 6.1 Endpoint principal
**POST** `/api/slack/events`

#### Headers entrants (Slack)
- `X-Slack-Signature: v0=...`
- `X-Slack-Request-Timestamp: 1700000000`
- `Content-Type: application/json`

### 6.2 URL Verification (setup)
Slack envoie ce POST quand tu configures la Request URL.

**Entrée :**
```json
{
  "type": "url_verification",
  "challenge": "3eZbrw1aBm...",
  "token": "..."
}
```

**Sortie (200) :**
```json
{ "challenge": "3eZbrw1aBm..." }
```

### 6.3 Event callback
**Entrée (structure générale) :**
```json
{
  "type": "event_callback",
  "team_id": "T123",
  "api_app_id": "A123",
  "event_id": "Ev123",
  "event_time": 1700000000,
  "event": {
    "type": "message",
    "channel": "C123",
    "user": "U123",
    "text": "hello",
    "ts": "1700000000.000200",
    "thread_ts": "1700000000.000200",
    "subtype": null
  }
}
```

**Sortie :**
- Répondre **immédiatement** `200 OK` (body vide ou `{ "ok": true }`)

---

## 7) Sécurité — Vérification de signature (obligatoire)

### Entrées
- `timestamp` = header `X-Slack-Request-Timestamp`
- `signature` = header `X-Slack-Signature`
- `raw_body` = body brut (string)
- `signing_secret` = `SLACK_SIGNING_SECRET`

### Algorithme
1. Rejeter si `abs(now - timestamp) > 300s` (anti-replay)
2. `basestring = "v0:{timestamp}:{raw_body}"`
3. `computed = "v0=" + hex(hmac_sha256(signing_secret, basestring))`
4. Comparer `computed` et `signature` (comparaison constante)

### Sorties
- Si invalide: `401 Unauthorized`
- Si valide: continuer

---

## 8) Données à stocker — Modèle minimal (DB)

### 8.1 Table `slack_events` (dédup)
- `event_id` (PK/unique)
- `team_id`
- `event_time` (int)
- `payload_json` (jsonb)
- `received_at` (timestamp)

### 8.2 Table `messages`
Clés :
- `team_id`
- `channel_id`
- `ts` (unique par channel)

Champs :
- `thread_ts` (nullable)
- `user_id`
- `text`
- `subtype` (nullable)
- `edited_ts` (nullable)
- `deleted` (bool)
- `raw_json` (jsonb)
- `created_at` / `updated_at`

### 8.3 Table `reactions`
- `team_id`
- `channel_id`
- `message_ts`
- `user_id`
- `reaction_name`
- `event_ts`
- unique(`team_id`, `channel_id`, `message_ts`, `user_id`, `reaction_name`)

---

## 9) Slack Web API — Méthodes à utiliser (backfill & enrichissement)

> Base URL: `https://slack.com/api/<method>`  
> Auth: `Authorization: Bearer xoxb-...`

### 9.1 `conversations.list`
**Objectif :** lister les canaux accessibles

- Entrée (query) :
  - `types=public_channel,private_channel`
  - `exclude_archived=true`
  - `limit=200`
  - `cursor=<...>` (pagination)

- Sortie (JSON) :
  - `ok: true`
  - `channels: [...]`
  - `response_metadata.next_cursor`

### 9.2 `conversations.history`
**Objectif :** backfill d’un canal

- Entrée :
  - `channel=C123`
  - `oldest=1700000000.000000` (optionnel)
  - `latest=1700100000.000000` (optionnel)
  - `inclusive=true|false`
  - `limit=100`
  - `cursor=<...>`

- Sortie :
  - `ok`
  - `messages: [...]`
  - `has_more`
  - `response_metadata.next_cursor`

### 9.3 `conversations.replies`
**Objectif :** récupérer un thread complet

- Entrée :
  - `channel=C123`
  - `ts=1700000000.000200` (ts du message parent)
  - `limit=200` (jusqu’à 1000 selon Slack)
  - `cursor=<...>` (si nécessaire)

- Sortie :
  - `ok`
  - `messages: [parent, replies...]`
  - `has_more` + `response_metadata.next_cursor` (si paginé)

### 9.4 `reactions.get` (optionnel)
**Objectif :** enrichir/réconcilier réactions

- Entrée :
  - `channel=C123`
  - `timestamp=1700000000.000200`
  - `full=true` (optionnel)

- Sortie :
  - `ok`
  - `message.reactions: [...]`

---

## 10) Stratégie de traitement (MVP)

### 10.1 Ingestion Events API (temps réel)
Pour chaque event reçu :
1. Vérifier signature
2. Si `type=url_verification` : renvoyer challenge
3. Si `type=event_callback` :
   - Dédup : si `event_id` déjà vu → ignorer
   - Selon `event.type` :
     - `message` :
       - Ignorer certains `subtype` si nécessaire (ex: `bot_message`)
       - Upsert dans `messages`
     - `reaction_added` :
       - Insérer dans `reactions`
   - ACK 200

### 10.2 Backfill (optionnel mais recommandé)
- Cron nightly :
  - `conversations.list` → pour chaque channel :
    - `conversations.history` sur fenêtre (depuis dernier `ts` archivé)
    - Pour chaque message parent de thread : `conversations.replies`

### 10.3 Déduplication & Idempotence
- Niveau event : `event_id`
- Niveau message : (`channel_id`, `ts`) unique

---

## 11) Hébergement — Vercel

### Variables d’environnement
- `SLACK_SIGNING_SECRET`
- `SLACK_BOT_TOKEN`
- `DATABASE_URL` (selon DB)
- (optionnel) `UPSTASH_REDIS_REST_URL`, `UPSTASH_REDIS_REST_TOKEN`

### Routes proposées
- `POST /api/slack/events` — réception events
- `GET /api/health` — check
- `POST /api/admin/backfill` (protégé) — déclenche backfill manuel

---

## 12) Options DB / stockage (free tiers — à vérifier)
### Postgres managé
- Supabase (Postgres)
- Neon (Postgres serverless)

### SQLite edge
- Turso

### Redis/KV (queue/dédup/cache)
- Upstash Redis

### Objet storage (optionnel : pièces jointes / gros logs)
- Supabase Storage / Cloudflare R2 / AWS S3

---

## 13) Backlog (tâches)

### M1 — Ingestion live
- [ ] Créer Slack App + scopes + events
- [ ] Endpoint Vercel `/api/slack/events` + url_verification
- [ ] Vérification signature Slack
- [ ] DB schema + migrations
- [ ] Dédup `event_id`
- [ ] Stockage messages + réactions

### M2 — Backfill & threads
- [ ] Intégrer Web API (`conversations.list/history/replies`)
- [ ] Cron backfill (Vercel Cron)
- [ ] Reconciliation threads

### M3 — Highlights
- [ ] Score d’importance (réactions, participants uniques, longueur, mots-clés)
- [ ] Digest par channel/semaine
- [ ] Export Markdown/HTML pour blogging interne

---

## 14) Critères de réussite (MVP)
- Réception fiable des events (signature OK, ACK rapide)
- Archivage des messages publics et des privés où l’app est membre
- Reconstruction des threads via `thread_ts`
- Déduplication (pas de doublons malgré retries Slack)

---

## 15) Notes de gouvernance
- Transparence : informer le workspace de l’archivage
- Règle interne : “tout canal privé à archiver doit inviter le bot”
- Définir une politique de rétention côté DB
