#!/usr/bin/env bash
set -euo pipefail

LISTEN="${CURL_CFFI_PROXY_LISTEN:-127.0.0.1:8787}"
UPSTREAM_ORIGIN="${CURL_CFFI_PROXY_UPSTREAM_ORIGIN:-https://chatgpt.com}"
IMPERSONATE="${CURL_CFFI_PROXY_IMPERSONATE:-chrome124}"
PROXY_URL="${CURL_CFFI_PROXY_URL:-socks5h://127.0.0.1:40000}"
PYTHON_BIN="${PYTHON_BIN:-python3}"
INSTALL_DEPS=false
VERBOSE=false
NO_UPSTREAM_PROXY=false
PRINT_ENV=false

usage() {
  cat <<'EOF'
Usage:
  scripts/run-curl-cffi-chatgpt-proxy.sh [options]

Options:
  --listen HOST:PORT         Local listen address (default: 127.0.0.1:8787)
  --proxy URL                Upstream proxy for curl_cffi (default: socks5h://127.0.0.1:40000)
  --no-upstream-proxy        Connect to chatgpt.com directly without an extra proxy hop
  --upstream-origin URL      Upstream origin (default: https://chatgpt.com)
  --impersonate PROFILE      curl_cffi impersonation profile (default: chrome124)
  --python BIN               Python executable to use (default: python3)
  --install-deps             Install curl_cffi if it is missing
  --verbose                  Enable verbose proxy logging
  --print-env                Print the recommended CodexManager env snippet and exit
EOF
}

step() {
  printf '%s\n' "$*"
}

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: required command not found: $1" >&2
    exit 1
  fi
}

ensure_python_dependency() {
  if "$PYTHON_BIN" -c 'import curl_cffi' >/dev/null 2>&1; then
    return
  fi
  if [[ "$INSTALL_DEPS" != "true" ]]; then
    echo "error: python package 'curl_cffi' is missing; rerun with --install-deps" >&2
    exit 1
  fi
  step "installing python dependency: curl_cffi"
  "$PYTHON_BIN" -m pip install --upgrade curl_cffi
}

print_env_snippet() {
  cat <<EOF
CODEXMANAGER_UPSTREAM_BASE_URL=http://${LISTEN}/backend-api/codex
CODEXMANAGER_GATEWAY_MODE=enhanced
# Leave CODEXMANAGER_UPSTREAM_PROXY_URL unset.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --listen)
      LISTEN="${2:-}"
      shift 2
      ;;
    --proxy)
      PROXY_URL="${2:-}"
      shift 2
      ;;
    --no-upstream-proxy)
      NO_UPSTREAM_PROXY=true
      shift
      ;;
    --upstream-origin)
      UPSTREAM_ORIGIN="${2:-}"
      shift 2
      ;;
    --impersonate)
      IMPERSONATE="${2:-}"
      shift 2
      ;;
    --python)
      PYTHON_BIN="${2:-}"
      shift 2
      ;;
    --install-deps)
      INSTALL_DEPS=true
      shift
      ;;
    --verbose)
      VERBOSE=true
      shift
      ;;
    --print-env)
      PRINT_ENV=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ "$PRINT_ENV" == "true" ]]; then
  print_env_snippet
  exit 0
fi

need_cmd "$PYTHON_BIN"
ensure_python_dependency

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PY_SCRIPT="$SCRIPT_DIR/curl_cffi_chatgpt_proxy.py"

if [[ ! -f "$PY_SCRIPT" ]]; then
  echo "error: proxy entry script not found: $PY_SCRIPT" >&2
  exit 1
fi

cmd=(
  "$PYTHON_BIN" "$PY_SCRIPT"
  --listen "$LISTEN"
  --upstream-origin "$UPSTREAM_ORIGIN"
  --impersonate "$IMPERSONATE"
)

if [[ "$NO_UPSTREAM_PROXY" != "true" ]]; then
  cmd+=(--proxy "$PROXY_URL")
fi
if [[ "$VERBOSE" == "true" ]]; then
  cmd+=(--verbose)
fi

step "launching curl_cffi ChatGPT proxy"
if [[ "$NO_UPSTREAM_PROXY" == "true" ]]; then
  step "  upstream proxy: <direct>"
else
  step "  upstream proxy: $PROXY_URL"
fi
step "  listen: http://$LISTEN"
step "  suggested codexmanager.env:"
print_env_snippet | sed 's/^/    /'

exec "${cmd[@]}"
