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
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from pprint import pprint
from typing import Literal, get_args

from dotenv import load_dotenv
from huggingface_hub import HfApi, whoami

REPO_ID_HF = "daxida/wty-release"
REPO_HF = f"https://huggingface.co/datasets/{REPO_ID_HF}"
REPO_ID_GH = "https://github.com/daxida/wty"

type DictTy = Literal["main", "ipa", "ipa-merged", "glossary"]
type CmdTy = Literal["publish", "squash"]

CMD_CHOICES = get_args(CmdTy.__value__)


@dataclass
class Args:
    cmd: CmdTy


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

        # Stage structure
        self.stage = self.release / "stage"
        self.version = self.stage / "versions" / release_version()
        self.latest = self.stage / "latest"

    def setup(self) -> None:
        self.release.mkdir(exist_ok=True)

    def check_dict_dir(self) -> None:
        if not self.dictionary.exists() or not any(self.dictionary.iterdir()):
            print(f"No files found in {self.dictionary}")
            exit(1)


PM = PathManager(Path("data"))
"""Global to simplify the argument passing. Should be read-only."""


def double_check(msg: str = "") -> None:
    if msg:
        print(msg)
    if input("Proceed? [y/n] ") != "y":
        print("Exiting.")
        exit(1)


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


def prepare_stage() -> None:
    """Take the release folder created with wty and structure it to comply with
    huggingface upload_large_folder.

    The resulting layout is:

        stage/
        ├── versions/
        │   └── {version}/
        │       ├── dict/
        │       ├── index/
        │       ├── README.md
        │       └── log.txt
        └── latest/
            ├── dict/
            ├── index/
            ├── README.md
            └── log.txt

    We copy (not move) files so the local release directory remains intact.

    The README and log shown on the Hugging Face repo root are handled
    separately and do not require upload_large_folder.
    """
    PM.stage.mkdir()  # Fail if exists

    for destination in (PM.version, PM.latest):
        print(f"[stage] copying release to {destination}...")
        shutil.copytree(str(PM.dictionary), destination / "dict")
        shutil.copytree(str(PM.index), destination / "index")


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


# https://huggingface.co/new-dataset
# https://huggingface.co/settings/tokens
def upload_to_huggingface() -> None:
    PM.check_dict_dir()

    login_to_huggingface()

    dict_dir = PM.dictionary
    _, size = stats(dict_dir)
    stage_dir = PM.stage
    version = release_version()
    git_cmd = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=".")
    commit_sha = git_cmd.decode().strip()
    commit_sha_short = commit_sha[:7]

    kwargs = dict(
        folder_path=str(stage_dir),
        repo_id=REPO_ID_HF,
        repo_type="dataset",
    )

    print()
    print(commit_sha_short, commit_sha)
    pprint(kwargs)
    print(f"{version=}")
    print()
    print(f"Upload {dict_dir} ({size}) to {REPO_ID_HF}?")
    double_check()

    api = HfApi()

    # Upload dict + index (stage folder)
    prepare_stage()
    # TODO: deprecated, use upload_folder
    api.upload_large_folder(**kwargs)  # type: ignore
    print(f"Upload complete @ https://huggingface.co/datasets/{REPO_ID_HF}")

    # Upload README and logs at root, and also to latest and versions folders.
    readme_path = PM.readme
    update_readme_local(readme_path, commit_sha, version)

    for folder_in_repo in ("", f"versions/{release_version()}", "latest"):
        api.upload_file(
            path_or_fileobj=str(readme_path),
            path_in_repo=f"{folder_in_repo}/README.md",
            repo_id=REPO_ID_HF,
            repo_type="dataset",
            commit_message=f"[{version}] update README",
        )
        print(f"Uploaded README @ {folder_in_repo or 'root'}")


def super_squash() -> None:
    """Squash the huggingface repo history.

    Huggingface will complain once we reach a certain amount of commits.
    Since the commits are mangled due to upload_large_folder anyway, we don't care
    too much about the history, and they claim this speeds things up...
    """
    login_to_huggingface()
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


def pre_stage() -> None:
    """Create a release folder, then move "/dict" and "/index" into it"""
    PM.release.mkdir(exist_ok=True)
    # /dict and /index should be at release parent folder
    for folder in ("dict", "index"):
        src = PM.release.parent / folder
        dst = PM.release / folder
        src.rename(dst)
        print(f"[pre-stage] moved: {src} -> {dst}")


def parse_args() -> Args:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "cmd",
        nargs="?",
        default="publish",
        choices=CMD_CHOICES,
        help="Command to run (default: publish)",
    )
    args = parser.parse_args()
    return Args(cmd=args.cmd)


def main() -> None:
    args = parse_args()
    match args.cmd:
        case "publish":
            pre_stage()
            upload_to_huggingface()
        case "squash":
            super_squash()


if __name__ == "__main__":
    main()
