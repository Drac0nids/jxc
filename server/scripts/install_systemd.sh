#!/usr/bin/env bash
set -euo pipefail

SERVICE_NAME="jxc-server.service"
SERVICE_SOURCE="/projects/jxcServer/deploy/${SERVICE_NAME}"
SERVICE_TARGET="/etc/systemd/system/${SERVICE_NAME}"

run_as_root() {
  if [[ "$(id -u)" -eq 0 ]]; then
    "$@"
    return
  fi

  if sudo -n true 2>/dev/null; then
    sudo -n "$@"
    return
  fi

  echo "[ERROR] sudo permission is required (passwordless sudo recommended for deployment user)" >&2
  exit 1
}

if [[ ! -f "${SERVICE_SOURCE}" ]]; then
  echo "[ERROR] service file not found: ${SERVICE_SOURCE}" >&2
  exit 1
fi

echo "[INFO] installing ${SERVICE_NAME} -> ${SERVICE_TARGET}"
run_as_root cp "${SERVICE_SOURCE}" "${SERVICE_TARGET}"

echo "[INFO] reloading systemd daemon"
run_as_root systemctl daemon-reload

echo "[INFO] enabling ${SERVICE_NAME}"
run_as_root systemctl enable "${SERVICE_NAME}"

echo "[INFO] restarting ${SERVICE_NAME}"
run_as_root systemctl restart "${SERVICE_NAME}"

echo "[INFO] current status"
run_as_root systemctl --no-pager --full status "${SERVICE_NAME}"