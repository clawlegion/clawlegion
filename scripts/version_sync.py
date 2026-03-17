#!/usr/bin/env python3
"""Synchronize and validate release versions across ClawLegion packages."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover
    import tomli as tomllib  # type: ignore


ROOT = Path(__file__).resolve().parents[1]
ROOT_CARGO = ROOT / "Cargo.toml"
VERSION_FILE = ROOT / "VERSION"
WEB_PACKAGE = ROOT / "web" / "package.json"
PYPROJECT = ROOT / "crates" / "sdk" / "python" / "pyproject.toml"
PYTHON_INIT = ROOT / "crates" / "sdk" / "python" / "clawlegion" / "__init__.py"
NPM_PACKAGE = ROOT / "packages" / "npm" / "package.json"


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def write_text(path: Path, content: str) -> None:
    path.write_text(content, encoding="utf-8")

def root_version() -> str:
    value = read_text(VERSION_FILE).strip()
    if not value:
        raise ValueError("VERSION file is empty")
    return value


def cargo_workspace_version() -> str:
    data = tomllib.loads(read_text(ROOT_CARGO))
    return data["workspace"]["package"]["version"]


def package_json_version(path: Path) -> str:
    return json.loads(read_text(path))["version"]


def pyproject_version() -> str:
    data = tomllib.loads(read_text(PYPROJECT))
    return data["project"]["version"]


def python_init_version() -> str:
    match = re.search(r'^__version__ = "([^"]+)"$', read_text(PYTHON_INIT), re.MULTILINE)
    if not match:
        raise ValueError(f"could not find __version__ in {PYTHON_INIT}")
    return match.group(1)


def update_regex(path: Path, pattern: str, replacement: str, expected_count: int = 1) -> None:
    content = read_text(path)
    updated, count = re.subn(pattern, replacement, content, flags=re.MULTILINE)
    if count != expected_count:
        raise ValueError(f"expected {expected_count} replacements in {path}, got {count}")
    write_text(path, updated)


def set_package_json_version(path: Path, version: str) -> None:
    data = json.loads(read_text(path))
    data["version"] = version
    write_text(path, json.dumps(data, indent=2) + "\n")


def set_versions(version: str) -> None:
    write_text(VERSION_FILE, f"{version}\n")
    update_regex(
        ROOT_CARGO,
        r'^(version = )"[^"]+"$',
        rf'\1"{version}"',
    )
    update_regex(
        ROOT_CARGO,
        r'^(clawlegion-[a-z-]+ = \{ version = )"[^"]+"(, path = "crates/[^"]+" \})$',
        rf'\1"{version}"\2',
        expected_count=8,
    )
    set_package_json_version(WEB_PACKAGE, version)
    set_package_json_version(NPM_PACKAGE, version)
    update_regex(
        PYPROJECT,
        r'^(version = )"[^"]+"$',
        rf'\1"{version}"',
    )
    update_regex(
        PYTHON_INIT,
        r'^__version__ = "[^"]+"$',
        f'__version__ = "{version}"',
    )


def collect_versions() -> dict[str, str]:
    return {
        "root_version": root_version(),
        "cargo_workspace": cargo_workspace_version(),
        "web_package": package_json_version(WEB_PACKAGE),
        "python_pyproject": pyproject_version(),
        "python_init": python_init_version(),
        "npm_package": package_json_version(NPM_PACKAGE),
    }


def check_versions() -> int:
    versions = collect_versions()
    expected = versions["root_version"]
    mismatches = {
        name: value for name, value in versions.items() if value != expected
    }
    if mismatches:
        print(json.dumps({"expected": expected, "mismatches": mismatches}, indent=2))
        return 1
    print(
        json.dumps(
            {
                "status": "ok",
                "version": expected,
                "files": versions,
            },
            indent=2,
        )
    )
    return 0


SEMVER_PATTERN = re.compile(
    r"^(?P<core>\d+\.\d+\.\d+)"
    r"(?:-(?P<prerelease>[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
    r"(?:\+(?P<build>[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$"
)


def semver_prerelease(version: str) -> str | None:
    match = SEMVER_PATTERN.fullmatch(version)
    if not match:
        raise ValueError(f"unsupported version format: {version}")
    return match.group("prerelease")


def release_version(base_version: str, branch: str, default_branch: str, run_number: str) -> str:
    del run_number
    prerelease = semver_prerelease(base_version)
    if branch != default_branch and not prerelease:
        raise ValueError(
            "non-default branches require an explicit prerelease version in the repository"
        )
    return base_version


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    subparsers.add_parser("check")

    sync_parser = subparsers.add_parser("set-version")
    sync_parser.add_argument("version")

    subparsers.add_parser("sync")

    release_parser = subparsers.add_parser("release-version")
    release_parser.add_argument("--ref-name", required=True)
    release_parser.add_argument("--default-branch", required=True)
    release_parser.add_argument("--run-number", required=True)

    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.command == "check":
        return check_versions()
    if args.command == "set-version":
        set_versions(args.version)
        print(json.dumps({"status": "updated", "version": args.version}, indent=2))
        return 0
    if args.command == "sync":
        version = root_version()
        set_versions(version)
        print(json.dumps({"status": "synced", "version": version}, indent=2))
        return 0
    if args.command == "release-version":
        version = release_version(
            base_version=root_version(),
            branch=args.ref_name,
            default_branch=args.default_branch,
            run_number=args.run_number,
        )
        print(version)
        return 0
    raise AssertionError("unreachable")


if __name__ == "__main__":
    sys.exit(main())
