#!/usr/bin/env bash
set -Eeuo pipefail

readonly EXPECTED_FRONTEND_ROOT="${EXPECTED_FRONTEND_ROOT:-/opt/yourtj-preview/releases/main}"
readonly MAIN_RUNTIME_ENV_FILE="${MAIN_RUNTIME_ENV_FILE:-/opt/yourtj-preview/shared/main-runtime.env}"
readonly MAIN_EMAIL_ENV_FILE="${MAIN_EMAIL_ENV_FILE:-/opt/yourtj-preview/shared/email-main.env}"
readonly WALLET_KEY_CUTOVER_MARKER="${WALLET_KEY_CUTOVER_MARKER:-/opt/yourtj-preview/shared/migration-0067-wallet-key-cutover.complete}"
readonly WALLET_KEY_CUTOVER_DRAIN_SECONDS=360
readonly BACKEND_CONTAINER="main-be"
readonly FRONTEND_CONTAINER="main-fe"
readonly BACKEND_IMAGE="yourtj-api:main"
readonly BACKEND_PORT="16000"
readonly FRONTEND_PORT="15000"
readonly HEALTH_ATTEMPTS="${DEPLOY_HEALTH_ATTEMPTS:-60}"
readonly HEALTH_DELAY_SECONDS="${DEPLOY_HEALTH_DELAY_SECONDS:-1}"
readonly REVISION_LABEL_TEMPLATE='{{index .Config.Labels "org.opencontainers.image.revision"}}'

DEPLOYMENT_STARTED=0
DEPLOYMENT_SUCCEEDED=0
BACKEND_BACKED_UP=0
BACKEND_NEW_STARTED=0
BACKEND_NEW_READY=0
FRONTEND_BACKED_UP=0
FRONTEND_NEW_STARTED=0
WALLET_KEY_CUTOVER_REQUIRED=0
WALLET_KEY_CUTOVER_MARKER_TEMP=""

usage() {
  echo "usage: deploy-main.sh <frontend_dist_dir> <backend_image_tar> <media_env_file> <git_revision> <media_verifier> <frontend_nginx_template> <wallet_cutover_approved_revision>" >&2
  exit 64
}

fail() {
  echo "deploy-main: $*" >&2
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

  echo "  ${description}: failed after ${HEALTH_ATTEMPTS} attempts" >&2
  return 1
}

require_regular_secret_file() {
  local path="$1"
  local label="$2"
  local permissions

  [[ -f "$path" && ! -L "$path" ]] || fail "${label} must be a regular, non-symlink file"
  permissions=$(stat -c '%a' "$path" 2>/dev/null || stat -f '%Lp' "$path")
  [[ "$permissions" == "600" || "$permissions" == "400" ]] ||
    fail "${label} permissions must be 600 or 400"
}

require_env_key() {
  local env_file="$1"
  local key="$2"
  grep -Eq "^${key}=.+$" "$env_file" || fail "${key} is missing from the runtime environment"
}

verify_container_environment() {
  local environment
  local key
  environment=$(docker inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$BACKEND_CONTAINER")

  for key in \
    BIND_ADDRESS \
    OSS_REGION \
    OSS_BUCKET \
    OSS_ACCESS_KEY_ID \
    OSS_ACCESS_KEY_SECRET \
    OSS_ROLE_ARN \
    OSS_CALLBACK_BASE_URL \
    MEDIA_DELIVERY_OSS_BUCKET \
    MEDIA_DELIVERY_OSS_ACCESS_KEY_ID \
    MEDIA_DELIVERY_OSS_ACCESS_KEY_SECRET \
    MEDIA_CDN_BASE_URL \
    MEDIA_CDN_PRIMARY_KEY \
    MEDIA_CDN_SECONDARY_KEY \
    MEDIA_CDN_SIGNING_KEY_SLOT \
    MEDIA_CDN_URL_TTL_SECONDS \
    CDN_ACCESS_KEY_ID \
    CDN_ACCESS_KEY_SECRET \
    EMAIL_ENCRYPTION_ACTIVE_VERSION \
    EMAIL_ENCRYPTION_ACTIVE_AEAD \
    EMAIL_ENCRYPTION_ACTIVE_BLIND \
    EMAIL_ENCRYPTION_STRICT; do
    grep -Eq "^${key}=.+$" <<<"$environment" || fail "${key} was not injected into main-be"
  done
  grep -Fxq "BIND_ADDRESS=127.0.0.1" <<<"$environment" ||
    fail "main backend must bind only to loopback on the shared host"
  grep -Fxq "EMAIL_ENCRYPTION_STRICT=true" <<<"$environment" ||
    fail "main backend must enforce encrypted email storage"
  echo "  Media runtime environment: present"
}

verify_container_revision() {
  local container="$1"
  local revision
  revision=$(docker inspect --format "$REVISION_LABEL_TEMPLATE" "$container")
  [[ "$revision" == "$GIT_REVISION" ]] ||
    fail "${container} revision label does not match the deployment revision"
}

preview_port() {
  local prefix="$1"
  local pr_number="$2"
  printf '%s%03d' "$prefix" "$pr_number"
}

quarantine_unsafe_preview_containers() {
  local backend_environment
  local container
  local expected_port
  local kind
  local mapping
  local pr_number
  local unsafe_prs=""

  while IFS= read -r container; do
    if [[ "$container" =~ ^pr-([1-9][0-9]{0,2})-(fe|be)$ ]]; then
      pr_number="${BASH_REMATCH[1]}"
      kind="${BASH_REMATCH[2]}"
      if [[ "$kind" == "fe" ]]; then
        expected_port="$(preview_port 15 "$pr_number")"
        mapping="$(docker port "$container" 80/tcp 2>/dev/null || true)"
        if [[ "$mapping" != "127.0.0.1:${expected_port}" ]]; then
          [[ " $unsafe_prs " == *" $pr_number "* ]] || unsafe_prs+=" $pr_number"
        fi
      else
        expected_port="$(preview_port 16 "$pr_number")"
        backend_environment="$(docker inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$container")"
        if ! grep -Fxq "BIND_ADDRESS=127.0.0.1" <<<"$backend_environment" ||
          ! grep -Fxq "PORT=${expected_port}" <<<"$backend_environment"; then
          [[ " $unsafe_prs " == *" $pr_number "* ]] || unsafe_prs+=" $pr_number"
        fi
      fi
    fi
  done < <(docker ps --format '{{.Names}}')

  if [[ -z "$unsafe_prs" ]]; then
    return
  fi
  for pr_number in $unsafe_prs; do
    echo "deploy-main: stopping unsafe legacy preview PR #${pr_number}; redeploy it with the loopback-only script" >&2
    for kind in fe be; do
      container="pr-${pr_number}-${kind}"
      if container_exists "$container"; then
        docker stop "$container" >/dev/null ||
          fail "could not stop unsafe preview container ${container}"
      fi
    done
  done
}

rollback_deployment() {
  set +e
  echo "deploy-main: deployment failed; restoring compatible frontend state" >&2

  if ((FRONTEND_NEW_STARTED == 1)) && container_exists "$FRONTEND_CONTAINER"; then
    docker stop "$FRONTEND_CONTAINER" >/dev/null 2>&1
    docker rm "$FRONTEND_CONTAINER" >/dev/null 2>&1
  fi
  if ((FRONTEND_BACKED_UP == 1)) && container_exists "$FRONTEND_BACKUP"; then
    docker rename "$FRONTEND_BACKUP" "$FRONTEND_CONTAINER" >/dev/null 2>&1
    docker start "$FRONTEND_CONTAINER" >/dev/null 2>&1
  elif ((FRONTEND_NEW_STARTED == 0)) && container_exists "$FRONTEND_CONTAINER"; then
    docker start "$FRONTEND_CONTAINER" >/dev/null 2>&1
  fi

  if ((BACKEND_NEW_STARTED == 1)); then
    if ((BACKEND_NEW_READY == 0)) && container_exists "$BACKEND_CONTAINER"; then
      docker stop "$BACKEND_CONTAINER" >/dev/null 2>&1
    fi
    echo "deploy-main: forward-only schema cutover; previous backend remains stopped" >&2
    echo "deploy-main: fix forward with a new deployment; do not restart ${BACKEND_BACKUP}" >&2
  elif ((BACKEND_BACKED_UP == 1)) && container_exists "$BACKEND_BACKUP"; then
    docker rename "$BACKEND_BACKUP" "$BACKEND_CONTAINER" >/dev/null 2>&1
    docker start "$BACKEND_CONTAINER" >/dev/null 2>&1
  elif container_exists "$BACKEND_CONTAINER"; then
    docker start "$BACKEND_CONTAINER" >/dev/null 2>&1
  fi
}

handle_exit() {
  local status="$?"
  trap - EXIT
  if [[ -n "$WALLET_KEY_CUTOVER_MARKER_TEMP" ]]; then
    rm -f "$WALLET_KEY_CUTOVER_MARKER_TEMP"
  fi
  if ((DEPLOYMENT_STARTED == 1 && DEPLOYMENT_SUCCEEDED == 0)); then
    rollback_deployment
  fi
  exit "$status"
}

[[ "$#" -eq 7 ]] || usage
[[ "$HEALTH_ATTEMPTS" =~ ^[1-9][0-9]*$ ]] || fail "health attempts must be a positive integer"
[[ "$HEALTH_DELAY_SECONDS" =~ ^[0-9]+([.][0-9]+)?$ ]] || fail "health delay must be numeric"

readonly FRONTEND_DIR="$1"
readonly BACKEND_IMAGE_TAR="$2"
readonly OSS_ENV_FILE="$3"
readonly GIT_REVISION="$4"
readonly OSS_VERIFIER="$5"
readonly NGINX_TEMPLATE="$6"
readonly WALLET_KEY_CUTOVER_APPROVED_REVISION="$7"
readonly RENDERED_NGINX="${FRONTEND_DIR}/nginx.conf"
readonly FRONTEND_RELEASE_DIR="${FRONTEND_DIR%/frontend}"
readonly FRONTEND_RELEASE_NAME="${FRONTEND_RELEASE_DIR##*/}"
BACKUP_SUFFIX="$(date +%s)-$$"
readonly BACKUP_SUFFIX
readonly BACKEND_BACKUP="${BACKEND_CONTAINER}-rollback-${BACKUP_SUFFIX}"
readonly FRONTEND_BACKUP="${FRONTEND_CONTAINER}-rollback-${BACKUP_SUFFIX}"

[[ "$GIT_REVISION" =~ ^[0-9a-f]{40}$ ]] || fail "git revision must be a full commit SHA"
[[ "${FRONTEND_RELEASE_DIR%/*}" == "$EXPECTED_FRONTEND_ROOT" \
  && "$FRONTEND_RELEASE_NAME" =~ ^${GIT_REVISION}-[1-9][0-9]*-[1-9][0-9]*$ ]] ||
  fail "frontend path does not match an immutable main release"
[[ -f "${FRONTEND_DIR}/index.html" ]] || fail "frontend index.html is missing"
[[ "$BACKEND_IMAGE_TAR" == /tmp/api-image-main-*.tar && -f "$BACKEND_IMAGE_TAR" ]] ||
  fail "backend image archive is missing or outside the allowed path"
[[ "$OSS_ENV_FILE" == /tmp/yourtj-main-oss-*.env ]] || fail "OSS env file is outside the allowed path"
[[ "$OSS_VERIFIER" == /tmp/verify-oss-*.py && -f "$OSS_VERIFIER" && ! -L "$OSS_VERIFIER" ]] ||
  fail "OSS verifier is missing or outside the allowed path"
[[ "$NGINX_TEMPLATE" == /tmp/frontend-nginx-main-*.conf.template \
  && -f "$NGINX_TEMPLATE" && ! -L "$NGINX_TEMPLATE" ]] ||
  fail "frontend Nginx template is missing or outside the allowed path"

require_regular_secret_file "$MAIN_RUNTIME_ENV_FILE" "main runtime env file"
require_regular_secret_file "$MAIN_EMAIL_ENV_FILE" "main email env file"
require_regular_secret_file "$OSS_ENV_FILE" "OSS env file"
if [[ -e "$WALLET_KEY_CUTOVER_MARKER" || -L "$WALLET_KEY_CUTOVER_MARKER" ]]; then
  require_regular_secret_file "$WALLET_KEY_CUTOVER_MARKER" "wallet key cutover marker"
  grep -Fxq "migration=0067" "$WALLET_KEY_CUTOVER_MARKER" ||
    fail "wallet key cutover marker is malformed"
else
  WALLET_KEY_CUTOVER_REQUIRED=1
  [[ "$WALLET_KEY_CUTOVER_APPROVED_REVISION" == "$GIT_REVISION" ]] ||
    fail "wallet key cutover requires approval for the exact deployment revision"
fi

for key in \
  DATABASE_URL \
  REDIS_URL \
  MEILI_URL \
  JWT_SECRET \
  CREDIT_SYSTEM_PRIVATE_KEY \
  EMAIL_ENCRYPTION_ACTIVE_VERSION \
  EMAIL_ENCRYPTION_ACTIVE_AEAD \
  EMAIL_ENCRYPTION_ACTIVE_BLIND \
  EMAIL_ENCRYPTION_STRICT; do
  require_env_key "$MAIN_RUNTIME_ENV_FILE" "$key"
done
email_encryption_version="$(sed -n 's/^EMAIL_ENCRYPTION_ACTIVE_VERSION=//p' "$MAIN_RUNTIME_ENV_FILE")"
email_encryption_aead="$(sed -n 's/^EMAIL_ENCRYPTION_ACTIVE_AEAD=//p' "$MAIN_RUNTIME_ENV_FILE")"
email_encryption_blind="$(sed -n 's/^EMAIL_ENCRYPTION_ACTIVE_BLIND=//p' "$MAIN_RUNTIME_ENV_FILE")"
[[ "$email_encryption_version" =~ ^[1-9][0-9]*$ ]] ||
  fail "EMAIL_ENCRYPTION_ACTIVE_VERSION must be a positive integer"
[[ "$email_encryption_aead" =~ ^[0-9a-fA-F]{64}$ ]] ||
  fail "EMAIL_ENCRYPTION_ACTIVE_AEAD must be a 32-byte hex key"
[[ "$email_encryption_blind" =~ ^[0-9a-fA-F]{64}$ ]] ||
  fail "EMAIL_ENCRYPTION_ACTIVE_BLIND must be a 32-byte hex key"
[[ "$email_encryption_aead" != "$email_encryption_blind" ]] ||
  fail "email AEAD and blind-index keys must differ"
grep -Fxq "EMAIL_ENCRYPTION_STRICT=true" "$MAIN_RUNTIME_ENV_FILE" ||
  fail "main staging requires EMAIL_ENCRYPTION_STRICT=true"

quarantine_unsafe_preview_containers

echo "=== MAIN PREFLIGHT ==="
python3 "$OSS_VERIFIER" --env-file "$OSS_ENV_FILE"
docker image inspect nginx:alpine >/dev/null 2>&1 || fail "nginx:alpine is not available"
MEDIA_CDN_BASE_URL="$(sed -n 's/^MEDIA_CDN_BASE_URL=//p' "$OSS_ENV_FILE")"
readonly MEDIA_CDN_BASE_URL
[[ "$MEDIA_CDN_BASE_URL" =~ ^https://[A-Za-z0-9.-]+$ ]] ||
  fail "MEDIA_CDN_BASE_URL is not one exact HTTPS origin"
OSS_REGION="$(sed -n 's/^OSS_REGION=//p' "$OSS_ENV_FILE")"
readonly OSS_REGION
[[ "$OSS_REGION" =~ ^[a-z0-9]+(-[a-z0-9]+)+$ ]] || fail "OSS_REGION has an invalid format"
OSS_BUCKET="$(sed -n 's/^OSS_BUCKET=//p' "$OSS_ENV_FILE")"
readonly OSS_BUCKET
[[ "$OSS_BUCKET" =~ ^[a-z0-9][a-z0-9-]{1,61}[a-z0-9]$ ]] ||
  fail "OSS_BUCKET has an invalid format"
readonly MEDIA_INGEST_ORIGIN="https://${OSS_BUCKET}.oss-${OSS_REGION}.aliyuncs.com"
[[ "$(grep -c '__MEDIA_CDN_ORIGIN__' "$NGINX_TEMPLATE")" -eq 1 ]] ||
  fail "frontend Nginx template must contain one CDN-origin placeholder"
[[ "$(grep -c '__MEDIA_INGEST_ORIGIN__' "$NGINX_TEMPLATE")" -eq 1 ]] ||
  fail "frontend Nginx template must contain one Ingest-origin placeholder"
sed \
  -e "s|__MEDIA_CDN_ORIGIN__|${MEDIA_CDN_BASE_URL}|g" \
  -e "s|__MEDIA_INGEST_ORIGIN__|${MEDIA_INGEST_ORIGIN}|g" \
  "$NGINX_TEMPLATE" > "$RENDERED_NGINX"
if grep -Eq '__MEDIA_(CDN|INGEST)_ORIGIN__' "$RENDERED_NGINX"; then
  fail "frontend Nginx rendering failed"
fi
docker run --rm \
  -v "${FRONTEND_DIR}:/usr/share/nginx/html:ro" \
  -v "${RENDERED_NGINX}:/etc/nginx/conf.d/default.conf:ro" \
  nginx:alpine nginx -t >/dev/null

echo "  Loading backend image..."
docker load <"$BACKEND_IMAGE_TAR" >/dev/null
docker image inspect "$BACKEND_IMAGE" >/dev/null 2>&1 || fail "loaded archive did not provide ${BACKEND_IMAGE}"

trap handle_exit EXIT
DEPLOYMENT_STARTED=1

if container_exists "$BACKEND_CONTAINER"; then
  docker stop "$BACKEND_CONTAINER" >/dev/null
  BACKEND_BACKED_UP=1
  docker rename "$BACKEND_CONTAINER" "$BACKEND_BACKUP"
fi

backend_command=(api --enforce-controlled-wallet-migration)
if ((WALLET_KEY_CUTOVER_REQUIRED == 1)); then
  echo "  Wallet-key cutover: writers stopped; draining signing intents for ${WALLET_KEY_CUTOVER_DRAIN_SECONDS}s..."
  sleep "$WALLET_KEY_CUTOVER_DRAIN_SECONDS"
  backend_command+=(--wallet-key-cutover-drained)
fi

echo "  Starting main backend..."
docker create \
  --env-file "$MAIN_RUNTIME_ENV_FILE" \
  --env-file "$MAIN_EMAIL_ENV_FILE" \
  --env-file "$OSS_ENV_FILE" \
  --name "$BACKEND_CONTAINER" \
  --restart unless-stopped \
  --network host \
  --label "org.opencontainers.image.revision=${GIT_REVISION}" \
  --label "yourtj.environment=main-staging" \
  -e "BIND_ADDRESS=127.0.0.1" \
  -e "PORT=${BACKEND_PORT}" \
  -e "RUST_LOG=info" \
  "$BACKEND_IMAGE" "${backend_command[@]}" >/dev/null
BACKEND_NEW_STARTED=1
docker start "$BACKEND_CONTAINER" >/dev/null

wait_for_url "backend direct health" "http://127.0.0.1:${BACKEND_PORT}/health"
wait_for_url "backend direct readiness" "http://127.0.0.1:${BACKEND_PORT}/ready"
verify_container_environment
BACKEND_NEW_READY=1
verify_container_revision "$BACKEND_CONTAINER"

if ((WALLET_KEY_CUTOVER_REQUIRED == 1)); then
  WALLET_KEY_CUTOVER_MARKER_TEMP="${WALLET_KEY_CUTOVER_MARKER}.tmp-${BACKUP_SUFFIX}"
  umask 077
  {
    echo "migration=0067"
    echo "revision=${GIT_REVISION}"
    date -u '+completed_at=%Y-%m-%dT%H:%M:%SZ'
  } > "$WALLET_KEY_CUTOVER_MARKER_TEMP"
  chmod 600 "$WALLET_KEY_CUTOVER_MARKER_TEMP"
  mv "$WALLET_KEY_CUTOVER_MARKER_TEMP" "$WALLET_KEY_CUTOVER_MARKER"
  echo "  Wallet-key cutover: migration and pre-serve ledger verification complete"
fi

if container_exists "$FRONTEND_CONTAINER"; then
  docker stop "$FRONTEND_CONTAINER" >/dev/null
  FRONTEND_BACKED_UP=1
  docker rename "$FRONTEND_CONTAINER" "$FRONTEND_BACKUP"
fi

echo "  Starting main frontend..."
docker run -d \
  --name "$FRONTEND_CONTAINER" \
  --restart unless-stopped \
  --label "org.opencontainers.image.revision=${GIT_REVISION}" \
  --label "yourtj.environment=main-staging" \
  -p "127.0.0.1:${FRONTEND_PORT}:80" \
  -v "${FRONTEND_DIR}:/usr/share/nginx/html:ro" \
  -v "${RENDERED_NGINX}:/etc/nginx/conf.d/default.conf:ro" \
  nginx:alpine >/dev/null
FRONTEND_NEW_STARTED=1

[[ "$(docker port "$FRONTEND_CONTAINER" 80/tcp)" == "127.0.0.1:${FRONTEND_PORT}" ]] ||
  fail "main frontend host port is not loopback-only"

wait_for_url "frontend direct health" "http://127.0.0.1:${FRONTEND_PORT}/"
wait_for_url "frontend public route" "http://127.0.0.1:8080/"
wait_for_url "backend public route" "http://127.0.0.1:8080/api/v2/health"
wait_for_url "backend public readiness" "http://127.0.0.1:8080/api/v2/ready"

verify_container_revision "$BACKEND_CONTAINER"
verify_container_revision "$FRONTEND_CONTAINER"
quarantine_unsafe_preview_containers

docker rm "$BACKEND_BACKUP" >/dev/null 2>&1 || true
docker rm "$FRONTEND_BACKUP" >/dev/null 2>&1 || true
DEPLOYMENT_SUCCEEDED=1

echo "=== MAIN DEPLOYED ==="
echo "  revision: ${GIT_REVISION}"
