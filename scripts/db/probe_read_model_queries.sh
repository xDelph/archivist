#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

TAB="all"
LIMIT="50"
EXPLAIN="false"
ENV_FILE="${REPO_ROOT}/.env"
DB_URL_OVERRIDE=""
API_URL=""

usage() {
  cat <<'EOF'
Usage:
  scripts/db/probe_read_model_queries.sh [options]

Options:
  --tab <top|week|month|recent|all>   Query family to run (default: all)
  --limit <N>                         Number of rows sampled per query (default: 50)
  --explain                           Run EXPLAIN (ANALYZE, BUFFERS) before each query
  --env-file <path>                   Env file to source (default: ./.env)
  --db-url <postgres-url>             Override DATABASE_URL(_UNPOOLED)
  --api-url <base-url>                Optional API probe (ex: http://localhost:3100)
  -h, --help                          Show this help

Examples:
  scripts/db/probe_read_model_queries.sh --tab top --limit 20
  scripts/db/probe_read_model_queries.sh --tab all --limit 100 --explain
  scripts/db/probe_read_model_queries.sh --tab all --api-url http://localhost:3100
EOF
}

die() {
  echo "error: $*" >&2
  exit 1
}

print_section() {
  echo
  echo "=== $1 ==="
}

resolve_db_url() {
  local raw_url="$1"
  if [[ -z "${raw_url}" ]]; then
    die "missing database URL"
  fi

  # Keep explicit endpoint option if already present.
  if [[ "${raw_url}" == *"options=endpoint%3D"* || "${raw_url}" == *"options=endpoint="* ]]; then
    printf '%s\n' "${raw_url}"
    return 0
  fi

  # Neon requires endpoint option for older libpq clients.
  local endpoint_id=""
  endpoint_id="$(
    node -e '
      try {
        const u = new URL(process.argv[1]);
        const host = u.hostname || "";
        const first = host.split(".")[0] || "";
        process.stdout.write(first);
      } catch (_err) {}
    ' "${raw_url}"
  )"

  if [[ "${endpoint_id}" == ep-* ]]; then
    if [[ "${raw_url}" == *"?"* ]]; then
      printf '%s&options=endpoint%%3D%s\n' "${raw_url}" "${endpoint_id}"
    else
      printf '%s?options=endpoint%%3D%s\n' "${raw_url}" "${endpoint_id}"
    fi
    return 0
  fi

  printf '%s\n' "${raw_url}"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tab)
      [[ $# -ge 2 ]] || die "missing value after --tab"
      TAB="$2"
      shift 2
      ;;
    --limit)
      [[ $# -ge 2 ]] || die "missing value after --limit"
      LIMIT="$2"
      shift 2
      ;;
    --explain)
      EXPLAIN="true"
      shift
      ;;
    --env-file)
      [[ $# -ge 2 ]] || die "missing value after --env-file"
      ENV_FILE="$2"
      shift 2
      ;;
    --db-url)
      [[ $# -ge 2 ]] || die "missing value after --db-url"
      DB_URL_OVERRIDE="$2"
      shift 2
      ;;
    --api-url)
      [[ $# -ge 2 ]] || die "missing value after --api-url"
      API_URL="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown option: $1"
      ;;
  esac
done

case "${TAB}" in
  top|week|month|recent|all) ;;
  *) die "invalid --tab '${TAB}' (expected: top|week|month|recent|all)" ;;
esac

[[ "${LIMIT}" =~ ^[0-9]+$ ]] || die "--limit must be a positive integer"
if [[ "${LIMIT}" -le 0 ]]; then
  die "--limit must be > 0"
fi

if [[ -f "${ENV_FILE}" ]]; then
  # shellcheck source=/dev/null
  set -a
  source "${ENV_FILE}"
  set +a
fi

RAW_DB_URL="${DB_URL_OVERRIDE:-${DATABASE_URL_UNPOOLED:-${DATABASE_URL:-}}}"
DB_URL="$(resolve_db_url "${RAW_DB_URL}")"
[[ -n "${DB_URL}" ]] || die "DATABASE_URL_UNPOOLED or DATABASE_URL must be set"

PSQL=(psql "${DB_URL}" -v ON_ERROR_STOP=1 -P pager=off)

run_sql() {
  local title="$1"
  local sql="$2"
  print_section "${title}"
  "${PSQL[@]}" -c "${sql}"
}

run_explain_if_enabled() {
  local title="$1"
  local sql="$2"
  if [[ "${EXPLAIN}" == "true" ]]; then
    print_section "EXPLAIN ${title}"
    "${PSQL[@]}" -c "EXPLAIN (ANALYZE, BUFFERS, VERBOSE) ${sql}"
  fi
}

run_overview() {
  run_sql "Dataset Overview" "
    SELECT
      (SELECT COUNT(*) FROM messages) AS messages_count,
      (SELECT COUNT(*) FROM thread_rollups) AS thread_rollups_count,
      (SELECT COUNT(*) FROM thread_period_scores WHERE period_kind = 'week') AS weekly_scores_count,
      (SELECT COUNT(*) FROM thread_period_scores WHERE period_kind = 'month') AS monthly_scores_count,
      (SELECT COUNT(*) FROM channels) AS channels_count,
      (SELECT COUNT(*) FROM users) AS users_count;
  "

  run_sql "Aggregation Queue Status" "
    SELECT status, COUNT(*)::bigint AS jobs
    FROM aggregation_jobs
    WHERE job_kind = 'thread_rollup'
    GROUP BY status
    ORDER BY status;
  "
}

run_top() {
  local query="
    SELECT
      tr.channel_id,
      COALESCE(ch.name, tr.channel_id) AS channel_name,
      tr.thread_ts,
      tr.score_total AS score,
      tr.reaction_count_total AS reactions,
      tr.reply_count_total AS replies,
      tr.participant_count_total AS participants,
      tr.root_text AS root_text
    FROM thread_rollups tr
    LEFT JOIN channels ch ON ch.channel_id = tr.channel_id
    WHERE COALESCE(ch.name, tr.channel_id) != 'intro'
    ORDER BY tr.score_total DESC, tr.channel_id, tr.thread_ts
    LIMIT ${LIMIT}
  "

  run_explain_if_enabled "Top Query" "${query}"

  run_sql "Top Query - Sanity" "
    WITH ranked AS (${query}),
    duplicates AS (
      SELECT COUNT(*)::bigint AS duplicate_rows
      FROM (
        SELECT channel_id, thread_ts
        FROM ranked
        GROUP BY channel_id, thread_ts
        HAVING COUNT(*) > 1
      ) d
    ),
    ordering AS (
      SELECT COUNT(*)::bigint AS score_order_violations
      FROM (
        SELECT score, LAG(score) OVER (ORDER BY score DESC, channel_id, thread_ts) AS prev_score
        FROM ranked
      ) o
      WHERE prev_score IS NOT NULL
        AND score > prev_score
    )
    SELECT
      (SELECT COUNT(*) FROM ranked) AS rows_returned,
      (SELECT COUNT(*) FROM ranked WHERE COALESCE(root_text, '') = '') AS empty_root_text,
      (SELECT COUNT(*) FROM ranked WHERE COALESCE(channel_name, '') = '') AS empty_channel_name,
      (SELECT duplicate_rows FROM duplicates) AS duplicate_rows,
      (SELECT score_order_violations FROM ordering) AS score_order_violations;
  "

  run_sql "Top Query - Sample Rows" "
    WITH ranked AS (${query})
    SELECT channel_name, thread_ts, score, reactions, replies, participants
    FROM ranked
    ORDER BY score DESC, channel_id, thread_ts;
  "
}

run_week() {
  local query="
    SELECT
      tr.channel_id,
      COALESCE(ch.name, tr.channel_id) AS channel_name,
      tr.thread_ts,
      tps.period_start,
      tps.score_period AS rank_score,
      RANK() OVER (
        ORDER BY tps.score_period DESC, tps.channel_id, tps.thread_ts
      )::bigint AS rank
    FROM thread_period_scores tps
    JOIN thread_rollups tr
      ON tr.channel_id = tps.channel_id
     AND tr.thread_ts = tps.thread_ts
    LEFT JOIN channels ch
      ON ch.channel_id = tr.channel_id
    WHERE tps.period_kind = 'week'
      AND tps.period_start = date_trunc('week', CURRENT_DATE)::date
      AND COALESCE(ch.name, tr.channel_id) != 'intro'
    ORDER BY rank ASC
    LIMIT ${LIMIT}
  "

  run_explain_if_enabled "Week Query" "${query}"

  run_sql "Week Query - Sanity" "
    WITH ranked AS (${query}),
    ordering AS (
      SELECT COUNT(*)::bigint AS rank_order_violations
      FROM (
        SELECT rank, LAG(rank) OVER (ORDER BY rank, channel_id, thread_ts) AS prev_rank
        FROM ranked
      ) r
      WHERE prev_rank IS NOT NULL
        AND rank < prev_rank
    )
    SELECT
      (SELECT COUNT(*) FROM ranked) AS rows_returned,
      (SELECT COUNT(*) FROM ranked WHERE rank_score < 0) AS negative_scores,
      (SELECT COUNT(*) FROM ranked WHERE period_start <> date_trunc('week', CURRENT_DATE)::date) AS wrong_period_start,
      (SELECT rank_order_violations FROM ordering) AS rank_order_violations;
  "

  run_sql "Week Query - Sample Rows" "
    WITH ranked AS (${query})
    SELECT rank, channel_name, thread_ts, rank_score
    FROM ranked
    ORDER BY rank, channel_id, thread_ts;
  "
}

run_month() {
  local query="
    SELECT
      tr.channel_id,
      COALESCE(ch.name, tr.channel_id) AS channel_name,
      tr.thread_ts,
      tps.period_start,
      tps.score_period AS rank_score,
      RANK() OVER (
        ORDER BY tps.score_period DESC, tps.channel_id, tps.thread_ts
      )::bigint AS rank
    FROM thread_period_scores tps
    JOIN thread_rollups tr
      ON tr.channel_id = tps.channel_id
     AND tr.thread_ts = tps.thread_ts
    LEFT JOIN channels ch
      ON ch.channel_id = tr.channel_id
    WHERE tps.period_kind = 'month'
      AND tps.period_start = date_trunc('month', CURRENT_DATE)::date
      AND COALESCE(ch.name, tr.channel_id) != 'intro'
    ORDER BY rank ASC
    LIMIT ${LIMIT}
  "

  run_explain_if_enabled "Month Query" "${query}"

  run_sql "Month Query - Sanity" "
    WITH ranked AS (${query}),
    ordering AS (
      SELECT COUNT(*)::bigint AS rank_order_violations
      FROM (
        SELECT rank, LAG(rank) OVER (ORDER BY rank, channel_id, thread_ts) AS prev_rank
        FROM ranked
      ) r
      WHERE prev_rank IS NOT NULL
        AND rank < prev_rank
    )
    SELECT
      (SELECT COUNT(*) FROM ranked) AS rows_returned,
      (SELECT COUNT(*) FROM ranked WHERE rank_score < 0) AS negative_scores,
      (SELECT COUNT(*) FROM ranked WHERE period_start <> date_trunc('month', CURRENT_DATE)::date) AS wrong_period_start,
      (SELECT rank_order_violations FROM ordering) AS rank_order_violations;
  "

  run_sql "Month Query - Sample Rows" "
    WITH ranked AS (${query})
    SELECT rank, channel_name, thread_ts, rank_score
    FROM ranked
    ORDER BY rank, channel_id, thread_ts;
  "
}

run_recent() {
  local query="
    SELECT
      tr.channel_id,
      COALESCE(ch.name, tr.channel_id) AS channel_name,
      tr.thread_ts,
      tr.score_total AS score,
      tr.reaction_count_total AS reactions,
      tr.reply_count_total AS replies,
      tr.participant_count_total AS participants
    FROM thread_rollups tr
    LEFT JOIN channels ch ON ch.channel_id = tr.channel_id
    WHERE COALESCE(ch.name, tr.channel_id) != 'intro'
    ORDER BY tr.thread_ts DESC, tr.channel_id, tr.thread_ts
    LIMIT ${LIMIT}
  "

  run_explain_if_enabled "Recent Query" "${query}"

  run_sql "Recent Query - Sanity" "
    WITH ranked AS (${query}),
    numbered AS (
      SELECT *, ROW_NUMBER() OVER () AS rn
      FROM ranked
    ),
    ordering AS (
      SELECT COUNT(*)::bigint AS thread_ts_order_violations
      FROM (
        SELECT thread_ts, LAG(thread_ts) OVER (ORDER BY rn) AS prev_ts
        FROM numbered
      ) o
      WHERE prev_ts IS NOT NULL
        AND thread_ts > prev_ts
    )
    SELECT
      (SELECT COUNT(*) FROM ranked) AS rows_returned,
      (SELECT COUNT(*) FROM ranked WHERE thread_ts !~ '^[0-9]+(\.[0-9]+)?$') AS non_numeric_thread_ts,
      (SELECT thread_ts_order_violations FROM ordering) AS thread_ts_order_violations;
  "

  run_sql "Recent Query - Sample Rows" "
    WITH ranked AS (${query})
    SELECT channel_name, thread_ts, score, reactions, replies, participants
    FROM ranked
    ORDER BY thread_ts DESC, channel_id, thread_ts;
  "
}

run_api_probe() {
  local base="${API_URL%/}"
  local tabs_csv
  if [[ "${TAB}" == "all" ]]; then
    tabs_csv="top,week,month,recent"
  else
    tabs_csv="${TAB}"
  fi
  print_section "API JSON Probe"
  echo "base_url=${base}"

  node - "${base}" "${LIMIT}" "${tabs_csv}" <<'NODE'
const baseUrl = process.argv[2];
const limit = Number.parseInt(process.argv[3], 10) || 50;
const tabs = (process.argv[4] || "top,week,month,recent")
  .split(",")
  .map((value) => value.trim())
  .filter(Boolean);

async function run() {
  for (const tab of tabs) {
    const url = `${baseUrl}/api/record/threads?tab=${tab}&limit=${Math.min(limit, 20)}`;
    const response = await fetch(url);
    const body = await response.text();
    let parsed;
    let jsonOk = true;
    try {
      parsed = JSON.parse(body);
    } catch (_err) {
      jsonOk = false;
    }

    const threadsCount =
      jsonOk && parsed && Array.isArray(parsed.threads) ? parsed.threads.length : -1;
    const responseTab = jsonOk && parsed ? parsed.tab : "n/a";
    const tabMatches = responseTab === tab;
    console.log(
      `[api:${tab}] status=${response.status} json_ok=${jsonOk} tab_match=${tabMatches} response_tab=${responseTab} threads=${threadsCount}`
    );
    if (response.status !== 200) {
      process.exitCode = 1;
    }
    if (!jsonOk) {
      console.log(`[api:${tab}] body_sample=${body.slice(0, 180).replace(/\n/g, " ")}`);
      process.exitCode = 1;
    }
    if (jsonOk && !tabMatches) {
      process.exitCode = 1;
    }
  }
}

run().catch((err) => {
  console.error(`[api] probe failed: ${String(err)}`);
  process.exit(1);
});
NODE
}

print_section "Read-Model SQL Probe"
echo "tab=${TAB} limit=${LIMIT} explain=${EXPLAIN}"
echo "db_url_host=$(node -e 'const u=new URL(process.argv[1]); console.log(u.host);' "${DB_URL}")"

run_overview

case "${TAB}" in
  top)
    run_top
    ;;
  week)
    run_week
    ;;
  month)
    run_month
    ;;
  recent)
    run_recent
    ;;
  all)
    run_top
    run_week
    run_month
    run_recent
    ;;
esac

if [[ -n "${API_URL}" ]]; then
  run_api_probe
fi

print_section "Done"
echo "SQL probe finished."
