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
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Callable, NamedTuple, Optional
from xml.etree import ElementTree


REQUIRED_KEYS = frozenset(
    {
        "OSS_REGION",
        "OSS_BUCKET",
        "OSS_ACCESS_KEY_ID",
        "OSS_ACCESS_KEY_SECRET",
        "OSS_ROLE_ARN",
        "OSS_CALLBACK_BASE_URL",
        "MEDIA_DELIVERY_OSS_BUCKET",
        "MEDIA_DELIVERY_OSS_ACCESS_KEY_ID",
        "MEDIA_DELIVERY_OSS_ACCESS_KEY_SECRET",
        "MEDIA_CDN_BASE_URL",
        "MEDIA_CDN_PRIMARY_KEY",
        "MEDIA_CDN_SECONDARY_KEY",
        "MEDIA_CDN_SIGNING_KEY_SLOT",
        "MEDIA_CDN_URL_TTL_SECONDS",
        "CDN_ACCESS_KEY_ID",
        "CDN_ACCESS_KEY_SECRET",
    }
)
REGION = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)+$")
BUCKET = re.compile(r"^[a-z0-9][a-z0-9-]{1,61}[a-z0-9]$")
ROLE_ARN = re.compile(r"^acs:ram::[0-9]+:role/[A-Za-z0-9.@_-]+$")
SAFE_PROVIDER_CODE = re.compile(r"^[A-Za-z0-9_.:-]{1,80}$")
SAFE_TASK_IDS = re.compile(r"^[0-9]+(?:,[0-9]+)*$")
SMOKE_OBJECT_PREFIX = "assets/deploy-smoke"
SMOKE_WEBP = base64.b64decode("UklGRhoAAABXRUJQVlA4TA0AAAAvAAAAEAcQERGIiP4HAA==")
HTTP_TIMEOUT_SECONDS = 8
MAX_PROVIDER_BODY_BYTES = 64 * 1024
PURGE_DEADLINE_SECONDS = 600
PURGE_POLL_INTERVAL_SECONDS = 5
OSS_V4_ALGORITHM = "OSS4-HMAC-SHA256"
OSS_V4_PAYLOAD = "UNSIGNED-PAYLOAD"
OpenUrl = Callable[..., object]
UtcNow = Callable[[], datetime]
UuidFactory = Callable[[], uuid.UUID]


class PreflightError(RuntimeError):
    """A safe, operator-facing preflight failure."""


class OssV4Signature(NamedTuple):
    authorization: str
    canonical_request: str
    string_to_sign: str


class StsUploadGrant(NamedTuple):
    access_key_id: str
    access_key_secret: str
    security_token: str
    object_key: str


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
        raise PreflightError(
            "OSS environment file contains unsupported control characters"
        )

    config: dict[str, str] = {}
    for line in content.splitlines():
        if not line or line.startswith("#"):
            continue
        key, separator, value = line.partition("=")
        if not separator or key not in REQUIRED_KEYS or key in config:
            raise PreflightError("OSS environment file has an unknown or duplicate key")
        if (
            not value
            or value != value.strip()
            or any(character.isspace() for character in value)
        ):
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
    if not BUCKET.fullmatch(config["MEDIA_DELIVERY_OSS_BUCKET"]):
        raise PreflightError("MEDIA_DELIVERY_OSS_BUCKET has an invalid format")
    if config["OSS_BUCKET"] == config["MEDIA_DELIVERY_OSS_BUCKET"]:
        raise PreflightError("Ingest and Delivery must use separate private buckets")
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
        raise PreflightError(
            "OSS_CALLBACK_BASE_URL must be a credential-free HTTPS base URL"
        )

    cdn = urllib.parse.urlsplit(config["MEDIA_CDN_BASE_URL"])
    try:
        cdn_port = cdn.port
    except ValueError as error:
        raise PreflightError(
            "MEDIA_CDN_BASE_URL must be one exact HTTPS origin"
        ) from error
    if (
        cdn.scheme != "https"
        or not cdn.hostname
        or cdn.username
        or cdn.password
        or cdn_port is not None
        or cdn.path not in {"", "/"}
        or cdn.query
        or cdn.fragment
        or not re.fullmatch(r"[A-Za-z0-9.-]+", cdn.hostname)
    ):
        raise PreflightError("MEDIA_CDN_BASE_URL must be one exact HTTPS origin")
    if config["MEDIA_CDN_SIGNING_KEY_SLOT"] not in {"primary", "secondary"}:
        raise PreflightError("MEDIA_CDN_SIGNING_KEY_SLOT must be primary or secondary")
    if config["MEDIA_CDN_URL_TTL_SECONDS"] != "300":
        raise PreflightError("MEDIA_CDN_URL_TTL_SECONDS must be exactly 300")
    if (
        config["MEDIA_CDN_PRIMARY_KEY"] == config["MEDIA_CDN_SECONDARY_KEY"]
        or len(config["MEDIA_CDN_PRIMARY_KEY"]) < 6
        or len(config["MEDIA_CDN_SECONDARY_KEY"]) < 6
    ):
        raise PreflightError("CDN signing keys must be distinct and rotatable")
    principals = {
        config["OSS_ACCESS_KEY_ID"],
        config["MEDIA_DELIVERY_OSS_ACCESS_KEY_ID"],
        config["CDN_ACCESS_KEY_ID"],
    }
    if len(principals) != 3:
        raise PreflightError(
            "Ingest, Delivery writer, and CDN purge principals must differ"
        )


def percent_encode(value: str) -> str:
    return urllib.parse.quote(value, safe="-_.~")


def utc_now() -> datetime:
    return datetime.now(timezone.utc)


def read_bounded(response: object, limit: int, label: str) -> bytes:
    headers = getattr(response, "headers", None)
    content_length = headers.get("Content-Length") if headers is not None else None
    if content_length is not None:
        try:
            declared_length = int(content_length)
        except (TypeError, ValueError) as error:
            raise PreflightError(
                f"{label} returned an invalid Content-Length"
            ) from error
        if declared_length < 0 or declared_length > limit:
            raise PreflightError(f"{label} exceeded the response-size limit")

    payload = response.read(limit + 1)
    if len(payload) > limit:
        raise PreflightError(f"{label} exceeded the response-size limit")
    return payload


def read_json(response: object, label: str) -> dict[str, object]:
    try:
        payload = json.loads(read_bounded(response, MAX_PROVIDER_BODY_BYTES, label))
    except (UnicodeError, json.JSONDecodeError) as error:
        raise PreflightError(f"{label} did not return valid JSON") from error
    if not isinstance(payload, dict):
        raise PreflightError(f"{label} did not return a JSON object")
    return payload


def response_header(response: object, name: str) -> Optional[str]:
    headers = getattr(response, "headers", None)
    if headers is None:
        return None
    value = headers.get(name)
    return value if isinstance(value, str) else None


def sign_oss_v4(
    access_key_id: str,
    access_key_secret: str,
    region: str,
    timestamp: str,
    method: str,
    canonical_uri: str,
    canonical_query: str,
    canonical_headers: dict[str, str],
    additional_header_names: tuple[str, ...] = (),
) -> OssV4Signature:
    if not re.fullmatch(r"[0-9]{8}T[0-9]{6}Z", timestamp):
        raise PreflightError("OSS V4 timestamp is invalid")
    if not REGION.fullmatch(region):
        raise PreflightError("OSS V4 signing region is invalid")
    if method not in {"PUT", "GET", "HEAD", "DELETE"}:
        raise PreflightError("OSS V4 method is invalid")
    if (
        not canonical_uri.startswith("/")
        or "?" in canonical_uri
        or "#" in canonical_uri
        or "\r" in canonical_uri
        or "\n" in canonical_uri
    ):
        raise PreflightError("OSS V4 canonical URI is invalid")
    if (
        canonical_query.startswith("?")
        or "#" in canonical_query
        or "\r" in canonical_query
        or "\n" in canonical_query
    ):
        raise PreflightError("OSS V4 canonical query is invalid")

    normalized_headers: dict[str, str] = {}
    for name, value in canonical_headers.items():
        if name != name.lower() or not re.fullmatch(r"[a-z0-9-]+", name):
            raise PreflightError("OSS V4 canonical header name is invalid")
        if "\r" in value or "\n" in value:
            raise PreflightError("OSS V4 canonical header value is invalid")
        normalized_headers[name] = value.strip()
    if normalized_headers.get("x-oss-date") != timestamp:
        raise PreflightError("OSS V4 canonical date header is invalid")
    if normalized_headers.get("x-oss-content-sha256") != OSS_V4_PAYLOAD:
        raise PreflightError("OSS V4 payload marker is invalid")

    additional_names = tuple(sorted(additional_header_names))
    if len(additional_names) != len(set(additional_names)):
        raise PreflightError("OSS V4 additional headers contain duplicates")
    for name in additional_names:
        if (
            name not in normalized_headers
            or name in {"content-md5", "content-type"}
            or name.startswith("x-oss-")
        ):
            raise PreflightError("OSS V4 additional header is invalid")
    additional_headers = ";".join(additional_names)
    canonical_headers_text = "".join(
        f"{name}:{normalized_headers[name]}\n" for name in sorted(normalized_headers)
    )
    canonical_request = (
        f"{method}\n{canonical_uri}\n{canonical_query}\n{canonical_headers_text}\n"
        f"{additional_headers}\n{OSS_V4_PAYLOAD}"
    )
    signing_date = timestamp[:8]
    scope = f"{signing_date}/{region}/oss/aliyun_v4_request"
    canonical_hash = hashlib.sha256(canonical_request.encode()).hexdigest()
    string_to_sign = f"{OSS_V4_ALGORITHM}\n{timestamp}\n{scope}\n{canonical_hash}"
    date_key = hmac.new(
        f"aliyun_v4{access_key_secret}".encode(),
        signing_date.encode(),
        hashlib.sha256,
    ).digest()
    region_key = hmac.new(date_key, region.encode(), hashlib.sha256).digest()
    service_key = hmac.new(region_key, b"oss", hashlib.sha256).digest()
    signing_key = hmac.new(service_key, b"aliyun_v4_request", hashlib.sha256).digest()
    signature = hmac.new(
        signing_key, string_to_sign.encode(), hashlib.sha256
    ).hexdigest()
    authorization_fields = [f"Credential={access_key_id}/{scope}"]
    if additional_headers:
        authorization_fields.append(f"AdditionalHeaders={additional_headers}")
    authorization_fields.append(f"Signature={signature}")
    authorization = f"{OSS_V4_ALGORITHM} {','.join(authorization_fields)}"
    return OssV4Signature(authorization, canonical_request, string_to_sign)


def build_oss_v4_request(
    config: dict[str, str],
    method: str,
    object_key: str,
    timestamp: datetime,
    body: Optional[bytes] = None,
) -> urllib.request.Request:
    if not re.fullmatch(r"assets/deploy-smoke/[0-9a-f]{32}\.webp", object_key):
        raise PreflightError("synthetic Delivery object key is invalid")
    if (method == "PUT") != (body is not None):
        raise PreflightError("synthetic Delivery request body is invalid")

    bucket = config["MEDIA_DELIVERY_OSS_BUCKET"]
    request_timestamp = timestamp.astimezone(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    canonical_headers = {
        "x-oss-content-sha256": OSS_V4_PAYLOAD,
        "x-oss-date": request_timestamp,
    }
    if body is not None:
        canonical_headers.update(
            {
                "content-type": "image/webp",
                "x-oss-forbid-overwrite": "true",
                "x-oss-meta-content-sha256": hashlib.sha256(body).hexdigest(),
            }
        )
    encoded_key = urllib.parse.quote(object_key, safe="/-_.~")
    signed = sign_oss_v4(
        config["MEDIA_DELIVERY_OSS_ACCESS_KEY_ID"],
        config["MEDIA_DELIVERY_OSS_ACCESS_KEY_SECRET"],
        config["OSS_REGION"],
        request_timestamp,
        method,
        f"/{bucket}/{encoded_key}",
        "",
        canonical_headers,
    )
    endpoint = f"https://{bucket}.oss-{config['OSS_REGION']}.aliyuncs.com/{encoded_key}"
    return urllib.request.Request(
        endpoint,
        data=body,
        headers={
            "Accept-Encoding": "identity",
            **canonical_headers,
            "Authorization": signed.authorization,
        },
        method=method,
    )


def build_type_a_url(
    config: dict[str, str], object_key: str, timestamp: datetime, nonce: str
) -> str:
    path = f"/{object_key}"
    issued_at = int(timestamp.astimezone(timezone.utc).timestamp())
    key = config[
        "MEDIA_CDN_PRIMARY_KEY"
        if config["MEDIA_CDN_SIGNING_KEY_SLOT"] == "primary"
        else "MEDIA_CDN_SECONDARY_KEY"
    ]
    digest = hashlib.md5(f"{path}-{issued_at}-{nonce}-0-{key}".encode()).hexdigest()
    auth_key = f"{issued_at}-{nonce}-0-{digest}"
    return f"{config['MEDIA_CDN_BASE_URL'].rstrip('/')}{path}?auth_key={auth_key}"


def build_cdn_rpc_request(
    config: dict[str, str],
    action: str,
    action_parameters: dict[str, str],
    timestamp: datetime,
    nonce: str,
) -> urllib.request.Request:
    parameters = {
        "AccessKeyId": config["CDN_ACCESS_KEY_ID"],
        "Action": action,
        "Format": "JSON",
        "SignatureMethod": "HMAC-SHA1",
        "SignatureNonce": nonce,
        "SignatureVersion": "1.0",
        "Timestamp": timestamp.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "Version": "2018-05-10",
        **action_parameters,
    }
    canonical = "&".join(
        f"{percent_encode(key)}={percent_encode(parameters[key])}"
        for key in sorted(parameters)
    )
    string_to_sign = f"POST&%2F&{percent_encode(canonical)}"
    parameters["Signature"] = base64.b64encode(
        hmac.new(
            f"{config['CDN_ACCESS_KEY_SECRET']}&".encode(),
            string_to_sign.encode(),
            hashlib.sha1,
        ).digest()
    ).decode()
    body = "&".join(
        f"{percent_encode(key)}={percent_encode(parameters[key])}"
        for key in sorted(parameters)
    ).encode()
    return urllib.request.Request(
        "https://cdn.aliyuncs.com/",
        data=body,
        headers={"Content-Type": "application/x-www-form-urlencoded"},
        method="POST",
    )


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
                    "Resource": [f"acs:oss:*:*:{config['OSS_BUCKET']}/{object_key}"],
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
        body = read_bounded(error, MAX_PROVIDER_BODY_BYTES, "provider error response")
    except PreflightError:
        return "Unknown"
    try:
        payload = json.loads(body)
        code = (
            payload.get("Code", "Unknown") if isinstance(payload, dict) else "Unknown"
        )
    except (UnicodeError, json.JSONDecodeError):
        try:
            code = ElementTree.fromstring(body).findtext("Code", default="Unknown")
        except ElementTree.ParseError:
            return "Unknown"
    return (
        code
        if isinstance(code, str) and SAFE_PROVIDER_CODE.fullmatch(code)
        else "Unknown"
    )


def check_bucket_endpoint(
    config: dict[str, str], opener: OpenUrl, bucket_key: str = "OSS_BUCKET"
) -> None:
    endpoint = f"https://{config[bucket_key]}.oss-{config['OSS_REGION']}.aliyuncs.com/"
    request = urllib.request.Request(endpoint, method="HEAD")
    try:
        with opener(request, timeout=HTTP_TIMEOUT_SECONDS) as response:
            raise PreflightError(
                f"OSS bucket accepted an unsigned request with HTTP {response.status}"
            )
    except urllib.error.HTTPError as error:
        try:
            if error.code != 403:
                raise PreflightError(
                    f"OSS bucket endpoint check failed with HTTP {error.code}"
                ) from error
        finally:
            error.close()
    except urllib.error.URLError as error:
        raise PreflightError(
            "cannot reach the configured OSS bucket endpoint"
        ) from error


def check_callback_endpoint(config: dict[str, str], opener: OpenUrl) -> None:
    callback_url = (
        f"{config['OSS_CALLBACK_BASE_URL'].rstrip('/')}/api/v2/media/callback"
    )
    request = urllib.request.Request(
        callback_url,
        data=b"{}",
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with opener(request, timeout=HTTP_TIMEOUT_SECONDS) as response:
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


def check_sts(config: dict[str, str], opener: OpenUrl) -> StsUploadGrant:
    nonce = str(uuid.uuid4())
    request_url = build_assume_role_url(config, utc_now(), nonce)
    request = urllib.request.Request(request_url, method="GET")
    try:
        with opener(request, timeout=HTTP_TIMEOUT_SECONDS) as response:
            payload = read_json(response, "STS AssumeRole response")
    except urllib.error.HTTPError as error:
        try:
            code = provider_code(error)
            raise PreflightError(
                f"STS AssumeRole failed with HTTP {error.code}, code={code}"
            ) from error
        finally:
            error.close()
    except (urllib.error.URLError, UnicodeError, json.JSONDecodeError) as error:
        raise PreflightError(
            "STS AssumeRole did not return a valid response"
        ) from error

    credentials = payload.get("Credentials") if isinstance(payload, dict) else None
    required = {"AccessKeyId", "AccessKeySecret", "SecurityToken", "Expiration"}
    if not isinstance(credentials, dict) or not required.issubset(credentials):
        raise PreflightError("STS AssumeRole response is missing temporary credentials")
    values = [
        credentials.get(name)
        for name in ("AccessKeyId", "AccessKeySecret", "SecurityToken")
    ]
    if any(not isinstance(value, str) or not value for value in values):
        raise PreflightError(
            "STS AssumeRole response contains invalid temporary credentials"
        )
    return StsUploadGrant(
        values[0], values[1], values[2], f"uploads/deploy-smoke/{nonce}"
    )


def build_ingest_v4_request(
    config: dict[str, str],
    method: str,
    object_key: str,
    timestamp: datetime,
    body: Optional[bytes],
    grant: Optional[StsUploadGrant],
    forbid_overwrite: bool,
) -> urllib.request.Request:
    if not re.fullmatch(r"uploads/deploy-smoke/[0-9a-f-]{36}", object_key):
        raise PreflightError("synthetic Ingest object key is invalid")
    if (method == "PUT") != (body is not None):
        raise PreflightError("synthetic Ingest request body is invalid")
    if grant is not None and grant.object_key != object_key:
        raise PreflightError("STS grant is not scoped to the synthetic Ingest key")
    access_key_id = (
        grant.access_key_id if grant is not None else config["OSS_ACCESS_KEY_ID"]
    )
    access_key_secret = (
        grant.access_key_secret
        if grant is not None
        else config["OSS_ACCESS_KEY_SECRET"]
    )
    request_timestamp = timestamp.astimezone(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    canonical_headers = {
        "x-oss-content-sha256": OSS_V4_PAYLOAD,
        "x-oss-date": request_timestamp,
    }
    if grant is not None:
        canonical_headers["x-oss-security-token"] = grant.security_token
    if body is not None:
        canonical_headers["content-type"] = "application/octet-stream"
    if forbid_overwrite:
        canonical_headers["x-oss-forbid-overwrite"] = "true"
    bucket = config["OSS_BUCKET"]
    encoded_key = urllib.parse.quote(object_key, safe="/-_.~")
    signed = sign_oss_v4(
        access_key_id,
        access_key_secret,
        config["OSS_REGION"],
        request_timestamp,
        method,
        f"/{bucket}/{encoded_key}",
        "",
        canonical_headers,
    )
    endpoint = f"https://{bucket}.oss-{config['OSS_REGION']}.aliyuncs.com/{encoded_key}"
    return urllib.request.Request(
        endpoint,
        data=body,
        headers={
            "Accept-Encoding": "identity",
            **canonical_headers,
            "Authorization": signed.authorization,
        },
        method=method,
    )


def check_ingest_prevent_overwrite(
    config: dict[str, str], opener: OpenUrl, grant: StsUploadGrant
) -> None:
    first_body = b"yourtj-ingest-preflight-a"
    second_body = b"yourtj-ingest-preflight-b"
    object_may_exist = False
    primary_error: Optional[PreflightError] = None
    cleanup_failed = False
    try:
        object_may_exist = True
        first = build_ingest_v4_request(
            config, "PUT", grant.object_key, utc_now(), first_body, grant, True
        )
        with opener(first, timeout=HTTP_TIMEOUT_SECONDS) as response:
            if response.status != 200:
                raise PreflightError("initial synthetic Ingest PUT was rejected")
            read_bounded(response, 1024, "initial synthetic Ingest PUT response")
        overwrite = build_ingest_v4_request(
            config, "PUT", grant.object_key, utc_now(), second_body, grant, False
        )
        try:
            with opener(overwrite, timeout=HTTP_TIMEOUT_SECONDS) as response:
                raise PreflightError(
                    f"Ingest prevent-overwrite rule allowed HTTP {response.status}"
                )
        except urllib.error.HTTPError as error:
            try:
                code = provider_code(error)
                if error.code != 409 or code != "FileAlreadyExists":
                    raise PreflightError(
                        "Ingest prevent-overwrite rule did not reject a headerless overwrite"
                    ) from error
            finally:
                error.close()
    except (PreflightError, urllib.error.URLError) as error:
        primary_error = (
            error
            if isinstance(error, PreflightError)
            else PreflightError("cannot verify the Ingest prevent-overwrite rule")
        )
    finally:
        if object_may_exist:
            delete = build_ingest_v4_request(
                config, "DELETE", grant.object_key, utc_now(), None, None, False
            )
            try:
                with opener(delete, timeout=HTTP_TIMEOUT_SECONDS) as response:
                    if response.status not in {200, 204}:
                        cleanup_failed = True
            except urllib.error.HTTPError as error:
                cleanup_failed = error.code != 404
                error.close()
            except urllib.error.URLError:
                cleanup_failed = True
    if primary_error is not None:
        if cleanup_failed:
            raise PreflightError(
                f"{primary_error}; synthetic Ingest cleanup was incomplete"
            ) from primary_error
        raise primary_error
    if cleanup_failed:
        raise PreflightError("synthetic Ingest cleanup was incomplete")


def provider_request_error(label: str, error: urllib.error.HTTPError) -> PreflightError:
    code = provider_code(error)
    return PreflightError(f"{label} failed with HTTP {error.code}, code={code}")


def put_delivery_fixture(
    config: dict[str, str],
    opener: OpenUrl,
    object_key: str,
    timestamp: datetime,
) -> None:
    request = build_oss_v4_request(config, "PUT", object_key, timestamp, SMOKE_WEBP)
    try:
        with opener(request, timeout=HTTP_TIMEOUT_SECONDS) as response:
            if response.status != 200:
                raise PreflightError(
                    f"Delivery fixture PUT returned HTTP {response.status}"
                )
            read_bounded(response, 1024, "Delivery fixture PUT response")
    except urllib.error.HTTPError as error:
        try:
            raise provider_request_error("Delivery fixture PUT", error) from error
        finally:
            error.close()
    except urllib.error.URLError as error:
        raise PreflightError("cannot write the synthetic Delivery object") from error


def head_delivery_fixture(
    config: dict[str, str],
    opener: OpenUrl,
    object_key: str,
    timestamp: datetime,
) -> None:
    request = build_oss_v4_request(config, "HEAD", object_key, timestamp)
    try:
        with opener(request, timeout=HTTP_TIMEOUT_SECONDS) as response:
            if response.status != 200:
                raise PreflightError(
                    f"Delivery fixture HEAD returned HTTP {response.status}"
                )
            expected_digest = hashlib.sha256(SMOKE_WEBP).hexdigest()
            if response_header(response, "Content-Length") != str(len(SMOKE_WEBP)):
                raise PreflightError("Delivery fixture HEAD returned the wrong length")
            if response_header(response, "Content-Type") != "image/webp":
                raise PreflightError(
                    "Delivery fixture HEAD returned the wrong media type"
                )
            if (
                response_header(response, "x-oss-meta-content-sha256")
                != expected_digest
            ):
                raise PreflightError(
                    "Delivery fixture HEAD returned the wrong digest metadata"
                )
    except urllib.error.HTTPError as error:
        try:
            raise provider_request_error("Delivery fixture HEAD", error) from error
        finally:
            error.close()
    except urllib.error.URLError as error:
        raise PreflightError("cannot verify the synthetic Delivery object") from error


def get_delivery_fixture(
    config: dict[str, str],
    opener: OpenUrl,
    object_key: str,
    timestamp: datetime,
) -> None:
    request = build_oss_v4_request(config, "GET", object_key, timestamp)
    try:
        with opener(request, timeout=HTTP_TIMEOUT_SECONDS) as response:
            if response.status != 200:
                raise PreflightError(
                    f"Delivery fixture GET returned HTTP {response.status}"
                )
            if response_header(response, "Content-Type") != "image/webp":
                raise PreflightError(
                    "Delivery fixture GET returned the wrong media type"
                )
            payload = read_bounded(
                response, len(SMOKE_WEBP), "Delivery fixture GET response"
            )
            if payload != SMOKE_WEBP:
                raise PreflightError("Delivery fixture GET returned different bytes")
    except urllib.error.HTTPError as error:
        try:
            raise provider_request_error("Delivery fixture GET", error) from error
        finally:
            error.close()
    except urllib.error.URLError as error:
        raise PreflightError("cannot read the synthetic Delivery object") from error


def delete_delivery_fixture(
    config: dict[str, str],
    opener: OpenUrl,
    object_key: str,
    timestamp: datetime,
) -> None:
    request = build_oss_v4_request(config, "DELETE", object_key, timestamp)
    try:
        with opener(request, timeout=HTTP_TIMEOUT_SECONDS) as response:
            if response.status not in {200, 204}:
                raise PreflightError(
                    f"Delivery fixture DELETE returned HTTP {response.status}"
                )
            read_bounded(response, 1024, "Delivery fixture DELETE response")
    except urllib.error.HTTPError as error:
        try:
            if error.code != 404:
                raise provider_request_error(
                    "Delivery fixture DELETE", error
                ) from error
        finally:
            error.close()
    except urllib.error.URLError as error:
        raise PreflightError("cannot delete the synthetic Delivery object") from error


def check_unsigned_cdn_fixture(
    config: dict[str, str], opener: OpenUrl, object_key: str
) -> None:
    request = urllib.request.Request(
        f"{config['MEDIA_CDN_BASE_URL'].rstrip('/')}/{object_key}",
        headers={"Accept-Encoding": "identity"},
        method="GET",
    )
    try:
        with opener(request, timeout=HTTP_TIMEOUT_SECONDS) as response:
            raise PreflightError(
                f"unsigned CDN fixture unexpectedly returned HTTP {response.status}"
            )
    except urllib.error.HTTPError as error:
        try:
            if error.code != 403:
                raise PreflightError(
                    f"unsigned CDN fixture returned HTTP {error.code}; expected 403"
                ) from error
        finally:
            error.close()
    except urllib.error.URLError as error:
        raise PreflightError(
            "cannot reach the synthetic Delivery object through CDN"
        ) from error


def get_signed_cdn_fixture(
    config: dict[str, str],
    opener: OpenUrl,
    object_key: str,
    timestamp: datetime,
    nonce: str,
) -> None:
    request = urllib.request.Request(
        build_type_a_url(config, object_key, timestamp, nonce),
        headers={"Accept-Encoding": "identity"},
        method="GET",
    )
    try:
        with opener(request, timeout=HTTP_TIMEOUT_SECONDS) as response:
            if response.status != 200:
                raise PreflightError(
                    f"signed CDN fixture returned HTTP {response.status}"
                )
            if (
                read_bounded(response, len(SMOKE_WEBP), "signed CDN fixture response")
                != SMOKE_WEBP
            ):
                raise PreflightError("signed CDN fixture returned different bytes")
    except urllib.error.HTTPError as error:
        try:
            raise PreflightError(
                f"signed CDN fixture failed with HTTP {error.code}"
            ) from error
        finally:
            error.close()
    except urllib.error.URLError as error:
        raise PreflightError("cannot read the signed synthetic CDN object") from error


def call_cdn_rpc(
    config: dict[str, str],
    opener: OpenUrl,
    action: str,
    action_parameters: dict[str, str],
    timestamp: datetime,
    nonce: str,
) -> dict[str, object]:
    request = build_cdn_rpc_request(config, action, action_parameters, timestamp, nonce)
    label = (
        "CDN purge submission"
        if action == "RefreshObjectCaches"
        else "CDN purge status"
    )
    try:
        with opener(request, timeout=HTTP_TIMEOUT_SECONDS) as response:
            if response.status != 200:
                raise PreflightError(f"{label} returned HTTP {response.status}")
            return read_json(response, f"{label} response")
    except urllib.error.HTTPError as error:
        try:
            raise provider_request_error(label, error) from error
        finally:
            error.close()
    except urllib.error.URLError as error:
        raise PreflightError(f"cannot reach Alibaba Cloud {label}") from error


def normalized_task_ids(value: object) -> str:
    if type(value) is int:
        value = str(value)
    if (
        not isinstance(value, str)
        or len(value) > 255
        or not SAFE_TASK_IDS.fullmatch(value)
    ):
        raise PreflightError("CDN purge returned an invalid task identifier")
    return value


def submit_cdn_purge(
    config: dict[str, str],
    opener: OpenUrl,
    object_key: str,
    timestamp: datetime,
    nonce: str,
) -> str:
    object_url = f"{config['MEDIA_CDN_BASE_URL'].rstrip('/')}/{object_key}"
    response = call_cdn_rpc(
        config,
        opener,
        "RefreshObjectCaches",
        {"ObjectPath": object_url, "ObjectType": "File"},
        timestamp,
        nonce,
    )
    return normalized_task_ids(response.get("RefreshTaskId"))


def cdn_purge_is_complete(
    config: dict[str, str],
    opener: OpenUrl,
    task_ids: str,
    timestamp: datetime,
    nonce: str,
) -> bool:
    expected_ids = set(normalized_task_ids(task_ids).split(","))
    response = call_cdn_rpc(
        config,
        opener,
        "DescribeRefreshTaskById",
        {"TaskId": task_ids},
        timestamp,
        nonce,
    )
    tasks = response.get("Tasks")
    if not isinstance(tasks, list) or not tasks:
        raise PreflightError("CDN purge task was not found")

    observed_ids: set[str] = set()
    is_refreshing = False
    for task in tasks:
        if not isinstance(task, dict):
            raise PreflightError("CDN purge status response is invalid")
        observed_ids.update(normalized_task_ids(task.get("TaskId")).split(","))
        status = task.get("Status")
        if status == "Complete":
            continue
        if status == "Refreshing":
            is_refreshing = True
            continue
        if status in {"Timeout", "Canceled", "Failed"}:
            raise PreflightError("CDN purge task entered a terminal failure")
        raise PreflightError("CDN purge task returned an unknown status")
    if observed_ids != expected_ids:
        raise PreflightError("CDN purge status referred to a different task")
    return not is_refreshing


def poll_cdn_purge(
    config: dict[str, str],
    opener: OpenUrl,
    task_ids: str,
    now: UtcNow,
    uuid_factory: UuidFactory,
    monotonic: Callable[[], float] = time.monotonic,
    sleep: Callable[[float], None] = time.sleep,
    deadline_seconds: int = PURGE_DEADLINE_SECONDS,
) -> None:
    deadline = monotonic() + deadline_seconds
    while True:
        if monotonic() >= deadline:
            raise PreflightError("CDN purge did not complete before the deadline")
        is_complete = cdn_purge_is_complete(
            config,
            opener,
            task_ids,
            now(),
            str(uuid_factory()),
        )
        if monotonic() > deadline:
            raise PreflightError("CDN purge did not complete before the deadline")
        if is_complete:
            return
        remaining = deadline - monotonic()
        sleep(min(PURGE_POLL_INTERVAL_SECONDS, remaining))


def run_delivery_smoke(
    config: dict[str, str],
    opener: OpenUrl,
    now: UtcNow = utc_now,
    uuid_factory: UuidFactory = uuid.uuid4,
    monotonic: Callable[[], float] = time.monotonic,
    sleep: Callable[[float], None] = time.sleep,
) -> None:
    object_key = f"{SMOKE_OBJECT_PREFIX}/{uuid_factory().hex}.webp"
    object_may_exist = False
    purge_completed = False
    primary_error: Optional[PreflightError] = None
    cleanup_failed = False
    try:
        object_may_exist = True
        put_delivery_fixture(config, opener, object_key, now())
        head_delivery_fixture(config, opener, object_key, now())
        get_delivery_fixture(config, opener, object_key, now())
        check_unsigned_cdn_fixture(config, opener, object_key)
        get_signed_cdn_fixture(
            config,
            opener,
            object_key,
            now(),
            uuid_factory().hex,
        )
        task_ids = submit_cdn_purge(
            config,
            opener,
            object_key,
            now(),
            str(uuid_factory()),
        )
        poll_cdn_purge(
            config,
            opener,
            task_ids,
            now,
            uuid_factory,
            monotonic,
            sleep,
        )
        purge_completed = True
    except PreflightError as error:
        primary_error = error
    finally:
        if object_may_exist:
            if not purge_completed:
                try:
                    submit_cdn_purge(
                        config,
                        opener,
                        object_key,
                        now(),
                        str(uuid_factory()),
                    )
                except PreflightError:
                    cleanup_failed = True
            try:
                delete_delivery_fixture(config, opener, object_key, now())
            except PreflightError:
                cleanup_failed = True

    if primary_error is not None:
        if cleanup_failed:
            raise PreflightError(
                f"{primary_error}; synthetic media cleanup was incomplete"
            ) from primary_error
        raise primary_error
    if cleanup_failed:
        raise PreflightError("synthetic media cleanup was incomplete")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--env-file", required=True, type=Path)
    arguments = parser.parse_args()

    try:
        config = load_config(arguments.env_file)
        opener = urllib.request.build_opener(NoRedirect()).open
        check_bucket_endpoint(config, opener)
        print("  Ingest OSS bucket endpoint: private and reachable")
        check_bucket_endpoint(config, opener, "MEDIA_DELIVERY_OSS_BUCKET")
        print("  Delivery OSS bucket endpoint: private and reachable")
        check_callback_endpoint(config, opener)
        print("  OSS callback endpoint: reachable over HTTPS")
        grant = check_sts(config, opener)
        check_ingest_prevent_overwrite(config, opener, grant)
        print(
            "  Alibaba Cloud STS exact-key upload and Ingest overwrite protection: OK"
        )
        run_delivery_smoke(config, opener)
        print("  Delivery OSS/CDN write-read-auth-purge cleanup: OK")
    except PreflightError as error:
        print(f"OSS preflight failed: {error}", file=sys.stderr)
        return 1
    except Exception:
        print(
            "OSS preflight failed: unexpected internal preflight error", file=sys.stderr
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
