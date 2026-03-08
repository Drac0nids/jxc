#!/usr/bin/env bash
set -euo pipefail

CLIENT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${CLIENT_DIR}/dist-windows"
TARGET_EXE="${CLIENT_DIR}/src-tauri/target/x86_64-pc-windows-gnu/release/app.exe"
TARGET_WEBVIEW="${CLIENT_DIR}/src-tauri/target/x86_64-pc-windows-gnu/release/WebView2Loader.dll"
ZIP_NAME="jxc-windows-x64-portable.zip"

if ! command -v zip >/dev/null 2>&1; then
  echo "[ERROR] zip command not found" >&2
  exit 1
fi

if ! command -v shasum >/dev/null 2>&1; then
  echo "[ERROR] shasum command not found" >&2
  exit 1
fi

if [[ ! -f "${TARGET_EXE}" ]]; then
  echo "[ERROR] app.exe not found: ${TARGET_EXE}" >&2
  echo "[HINT] run: npm run tauri:build:win:portable" >&2
  exit 1
fi

mkdir -p "${DIST_DIR}"
cp "${TARGET_EXE}" "${DIST_DIR}/app.exe"

if [[ -f "${TARGET_WEBVIEW}" ]]; then
  cp "${TARGET_WEBVIEW}" "${DIST_DIR}/WebView2Loader.dll"
elif [[ ! -f "${DIST_DIR}/WebView2Loader.dll" ]]; then
  echo "[ERROR] WebView2Loader.dll not found in target or dist-windows" >&2
  exit 1
fi

rm -f "${DIST_DIR}/${ZIP_NAME}" "${DIST_DIR}/${ZIP_NAME}.sha256"

(
  cd "${DIST_DIR}"
  zip -9 "${ZIP_NAME}" app.exe WebView2Loader.dll >/dev/null
  shasum -a 256 "${ZIP_NAME}" > "${ZIP_NAME}.sha256"
)

echo "[INFO] portable package ready"
echo "[INFO] exe: ${DIST_DIR}/app.exe"
echo "[INFO] zip: ${DIST_DIR}/${ZIP_NAME}"
echo "[INFO] sha256: ${DIST_DIR}/${ZIP_NAME}.sha256"
