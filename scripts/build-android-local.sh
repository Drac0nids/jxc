#!/usr/bin/env bash
# =============================================================================
# build-android-local.sh — 构建 Android 单机版 APK（服务端进程内嵌）
#
# 流程：
#   1. 用 cargo-ndk 把 Rust 服务端编译为各 ABI 的 libjxc_server.so
#   2. 复制到 app/android/app/src/main/jniLibs/<abi>/
#   3. 用 Flutter 打包 APK（内嵌 .so 随包分发）
#
# 依赖：
#   rustup target add aarch64-linux-android armv7-linux-androideabi
#   cargo install cargo-ndk
#   ANDROID_NDK_HOME 指向 NDK（默认取本机 Homebrew cmdline-tools 下的 NDK）
#
# 用法：
#   ./scripts/build-android-local.sh                 # arm64 + armv7
#   ABIS=arm64-v8a ./scripts/build-android-local.sh  # 只编 arm64（更快）
# =============================================================================
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SERVER_DIR="$REPO_ROOT/server"
APP_DIR="$REPO_ROOT/app"
JNI_LIBS_DIR="$APP_DIR/android/app/src/main/jniLibs"

ABIS="${ABIS:-arm64-v8a armeabi-v7a}"

if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  NDK_CANDIDATE="$(ls -d /opt/homebrew/share/android-commandlinetools/ndk/* 2>/dev/null | tail -1 || true)"
  if [[ -n "$NDK_CANDIDATE" ]]; then
    export ANDROID_NDK_HOME="$NDK_CANDIDATE"
  else
    echo "[error] 未找到 NDK，请设置 ANDROID_NDK_HOME" >&2
    exit 1
  fi
fi
echo "[build] NDK: $ANDROID_NDK_HOME"

# ── 1. 编译服务端为各 ABI 的 .so ──────────────────────────────────────────────
echo "[build] 编译服务端（ABI: ${ABIS})..."
cd "$SERVER_DIR"
NDK_TARGETS=()
for abi in $ABIS; do
  NDK_TARGETS+=(-t "$abi")
done
cargo ndk "${NDK_TARGETS[@]}" -o "$JNI_LIBS_DIR" build --release --lib

echo "[build] 产物："
find "$JNI_LIBS_DIR" -name "libjxc_server.so" -exec ls -lh {} \;

# ── 2. 打包 APK ───────────────────────────────────────────────────────────────
echo "[build] 打包 APK..."
cd "$APP_DIR"
flutter pub get

FLUTTER_PLATFORMS=""
for abi in $ABIS; do
  case "$abi" in
    arm64-v8a)   FLUTTER_PLATFORMS="${FLUTTER_PLATFORMS:+$FLUTTER_PLATFORMS,}android-arm64" ;;
    armeabi-v7a) FLUTTER_PLATFORMS="${FLUTTER_PLATFORMS:+$FLUTTER_PLATFORMS,}android-arm" ;;
    x86_64)      FLUTTER_PLATFORMS="${FLUTTER_PLATFORMS:+$FLUTTER_PLATFORMS,}android-x64" ;;
  esac
done

flutter build apk --release --target-platform "$FLUTTER_PLATFORMS"

APK="$APP_DIR/build/app/outputs/flutter-apk/app-release.apk"
echo ""
echo "✅ 构建完成：$APK"
ls -lh "$APK"
