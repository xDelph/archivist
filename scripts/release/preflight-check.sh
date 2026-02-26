#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-.env}"
TEMPLATE_FILE=".env.example"

if [[ ! -f "$TEMPLATE_FILE" ]]; then
  echo "ERROR: missing template file $TEMPLATE_FILE"
  exit 1
fi

if [[ ! -f "$ENV_FILE" ]]; then
  echo "ERROR: missing env file $ENV_FILE"
  exit 1
fi

OPTIONAL_KEYS=(
  "SLACK_USER_TOKEN"
  "CLOUDFLARED_R2_TOKEN"
  "NEXT_PUBLIC_API_BASE_URL"
)

is_optional() {
  local key="$1"
  for optional in "${OPTIONAL_KEYS[@]}"; do
    if [[ "$optional" == "$key" ]]; then
      return 0
    fi
  done
  return 1
}

missing_keys=()
empty_keys=()

mapfile -t expected_keys < <(grep -E '^[A-Z0-9_]+=' "$TEMPLATE_FILE" | cut -d'=' -f1)

for key in "${expected_keys[@]}"; do
  if ! grep -Eq "^${key}=" "$ENV_FILE"; then
    if ! is_optional "$key"; then
      missing_keys+=("$key")
    fi
    continue
  fi

  value="$(grep -E "^${key}=" "$ENV_FILE" | tail -n 1 | cut -d'=' -f2-)"
  if [[ -z "$value" ]] && ! is_optional "$key"; then
    empty_keys+=("$key")
  fi
done

if ((${#missing_keys[@]} > 0)); then
  echo "ERROR: missing required keys in $ENV_FILE:"
  for key in "${missing_keys[@]}"; do
    echo "  - $key"
  done
  exit 1
fi

if ((${#empty_keys[@]} > 0)); then
  echo "ERROR: empty required values in $ENV_FILE:"
  for key in "${empty_keys[@]}"; do
    echo "  - $key"
  done
  exit 1
fi

echo "OK: $ENV_FILE contains all required keys."
