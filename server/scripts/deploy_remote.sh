#!/usr/bin/env bash
set -euo pipefail

REMOTE_HOST="${REMOTE_HOST:-ubuntu@1.14.45.242}"
REMOTE_DIR="${REMOTE_DIR:-/projects/jxcServer}"
SERVICE_NAME="${SERVICE_NAME:-jxc-server.service}"
RUN_SMOKE_AUTH_REGISTER="${RUN_SMOKE_AUTH_REGISTER:-0}"
SMOKE_BASE_URL="${SMOKE_BASE_URL:-http://127.0.0.1:8080/api/v1}"
LOCAL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

run_remote_root() {
  local remote_cmd="$1"
  ssh "${REMOTE_HOST}" "if sudo -n true 2>/dev/null; then sudo -n bash -lc '${remote_cmd}'; else echo '[ERROR] passwordless sudo required on remote' >&2; exit 1; fi"
}

should_run_smoke_auth_register() {
  local normalized
  normalized="$(printf '%s' "${RUN_SMOKE_AUTH_REGISTER}" | tr '[:upper:]' '[:lower:]')"
  case "${normalized}" in
    1|true|yes|y)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

echo "[INFO] local_dir=${LOCAL_DIR}"
echo "[INFO] remote=${REMOTE_HOST}:${REMOTE_DIR}"

echo "[INFO] rsync source code"
rsync -az --delete --exclude target --exclude .git --exclude .env \
  "${LOCAL_DIR}/" "${REMOTE_HOST}:${REMOTE_DIR}/"

echo "[INFO] ensure remote .env and required keys"
ssh "${REMOTE_HOST}" "cd ${REMOTE_DIR} && \
  if [[ ! -f .env ]]; then cp .env.example .env; fi && \
  grep -q '^STORAGE_BACKEND=' .env || echo 'STORAGE_BACKEND=postgres' >> .env && \
  grep -q '^DATABASE_URL=' .env || echo 'DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/jxc' >> .env && \
  grep -q '^PG_MAX_CONNECTIONS=' .env || echo 'PG_MAX_CONNECTIONS=10' >> .env"

echo "[INFO] remote cargo test"
ssh "${REMOTE_HOST}" "export PATH=/home/ubuntu/.cargo/bin:\$PATH && cd ${REMOTE_DIR} && cargo test"

echo "[INFO] remote cargo build --release"
ssh "${REMOTE_HOST}" "export PATH=/home/ubuntu/.cargo/bin:\$PATH && cd ${REMOTE_DIR} && cargo build --release"

echo "[INFO] install/restart systemd service"
ssh "${REMOTE_HOST}" "cd ${REMOTE_DIR} && bash scripts/install_systemd.sh"

echo "[INFO] remote health check"
ssh "${REMOTE_HOST}" "cd ${REMOTE_DIR} && bash scripts/healthcheck.sh http://127.0.0.1:8080/health"

if should_run_smoke_auth_register; then
  echo "[INFO] remote auth/register smoke check"
  ssh "${REMOTE_HOST}" "cd ${REMOTE_DIR} && bash scripts/smoke_auth_register.sh '${SMOKE_BASE_URL}'"
else
  echo "[INFO] skip auth/register smoke check (set RUN_SMOKE_AUTH_REGISTER=1 to enable)"
fi

echo "[INFO] remote service status"
run_remote_root "systemctl --no-pager --full status ${SERVICE_NAME}"