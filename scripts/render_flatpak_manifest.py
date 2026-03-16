#!/usr/bin/env python3
"""Render the Flatpak manifest template for a specific release artifact."""

from __future__ import annotations

import argparse
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--template", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--archive-url", required=True)
    parser.add_argument("--archive-sha256", required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    content = Path(args.template).read_text(encoding="utf-8")
    content = content.replace("{{ARCHIVE_URL}}", args.archive_url)
    content = content.replace("{{ARCHIVE_SHA256}}", args.archive_sha256)
    Path(args.output).write_text(content, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
