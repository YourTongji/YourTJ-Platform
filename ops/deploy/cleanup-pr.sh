#!/usr/bin/env bash
set -euo pipefail

readonly PR_NUMBER="${1:-}"
readonly PREVIEW_ROOT="${PREVIEW_ROOT:-/opt/yourtj-preview/pr}"
readonly PREVIEW_DB_HOST="${PREVIEW_DB_HOST:-127.0.0.1}"
readonly PREVIEW_DB_PORT="${PREVIEW_DB_PORT:-5433}"
readonly PREVIEW_DB_USER="${PREVIEW_DB_USER:-yourtj_preview}"
readonly PREVIEW_PGPASS_FILE="${PREVIEW_PGPASS_FILE:-${HOME}/.pgpass}"

if [[ ! "$PR_NUMBER" =~ ^[1-9][0-9]{0,2}$ ]]; then
  echo "invalid PR number" >&2
  exit 64
fi
if [[ ! -f "$PREVIEW_PGPASS_FILE" || -L "$PREVIEW_PGPASS_FILE" \
  || "$(stat -c '%a' "$PREVIEW_PGPASS_FILE")" != "600" ]]; then
  echo "preview .pgpass must be a regular mode-600 file" >&2
  exit 78
fi
export PGPASSFILE="$PREVIEW_PGPASS_FILE"

docker rm -f "pr-${PR_NUMBER}-fe" "pr-${PR_NUMBER}-be" >/dev/null 2>&1 || true
docker image rm "yourtj-api:pr-${PR_NUMBER}" >/dev/null 2>&1 || true
rm -rf "${PREVIEW_ROOT:?}/${PR_NUMBER}"

# libpq reads the database credential from the preview operator's protected password file.
psql \
  --host "$PREVIEW_DB_HOST" \
  --port "$PREVIEW_DB_PORT" \
  --username "$PREVIEW_DB_USER" \
  --dbname postgres \
  --set ON_ERROR_STOP=1 \
  --command "DROP DATABASE IF EXISTS \"yourtj_pr_${PR_NUMBER}\" WITH (FORCE)"
