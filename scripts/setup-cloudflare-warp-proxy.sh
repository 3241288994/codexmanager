#!/usr/bin/env bash
set -euo pipefail

PORT=40000
SKIP_INSTALL=false
DISCONNECT_FIRST=false

usage() {
  cat <<'EOF'
Usage:
  scripts/setup-cloudflare-warp-proxy.sh [--port 40000] [--skip-install] [--disconnect-first]

What this script does:
  1. Installs Cloudflare WARP on Ubuntu/Debian (unless --skip-install is used)
  2. Registers the local device if needed
  3. Switches WARP into local proxy mode
  4. Sets the proxy port (default: 40000)
  5. Connects WARP and prints the next-step commands for CodexManager

Notes:
  - This script intentionally uses local proxy mode instead of full-tunnel mode.
  - It tries both new and legacy warp-cli subcommands to stay compatible with different client versions.
EOF
}

step() {
  printf '%s\n' "$*"
}

have_cmd() {
  command -v "$1" >/dev/null 2>&1
}

need_cmd() {
  if ! have_cmd "$1"; then
    echo "error: required command not found: $1" >&2
    exit 1
  fi
}

as_root() {
  if [[ "${EUID:-$(id -u)}" -eq 0 ]]; then
    "$@"
  else
    sudo "$@"
  fi
}

detect_codename() {
  if have_cmd lsb_release; then
    lsb_release -cs
    return
  fi
  if [[ -r /etc/os-release ]]; then
    # shellcheck disable=SC1091
    . /etc/os-release
    if [[ -n "${VERSION_CODENAME:-}" ]]; then
      printf '%s\n' "$VERSION_CODENAME"
      return
    fi
    if [[ -n "${UBUNTU_CODENAME:-}" ]]; then
      printf '%s\n' "$UBUNTU_CODENAME"
      return
    fi
  fi
  echo "error: unable to detect distro codename" >&2
  exit 1
}

install_cloudflare_warp() {
  local codename arch repo_line
  codename="$(detect_codename)"
  arch="$(dpkg --print-architecture)"
  repo_line="deb [arch=${arch} signed-by=/usr/share/keyrings/cloudflare-warp-archive-keyring.gpg] https://pkg.cloudflareclient.com/ ${codename} main"

  step "installing Cloudflare WARP apt repo for ${codename}/${arch}"
  need_cmd curl
  need_cmd gpg
  as_root mkdir -p /usr/share/keyrings /etc/apt/sources.list.d
  curl -fsSL https://pkg.cloudflareclient.com/pubkey.gpg \
    | as_root gpg --yes --dearmor --output /usr/share/keyrings/cloudflare-warp-archive-keyring.gpg
  printf '%s\n' "$repo_line" | as_root tee /etc/apt/sources.list.d/cloudflare-client.list >/dev/null
  as_root apt-get update
  as_root apt-get install -y cloudflare-warp
}

warp_cli() {
  warp-cli --accept-tos "$@"
}

warp_try() {
  if warp_cli "$@" >/dev/null 2>&1; then
    return 0
  fi
  return 1
}

ensure_registered() {
  if warp_try registration show; then
    step "WARP device registration already exists"
    return
  fi
  if warp_try registration new; then
    step "created a new WARP device registration"
    return
  fi
  if warp_try register; then
    step "created a new WARP device registration via legacy command"
    return
  fi
  echo "error: failed to register WARP device" >&2
  exit 1
}

set_proxy_mode() {
  if warp_try tunnel protocol set MASQUE; then
    step "set WARP tunnel protocol to MASQUE"
  else
    step "warning: failed to force MASQUE via CLI; continuing anyway"
  fi

  if warp_try mode proxy; then
    step "switched WARP to proxy mode"
    return
  fi
  if warp_try set-mode proxy; then
    step "switched WARP to proxy mode via legacy command"
    return
  fi
  echo "error: failed to switch WARP into proxy mode" >&2
  exit 1
}

set_proxy_port() {
  local port="$1"
  if warp_try proxy port "$port"; then
    step "set WARP proxy port to ${port}"
    return
  fi
  if warp_try set-proxy-port "$port"; then
    step "set WARP proxy port to ${port} via legacy command"
    return
  fi
  echo "error: failed to set WARP proxy port to ${port}" >&2
  exit 1
}

connect_warp() {
  if [[ "$DISCONNECT_FIRST" == "true" ]]; then
    warp_try disconnect || true
  fi
  if warp_try connect; then
    step "connected WARP"
    return
  fi
  echo "error: failed to connect WARP" >&2
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --port)
      PORT="${2:-}"
      shift 2
      ;;
    --skip-install)
      SKIP_INSTALL=true
      shift
      ;;
    --disconnect-first)
      DISCONNECT_FIRST=true
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

if [[ ! "$PORT" =~ ^[0-9]+$ ]] || (( PORT < 1 || PORT > 65535 )); then
  echo "error: --port must be an integer between 1 and 65535" >&2
  exit 2
fi

need_cmd dpkg

if [[ "$SKIP_INSTALL" != "true" ]]; then
  install_cloudflare_warp
fi

need_cmd warp-cli

ensure_registered
set_proxy_mode
set_proxy_port "$PORT"
connect_warp

step ""
step "WARP status:"
warp_cli status || true

step ""
step "Suggested next steps:"
step "  1. Start the local curl_cffi proxy:"
step "     scripts/run-curl-cffi-chatgpt-proxy.sh --proxy socks5h://127.0.0.1:${PORT} --install-deps"
step "  2. Set these values in codexmanager.env:"
step "     CODEXMANAGER_UPSTREAM_BASE_URL=http://127.0.0.1:8787/backend-api/codex"
step "     CODEXMANAGER_GATEWAY_MODE=enhanced"
step "  3. Leave CODEXMANAGER_UPSTREAM_PROXY_URL unset to avoid double-proxying local requests"
