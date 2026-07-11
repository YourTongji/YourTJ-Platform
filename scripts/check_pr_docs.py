#!/usr/bin/env python3
"""Require every pull request to declare and match its documentation impact."""

from __future__ import annotations

import os
import re
import subprocess
import sys


SECTION = re.compile(
    r"^##\s+(?:文档影响|Documentation impact)\s*$\n(?P<body>.*?)(?=^##\s|\Z)",
    re.IGNORECASE | re.MULTILINE | re.DOTALL,
)
HTML_COMMENT = re.compile(r"<!--.*?-->", re.DOTALL)
NONE_DECLARATION = re.compile(r"\bDocs\s+impact\s*:\s*none\b", re.IGNORECASE)


def changed_files(base_sha: str, head_sha: str) -> list[str]:
    result = subprocess.run(
        ["git", "diff", "--name-only", "--diff-filter=ACDMRT", base_sha, head_sha],
        check=True,
        capture_output=True,
        text=True,
    )
    return [line for line in result.stdout.splitlines() if line]


def is_documentation(path: str) -> bool:
    return (
        path.startswith(("docs/", ".agents/skills/"))
        or path in {"AGENTS.md", "README.md", ".github/PULL_REQUEST_TEMPLATE.md"}
        or path.endswith("/README.md")
    )


def requires_canonical_docs(path: str) -> bool:
    return (
        path == "contract/openapi.yaml"
        or path.startswith(("backend/migrations/", ".github/workflows/"))
        or path in {"backend/.env.example", "docker-compose.yml"}
    )


def validate(body: str, paths: list[str]) -> list[str]:
    errors: list[str] = []
    match = SECTION.search(body)
    if match is None:
        return ["PR body must contain a '## 文档影响' or '## Documentation impact' section"]

    content = HTML_COMMENT.sub("", match.group("body")).strip()
    if not content:
        return ["PR documentation-impact section must not be empty"]

    docs_changed = any(is_documentation(path) for path in paths)
    docs_required = any(requires_canonical_docs(path) for path in paths)
    declares_none = NONE_DECLARATION.search(content) is not None
    if docs_changed and declares_none:
        errors.append("PR changes documentation but declares 'Docs impact: none'")
    elif docs_required and not docs_changed:
        errors.append("contract, migration, workflow, or runtime-config changes require canonical docs")
    elif not docs_changed and not declares_none:
        errors.append("PR changes no canonical documentation; declare 'Docs impact: none' with a reason")

    if declares_none:
        explanation = NONE_DECLARATION.sub("", content).strip(" .:;，。；-—\n\t")
        if len(explanation) < 15:
            errors.append("'Docs impact: none' requires a concrete explanation")

    return errors


def main() -> int:
    body = os.environ.get("PR_BODY", "")
    base_sha = os.environ.get("BASE_SHA", "")
    head_sha = os.environ.get("HEAD_SHA", "")
    if not base_sha or not head_sha:
        print("BASE_SHA and HEAD_SHA are required", file=sys.stderr)
        return 2

    try:
        paths = changed_files(base_sha, head_sha)
    except subprocess.CalledProcessError as error:
        print(error.stderr or "could not inspect PR changed files", file=sys.stderr)
        return 2

    errors = validate(body, paths)
    if errors:
        print("PR documentation-impact validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print("PR documentation-impact validation passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
