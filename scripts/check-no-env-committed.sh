#!/usr/bin/env bash
# Run before push: ensures .env is not tracked by git.
set -euo pipefail
blocked=$(git ls-files '.env' '.env.*' 2>/dev/null | grep -v '\.env\.example$' || true)
if [[ -n "$blocked" ]]; then
  echo "BLOCKED: secret env files are tracked by git:" >&2
  echo "$blocked" >&2
  exit 1
fi
echo "OK — .env is not in git"
