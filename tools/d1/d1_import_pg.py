#!/usr/bin/env python3
"""Import a D1 SQLite snapshot into empty Selection and Courses raw tables.

The D1 schema uses camelCase while PostgreSQL uses snake_case. This tool owns
that explicit mapping and fails before writing when a source column is missing
or any target raw table already contains data.

By default it connects through ``DATABASE_URL`` with psycopg2. ``--emit-copy``
instead writes one atomic psql COPY stream to stdout, which is useful when the
database is reachable only through an operational shell.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import io
import json
import os
import sqlite3
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, TextIO

BACKUP_DATABASE_NAME = "jcourse-db-backup"
MANIFEST_SCHEMA_VERSION = 1
APPROVAL_REASON_MIN_LENGTH = 10
APPROVAL_REASON_MAX_LENGTH = 500
OPERATOR_LABEL_MIN_LENGTH = 3
OPERATOR_LABEL_MAX_LENGTH = 64
ESSENTIAL_TABLES = (
    "calendar",
    "coursenature",
    "coursenature_by_calendar",
    "campus",
    "faculty",
    "major",
    "coursedetail",
    "teacher",
    "teacher_timeslots",
    "majorandcourse",
    "fetchlog",
)
LEGACY_COURSE_TABLES = ("teachers", "courses", "course_aliases")


@dataclass(frozen=True)
class TableSpec:
    source: str
    target: str
    columns: tuple[tuple[str, str], ...]


SELECTION_TABLES = (
    TableSpec(
        "calendar",
        "selection.pk_calendars",
        (("calendarId", "calendar_id"), ("calendarIdI18n", "calendar_name")),
    ),
    TableSpec(
        "language",
        "selection.pk_languages",
        (
            ("teachingLanguage", "teaching_language"),
            ("teachingLanguageI18n", "teaching_language_i18n"),
            ("calendarId", "calendar_id"),
        ),
    ),
    TableSpec(
        "coursenature",
        "selection.pk_course_natures",
        (
            ("courseLabelId", "course_label_id"),
            ("courseLabelName", "course_label_name"),
            ("calendarId", "calendar_id"),
        ),
    ),
    TableSpec(
        "coursenature_by_calendar",
        "selection.pk_course_natures_by_calendar",
        (
            ("calendarId", "calendar_id"),
            ("courseLabelId", "course_label_id"),
            ("courseLabelName", "course_label_name"),
        ),
    ),
    TableSpec(
        "assessment",
        "selection.pk_assessments",
        (
            ("assessmentMode", "assessment_mode"),
            ("assessmentModeI18n", "assessment_mode_i18n"),
            ("calendarId", "calendar_id"),
        ),
    ),
    TableSpec(
        "campus",
        "selection.pk_campuses",
        (
            ("campus", "campus"),
            ("campusI18n", "campus_i18n"),
            ("calendarId", "calendar_id"),
        ),
    ),
    TableSpec(
        "faculty",
        "selection.pk_faculties",
        (
            ("faculty", "faculty"),
            ("facultyI18n", "faculty_i18n"),
            ("calendarId", "calendar_id"),
        ),
    ),
    TableSpec(
        "major",
        "selection.pk_majors",
        (
            ("id", "id"),
            ("code", "code"),
            ("grade", "grade"),
            ("name", "name"),
            ("calendarId", "calendar_id"),
        ),
    ),
    TableSpec(
        "coursedetail",
        "selection.pk_course_details",
        (
            ("id", "id"),
            ("code", "code"),
            ("name", "name"),
            ("courseLabelId", "course_label_id"),
            ("assessmentMode", "assessment_mode"),
            ("period", "period"),
            ("weekHour", "week_hour"),
            ("campus", "campus"),
            ("number", "number"),
            ("elcNumber", "elc_number"),
            ("startWeek", "start_week"),
            ("endWeek", "end_week"),
            ("courseCode", "course_code"),
            ("courseName", "course_name"),
            ("credit", "credit"),
            ("teachingLanguage", "teaching_language"),
            ("faculty", "faculty"),
            ("calendarId", "calendar_id"),
            ("newCourseCode", "new_course_code"),
            ("newCode", "new_code"),
        ),
    ),
    TableSpec(
        "teacher",
        "selection.pk_teachers_raw",
        (
            ("id", "id"),
            ("teachingClassId", "teaching_class_id"),
            ("teacherCode", "teacher_code"),
            ("teacherName", "teacher_name"),
            ("arrangeInfoText", "arrange_info_text"),
        ),
    ),
    TableSpec(
        "teacher_timeslots",
        "selection.pk_teacher_timeslots",
        (
            ("calendar_id", "calendar_id"),
            ("teaching_class_id", "teaching_class_id"),
            ("occupy_day", "occupy_day"),
            ("occupy_section", "occupy_section"),
            ("teacher_code", "teacher_code"),
            ("teacher_name", "teacher_name"),
        ),
    ),
    TableSpec(
        "majorandcourse",
        "selection.pk_major_courses",
        (("majorId", "major_id"), ("courseId", "course_id")),
    ),
    TableSpec(
        "fetchlog",
        "selection.pk_fetch_logs",
        (("fetchTime", "fetch_time"), ("msg", "msg")),
    ),
)

LEGACY_COURSE_SPECS = (
    TableSpec(
        "teachers",
        "courses.pk_legacy_teachers",
        (
            ("id", "id"),
            ("tid", "tid"),
            ("name", "name"),
            ("title", "title"),
            ("department", "department"),
        ),
    ),
    TableSpec(
        "courses",
        "courses.pk_legacy_courses",
        (
            ("id", "id"),
            ("code", "code"),
            ("name", "name"),
            ("credit", "credit"),
            ("department", "department"),
            ("teacher_id", "teacher_id"),
            ("review_count", "review_count"),
            ("review_avg", "review_avg"),
            ("search_keywords", "search_keywords"),
            ("is_legacy", "is_legacy"),
            ("is_icu", "is_icu"),
        ),
    ),
    TableSpec(
        "course_aliases",
        "courses.pk_legacy_course_aliases",
        (
            ("system", "system"),
            ("alias", "alias"),
            ("course_id", "course_id"),
            ("created_at", "created_at"),
        ),
    ),
)

TABLES = SELECTION_TABLES + LEGACY_COURSE_SPECS

NULL_MARKER = "__YOURTJ_D1_NULL_20260711__"

NUMERIC_TARGET_COLUMNS = {
    "calendar_id",
    "created_at",
    "course_label_id",
    "credit",
    "elc_number",
    "end_week",
    "fetch_time",
    "grade",
    "id",
    "is_icu",
    "is_legacy",
    "major_id",
    "number",
    "occupy_day",
    "occupy_section",
    "period",
    "review_avg",
    "review_count",
    "course_id",
    "start_week",
    "teaching_class_id",
    "teacher_id",
    "week_hour",
}


def quote_identifier(identifier: str) -> str:
    return f'"{identifier.replace(chr(34), chr(34) * 2)}"'


def source_query(spec: TableSpec) -> str:
    columns = ", ".join(quote_identifier(source) for source, _ in spec.columns)
    return f"SELECT {columns} FROM {quote_identifier(spec.source)}"


def copy_statement(spec: TableSpec) -> str:
    columns = ", ".join(quote_identifier(target) for _, target in spec.columns)
    return (
        f"COPY {spec.target} ({columns}) FROM STDIN WITH "
        f"(FORMAT CSV, NULL '{NULL_MARKER}', FORCE_NULL ({columns}))"
    )


def validate_source(database: sqlite3.Connection) -> None:
    for spec in TABLES:
        available = {
            row[1]
            for row in database.execute(
                f"PRAGMA table_info({quote_identifier(spec.source)})"
            )
        }
        missing = [source for source, _ in spec.columns if source not in available]
        if missing:
            raise ValueError(f"{spec.source} is missing columns: {', '.join(missing)}")


def source_counts(database: sqlite3.Connection) -> dict[str, int]:
    return {
        spec.source: int(
            database.execute(
                f"SELECT COUNT(*) FROM {quote_identifier(spec.source)}"
            ).fetchone()[0]
        )
        for spec in TABLES
    }


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def build_manifest(args: argparse.Namespace, counts: dict[str, int]) -> dict[str, Any]:
    return {
        "schemaVersion": MANIFEST_SCHEMA_VERSION,
        "sourceDatabase": args.source_database,
        "snapshotSha256": sha256_file(args.source),
        "snapshotBytes": args.source.stat().st_size,
        "snapshotExportedAt": args.snapshot_exported_at,
        "sourceTableCounts": counts,
    }


def paths_alias(left: Path, right: Path) -> bool:
    if left.exists() and right.exists() and os.path.samefile(left, right):
        return True
    return left.resolve(strict=False) == right.resolve(strict=False)


def validate_manifest_paths(
    source: Path,
    manifest_out: Path | None,
    compare_manifest: Path | None,
) -> None:
    paths = [
        ("source", source),
        ("manifest output", manifest_out),
        ("comparison manifest", compare_manifest),
    ]
    configured = [(label, path) for label, path in paths if path is not None]
    for index, (left_label, left_path) in enumerate(configured):
        for right_label, right_path in configured[index + 1 :]:
            if paths_alias(left_path, right_path):
                raise ValueError(f"{left_label} and {right_label} must be different files")


def write_manifest(path: Path, manifest: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
    )
    temporary = Path(temporary_name)
    try:
        os.fchmod(descriptor, 0o600)
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as output:
            descriptor = -1
            json.dump(manifest, output, ensure_ascii=False, indent=2, sort_keys=True)
            output.write("\n")
            output.flush()
            os.fsync(output.fileno())
        try:
            os.link(temporary, path)
        except FileExistsError as error:
            raise FileExistsError(f"manifest output already exists: {path}") from error
        temporary.unlink()
    except Exception:
        if descriptor >= 0:
            os.close(descriptor)
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass
        raise


def validated_table_counts(value: object, surface: str) -> dict[str, int]:
    expected = {spec.source for spec in TABLES}
    if not isinstance(value, dict) or set(value) != expected:
        raise ValueError(
            f"{surface} must contain exactly the {len(TABLES)} supported table counts"
        )
    counts: dict[str, int] = {}
    for table, count in value.items():
        if isinstance(count, bool) or not isinstance(count, int) or count < 0:
            raise ValueError(f"{surface} contains an invalid count for {table}")
        counts[table] = count
    return counts


def load_comparison_manifest(
    path: Path, current_manifest: dict[str, Any]
) -> dict[str, Any]:
    try:
        previous = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read comparison manifest: {error}") from error
    if not isinstance(previous, dict):
        raise ValueError("comparison manifest must be a JSON object")
    if previous.get("schemaVersion") != MANIFEST_SCHEMA_VERSION:
        raise ValueError("comparison manifest has an unsupported schemaVersion")
    if previous.get("sourceDatabase") != current_manifest["sourceDatabase"]:
        raise ValueError("comparison manifest sourceDatabase does not match")
    snapshot_sha256 = previous.get("snapshotSha256")
    if (
        not isinstance(snapshot_sha256, str)
        or len(snapshot_sha256) != 64
        or any(character not in "0123456789abcdef" for character in snapshot_sha256)
    ):
        raise ValueError("comparison manifest has an invalid snapshotSha256")
    previous["sourceTableCounts"] = validated_table_counts(
        previous.get("sourceTableCounts"), "comparison manifest sourceTableCounts"
    )
    return previous


def bounded_approval_reason(args: argparse.Namespace) -> str:
    reason = (args.approval_reason or "").strip()
    if not APPROVAL_REASON_MIN_LENGTH <= len(reason) <= APPROVAL_REASON_MAX_LENGTH:
        raise ValueError(
            "approval reason must contain between "
            f"{APPROVAL_REASON_MIN_LENGTH} and {APPROVAL_REASON_MAX_LENGTH} characters"
        )
    return reason


def validated_operator_label(value: str) -> str:
    label = value.strip()
    allowed = set("abcdefghijklmnopqrstuvwxyz0123456789._:/-")
    if (
        not OPERATOR_LABEL_MIN_LENGTH <= len(label) <= OPERATOR_LABEL_MAX_LENGTH
        or label[0] not in "abcdefghijklmnopqrstuvwxyz0123456789"
        or any(character not in allowed for character in label)
    ):
        raise ValueError(
            "imported-by must be a 3-64 character lowercase role/service label"
        )
    return label


def validate_snapshot_completeness(
    args: argparse.Namespace,
    manifest: dict[str, Any],
    comparison_manifest: dict[str, Any] | None,
) -> dict[str, Any]:
    current_counts = validated_table_counts(
        manifest.get("sourceTableCounts"), "current sourceTableCounts"
    )
    empty_essential = [table for table in ESSENTIAL_TABLES if current_counts[table] == 0]
    empty_legacy = [table for table in LEGACY_COURSE_TABLES if current_counts[table] == 0]
    if empty_essential or empty_legacy:
        raise ValueError(
            "essential source tables must be non-empty: "
            + ", ".join(empty_essential + empty_legacy)
        )

    approves_unbaselined = bool(args.approve_unbaselined_snapshot)
    approves_decrease = bool(args.approve_count_decrease)
    if comparison_manifest is None:
        if not approves_unbaselined:
            raise ValueError(
                "a comparison manifest is required unless "
                "--approve-unbaselined-snapshot is explicitly supplied"
            )
        if approves_decrease:
            raise ValueError("count-decrease approval requires a comparison manifest")
        return {
            "completenessApproved": True,
            "approvalMode": "unbaselined",
            "approvalReason": bounded_approval_reason(args),
            "approvedCoreCounts": {
                table: current_counts[table] for table in ESSENTIAL_TABLES
            },
            "legacyCourseApprovalMode": "unbaselined",
            "approvedLegacyCourseCounts": {
                table: current_counts[table] for table in LEGACY_COURSE_TABLES
            },
        }

    if approves_unbaselined:
        raise ValueError(
            "--approve-unbaselined-snapshot cannot be combined with --compare-manifest"
        )
    previous_counts = comparison_manifest["sourceTableCounts"]
    decreases = {
        table: {"before": previous_counts[table], "after": current_counts[table]}
        for table in ESSENTIAL_TABLES
        if current_counts[table] < previous_counts[table]
    }
    legacy_decreases = {
        table: {"before": previous_counts[table], "after": current_counts[table]}
        for table in LEGACY_COURSE_TABLES
        if current_counts[table] < previous_counts[table]
    }
    all_decreases = {**decreases, **legacy_decreases}
    if all_decreases and not approves_decrease:
        raise ValueError(
            "essential table counts decreased; inspect the diff and explicitly approve: "
            + ", ".join(all_decreases)
        )
    if approves_decrease and not all_decreases:
        raise ValueError("--approve-count-decrease was supplied but no essential count decreased")
    if args.approval_reason and not approves_decrease:
        raise ValueError("--approval-reason is only valid for an explicit approval mode")

    validation: dict[str, Any] = {
        "completenessApproved": True,
        "approvalMode": "countDecreaseOverride" if decreases else "baselineCompared",
        "baselineSnapshotSha256": comparison_manifest["snapshotSha256"],
        "baselineCoreCounts": {
            table: previous_counts[table] for table in ESSENTIAL_TABLES
        },
        "legacyCourseApprovalMode": (
            "countDecreaseOverride" if legacy_decreases else "baselineCompared"
        ),
        "baselineLegacyCourseCounts": {
            table: previous_counts[table] for table in LEGACY_COURSE_TABLES
        },
    }
    if all_decreases:
        validation["approvalReason"] = bounded_approval_reason(args)
    if decreases:
        validation["countDecreases"] = decreases
    if legacy_decreases:
        validation["legacyCourseCountDecreases"] = legacy_decreases
    return validation


def report_manifest_diff(
    previous: dict[str, Any], manifest: dict[str, Any]
) -> None:
    previous_counts = previous["sourceTableCounts"]
    current_counts = manifest["sourceTableCounts"]
    print("Snapshot row-count delta:", file=sys.stderr)
    for spec in TABLES:
        before = previous_counts[spec.source]
        after = int(current_counts[spec.source])
        print(f"  {spec.source}: {after - before:+d} ({before} -> {after})", file=sys.stderr)


def process_manifest_files(
    args: argparse.Namespace,
    manifest: dict[str, Any],
    comparison_manifest: dict[str, Any] | None,
) -> None:
    if comparison_manifest is not None:
        report_manifest_diff(comparison_manifest, manifest)
    if args.manifest_out:
        write_manifest(args.manifest_out, manifest)


def sql_literal(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"


def target_count_guard(counts: dict[str, int]) -> str:
    checks = []
    for spec in TABLES:
        expected = counts[spec.source]
        checks.append(
            f"  IF (SELECT COUNT(*) FROM {spec.target}) <> {expected} THEN\n"
            f"    RAISE EXCEPTION 'row-count mismatch for {spec.target}';\n"
            "  END IF;"
        )
    return "DO $validate$\nBEGIN\n" + "\n".join(checks) + "\nEND\n$validate$;"


def counts_for_specs(
    counts: dict[str, int], specs: tuple[TableSpec, ...]
) -> dict[str, int]:
    return {spec.source: counts[spec.source] for spec in specs}


def import_run_insert(
    manifest: dict[str, Any], imported_by: str, validation_result: dict[str, Any]
) -> str:
    exported_at = manifest["snapshotExportedAt"]
    exported_sql = (
        "NULL"
        if exported_at is None
        else f"{sql_literal(exported_at)}::timestamptz"
    )
    selection_counts = counts_for_specs(
        manifest["sourceTableCounts"], SELECTION_TABLES
    )
    counts = json.dumps(selection_counts, separators=(",", ":"), sort_keys=True)
    validation = json.dumps(validation_result, separators=(",", ":"), sort_keys=True)
    return (
        "INSERT INTO selection.import_runs ("
        "snapshot_sha256, snapshot_bytes, source_database, snapshot_exported_at, "
        "imported_by, source_table_counts, target_table_counts, validation"
        ") VALUES ("
        f"{sql_literal(manifest['snapshotSha256'])}, {manifest['snapshotBytes']}, "
        f"{sql_literal(manifest['sourceDatabase'])}, {exported_sql}, "
        f"{sql_literal(imported_by)}, {sql_literal(counts)}::jsonb, "
        f"{sql_literal(counts)}::jsonb, {sql_literal(validation)}::jsonb"
        ");"
    )


def legacy_import_run_insert(
    manifest: dict[str, Any], validation_result: dict[str, Any]
) -> str:
    legacy_counts = counts_for_specs(
        manifest["sourceTableCounts"], LEGACY_COURSE_SPECS
    )
    counts = json.dumps(legacy_counts, separators=(",", ":"), sort_keys=True)
    validation = json.dumps(validation_result, separators=(",", ":"), sort_keys=True)
    return (
        "INSERT INTO courses.legacy_import_runs ("
        "snapshot_sha256, source_database, source_table_counts, "
        "target_table_counts, validation"
        ") VALUES ("
        f"{sql_literal(manifest['snapshotSha256'])}, "
        f"{sql_literal(manifest['sourceDatabase'])}, "
        f"{sql_literal(counts)}::jsonb, {sql_literal(counts)}::jsonb, "
        f"{sql_literal(validation)}::jsonb"
        ");"
    )


def write_rows(database: sqlite3.Connection, spec: TableSpec, output: TextIO) -> int:
    writer = csv.writer(output, lineterminator="\n", quoting=csv.QUOTE_NONNUMERIC)
    count = 0
    for row in database.execute(source_query(spec)):
        normalized = []
        for value, (_, target) in zip(row, spec.columns, strict=True):
            if value == NULL_MARKER:
                raise ValueError(f"{spec.source} contains the reserved null marker")
            if value is None or (
                target in NUMERIC_TARGET_COLUMNS
                and isinstance(value, str)
                and not value.strip()
            ):
                normalized.append(NULL_MARKER)
            else:
                normalized.append(value)
        writer.writerow(normalized)
        count += 1
    return count


def target_empty_guard() -> str:
    checks = "\n      OR ".join(
        f"EXISTS (SELECT 1 FROM {spec.target})" for spec in TABLES
    )
    return (
        "DO $guard$\nBEGIN\n"
        f"  IF {checks} THEN\n"
        "    RAISE EXCEPTION 'target raw tables must be empty before D1 import';\n"
        "  END IF;\nEND\n$guard$;"
    )


def emit_copy_stream(
    database: sqlite3.Connection,
    output: TextIO,
    manifest: dict[str, Any],
    imported_by: str,
    validation_result: dict[str, Any],
) -> int:
    targets = ", ".join(spec.target for spec in TABLES)
    print(r"\set ON_ERROR_STOP on", file=output)
    print("BEGIN;", file=output)
    print(f"LOCK TABLE {targets} IN SHARE ROW EXCLUSIVE MODE;", file=output)
    print(target_empty_guard(), file=output)
    total = 0
    for spec in TABLES:
        print(f"{copy_statement(spec)};", file=output)
        count = write_rows(database, spec, output)
        print(r"\.", file=output)
        print(f"{spec.source}: {count} rows", file=sys.stderr)
        total += count
    print(target_count_guard(manifest["sourceTableCounts"]), file=output)
    print(import_run_insert(manifest, imported_by, validation_result), file=output)
    print(legacy_import_run_insert(manifest, validation_result), file=output)
    print("COMMIT;", file=output)
    return total


def import_direct(
    database: sqlite3.Connection,
    database_url: str,
    manifest: dict[str, Any],
    imported_by: str,
    validation_result: dict[str, Any],
) -> int:
    try:
        import psycopg2
    except ImportError as error:
        raise RuntimeError("psycopg2 is required unless --emit-copy is used") from error

    connection = psycopg2.connect(database_url)
    try:
        with connection.cursor() as cursor:
            targets = ", ".join(spec.target for spec in TABLES)
            cursor.execute(f"LOCK TABLE {targets} IN SHARE ROW EXCLUSIVE MODE")
            cursor.execute(target_empty_guard())
            total = 0
            for spec in TABLES:
                buffer = io.StringIO()
                count = write_rows(database, spec, buffer)
                buffer.seek(0)
                cursor.copy_expert(copy_statement(spec), buffer)
                print(f"{spec.source}: {count} rows")
                total += count
            target_counts: dict[str, int] = {}
            for spec in TABLES:
                cursor.execute(f"SELECT COUNT(*) FROM {spec.target}")
                target_counts[spec.source] = int(cursor.fetchone()[0])
            if target_counts != manifest["sourceTableCounts"]:
                raise ValueError("PostgreSQL target row counts do not match the D1 snapshot")
            cursor.execute(
                "INSERT INTO selection.import_runs ("
                "snapshot_sha256, snapshot_bytes, source_database, snapshot_exported_at, "
                "imported_by, source_table_counts, target_table_counts, validation"
                ") VALUES (%s, %s, %s, %s::timestamptz, %s, %s::jsonb, %s::jsonb, %s::jsonb)",
                (
                    manifest["snapshotSha256"],
                    manifest["snapshotBytes"],
                    manifest["sourceDatabase"],
                    manifest["snapshotExportedAt"],
                    imported_by,
                    json.dumps(
                        counts_for_specs(
                            manifest["sourceTableCounts"], SELECTION_TABLES
                        ),
                        sort_keys=True,
                    ),
                    json.dumps(
                        counts_for_specs(target_counts, SELECTION_TABLES),
                        sort_keys=True,
                    ),
                    json.dumps(validation_result, sort_keys=True),
                ),
            )
            cursor.execute(
                "INSERT INTO courses.legacy_import_runs ("
                "snapshot_sha256, source_database, source_table_counts, "
                "target_table_counts, validation"
                ") VALUES (%s, %s, %s::jsonb, %s::jsonb, %s::jsonb)",
                (
                    manifest["snapshotSha256"],
                    manifest["sourceDatabase"],
                    json.dumps(
                        counts_for_specs(
                            manifest["sourceTableCounts"], LEGACY_COURSE_SPECS
                        ),
                        sort_keys=True,
                    ),
                    json.dumps(
                        counts_for_specs(target_counts, LEGACY_COURSE_SPECS),
                        sort_keys=True,
                    ),
                    json.dumps(validation_result, sort_keys=True),
                ),
            )
        connection.commit()
        return total
    except Exception:
        connection.rollback()
        raise
    finally:
        connection.close()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=Path("d1_export.db"))
    parser.add_argument(
        "--emit-copy",
        action="store_true",
        help="emit an atomic psql COPY stream instead of connecting with psycopg2",
    )
    parser.add_argument(
        "--source-database",
        choices=(BACKUP_DATABASE_NAME,),
        required=True,
        help="operator-attested backup database name stored with the import audit record",
    )
    parser.add_argument(
        "--snapshot-exported-at",
        help="optional RFC 3339 export timestamp stored separately from source freshness",
    )
    parser.add_argument(
        "--imported-by",
        required=True,
        help="non-PII lowercase role/service label stored with the import audit record",
    )
    parser.add_argument(
        "--manifest-out",
        type=Path,
        help="atomically write the pre-import source manifest as JSON",
    )
    parser.add_argument(
        "--compare-manifest",
        type=Path,
        help="validate and print row-count deltas against an approved earlier manifest",
    )
    parser.add_argument(
        "--approve-unbaselined-snapshot",
        action="store_true",
        help="explicitly approve the first snapshot when no comparison manifest exists",
    )
    parser.add_argument(
        "--approve-count-decrease",
        action="store_true",
        help="explicitly approve decreases in essential tables after reviewing the diff",
    )
    parser.add_argument(
        "--approval-reason",
        help="bounded operator rationale required for unbaselined or count-decrease approval",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        validate_manifest_paths(args.source, args.manifest_out, args.compare_manifest)
        imported_by = validated_operator_label(args.imported_by)
    except (OSError, ValueError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    if not args.source.is_file():
        print(f"ERROR: snapshot not found: {args.source}", file=sys.stderr)
        return 1
    database = sqlite3.connect(f"file:{args.source}?mode=ro", uri=True)
    try:
        validate_source(database)
        counts = source_counts(database)
        manifest = build_manifest(args, counts)
        comparison_manifest = (
            load_comparison_manifest(args.compare_manifest, manifest)
            if args.compare_manifest
            else None
        )
        completeness = validate_snapshot_completeness(
            args, manifest, comparison_manifest
        )
        validation_result = {
            "rowCountsMatched": True,
            "sourceSchemaValidated": True,
            **completeness,
        }
        process_manifest_files(args, manifest, comparison_manifest)
        if args.emit_copy:
            total = emit_copy_stream(
                database,
                sys.stdout,
                manifest,
                imported_by,
                validation_result,
            )
        else:
            database_url = os.environ.get("DATABASE_URL")
            if not database_url:
                print("ERROR: DATABASE_URL is not set", file=sys.stderr)
                return 1
            total = import_direct(
                database,
                database_url,
                manifest,
                imported_by,
                validation_result,
            )
    except Exception as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    finally:
        database.close()
    print(f"Imported {total} rows across {len(TABLES)} tables", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
