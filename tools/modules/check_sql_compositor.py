#!/usr/bin/env python3
"""Compile the SQL-mode compositor with the shipped example module (#1468).

`rustycore-module sync` writes a different `crates/world-modules/src/main.rs`
once a composed module ships SQL: it calls `world_server::run_with_composition`
and embeds `modules.lock.toml` with `include_str!`. No checked-in tree contains
that shape (the repository composes zero modules), so nothing else compiles it.
This check installs `tools/modules/examples/milestones` under the ignored
`modules/`, syncs, adds the workspace member, runs the example's tests and
`cargo check -p world-modules`, and then restores every file it touched.

Tracked files are backed up under the checkout's git directory before the
first change. A run killed before it could restore leaves that backup behind;
the next run restores it first, so an interruption never leaves the checkout
composed.
"""
from __future__ import annotations

import argparse
import re
import shutil
import signal
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
EXAMPLE = "tools/modules/examples/milestones"
CHECKOUT = "modules/milestones"
PACKAGE = "example-milestones"
# Every tracked file install/sync/cargo can rewrite.
TOUCHED = (
    "Cargo.toml",
    "Cargo.lock",
    "modules.lock.toml",
    "crates/world-modules/Cargo.toml",
    "crates/world-modules/src/main.rs",
)
ABSENT_SUFFIX = ".absent"
COMPLETE = "backup-complete"


class CheckError(Exception):
    pass


def backup_directory(root: Path) -> Path:
    """Per checkout (a worktree has its own git directory), outside the tree."""
    result = subprocess.run(
        ["git", "rev-parse", "--git-path", "rustycore-sql-compositor-backup"],
        cwd=root, capture_output=True, text=True, check=False,
    )
    if result.returncode != 0:
        raise CheckError(f"{root} is not a git checkout: {result.stderr.strip()}")
    path = Path(result.stdout.strip())
    return path if path.is_absolute() else root / path


def save(root: Path, backup: Path) -> None:
    backup.mkdir(parents=True)
    for relative in TOUCHED:
        source = root / relative
        target = backup / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        if source.is_file():
            shutil.copy2(source, target)
        else:
            target.with_name(target.name + ABSENT_SUFFIX).touch()
    # Written last: a backup without it was interrupted before any change.
    (backup / COMPLETE).touch()


def restore(root: Path, backup: Path) -> None:
    if (backup / COMPLETE).is_file():
        for relative in TOUCHED:
            saved = backup / relative
            target = root / relative
            if saved.is_file():
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(saved, target)
            elif saved.with_name(saved.name + ABSENT_SUFFIX).is_file() and target.exists():
                target.unlink()
                parent = target.parent
                while parent != root and parent.is_dir() and not any(parent.iterdir()):
                    parent.rmdir()
                    parent = parent.parent
        shutil.rmtree(root / CHECKOUT, ignore_errors=True)
    shutil.rmtree(backup)


def add_member(root: Path) -> None:
    manifest = root / "Cargo.toml"
    text = manifest.read_text(encoding="utf-8")
    updated, count = re.subn(
        r"(?m)^members = \[", f'members = [\n    "{CHECKOUT}",', text, count=1
    )
    if count != 1:
        raise CheckError("the root Cargo.toml has no `members = [` line")
    manifest.write_text(updated, encoding="utf-8")


def run(command: list[str], root: Path) -> None:
    print(f"$ {' '.join(command)}", flush=True)
    result = subprocess.run(command, cwd=root, check=False)
    if result.returncode != 0:
        raise CheckError(f"`{' '.join(command)}` exited {result.returncode}")


def compose_and_compile(root: Path, cargo: str, jobs: int) -> None:
    manager = [sys.executable, str(root / "tools/modules/rustycore-module")]
    run([*manager, "install", "--path", EXAMPLE], root)
    run([*manager, "sync"], root)
    main_rs = (root / "crates/world-modules/src/main.rs").read_text(encoding="utf-8")
    for expected in ("world_server::run_with_composition(", 'include_str!("../../../modules.lock.toml")'):
        if expected not in main_rs:
            raise CheckError(f"the synced compositor does not contain {expected!r}: not SQL mode")
    add_member(root)
    # The member is new, so Cargo.lock must change: --offline, not --locked.
    run([cargo, "test", "--offline", "-p", PACKAGE, "--jobs", str(jobs)], root)
    run([cargo, "check", "--offline", "-p", "world-modules", "--jobs", str(jobs)], root)
    run([*manager, "check"], root)


def status(root: Path) -> str:
    return subprocess.run(
        ["git", "status", "--porcelain=v1", "--untracked-files=normal"],
        cwd=root, capture_output=True, text=True, check=True,
    ).stdout


def raise_on_signal(signum: int, _frame: object) -> None:
    raise CheckError(f"interrupted by signal {signum}; restoring")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--jobs", type=int, default=1)
    parser.add_argument("--cargo", default="cargo", help="the Cargo executable (tests stub it)")
    parser.add_argument("--root", type=Path, default=REPO_ROOT)
    args = parser.parse_args(argv)
    root = args.root.resolve()
    try:
        backup = backup_directory(root)
        if backup.exists():
            print(f"restoring the checkout from an interrupted run ({backup})", flush=True)
            restore(root, backup)
        if (root / CHECKOUT).exists():
            raise CheckError(
                f"{CHECKOUT} already exists; this check installs the example there and "
                "would not restore your checkout — move it away first"
            )
        before = status(root)
        for signum in (signal.SIGTERM, signal.SIGHUP, signal.SIGINT):
            signal.signal(signum, raise_on_signal)
        save(root, backup)
        failure: CheckError | None = None
        try:
            compose_and_compile(root, args.cargo, args.jobs)
        except CheckError as error:
            failure = error
        finally:
            restore(root, backup)
        if status(root) != before:
            raise CheckError("the checkout differs after the restore:\n" + status(root))
        if failure is not None:
            raise failure
    except CheckError as error:
        print(f"sql compositor check: FAIL: {error}", file=sys.stderr)
        return 1
    print("sql compositor check: PASS (example.milestones composed in SQL mode; "
          f"{PACKAGE} tests and world-modules check passed; checkout restored)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
