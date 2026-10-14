#!/usr/bin/env python3
"""Tests for scripts/version.py."""

import io
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import version  # noqa: E402


APP_FILES = (
    "desktop/package.json",
    "desktop/src-tauri/tauri.conf.json",
    "desktop/src-tauri/Cargo.toml",
    "desktop/src-tauri/Cargo.lock",
    "desktop/src-tauri/src/mcp.rs",
)


def write(root, relative, text):
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def git(root, args):
    proc = subprocess.run(["git", *args], cwd=root, capture_output=True, text=True)
    if proc.returncode != 0:
        detail = (proc.stderr or proc.stdout).strip()
        raise AssertionError(detail or f"git {' '.join(args)} failed")


def init_repo(root):
    git(root, ["init", "--template="])
    git(root, ["config", "user.email", "test@example.com"])
    git(root, ["config", "user.name", "Test"])
    git(root, ["config", "commit.gpgsign", "false"])


def app_fixture(root, app_version="0.1.0"):
    write(
        root,
        "desktop/package.json",
        '{\n  "name": "desktop",\n  "version": "%s",\n  "dependencies": {\n    "react": "^19.1.0"\n  }\n}\n'
        % app_version,
    )
    write(
        root,
        "desktop/src-tauri/tauri.conf.json",
        '{\n  "productName": "LiveUse",\n  "version": "%s"\n}\n' % app_version,
    )
    write(
        root,
        "desktop/src-tauri/Cargo.toml",
        '[package]\nname = "desktop"\nversion = "%s"\n\n[dependencies]\ntauri = { version = "2" }\n'
        % app_version,
    )
    write(
        root,
        "desktop/src-tauri/Cargo.lock",
        '[[package]]\nname = "desktop"\nversion = "%s"\ndependencies = [\n]\n\n[[package]]\nname = "other"\nversion = "0.1.0"\n'
        % app_version,
    )
    write(
        root,
        "desktop/src-tauri/src/mcp.rs",
        'const LEGACY: &str = "0.1.0";\n'
        '        "serverInfo": { "name": "liveuse", "version": "%s" }\n' % app_version,
    )
    write(root, "firmware/VERSION", "0.5.8\n")


def commit_all(root, message="initial"):
    git(root, ["add", "-A"])
    git(root, ["commit", "-m", message])


def head_count(root):
    proc = subprocess.run(
        ["git", "rev-list", "--count", "HEAD"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    return int(proc.stdout.strip())


def head_subject(root):
    proc = subprocess.run(
        ["git", "log", "-1", "--format=%s"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    return proc.stdout.strip()


def porcelain(root):
    proc = subprocess.run(
        ["git", "status", "--porcelain"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    return proc.stdout


def run_cli(root, args):
    stdout = io.StringIO()
    stderr = io.StringIO()
    with redirect_stdout(stdout), redirect_stderr(stderr):
        code = version.main(args, root=root)
    return code, stdout.getvalue(), stderr.getvalue()


class ResolveTest(unittest.TestCase):
    def test_patch_minor_major_and_mayor(self):
        self.assertEqual(version.resolve_version("0.1.0", "patch"), "0.1.1")
        self.assertEqual(version.resolve_version("0.1.9", ""), "0.1.10")
        self.assertEqual(version.resolve_version("0.1.4", "minor"), "0.2.0")
        self.assertEqual(version.resolve_version("0.1.4", "Minor"), "0.2.0")
        self.assertEqual(version.resolve_version("0.1.4", "major"), "1.0.0")
        self.assertEqual(version.resolve_version("0.1.4", "mayor"), "1.0.0")
        self.assertEqual(version.resolve_version("1.2.3", " 9.8.7 "), "9.8.7")

    def test_invalid_argument(self):
        for bad in ("1.2", "v1.2.3", "nope"):
            with self.assertRaises(SystemExit):
                with redirect_stderr(io.StringIO()):
                    version.resolve_version("1.2.3", bad)


class ApplyTest(unittest.TestCase):
    def setUp(self):
        base = Path(__file__).resolve().parent.parent
        self.tmp = tempfile.TemporaryDirectory(dir=base)
        self.root = Path(self.tmp.name)
        init_repo(self.root)
        app_fixture(self.root)
        commit_all(self.root)

    def tearDown(self):
        self.tmp.cleanup()

    def test_app_patch_writes_every_copy_and_commits(self):
        code, stdout, _stderr = run_cli(self.root, ["app"])
        self.assertEqual(code, 0)
        self.assertEqual(stdout, "0.1.1\n")
        package = (self.root / "desktop/package.json").read_text(encoding="utf-8")
        self.assertIn('"version": "0.1.1"', package)
        self.assertIn('"react": "^19.1.0"', package)
        self.assertIn(
            '"version": "0.1.1"',
            (self.root / "desktop/src-tauri/tauri.conf.json").read_text(encoding="utf-8"),
        )
        cargo = (self.root / "desktop/src-tauri/Cargo.toml").read_text(encoding="utf-8")
        self.assertIn('version = "0.1.1"', cargo)
        self.assertIn('tauri = { version = "2" }', cargo)
        lock = (self.root / "desktop/src-tauri/Cargo.lock").read_text(encoding="utf-8")
        self.assertIn('name = "desktop"\nversion = "0.1.1"', lock)
        self.assertIn('name = "other"\nversion = "0.1.0"', lock)
        mcp = (self.root / "desktop/src-tauri/src/mcp.rs").read_text(encoding="utf-8")
        self.assertIn('"version": "0.1.1"', mcp)
        self.assertIn('const LEGACY: &str = "0.1.0";', mcp)
        self.assertEqual(head_count(self.root), 2)
        self.assertEqual(head_subject(self.root), "chore: update app to version 0.1.1")
        self.assertEqual(porcelain(self.root), "")

    def test_app_set_and_mayor(self):
        code, stdout, _stderr = run_cli(self.root, ["app", "2.0.0"])
        self.assertEqual((code, stdout), (0, "2.0.0\n"))
        code, stdout, _stderr = run_cli(self.root, ["app", "mayor"])
        self.assertEqual((code, stdout), (0, "3.0.0\n"))

    def test_same_version_does_not_commit(self):
        code, stdout, _stderr = run_cli(self.root, ["app", "0.1.0"])
        self.assertEqual((code, stdout), (0, "0.1.0\n"))
        self.assertEqual(head_count(self.root), 1)

    def test_app_refuses_a_dirty_tree(self):
        write(self.root, "notes.txt", "dirty\n")
        with self.assertRaises(SystemExit):
            run_cli(self.root, ["app", "patch"])
        package = (self.root / "desktop/package.json").read_text(encoding="utf-8")
        self.assertIn('"version": "0.1.0"', package)
        self.assertEqual(head_count(self.root), 1)

    def test_app_refuses_mismatched_versions(self):
        write(
            self.root,
            "desktop/src-tauri/tauri.conf.json",
            '{\n  "productName": "LiveUse",\n  "version": "0.9.0"\n}\n',
        )
        commit_all(self.root, "diverge")
        with self.assertRaises(SystemExit):
            run_cli(self.root, ["app", "patch"])

    def test_current_and_next_do_not_write(self):
        code, stdout, _stderr = run_cli(self.root, ["current", "app"])
        self.assertEqual((code, stdout), (0, "0.1.0\n"))
        code, stdout, _stderr = run_cli(self.root, ["next", "firmware", "minor"])
        self.assertEqual((code, stdout), (0, "0.6.0\n"))
        self.assertEqual((self.root / "firmware/VERSION").read_text(encoding="utf-8"), "0.5.8\n")
        self.assertEqual(head_count(self.root), 1)
        self.assertEqual(porcelain(self.root), "")

    def test_firmware_patch_writes_version_without_committing(self):
        code, stdout, _stderr = run_cli(self.root, ["firmware", "patch"])
        self.assertEqual((code, stdout), (0, "0.5.9\n"))
        self.assertEqual((self.root / "firmware/VERSION").read_text(encoding="utf-8"), "0.5.9\n")
        self.assertEqual(head_count(self.root), 1)
        self.assertIn("firmware/VERSION", porcelain(self.root))

    def test_firmware_allows_version_files_and_refuses_other_dirt(self):
        write(self.root, "firmware/CHANGELOG.md", "# Changelog\n")
        code, stdout, _stderr = run_cli(self.root, ["firmware", "major"])
        self.assertEqual((code, stdout), (0, "1.0.0\n"))
        self.assertEqual(head_count(self.root), 1)

        write(self.root, "notes.txt", "nope\n")
        with self.assertRaises(SystemExit):
            run_cli(self.root, ["firmware", "patch"])
        self.assertEqual((self.root / "firmware/VERSION").read_text(encoding="utf-8"), "1.0.0\n")


if __name__ == "__main__":
    unittest.main()
