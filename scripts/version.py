#!/usr/bin/env python3
"""Read, calculate, and write the LiveUse app version.

app-release calls this. With no part, the patch number increments. patch,
minor, and major increment that component. mayor is major. X.Y.Z sets that
version.

current and next only print. app writes the app version files and commits
them.
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VERSION_RE = re.compile(r"^(\d+)\.(\d+)\.(\d+)$")
PARTS = {
    "patch": "patch",
    "minor": "minor",
    "major": "major",
    "mayor": "major",
}
JSON_VERSION_RE = re.compile(r'("version": ")(\d+\.\d+\.\d+)(")')
CARGO_VERSION_RE = re.compile(r'^(version = ")(\d+\.\d+\.\d+)(")', re.MULTILINE)
LOCK_VERSION_RE = re.compile(
    r'(\[\[package\]\]\r?\nname = "desktop"\r?\nversion = ")(\d+\.\d+\.\d+)("\r?\n)'
)
MCP_VERSION_RE = re.compile(
    r'("serverInfo": \{ "name": "liveuse", "version": ")(\d+\.\d+\.\d+)(" \})'
)
APP_FIELDS = (
    ("desktop/package.json", JSON_VERSION_RE),
    ("desktop/src-tauri/tauri.conf.json", JSON_VERSION_RE),
    ("desktop/src-tauri/Cargo.toml", CARGO_VERSION_RE),
    ("desktop/src-tauri/Cargo.lock", LOCK_VERSION_RE),
    ("desktop/src-tauri/src/mcp.rs", MCP_VERSION_RE),
)

def fail(message):
    print(message, file=sys.stderr)
    raise SystemExit(1)


def version_tuple(version):
    match = VERSION_RE.match(version)
    if not match:
        fail(f"A version has to look like 1.2.3, not {version!r}.")
    return tuple(int(part) for part in match.groups())


def resolve_version(current, part):
    major, minor, patch = version_tuple(current)
    token = part.strip().lower()
    if token == "":
        token = "patch"
    kind = PARTS.get(token)
    if kind == "patch":
        return f"{major}.{minor}.{patch + 1}"
    if kind == "minor":
        return f"{major}.{minor + 1}.0"
    if kind == "major":
        return f"{major + 1}.0.0"
    if VERSION_RE.match(token):
        return token
    fail(
        "The version argument has to be patch, minor, major, or a version like 1.2.3, "
        f"not {part.strip()!r}."
    )


def read_text(path):
    try:
        return path.read_text(encoding="utf-8")
    except OSError as err:
        fail(f"Can't read {path}: {err}")


def field_version(text, pattern, label):
    found = pattern.findall(text)
    if len(found) != 1:
        fail(f"{label} has {len(found)} version strings, and it needs 1.")
    return found[0][1]


def replace_field(text, pattern, new, label):
    field_version(text, pattern, label)
    return pattern.sub(lambda match: match.group(1) + new + match.group(3), text, count=1)


def read_app_version(root):
    found = []
    for relative, pattern in APP_FIELDS:
        label = relative
        found.append((label, field_version(read_text(root / relative), pattern, label)))
    versions = {item[1] for item in found}
    if len(versions) != 1:
        detail = ", ".join(f"{label} is {value}" for label, value in found)
        fail(f"App version files disagree: {detail}.")
    version = versions.pop()
    version_tuple(version)
    return version


def read_version(root, target):
    if target == "app":
        return read_app_version(root)
    fail(f"The target has to be app, not {target!r}.")


def git_run(root, args):
    proc = subprocess.run(
        ["git", *args],
        cwd=root,
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        detail = (proc.stderr or proc.stdout).strip()
        fail(detail or f"git {' '.join(args)} failed.")
    return proc


def dirty_paths(root):
    proc = git_run(root, ["status", "--porcelain", "--untracked-files=all"])
    paths = []
    for line in proc.stdout.splitlines():
        if len(line) < 4:
            continue
        entry = line[3:]
        if " -> " in entry:
            entry = entry.split(" -> ", 1)[1]
        if entry.startswith('"') and entry.endswith('"'):
            entry = entry[1:-1]
        paths.append(entry)
    return paths


def require_app_tree(root):
    paths = dirty_paths(root)
    if paths:
        fail(
            "The working tree has uncommitted changes. "
            "Commit them before changing the app version."
        )


def write_app_version(root, new):
    paths = []
    for relative, pattern in APP_FIELDS:
        path = root / relative
        updated = replace_field(read_text(path), pattern, new, relative)
        path.write_text(updated, encoding="utf-8")
        paths.append(relative)
    return paths


def commit_app(root, new, paths):
    git_run(root, ["add", "--", *paths])
    git_run(root, ["commit", "-m", f"chore: update app to version {new}", "--", *paths])
    print(f"Committed app {new}.", file=sys.stderr)


def apply_version(root, target, part="patch"):
    root = Path(root)
    if target == "app":
        require_app_tree(root)
        current = read_app_version(root)
        new = resolve_version(current, part)
        if new == current:
            print(f"Already {current}.", file=sys.stderr)
            return current
        paths = write_app_version(root, new)
        commit_app(root, new, paths)
        return new
    fail(f"The target has to be app, not {target!r}.")


def main(argv=None, root=None):
    root = Path(root) if root is not None else ROOT
    parser = argparse.ArgumentParser(
        prog="version.py",
        description=(
            "Read, calculate, and write the app version. "
            "patch, minor, and major increment that component. "
            "mayor is major. X.Y.Z sets that version. "
            "With no part, the patch number increments."
        ),
    )
    sub = parser.add_subparsers(dest="action", required=True)

    for name in ("app",):
        command = sub.add_parser(name, help=f"Write the next {name} version.")
        command.add_argument(
            "part",
            nargs="?",
            default="patch",
            help="patch, minor, major, mayor, or X.Y.Z. Default: patch.",
        )
        command.set_defaults(target=name, mode="apply")

    current = sub.add_parser("current", help="Print the current version.")
    current.add_argument("target", choices=("app",))
    current.set_defaults(mode="current")

    upcoming = sub.add_parser("next", help="Print the next version without writing it.")
    upcoming.add_argument("target", choices=("app",))
    upcoming.add_argument("part", nargs="?", default="patch")
    upcoming.set_defaults(mode="next")

    args = parser.parse_args(argv)
    if args.mode == "current":
        print(read_version(root, args.target))
        return 0
    if args.mode == "next":
        print(resolve_version(read_version(root, args.target), args.part))
        return 0
    print(apply_version(root, args.target, args.part))
    return 0


if __name__ == "__main__":
    sys.exit(main())
