from __future__ import annotations

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

if [[ "$1" == "container" && "$2" == "inspect" ]]; then
  test -f "${state}/container-$3"
  exit
fi
if [[ "$1" == "image" && "$2" == "inspect" ]]; then
  exit 0
fi
if [[ "$1" == "load" ]]; then
  exit 0
fi
if [[ "$1" == "stop" ]]; then
  test -f "${state}/container-$2"
  exit
fi
if [[ "$1" == "rename" ]]; then
  mv "${state}/container-$2" "${state}/container-$3"
  exit
fi
if [[ "$1" == "start" ]]; then
  touch "${state}/container-$2"
  exit
fi
if [[ "$1" == "rm" ]]; then
  rm -f "${state}/container-$2"
  exit
fi
if [[ "$1" == "run" ]]; then
  shift
  name=""
  while (($#)); do
    if [[ "$1" == "--name" ]]; then
      name="$2"
      shift 2
      continue
    fi
    shift
  done
  test -n "$name"
  touch "${state}/container-${name}"
  echo fake-container-id
  exit
fi
if [[ "$1" == "inspect" && "$2" == "--format" ]]; then
  if [[ "$3" == *".Config.Env"* ]]; then
    for key in OSS_REGION OSS_BUCKET OSS_ACCESS_KEY_ID OSS_ACCESS_KEY_SECRET OSS_ROLE_ARN OSS_CALLBACK_BASE_URL; do
      echo "${key}=set"
    done
  else
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

        self.frontend_root = self.root / "releases"
        self.frontend = self.frontend_root / REVISION / "frontend"
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
        )
        self.email_env = self.root / "email.env"
        self.email_env.write_text("EMAIL_PROVIDER=log\n")
        os.chmod(self.runtime_env, 0o600)
        os.chmod(self.email_env, 0o600)

        image = tempfile.NamedTemporaryFile(prefix="api-image-main-", suffix=".tar", dir="/tmp", delete=False)
        image.close()
        self.image = Path(image.name)
        self.addCleanup(self.image.unlink, missing_ok=True)

        oss = tempfile.NamedTemporaryFile(prefix="yourtj-main-oss-", suffix=".env", dir="/tmp", delete=False)
        oss.close()
        self.oss_env = Path(oss.name)
        self.oss_env.write_text("OSS_REGION=cn-shanghai\n")
        os.chmod(self.oss_env, 0o600)
        self.addCleanup(self.oss_env.unlink, missing_ok=True)

        verifier = tempfile.NamedTemporaryFile(prefix="verify-oss-", suffix=".py", dir="/tmp", delete=False)
        verifier.close()
        self.verifier = Path(verifier.name)
        self.verifier.write_text("#!/usr/bin/env python3\nprint('preflight ok')\n")
        self.addCleanup(self.verifier.unlink, missing_ok=True)

    @staticmethod
    def write_executable(path: Path, content: str) -> None:
        path.write_text(content)
        os.chmod(path, 0o700)

    def run_deploy(self, *, fail_after: int | None = None) -> subprocess.CompletedProcess[str]:
        environment = os.environ.copy()
        environment.update(
            {
                "PATH": f"{self.fake_bin}:{environment['PATH']}",
                "EXPECTED_FRONTEND_ROOT": str(self.frontend_root),
                "MAIN_RUNTIME_ENV_FILE": str(self.runtime_env),
                "MAIN_EMAIL_ENV_FILE": str(self.email_env),
                "DEPLOY_HEALTH_ATTEMPTS": "2",
                "DEPLOY_HEALTH_DELAY_SECONDS": "0",
                "FAKE_DOCKER_STATE": str(self.state),
                "FAKE_REVISION": REVISION,
            }
        )
        if fail_after is not None:
            environment["FAKE_CURL_FAIL_AFTER"] = str(fail_after)
        return subprocess.run(
            [
                str(DEPLOY_SCRIPT),
                str(self.frontend),
                str(self.image),
                str(self.oss_env),
                REVISION,
                str(self.verifier),
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
        result = self.run_deploy()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.state / "container-main-be").exists())
        self.assertTrue((self.state / "container-main-fe").exists())
        self.assertFalse(list(self.state.glob("container-*-rollback-*")))
        self.assertIn("MAIN DEPLOYED", result.stdout)

    def test_failed_new_backend_restores_previous_containers(self):
        self.seed_current_containers()
        result = self.run_deploy(fail_after=1)
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue((self.state / "container-main-be").exists())
        self.assertTrue((self.state / "container-main-fe").exists())
        self.assertFalse(list(self.state.glob("container-*-rollback-*")))
        self.assertIn("restoring previous containers", result.stderr)


if __name__ == "__main__":
    unittest.main()
