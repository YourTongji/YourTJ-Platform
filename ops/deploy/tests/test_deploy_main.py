from __future__ import annotations

import fcntl
import os
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[3]
DEPLOY_SCRIPT = ROOT / "ops/deploy/deploy-main.sh"
REVISION = "a" * 40


FAKE_DOCKER = r"""#!/usr/bin/env bash
set -eu
state="${FAKE_DOCKER_STATE:?}"
revision_template='{{index .Config.Labels "org.opencontainers.image.revision"}}'
printf '%s\n' "$*" >> "${state}/docker-calls"

if [[ "$1" == "container" && "$2" == "inspect" ]]; then
  test -f "${state}/container-$3"
  exit
fi
if [[ "$1" == "image" && "$2" == "inspect" ]]; then
  exit 0
fi
if [[ "$1" == "ps" ]]; then
  for path in "${state}"/container-*; do
    [[ -e "$path" ]] || continue
    name="${path##*/container-}"
    [[ -f "${state}/stopped-${name}" ]] || echo "$name"
  done
  exit
fi
if [[ "$1" == "load" ]]; then
  exit 0
fi
if [[ "$1" == "stop" ]]; then
  test -f "${state}/container-$2"
  touch "${state}/stopped-$2"
  exit
fi
if [[ "$1" == "rename" ]]; then
  mv "${state}/container-$2" "${state}/container-$3"
  if [[ -f "${state}/stopped-$2" ]]; then
    mv "${state}/stopped-$2" "${state}/stopped-$3"
  fi
  exit
fi
if [[ "$1" == "start" ]]; then
  touch "${state}/container-$2"
  if [[ "${FAKE_DOCKER_FAIL_BACKEND_START:-}" == "1" && "$2" == "main-be" ]]; then
    touch "${state}/stopped-$2"
    exit 1
  fi
  rm -f "${state}/stopped-$2"
  exit
fi
if [[ "$1" == "rm" ]]; then
  rm -f "${state}/container-$2"
  rm -f "${state}/stopped-$2"
  exit
fi
if [[ "$1" == "port" ]]; then
  test "$3" = "80/tcp"
  if [[ "$2" == "main-fe" ]]; then
    echo "127.0.0.1:15000"
  elif [[ "$2" =~ ^pr-([1-9][0-9]{0,2})-fe$ ]]; then
    pr_number="${BASH_REMATCH[1]}"
    if [[ "${FAKE_UNSAFE_PREVIEW:-}" == "$pr_number" ]]; then
      printf '0.0.0.0:15%03d\n' "$pr_number"
    else
      printf '127.0.0.1:15%03d\n' "$pr_number"
    fi
  else
    exit 99
  fi
  exit
fi
if [[ "$1" == "run" || "$1" == "create" ]]; then
  command="$1"
  shift
  name=""
  disposable=0
  while (($#)); do
    if [[ "$1" == "--rm" ]]; then
      disposable=1
      shift
      continue
    fi
    if [[ "$1" == "--name" ]]; then
      name="$2"
      shift 2
      continue
    fi
    shift
  done
  if [[ "$command" == "run" ]] && ((disposable == 1)) && [[ -z "$name" ]]; then
    exit 0
  fi
  test -n "$name"
  touch "${state}/container-${name}"
  echo fake-container-id
  exit
fi
if [[ "$1" == "inspect" && "$2" == "--format" ]]; then
  if [[ "$3" == *".Config.Env"* ]]; then
    for key in \
      BIND_ADDRESS \
      OSS_REGION OSS_BUCKET OSS_ACCESS_KEY_ID OSS_ACCESS_KEY_SECRET OSS_ROLE_ARN \
      OSS_CALLBACK_BASE_URL MEDIA_DELIVERY_OSS_BUCKET MEDIA_DELIVERY_OSS_ACCESS_KEY_ID \
      MEDIA_DELIVERY_OSS_ACCESS_KEY_SECRET MEDIA_CDN_BASE_URL MEDIA_CDN_PRIMARY_KEY \
      MEDIA_CDN_SECONDARY_KEY MEDIA_CDN_SIGNING_KEY_SLOT MEDIA_CDN_URL_TTL_SECONDS \
      CDN_ACCESS_KEY_ID CDN_ACCESS_KEY_SECRET EMAIL_ENCRYPTION_ACTIVE_VERSION \
      EMAIL_ENCRYPTION_ACTIVE_AEAD EMAIL_ENCRYPTION_ACTIVE_BLIND \
      EMAIL_ENCRYPTION_STRICT; do
      if [[ "$key" == "BIND_ADDRESS" ]]; then
        if [[ "$4" =~ ^pr-([1-9][0-9]{0,2})-be$ \
          && "${FAKE_UNSAFE_PREVIEW:-}" == "${BASH_REMATCH[1]}" ]]; then
          echo "BIND_ADDRESS=0.0.0.0"
        else
          echo "BIND_ADDRESS=127.0.0.1"
        fi
      elif [[ "$key" == "EMAIL_ENCRYPTION_STRICT" ]]; then
        echo "EMAIL_ENCRYPTION_STRICT=true"
      else
        echo "${key}=set"
      fi
    done
    if [[ "$4" =~ ^pr-([1-9][0-9]{0,2})-be$ ]]; then
      printf 'PORT=16%03d\n' "${BASH_REMATCH[1]}"
    fi
  else
    test "$3" = "$revision_template"
    [[ "$4" == "main-be" || "$4" == "main-fe" ]]
    echo "${FAKE_REVISION:?}"
  fi
  exit
fi

echo "unexpected docker invocation: $*" >&2
exit 99
"""


FAKE_CURL = r"""#!/usr/bin/env bash
set -eu
state="${FAKE_DOCKER_STATE:?}"
count_file="${state}/curl-count"
count=0
if [[ -f "$count_file" ]]; then
  count=$(cat "$count_file")
fi
count=$((count + 1))
echo "$count" > "$count_file"
if [[ -n "${FAKE_CURL_FAIL_AFTER:-}" ]] && ((count >= FAKE_CURL_FAIL_AFTER)); then
  exit 22
fi
exit 0
"""


FAKE_SLEEP = r"""#!/usr/bin/env bash
set -eu
printf '%s\n' "$1" >> "${FAKE_DOCKER_STATE:?}/sleep-calls"
if [[ -n "${FAKE_SLEEP_SIGNAL_PARENT:-}" ]]; then
  kill -s "$FAKE_SLEEP_SIGNAL_PARENT" "$PPID"
fi
"""


FAKE_FLOCK = r"""#!/usr/bin/env python3
import fcntl
import sys


nonblocking = "-n" in sys.argv[1:-1]
operation = fcntl.LOCK_EX | (fcntl.LOCK_NB if nonblocking else 0)
try:
    fcntl.flock(int(sys.argv[-1]), operation)
except BlockingIOError:
    raise SystemExit(1)
"""


class DeployMainTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.state = self.root / "state"
        self.fake_bin = self.root / "bin"
        self.state.mkdir()
        self.fake_bin.mkdir()
        self.write_executable(self.fake_bin / "docker", FAKE_DOCKER)
        self.write_executable(self.fake_bin / "curl", FAKE_CURL)
        self.write_executable(self.fake_bin / "sleep", FAKE_SLEEP)
        self.write_executable(self.fake_bin / "flock", FAKE_FLOCK)

        self.frontend_root = self.root / "releases"
        self.frontend = self.frontend_root / f"{REVISION}-123-1" / "frontend"
        self.frontend.mkdir(parents=True)
        (self.frontend / "index.html").write_text("ok")

        self.runtime_env = self.root / "main-runtime.env"
        self.runtime_env.write_text(
            "DATABASE_URL=set\n"
            "DATABASE_REPLICA_URL=set\n"
            "REDIS_URL=set\n"
            "MEILI_URL=set\n"
            "JWT_SECRET=set\n"
            "CREDIT_SYSTEM_PRIVATE_KEY=set\n"
            "EMAIL_ENCRYPTION_ACTIVE_VERSION=1\n"
            f"EMAIL_ENCRYPTION_ACTIVE_AEAD={'1' * 64}\n"
            f"EMAIL_ENCRYPTION_ACTIVE_BLIND={'2' * 64}\n"
            "EMAIL_ENCRYPTION_STRICT=true\n"
        )
        self.email_env = self.root / "email.env"
        self.email_env.write_text("EMAIL_PROVIDER=log\n")
        os.chmod(self.runtime_env, 0o600)
        os.chmod(self.email_env, 0o600)
        self.wallet_cutover_marker = self.root / "wallet-cutover.complete"
        self.wallet_cutover_marker.write_text("migration=0067\nrevision=previous\n")
        os.chmod(self.wallet_cutover_marker, 0o600)
        self.deploy_lock = self.root / "deploy-main.lock"

        image = tempfile.NamedTemporaryFile(prefix="api-image-main-", suffix=".tar", dir="/tmp", delete=False)
        image.close()
        self.image = Path(image.name)
        self.addCleanup(self.image.unlink, missing_ok=True)

        oss = tempfile.NamedTemporaryFile(prefix="yourtj-main-oss-", suffix=".env", dir="/tmp", delete=False)
        oss.close()
        self.oss_env = Path(oss.name)
        self.oss_env.write_text(
            "OSS_REGION=cn-shanghai\n"
            "OSS_BUCKET=yourtj-media\n"
            "MEDIA_CDN_BASE_URL=https://media.example.test\n"
        )
        os.chmod(self.oss_env, 0o600)
        self.addCleanup(self.oss_env.unlink, missing_ok=True)

        verifier = tempfile.NamedTemporaryFile(prefix="verify-oss-", suffix=".py", dir="/tmp", delete=False)
        verifier.close()
        self.verifier = Path(verifier.name)
        self.verifier.write_text("#!/usr/bin/env python3\nprint('preflight ok')\n")
        self.addCleanup(self.verifier.unlink, missing_ok=True)

        nginx = tempfile.NamedTemporaryFile(
            prefix="frontend-nginx-main-", suffix=".conf.template", dir="/tmp", delete=False
        )
        nginx.close()
        self.nginx = Path(nginx.name)
        self.nginx.write_text(
            "server { listen 80; # __MEDIA_CDN_ORIGIN__ __MEDIA_INGEST_ORIGIN__\n"
            "root /usr/share/nginx/html; }\n"
        )
        self.addCleanup(self.nginx.unlink, missing_ok=True)

    @staticmethod
    def write_executable(path: Path, content: str) -> None:
        path.write_text(content)
        os.chmod(path, 0o700)

    def run_deploy(
        self,
        *,
        fail_after: int | None = None,
        fake_revision: str = REVISION,
        unsafe_preview: int | None = None,
        fail_backend_start: bool = False,
        signal_during_sleep: str | None = None,
        cutover_approval: str = "not-approved",
    ) -> subprocess.CompletedProcess[str]:
        environment = os.environ.copy()
        environment.update(
            {
                "PATH": f"{self.fake_bin}:{environment['PATH']}",
                "EXPECTED_FRONTEND_ROOT": str(self.frontend_root),
                "MAIN_RUNTIME_ENV_FILE": str(self.runtime_env),
                "MAIN_EMAIL_ENV_FILE": str(self.email_env),
                "WALLET_KEY_CUTOVER_MARKER": str(self.wallet_cutover_marker),
                "DEPLOY_LOCK_FILE": str(self.deploy_lock),
                "DEPLOY_HEALTH_ATTEMPTS": "2",
                "DEPLOY_HEALTH_DELAY_SECONDS": "0",
                "FAKE_DOCKER_STATE": str(self.state),
                "FAKE_REVISION": fake_revision,
            }
        )
        if fail_after is not None:
            environment["FAKE_CURL_FAIL_AFTER"] = str(fail_after)
        if unsafe_preview is not None:
            environment["FAKE_UNSAFE_PREVIEW"] = str(unsafe_preview)
        if fail_backend_start:
            environment["FAKE_DOCKER_FAIL_BACKEND_START"] = "1"
        if signal_during_sleep is not None:
            environment["FAKE_SLEEP_SIGNAL_PARENT"] = signal_during_sleep
        return subprocess.run(
            [
                str(DEPLOY_SCRIPT),
                str(self.frontend),
                str(self.image),
                str(self.oss_env),
                REVISION,
                str(self.verifier),
                str(self.nginx),
                cutover_approval,
            ],
            env=environment,
            capture_output=True,
            text=True,
            check=False,
        )

    def seed_current_containers(self) -> None:
        (self.state / "container-main-be").touch()
        (self.state / "container-main-fe").touch()

    def test_success_replaces_both_containers(self):
        self.seed_current_containers()
        result = self.run_deploy(cutover_approval=REVISION)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.state / "container-main-be").exists())
        self.assertTrue((self.state / "container-main-fe").exists())
        self.assertFalse(list(self.state.glob("container-*-rollback-*")))
        self.assertIn("MAIN DEPLOYED", result.stdout)
        rendered_nginx = (self.frontend / "nginx.conf").read_text()
        self.assertIn("https://media.example.test", rendered_nginx)
        self.assertIn("https://yourtj-media.oss-cn-shanghai.aliyuncs.com", rendered_nginx)
        self.assertNotIn("__MEDIA_", rendered_nginx)
        docker_calls = (self.state / "docker-calls").read_text()
        self.assertIn("--enforce-controlled-wallet-migration", docker_calls)
        self.assertNotIn("--wallet-key-cutover-drained", docker_calls)

    def test_first_wallet_key_cutover_drains_and_records_verified_revision(self):
        self.seed_current_containers()
        self.wallet_cutover_marker.unlink()

        result = self.run_deploy(cutover_approval=REVISION)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("draining signing intents for 360s", result.stdout)
        self.assertIn(
            "migration and pre-serve ledger verification complete",
            result.stdout,
        )
        sleep_calls = [
            int(seconds)
            for seconds in (self.state / "sleep-calls").read_text().splitlines()
        ]
        self.assertEqual(sleep_calls, [30] * 12)
        self.assertEqual(sum(sleep_calls), 360)
        progress_lines = [
            line for line in result.stdout.splitlines() if "drain progress" in line
        ]
        self.assertEqual(len(progress_lines), 12)
        self.assertIn("drain progress 30/360s", progress_lines[0])
        self.assertIn("drain progress 360/360s", progress_lines[-1])
        marker = self.wallet_cutover_marker.read_text()
        self.assertIn("migration=0067", marker)
        self.assertIn(f"revision={REVISION}", marker)
        self.assertIn(
            "--wallet-key-cutover-drained",
            (self.state / "docker-calls").read_text(),
        )

    def test_completed_wallet_key_cutover_accepts_no_approval_sentinel(self):
        result = self.run_deploy()

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("draining signing intents", result.stdout)

    def test_rejects_concurrent_deploy_before_stopping_backend(self):
        self.seed_current_containers()
        with self.deploy_lock.open("w") as lock_file:
            fcntl.flock(lock_file, fcntl.LOCK_EX | fcntl.LOCK_NB)
            result = self.run_deploy(cutover_approval=REVISION)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("another main deployment is still active", result.stderr)
        self.assertFalse((self.state / "stopped-main-be").exists())

    def test_termination_during_wallet_drain_restores_previous_backend(self):
        self.seed_current_containers()
        self.wallet_cutover_marker.unlink()

        result = self.run_deploy(
            signal_during_sleep="TERM",
            cutover_approval=REVISION,
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertTrue((self.state / "container-main-be").exists())
        self.assertFalse((self.state / "stopped-main-be").exists())
        self.assertFalse(list(self.state.glob("container-main-be-rollback-*")))
        self.assertFalse(self.wallet_cutover_marker.exists())
        self.assertIn("restoring compatible frontend state", result.stderr)

    def test_missing_wallet_key_marker_rejects_no_approval_before_stopping_backend(self):
        self.seed_current_containers()
        self.wallet_cutover_marker.unlink()

        result = self.run_deploy()

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("requires approval for the exact deployment revision", result.stderr)
        self.assertTrue((self.state / "container-main-be").exists())
        self.assertFalse((self.state / "stopped-main-be").exists())

    def test_rejects_malformed_wallet_key_cutover_approval(self):
        result = self.run_deploy(cutover_approval="not-approved; uname")

        self.assertNotEqual(result.returncode, 0)
        self.assertIn(
            "approval must be not-approved or a full lowercase commit SHA",
            result.stderr,
        )
        self.assertFalse((self.state / "container-main-be").exists())

    def test_wallet_key_cutover_requires_exact_revision_approval_before_stopping_backend(self):
        self.seed_current_containers()
        self.wallet_cutover_marker.unlink()

        result = self.run_deploy(cutover_approval="b" * 40)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("requires approval for the exact deployment revision", result.stderr)
        self.assertTrue((self.state / "container-main-be").exists())
        self.assertFalse((self.state / "stopped-main-be").exists())
        self.assertFalse(self.wallet_cutover_marker.exists())

    def test_rejects_runtime_without_strict_email_encryption(self):
        self.runtime_env.write_text(
            self.runtime_env.read_text().replace(
                "EMAIL_ENCRYPTION_STRICT=true", "EMAIL_ENCRYPTION_STRICT=false"
            )
        )
        result = self.run_deploy()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("requires EMAIL_ENCRYPTION_STRICT=true", result.stderr)
        self.assertFalse((self.state / "container-main-be").exists())

    def test_quarantines_legacy_preview_bound_to_wildcard_interfaces(self):
        (self.state / "container-pr-41-fe").touch()
        (self.state / "container-pr-41-be").touch()
        result = self.run_deploy(unsafe_preview=41)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.state / "stopped-pr-41-fe").exists())
        self.assertTrue((self.state / "stopped-pr-41-be").exists())
        self.assertIn("stopping unsafe legacy preview PR #41", result.stderr)

    def test_failed_new_backend_keeps_previous_backend_stopped(self):
        self.seed_current_containers()
        result = self.run_deploy(fail_after=1)
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue((self.state / "container-main-be").exists())
        self.assertTrue((self.state / "container-main-fe").exists())
        self.assertTrue((self.state / "stopped-main-be").exists())
        self.assertEqual(len(list(self.state.glob("container-main-be-rollback-*"))), 1)
        self.assertFalse(list(self.state.glob("container-main-fe-rollback-*")))
        self.assertIn("forward-only schema cutover", result.stderr)
        self.assertIn("previous backend remains stopped", result.stderr)

    def test_backend_start_failure_never_restores_the_previous_revision(self):
        self.seed_current_containers()
        result = self.run_deploy(fail_backend_start=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue((self.state / "container-main-be").exists())
        self.assertTrue((self.state / "stopped-main-be").exists())
        self.assertEqual(len(list(self.state.glob("container-main-be-rollback-*"))), 1)
        self.assertIn("forward-only schema cutover", result.stderr)

    def test_post_readiness_failure_keeps_new_backend_and_restores_frontend(self):
        self.seed_current_containers()
        result = self.run_deploy(fake_revision="b" * 40)
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue((self.state / "container-main-be").exists())
        self.assertTrue((self.state / "container-main-fe").exists())
        self.assertFalse((self.state / "stopped-main-be").exists())
        self.assertEqual(len(list(self.state.glob("container-main-be-rollback-*"))), 1)
        self.assertFalse(list(self.state.glob("container-main-fe-rollback-*")))
        self.assertIn("revision label does not match the deployment revision", result.stderr)
        self.assertIn("forward-only schema cutover", result.stderr)


if __name__ == "__main__":
    unittest.main()
