from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
import urllib.error
import urllib.parse
from datetime import datetime, timezone
from io import BytesIO
from pathlib import Path


MODULE_PATH = Path(__file__).parents[1] / "verify_oss.py"
SPEC = importlib.util.spec_from_file_location("verify_oss", MODULE_PATH)
assert SPEC and SPEC.loader
verify_oss = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verify_oss)


VALID_CONFIG = {
    "OSS_REGION": "cn-shanghai",
    "OSS_BUCKET": "yourtj-media",
    "OSS_ACCESS_KEY_ID": "test-access-key",
    "OSS_ACCESS_KEY_SECRET": "test-access-secret",
    "OSS_ROLE_ARN": "acs:ram::1234567890:role/yourtj-upload",
    "OSS_CALLBACK_BASE_URL": "https://api.example.test",
}


class FakeResponse(BytesIO):
    def __init__(self, payload: bytes, status: int = 200):
        super().__init__(payload)
        self.status = status

    def __enter__(self):
        return self

    def __exit__(self, *_args):
        self.close()


class VerifyOssTests(unittest.TestCase):
    def write_config(self, config: dict[str, str]) -> Path:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        path = Path(directory.name) / "oss.env"
        path.write_text("".join(f"{key}={value}\n" for key, value in config.items()))
        return path

    def test_load_config_requires_every_known_key(self):
        config = dict(VALID_CONFIG)
        del config["OSS_ROLE_ARN"]
        with self.assertRaisesRegex(verify_oss.PreflightError, "missing required keys"):
            verify_oss.load_config(self.write_config(config))

    def test_load_config_rejects_non_https_callback(self):
        config = dict(VALID_CONFIG)
        config["OSS_CALLBACK_BASE_URL"] = "http://api.example.test"
        with self.assertRaisesRegex(verify_oss.PreflightError, "HTTPS"):
            verify_oss.load_config(self.write_config(config))

    def test_assume_role_signature_is_stable(self):
        url = verify_oss.build_assume_role_url(
            VALID_CONFIG,
            datetime(2026, 7, 12, 3, 30, tzinfo=timezone.utc),
            "00000000-0000-0000-0000-000000000000",
        )
        self.assertIn("Action=AssumeRole", url)
        self.assertIn("Signature=", url)
        self.assertIn("RoleSessionName=yourtj-deploy-smoke", url)
        self.assertNotIn(VALID_CONFIG["OSS_ACCESS_KEY_SECRET"], url)
        parameters = urllib.parse.parse_qs(urllib.parse.urlsplit(url).query)
        policy = json.loads(parameters["Policy"][0])
        self.assertEqual(
            policy,
            {
                "Version": "1",
                "Statement": [
                    {
                        "Effect": "Allow",
                        "Action": ["oss:PutObject"],
                        "Resource": [
                            "acs:oss:*:*:yourtj-media/uploads/deploy-smoke/"
                            "00000000-0000-0000-0000-000000000000"
                        ],
                    }
                ],
            },
        )
        self.assertEqual(parameters["Signature"][0], "U71frEARsHUYZ5X/WxuRw029+7c=")

    def test_bucket_check_accepts_private_bucket_response(self):
        def forbidden(_request, timeout):
            self.assertEqual(timeout, 8)
            raise urllib.error.HTTPError(
                "https://example.test", 403, "Forbidden", {}, BytesIO()
            )

        verify_oss.check_bucket_endpoint(VALID_CONFIG, forbidden)

    def test_callback_check_requires_expected_rejection(self):
        def missing(_request, timeout):
            self.assertEqual(timeout, 8)
            raise urllib.error.HTTPError(
                "https://example.test", 404, "Not Found", {}, BytesIO()
            )

        with self.assertRaisesRegex(verify_oss.PreflightError, "HTTP 404"):
            verify_oss.check_callback_endpoint(VALID_CONFIG, missing)

    def test_sts_check_requires_temporary_credentials(self):
        def incomplete(_request, timeout):
            self.assertEqual(timeout, 8)
            return FakeResponse(json.dumps({"Credentials": {}}).encode())

        with self.assertRaisesRegex(verify_oss.PreflightError, "missing"):
            verify_oss.check_sts(VALID_CONFIG, incomplete)


if __name__ == "__main__":
    unittest.main()
