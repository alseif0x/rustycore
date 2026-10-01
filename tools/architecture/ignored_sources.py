#!/usr/bin/env python3
"""Reject git-ignored source files under crates/ (#1241 F3-6b).

A source file that a .gitignore rule hides can be mounted by a committed module
(`#[path = ".../f3_shims.rs"] mod f3_shims;`) and still pass every local build,
because it exists in the working tree. A fresh clone then lacks it. F3-4 shipped
exactly that: the repo-wide `skills/` rule hid
`crates/wow-world/unit_tests/session/progression/skills/f3_shims.rs`.

`check` lists `git ls-files --others --ignored --exclude-standard -- crates/`,
keeps the physical inventory's SOURCE_SUFFIXES, skips build output under any
`target/` directory, and fails when anything remains, printing the matching rule
from `git check-ignore -v` for each path. Read-only (GIT_OPTIONAL_LOCKS=0).
"""
from __future__ import annotations

import argparse
import os
import pathlib
import subprocess
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import physical_files as physical  # noqa: E402

SCOPE = "crates/"


class IgnoredSourcesError(RuntimeError):
    """A git failure; reported with exit status 2."""


def git(root: pathlib.Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[bytes]:
    environment = dict(os.environ, GIT_OPTIONAL_LOCKS="0")
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, env=environment)
    if check and result.returncode != 0:
        detail = result.stderr.decode("utf-8", "replace").strip()
        raise IgnoredSourcesError(f"git {' '.join(args)} failed ({result.returncode}): {detail}")
    return result


def ignored_sources(root: pathlib.Path) -> list[str]:
    listed = git(root, "ls-files", "-z", "--others", "--ignored", "--exclude-standard", "--", SCOPE).stdout
    paths = [path for path in listed.decode("utf-8", "surrogateescape").split("\0") if path]
    return sorted(
        path for path in paths
        if pathlib.PurePosixPath(path).suffix in physical.SOURCE_SUFFIXES
        and "target" not in pathlib.PurePosixPath(path).parts
    )


def matching_rule(root: pathlib.Path, path: str) -> str:
    result = git(root, "check-ignore", "-v", "--no-index", "--", path, check=False)
    return result.stdout.decode("utf-8", "replace").strip().split("\t", 1)[0] or "(rule not reported)"


def check(root: pathlib.Path) -> int:
    paths = ignored_sources(root)
    if not paths:
        print(f"ignored sources: PASS (no git-ignored source files under {SCOPE})")
        return 0
    print(f"ignored sources: FAIL ({len(paths)} git-ignored source file(s) under {SCOPE}; "
          "a fresh clone would lack them)")
    for path in paths:
        print(f"  {path}  <- {matching_rule(root, path)}")
    return 1


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("check", help="fail on git-ignored source files under crates/")
    run.add_argument("--root", default=None, help="repository root (default: this checkout)")
    args = parser.parse_args(argv)
    try:
        start = pathlib.Path(args.root) if args.root else pathlib.Path(__file__).resolve().parent
        root = pathlib.Path(git(start, "rev-parse", "--show-toplevel").stdout.decode().strip())
        return check(root)
    except IgnoredSourcesError as error:
        print(f"ignored sources: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
