#!/usr/bin/env python3
"""Export all tables from Cloudflare D1 to a local SQLite database.

Environment variables:
  CLOUDFLARE_ACCOUNT_ID   — Cloudflare account ID
  CLOUDFLARE_D1_DATABASE_ID — D1 database UUID
  CLOUDFLARE_API_TOKEN    — API token with d1:read permission

Output: d1_export.db in the current directory.
"""

import json
import os
import sqlite3
import sys
import requests

D1_URL = (
    "https://api.cloudflare.com/client/v4/accounts"
    f"/{os.environ['CLOUDFLARE_ACCOUNT_ID']}"
    f"/d1/database/{os.environ['CLOUDFLARE_D1_DATABASE_ID']}"
)

HEADERS = {
    "Authorization": f"Bearer {os.environ['CLOUDFLARE_API_TOKEN']}",
    "Content-Type": "application/json",
}

# Cloudflare internal tables to skip
SKIP_TABLES = {"_cf_KV"}


def d1_query(sql: str, params: list | None = None) -> list[dict]:
    """Run a D1 query and return the result rows."""
    body: dict = {"sql": sql}
    if params:
        body["params"] = params
    resp = requests.post(f"{D1_URL}/query", headers=HEADERS, json=body)
    resp.raise_for_status()
    data = resp.json()
    if not data.get("success"):
        raise RuntimeError(f"D1 error: {data.get('errors')}")
    result = data.get("result", [])
    if not result:
        return []
    return result[0].get("results", [])


def list_tables() -> list[str]:
    """Return all table names in the D1 database."""
    rows = d1_query(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
    )
    return [r["name"] for r in rows if r["name"] not in SKIP_TABLES]


def create_sqlite_table(cur, table: str, columns: list[dict]):
    """Create a matching SQLite table from D1 column metadata."""
    col_defs = []
    for col in columns:
        name = col["name"]
        dtype = col.get("type", "TEXT").upper()
        # Map common D1 types to SQLite
        if "INT" in dtype:
            sql_type = "INTEGER"
        elif dtype in ("REAL", "FLOAT", "DOUBLE"):
            sql_type = "REAL"
        else:
            sql_type = "TEXT"
        col_defs.append(f'"{name}" {sql_type}')
    col_defs_str = ", ".join(col_defs)
    cur.execute(f'CREATE TABLE IF NOT EXISTS "{table}" ({col_defs_str})')


def export_table(cur, table: str):
    """Export all rows from a D1 table into SQLite."""
    # Get column info
    info = d1_query(f"PRAGMA table_info('{table}')")
    columns = info if info else []

    create_sqlite_table(cur, table, columns)
    col_names = [c["name"] for c in columns]

    rows = d1_query(f'SELECT * FROM "{table}"')
    if not rows:
        return 0

    # Build parameterized INSERT
    placeholders = ", ".join("?" for _ in col_names)
    quoted_cols = ", ".join(f'"{c}"' for c in col_names)
    sql = f'INSERT INTO "{table}" ({quoted_cols}) VALUES ({placeholders})'

    for row in rows:
        values = tuple(row.get(c) for c in col_names)
        cur.execute(sql, values)

    return len(rows)


def main():
    db_path = "d1_export.db"
    print(f"Exporting D1 to {db_path} ...")

    db = sqlite3.connect(db_path)
    cur = db.cursor()
    cur.execute("PRAGMA journal_mode=WAL")

    tables = list_tables()
    print(f"Found {len(tables)} tables: {', '.join(tables)}")

    total = 0
    for table in tables:
        count = export_table(cur, table)
        total += count
        print(f"  {table}: {count} rows")

    db.commit()
    db.close()
    print(f"Done. {total} rows across {len(tables)} tables.")


if __name__ == "__main__":
    main()
