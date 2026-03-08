#!/usr/bin/env bash
set -euo pipefail

HEALTH_URL="${1:-http://127.0.0.1:8080/health}"
CONNECT_TIMEOUT="${CONNECT_TIMEOUT:-3}"
MAX_TIME="${MAX_TIME:-10}"

echo "[INFO] health url: ${HEALTH_URL}"

response="$(curl -fsS \
  --connect-timeout "${CONNECT_TIMEOUT}" \
  --max-time "${MAX_TIME}" \
  "${HEALTH_URL}")"

if command -v jq >/dev/null 2>&1; then
  code="$(printf '%s' "${response}" | jq -r '.code // empty')"
  message="$(printf '%s' "${response}" | jq -r '.message // empty')"
else
  code="$(printf '%s' "${response}" | sed -n 's/.*"code"[[:space:]]*:[[:space:]]*\([0-9][0-9]*\).*/\1/p' | head -n 1)"
  message="$(printf '%s' "${response}" | sed -n 's/.*"message"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)"
fi

if [[ -z "${code}" ]]; then
  echo "[ERROR] unable to parse health response: ${response}" >&2
  exit 1
fi

if [[ "${code}" != "200" ]]; then
  echo "[ERROR] health check failed: code=${code}, message=${message}" >&2
  exit 1
fi

echo "[INFO] health check passed: code=${code}, message=${message}"