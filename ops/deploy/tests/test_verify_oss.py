from __future__ import annotations

import hashlib
import importlib.util
import json
import tempfile
import unittest
import urllib.error
import urllib.parse
import uuid
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
    "MEDIA_DELIVERY_OSS_BUCKET": "yourtj-media-delivery",
    "MEDIA_DELIVERY_OSS_ACCESS_KEY_ID": "delivery-access-key",
    "MEDIA_DELIVERY_OSS_ACCESS_KEY_SECRET": "delivery-access-secret",
    "MEDIA_CDN_BASE_URL": "https://media.example.test",
    "MEDIA_CDN_PRIMARY_KEY": "primary-signing-key",
    "MEDIA_CDN_SECONDARY_KEY": "secondary-signing-key",
    "MEDIA_CDN_SIGNING_KEY_SLOT": "primary",
    "MEDIA_CDN_URL_TTL_SECONDS": "300",
    "CDN_ACCESS_KEY_ID": "purge-access-key",
    "CDN_ACCESS_KEY_SECRET": "purge-access-secret",
}


class FakeResponse(BytesIO):
    def __init__(
        self,
        payload: bytes,
        status: int = 200,
        headers: dict[str, str] | None = None,
    ):
        super().__init__(payload)
        self.status = status
        self.headers = headers or {}

    def __enter__(self):
        return self

    def __exit__(self, *_args):
        self.close()


class FakeClock:
    def __init__(self):
        self.value = 0.0

    def monotonic(self) -> float:
        return self.value

    def sleep(self, seconds: float) -> None:
        self.value += seconds


class SequenceUuids:
    def __init__(self, *values: str):
        self.values = [uuid.UUID(value) for value in values]

    def __call__(self) -> uuid.UUID:
        if not self.values:
            raise AssertionError("test exhausted UUID sequence")
        return self.values.pop(0)


def forbidden(url: str, status: int, payload: bytes = b"") -> urllib.error.HTTPError:
    return urllib.error.HTTPError(url, status, "Rejected", {}, BytesIO(payload))


def delivery_headers() -> dict[str, str]:
    return {
        "Content-Length": str(len(verify_oss.SMOKE_WEBP)),
        "Content-Type": "image/webp",
        "x-oss-meta-content-sha256": hashlib.sha256(verify_oss.SMOKE_WEBP).hexdigest(),
    }


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

    def test_load_config_requires_separate_storage_and_provider_principals(self):
        shared_bucket = dict(VALID_CONFIG)
        shared_bucket["MEDIA_DELIVERY_OSS_BUCKET"] = shared_bucket["OSS_BUCKET"]
        with self.assertRaisesRegex(verify_oss.PreflightError, "separate private buckets"):
            verify_oss.load_config(self.write_config(shared_bucket))

        shared_principal = dict(VALID_CONFIG)
        shared_principal["CDN_ACCESS_KEY_ID"] = shared_principal["OSS_ACCESS_KEY_ID"]
        with self.assertRaisesRegex(verify_oss.PreflightError, "principals must differ"):
            verify_oss.load_config(self.write_config(shared_principal))

    def test_load_config_rejects_cdn_path_and_non_rotatable_keys(self):
        path = dict(VALID_CONFIG)
        path["MEDIA_CDN_BASE_URL"] = "https://media.example.test/assets"
        with self.assertRaisesRegex(verify_oss.PreflightError, "exact HTTPS origin"):
            verify_oss.load_config(self.write_config(path))

        repeated_key = dict(VALID_CONFIG)
        repeated_key["MEDIA_CDN_SECONDARY_KEY"] = repeated_key["MEDIA_CDN_PRIMARY_KEY"]
        with self.assertRaisesRegex(verify_oss.PreflightError, "distinct"):
            verify_oss.load_config(self.write_config(repeated_key))

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

    def test_oss_v4_signer_matches_runtime_delete_vector(self):
        signed = verify_oss.sign_oss_v4(
            "test-ak",
            "test-secret",
            "cn-shanghai",
            "20260711T080910Z",
            "DELETE",
            "/yourtj/uploads/42/image/"
            "00000000-0000-0000-0000-000000000000.png",
            "",
            {
                "x-oss-content-sha256": "UNSIGNED-PAYLOAD",
                "x-oss-date": "20260711T080910Z",
            },
        )
        self.assertEqual(
            hashlib.sha256(signed.canonical_request.encode()).hexdigest(),
            "86369ab7d64524b931621c06f2218d8f44a15ce27cec51f63c026b102f248f03",
        )
        self.assertEqual(
            signed.authorization,
            "OSS4-HMAC-SHA256 Credential=test-ak/20260711/cn-shanghai/oss/"
            "aliyun_v4_request,Signature="
            "bea7f516674b3ec8cf23f99e9a4a148eda7377c2e4ff995b4e57823dc0956b40",
        )

    def test_oss_v4_canonical_request_matches_official_put_vector(self):
        signed = verify_oss.sign_oss_v4(
            "LTAIEXAMPLE",
            "yourAccessKeySecret",
            "cn-hangzhou",
            "20250411T064124Z",
            "PUT",
            "/examplebucket/exampleobject",
            "",
            {
                "content-disposition": "attachment",
                "content-length": "3",
                "content-md5": "ICy5YqxZB1uWSwcVLSNLcA==",
                "content-type": "text/plain",
                "x-oss-content-sha256": "UNSIGNED-PAYLOAD",
                "x-oss-date": "20250411T064124Z",
            },
            ("content-disposition", "content-length"),
        )
        self.assertEqual(
            hashlib.sha256(signed.canonical_request.encode()).hexdigest(),
            "c46d96390bdbc2d739ac9363293ae9d710b14e48081fcb22cd8ad54b63136eca",
        )
        self.assertEqual(
            signed.string_to_sign,
            "OSS4-HMAC-SHA256\n20250411T064124Z\n"
            "20250411/cn-hangzhou/oss/aliyun_v4_request\n"
            "c46d96390bdbc2d739ac9363293ae9d710b14e48081fcb22cd8ad54b63136eca",
        )
        self.assertIn(
            "AdditionalHeaders=content-disposition;content-length",
            signed.authorization,
        )

    def test_delivery_oss_v4_and_cdn_signatures_are_stable(self):
        timestamp = datetime(2026, 7, 12, 3, 30, tzinfo=timezone.utc)
        object_key = (
            "assets/deploy-smoke/00000000000000000000000000000000.webp"
        )
        oss_request = verify_oss.build_oss_v4_request(
            VALID_CONFIG,
            "PUT",
            object_key,
            timestamp,
            verify_oss.SMOKE_WEBP,
        )
        headers = {key.lower(): value for key, value in oss_request.header_items()}
        self.assertEqual(headers["x-oss-date"], "20260712T033000Z")
        self.assertEqual(headers["x-oss-content-sha256"], "UNSIGNED-PAYLOAD")
        self.assertEqual(headers["x-oss-forbid-overwrite"], "true")
        self.assertNotIn("date", headers)
        self.assertEqual(
            headers["authorization"],
            "OSS4-HMAC-SHA256 Credential=delivery-access-key/20260712/cn-shanghai/"
            "oss/aliyun_v4_request,Signature="
            "e10890da97a585f4604c91abc743667b1d0ea574bb5f07d73179a65a0078d647",
        )
        self.assertNotIn("content-md5", headers)
        self.assertNotIn(VALID_CONFIG["MEDIA_DELIVERY_OSS_ACCESS_KEY_SECRET"], str(headers))

        signed_url = verify_oss.build_type_a_url(
            VALID_CONFIG,
            object_key,
            timestamp,
            "00112233445566778899aabbccddeeff",
        )
        self.assertEqual(
            signed_url,
            "https://media.example.test/assets/deploy-smoke/"
            "00000000000000000000000000000000.webp?auth_key="
            "1783827000-00112233445566778899aabbccddeeff-0-"
            "c1426cf1bebbfd1ae191d4da9306c28f",
        )

        rpc_request = verify_oss.build_cdn_rpc_request(
            VALID_CONFIG,
            "RefreshObjectCaches",
            {
                "ObjectPath": f"https://media.example.test/{object_key}",
                "ObjectType": "File",
            },
            timestamp,
            "00000000-0000-0000-0000-000000000000",
        )
        parameters = urllib.parse.parse_qs(rpc_request.data.decode())
        self.assertEqual(parameters["Signature"], ["cWB8YRV0ZOQT57VVt4C+kMcrEDc="])
        self.assertNotIn(VALID_CONFIG["CDN_ACCESS_KEY_SECRET"], rpc_request.data.decode())

    def test_bucket_check_accepts_private_bucket_response(self):
        def forbidden(_request, timeout):
            self.assertEqual(timeout, 8)
            raise urllib.error.HTTPError(
                "https://example.test", 403, "Forbidden", {}, BytesIO()
            )

        verify_oss.check_bucket_endpoint(VALID_CONFIG, forbidden)

        def public(_request, timeout):
            self.assertEqual(timeout, 8)
            return FakeResponse(b"", status=200)

        with self.assertRaisesRegex(verify_oss.PreflightError, "unsigned request"):
            verify_oss.check_bucket_endpoint(VALID_CONFIG, public)

    def test_ingest_preflight_requires_headerless_overwrite_rejection_and_cleans(self):
        actions: list[str] = []
        grant = verify_oss.StsUploadGrant(
            "sts-ak",
            "sts-secret",
            "sts-token",
            "uploads/deploy-smoke/00000000-0000-0000-0000-000000000042",
        )

        def opener(request, timeout):
            self.assertEqual(timeout, 8)
            actions.append(request.method)
            headers = {key.lower(): value for key, value in request.header_items()}
            if request.method == "PUT" and len(actions) == 1:
                self.assertEqual(headers["x-oss-forbid-overwrite"], "true")
                self.assertEqual(headers["x-oss-security-token"], "sts-token")
                return FakeResponse(b"")
            if request.method == "PUT":
                self.assertNotIn("x-oss-forbid-overwrite", headers)
                raise forbidden(
                    request.full_url,
                    409,
                    b"<Error><Code>FileAlreadyExists</Code></Error>",
                )
            if request.method == "DELETE":
                self.assertNotIn("x-oss-security-token", headers)
                return FakeResponse(b"", status=204)
            raise AssertionError("unexpected Ingest preflight request")

        verify_oss.check_ingest_prevent_overwrite(VALID_CONFIG, opener, grant)
        self.assertEqual(actions, ["PUT", "PUT", "DELETE"])

        def overwrite_allowed(request, timeout):
            self.assertEqual(timeout, 8)
            if request.method == "DELETE":
                return FakeResponse(b"", status=204)
            return FakeResponse(b"")

        with self.assertRaisesRegex(verify_oss.PreflightError, "allowed HTTP 200"):
            verify_oss.check_ingest_prevent_overwrite(VALID_CONFIG, overwrite_allowed, grant)

    def test_callback_check_requires_expected_rejection(self):
        def missing(_request, timeout):
            self.assertEqual(timeout, 8)
            raise urllib.error.HTTPError(
                "https://example.test", 404, "Not Found", {}, BytesIO()
            )

        with self.assertRaisesRegex(verify_oss.PreflightError, "HTTP 404"):
            verify_oss.check_callback_endpoint(VALID_CONFIG, missing)

    def test_delivery_smoke_writes_verifies_purges_and_deletes(self):
        actions: list[str] = []
        purge_statuses = ["Refreshing", "Complete"]

        def opener(request, timeout):
            self.assertEqual(timeout, 8)
            parsed = urllib.parse.urlsplit(request.full_url)
            if parsed.hostname == "yourtj-media-delivery.oss-cn-shanghai.aliyuncs.com":
                actions.append(request.method)
                headers = {
                    key.lower(): value for key, value in request.header_items()
                }
                self.assertEqual(headers["x-oss-date"], "20260712T033000Z")
                self.assertEqual(
                    headers["x-oss-content-sha256"], "UNSIGNED-PAYLOAD"
                )
                self.assertTrue(
                    headers["authorization"].startswith(
                        "OSS4-HMAC-SHA256 Credential=delivery-access-key/"
                        "20260712/cn-shanghai/oss/aliyun_v4_request,Signature="
                    )
                )
                if request.method == "PUT":
                    self.assertEqual(request.data, verify_oss.SMOKE_WEBP)
                    self.assertEqual(headers["x-oss-forbid-overwrite"], "true")
                    self.assertNotIn("content-md5", headers)
                    return FakeResponse(b"")
                if request.method == "HEAD":
                    return FakeResponse(b"", headers=delivery_headers())
                if request.method == "GET":
                    return FakeResponse(
                        verify_oss.SMOKE_WEBP,
                        headers={"Content-Type": "image/webp"},
                    )
                if request.method == "DELETE":
                    return FakeResponse(b"", status=204)
            if parsed.hostname == "media.example.test":
                if parsed.query:
                    actions.append("CDN_SIGNED")
                    return FakeResponse(verify_oss.SMOKE_WEBP)
                actions.append("CDN_UNSIGNED")
                raise forbidden(request.full_url, 403)
            if parsed.hostname == "cdn.aliyuncs.com":
                parameters = urllib.parse.parse_qs(request.data.decode())
                action = parameters["Action"][0]
                actions.append(action)
                if action == "RefreshObjectCaches":
                    return FakeResponse(b'{"RefreshTaskId":"123"}')
                status = purge_statuses.pop(0)
                return FakeResponse(
                    json.dumps(
                        {"Tasks": [{"Status": status, "TaskId": "123"}]}
                    ).encode()
                )
            raise AssertionError("unexpected synthetic request")

        clock = FakeClock()
        uuid_factory = SequenceUuids(
            "11111111-1111-1111-1111-111111111111",
            "22222222-2222-2222-2222-222222222222",
            "33333333-3333-3333-3333-333333333333",
            "44444444-4444-4444-4444-444444444444",
            "55555555-5555-5555-5555-555555555555",
        )
        verify_oss.run_delivery_smoke(
            VALID_CONFIG,
            opener,
            now=lambda: datetime(2026, 7, 12, 3, 30, tzinfo=timezone.utc),
            uuid_factory=uuid_factory,
            monotonic=clock.monotonic,
            sleep=clock.sleep,
        )
        self.assertEqual(
            actions,
            [
                "PUT",
                "HEAD",
                "GET",
                "CDN_UNSIGNED",
                "CDN_SIGNED",
                "RefreshObjectCaches",
                "DescribeRefreshTaskById",
                "DescribeRefreshTaskById",
                "DELETE",
            ],
        )
        self.assertEqual(clock.value, 5)

    def test_cdn_purge_polling_has_a_bounded_deadline(self):
        calls = 0

        def opener(request, timeout):
            nonlocal calls
            self.assertEqual(timeout, 8)
            parameters = urllib.parse.parse_qs(request.data.decode())
            self.assertEqual(parameters["Action"], ["DescribeRefreshTaskById"])
            calls += 1
            return FakeResponse(
                b'{"Tasks":[{"Status":"Refreshing","TaskId":"123"}]}'
            )

        clock = FakeClock()
        uuid_factory = SequenceUuids(
            "11111111-1111-1111-1111-111111111111",
            "22222222-2222-2222-2222-222222222222",
        )
        with self.assertRaisesRegex(verify_oss.PreflightError, "before the deadline"):
            verify_oss.poll_cdn_purge(
                VALID_CONFIG,
                opener,
                "123",
                now=lambda: datetime(2026, 7, 12, 3, 30, tzinfo=timezone.utc),
                uuid_factory=uuid_factory,
                monotonic=clock.monotonic,
                sleep=clock.sleep,
                deadline_seconds=6,
            )
        self.assertEqual(calls, 2)
        self.assertEqual(clock.value, 6)

    def test_cdn_purge_visibility_lag_is_retried(self):
        responses = [
            b'{"Tasks":[]}',
            b'{"Tasks":[{"Status":"Complete","TaskId":"123"}]}',
        ]

        def opener(request, timeout):
            self.assertEqual(timeout, 8)
            parameters = urllib.parse.parse_qs(request.data.decode())
            self.assertEqual(parameters["Action"], ["DescribeRefreshTaskById"])
            return FakeResponse(responses.pop(0))

        clock = FakeClock()
        uuid_factory = SequenceUuids(
            "11111111-1111-1111-1111-111111111111",
            "22222222-2222-2222-2222-222222222222",
        )
        verify_oss.poll_cdn_purge(
            VALID_CONFIG,
            opener,
            "123",
            now=lambda: datetime(2026, 7, 12, 3, 30, tzinfo=timezone.utc),
            uuid_factory=uuid_factory,
            monotonic=clock.monotonic,
            sleep=clock.sleep,
        )
        self.assertEqual(responses, [])
        self.assertEqual(clock.value, 5)

    def test_unsigned_cdn_404_is_rejected_and_fixture_is_cleaned_up(self):
        actions: list[str] = []

        def opener(request, timeout):
            self.assertEqual(timeout, 8)
            parsed = urllib.parse.urlsplit(request.full_url)
            if parsed.hostname == "yourtj-media-delivery.oss-cn-shanghai.aliyuncs.com":
                actions.append(request.method)
                if request.method == "PUT":
                    return FakeResponse(b"")
                if request.method == "HEAD":
                    return FakeResponse(b"", headers=delivery_headers())
                if request.method == "GET":
                    return FakeResponse(
                        verify_oss.SMOKE_WEBP,
                        headers={"Content-Type": "image/webp"},
                    )
                if request.method == "DELETE":
                    return FakeResponse(b"", status=204)
            if parsed.hostname == "media.example.test":
                actions.append("CDN_UNSIGNED")
                raise forbidden(request.full_url, 404)
            if parsed.hostname == "cdn.aliyuncs.com":
                actions.append("RefreshObjectCaches")
                return FakeResponse(b'{"RefreshTaskId":"987"}')
            raise AssertionError("unexpected synthetic request")

        uuid_factory = SequenceUuids(
            "11111111-1111-1111-1111-111111111111",
            "22222222-2222-2222-2222-222222222222",
        )
        with self.assertRaisesRegex(verify_oss.PreflightError, "HTTP 404; expected 403"):
            verify_oss.run_delivery_smoke(
                VALID_CONFIG,
                opener,
                now=lambda: datetime(2026, 7, 12, 3, 30, tzinfo=timezone.utc),
                uuid_factory=uuid_factory,
            )
        self.assertEqual(
            actions,
            ["PUT", "HEAD", "GET", "CDN_UNSIGNED", "RefreshObjectCaches", "DELETE"],
        )

    def test_delivery_permission_failure_is_safe_and_attempts_cleanup(self):
        actions: list[str] = []

        def opener(request, timeout):
            self.assertEqual(timeout, 8)
            parsed = urllib.parse.urlsplit(request.full_url)
            if parsed.hostname == "yourtj-media-delivery.oss-cn-shanghai.aliyuncs.com":
                actions.append(request.method)
                if request.method == "PUT":
                    raise forbidden(
                        request.full_url,
                        403,
                        b"<Error><Code>AccessDenied</Code>"
                        b"<Details>do-not-print-this</Details></Error>",
                    )
                raise forbidden(
                    request.full_url,
                    403,
                    b"<Error><Code>AccessDenied</Code></Error>",
                )
            if parsed.hostname == "cdn.aliyuncs.com":
                actions.append("RefreshObjectCaches")
                return FakeResponse(b'{"RefreshTaskId":"654"}')
            raise AssertionError("unexpected synthetic request")

        uuid_factory = SequenceUuids(
            "11111111-1111-1111-1111-111111111111",
            "22222222-2222-2222-2222-222222222222",
        )
        with self.assertRaises(verify_oss.PreflightError) as raised:
            verify_oss.run_delivery_smoke(
                VALID_CONFIG,
                opener,
                now=lambda: datetime(2026, 7, 12, 3, 30, tzinfo=timezone.utc),
                uuid_factory=uuid_factory,
            )
        message = str(raised.exception)
        self.assertIn("HTTP 403, code=AccessDenied", message)
        self.assertIn("cleanup was incomplete", message)
        self.assertNotIn("do-not-print-this", message)
        self.assertNotIn("assets/deploy-smoke", message)
        self.assertNotIn("654", message)
        self.assertEqual(actions, ["PUT", "RefreshObjectCaches", "DELETE"])

    def test_sts_check_requires_temporary_credentials(self):
        def incomplete(_request, timeout):
            self.assertEqual(timeout, 8)
            return FakeResponse(json.dumps({"Credentials": {}}).encode())

        with self.assertRaisesRegex(verify_oss.PreflightError, "missing"):
            verify_oss.check_sts(VALID_CONFIG, incomplete)


if __name__ == "__main__":
    unittest.main()
