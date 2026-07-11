#!/usr/bin/env python3
"""Validate YourTJ's documentation layout, metadata, status language, and links."""

from __future__ import annotations

import os
import re
import sys
from pathlib import Path
from urllib.parse import unquote, urlparse


DOC_CATEGORIES = {"architecture", "development", "operations", "product", "security"}
ROOT_MARKDOWN = {"AGENTS.md", "README.md"}
METADATA_FIELDS = ("文档类型", "状态", "负责人", "最近核验")
STALE_STATUS = re.compile(r"DELIVERED IN THIS PR|FOLLOW-UP|against this PR", re.IGNORECASE)
KEBAB_MARKDOWN = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*\.md")
KEBAB_NAME = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*")
MARKDOWN_LINK = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")
FENCED_CODE = re.compile(r"```.*?```", re.DOTALL)


def find_repo_root() -> Path:
    for candidate in Path(__file__).resolve().parents:
        if (candidate / ".git").exists():
            return candidate
    raise RuntimeError("could not locate repository root")


def local_link_target(raw_target: str) -> str | None:
    target = raw_target.strip()
    if target.startswith("<") and ">" in target:
        target = target[1 : target.index(">")]
    else:
        target = target.split(maxsplit=1)[0]

    parsed = urlparse(target)
    if parsed.scheme or target.startswith(("#", "/")):
        return None

    path = unquote(target.split("#", 1)[0].split("?", 1)[0])
    return path or None


def markdown_files(repo_root: Path) -> list[Path]:
    skipped_parts = {".git", ".codex", "node_modules", "target"}
    return sorted(
        path
        for path in repo_root.rglob("*.md")
        if not skipped_parts.intersection(path.relative_to(repo_root).parts)
    )


def has_exact_case(repo_root: Path, destination: Path) -> bool:
    normalized = Path(os.path.normpath(destination))
    try:
        relative = normalized.relative_to(repo_root)
    except ValueError:
        return False

    current = repo_root
    for part in relative.parts:
        if not current.is_dir() or part not in {entry.name for entry in current.iterdir()}:
            return False
        current /= part
    return True


def validate_repo_skills(repo_root: Path) -> list[str]:
    errors: list[str] = []
    skills_root = repo_root / ".agents" / "skills"
    if not skills_root.exists():
        return errors

    for skill_dir in sorted(path for path in skills_root.iterdir() if path.is_dir()):
        relative = skill_dir.relative_to(repo_root)
        skill_file = skill_dir / "SKILL.md"
        if not skill_file.is_file():
            errors.append(f"{relative}: missing SKILL.md")
            continue

        text = skill_file.read_text(encoding="utf-8")
        lines = text.splitlines()
        if not lines or lines[0] != "---":
            errors.append(f"{relative}/SKILL.md: missing YAML frontmatter")
            continue
        try:
            closing_index = lines.index("---", 1)
        except ValueError:
            errors.append(f"{relative}/SKILL.md: unclosed YAML frontmatter")
            continue

        metadata: dict[str, str] = {}
        for line in lines[1:closing_index]:
            key, separator, value = line.partition(":")
            if not separator or not key.strip() or not value.strip():
                errors.append(f"{relative}/SKILL.md: frontmatter must use scalar key/value pairs")
                continue
            metadata[key.strip()] = value.strip()

        if set(metadata) != {"name", "description"}:
            errors.append(f"{relative}/SKILL.md: frontmatter permits only name and description")
        name = metadata.get("name", "")
        if name != skill_dir.name or not KEBAB_NAME.fullmatch(name):
            errors.append(f"{relative}/SKILL.md: skill name must match its kebab-case directory")
        description = metadata.get("description", "")
        if len(description) < 40 or "TODO" in description:
            errors.append(f"{relative}/SKILL.md: description must explain behavior and trigger conditions")

        interface_file = skill_dir / "agents" / "openai.yaml"
        if not interface_file.is_file():
            errors.append(f"{relative}: missing agents/openai.yaml")
            continue
        interface = interface_file.read_text(encoding="utf-8")
        short_match = re.search(r'^\s*short_description:\s*"([^"]+)"\s*$', interface, re.MULTILINE)
        if not short_match or not 25 <= len(short_match.group(1)) <= 64:
            errors.append(f"{relative}/agents/openai.yaml: short_description must be 25-64 characters")
        if not re.search(r'^\s*display_name:\s*"[^"]+"\s*$', interface, re.MULTILINE):
            errors.append(f"{relative}/agents/openai.yaml: missing quoted display_name")
        if f"${name}" not in interface:
            errors.append(f"{relative}/agents/openai.yaml: default_prompt must mention ${name}")

    return errors


def validate_docs(repo_root: Path) -> list[str]:
    errors: list[str] = []
    docs_root = repo_root / "docs"
    canonical_docs = sorted(docs_root.rglob("*.md"))

    for root_doc in sorted(repo_root.glob("*.md")):
        if root_doc.name not in ROOT_MARKDOWN:
            errors.append(f"{root_doc.relative_to(repo_root)}: root Markdown must be classified under docs/")

    for path in canonical_docs:
        relative = path.relative_to(repo_root)
        within_docs = path.relative_to(docs_root)

        if len(within_docs.parts) == 1:
            if path.name != "README.md":
                errors.append(f"{relative}: docs/ root only permits README.md")
        elif within_docs.parts[0] not in DOC_CATEGORIES:
            errors.append(f"{relative}: unknown documentation category")

        if path.name != "README.md" and not KEBAB_MARKDOWN.fullmatch(path.name):
            errors.append(f"{relative}: filename must be lowercase kebab-case")

        text = path.read_text(encoding="utf-8")
        header = "\n".join(text.splitlines()[:20])
        for field in METADATA_FIELDS:
            if f"> {field}：" not in header:
                errors.append(f"{relative}: missing metadata field {field}")

        if not re.search(r"^> 状态：(Active|Draft|Deprecated)$", header, re.MULTILINE):
            errors.append(f"{relative}: lifecycle status must be Active, Draft, or Deprecated")

        prose = FENCED_CODE.sub("", text)
        if len(re.findall(r"^# ", prose, re.MULTILINE)) != 1:
            errors.append(f"{relative}: document must contain exactly one H1")

        stale_match = STALE_STATUS.search(text)
        if stale_match:
            errors.append(f"{relative}: stale PR-relative status phrase {stale_match.group(0)!r}")

    for source in markdown_files(repo_root):
        prose = FENCED_CODE.sub("", source.read_text(encoding="utf-8"))
        for match in MARKDOWN_LINK.finditer(prose):
            target = local_link_target(match.group(1))
            if target is None:
                continue
            destination = source.parent / target
            if not destination.exists() or not has_exact_case(repo_root, destination):
                relative_source = source.relative_to(repo_root)
                errors.append(f"{relative_source}: broken local link {match.group(1)!r}")

    errors.extend(validate_repo_skills(repo_root))
    return errors


def main() -> int:
    repo_root = find_repo_root()
    errors = validate_docs(repo_root)
    if errors:
        print("Documentation validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print("Documentation validation passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
