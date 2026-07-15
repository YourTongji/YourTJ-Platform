#!/usr/bin/env python3
"""Import a D1 SQLite snapshot into empty ``selection.pk_*`` tables.

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


@dataclass(frozen=True)
class TableSpec:
    source: str
    target: str
    columns: tuple[tuple[str, str], ...]


TABLES = (
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

NULL_MARKER = "__YOURTJ_D1_NULL_20260711__"

NUMERIC_TARGET_COLUMNS = {
    "calendar_id",
    "course_label_id",
    "credit",
    "elc_number",
    "end_week",
    "fetch_time",
    "grade",
    "id",
    "major_id",
    "number",
    "occupy_day",
    "occupy_section",
    "period",
    "course_id",
    "start_week",
    "teaching_class_id",
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
        "schemaVersion": 1,
        "sourceDatabase": args.source_database,
        "snapshotSha256": sha256_file(args.source),
        "snapshotBytes": args.source.stat().st_size,
        "snapshotExportedAt": args.snapshot_exported_at,
        "sourceTableCounts": counts,
    }


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
        os.replace(temporary, path)
        os.chmod(path, 0o600)
    except Exception:
        if descriptor >= 0:
            os.close(descriptor)
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass
        raise


def report_manifest_diff(path: Path, manifest: dict[str, Any]) -> None:
    previous = json.loads(path.read_text(encoding="utf-8"))
    previous_counts = previous.get("sourceTableCounts")
    if not isinstance(previous_counts, dict):
        raise ValueError("comparison manifest is missing sourceTableCounts")
    current_counts = manifest["sourceTableCounts"]
    print("Snapshot row-count delta:", file=sys.stderr)
    for spec in TABLES:
        before = int(previous_counts.get(spec.source, 0))
        after = int(current_counts[spec.source])
        print(f"  {spec.source}: {after - before:+d} ({before} -> {after})", file=sys.stderr)


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


def import_run_insert(manifest: dict[str, Any], imported_by: str) -> str:
    exported_at = manifest["snapshotExportedAt"]
    exported_sql = (
        "NULL"
        if exported_at is None
        else f"{sql_literal(exported_at)}::timestamptz"
    )
    counts = json.dumps(manifest["sourceTableCounts"], separators=(",", ":"), sort_keys=True)
    validation = json.dumps(
        {"rowCountsMatched": True, "sourceSchemaValidated": True},
        separators=(",", ":"),
        sort_keys=True,
    )
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
        "    RAISE EXCEPTION 'selection raw tables must be empty before D1 import';\n"
        "  END IF;\nEND\n$guard$;"
    )


def emit_copy_stream(
    database: sqlite3.Connection,
    output: TextIO,
    manifest: dict[str, Any],
    imported_by: str,
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
    print(import_run_insert(manifest, imported_by), file=output)
    print("COMMIT;", file=output)
    return total


def import_direct(
    database: sqlite3.Connection,
    database_url: str,
    manifest: dict[str, Any],
    imported_by: str,
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
                    json.dumps(manifest["sourceTableCounts"], sort_keys=True),
                    json.dumps(target_counts, sort_keys=True),
                    json.dumps(
                        {"rowCountsMatched": True, "sourceSchemaValidated": True},
                        sort_keys=True,
                    ),
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
        default=os.environ.get("USER", "unknown"),
        help="operator label stored with the import audit record",
    )
    parser.add_argument(
        "--manifest-out",
        type=Path,
        help="atomically write the pre-import source manifest as JSON",
    )
    parser.add_argument(
        "--compare-manifest",
        type=Path,
        help="print per-table row-count deltas against an earlier manifest",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.source.is_file():
        print(f"ERROR: snapshot not found: {args.source}", file=sys.stderr)
        return 1
    database = sqlite3.connect(f"file:{args.source}?mode=ro", uri=True)
    try:
        validate_source(database)
        counts = source_counts(database)
        manifest = build_manifest(args, counts)
        if args.manifest_out:
            write_manifest(args.manifest_out, manifest)
        if args.compare_manifest:
            report_manifest_diff(args.compare_manifest, manifest)
        if args.emit_copy:
            total = emit_copy_stream(database, sys.stdout, manifest, args.imported_by)
        else:
            database_url = os.environ.get("DATABASE_URL")
            if not database_url:
                print("ERROR: DATABASE_URL is not set", file=sys.stderr)
                return 1
            total = import_direct(database, database_url, manifest, args.imported_by)
    except Exception as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    finally:
        database.close()
    print(f"Imported {total} rows across {len(TABLES)} tables", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
