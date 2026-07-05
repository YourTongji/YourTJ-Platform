#!/usr/bin/env python3
"""Generate SQL INSERTs for reviews.* from d1_export.db.

Output: SQL to stdout (pipe into psql).

Environment variables:
  DATABASE_URL — not used directly; output goes to stdout

Writes INSERT statements using OVERRIDING SYSTEM VALUE
for reviews.reviews (which uses GENERATED ALWAYS AS IDENTITY).
"""

import os
import sqlite3
import sys

DB_PATH = "d1_export.db"


def sql_literal(val) -> str:
    """Convert a Python value to a SQL literal string."""
    if val is None:
        return "NULL"
    if isinstance(val, (int, float)):
        return str(val)
    escaped = str(val).replace("'", "''")
    return f"'{escaped}'"


def gen_reviews(cur):
    """Generate INSERTs for reviews.reviews."""
    cur.execute("SELECT * FROM reviews")
    cols = [d[0] for d in cur.description]
    rows = cur.fetchall()
    print(f"\n-- reviews.reviews: {len(rows)} rows", file=sys.stderr)

    if not rows:
        return

    col_list = ", ".join(cols)
    print(f"INSERT INTO reviews.reviews ({col_list})")
    print("OVERRIDING SYSTEM VALUE")
    print("VALUES")

    for i, row in enumerate(rows):
        vals = ", ".join(sql_literal(v) for v in row)
        sep = "," if i < len(rows) - 1 else ";"
        print(f"  ({vals}){sep}")


def gen_review_likes(cur):
    """Generate INSERTs for reviews.review_likes."""
    cur.execute("SELECT * FROM review_likes")
    cols = [d[0] for d in cur.description]
    rows = cur.fetchall()
    print(f"\n-- reviews.review_likes: {len(rows)} rows", file=sys.stderr)


def gen_review_reports(cur):
    """Generate INSERTs for reviews.review_reports."""
    cur.execute("SELECT * FROM review_reports")
    cols = [d[0] for d in cur.description]
    rows = cur.fetchall()
    print(f"\n-- reviews.review_reports: {len(rows)} rows", file=sys.stderr)


def main():
    if not os.path.exists(DB_PATH):
        print(f"ERROR: {DB_PATH} not found.", file=sys.stderr)
        sys.exit(1)

    db = sqlite3.connect(DB_PATH)
    db.row_factory = sqlite3.Row
    cur = db.cursor()

    print("BEGIN;")
    gen_reviews(cur)
    gen_review_likes(cur)
    gen_review_reports(cur)
    print("COMMIT;")

    db.close()


if __name__ == "__main__":
    main()
