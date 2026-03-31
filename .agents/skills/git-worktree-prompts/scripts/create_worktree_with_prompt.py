#!/usr/bin/env python3
"""
Create a sibling git worktree and seed a root prompt file.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

READ_ONCE_HEADER = (
    "Read this file once, follow it, then delete this file before making other edits."
)


def run_git(repo_root: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=repo_root,
        text=True,
        capture_output=True,
        check=False,
    )


def repo_root_from(path: str | None) -> Path:
    cwd = Path(path).resolve() if path else Path.cwd()
    result = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        cwd=cwd,
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        message = result.stderr.strip() or "failed to resolve git repo root"
        raise SystemExit(f"error: {message}")
    return Path(result.stdout.strip()).resolve()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Create a sibling git worktree and write a prompt.txt file.",
    )
    parser.add_argument(
        "--repo-root",
        help="Optional path inside the target git repository. Defaults to the current working directory.",
    )
    parser.add_argument(
        "--name",
        required=True,
        help="Sibling worktree directory name.",
    )
    parser.add_argument(
        "--branch",
        required=True,
        help="Branch to create or check out in the worktree.",
    )
    parser.add_argument(
        "--base-ref",
        default="HEAD",
        help="Base ref for a new branch. Defaults to HEAD.",
    )
    parser.add_argument(
        "--path",
        help="Explicit worktree path. Defaults to <repo-parent>/<name>.",
    )
    parser.add_argument(
        "--prompt-name",
        default="prompt.txt",
        help="Prompt filename written at the worktree root. Defaults to prompt.txt.",
    )
    parser.add_argument(
        "--force-prompt",
        action="store_true",
        help="Overwrite the prompt file if it already exists.",
    )
    parser.add_argument(
        "--checkout-existing",
        action="store_true",
        help="Check out an existing branch instead of creating a new one.",
    )
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument(
        "--prompt-file",
        help="Read prompt contents from this file.",
    )
    source.add_argument(
        "--prompt-stdin",
        action="store_true",
        help="Read prompt contents from stdin.",
    )
    return parser.parse_args()


def read_prompt(args: argparse.Namespace) -> str:
    if args.prompt_file:
        prompt = Path(args.prompt_file).read_text()
    else:
        prompt = sys.stdin.read()
    prompt = prompt.strip()
    if not prompt:
        raise SystemExit("error: prompt content is empty")
    if not prompt.startswith(READ_ONCE_HEADER):
        prompt = f"{READ_ONCE_HEADER}\n\n{prompt}"
    return prompt + "\n"


def ensure_worktree(repo_root: Path, worktree_path: Path, args: argparse.Namespace) -> None:
    if worktree_path.exists():
        raise SystemExit(f"error: worktree path already exists: {worktree_path}")

    if args.checkout_existing:
        command = ["worktree", "add", str(worktree_path), args.branch]
    else:
        command = ["worktree", "add", str(worktree_path), "-b", args.branch, args.base_ref]

    result = run_git(repo_root, *command)
    if result.returncode != 0:
        message = result.stderr.strip() or result.stdout.strip() or "git worktree add failed"
        raise SystemExit(f"error: {message}")


def write_prompt(worktree_path: Path, prompt_name: str, prompt_text: str, force: bool) -> Path:
    prompt_path = worktree_path / prompt_name
    if prompt_path.exists() and not force:
        raise SystemExit(f"error: prompt file already exists: {prompt_path}")
    prompt_path.write_text(prompt_text)
    return prompt_path


def main() -> int:
    args = parse_args()
    repo_root = repo_root_from(args.repo_root)
    worktree_path = Path(args.path).resolve() if args.path else repo_root.parent / args.name
    prompt_text = read_prompt(args)

    ensure_worktree(repo_root, worktree_path, args)
    prompt_path = write_prompt(worktree_path, args.prompt_name, prompt_text, args.force_prompt)

    print(f"repo_root={repo_root}")
    print(f"worktree_path={worktree_path}")
    print(f"branch={args.branch}")
    print(f"prompt_path={prompt_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
