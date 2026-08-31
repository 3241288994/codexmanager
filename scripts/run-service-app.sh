#!/usr/bin/env bash
set -euo pipefail

BUILD_PROFILE="release"
CLEAN_DIST=false
NO_OPEN=false

usage() {
  cat <<'EOF'
Usage: scripts/run-service-app.sh [options]

Build the frontend, embed it into codexmanager-web, then start codexmanager-start.

Options:
  --debug       Use a debug Rust build.
  --release     Use a release Rust build. This is the default.
  --clean-dist  Remove apps/out before building the frontend.
  --no-open     Do not open the browser automatically.
  -h, --help    Show this help text.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --debug)
      BUILD_PROFILE="debug"
      shift
      ;;
    --release)
      BUILD_PROFILE="release"
      shift
      ;;
    --clean-dist)
      CLEAN_DIST=true
      shift
      ;;
    --no-open)
      NO_OPEN=true
      shift
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
APPS_ROOT="$ROOT/apps"
DIST_DIR="$APPS_ROOT/out"
TARGET_ROOT="${CARGO_TARGET_DIR:-$ROOT/target}"

cd "$ROOT"

step() { echo "$*"; }

command -v cargo >/dev/null 2>&1 || { echo "cargo not found in PATH" >&2; exit 1; }
command -v pnpm >/dev/null 2>&1 || { echo "pnpm not found in PATH" >&2; exit 1; }

if [[ "$CLEAN_DIST" == "true" ]]; then
  step "remove $DIST_DIR"
  rm -rf "$DIST_DIR"
fi

if [[ ! -d "$APPS_ROOT/node_modules" ]]; then
  step "install frontend dependencies"
  pnpm -C "$APPS_ROOT" install --frozen-lockfile
fi

step "build frontend static assets"
pnpm -C "$APPS_ROOT" run build:desktop

if [[ ! -f "$DIST_DIR/index.html" ]]; then
  echo "frontend build failed: $DIST_DIR/index.html not found" >&2
  exit 1
fi

cargo_args=(-p codexmanager-service -p codexmanager-web -p codexmanager-start)
if [[ "$BUILD_PROFILE" == "release" ]]; then
  cargo_args=(--release "${cargo_args[@]}")
fi

step "build service app binaries ($BUILD_PROFILE, embedded frontend)"
cargo build "${cargo_args[@]}"

BIN_DIR="$TARGET_ROOT/$BUILD_PROFILE"
START_BIN="$BIN_DIR/codexmanager-start"
if [[ ! -x "$START_BIN" ]]; then
  echo "start binary not found: $START_BIN" >&2
  exit 1
fi

if [[ "$NO_OPEN" == "true" ]]; then
  export CODEXMANAGER_WEB_NO_OPEN=1
fi

step "start CodexManager service app"
exec "$START_BIN"
