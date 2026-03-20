#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${1:-https://arkivist-backend.vercel.app}"
RUNS="${2:-5}"

echo "Probing backend: ${BASE_URL}"
echo "Runs per endpoint: ${RUNS}"
echo

probe() {
  local path="$1"
  local label="$2"
  local i
  for ((i = 1; i <= RUNS; i += 1)); do
    local header_file
    header_file="$(mktemp)"
    local out
    out="$(curl -sS -D "${header_file}" -o /dev/null \
      -w "code=%{http_code} ttfb=%{time_starttransfer}s total=%{time_total}s size=%{size_download}" \
      "${BASE_URL%/}${path}")"
    local vercel_id
    vercel_id="$(awk -F': ' 'tolower($1)=="x-vercel-id"{print $2}' "${header_file}" | tr -d '\r')"
    local vercel_cache
    vercel_cache="$(awk -F': ' 'tolower($1)=="x-vercel-cache"{print $2}' "${header_file}" | tr -d '\r')"
    echo "[${label}] run=${i} ${out} x-vercel-cache=${vercel_cache:-n/a} x-vercel-id=${vercel_id:-n/a}"
    rm -f "${header_file}"
  done
  echo
}

probe "/api/health" "health"
probe "/api/record/threads?tab=top&limit=200" "threads-top-200"
probe "/api/record/threads?tab=week&limit=200" "threads-week-200"
probe "/api/record/threads?tab=month&limit=200" "threads-month-200"

echo "Done."
