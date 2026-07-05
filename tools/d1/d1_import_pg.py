#!/usr/bin/env python3
"""Import selection.pk_* tables from d1_export.db into PG.

Environment variables:
  DATABASE_URL — PostgreSQL connection string

This reads d1_export.db (produced by d1_export.py) and INSERTs each
selection.pk_* table into the target Postgres database.
"""

import os
import sqlite3
import psycopg2
import sys

DB_PATH = "d1_export.db"

PK_TABLES = [
    "calendar",  # → selection.pk_calendars
    "language",  # → selection.pk_languages
    "coursenature",  # → selection.pk_course_natures
    "coursenature_by_calendar",  # → selection.pk_course_natures_by_calendar
    "assessment",  # → selection.pk_assessments
    "campus",  # → selection.pk_campuses
    "faculty",  # → selection.pk_faculties
    "major",  # → selection.pk_majors
    "coursedetail",  # → selection.pk_course_details
    "teacher",  # → selection.pk_teachers_raw
    "teacher_timeslots",  # → selection.pk_teacher_timeslots
    "majorandcourse",  # → selection.pk_major_courses
    "fetchlog",  # → selection.pk_fetch_logs
]

TABLE_MAP = {
    "calendar": "selection.pk_calendars",
    "language": "selection.pk_languages",
    "coursenature": "selection.pk_course_natures",
    "coursenature_by_calendar": "selection.pk_course_natures_by_calendar",
    "assessment": "selection.pk_assessments",
    "campus": "selection.pk_campuses",
    "faculty": "selection.pk_faculties",
    "major": "selection.pk_majors",
    "coursedetail": "selection.pk_course_details",
    "teacher": "selection.pk_teachers_raw",
    "teacher_timeslots": "selection.pk_teacher_timeslots",
    "majorandcourse": "selection.pk_major_courses",
    "fetchlog": "selection.pk_fetch_logs",
}


def import_table(sqlite_cur, pg_cur, d1_table: str):
    pg_table = TABLE_MAP[d1_table]
    print(f"  {d1_table} → {pg_table} ...", end=" ")

    # Get column names from SQLite
    sqlite_cur.execute(f'PRAGMA table_info("{d1_table}")')
    cols = [row[1] for row in sqlite_cur.fetchall()]
    if not cols:
        print("(empty table, skipped)")
        return 0

    sqlite_cur.execute(f'SELECT * FROM "{d1_table}"')
    rows = sqlite_cur.fetchall()

    if not rows:
        print("(0 rows)")
        return 0

    col_list = ", ".join(f'"{c}"' for c in cols)
    placeholders = ", ".join("%s" for _ in cols)
    sql = f"INSERT INTO {pg_table} ({col_list}) VALUES ({placeholders}) ON CONFLICT DO NOTHING"

    count = 0
    for row in rows:
        try:
            pg_cur.execute(sql, row)
            count += 1
        except Exception as e:
            print(f"\n    WARNING: row skipped: {e}")

    print(f"({count} rows)")
    return count


def main():
    if not os.path.exists(DB_PATH):
        print(f"ERROR: {DB_PATH} not found. Run d1_export.py first.", file=sys.stderr)
        sys.exit(1)

    db_url = os.environ.get("DATABASE_URL")
    if not db_url:
        print("ERROR: DATABASE_URL not set", file=sys.stderr)
        sys.exit(1)

    print(f"Reading {DB_PATH} ...")
    db = sqlite3.connect(DB_PATH)
    db.row_factory = sqlite3.Row
    cur = db.cursor()

    print(f"Connecting to PG ({db_url.split('@')[1].split('/')[0]}) ...")
    conn = psycopg2.connect(db_url)
    pg = conn.cursor()

    total = 0
    for table in PK_TABLES:
        try:
            count = import_table(cur, pg, table)
            total += count
        except Exception as e:
            print(f"\n  ERROR: {e}")
            conn.rollback()
            db.close()
            sys.exit(1)

    conn.commit()
    pg.close()
    conn.close()
    db.close()
    print(f"\nDone. Imported {total} rows across {len(PK_TABLES)} tables.")


if __name__ == "__main__":
    main()
