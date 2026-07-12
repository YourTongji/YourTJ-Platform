#!/usr/bin/env python3
"""Fail-closed Alibaba OSS/STS deployment preflight without printing credentials."""

from __future__ import annotations

import argparse
import base64
import hashlib
import hmac
import json
import re
import sys
import urllib.error
import urllib.parse
import urllib.request
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Callable


REQUIRED_KEYS = frozenset(
    {
        "OSS_REGION",
        "OSS_BUCKET",
        "OSS_ACCESS_KEY_ID",
        "OSS_ACCESS_KEY_SECRET",
        "OSS_ROLE_ARN",
        "OSS_CALLBACK_BASE_URL",
    }
)
REGION = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)+$")
BUCKET = re.compile(r"^[a-z0-9][a-z0-9-]{1,61}[a-z0-9]$")
ROLE_ARN = re.compile(r"^acs:ram::[0-9]+:role/[A-Za-z0-9.@_-]+$")
SAFE_PROVIDER_CODE = re.compile(r"^[A-Za-z0-9_.:-]{1,80}$")
OpenUrl = Callable[..., object]


class PreflightError(RuntimeError):
    """A safe, operator-facing preflight failure."""


class NoRedirect(urllib.request.HTTPRedirectHandler):
    """Keep provider redirects observable instead of following them."""

    def redirect_request(self, req, fp, code, msg, headers, newurl):  # noqa: ANN001
        return None


def load_config(path: Path) -> dict[str, str]:
    try:
        content = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise PreflightError("cannot read the OSS environment file") from error

    if "\x00" in content or "\r" in content:
        raise PreflightError("OSS environment file contains unsupported control characters")

    config: dict[str, str] = {}
    for line in content.splitlines():
        if not line or line.startswith("#"):
            continue
        key, separator, value = line.partition("=")
        if not separator or key not in REQUIRED_KEYS or key in config:
            raise PreflightError("OSS environment file has an unknown or duplicate key")
        if not value or value != value.strip() or any(character.isspace() for character in value):
            raise PreflightError(f"{key} is empty or contains whitespace")
        config[key] = value

    missing = REQUIRED_KEYS - config.keys()
    if missing:
        raise PreflightError("OSS environment file is missing required keys")
    validate_config(config)
    return config


def validate_config(config: dict[str, str]) -> None:
    if not REGION.fullmatch(config["OSS_REGION"]):
        raise PreflightError("OSS_REGION has an invalid format")
    if not BUCKET.fullmatch(config["OSS_BUCKET"]):
        raise PreflightError("OSS_BUCKET has an invalid format")
    if not ROLE_ARN.fullmatch(config["OSS_ROLE_ARN"]):
        raise PreflightError("OSS_ROLE_ARN has an invalid format")

    callback = urllib.parse.urlsplit(config["OSS_CALLBACK_BASE_URL"])
    if (
        callback.scheme != "https"
        or not callback.hostname
        or callback.username
        or callback.password
        or callback.query
        or callback.fragment
    ):
        raise PreflightError("OSS_CALLBACK_BASE_URL must be a credential-free HTTPS base URL")


def percent_encode(value: str) -> str:
    return urllib.parse.quote(value, safe="-_.~")


def build_assume_role_url(
    config: dict[str, str], timestamp: datetime, nonce: str
) -> str:
    object_key = f"uploads/deploy-smoke/{nonce}"
    policy = json.dumps(
        {
            "Version": "1",
            "Statement": [
                {
                    "Effect": "Allow",
                    "Action": ["oss:PutObject"],
                    "Resource": [
                        f"acs:oss:*:*:{config['OSS_BUCKET']}/{object_key}"
                    ],
                    "Condition": {
                        "NumericLessThanEquals": {"oss:ContentLength": 20 * 1024 * 1024}
                    },
                }
            ],
        },
        separators=(",", ":"),
    )
    parameters = {
        "AccessKeyId": config["OSS_ACCESS_KEY_ID"],
        "Action": "AssumeRole",
        "DurationSeconds": "900",
        "Format": "JSON",
        "Policy": policy,
        "RoleArn": config["OSS_ROLE_ARN"],
        "RoleSessionName": "yourtj-deploy-smoke",
        "SignatureMethod": "HMAC-SHA1",
        "SignatureNonce": nonce,
        "SignatureVersion": "1.0",
        "Timestamp": timestamp.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "Version": "2015-04-01",
    }
    canonical = "&".join(
        f"{percent_encode(key)}={percent_encode(parameters[key])}"
        for key in sorted(parameters)
    )
    string_to_sign = f"GET&%2F&{percent_encode(canonical)}"
    signature = base64.b64encode(
        hmac.new(
            f"{config['OSS_ACCESS_KEY_SECRET']}&".encode(),
            string_to_sign.encode(),
            hashlib.sha1,
        ).digest()
    ).decode()
    parameters["Signature"] = signature
    query = "&".join(
        f"{percent_encode(key)}={percent_encode(parameters[key])}"
        for key in sorted(parameters)
    )
    return f"https://sts.aliyuncs.com/?{query}"


def provider_code(error: urllib.error.HTTPError) -> str:
    try:
        payload = json.loads(error.read().decode("utf-8"))
        code = payload.get("Code", "Unknown")
    except (UnicodeError, json.JSONDecodeError, AttributeError):
        return "Unknown"
    return code if isinstance(code, str) and SAFE_PROVIDER_CODE.fullmatch(code) else "Unknown"


def check_bucket_endpoint(config: dict[str, str], opener: OpenUrl) -> None:
    endpoint = f"https://{config['OSS_BUCKET']}.oss-{config['OSS_REGION']}.aliyuncs.com/"
    request = urllib.request.Request(endpoint, method="HEAD")
    try:
        with opener(request, timeout=8) as response:
            if response.status != 200:
                raise PreflightError("OSS bucket endpoint returned an unexpected status")
    except urllib.error.HTTPError as error:
        try:
            if error.code != 403:
                raise PreflightError(
                    f"OSS bucket endpoint check failed with HTTP {error.code}"
                ) from error
        finally:
            error.close()
    except urllib.error.URLError as error:
        raise PreflightError("cannot reach the configured OSS bucket endpoint") from error


def check_callback_endpoint(config: dict[str, str], opener: OpenUrl) -> None:
    callback_url = f"{config['OSS_CALLBACK_BASE_URL'].rstrip('/')}/api/v2/media/callback"
    request = urllib.request.Request(
        callback_url,
        data=b"{}",
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with opener(request, timeout=8) as response:
            raise PreflightError(
                f"unsigned OSS callback unexpectedly returned HTTP {response.status}"
            )
    except urllib.error.HTTPError as error:
        try:
            if error.code not in {400, 401}:
                raise PreflightError(
                    f"OSS callback reachability check failed with HTTP {error.code}"
                ) from error
        finally:
            error.close()
    except urllib.error.URLError as error:
        raise PreflightError("cannot reach OSS_CALLBACK_BASE_URL over HTTPS") from error


def check_sts(config: dict[str, str], opener: OpenUrl) -> None:
    request_url = build_assume_role_url(config, datetime.now(timezone.utc), str(uuid.uuid4()))
    request = urllib.request.Request(request_url, method="GET")
    try:
        with opener(request, timeout=8) as response:
            payload = json.load(response)
    except urllib.error.HTTPError as error:
        try:
            code = provider_code(error)
            raise PreflightError(
                f"STS AssumeRole failed with HTTP {error.code}, code={code}"
            ) from error
        finally:
            error.close()
    except (urllib.error.URLError, UnicodeError, json.JSONDecodeError) as error:
        raise PreflightError("STS AssumeRole did not return a valid response") from error

    credentials = payload.get("Credentials") if isinstance(payload, dict) else None
    required = {"AccessKeyId", "AccessKeySecret", "SecurityToken", "Expiration"}
    if not isinstance(credentials, dict) or not required.issubset(credentials):
        raise PreflightError("STS AssumeRole response is missing temporary credentials")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--env-file", required=True, type=Path)
    arguments = parser.parse_args()

    try:
        config = load_config(arguments.env_file)
        opener = urllib.request.build_opener(NoRedirect()).open
        check_bucket_endpoint(config, opener)
        print("  OSS bucket endpoint: reachable")
        check_callback_endpoint(config, opener)
        print("  OSS callback endpoint: reachable over HTTPS")
        check_sts(config, opener)
        print("  Alibaba Cloud STS AssumeRole: OK")
    except PreflightError as error:
        print(f"OSS preflight failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
