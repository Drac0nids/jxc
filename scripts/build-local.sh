#!/usr/bin/env bash
# =============================================================================
# build-local.sh — Build the JXC standalone Windows executable (local mode)
#
# This script:
#   1. Compiles the Rust server for the target platform
#   2. Copies it to client/src-tauri/binaries/ with the Tauri-expected name
#   3. Builds the Vite frontend
#   4. Runs `cargo tauri build` to bundle everything
#
# Usage (on macOS for native dev/test):
#   ./scripts/build-local.sh
#
# Usage (cross-compile to Windows, requires `cross`):
#   TARGET=x86_64-pc-windows-gnu ./scripts/build-local.sh
# =============================================================================
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SERVER_DIR="$REPO_ROOT/server"
CLIENT_DIR="$REPO_ROOT/client"
BINARIES_DIR="$CLIENT_DIR/src-tauri/binaries"

TARGET="${TARGET:-}"          # e.g. x86_64-pc-windows-gnu for cross-compile
CROSS="${CROSS:-false}"       # set to "true" to use `cross` instead of `cargo`

# ── 1. Determine host triple ──────────────────────────────────────────────────
if [[ -z "$TARGET" ]]; then
  TARGET=$(rustc -vV | grep "host:" | awk '{print $2}')
  echo "[build] Native target: $TARGET"
else
  echo "[build] Cross-compile target: $TARGET"
fi

# ── 2. Build server binary ────────────────────────────────────────────────────
echo "[build] Compiling server..."
cd "$SERVER_DIR"

if [[ "$CROSS" == "true" ]]; then
  cross build --release --target "$TARGET"
else
  cargo build --release --target "$TARGET"
fi

# ── 3. Copy sidecar to binaries/ ─────────────────────────────────────────────
mkdir -p "$BINARIES_DIR"

# Detect binary extension (Windows needs .exe)
if [[ "$TARGET" == *"windows"* ]]; then
  EXT=".exe"
else
  EXT=""
fi

SRC="$SERVER_DIR/target/$TARGET/release/server$EXT"
DST="$BINARIES_DIR/server-$TARGET$EXT"

echo "[build] Copying $SRC -> $DST"
cp "$SRC" "$DST"

# ── 4. Build frontend ─────────────────────────────────────────────────────────
echo "[build] Building Vite frontend..."
cd "$CLIENT_DIR"
npm run build

# ── 5. Bundle with Tauri ──────────────────────────────────────────────────────
echo "[build] Bundling with Tauri..."
cd "$CLIENT_DIR"

if [[ -z "$TARGET" || "$TARGET" == "$(rustc -vV | grep host | awk '{print $2}')" ]]; then
  npx tauri build
else
  npx tauri build --target "$TARGET"
fi

echo ""
echo "✅ Build complete!"
echo "   Bundles: $CLIENT_DIR/src-tauri/target/$TARGET/release/bundle/"
