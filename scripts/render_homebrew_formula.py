#!/usr/bin/env python3
"""Render the Homebrew formula from a simple template."""

from __future__ import annotations

import argparse
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--template", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--macos-x86-url", required=True)
    parser.add_argument("--macos-x86-sha", required=True)
    parser.add_argument("--macos-arm-url", required=True)
    parser.add_argument("--macos-arm-sha", required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    content = Path(args.template).read_text(encoding="utf-8")
    replacements = {
        "{{VERSION}}": args.version,
        "{{MACOS_X86_64_URL}}": args.macos_x86_url,
        "{{MACOS_X86_64_SHA256}}": args.macos_x86_sha,
        "{{MACOS_ARM64_URL}}": args.macos_arm_url,
        "{{MACOS_ARM64_SHA256}}": args.macos_arm_sha,
    }
    for old, new in replacements.items():
        content = content.replace(old, new)
    Path(args.output).write_text(content, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
