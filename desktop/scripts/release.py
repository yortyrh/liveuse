#!/usr/bin/env python3
"""Publish the current macOS app to the public releases repository.

After the upload, this repository gets the same app-vX.Y.Z tag on HEAD and a
GitHub release that links to the download. That release has no assets.
A published version is not replaced. The next version is a new tag.
This script writes apps.json. It does not write manifest.json.
"""

import argparse
import base64
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
DESKTOP = ROOT / "desktop"
DEFAULT_REPO = "yortyrh/liveuse-releases"
DEFAULT_SOURCE_REPO = "yortyrh/liveuse"
BRANCH = "main"
VERSION_RE = re.compile(r"^(\d+)\.(\d+)\.(\d+)$")
REPO_RE = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
PRODUCT_RE = re.compile(r"^[A-Za-z0-9._-]+$")
TARGETS = (
    {
        "id": "macos-x64",
        "triple": "x86_64-apple-darwin",
        "arch": "x64",
        "name": "macOS Intel",
    },
    {
        "id": "macos-aarch64",
        "triple": "aarch64-apple-darwin",
        "arch": "aarch64",
        "name": "macOS Apple Silicon",
    },
)


def fail(message):
    print(message, file=sys.stderr)
    raise SystemExit(1)


def read_text(path):
    try:
        return path.read_text(encoding="utf-8")
    except OSError as err:
        fail(f"Can't read {path.relative_to(ROOT)}: {err}")


def read_json(path):
    try:
        data = json.loads(read_text(path))
    except json.JSONDecodeError as err:
        fail(f"{path.relative_to(ROOT)} is not valid JSON: {err}")
    if not isinstance(data, dict):
        fail(f"{path.relative_to(ROOT)} has to be a JSON object.")
    return data


def version_tuple(version):
    match = VERSION_RE.match(version)
    if not match:
        fail(f"A version has to look like 1.2.3, not {version!r}.")
    return tuple(int(part) for part in match.groups())


def require_version():
    package = read_json(DESKTOP / "package.json")
    tauri = read_json(DESKTOP / "src-tauri" / "tauri.conf.json")
    package_version = package.get("version")
    tauri_version = tauri.get("version")
    if not isinstance(package_version, str) or not isinstance(tauri_version, str):
        fail("desktop/package.json and tauri.conf.json both need a version string.")
    if package_version != tauri_version:
        fail(
            f"desktop/package.json is {package_version} and "
            f"src-tauri/tauri.conf.json is {tauri_version}."
        )
    version_tuple(package_version)
    product = tauri.get("productName")
    if not isinstance(product, str) or not PRODUCT_RE.match(product):
        fail(
            f"productName has to be letters, digits, ., _ or -, not {product!r}. "
            "That token is the DMG file name."
        )
    return package_version, product


def release_repo():
    repo = os.environ.get("LIVEUSE_RELEASE_REPO", DEFAULT_REPO).strip()
    if not REPO_RE.match(repo):
        fail(f"LIVEUSE_RELEASE_REPO has to look like owner/name, not {repo!r}.")
    return repo


def source_repo():
    repo = os.environ.get("LIVEUSE_SOURCE_REPO", DEFAULT_SOURCE_REPO).strip()
    if not REPO_RE.match(repo):
        fail(f"LIVEUSE_SOURCE_REPO has to look like owner/name, not {repo!r}.")
    return repo


def git(args):
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True)


def require_clean():
    proc = subprocess.run(
        ["git", "status", "--porcelain"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        fail(proc.stderr.strip() or "git status failed.")
    if proc.stdout.strip():
        fail("The working tree has uncommitted changes. Commit them before publishing.")


def require_macos():
    if sys.platform != "darwin":
        fail("macOS app bundles are built on macOS.")


def require_tool(name, hint):
    found = shutil.which(name)
    if found is None:
        fail(hint)
    return found


def require_rust_targets():
    require_tool("rustup", "Can't find rustup. Install the Rust toolchain and run this command again.")
    proc = subprocess.run(
        ["rustup", "target", "list", "--installed"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        fail(proc.stderr.strip() or "rustup target list failed.")
    installed = set(proc.stdout.split())
    missing = [item["triple"] for item in TARGETS if item["triple"] not in installed]
    if missing:
        joined = " ".join(missing)
        fail(f"Missing Rust targets: {joined}. Run: rustup target add {joined}")


def has_developer_id():
    if shutil.which("security") is None:
        return False
    proc = subprocess.run(
        ["security", "find-identity", "-v", "-p", "codesigning"],
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        return False
    return any("Developer ID Application:" in line for line in proc.stdout.splitlines())


def bundle_root():
    override = os.environ.get("CARGO_TARGET_DIR", "").strip()
    if override:
        return Path(override)
    return DESKTOP / "src-tauri" / "target"


def dmg_file(product, version, arch):
    return f"{product}_{version}_{arch}.dmg"


def dmg_path(product, version, target):
    filename = dmg_file(product, version, target["arch"])
    return bundle_root() / target["triple"] / "release" / "bundle" / "dmg" / filename


def build_target(target, signed):
    print(f"Compiling {target['triple']}...", file=sys.stderr)
    command = [
        "pnpm",
        "tauri",
        "build",
        "--target",
        target["triple"],
        "--bundles",
        "dmg",
        "--ci",
    ]
    if not signed:
        command.append("--no-sign")
    proc = subprocess.run(command, cwd=DESKTOP)
    if proc.returncode != 0:
        fail("The build failed. Nothing was published.")


def sha256_file(path):
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def apps_document(version, repo, signed, built):
    tag = f"app-v{version}"
    downloads = []
    for target, path in built:
        filename = path.name
        downloads.append(
            {
                "id": target["id"],
                "os": "macos",
                "arch": target["arch"],
                "name": target["name"],
                "file": filename,
                "url": f"https://github.com/{repo}/releases/download/{tag}/{filename}",
                "sha256": sha256_file(path),
                "size": path.stat().st_size,
            }
        )
    body = {
        "schema": 1,
        "version": version,
        "tag": tag,
        "signed": signed,
        "downloads": downloads,
    }
    text = json.dumps(body, indent=2, ensure_ascii=False) + "\n"
    return text, text.encode("utf-8")


def release_notes(version, signed, text):
    body = json.loads(text)
    lines = [f"LiveUse {version} for macOS.", ""]
    for item in body["downloads"]:
        lines.append(f"- `{item['file']}` ({item['name']})")
        lines.append(f"  SHA-256: `{item['sha256']}`")
    lines.append("")
    if signed:
        lines.append("Signed with a Developer ID certificate. Not notarized by this script.")
    else:
        lines.append(
            "Not signed or notarized. macOS may block the first open. "
            "Right-click the app and choose Open."
        )
    return "\n".join(lines)


def gh(args, payload=None):
    if shutil.which("gh") is None:
        fail("Can't find gh. Install GitHub CLI and run gh auth login.")
    env = os.environ.copy()
    env["GH_PROMPT_DISABLED"] = "1"
    kwargs = {
        "text": True,
        "capture_output": True,
        "cwd": ROOT,
        "env": env,
    }
    if payload is None:
        kwargs["stdin"] = subprocess.DEVNULL
    else:
        kwargs["input"] = json.dumps(payload)
    return subprocess.run(["gh", *args], **kwargs)


def require_gh_auth():
    proc = gh(["auth", "status"])
    if proc.returncode != 0:
        detail = (proc.stderr or proc.stdout).strip()
        fail("gh is not logged in. Run gh auth login and try again.\n" + detail)


def gh_json(args, payload=None):
    proc = gh(args, payload)
    if proc.returncode != 0:
        return None, proc
    try:
        return json.loads(proc.stdout), proc
    except json.JSONDecodeError:
        fail(proc.stdout.strip() or proc.stderr.strip() or "gh returned unexpected output.")


def repo_view(repo):
    data, proc = gh_json(
        ["repo", "view", repo, "--json", "visibility,isEmpty,defaultBranchRef"]
    )
    if data is None:
        message = (proc.stderr or proc.stdout).lower()
        if "not found" in message or "could not resolve" in message:
            return None
        fail((proc.stderr or proc.stdout).strip() or f"Can't view {repo}.")
    return data


def default_branch(info):
    ref = info.get("defaultBranchRef") or {}
    name = ref.get("name")
    if not isinstance(name, str) or not name.strip():
        return None
    return name


def require_public_main(repo, info):
    if info.get("visibility") != "PUBLIC":
        fail(f"{repo} is not public. The site has to read apps.json without a token.")
    branch = default_branch(info)
    if branch != BRANCH:
        fail(
            f"{repo} uses branch {branch or 'nothing'}. "
            f"The app manifest URL is /{BRANCH}/apps.json, so the default branch has to be {BRANCH}."
        )


def contents_of(repo, path):
    data, proc = gh_json(["api", f"repos/{repo}/contents/{path}?ref={BRANCH}"])
    if data is None:
        message = (proc.stderr or proc.stdout).lower()
        if "not found" in message or "repository is empty" in message:
            return None
        fail((proc.stderr or proc.stdout).strip() or f"Can't read {path} from {repo}.")
    if data.get("type") != "file" or "content" not in data:
        fail(f"{path} on {repo} is not a file.")
    try:
        current = base64.b64decode(data["content"])
    except (ValueError, TypeError) as err:
        fail(f"Can't decode {path} from {repo}: {err}")
    return data.get("sha"), current


def put_contents(repo, path, data, message):
    existing = contents_of(repo, path)
    if existing is not None and existing[1] == data:
        return False
    payload = {
        "message": message,
        "content": base64.b64encode(data).decode("ascii"),
        "branch": BRANCH,
    }
    if existing is not None:
        payload["sha"] = existing[0]
    proc = gh(["api", "--method", "PUT", f"repos/{repo}/contents/{path}", "--input", "-"], payload)
    if proc.returncode != 0:
        fail((proc.stderr or proc.stdout).strip() or f"Can't write {path} on {repo}.")
    return True


def ensure_repo(repo):
    info = repo_view(repo)
    if info is None:
        print(f"Creating {repo}...", file=sys.stderr)
        proc = gh(
            [
                "repo",
                "create",
                repo,
                "--public",
                "--description",
                "Public LiveUse firmware and macOS app releases.",
                "--add-readme",
            ]
        )
        if proc.returncode != 0:
            fail((proc.stderr or proc.stdout).strip() or f"Can't create {repo}.")
        info = repo_view(repo)
        if info is None:
            fail(f"Created {repo}, but gh repo view cannot see it.")
    if info.get("visibility") != "PUBLIC":
        fail(f"{repo} is not public. The site has to read apps.json without a token.")
    if info.get("isEmpty") or default_branch(info) is None:
        fail(f"{repo} has no commits on {BRANCH}.")
    require_public_main(repo, info)


def release_view(repo, tag):
    data, proc = gh_json(["release", "view", tag, "--repo", repo, "--json", "url,isDraft"])
    if data is None:
        message = (proc.stderr or proc.stdout).lower()
        if "not found" in message or "release not found" in message:
            return None
        fail((proc.stderr or proc.stdout).strip() or f"Can't view release {tag}.")
    return data


def require_unpublished(repo, tag):
    existing = release_view(repo, tag)
    if existing is None:
        return
    if existing.get("isDraft"):
        fail(
            f"{tag} is a draft on {repo}. Delete that draft on GitHub, then run this command again."
        )
    fail(
        f"{tag} is already published: {existing.get('url')}\n"
        "A published version is not replaced."
    )


def apps_version_on_main(repo):
    existing = contents_of(repo, "apps.json")
    if existing is None:
        return None
    try:
        body = json.loads(existing[1].decode("utf-8"))
    except (UnicodeError, json.JSONDecodeError):
        return None
    version = body.get("version") if isinstance(body, dict) else None
    if not isinstance(version, str) or not VERSION_RE.match(version):
        return None
    return version_tuple(version)


def point_apps(repo, version, data):
    current = apps_version_on_main(repo)
    if current is not None and current > version_tuple(version):
        fail(
            f"apps.json on {repo} is already newer than {version}. "
            "This command does not roll it back."
        )
    changed = put_contents(repo, "apps.json", data, f"Publish app {version}.")
    url = f"https://raw.githubusercontent.com/{repo}/{BRANCH}/apps.json"
    if changed:
        print(url)
    else:
        print(f"{url} already matches {version}.")


def create_release(repo, version, notes, dmg_paths, apps_bytes):
    tag = f"app-v{version}"
    with tempfile.TemporaryDirectory() as tmp:
        folder = Path(tmp)
        notes_path = folder / "notes.md"
        notes_path.write_text(notes + "\n", encoding="utf-8")
        apps_path = folder / "apps.json"
        apps_path.write_bytes(apps_bytes)
        print(f"Publishing {tag}...", file=sys.stderr)
        proc = gh(
            [
                "release",
                "create",
                tag,
                "--repo",
                repo,
                "--title",
                f"LiveUse {version}",
                "--notes-file",
                str(notes_path),
                "--latest=false",
                str(apps_path),
                *[str(path) for path in dmg_paths],
            ]
        )
        if proc.returncode != 0:
            fail((proc.stderr or proc.stdout).strip() or f"Can't create {tag}.")
        if proc.stderr.strip():
            print(proc.stderr.strip(), file=sys.stderr)
    print(f"https://github.com/{repo}/releases/tag/{tag}")


def source_notes(version, repo, notes):
    tag = f"app-v{version}"
    link = f"Download: https://github.com/{repo}/releases/tag/{tag}"
    return f"{notes}\n\n{link}"


def tag_source(tag, version):
    head = git(["rev-parse", "HEAD"])
    if head.returncode != 0:
        fail(head.stderr.strip() or "git rev-parse HEAD failed.")
    head = head.stdout.strip()
    existing = git(["rev-parse", "--verify", "--quiet", f"refs/tags/{tag}^{{commit}}"])
    if existing.returncode == 0:
        if existing.stdout.strip() != head:
            fail(f"{tag} already exists on {existing.stdout.strip()[:7]}, not on HEAD.")
    else:
        proc = git(["tag", "-a", tag, "-m", f"LiveUse {version}", head])
        if proc.returncode != 0:
            fail(proc.stderr.strip() or f"Can't create tag {tag}.")
    proc = git(["push", "origin", f"refs/tags/{tag}"])
    if proc.returncode != 0:
        fail(proc.stderr.strip() or f"Can't push tag {tag}.")


def register_source(version, release_repo_name, notes):
    repo = source_repo()
    tag = f"app-v{version}"
    tag_source(tag, version)
    if release_view(repo, tag) is not None:
        print(f"{tag} is already registered on {repo}.", file=sys.stderr)
        return
    with tempfile.TemporaryDirectory() as tmp:
        notes_path = Path(tmp) / "notes.md"
        notes_path.write_text(source_notes(version, release_repo_name, notes) + "\n", encoding="utf-8")
        proc = gh(
            [
                "release",
                "create",
                tag,
                "--repo",
                repo,
                "--verify-tag",
                "--title",
                f"LiveUse {version}",
                "--notes-file",
                str(notes_path),
                "--latest=false",
            ]
        )
    if proc.returncode != 0:
        fail((proc.stderr or proc.stdout).strip() or f"Can't create {tag} on {repo}.")
    print(f"https://github.com/{repo}/releases/tag/{tag}")


def register_existing(version):
    repo = release_repo()
    require_gh_auth()
    tag = f"app-v{version}"
    existing = release_view(repo, tag)
    if existing is None or existing.get("isDraft"):
        fail(f"{tag} is not a published release on {repo}.")
    data, proc = gh_json(["release", "view", tag, "--repo", repo, "--json", "body"])
    if data is None:
        fail((proc.stderr or proc.stdout).strip() or f"Can't read the notes of {tag}.")
    register_source(version, repo, data.get("body", "").strip())


def download_apps(repo, tag):
    with tempfile.TemporaryDirectory() as tmp:
        proc = gh(
            [
                "release",
                "download",
                tag,
                "--repo",
                repo,
                "--pattern",
                "apps.json",
                "--dir",
                tmp,
            ]
        )
        if proc.returncode != 0:
            fail((proc.stderr or proc.stdout).strip() or f"Can't download apps.json from {tag}.")
        path = Path(tmp) / "apps.json"
        if not path.is_file():
            fail(f"{tag} has no apps.json asset.")
        return path.read_bytes()


def point_existing(version):
    repo = release_repo()
    require_gh_auth()
    info = repo_view(repo)
    tag = f"app-v{version}"
    if info is None:
        fail(f"{repo} does not exist, so {tag} is not published.")
    require_public_main(repo, info)
    existing = release_view(repo, tag)
    if existing is None or existing.get("isDraft"):
        fail(f"{tag} is not a published release on {repo}.")
    data = download_apps(repo, tag)
    try:
        body = json.loads(data.decode("utf-8"))
    except (UnicodeError, json.JSONDecodeError) as err:
        fail(f"The apps.json asset on {tag} is not JSON: {err}")
    if not isinstance(body, dict) or body.get("version") != version or body.get("tag") != tag:
        fail(f"The apps.json asset on {tag} is not app {version}.")
    point_apps(repo, version, data)


def compile_both(product, version, signed):
    require_macos()
    require_tool("pnpm", "Can't find pnpm. Install it and run this command again.")
    require_rust_targets()
    built = []
    for target in TARGETS:
        build_target(target, signed)
        path = dmg_path(product, version, target)
        if not path.is_file():
            fail(f"The build did not produce {path}.")
        built.append((target, path))
    return built


def publish(version, product):
    repo = release_repo()
    require_clean()
    require_gh_auth()
    ensure_repo(repo)
    require_unpublished(repo, f"app-v{version}")
    signed = has_developer_id()
    built = compile_both(product, version, signed)
    text, apps_bytes = apps_document(version, repo, signed, built)
    notes = release_notes(version, signed, text)
    dmg_paths = [path for _target, path in built]
    create_release(repo, version, notes, dmg_paths, apps_bytes)
    try:
        point_apps(repo, version, apps_bytes)
    except SystemExit as err:
        print(
            "The release is up, but apps.json on main was not updated.\n"
            "The previous apps.json is still what a reader would fetch.\n"
            "Retry with: python3 desktop/scripts/release.py --point",
            file=sys.stderr,
        )
        raise err
    try:
        register_source(version, repo, notes)
    except SystemExit as err:
        print(
            f"The release is up, but it is not registered on {source_repo()}.\n"
            "Retry with: python3 desktop/scripts/release.py --register",
            file=sys.stderr,
        )
        raise err


def main(argv=None):
    parser = argparse.ArgumentParser(
        prog="release.py",
        description=(
            "Publish the current macOS app to the public releases repository. "
            "Then tag HEAD and register the release on the source repository. "
            "manifest.json is left untouched."
        ),
    )
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument(
        "--check",
        action="store_true",
        help="Check that the app version strings match. Do not compile or upload.",
    )
    mode.add_argument(
        "--dry-run",
        action="store_true",
        help="Compile both macOS architectures and print apps.json. Do not upload.",
    )
    mode.add_argument(
        "--point",
        action="store_true",
        help="Write the published apps.json onto main. Use this if that step failed.",
    )
    mode.add_argument(
        "--register",
        action="store_true",
        help="Tag HEAD and register the published release on the source repository. "
        "Use this if that step failed.",
    )
    args = parser.parse_args(argv)

    version, product = require_version()
    if args.check:
        names = ", ".join(item["id"] for item in TARGETS)
        print(f"{version}: {names}")
        return 0
    if args.point:
        point_existing(version)
        return 0
    if args.register:
        register_existing(version)
        return 0

    signed = has_developer_id()
    if args.dry_run:
        built = compile_both(product, version, signed)
        text, _apps_bytes = apps_document(version, release_repo(), signed, built)
        print(text, end="")
        return 0

    publish(version, product)
    return 0


if __name__ == "__main__":
    sys.exit(main())
