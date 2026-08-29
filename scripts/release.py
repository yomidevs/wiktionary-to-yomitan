"""Publish a release made with `wty release` to huggingface.

Uploading to the hub requires:
pip install python-dotenv huggingface-hub

---

To modify the huggingface repo:
git clone https://huggingface.co/datasets/daxida/wty-release
...
changes
...
git push
(when it says enter password, actually type the token...)
"""

import argparse
import datetime
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from pprint import pprint
from typing import Literal

from dotenv import load_dotenv
from huggingface_hub import HfApi, whoami

REPO_ID_HF = "daxida/wty-release"
REPO_HF = f"https://huggingface.co/datasets/{REPO_ID_HF}"
REPO_ID_GH = "https://github.com/daxida/wty"

type DictTy = Literal["main", "ipa", "ipa-merged", "glossary"]
type CmdTy = Literal["publish", "squash", "tag"]
type TagCmdTy = Literal["list", "create", "delete"]


@dataclass
class Args:
    cmd: CmdTy
    tag_cmd: TagCmdTy | None
    tag: str | None
    skip_stage: bool


def release_version() -> str:
    """The version of the release.

    Different from the crate semantic version. This uses calver.
    """
    return datetime.datetime.now().strftime("%Y-%m-%d")


class PathManager:
    def __init__(self, root_dir: Path) -> None:
        self.root_dir = root_dir

        self.release = self.root_dir / "release"
        self.dictionary = self.release / "dict"  # self.dict has messed highlighting
        self.index = self.release / "index"
        self.readme = self.release / "README.md"
        self.download = self.release / "kaikki"

        # These are at the "github repo root"
        self.assets = Path("assets")
        self.languages_json = self.assets / "languages.json"
        self.log = Path("log.txt")

    def setup(self) -> None:
        self.release.mkdir(exist_ok=True)

    def check_release_dirs(self) -> None:
        for folder in (self.dictionary, self.index):
            if not folder.exists() or not any(folder.iterdir()):
                print(f"No files found in {folder}")
                sys.exit(1)


PM = PathManager(Path("data"))
"""Global to simplify the argument passing. Should be read-only."""


def double_check(msg: str = "") -> None:
    if msg:
        print(msg)
    if input("Proceed? [y/n] ") != "y":
        print("Exiting.")
        sys.exit(1)


def human_size(size_bytes: float, precision: int = 2) -> str:
    for unit in ("B", "KB", "MB"):
        if size_bytes < 1024:
            return f"{size_bytes:.{precision}f} {unit}"
        size_bytes /= 1024
    return f"{size_bytes:.{precision}f} GB"


def stats(
    path: Path,
    *,
    file_pattern: str | None = None,
    endswith: str | None = None,
) -> tuple[int, str]:
    n_files = 0
    size_files = 0
    for f in path.rglob("*"):
        if f.is_file():
            if file_pattern is not None and not re.match(file_pattern, f.name):
                continue
            if endswith is not None and not f.name.endswith(endswith):
                continue
            n_files += 1
            size_files += f.stat().st_size
    return n_files, human_size(size_files)


def login_to_huggingface() -> None:
    try:
        # Requires an ".env" file with
        # HF_TOKEN="hf_..."
        load_dotenv()
        user_info = whoami()
        print(f"✓ Successfully logged in as: {user_info['name']}")
    except Exception as e:
        print(f"✗ Login failed: {e}")
        sys.exit(1)


def upload_release(api: HfApi, version: str) -> None:
    """Upload dict + index to the latest folder.

    `latest` is what the dictionary indexes point at to check for updates, so the
    path is fixed: it is baked into every dictionary already installed in yomitan
    (see docs/update.md). Older releases used to be a `versions/{version}` copy of
    these same files; they are git tags now, so we upload one copy instead of two.

    The resulting layout is:

        latest/
        ├── dict/
        └── index/

    The README of each folder is uploaded separately, see upload_to_huggingface.
    """
    for folder, source in (("dict", PM.dictionary), ("index", PM.index)):
        destination = f"latest/{folder}"
        print(f"[upload] {source} -> {destination}")
        api.upload_folder(
            folder_path=str(source),
            path_in_repo=destination,
            repo_id=REPO_ID_HF,
            repo_type="dataset",
            commit_message=f"[{version}] upload {destination}",
        )
        print(f"[upload] complete @ {destination}")


def tag_release(api: HfApi, version: str) -> None:
    """Tag the release, replacing the old `versions/{version}` copy.

    NOTE: tags can be deleted via the CLI: hf repos tag delete ...
    """
    api.create_tag(
        REPO_ID_HF,
        tag=version,
        repo_type="dataset",
        tag_message=f"wty release {version}",
        # A publish can be resumed with --skip-stage, so the tag may exist already.
        exist_ok=True,
    )
    print(f"Tagged release @ {REPO_HF}/tree/{version}")


def list_tags() -> None:
    """List the release tags of the hf repo."""
    refs = HfApi().list_repo_refs(REPO_ID_HF, repo_type="dataset")
    for ref in refs.tags:
        print(f"{ref.name}\t{ref.target_commit}")
    if not refs.tags:
        print(f"No tags in {REPO_ID_HF}")


def delete_tag(version: str) -> None:
    """Delete a release tag of the hf repo."""
    print()
    print(f"Delete the tag {version} of {REPO_ID_HF}?")
    double_check()

    HfApi().delete_tag(REPO_ID_HF, tag=version, repo_type="dataset")
    print(f"Deleted tag {version}")


def create_tag(version: str) -> None:
    """Tag a release of the hf repo, without uploading anything."""
    print()
    print(f"Tag {REPO_ID_HF} as {version}, without uploading?")
    double_check()

    tag_release(HfApi(), version)


# https://huggingface.co/new-dataset
# https://huggingface.co/settings/tokens
def upload_to_huggingface() -> None:
    PM.check_release_dirs()

    dict_dir = PM.dictionary
    _, size = stats(dict_dir)
    version = release_version()
    git_cmd = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=".")
    commit_sha = git_cmd.decode().strip()
    commit_sha_short = commit_sha[:7]

    print()
    print(commit_sha_short, commit_sha)
    pprint({"repo_id": REPO_ID_HF, "repo_type": "dataset"})
    print(f"{version=}")
    print()
    print(f"Upload {dict_dir} ({size}) to {REPO_ID_HF}?")
    double_check()

    api = HfApi()

    upload_release(api, version)
    print(f"Upload complete @ https://huggingface.co/datasets/{REPO_ID_HF}")

    # Upload README and logs at root, and also to the latest folder.
    readme_path = PM.readme
    update_readme_local(readme_path, commit_sha, version)

    for folder_in_repo in ("", "latest"):
        api.upload_file(
            path_or_fileobj=str(readme_path),
            path_in_repo=f"{folder_in_repo}/README.md",
            repo_id=REPO_ID_HF,
            repo_type="dataset",
            commit_message=f"[{version}] update README",
        )
        print(f"Uploaded README @ {folder_in_repo or 'root'}")

    tag_release(api, version)


def super_squash() -> None:
    """Squash the huggingface repo history.

    Huggingface will complain once we reach a certain amount of commits.
    Since the commits are mangled due to upload_folder anyway, we don't care
    too much about the history, and they claim this speeds things up...
    """
    api = HfApi()
    api.super_squash_history(
        repo_id=REPO_ID_HF,
        repo_type="dataset",
    )


def update_readme_local(readme_path: Path, commit_sha: str, version: str) -> None:
    """Write the README of the huggingface repo @ readme_path."""
    commit_sha_short = commit_sha[:7]
    commit_sha_link = f"{REPO_ID_GH}/commit/{commit_sha}"
    logs_link = f"{REPO_HF}/blob/main/log.txt"

    readme_content = f"""---
license: cc-by-sa-4.0
---
⚠️ **This dataset is automatically uploaded.**

For source code and issue tracking, visit the GitHub repo at [wty]({REPO_ID_GH})

version: {version}

commit: [{commit_sha_short}]({commit_sha_link})

logs: [link]({logs_link})
"""

    readme_path.write_text(readme_content, encoding="utf-8")


def stage() -> None:
    """Create a release folder, then move "/dict" and "/index" into it"""
    PM.release.mkdir(exist_ok=True)
    # /dict and /index should be at release parent folder
    for folder in ("dict", "index"):
        src = PM.release.parent / folder
        dst = PM.release / folder
        if not src.exists() and dst.exists():
            print(
                f"[stage] already moved: {dst}. Use --skip-stage to resume a publish."
            )
            sys.exit(1)
        src.rename(dst)
        print(f"[stage] moved: {src} -> {dst}")


def parse_args() -> Args:
    parser = argparse.ArgumentParser()
    parser.set_defaults(skip_stage=False, tag_cmd=None, tag=None)
    sub = parser.add_subparsers(dest="cmd", required=True, metavar="command")

    publish = sub.add_parser("publish", help="Upload the release, then tag it")
    publish.add_argument(
        "--skip-stage",
        action="store_true",
        help="Do not move /dict and /index into the release folder (they are already there)",
    )

    sub.add_parser("squash", help="Squash the history of the hf repo")

    tag = sub.add_parser("tag", help="Manage the release tags of the hf repo")
    tag_sub = tag.add_subparsers(dest="tag_cmd", required=True, metavar="command")
    tag_sub.add_parser("list", help="List tags for the repo")
    for name in ("create", "delete"):
        tag_cmd = tag_sub.add_parser(name, help=f"{name.title()} a tag for the repo")
        tag_cmd.add_argument(
            "tag",
            nargs="?",
            default=None,
            help=f"Tag to {name} (default: the current version)",
        )

    args = parser.parse_args()
    return Args(
        cmd=args.cmd,
        tag_cmd=args.tag_cmd,
        skip_stage=args.skip_stage,
        tag=args.tag,
    )


def main() -> None:
    args = parse_args()

    login_to_huggingface()

    match args.cmd:
        case "publish":
            if not args.skip_stage:
                stage()
            upload_to_huggingface()
        case "squash":
            super_squash()
        case "tag":
            version = args.tag or release_version()
            match args.tag_cmd:
                case "list":
                    list_tags()
                case "create":
                    create_tag(version)
                case "delete":
                    delete_tag(version)


if __name__ == "__main__":
    main()
