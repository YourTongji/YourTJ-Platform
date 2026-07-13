#!/usr/bin/env bash
set -Eeuo pipefail

readonly PGPASS_FILE="${PREVIEW_PGPASS_FILE:-${HOME}/.pgpass}"
readonly POSTGRES_HOST="${PREVIEW_POSTGRES_HOST:-127.0.0.1}"
readonly POSTGRES_PORT="${PREVIEW_POSTGRES_PORT:-5433}"
readonly POSTGRES_USER="${PREVIEW_POSTGRES_USER:-yourtj_preview}"
readonly REDIS_URL="${PREVIEW_REDIS_URL:-redis://127.0.0.1:6380}"
readonly MEILI_URL="${PREVIEW_MEILI_URL:-http://127.0.0.1:7701}"
readonly HEALTH_ATTEMPTS="${DEPLOY_HEALTH_ATTEMPTS:-60}"
readonly HEALTH_DELAY_SECONDS="${DEPLOY_HEALTH_DELAY_SECONDS:-1}"
readonly REVISION_LABEL_TEMPLATE='{{index .Config.Labels "org.opencontainers.image.revision"}}'

usage() {
  echo "usage: deploy-pr.sh <pr_number> <frontend_dist_dir> <git_revision> <frontend_nginx_template>" >&2
  exit 64
}

fail() {
  echo "deploy-pr: $*" >&2
  exit 1
}

container_exists() {
  docker container inspect "$1" >/dev/null 2>&1
}

wait_for_url() {
  local description="$1"
  local url="$2"
  local attempt
  for ((attempt = 1; attempt <= HEALTH_ATTEMPTS; attempt += 1)); do
    if curl --fail --silent --show-error --max-time 3 "$url" >/dev/null 2>&1; then
      echo "  ${description}: OK"
      return 0
    fi
    sleep "$HEALTH_DELAY_SECONDS"
  done
  fail "${description} failed after ${HEALTH_ATTEMPTS} attempts"
}

preview_port() {
  local prefix="$1"
  local pr_number="$2"
  if ((pr_number < 10)); then
    printf '%s00%s' "$prefix" "$pr_number"
  elif ((pr_number < 100)); then
    printf '%s0%s' "$prefix" "$pr_number"
  else
    printf '%s%s' "$prefix" "$pr_number"
  fi
}

[[ "$#" -eq 4 ]] || usage
[[ "$1" =~ ^[1-9][0-9]{0,2}$ ]] || fail "PR number must be between 1 and 999"
[[ "$3" =~ ^[0-9a-f]{40}$ ]] || fail "git revision must be a full commit SHA"
[[ "$HEALTH_ATTEMPTS" =~ ^[1-9][0-9]*$ ]] || fail "health attempts must be positive"
[[ "$HEALTH_DELAY_SECONDS" =~ ^[0-9]+([.][0-9]+)?$ ]] || fail "health delay must be numeric"

readonly PR_NUMBER="$1"
readonly FRONTEND_DIR="$2"
readonly GIT_REVISION="$3"
readonly NGINX_TEMPLATE="$4"
readonly DATABASE_NAME="yourtj_pr_${PR_NUMBER}"
FRONTEND_PORT="$(preview_port 15 "$PR_NUMBER")"
readonly FRONTEND_PORT
BACKEND_PORT="$(preview_port 16 "$PR_NUMBER")"
readonly BACKEND_PORT
readonly FRONTEND_CONTAINER="pr-${PR_NUMBER}-fe"
readonly BACKEND_CONTAINER="pr-${PR_NUMBER}-be"
readonly BACKEND_IMAGE="yourtj-api:pr-${PR_NUMBER}"
readonly EXPECTED_FRONTEND_PATTERN="^/opt/yourtj-preview/pr/${PR_NUMBER}/releases/${GIT_REVISION}-[1-9][0-9]*-[1-9][0-9]*/frontend$"
readonly RENDERED_NGINX="${FRONTEND_DIR}/nginx.conf"
readonly BACKEND_SECRET_FILE="/opt/yourtj-preview/pr/${PR_NUMBER}/backend-secrets.env"
BACKUP_SUFFIX="$(date +%s)-$$"
readonly BACKUP_SUFFIX
readonly FRONTEND_BACKUP="${FRONTEND_CONTAINER}-rollback-${BACKUP_SUFFIX}"
readonly BACKEND_BACKUP="${BACKEND_CONTAINER}-rollback-${BACKUP_SUFFIX}"

[[ "$FRONTEND_DIR" =~ $EXPECTED_FRONTEND_PATTERN ]] ||
  fail "frontend path is outside the immutable PR release"
[[ -f "${FRONTEND_DIR}/index.html" ]] || fail "frontend index.html is missing"
[[ -f "$NGINX_TEMPLATE" && ! -L "$NGINX_TEMPLATE" ]] || fail "frontend Nginx template is invalid"
[[ -f "$PGPASS_FILE" && ! -L "$PGPASS_FILE" ]] || fail "preview .pgpass is missing"
[[ "$(stat -c '%a' "$PGPASS_FILE")" == "600" ]] || fail "preview .pgpass must have mode 600"
docker image inspect "$BACKEND_IMAGE" >/dev/null 2>&1 || fail "backend image is missing"
grep -q '__MEDIA_CDN_ORIGIN__' "$NGINX_TEMPLATE" || fail "frontend Nginx placeholder is missing"
sed 's|__MEDIA_CDN_ORIGIN__|https://media.invalid|g' "$NGINX_TEMPLATE" > "$RENDERED_NGINX"
if grep -q '__MEDIA_CDN_ORIGIN__' "$RENDERED_NGINX"; then
  fail "frontend Nginx rendering failed"
fi

if [[ ! -f "$BACKEND_SECRET_FILE" ]]; then
  umask 077
  printf 'JWT_SECRET=%s\nCREDIT_SYSTEM_PRIVATE_KEY=%s\n' \
    "$(openssl rand -hex 32)" "$(openssl rand -hex 32)" > "$BACKEND_SECRET_FILE"
fi
[[ -f "$BACKEND_SECRET_FILE" && ! -L "$BACKEND_SECRET_FILE" ]] ||
  fail "preview backend secret file is invalid"
[[ "$(stat -c '%a' "$BACKEND_SECRET_FILE")" == "600" ]] ||
  fail "preview backend secret file must have mode 600"
[[ "$(wc -l < "$BACKEND_SECRET_FILE")" -eq 2 ]] || fail "preview backend secret file is invalid"
grep -Eq '^JWT_SECRET=[0-9a-f]{64}$' "$BACKEND_SECRET_FILE" || fail "preview JWT secret is invalid"
grep -Eq '^CREDIT_SYSTEM_PRIVATE_KEY=[0-9a-f]{64}$' "$BACKEND_SECRET_FILE" ||
  fail "preview credit signing seed is invalid"

export PGPASSFILE="$PGPASS_FILE"
database_exists="$(psql -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" \
  -d postgres -tAc "SELECT 1 FROM pg_database WHERE datname = '${DATABASE_NAME}'")"
if [[ "$database_exists" != "1" ]]; then
  psql -v ON_ERROR_STOP=1 -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" \
    -d postgres -c "CREATE DATABASE \"${DATABASE_NAME}\" OWNER ${POSTGRES_USER};" >/dev/null
fi

backend_backed_up=0
frontend_backed_up=0
backend_new_started=0
backend_new_ready=0
frontend_new_started=0
deployment_succeeded=0
rollback() {
  local status="$?"
  trap - EXIT
  if ((deployment_succeeded == 0)); then
    if ((frontend_new_started == 1)); then
      docker rm -f "$FRONTEND_CONTAINER" >/dev/null 2>&1 || true
    fi
    if ((frontend_backed_up == 1)) && container_exists "$FRONTEND_BACKUP"; then
      docker rename "$FRONTEND_BACKUP" "$FRONTEND_CONTAINER" >/dev/null
      docker start "$FRONTEND_CONTAINER" >/dev/null
    elif ((frontend_new_started == 0)) && container_exists "$FRONTEND_CONTAINER"; then
      docker start "$FRONTEND_CONTAINER" >/dev/null 2>&1 || true
    fi
    if ((backend_new_started == 1)); then
      if ((backend_new_ready == 0)) && container_exists "$BACKEND_CONTAINER"; then
        docker stop "$BACKEND_CONTAINER" >/dev/null 2>&1 || true
      fi
      echo "deploy-pr: forward-only schema cutover; previous backend remains stopped" >&2
      echo "deploy-pr: fix forward with a new preview deployment; do not restart ${BACKEND_BACKUP}" >&2
    elif ((backend_backed_up == 1)) && container_exists "$BACKEND_BACKUP"; then
      docker rename "$BACKEND_BACKUP" "$BACKEND_CONTAINER" >/dev/null
      docker start "$BACKEND_CONTAINER" >/dev/null
    elif container_exists "$BACKEND_CONTAINER"; then
      docker start "$BACKEND_CONTAINER" >/dev/null 2>&1 || true
    fi
  fi
  exit "$status"
}
trap rollback EXIT

if container_exists "$FRONTEND_CONTAINER"; then
  docker stop "$FRONTEND_CONTAINER" >/dev/null
  docker rename "$FRONTEND_CONTAINER" "$FRONTEND_BACKUP"
  frontend_backed_up=1
fi
if container_exists "$BACKEND_CONTAINER"; then
  docker stop "$BACKEND_CONTAINER" >/dev/null
  docker rename "$BACKEND_CONTAINER" "$BACKEND_BACKUP"
  backend_backed_up=1
fi

CREATED_AT="$(date +%s)"
readonly CREATED_AT
docker create \
  --env-file "$BACKEND_SECRET_FILE" \
  --name "$BACKEND_CONTAINER" \
  --network host \
  --restart no \
  --label "org.opencontainers.image.revision=${GIT_REVISION}" \
  --label "yourtj.environment=pr-preview" \
  --label "yourtj-ttl-hrs=24" \
  --label "yourtj-created-at=${CREATED_AT}" \
  -v "${PGPASS_FILE}:/run/secrets/preview.pgpass:ro" \
  -e "PGPASSFILE=/run/secrets/preview.pgpass" \
  -e "BIND_ADDRESS=127.0.0.1" \
  -e "PORT=${BACKEND_PORT}" \
  -e "DATABASE_URL=postgres://${POSTGRES_USER}@${POSTGRES_HOST}:${POSTGRES_PORT}/${DATABASE_NAME}" \
  -e "DATABASE_REPLICA_URL=" \
  -e "REDIS_URL=${REDIS_URL}" \
  -e "MEILI_URL=${MEILI_URL}" \
  -e "EMAIL_PROVIDER=log" \
  -e "RUST_LOG=info" \
  "$BACKEND_IMAGE" >/dev/null
backend_new_started=1
docker start "$BACKEND_CONTAINER" >/dev/null
backend_environment=$(docker inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$BACKEND_CONTAINER")
grep -Fxq "BIND_ADDRESS=127.0.0.1" <<<"$backend_environment" ||
  fail "preview backend must bind only to loopback on the shared host"

docker run -d \
  --name "$FRONTEND_CONTAINER" \
  --restart no \
  --label "org.opencontainers.image.revision=${GIT_REVISION}" \
  --label "yourtj.environment=pr-preview" \
  --label "yourtj-ttl-hrs=24" \
  --label "yourtj-created-at=${CREATED_AT}" \
  -p "127.0.0.1:${FRONTEND_PORT}:80" \
  -v "${FRONTEND_DIR}:/usr/share/nginx/html:ro" \
  -v "${RENDERED_NGINX}:/etc/nginx/conf.d/default.conf:ro" \
  nginx:alpine >/dev/null
frontend_new_started=1
[[ "$(docker port "$FRONTEND_CONTAINER" 80/tcp)" == "127.0.0.1:${FRONTEND_PORT}" ]] ||
  fail "preview frontend host port is not loopback-only"

wait_for_url "backend readiness" "http://127.0.0.1:${BACKEND_PORT}/ready"
backend_revision=$(docker inspect --format "$REVISION_LABEL_TEMPLATE" "$BACKEND_CONTAINER")
[[ "$backend_revision" == "$GIT_REVISION" ]] || fail "preview backend revision label mismatch"
backend_new_ready=1
frontend_revision=$(docker inspect --format "$REVISION_LABEL_TEMPLATE" "$FRONTEND_CONTAINER")
[[ "$frontend_revision" == "$GIT_REVISION" ]] || fail "preview frontend revision label mismatch"
wait_for_url "frontend" "http://127.0.0.1:${FRONTEND_PORT}/"
wait_for_url "public preview" "http://127.0.0.1:8080/pr-${PR_NUMBER}/"
wait_for_url "public API readiness" "http://127.0.0.1:8080/pr-${PR_NUMBER}/api/v2/ready"

docker rm "$FRONTEND_BACKUP" "$BACKEND_BACKUP" >/dev/null 2>&1 || true
deployment_succeeded=1
echo "PR #${PR_NUMBER} deployed at revision ${GIT_REVISION}"
