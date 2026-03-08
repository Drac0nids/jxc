#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "[ERROR] DATABASE_URL is required" >&2
  exit 1
fi

BACKUP_DIR="${BACKUP_DIR:-/projects/jxcServer/backups}"
BACKUP_RETENTION_DAYS="${BACKUP_RETENTION_DAYS:-7}"
timestamp="$(date +%Y%m%d_%H%M%S)"
backup_file="${BACKUP_DIR}/jxc_${timestamp}.sql.gz"

mkdir -p "${BACKUP_DIR}"

echo "[INFO] writing backup: ${backup_file}"
pg_dump "${DATABASE_URL}" | gzip -9 > "${backup_file}"

echo "[INFO] backup size: $(du -h "${backup_file}" | awk '{print $1}')"

echo "[INFO] cleaning files older than ${BACKUP_RETENTION_DAYS} days"
find "${BACKUP_DIR}" -type f -name 'jxc_*.sql.gz' -mtime +"${BACKUP_RETENTION_DAYS}" -delete

echo "[INFO] latest backups"
ls -1t "${BACKUP_DIR}" | head -n 10