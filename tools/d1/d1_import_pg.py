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
import io
import os
import sqlite3
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import TextIO


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


def emit_copy_stream(database: sqlite3.Connection, output: TextIO) -> int:
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
    print("COMMIT;", file=output)
    return total


def import_direct(database: sqlite3.Connection, database_url: str) -> int:
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
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.source.is_file():
        print(f"ERROR: snapshot not found: {args.source}", file=sys.stderr)
        return 1
    database = sqlite3.connect(f"file:{args.source}?mode=ro", uri=True)
    try:
        validate_source(database)
        if args.emit_copy:
            total = emit_copy_stream(database, sys.stdout)
        else:
            database_url = os.environ.get("DATABASE_URL")
            if not database_url:
                print("ERROR: DATABASE_URL is not set", file=sys.stderr)
                return 1
            total = import_direct(database, database_url)
    except Exception as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    finally:
        database.close()
    print(f"Imported {total} rows across {len(TABLES)} tables", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
