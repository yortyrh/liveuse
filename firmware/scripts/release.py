#!/usr/bin/env python3
"""Publish the current firmware version to the public releases repository.

The source repositories are not tagged and nothing is pushed to them.
A published version is not replaced. The next version is a new tag.
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
FIRMWARE = ROOT / "firmware"
DEFAULT_REPO = "yortyrh/liveuse-releases"
BRANCH = "main"
VERSION_RE = re.compile(r"^(\d+)\.(\d+)\.(\d+)$")
HEADING_RE = re.compile(r"^## \[(\d+\.\d+\.\d+)\]", re.MULTILINE)
README_VERSION_RE = re.compile(r"Version \*\*(\d+\.\d+\.\d+)\*\*")
FW_RE = re.compile(r"fw=(\d+\.\d+\.\d+)")
REPO_RE = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
ID_RE = re.compile(r"^[A-Za-z0-9_-]+$")
HEX_ID_RE = re.compile(r"^0x[0-9A-Fa-f]{4}$")
README_TEXT = "This repository only publishes LiveUse firmware versions.\n"


def fail(message):
    print(message, file=sys.stderr)
    raise SystemExit(1)


def read_text(path):
    try:
        return path.read_text(encoding="utf-8")
    except OSError as err:
        fail(f"Can't read {path.relative_to(ROOT)}: {err}")


def version_tuple(version):
    match = VERSION_RE.match(version)
    if not match:
        fail(f"A version has to look like 1.2.3, not {version!r}.")
    return tuple(int(part) for part in match.groups())


def require_version():
    version = read_text(FIRMWARE / "VERSION").strip()
    version_tuple(version)

    sketch = read_text(FIRMWARE / "src" / "main.cpp")
    sketch_versions = FW_RE.findall(sketch)
    if sketch_versions != [version]:
        fail(
            f"firmware/src/main.cpp prints fw={sketch_versions or 'nothing'}, "
            f"and firmware/VERSION is {version}."
        )

    readme = read_text(FIRMWARE / "README.md")
    readme_version = README_VERSION_RE.search(readme)
    readme_fw = FW_RE.findall(readme)
    if readme_version is None or readme_version.group(1) != version or readme_fw != [version]:
        found = readme_version.group(1) if readme_version else "nothing"
        fail(
            f"firmware/README.md says version {found} and fw={readme_fw or 'nothing'}, "
            f"and firmware/VERSION is {version}."
        )

    changelog = read_text(FIRMWARE / "CHANGELOG.md")
    headings = HEADING_RE.findall(changelog)
    if not headings or headings[0] != version:
        newest = headings[0] if headings else "nothing"
        fail(
            f"The newest firmware/CHANGELOG.md heading is {newest}, "
            f"and firmware/VERSION is {version}."
        )
    return version, changelog_notes(changelog, version)


def changelog_notes(changelog, version):
    marker = f"## [{version}]"
    start = changelog.find(marker)
    if start < 0:
        fail(f"firmware/CHANGELOG.md has no section for {version}.")
    lines = changelog[start:].splitlines()
    kept = [lines[0]]
    for line in lines[1:]:
        if line.startswith("## ["):
            break
        kept.append(line)
    notes = "\n".join(kept).strip()
    if not notes:
        fail(f"The changelog section for {version} is empty.")
    return notes


def require_usb(usb, label):
    if not isinstance(usb, dict):
        fail(f"{label} has to be an object with vid and pid.")
    for key in ("vid", "pid"):
        value = usb.get(key)
        if not isinstance(value, str) or not HEX_ID_RE.match(value):
            fail(f"{label}.{key} has to look like 0x2886.")
    return {"vid": usb["vid"], "pid": usb["pid"]}


def require_catalog():
    path = FIRMWARE / "boards.json"
    try:
        catalog = json.loads(read_text(path))
    except json.JSONDecodeError as err:
        fail(f"firmware/boards.json is not valid JSON: {err}")
    boards = catalog.get("boards") if isinstance(catalog, dict) else None
    if not isinstance(boards, list) or not boards:
        fail("firmware/boards.json needs a non-empty boards list.")

    seen = set()
    checked = []
    for board in boards:
        if not isinstance(board, dict):
            fail("Each board in firmware/boards.json has to be an object.")
        board_id = board.get("id")
        if not isinstance(board_id, str) or not ID_RE.match(board_id):
            fail(f"A board id has to be letters, digits, _ or -, not {board_id!r}.")
        if board_id in seen:
            fail(f"firmware/boards.json lists {board_id} twice.")
        seen.add(board_id)
        name = board.get("name")
        env = board.get("env")
        image = board.get("image")
        if not isinstance(name, str) or not name.strip():
            fail(f"{board_id} needs a name.")
        if not isinstance(env, str) or not ID_RE.match(env):
            fail(f"{board_id} needs a PlatformIO env name.")
        if not isinstance(image, str) or image.startswith(("/", "\\")) or ".." in Path(image).parts:
            fail(f"{board_id} image has to be a path inside firmware/.")
        usb = board.get("usb")
        if not isinstance(usb, dict):
            fail(f"{board_id} needs a usb object.")
        flash = board.get("flash")
        if not isinstance(flash, dict):
            fail(f"{board_id} needs a flash object.")
        for key in ("protocol", "enter", "offset"):
            value = flash.get(key)
            if not isinstance(value, str) or not value.strip():
                fail(f"{board_id} flash.{key} has to be a non-empty string.")
        checked.append(
            {
                "id": board_id,
                "name": name,
                "env": env,
                "image": image,
                "usb": {
                    "running": require_usb(usb.get("running"), f"{board_id} usb.running"),
                    "bootloader": require_usb(
                        usb.get("bootloader"), f"{board_id} usb.bootloader"
                    ),
                },
                "flash": dict(flash),
            }
        )
    return checked


def release_repo():
    repo = os.environ.get("LIVEUSE_RELEASE_REPO", DEFAULT_REPO).strip()
    if not REPO_RE.match(repo):
        fail(f"LIVEUSE_RELEASE_REPO has to look like owner/name, not {repo!r}.")
    return repo


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


def pio_bin():
    for name in ("pio", "platformio"):
        found = shutil.which(name)
        if found:
            return found
    fail("Can't find pio (PlatformIO). Install it and run this command again.")


def compile_board(pio, board):
    print(f"Compiling {board['env']}...", file=sys.stderr)
    proc = subprocess.run([pio, "run", "-e", board["env"]], cwd=FIRMWARE)
    if proc.returncode != 0:
        fail("The build failed. Nothing was published.")
    image = (FIRMWARE / board["image"]).resolve()
    try:
        image.relative_to(FIRMWARE.resolve())
    except ValueError:
        fail(f"{board['id']} image escapes firmware/.")
    if not image.is_file():
        fail(f"The build did not produce {board['image']}.")
    return image


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
        fail(f"{repo} is not public. The app has to read the manifest without a token.")
    branch = default_branch(info)
    if branch != BRANCH:
        fail(
            f"{repo} uses branch {branch or 'nothing'}. "
            f"The manifest URL is /{BRANCH}/manifest.json, so the default branch has to be {BRANCH}."
        )


def write_readme(repo):
    put_contents(
        repo,
        "README.md",
        README_TEXT.encode("utf-8"),
        "Describe the releases repository.",
    )


def ensure_repo(repo):
    info = repo_view(repo)
    created = info is None
    if created:
        print(f"Creating {repo}...", file=sys.stderr)
        proc = gh(
            [
                "repo",
                "create",
                repo,
                "--public",
                "--description",
                README_TEXT.strip(),
                "--add-readme",
            ]
        )
        if proc.returncode != 0:
            fail((proc.stderr or proc.stdout).strip() or f"Can't create {repo}.")
        info = repo_view(repo)
        if info is None:
            fail(f"Created {repo}, but gh repo view cannot see it.")
    if info.get("visibility") != "PUBLIC":
        fail(f"{repo} is not public. The app has to read the manifest without a token.")
    # An empty repository has no branch until the first commit. GitHub reports that
    # branch name as blank, which is not a different default branch.
    if info.get("isEmpty") or default_branch(info) is None:
        print(f"{repo} has no commits. Creating {BRANCH}...", file=sys.stderr)
        write_readme(repo)
        info = repo_view(repo)
        if info is None:
            fail(f"Can't view {repo} after creating {BRANCH}.")
    elif created:
        write_readme(repo)
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


def manifest_for(version, notes, repo, built):
    tag = f"firmware-v{version}"
    boards = []
    files = {}
    for board, image in built:
        data = image.read_bytes()
        filename = f"liveuse-{board['id']}-{version}.bin"
        if filename in files:
            fail(f"Two boards would publish {filename}.")
        boards.append(
            {
                "id": board["id"],
                "name": board["name"],
                "file": filename,
                "url": f"https://github.com/{repo}/releases/download/{tag}/{filename}",
                "sha256": hashlib.sha256(data).hexdigest(),
                "size": len(data),
                "usb": board["usb"],
                "flash": board["flash"],
            }
        )
        files[filename] = data
    body = {
        "schema": 1,
        "version": version,
        "tag": tag,
        "notes": notes,
        "boards": boards,
    }
    text = json.dumps(body, indent=2, ensure_ascii=False) + "\n"
    files["manifest.json"] = text.encode("utf-8")
    return text, files


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


def manifest_version_on_main(repo):
    existing = contents_of(repo, "manifest.json")
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


def point_manifest(repo, version, data):
    current = manifest_version_on_main(repo)
    if current is not None and current > version_tuple(version):
        fail(
            f"manifest.json on {repo} is already newer than {version}. "
            "This command does not roll it back."
        )
    changed = put_contents(repo, "manifest.json", data, f"Publish firmware {version}.")
    url = f"https://raw.githubusercontent.com/{repo}/{BRANCH}/manifest.json"
    if changed:
        print(url)
    else:
        print(f"{url} already matches {version}.")


def create_release(repo, version, notes, files):
    tag = f"firmware-v{version}"
    with tempfile.TemporaryDirectory() as tmp:
        folder = Path(tmp)
        notes_path = folder / "notes.md"
        notes_path.write_text(notes + "\n", encoding="utf-8")
        paths = []
        for name, data in files.items():
            path = folder / name
            path.write_bytes(data)
            paths.append(str(path))
        print(f"Publishing {tag}...", file=sys.stderr)
        proc = gh(
            [
                "release",
                "create",
                tag,
                "--repo",
                repo,
                "--title",
                f"Firmware {version}",
                "--notes-file",
                str(notes_path),
                "--latest",
                *paths,
            ]
        )
        if proc.returncode != 0:
            fail((proc.stderr or proc.stdout).strip() or f"Can't create {tag}.")
        if proc.stderr.strip():
            print(proc.stderr.strip(), file=sys.stderr)
    print(f"https://github.com/{repo}/releases/tag/{tag}")


def download_manifest(repo, tag):
    with tempfile.TemporaryDirectory() as tmp:
        proc = gh(
            [
                "release",
                "download",
                tag,
                "--repo",
                repo,
                "--pattern",
                "manifest.json",
                "--dir",
                tmp,
            ]
        )
        if proc.returncode != 0:
            fail((proc.stderr or proc.stdout).strip() or f"Can't download manifest.json from {tag}.")
        path = Path(tmp) / "manifest.json"
        if not path.is_file():
            fail(f"{tag} has no manifest.json asset.")
        return path.read_bytes()


def point_existing(version):
    repo = release_repo()
    require_gh_auth()
    info = repo_view(repo)
    tag = f"firmware-v{version}"
    if info is None:
        fail(f"{repo} does not exist, so {tag} is not published.")
    require_public_main(repo, info)
    existing = release_view(repo, tag)
    if existing is None or existing.get("isDraft"):
        fail(f"{tag} is not a published release on {repo}.")
    data = download_manifest(repo, tag)
    try:
        body = json.loads(data.decode("utf-8"))
    except (UnicodeError, json.JSONDecodeError) as err:
        fail(f"The manifest asset on {tag} is not JSON: {err}")
    if not isinstance(body, dict) or body.get("version") != version:
        fail(f"The manifest asset on {tag} is not firmware {version}.")
    point_manifest(repo, version, data)


def publish(version, notes, boards):
    repo = release_repo()
    require_clean()
    require_gh_auth()
    ensure_repo(repo)
    require_unpublished(repo, f"firmware-v{version}")
    pio = pio_bin()
    built = [(board, compile_board(pio, board)) for board in boards]
    _text, files = manifest_for(version, notes, repo, built)
    create_release(repo, version, notes, files)
    try:
        point_manifest(repo, version, files["manifest.json"])
    except SystemExit as err:
        print(
            "The release is up, but manifest.json on main was not updated.\n"
            "The previous manifest is still what a reader would fetch.\n"
            "Retry with: python3 firmware/scripts/release.py --point",
            file=sys.stderr,
        )
        raise err


def main(argv=None):
    parser = argparse.ArgumentParser(
        prog="release.py",
        description=(
            "Publish the current firmware version to the public releases repository. "
            "The source repositories are not tagged or pushed."
        ),
    )
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument(
        "--check",
        action="store_true",
        help="Check the version strings and the board catalog. Do not compile or upload.",
    )
    mode.add_argument(
        "--dry-run",
        action="store_true",
        help="Compile and print the manifest. Do not upload.",
    )
    mode.add_argument(
        "--point",
        action="store_true",
        help="Write the published release manifest onto main. Use this if that step failed.",
    )
    args = parser.parse_args(argv)

    version, notes = require_version()
    boards = require_catalog()
    if args.check:
        names = ", ".join(board["id"] for board in boards)
        print(f"{version}: {names}")
        return 0
    if args.point:
        point_existing(version)
        return 0

    if args.dry_run:
        pio = pio_bin()
        built = [(board, compile_board(pio, board)) for board in boards]
        text, _files = manifest_for(version, notes, release_repo(), built)
        print(text, end="")
        return 0

    publish(version, notes, boards)
    return 0


if __name__ == "__main__":
    sys.exit(main())
