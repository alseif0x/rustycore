#!/usr/bin/env python3
"""Run policy checks selected from an immutable candidate diff.

This is the PR policy acceptance, not the independent exhaustive persistence
audit. Rust changes retain the historical syntax and policy-consistency checks;
snapshot changes additionally use the focused native verifier. No result here
claims a whole-workspace persistence census.
"""

from __future__ import annotations

import argparse
from collections.abc import Sequence
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
import time


SNAPSHOT = "tools/architecture/persistence-access-snapshot.json"
PERSISTENCE_POLICY = "tools/architecture/persistence-boundary-policy.json"
PERSISTENCE_ANNOTATIONS = "tools/architecture/persistence-boundary-workflows.json"
SESSION_POLICY = "tools/architecture/session-ownership-policy.json"
CENSUS_FILES = {
    "tools/architecture/wow-world-test-census.json",
    "tools/architecture/wow-world-test-ceiling.json",
}
CENSUS_TOOLS = {
    "tools/architecture/wow_world_test_census.py",
    "tools/architecture/wow_world_test_scan.py",
    "tools/architecture/test_wow_world_test_census.py",
}
PHYSICAL_FILES = {
    "tools/architecture/physical-file-policy.json",
    "tools/architecture/physical_files.py",
    "tools/architecture/test_physical_files.py",
}
SESSION_INDEX_TOOLS = {
    "tools/architecture/session_method_index.py",
    "tools/architecture/test_session_method_index.py",
}
NET_MOVE_FILES = {
    "tools/architecture/net-move-policy.json",
    "tools/architecture/net_move.py",
    "tools/architecture/test_net_move.py",
}
PERSISTENCE_CHECKER_PREFIX = "tools/architecture/handler-contract-check/"


def git(root: Path, *arguments: str) -> bytes:
    return subprocess.check_output(["git", *arguments], cwd=root)


def repository_root(start: Path) -> Path:
    top = subprocess.check_output(
        ["git", "rev-parse", "--show-toplevel"], cwd=start
    ).decode().strip()
    return Path(top).resolve()


def changed_paths(root: Path, base_input: str) -> tuple[str, str, list[str]]:
    head = git(root, "rev-parse", "--verify", "HEAD").decode().strip()
    base = git(
        root, "rev-parse", "--verify", "--end-of-options", f"{base_input}^{{commit}}"
    ).decode().strip()
    subprocess.run(["git", "merge-base", "--is-ancestor", base, head], cwd=root, check=True)
    dirty = git(root, "status", "--porcelain=v1", "-z", "--untracked-files=all")
    if dirty:
        raise ValueError("full policy acceptance requires a clean, committed candidate")
    paths = [
        os.fsdecode(path)
        for path in git(root, "diff", "--name-only", "--no-renames", "-z", base, head, "--").split(b"\0")
        if path
    ]
    if not paths:
        raise ValueError("empty candidate diff")
    return base, head, sorted(set(paths))


def cargo_checker(*checker_arguments: str) -> list[str]:
    return [
        "cargo", "run", "--release", "--locked",
        "--manifest-path", "tools/architecture/handler-contract-check/Cargo.toml",
        "--target-dir", "target/arch-tools",
        "--bin", "session-ownership-check", "--", *checker_arguments,
    ]


def plan(root: Path, paths: Sequence[str], base: str) -> tuple[list[tuple[str, list[str]]], bool]:
    path_set = set(paths)
    rust_sources = {
        path for path in paths
        if path.startswith("crates/") and Path(path).suffix == ".rs"
    }
    world_source = any(path.startswith("crates/wow-world/src/") for path in paths)
    # Census policy measures test identities and lines. Keep test fixtures and
    # their ratchets separate from the production ownership rules below.
    world_test_fixtures = {
        path for path in rust_sources
        if path.startswith("crates/wow-world") and "/unit_tests/" in path
    }
    world_source_changed = any(path.startswith("crates/wow-world") for path in rust_sources)
    census_changed = bool(path_set & (CENSUS_FILES | CENSUS_TOOLS))
    physical_changed = bool(path_set & PHYSICAL_FILES)
    architecture_policy_only_paths = (
        CENSUS_FILES | CENSUS_TOOLS | SESSION_INDEX_TOOLS | NET_MOVE_FILES
        | {SNAPSHOT, PERSISTENCE_POLICY, PERSISTENCE_ANNOTATIONS, SESSION_POLICY}
    )
    architecture_changed = any(
        path.startswith("tools/architecture/")
        and path not in architecture_policy_only_paths
        and not path.startswith(PERSISTENCE_CHECKER_PREFIX)
        for path in paths
    )
    rules_changed = bool(
        rust_sources
        or architecture_changed
    )
    snapshot_changed = SNAPSHOT in path_set
    semantic_persistence_policy_changed = bool(
        path_set & {PERSISTENCE_POLICY, PERSISTENCE_ANNOTATIONS}
    )
    scanner_changed = any(path.startswith(PERSISTENCE_CHECKER_PREFIX) for path in paths)
    require_exhaustive = semantic_persistence_policy_changed or scanner_changed
    session_contract_changed = bool(
        rust_sources or SESSION_POLICY in path_set
    )

    commands: list[tuple[str, list[str]]] = []

    if rules_changed:
        # One native check owns physical sizes, runtime ownership, dependencies,
        # hotspots and their self-tests; do not repeat its component scans.
        commands.append((
            "Architecture and runtime policies",
            ["python3", "tools/architecture/check_architecture.py", "check", "--self-test"],
        ))
    if physical_changed:
        commands.append((
            "Physical policy regression tests",
            ["python3", "-m", "unittest", "discover", "-s", "tools/architecture", "-p", "test_physical_files.py"],
        ))
    if rust_sources or "tools/architecture/ignored_sources.py" in path_set:
        commands.append((
            "Ignored crate source check",
            ["python3", "tools/architecture/ignored_sources.py", "check"],
        ))

    if world_source or path_set & NET_MOVE_FILES:
        commands.append((
            "World net-move policy",
            ["python3", "tools/architecture/net_move.py", "check", "--base", base],
        ))
        if path_set & NET_MOVE_FILES:
            commands.append((
                "World net-move checker tests",
                ["python3", "-m", "unittest", "discover", "-s", "tools/architecture", "-p", "test_net_move.py"],
            ))
    if world_source or path_set & SESSION_INDEX_TOOLS:
        commands.append((
            "WorldSession method ownership",
            ["python3", "tools/architecture/session_method_index.py", "check"],
        ))
        if "tools/architecture/session_method_index.py" in path_set:
            commands.append((
                "WorldSession index self-test",
                ["python3", "tools/architecture/session_method_index.py", "self-test"],
            ))
    if world_source_changed or census_changed:
        census_name = (
            "Test fixture census and ceiling"
            if world_test_fixtures or census_changed
            else "Inline world-source test census and ceiling"
        )
        commands.append((
            census_name,
            ["python3", "tools/architecture/wow_world_test_census.py", "check"],
        ))
        if path_set & {
            "tools/architecture/wow_world_test_census.py",
            "tools/architecture/wow_world_test_scan.py",
        }:
            commands.append((
                "World test census self-test",
                ["python3", "tools/architecture/wow_world_test_census.py", "self-test"],
            ))
        if "tools/architecture/test_wow_world_test_census.py" in path_set:
            commands.append((
                "World test census regression tests",
                ["python3", "-m", "unittest", "discover", "-s", "tools/architecture", "-p", "test_wow_world_test_census.py"],
            ))

    if require_exhaustive:
        commands.append((
            "Exhaustive session and persistence ownership",
            cargo_checker("check"),
        ))
    elif snapshot_changed:
        verifier = Path(__file__).resolve().with_name("full_snapshot.py")
        if not verifier.is_file():
            commands.append((
                "Exhaustive session and persistence ownership",
                cargo_checker("check"),
            ))
            require_exhaustive = True
        else:
            commands.append((
                "Changed persistence snapshot proof",
                ["python3", str(verifier), "--base", base],
            ))
    if not require_exhaustive and session_contract_changed:
        commands.append((
            "Session syntax ownership",
            cargo_checker("check", "--syntax-only"),
        ))
    if not require_exhaustive and (
        session_contract_changed or snapshot_changed or semantic_persistence_policy_changed
    ):
        commands.append((
            "Snapshot-derived persistence policy consistency",
            cargo_checker("print-persistence-policy", "--from-snapshot", SNAPSHOT),
        ))
    return commands, require_exhaustive


def check_rendered_policy(root: Path, command: Sequence[str]) -> tuple[int, str]:
    result = subprocess.run(
        command,
        cwd=root,
        env={**os.environ, "CARGO_BUILD_JOBS": os.environ.get("CARGO_BUILD_JOBS", "1")},
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode:
        return result.returncode, result.stderr.strip() or result.stdout.strip()
    try:
        generated = json.loads(result.stdout)
        checked = json.loads((root / PERSISTENCE_POLICY).read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError) as error:
        return 1, f"could not parse the generated or checked persistence policy: {error}"
    if generated != checked:
        return 1, "checked persistence policy disagrees with the snapshot-derived policy"
    return 0, "checked policy matches the current snapshot and workflow annotations"


def check_policy_input_shapes(root: Path) -> str:
    expected = {
        SNAPSHOT: 3,
        PERSISTENCE_POLICY: 1,
        PERSISTENCE_ANNOTATIONS: 2,
        SESSION_POLICY: 1,
    }
    values: dict[str, dict[str, object]] = {}
    for path, version in expected.items():
        value = json.loads((root / path).read_text(encoding="utf-8"))
        if not isinstance(value, dict) or value.get("schema_version") != version:
            raise ValueError(f"{path} must have schema_version {version}")
        values[path] = value
    if values[SNAPSHOT].get("accesses") is None or not isinstance(values[SNAPSHOT]["accesses"], list):
        raise ValueError(f"{SNAPSHOT} must contain an accesses array")
    if values[SESSION_POLICY].get("persistence_access_snapshot") != SNAPSHOT:
        raise ValueError(f"{SESSION_POLICY} must reference {SNAPSHOT}")
    if not isinstance(values[PERSISTENCE_ANNOTATIONS].get("workflows"), list):
        raise ValueError(f"{PERSISTENCE_ANNOTATIONS} must contain a workflows array")
    if not isinstance(values[PERSISTENCE_POLICY].get("groups"), list):
        raise ValueError(f"{PERSISTENCE_POLICY} must contain a groups array")
    return "all four persistence/session policy inputs retain their schema and reference"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True, help="exact integration base commit or its ref")
    parser.add_argument("--plan", action="store_true", help="print selected policy checks without running them")
    options = parser.parse_args()

    try:
        root = repository_root(Path.cwd())
        base, head, paths = changed_paths(root, options.base)
        commands, require_exhaustive = plan(root, paths, base)
    except (OSError, subprocess.CalledProcessError, ValueError) as error:
        print(f"full-policy: {error}", file=sys.stderr)
        return 2

    print(f"PR policy acceptance | base {base} | head {head}", flush=True)
    print("Changed paths:", *paths, sep="\n  ", flush=True)
    if not commands:
        print("Decision: this diff changes no inputs owned by the policy checks.", flush=True)
        return 0
    for name, command in commands:
        print(f"{name}: {shlex.join(command)}", flush=True)
    if any(name == "Changed persistence snapshot proof" for name, _ in commands):
        print("Snapshot proof exit 0 accepts the exact delta; exit 2 falls back to exhaustive native inventory; exit 1 blocks.", flush=True)
    if options.plan:
        if any(
            path.startswith("crates/") and Path(path).suffix == ".rs"
            or path in {SNAPSHOT, PERSISTENCE_POLICY, PERSISTENCE_ANNOTATIONS, SESSION_POLICY}
            for path in paths
        ):
            print("Internal check: all four persistence/session policy schemas and snapshot reference", flush=True)
        return 0

    started = time.monotonic()
    environment = os.environ.copy()
    environment.setdefault("CARGO_BUILD_JOBS", "1")
    exit_code = 0
    persistence_exhaustive = require_exhaustive
    if any(
        path.startswith("crates/") and Path(path).suffix == ".rs"
        or path in {SNAPSHOT, PERSISTENCE_POLICY, PERSISTENCE_ANNOTATIONS, SESSION_POLICY}
        for path in paths
    ):
        before = time.monotonic()
        try:
            detail = check_policy_input_shapes(root)
            code = 0
        except (OSError, json.JSONDecodeError, ValueError) as error:
            detail = str(error)
            code = 1
        print(f"Policy input schemas and reference: exit {code}, {time.monotonic() - before:.3f}s ({detail})", flush=True)
        if code:
            return code
    skip_after_exhaustive = False
    for name, command in commands:
        if skip_after_exhaustive and name in {
            "Session syntax ownership", "Snapshot-derived persistence policy consistency",
        }:
            print(f"{name}: covered by the exhaustive native checker", flush=True)
            continue
        before = time.monotonic()
        if name == "Changed persistence snapshot proof":
            result = subprocess.run(command, cwd=root, env=environment)
            elapsed = time.monotonic() - before
            print(f"{name}: exit {result.returncode}, {elapsed:.3f}s", flush=True)
            if result.returncode == 2:
                full_command = cargo_checker("check")
                print(f"Unsupported fast-path delta; running exhaustive check: {shlex.join(full_command)}", flush=True)
                full_before = time.monotonic()
                full_result = subprocess.run(full_command, cwd=root, env=environment)
                print(f"Exhaustive session and persistence ownership: exit {full_result.returncode}, {time.monotonic() - full_before:.3f}s", flush=True)
                if full_result.returncode:
                    return full_result.returncode
                persistence_exhaustive = True
                skip_after_exhaustive = True
                continue
            if result.returncode:
                return result.returncode
        elif name == "Snapshot-derived persistence policy consistency":
            code, detail = check_rendered_policy(root, command)
            print(f"{name}: exit {code}, {time.monotonic() - before:.3f}s ({detail})", flush=True)
        elif name == "Exhaustive session and persistence ownership":
            result = subprocess.run(command, cwd=root, env=environment)
            code = result.returncode
            persistence_exhaustive = code == 0
            print(f"{name}: exit {code}, {time.monotonic() - before:.3f}s", flush=True)
        else:
            result = subprocess.run(command, cwd=root, env=environment)
            code = result.returncode
            print(f"{name}: exit {code}, {time.monotonic() - before:.3f}s", flush=True)
        if code:
            exit_code = code
            print(f"Policy acceptance stopped after {time.monotonic() - started:.3f}s.", flush=True)
            return exit_code

    if (
        git(root, "rev-parse", "--verify", "HEAD").decode().strip() != head
        or git(root, "status", "--porcelain=v1", "-z", "--untracked-files=all")
    ):
        print("full-policy: candidate HEAD changed during acceptance", file=sys.stderr)
        return 2
    if any(
        path.startswith("crates/") and Path(path).suffix == ".rs"
        or path == SESSION_POLICY
        for path in paths
    ):
        print(
            "Coverage: exhaustive session and persistence inventory passed."
            if persistence_exhaustive
            else "Coverage: syntax ownership and snapshot-derived policy consistency passed; "
                 "the exhaustive whole-workspace persistence census is a separate audit and is not claimed.",
            flush=True,
        )
    print(f"PR policy acceptance: exit {exit_code}, {time.monotonic() - started:.3f}s", flush=True)
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
