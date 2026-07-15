#!/usr/bin/env python3
"""Export the jcourse-db-backup D1 database to a fresh local SQLite file.

Environment variables:
  CLOUDFLARE_ACCOUNT_ID   — Cloudflare account ID
  CLOUDFLARE_D1_DATABASE_ID — D1 database UUID
  CLOUDFLARE_API_TOKEN    — API token with d1:read permission

The configured database id is resolved through the Cloudflare API and must be
named jcourse-db-backup. Existing output files are never overwritten.
"""

import argparse
import os
import sqlite3
import sys
import tempfile
from pathlib import Path

import requests

BACKUP_DATABASE_NAME = "jcourse-db-backup"
REQUEST_TIMEOUT_SECONDS = 60

# Cloudflare internal tables to skip
SKIP_TABLES = {"_cf_KV"}


def cloudflare_config() -> tuple[str, dict[str, str]]:
    """Build the D1 API endpoint without resolving secrets during --help."""
    required = (
        "CLOUDFLARE_ACCOUNT_ID",
        "CLOUDFLARE_D1_DATABASE_ID",
        "CLOUDFLARE_API_TOKEN",
    )
    missing = [name for name in required if not os.environ.get(name)]
    if missing:
        raise RuntimeError(f"missing required environment variables: {', '.join(missing)}")
    url = (
        "https://api.cloudflare.com/client/v4/accounts"
        f"/{os.environ['CLOUDFLARE_ACCOUNT_ID']}"
        f"/d1/database/{os.environ['CLOUDFLARE_D1_DATABASE_ID']}"
    )
    headers = {
        "Authorization": f"Bearer {os.environ['CLOUDFLARE_API_TOKEN']}",
        "Content-Type": "application/json",
    }
    return url, headers


def checked_response(response: requests.Response) -> dict:
    if not response.ok:
        raise RuntimeError(f"Cloudflare D1 API returned HTTP {response.status_code}")
    data = response.json()
    if not data.get("success"):
        raise RuntimeError(f"D1 error: {data.get('errors')}")
    return data


def configured_database_name() -> str:
    """Resolve the configured id and fail closed unless it is the backup."""
    url, headers = cloudflare_config()
    try:
        response = requests.get(url, headers=headers, timeout=REQUEST_TIMEOUT_SECONDS)
    except requests.RequestException as error:
        raise RuntimeError("Cloudflare D1 API request failed") from error
    data = checked_response(response)
    result = data.get("result") or {}
    return str(result.get("name") or "")


def d1_query(sql: str, params: list | None = None) -> list[dict]:
    """Run a D1 query and return the result rows."""
    url, headers = cloudflare_config()
    body: dict = {"sql": sql}
    if params:
        body["params"] = params
    try:
        response = requests.post(
            f"{url}/query",
            headers=headers,
            json=body,
            timeout=REQUEST_TIMEOUT_SECONDS,
        )
    except requests.RequestException as error:
        raise RuntimeError("Cloudflare D1 query failed") from error
    data = checked_response(response)
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


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("d1_export.db"),
        help="fresh SQLite destination; an existing path is rejected",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    output = args.output
    if output.exists():
        raise RuntimeError(f"refusing to overwrite existing snapshot: {output}")
    if not output.parent.is_dir():
        raise RuntimeError(f"output directory does not exist: {output.parent}")

    database_name = configured_database_name()
    if database_name != BACKUP_DATABASE_NAME:
        raise RuntimeError(
            "refusing export: configured D1 database is "
            f"{database_name or '<unknown>'!r}, expected {BACKUP_DATABASE_NAME!r}"
        )

    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{output.name}.",
        suffix=".tmp",
        dir=output.parent,
    )
    os.close(descriptor)
    temporary = Path(temporary_name)
    db: sqlite3.Connection | None = None
    try:
        print(f"Exporting {database_name} to {output} ...")
        db = sqlite3.connect(temporary)
        cur = db.cursor()
        tables = list_tables()
        print(f"Found {len(tables)} tables: {', '.join(tables)}")

        total = 0
        for table in tables:
            count = export_table(cur, table)
            total += count
            print(f"  {table}: {count} rows")

        db.commit()
        db.close()
        db = None
        os.chmod(temporary, 0o600)
        os.replace(temporary, output)
    except Exception:
        if db is not None:
            db.rollback()
            db.close()
        temporary.unlink(missing_ok=True)
        raise
    print(f"Done. {total} rows across {len(tables)} tables.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"ERROR: {error}", file=sys.stderr)
        raise SystemExit(1) from None
