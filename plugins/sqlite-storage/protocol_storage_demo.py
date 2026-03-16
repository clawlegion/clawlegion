#!/usr/bin/env python3
"""SQLite storage protocol demo for Clawlegion v2.

Usage:
  python3 protocol_storage_demo.py --execute-storage-json '<json>'
"""

import json
import sqlite3
import sys
from pathlib import Path

DB_PATH = Path(__file__).resolve().parent / "storage-demo.db"


def ensure_schema(conn: sqlite3.Connection) -> None:
    conn.execute(
        """
        CREATE TABLE IF NOT EXISTS kv_store (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )
        """
    )
    conn.commit()


def run_operation(payload: dict) -> dict:
    op = payload.get("operation")
    data = payload.get("payload", {}) or {}

    with sqlite3.connect(DB_PATH) as conn:
        ensure_schema(conn)

        if op == "create":
            key = data["key"]
            value = json.dumps(data["value"], ensure_ascii=False)
            conn.execute(
                "INSERT INTO kv_store(key, value) VALUES(?, ?) "
                "ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                (key, value),
            )
            conn.commit()
            return {"ok": True, "data": {"written": True}, "error": None}

        if op == "read":
            key = data["key"]
            row = conn.execute("SELECT value FROM kv_store WHERE key = ?", (key,)).fetchone()
            return {
                "ok": True,
                "data": None if row is None else json.loads(row[0]),
                "error": None,
            }

        if op == "delete":
            key = data["key"]
            cursor = conn.execute("DELETE FROM kv_store WHERE key = ?", (key,))
            conn.commit()
            return {
                "ok": True,
                "data": {"deleted": cursor.rowcount > 0},
                "error": None,
            }

        if op == "exists":
            key = data["key"]
            row = conn.execute("SELECT 1 FROM kv_store WHERE key = ?", (key,)).fetchone()
            return {"ok": True, "data": {"exists": row is not None}, "error": None}

        if op == "query":
            prefix = data.get("prefix", "")
            rows = conn.execute(
                "SELECT key FROM kv_store WHERE key LIKE ? ORDER BY key ASC", (f"{prefix}%",)
            ).fetchall()
            return {
                "ok": True,
                "data": {"keys": [row[0] for row in rows]},
                "error": None,
            }

    return {"ok": False, "data": None, "error": f"unsupported operation: {op}"}


def main() -> int:
    if "--execute-storage-json" not in sys.argv:
        print("sqlite-storage protocol demo")
        return 0

    payload = json.loads(sys.argv[sys.argv.index("--execute-storage-json") + 1])
    response = run_operation(payload)
    print(json.dumps(response, ensure_ascii=False))
    return 0 if response.get("ok") else 1


if __name__ == "__main__":
    raise SystemExit(main())
