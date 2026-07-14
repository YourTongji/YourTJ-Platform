#!/usr/bin/env bash

set -euo pipefail

readonly GENERATOR_VERSION="7.22.0"
readonly GENERATOR_SHA256="3f1e6ce5c6ad4f15242c6170ab43aad4bad771622617eeece4a7d4f72ffaf329"
readonly GENERATOR_URL="https://repo1.maven.org/maven2/org/openapitools/openapi-generator-cli/${GENERATOR_VERSION}/openapi-generator-cli-${GENERATOR_VERSION}.jar"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
api_package="${repo_root}/mobile/packages/yourtj_api"
model_overrides="${api_package}/model_overrides"
cache_root="${XDG_CACHE_HOME:-${HOME:-/tmp}/.cache}/yourtj"
generator_jar="${cache_root}/openapi-generator-cli-${GENERATOR_VERSION}.jar"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/yourtj-dart-api.XXXXXX")"
generation_log="${temporary_root}/openapi-generator.log"
staged_lib=""
backup_lib=""
downloaded_jar=""
staged_lock=""

cleanup() {
  rm -rf "${temporary_root}"
  if [[ -n "${staged_lib}" ]]; then
    rm -rf "${staged_lib}"
  fi
  if [[ -n "${downloaded_jar}" ]]; then
    rm -f "${downloaded_jar}"
  fi
  if [[ -n "${staged_lock}" ]]; then
    rm -f "${staged_lock}"
  fi
  if [[ -n "${backup_lib}" && -d "${backup_lib}" ]]; then
    if [[ ! -e "${api_package}/lib" ]]; then
      mv "${backup_lib}" "${api_package}/lib"
    else
      rm -rf "${backup_lib}"
    fi
  fi
}
trap cleanup EXIT

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

download_generator() {
  mkdir -p "${cache_root}"
  if [[ -f "${generator_jar}" ]] && [[ "$(sha256_file "${generator_jar}")" == "${GENERATOR_SHA256}" ]]; then
    return
  fi

  rm -f "${generator_jar}"
  downloaded_jar="$(mktemp "${cache_root}/.openapi-generator.XXXXXX")"
  curl --fail --location --retry 3 \
    --proto '=https' \
    --proto-redir '=https' \
    --tlsv1.2 \
    --output "${downloaded_jar}" \
    "${GENERATOR_URL}"
  if [[ "$(sha256_file "${downloaded_jar}")" != "${GENERATOR_SHA256}" ]]; then
    rm -f "${downloaded_jar}"
    echo "OpenAPI Generator checksum mismatch" >&2
    exit 1
  fi
  mv "${downloaded_jar}" "${generator_jar}"
  downloaded_jar=""
}

for command_name in java curl dart; do
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "Required command is unavailable: ${command_name}" >&2
    exit 1
  fi
done

for package_file in pubspec.yaml pubspec.lock analysis_options.yaml build.yaml; do
  if [[ ! -f "${api_package}/${package_file}" ]]; then
    echo "Missing generated-package configuration: mobile/packages/yourtj_api/${package_file}" >&2
    exit 1
  fi
done

if [[ ! -f "${model_overrides}/forum_draft_payload.dart" ]]; then
  echo "Missing audited model override: mobile/packages/yourtj_api/model_overrides/forum_draft_payload.dart" >&2
  exit 1
fi
if [[ ! -d "${api_package}/test" ]]; then
  echo "Missing generated-client contract tests: mobile/packages/yourtj_api/test" >&2
  exit 1
fi

if [[ -L "${api_package}" || -L "${api_package}/lib" ]]; then
  echo "Generated package and lib must not be symbolic links" >&2
  exit 1
fi
if [[ "$(cd "${api_package}" && pwd -P)" != "${api_package}" ]]; then
  echo "Generated package resolved outside the repository" >&2
  exit 1
fi

download_generator

if ! java -jar "${generator_jar}" validate -i "${repo_root}/contract/openapi.yaml" >"${generation_log}" 2>&1; then
  cat "${generation_log}" >&2
  exit 1
fi

generated_package="${temporary_root}/yourtj_api"
if ! java -jar "${generator_jar}" generate \
  -g dart-dio \
  -i "${repo_root}/contract/openapi.yaml" \
  -o "${generated_package}" \
  --additional-properties="pubName=yourtj_api,pubVersion=0.1.0,pubDescription=Generated YourTJ API client,pubHomepage=https://yourtj.de,pubPublishTo=none,serializationLibrary=json_serializable,skipCopyWith=true,enumUnknownDefaultCase=true,disallowAdditionalPropertiesIfNotPresent=false" \
  --global-property="apiTests=false,modelTests=false,apiDocs=false,modelDocs=false" \
  >>"${generation_log}" 2>&1; then
  cat "${generation_log}" >&2
  exit 1
fi

cp "${api_package}/pubspec.yaml" "${generated_package}/pubspec.yaml"
cp "${api_package}/analysis_options.yaml" "${generated_package}/analysis_options.yaml"
cp "${api_package}/build.yaml" "${generated_package}/build.yaml"
cp "${api_package}/pubspec.lock" "${generated_package}/pubspec.lock"
cp -R "${api_package}/test" "${generated_package}/test"

# dart-dio 7.22 flattens discriminated oneOf schemas into an impossible all-fields-required class.
# Keep the OpenAPI union authoritative and replace only that generated model with a checked decoder.
cp "${model_overrides}/forum_draft_payload.dart" \
  "${generated_package}/lib/src/model/forum_draft_payload.dart"
rm -f "${generated_package}/lib/src/model/forum_draft_payload.g.dart"

(
  cd "${generated_package}"
  dart pub get --enforce-lockfile
  dart run build_runner build --delete-conflicting-outputs
  dart fix --apply --code=unused_import
  dart format lib >/dev/null
  dart analyze
  dart test
)

staged_lib="$(mktemp -d "${api_package}/../.yourtj-api-lib.XXXXXX")"
cp -R "${generated_package}/lib/." "${staged_lib}/"
backup_lib="$(mktemp -d "${api_package}/../.yourtj-api-backup.XXXXXX")"
rmdir "${backup_lib}"
mv "${api_package}/lib" "${backup_lib}"
mv "${staged_lib}" "${api_package}/lib"
staged_lib=""
rm -rf "${backup_lib}"
backup_lib=""
staged_lock="$(mktemp "${api_package}/.pubspec-lock.XXXXXX")"
cp "${generated_package}/pubspec.lock" "${staged_lock}"
mv "${staged_lock}" "${api_package}/pubspec.lock"
staged_lock=""

echo "Generated mobile/packages/yourtj_api from contract/openapi.yaml with OpenAPI Generator ${GENERATOR_VERSION}."
