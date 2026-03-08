#!/usr/bin/env bash
set -euo pipefail

API_BASE_URL="${1:-http://127.0.0.1:8080/api/v1}"
CONNECT_TIMEOUT="${CONNECT_TIMEOUT:-3}"
MAX_TIME="${MAX_TIME:-15}"

HTTP_STATUS=""
HTTP_BODY=""

log_info() {
  echo "[INFO] $*"
}

log_error() {
  echo "[ERROR] $*" >&2
}

json_get() {
  local json="$1"
  local path="$2"

  if command -v jq >/dev/null 2>&1; then
    printf '%s' "${json}" | jq -r ".${path} // empty"
    return
  fi

  python3 - "${path}" "${json}" <<'PY'
import json
import sys

path = sys.argv[1]
raw = sys.argv[2]

try:
    current = json.loads(raw)
except Exception:
    print("")
    raise SystemExit(0)

for part in path.split('.'):
    if not part:
        continue
    if isinstance(current, dict):
        current = current.get(part)
    elif isinstance(current, list):
        try:
            current = current[int(part)]
        except Exception:
            current = None
    else:
        current = None

    if current is None:
        print("")
        raise SystemExit(0)

if isinstance(current, bool):
    print("true" if current else "false")
elif isinstance(current, (dict, list)):
    print(json.dumps(current, ensure_ascii=False))
else:
    print(current)
PY
}

call_api() {
  local method="$1"
  local url="$2"
  local payload="${3:-}"
  local access_token="${4:-}"

  local -a curl_args
  curl_args=(
    -sS
    --connect-timeout "${CONNECT_TIMEOUT}"
    --max-time "${MAX_TIME}"
    -X "${method}"
    "${url}"
    -H "Accept: application/json"
  )

  if [[ -n "${payload}" ]]; then
    curl_args+=(
      -H "Content-Type: application/json"
      -d "${payload}"
    )
  fi

  if [[ -n "${access_token}" ]]; then
    curl_args+=(
      -H "Authorization: Bearer ${access_token}"
    )
  fi

  local response
  response="$(curl "${curl_args[@]}" -w $'\n%{http_code}')"

  HTTP_STATUS="$(printf '%s' "${response}" | tail -n1)"
  HTTP_BODY="$(printf '%s' "${response}" | sed '$d')"

  if [[ -z "${HTTP_BODY}" ]]; then
    HTTP_BODY='{}'
  fi
}

assert_http_and_code() {
  local step="$1"
  local expected_http="$2"
  local expected_code="$3"
  local actual_code

  actual_code="$(json_get "${HTTP_BODY}" "code")"
  if [[ "${HTTP_STATUS}" != "${expected_http}" || "${actual_code}" != "${expected_code}" ]]; then
    log_error "${step} 失败: http=${HTTP_STATUS}, code=${actual_code}, body=${HTTP_BODY}"
    exit 1
  fi
}

require_non_empty() {
  local step="$1"
  local value="$2"
  if [[ -z "${value}" ]]; then
    log_error "${step} 失败: 字段为空"
    exit 1
  fi
}

ts="$(date +%Y%m%d%H%M%S)"
suffix="${RANDOM}"
tenant_name="冒烟租户-${ts}"
username="smoke_owner_${ts}_${suffix}"
display_name="冒烟管理员"
password="SmokePass_${suffix}"

register_payload="$(cat <<JSON
{"tenant_name":"${tenant_name}","username":"${username}","name":"${display_name}","password":"${password}"}
JSON
)"

log_info "step=register username=${username}"
call_api "POST" "${API_BASE_URL}/auth/register" "${register_payload}"
assert_http_and_code "register" "200" "200"

register_access_token="$(json_get "${HTTP_BODY}" "data.access_token")"
register_refresh_token="$(json_get "${HTTP_BODY}" "data.refresh_token")"
register_tenant_id="$(json_get "${HTTP_BODY}" "data.tenant_id")"
register_role="$(json_get "${HTTP_BODY}" "data.user_info.role")"

require_non_empty "register.access_token" "${register_access_token}"
require_non_empty "register.refresh_token" "${register_refresh_token}"
require_non_empty "register.tenant_id" "${register_tenant_id}"

if [[ "${register_role}" != "OWNER" ]]; then
  log_error "register 失败: 期望 role=OWNER, 实际=${register_role}"
  exit 1
fi

login_payload="$(cat <<JSON
{"username":"${username}","password":"${password}"}
JSON
)"

log_info "step=login username=${username}"
call_api "POST" "${API_BASE_URL}/auth/login" "${login_payload}"
assert_http_and_code "login" "200" "200"

login_access_token="$(json_get "${HTTP_BODY}" "data.access_token")"
login_refresh_token="$(json_get "${HTTP_BODY}" "data.refresh_token")"
require_non_empty "login.access_token" "${login_access_token}"
require_non_empty "login.refresh_token" "${login_refresh_token}"

refresh_payload="$(cat <<JSON
{"refresh_token":"${login_refresh_token}"}
JSON
)"

log_info "step=refresh"
call_api "POST" "${API_BASE_URL}/auth/refresh" "${refresh_payload}"
assert_http_and_code "refresh" "200" "200"

refreshed_access_token="$(json_get "${HTTP_BODY}" "data.access_token")"
require_non_empty "refresh.access_token" "${refreshed_access_token}"

log_info "step=logout"
call_api "POST" "${API_BASE_URL}/auth/logout" "" "${refreshed_access_token}"
assert_http_and_code "logout" "200" "200"

logged_out="$(json_get "${HTTP_BODY}" "data.logged_out")"
if [[ "${logged_out}" != "true" ]]; then
  log_error "logout 失败: 期望 logged_out=true, 实际=${logged_out}"
  exit 1
fi

log_info "step=register_conflict username=${username}"
call_api "POST" "${API_BASE_URL}/auth/register" "${register_payload}"
assert_http_and_code "register_conflict" "409" "4090"

conflict_username="$(json_get "${HTTP_BODY}" "data.username")"
if [[ -n "${conflict_username}" && "${conflict_username}" != "${username}" ]]; then
  log_error "register_conflict 失败: 期望冲突用户名=${username}, 实际=${conflict_username}"
  exit 1
fi

log_info "auth register smoke passed tenant_id=${register_tenant_id} username=${username}"
