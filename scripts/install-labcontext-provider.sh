#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd "$script_dir/.." && pwd)"
provider_dir="$repo_dir/labcontext-provider"
install_root="${LABCONTEXT_PROVIDER_INSTALL_ROOT:-$HOME/.local/opt/labcontext-provider}"
python_bin="${LABCONTEXT_PROVIDER_PYTHON:-python3}"
venv_dir="$install_root/.venv"

if [[ ! -f "$provider_dir/pyproject.toml" ]]; then
  echo "LabContext Provider source is missing: $provider_dir" >&2
  exit 1
fi

"$python_bin" -c 'import sys; raise SystemExit(0 if sys.version_info >= (3, 11) else 1)' || {
  echo "LabContext Provider requires Python 3.11 or newer: $python_bin" >&2
  exit 1
}

mkdir -p "$install_root"
if [[ ! -x "$venv_dir/bin/python" ]]; then
  "$python_bin" -m venv "$venv_dir"
fi
"$venv_dir/bin/python" -m pip install --disable-pip-version-check --upgrade "$provider_dir"

echo "installed LabContext Provider to $venv_dir"
echo "example config: $provider_dir/labcontext.example.toml"
echo "command: $venv_dir/bin/labctx --config ~/.config/labcontext/provider.toml serve"
