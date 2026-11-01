#!/usr/bin/env python3
"""Compile the SQL-mode compositor with the shipped example module (#1468).

`rustycore-module sync` writes a different `crates/world-modules/src/main.rs`
once a composed module ships SQL: it calls `world_server::run_with_composition`
and embeds `modules.lock.toml` with `include_str!`. No checked-in tree contains
that shape (the repository composes zero modules), so nothing else compiles it.
This check installs `tools/modules/examples/milestones` under the ignored
`modules/`, syncs, adds the workspace member, runs the example's tests and
`cargo check -p world-modules`, and then restores every file it touched.

Before the first change it saves the touched files, HEAD and their hashes
under the checkout's git directory; once composed, it records the composed
hashes and a digest of the installed checkout. A run killed before it could
restore leaves that backup behind. The next run restores from it only when
nothing changed since: HEAD is the recorded one, every file is byte-identical
to its composed (or original) state and `modules/milestones` is the recorded
install. Anything else is refused with instructions, so a later commit or a
user's own `modules/milestones` is never overwritten.
"""
from __future__ import annotations

import argparse
import hashlib
import json
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
STATE = "state.json"
FILES = "files"


class CheckError(Exception):
    pass


def git(root: Path, *args: str) -> str:
    result = subprocess.run(["git", *args], cwd=root, capture_output=True, text=True, check=False)
    if result.returncode != 0:
        raise CheckError(f"git {' '.join(args)} failed ({result.stderr.strip()})")
    return result.stdout.strip()


def backup_directory(root: Path) -> Path:
    """Per checkout (a worktree has its own git directory), outside the tree."""
    path = Path(git(root, "rev-parse", "--git-path", "rustycore-sql-compositor-backup"))
    return path if path.is_absolute() else root / path


def file_hash(path: Path) -> str | None:
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else None


def tree_digest(path: Path) -> str | None:
    if not path.exists():
        return None
    digest = hashlib.sha256()
    for item in sorted(path.rglob("*")):
        if item.is_file() and "__pycache__" not in item.parts:
            digest.update(str(item.relative_to(path)).encode() + b"\0")
            digest.update(hashlib.sha256(item.read_bytes()).digest())
    return digest.hexdigest()


def hashes(root: Path) -> dict[str, str | None]:
    return {relative: file_hash(root / relative) for relative in TOUCHED}


def read_state(backup: Path) -> dict | None:
    try:
        return json.loads((backup / STATE).read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None


def write_state(backup: Path, state: dict) -> None:
    temporary = backup / (STATE + ".tmp")
    temporary.write_text(json.dumps(state, indent=2, sort_keys=True), encoding="utf-8")
    temporary.replace(backup / STATE)


def save(root: Path, backup: Path) -> dict:
    """Back up the touched files; the state file, written last, marks it usable."""
    backup.mkdir(parents=True)
    for relative in TOUCHED:
        source = root / relative
        if source.is_file():
            target = backup / FILES / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
    state = {"head": git(root, "rev-parse", "HEAD"), "original": hashes(root),
             "composed": None, "checkout": None}
    write_state(backup, state)
    return state


def record_composed(root: Path, backup: Path, state: dict) -> None:
    state["composed"] = hashes(root)
    state["checkout"] = tree_digest(root / CHECKOUT)
    write_state(backup, state)


def put_back(root: Path, backup: Path, state: dict) -> None:
    for relative in TOUCHED:
        target = root / relative
        if state["original"][relative] is not None:
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(backup / FILES / relative, target)
        elif target.exists():
            target.unlink()
            parent = target.parent
            while parent != root and parent.is_dir() and not any(parent.iterdir()):
                parent.rmdir()
                parent = parent.parent
    shutil.rmtree(root / CHECKOUT, ignore_errors=True)
    shutil.rmtree(backup)


def stale_backup_problems(root: Path, state: dict | None) -> list[str]:
    """Why a backup left by another run may not be restored automatically."""
    if state is None:
        return []  # It saved nothing usable: the tree was not changed yet.
    if state.get("composed") is None:
        return ["the interrupted run stopped before it recorded the composed state, "
                "so its changes cannot be told apart from yours"]
    problems = []
    head = git(root, "rev-parse", "HEAD")
    if head != state["head"]:
        problems.append(f"HEAD is {head}, not {state['head']} as when the backup was taken")
    for relative, current in hashes(root).items():
        if current not in (state["composed"][relative], state["original"][relative]):
            problems.append(f"{relative} changed since the interrupted run composed it")
    checkout = tree_digest(root / CHECKOUT)
    if checkout is not None and checkout != state["checkout"]:
        problems.append(f"{CHECKOUT} is not the example the interrupted run installed")
    return problems


def recover(root: Path, backup: Path) -> None:
    """Undo an interrupted run, or refuse when that could overwrite newer work."""
    state = read_state(backup)
    problems = stale_backup_problems(root, state)
    if problems:
        raise CheckError(
            f"an interrupted run left a backup in {backup}, and restoring it could overwrite "
            "newer work: " + "; ".join(problems) + ". Nothing was changed. Restore by hand: "
            f"the original files are under {backup / FILES} (a file missing there did not "
            f"exist), and {CHECKOUT} was installed by that run; then delete {backup}"
        )
    print(f"restoring the checkout from an interrupted run ({backup})", flush=True)
    if state is None:
        shutil.rmtree(backup)
        return
    put_back(root, backup, state)


def refuse_existing_checkout(root: Path) -> None:
    if (root / CHECKOUT).exists():
        raise CheckError(
            f"{CHECKOUT} already exists; this check installs the example there and "
            "would not restore your checkout — move it away first"
        )


def add_member(root: Path) -> None:
    manifest = root / "Cargo.toml"
    text = manifest.read_text(encoding="utf-8")
    updated, count = re.subn(
        r"(?m)^members = \[", f'members = [\n    "{CHECKOUT}",', text, count=1
    )
    if count != 1:
        raise CheckError("the root Cargo.toml has no `members = [` line")
    manifest.write_text(updated, encoding="utf-8")


def run(command: list[str], root: Path, discard_stdout: bool = False) -> None:
    print(f"$ {' '.join(command)}", flush=True)
    result = subprocess.run(
        command, cwd=root, check=False, stdout=subprocess.DEVNULL if discard_stdout else None
    )
    if result.returncode != 0:
        raise CheckError(f"`{' '.join(command)}` exited {result.returncode}")


def compose_and_compile(root: Path, backup: Path, state: dict, cargo: str, jobs: int) -> None:
    manager = [sys.executable, str(root / "tools/modules/rustycore-module")]
    run([*manager, "install", "--path", EXAMPLE], root)
    run([*manager, "sync"], root)
    main_rs = (root / "crates/world-modules/src/main.rs").read_text(encoding="utf-8")
    for expected in ("world_server::run_with_composition(", 'include_str!("../../../modules.lock.toml")'):
        if expected not in main_rs:
            raise CheckError(f"the synced compositor does not contain {expected!r}: not SQL mode")
    add_member(root)
    # The member is new, so Cargo.lock must change: --offline, not --locked.
    # Resolve it once here, so the composed state recorded next is final.
    run([cargo, "metadata", "--offline", "--format-version", "1"], root, discard_stdout=True)
    record_composed(root, backup, state)
    run([cargo, "test", "--offline", "-p", PACKAGE, "--jobs", str(jobs)], root)
    run([cargo, "check", "--offline", "-p", "world-modules", "--jobs", str(jobs)], root)
    run([*manager, "check"], root)


def status(root: Path) -> str:
    return git(root, "status", "--porcelain=v1", "--untracked-files=normal")


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
        # Decide about modules/milestones before restoring anything: without a
        # backup it is the operator's; with one, `recover` removes it only when
        # it is the exact install the interrupted run recorded.
        if not backup.exists():
            refuse_existing_checkout(root)
        else:
            recover(root, backup)
            refuse_existing_checkout(root)
        before = status(root)
        for signum in (signal.SIGTERM, signal.SIGHUP, signal.SIGINT):
            signal.signal(signum, raise_on_signal)
        state = save(root, backup)
        failure: CheckError | None = None
        try:
            compose_and_compile(root, backup, state, args.cargo, args.jobs)
        except CheckError as error:
            failure = error
        finally:
            # This process was the only writer since `save`, and modules/milestones
            # did not exist then.
            put_back(root, backup, state)
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
