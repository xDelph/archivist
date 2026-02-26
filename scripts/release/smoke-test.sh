#!/usr/bin/env bash
set -euo pipefail

BACKEND_URL="${1:-${BACKEND_URL:-}}"
FRONTEND_URL="${2:-${FRONTEND_URL:-}}"

if [[ -z "$BACKEND_URL" || -z "$FRONTEND_URL" ]]; then
  echo "Usage: $0 <backend_base_url> <frontend_base_url>"
  echo "Or set BACKEND_URL and FRONTEND_URL env vars."
  exit 1
fi

BACKEND_URL="${BACKEND_URL%/}"
FRONTEND_URL="${FRONTEND_URL%/}"

tmp_body="$(mktemp)"
trap 'rm -f "$tmp_body"' EXIT

check_status() {
  local url="$1"
  local expected="$2"
  local label="$3"
  local code
  code="$(curl -sS -o "$tmp_body" -w "%{http_code}" "$url")"
  if [[ "$code" != "$expected" ]]; then
    echo "ERROR: $label returned HTTP $code (expected $expected)"
    echo "Body:"
    cat "$tmp_body"
    return 1
  fi
  echo "OK: $label returned HTTP $expected"
}

assert_body_contains() {
  local needle="$1"
  local label="$2"
  if ! grep -q "$needle" "$tmp_body"; then
    echo "ERROR: $label body does not contain '$needle'"
    echo "Body:"
    cat "$tmp_body"
    return 1
  fi
  echo "OK: $label body contains '$needle'"
}

echo "Running smoke tests..."
echo "Backend:  $BACKEND_URL"
echo "Frontend: $FRONTEND_URL"

check_status "$BACKEND_URL/api/health" "200" "backend health"
assert_body_contains '"ok":true' "backend health"

check_status "$BACKEND_URL/api/record/threads?tab=top&limit=1" "200" "backend record threads"
assert_body_contains '"threads"' "backend record threads"
assert_body_contains '"overviewStats"' "backend record threads"
assert_body_contains '"messagesChange"' "backend record threads"

check_status "$FRONTEND_URL/" "200" "frontend home"
assert_body_contains "Archivist" "frontend home"

echo "Smoke test passed."
