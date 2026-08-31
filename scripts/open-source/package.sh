#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
release_name="${1:-CodexManager-open-source}"
output_dir="${2:-$repo_dir/dist-open-source}"
archive="$output_dir/$release_name.tar.gz"

if [[ ! "$release_name" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]]; then
  echo "release name must contain only letters, numbers, dots, underscores, and hyphens" >&2
  exit 1
fi

"$repo_dir/scripts/open-source/preflight.sh" "$repo_dir"
mkdir -p "$output_dir"
if [[ -e "$archive" ]]; then
  echo "refusing to overwrite existing archive: $archive" >&2
  exit 1
fi

SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-0}" \
  python3 "$repo_dir/scripts/open-source/package.py" \
    --root "$repo_dir" \
    --output "$archive"

if command -v sha256sum >/dev/null 2>&1; then
  sha256sum "$archive" > "$archive.sha256"
elif command -v shasum >/dev/null 2>&1; then
  shasum -a 256 "$archive" > "$archive.sha256"
else
  echo "sha256sum or shasum is required to write a checksum" >&2
  exit 1
fi
echo "$archive"
