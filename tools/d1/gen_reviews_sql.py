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
    cur.execute(
        """
        SELECT r.id, c.code, r.rating, r.comment, r.score, r.semester,
               r.approve_count, r.disapprove_count, r.is_hidden,
               r.is_legacy, r.is_icu, r.created_at, r.reviewer_name,
               r.reviewer_avatar, r.wallet_user_hash, r.edit_token
        FROM reviews r
        JOIN courses c ON c.id = r.course_id
        ORDER BY r.id
        """
    )
    rows = cur.fetchall()
    print(f"\n-- reviews.reviews: {len(rows)} rows", file=sys.stderr)

    if not rows:
        return

    target_cols = [
        "id",
        "course_id",
        "account_id",
        "rating",
        "comment",
        "score",
        "semester",
        "approve_count",
        "disapprove_count",
        "status",
        "is_legacy",
        "is_icu",
        "created_at",
        "updated_at",
        "reviewer_name",
        "reviewer_avatar",
        "wallet_user_hash",
        "edit_token",
    ]
    print(f"INSERT INTO reviews.reviews ({', '.join(target_cols)})")
    print("OVERRIDING SYSTEM VALUE")
    print("VALUES")

    for i, row in enumerate(rows):
        created_at = row[11]
        values = [
            sql_literal(row[0]),
            f"(SELECT id FROM courses.courses WHERE code = {sql_literal(row[1])})",
            "NULL",
            sql_literal(row[2]),
            sql_literal(row[3]),
            sql_literal(row[4]),
            sql_literal(row[5]),
            sql_literal(row[6]),
            sql_literal(row[7]),
            sql_literal("hidden" if row[8] else "visible"),
            sql_literal(row[9]),
            sql_literal(row[10]),
            f"to_timestamp({sql_literal(created_at)})",
            f"to_timestamp({sql_literal(created_at)})",
            f"COALESCE({sql_literal(row[12])}, '')",
            f"COALESCE({sql_literal(row[13])}, '')",
            sql_literal(row[14]),
            sql_literal(row[15]),
        ]
        vals = ", ".join(values)
        sep = "," if i < len(rows) - 1 else ";"
        print(f"  ({vals}){sep}")


def gen_review_likes(cur):
    """Preserve legacy client identities without fabricating platform accounts."""
    cols = ["review_id", "client_id", "created_at"]
    cur.execute(f"SELECT {', '.join(cols)} FROM review_likes")
    rows = cur.fetchall()
    print(f"\n-- reviews.legacy_review_likes: {len(rows)} rows", file=sys.stderr)

    if not rows:
        return

    col_list = ", ".join(cols)
    print(f"INSERT INTO reviews.legacy_review_likes ({col_list}) VALUES")

    for i, row in enumerate(rows):
        vals = ", ".join(sql_literal(v) for v in row)
        sep = "," if i < len(rows) - 1 else ";"
        print(f"  ({vals}){sep}")


def gen_review_reports(cur):
    """Preserve legacy reports for later identity-aware moderation import."""
    cols = [
        "id",
        "review_id",
        "client_id",
        "reason",
        "status",
        "admin_note",
        "created_at",
        "updated_at",
        "resolved_at",
    ]
    cur.execute(f"SELECT {', '.join(cols)} FROM review_reports")
    rows = cur.fetchall()
    print(f"\n-- reviews.legacy_review_reports: {len(rows)} rows", file=sys.stderr)

    if not rows:
        return

    col_list = ", ".join(cols)
    print(f"INSERT INTO reviews.legacy_review_reports ({col_list}) VALUES")

    for i, row in enumerate(rows):
        vals = ", ".join(sql_literal(v) for v in row)
        sep = "," if i < len(rows) - 1 else ";"
        print(f"  ({vals}){sep}")


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
