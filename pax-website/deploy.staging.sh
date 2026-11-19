#!/usr/bin/env bash
set -euo pipefail

website_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "$website_dir/tools/deploy_staging.py" "$@"
