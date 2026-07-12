#!/usr/bin/env bash
set -Eeuo pipefail

readonly EXPECTED_FRONTEND_ROOT="${EXPECTED_FRONTEND_ROOT:-/opt/yourtj-preview/releases/main}"
readonly MAIN_RUNTIME_ENV_FILE="${MAIN_RUNTIME_ENV_FILE:-/opt/yourtj-preview/shared/main-runtime.env}"
readonly MAIN_EMAIL_ENV_FILE="${MAIN_EMAIL_ENV_FILE:-/opt/yourtj-preview/shared/email-main.env}"
readonly BACKEND_CONTAINER="main-be"
readonly FRONTEND_CONTAINER="main-fe"
readonly BACKEND_IMAGE="yourtj-api:main"
readonly BACKEND_PORT="16000"
readonly FRONTEND_PORT="15000"
readonly HEALTH_ATTEMPTS="${DEPLOY_HEALTH_ATTEMPTS:-60}"
readonly HEALTH_DELAY_SECONDS="${DEPLOY_HEALTH_DELAY_SECONDS:-1}"

DEPLOYMENT_STARTED=0
DEPLOYMENT_SUCCEEDED=0
BACKEND_BACKED_UP=0
BACKEND_NEW_STARTED=0
FRONTEND_BACKED_UP=0
FRONTEND_NEW_STARTED=0

usage() {
  echo "usage: deploy-main.sh <frontend_dist_dir> <backend_image_tar> <oss_env_file> <git_revision> <oss_verifier>" >&2
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
    OSS_REGION \
    OSS_BUCKET \
    OSS_ACCESS_KEY_ID \
    OSS_ACCESS_KEY_SECRET \
    OSS_ROLE_ARN \
    OSS_CALLBACK_BASE_URL; do
    grep -Eq "^${key}=.+$" <<<"$environment" || fail "${key} was not injected into main-be"
  done
  echo "  OSS runtime environment: present"
}

rollback_deployment() {
  set +e
  echo "deploy-main: deployment failed; restoring previous containers" >&2

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

  if ((BACKEND_NEW_STARTED == 1)) && container_exists "$BACKEND_CONTAINER"; then
    docker stop "$BACKEND_CONTAINER" >/dev/null 2>&1
    docker rm "$BACKEND_CONTAINER" >/dev/null 2>&1
  fi
  if ((BACKEND_BACKED_UP == 1)) && container_exists "$BACKEND_BACKUP"; then
    docker rename "$BACKEND_BACKUP" "$BACKEND_CONTAINER" >/dev/null 2>&1
    docker start "$BACKEND_CONTAINER" >/dev/null 2>&1
  elif ((BACKEND_NEW_STARTED == 0)) && container_exists "$BACKEND_CONTAINER"; then
    docker start "$BACKEND_CONTAINER" >/dev/null 2>&1
  fi
}

handle_exit() {
  local status="$?"
  trap - EXIT
  if ((DEPLOYMENT_STARTED == 1 && DEPLOYMENT_SUCCEEDED == 0)); then
    rollback_deployment
  fi
  exit "$status"
}

[[ "$#" -eq 5 ]] || usage
[[ "$HEALTH_ATTEMPTS" =~ ^[1-9][0-9]*$ ]] || fail "health attempts must be a positive integer"
[[ "$HEALTH_DELAY_SECONDS" =~ ^[0-9]+([.][0-9]+)?$ ]] || fail "health delay must be numeric"

readonly FRONTEND_DIR="$1"
readonly BACKEND_IMAGE_TAR="$2"
readonly OSS_ENV_FILE="$3"
readonly GIT_REVISION="$4"
readonly OSS_VERIFIER="$5"
BACKUP_SUFFIX="$(date +%s)-$$"
readonly BACKUP_SUFFIX
readonly BACKEND_BACKUP="${BACKEND_CONTAINER}-rollback-${BACKUP_SUFFIX}"
readonly FRONTEND_BACKUP="${FRONTEND_CONTAINER}-rollback-${BACKUP_SUFFIX}"

[[ "$GIT_REVISION" =~ ^[0-9a-f]{40}$ ]] || fail "git revision must be a full commit SHA"
[[ "$FRONTEND_DIR" == "${EXPECTED_FRONTEND_ROOT}/${GIT_REVISION}/frontend" ]] ||
  fail "frontend path does not match the main release revision"
[[ -f "${FRONTEND_DIR}/index.html" ]] || fail "frontend index.html is missing"
[[ "$BACKEND_IMAGE_TAR" == /tmp/api-image-main-*.tar && -f "$BACKEND_IMAGE_TAR" ]] ||
  fail "backend image archive is missing or outside the allowed path"
[[ "$OSS_ENV_FILE" == /tmp/yourtj-main-oss-*.env ]] || fail "OSS env file is outside the allowed path"
[[ "$OSS_VERIFIER" == /tmp/verify-oss-*.py && -f "$OSS_VERIFIER" && ! -L "$OSS_VERIFIER" ]] ||
  fail "OSS verifier is missing or outside the allowed path"

require_regular_secret_file "$MAIN_RUNTIME_ENV_FILE" "main runtime env file"
require_regular_secret_file "$MAIN_EMAIL_ENV_FILE" "main email env file"
require_regular_secret_file "$OSS_ENV_FILE" "OSS env file"

for key in DATABASE_URL REDIS_URL MEILI_URL JWT_SECRET CREDIT_SYSTEM_PRIVATE_KEY; do
  require_env_key "$MAIN_RUNTIME_ENV_FILE" "$key"
done

echo "=== MAIN PREFLIGHT ==="
python3 "$OSS_VERIFIER" --env-file "$OSS_ENV_FILE"
docker image inspect nginx:alpine >/dev/null 2>&1 || fail "nginx:alpine is not available"

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

echo "  Starting main backend..."
docker run -d \
  --env-file "$MAIN_RUNTIME_ENV_FILE" \
  --env-file "$MAIN_EMAIL_ENV_FILE" \
  --env-file "$OSS_ENV_FILE" \
  --name "$BACKEND_CONTAINER" \
  --restart unless-stopped \
  --network host \
  --label "org.opencontainers.image.revision=${GIT_REVISION}" \
  --label "yourtj.environment=main-staging" \
  -e "PORT=${BACKEND_PORT}" \
  -e "RUST_LOG=info" \
  "$BACKEND_IMAGE" >/dev/null
BACKEND_NEW_STARTED=1

wait_for_url "backend direct health" "http://127.0.0.1:${BACKEND_PORT}/health"
verify_container_environment

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
  -p "${FRONTEND_PORT}:80" \
  -v "${FRONTEND_DIR}:/usr/share/nginx/html:ro" \
  -v /opt/yourtj-preview/pr-nginx.conf:/etc/nginx/conf.d/default.conf:ro \
  nginx:alpine >/dev/null
FRONTEND_NEW_STARTED=1

wait_for_url "frontend direct health" "http://127.0.0.1:${FRONTEND_PORT}/"
wait_for_url "frontend public route" "http://127.0.0.1:8080/"
wait_for_url "backend public route" "http://127.0.0.1:8080/api/v2/health"

[[ "$(docker inspect --format '{{index .Config.Labels \"org.opencontainers.image.revision\"}}' "$BACKEND_CONTAINER")" == "$GIT_REVISION" ]]
[[ "$(docker inspect --format '{{index .Config.Labels \"org.opencontainers.image.revision\"}}' "$FRONTEND_CONTAINER")" == "$GIT_REVISION" ]]

docker rm "$BACKEND_BACKUP" >/dev/null 2>&1 || true
docker rm "$FRONTEND_BACKUP" >/dev/null 2>&1 || true
DEPLOYMENT_SUCCEEDED=1

echo "=== MAIN DEPLOYED ==="
echo "  revision: ${GIT_REVISION}"
