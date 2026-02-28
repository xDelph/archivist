#!/usr/bin/env bash
set -euo pipefail

BASE_URL="http://localhost:3100"
LOG_FILE=""
CURL_TIMEOUT="${CURL_TIMEOUT:-15}"
TAIL_LINES="${TAIL_LINES:-400}"
STRICT_LOGS="${STRICT_LOGS:-1}"

usage() {
  cat <<'EOF'
Usage:
  scripts/dev/check-backend.sh [base_url] [log_file]
  scripts/dev/check-backend.sh [options]

Options:
  --base-url <url>         Backend base URL (default: http://localhost:3100)
  --log-file <path>        Explicit log file path (default: auto-detect)
  --curl-timeout <sec>     Curl timeout in seconds (default: 15)
  --tail-lines <n>         Number of log lines printed on failure (default: 400)
  --strict-logs <0|1>      Require log file presence (default: 1)
  -h, --help               Show this help
EOF
}

fail() {
  echo "ERROR: $*" >&2
  exit 1
}

pick_log_file() {
  if [[ -n "${LOG_FILE}" ]]; then
    [[ -f "${LOG_FILE}" ]] || fail "log file not found: ${LOG_FILE}"
    printf '%s\n' "${LOG_FILE}"
    return 0
  fi

  local candidates=(
    "apps/backend/logs/app.log"
    "logs/app.log"
  )

  local candidate
  for candidate in "${candidates[@]}"; do
    if [[ -f "${candidate}" ]]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  done

  if [[ "${STRICT_LOGS}" == "1" ]]; then
    fail "no app.log found (expected apps/backend/logs/app.log or logs/app.log)"
  fi

  printf '\n'
}

http_get() {
  local path="$1"
  local body_file="$2"
  local code
  code="$(curl -sS --max-time "${CURL_TIMEOUT}" -o "${body_file}" -w "%{http_code}" "${BASE_URL%/}${path}" || true)"
  printf '%s\n' "${code}"
}

check_health() {
  local body_file
  body_file="$(mktemp)"
  local code
  code="$(http_get "/api/health" "${body_file}")"
  [[ "${code}" == "200" ]] || fail "GET /api/health failed (status=${code}, body=$(head -c 160 "${body_file}"))"

  node - "${body_file}" <<'NODE'
const fs = require("fs");
const payload = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
if (!payload || payload.ok !== true) {
  console.error("unexpected /api/health payload", payload);
  process.exit(1);
}
NODE
  rm -f "${body_file}"
  echo "OK health"
}

check_tab() {
  local tab="$1"
  local body_file
  body_file="$(mktemp)"
  local code
  code="$(http_get "/api/record/threads?tab=${tab}&limit=50" "${body_file}")"
  [[ "${code}" == "200" ]] || fail "tab=${tab} failed (status=${code}, body=$(head -c 180 "${body_file}"))"

  local parsed
  parsed="$(node - "${body_file}" "${tab}" <<'NODE'
const fs = require("fs");
const bodyFile = process.argv[2];
const expectedTab = process.argv[3];
const payload = JSON.parse(fs.readFileSync(bodyFile, "utf8"));

if (!payload || payload.tab !== expectedTab) {
  console.error(`tab mismatch: expected=${expectedTab}, actual=${payload && payload.tab}`);
  process.exit(1);
}
if (!Array.isArray(payload.threads)) {
  console.error("threads is not an array");
  process.exit(1);
}
if (!Array.isArray(payload.channelStats)) {
  console.error("channelStats is not an array");
  process.exit(1);
}
if (!Array.isArray(payload.activityData)) {
  console.error("activityData is not an array");
  process.exit(1);
}
const firstId = payload.threads.length > 0 && payload.threads[0] && payload.threads[0].id
  ? String(payload.threads[0].id)
  : "";
console.log(`${payload.threads.length}|${firstId}`);
NODE
)"

  local threads_count="${parsed%%|*}"
  local first_id="${parsed#*|}"
  echo "OK tab=${tab} threads=${threads_count} first=${first_id:-<none>}"
  rm -f "${body_file}"
}

check_thread_endpoint_from_top() {
  local body_file
  body_file="$(mktemp)"
  local code
  code="$(http_get "/api/record/threads?tab=top&limit=1" "${body_file}")"
  [[ "${code}" == "200" ]] || fail "failed to fetch top tab for thread probe (status=${code})"

  local channel_id thread_ts
  read -r channel_id thread_ts < <(node - "${body_file}" <<'NODE'
const fs = require("fs");
const payload = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
if (!payload || !Array.isArray(payload.threads) || payload.threads.length === 0) {
  console.log("");
  process.exit(0);
}
const first = payload.threads[0] || {};
console.log(`${first.channelId || ""} ${first.ts || ""}`);
NODE
)
  rm -f "${body_file}"

  if [[ -z "${channel_id}" || -z "${thread_ts}" ]]; then
    echo "WARN top tab has no thread, skipping /api/record/thread probe"
    return 0
  fi

  local thread_body
  thread_body="$(mktemp)"
  code="$(http_get "/api/record/thread?channel_id=${channel_id}&ts=${thread_ts}" "${thread_body}")"
  [[ "${code}" == "200" ]] || fail "thread endpoint failed (status=${code}, channel=${channel_id}, ts=${thread_ts})"
  node - "${thread_body}" <<'NODE'
const fs = require("fs");
const payload = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
if (!payload || !Array.isArray(payload.messages)) {
  console.error("thread payload missing messages array");
  process.exit(1);
}
NODE
  rm -f "${thread_body}"
  echo "OK thread endpoint channel=${channel_id} ts=${thread_ts}"
}

scan_log_output() {
  local log_path="$1"
  local start_line="$2"
  [[ -n "${log_path}" ]] || return 0

  local total_lines
  total_lines="$(wc -l < "${log_path}" | tr -d ' ')"
  local from_line=$((start_line + 1))
  if (( from_line > total_lines )); then
    echo "WARN no new log lines captured in ${log_path}"
    return 0
  fi

  local new_lines
  new_lines="$(mktemp)"
  sed -n "${from_line},${total_lines}p" "${log_path}" > "${new_lines}"

  local error_pattern
  error_pattern='"level":"ERROR"|panic caught|handler failed|task crashed|NO_RESPONSE_FROM_FUNCTION|LambdaError|Process exited before completing request'
  if grep -E "${error_pattern}" "${new_lines}" >/dev/null; then
    echo "ERROR markers found in ${log_path} (new lines):" >&2
    tail -n "${TAIL_LINES}" "${new_lines}" >&2
    rm -f "${new_lines}"
    exit 1
  fi

  local tab
  for tab in top week month recent; do
    if ! grep -F "\"tab\":\"${tab}\"" "${new_lines}" >/dev/null; then
      echo "WARN no completion log found for tab=${tab} in ${log_path}"
    fi
  done

  echo "OK log scan (${log_path}, new_lines=$((total_lines - start_line)))"
  rm -f "${new_lines}"
}

parse_args() {
  local positional=()
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --base-url)
        [[ $# -ge 2 ]] || fail "missing value after --base-url"
        BASE_URL="$2"
        shift 2
        ;;
      --log-file)
        [[ $# -ge 2 ]] || fail "missing value after --log-file"
        LOG_FILE="$2"
        shift 2
        ;;
      --curl-timeout)
        [[ $# -ge 2 ]] || fail "missing value after --curl-timeout"
        CURL_TIMEOUT="$2"
        shift 2
        ;;
      --tail-lines)
        [[ $# -ge 2 ]] || fail "missing value after --tail-lines"
        TAIL_LINES="$2"
        shift 2
        ;;
      --strict-logs)
        [[ $# -ge 2 ]] || fail "missing value after --strict-logs"
        STRICT_LOGS="$2"
        shift 2
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      --)
        shift
        positional+=("$@")
        break
        ;;
      -*)
        fail "unknown option: $1"
        ;;
      *)
        positional+=("$1")
        shift
        ;;
    esac
  done

  if [[ ${#positional[@]} -ge 1 ]]; then
    BASE_URL="${positional[0]}"
  fi
  if [[ ${#positional[@]} -ge 2 ]]; then
    LOG_FILE="${positional[1]}"
  fi
  if [[ ${#positional[@]} -gt 2 ]]; then
    fail "too many positional arguments (expected at most: base_url [log_file])"
  fi
}

main() {
  parse_args "$@"
  echo "Backend check: base_url=${BASE_URL}"
  local log_path
  log_path="$(pick_log_file)"
  local start_line=0
  if [[ -n "${log_path}" ]]; then
    start_line="$(wc -l < "${log_path}" | tr -d ' ')"
    echo "Using log file: ${log_path} (start_line=${start_line})"
  else
    echo "Log scan disabled (STRICT_LOGS=0 and no log file found)"
  fi

  check_health
  check_tab "top"
  check_tab "week"
  check_tab "month"
  check_tab "recent"
  check_thread_endpoint_from_top
  scan_log_output "${log_path}" "${start_line}"

  echo "Backend check completed successfully."
}

main "$@"
