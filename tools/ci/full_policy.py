#!/usr/bin/env python3
"""Run the source-bound policy suite for a committed full-validation diff."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import shlex
import subprocess
import sys


SNAPSHOT = "tools/architecture/persistence-access-snapshot.json"
PERSISTENCE_POLICY = "tools/architecture/persistence-boundary-policy.json"
WORKFLOWS = "tools/architecture/persistence-boundary-workflows.json"
SESSION_POLICY = "tools/architecture/session-ownership-policy.json"
POLICY_FILES = {SNAPSHOT, PERSISTENCE_POLICY, WORKFLOWS, SESSION_POLICY}
CHECKER = "tools/architecture/handler-contract-check/Cargo.toml"
CHECKER_SOURCE = "tools/architecture/handler-contract-check/"
HANDLER_CONTRACT = "tools/architecture/world-handler-contract.tsv"
HANDLER_MODULE_POLICY = "tools/architecture/handler-module-policy.json"
ARCHITECTURE = "tools/architecture/"
ARCHITECTURE_DOC = "docs/architecture/ownership-and-boundaries.md"
WORLD = "crates/wow-world"
UNSUPPORTED = (
    "tools/architecture/map-runtime-spike/",
    "tools/architecture/modularity-lab/",
    "tools/architecture/modularity-conformance/",
)
SNAPSHOT_HELPER = Path(__file__).resolve().with_name("full_snapshot.py")
SELF_TESTS = {
    "session_method_index.py": "session_method_index.py",
    "wow_world_test_census.py": "wow_world_test_census.py",
    "wow_world_test_scan.py": "wow_world_test_census.py",
    "warning_units.py": "warning_units.py",
    "move_token_compare.py": "move_token_compare.py",
}
TEST_OWNERS = {
    "hotspot_metrics.py": "test_check_architecture.py",
    "net-move-policy.json": "test_net_move.py",
    "wow_world_coupling.py": "test_wow_world_coupling.py",
    "wow-world-test-census.json": "test_wow_world_test_census.py",
    "wow-world-test-ceiling.json": "test_wow_world_test_census.py",
    "session-method-index.json": "test_wow_world_test_census.py",
}


def py(*args: str | Path) -> list[str]:
    return [sys.executable, *(str(arg) for arg in args)]


def checker(*args: str) -> list[str]:
    return ["cargo", "run", "--release", "--locked", "--manifest-path", CHECKER,
            "--target-dir", "target/arch-tools", "--bin", "session-ownership-check", "--", *args]


def handler_checker(*args: str) -> list[str]:
    return ["cargo", "run", "--release", "--locked", "--manifest-path", CHECKER,
            "--target-dir", "target/arch-tools", "--bin", "handler-contract-check", "--", *args]


def changed_paths(base: str) -> list[str]:
    raw = subprocess.check_output(
        ["git", "diff", "--name-only", "--no-renames", "-z", base, "HEAD", "--"]
    )
    return [Path(path.decode()).as_posix() for path in raw.split(b"\0") if path]


def policy_lane(paths: set[str]) -> bool:
    return any(
        (path.startswith("crates/") and path.endswith(".rs"))
        or path.startswith(WORLD)
        or path in {"Cargo.toml", "Cargo.lock"}
        or path in POLICY_FILES
        or path.startswith(CHECKER_SOURCE)
        or path == ARCHITECTURE_DOC
        or (path.startswith(ARCHITECTURE) and Path(path).suffix != ".md")
        for path in paths
    )


def handler_contract_lane(paths: set[str], root: Path) -> bool:
    if (HANDLER_CONTRACT in paths or HANDLER_MODULE_POLICY in paths
            or paths & {"Cargo.toml", "Cargo.lock"}
            or any(path.startswith(CHECKER_SOURCE) for path in paths)):
        return True
    if not any(path.startswith("crates/") for path in paths):
        return False

    policy = json.loads((root / HANDLER_MODULE_POLICY).read_text(encoding="utf-8"))
    owners = policy.get("capability_owners") if isinstance(policy, dict) else None
    if not isinstance(owners, list):
        raise ValueError(f"{HANDLER_MODULE_POLICY} must contain capability_owners")
    packages = {"world-server"}
    for owner in owners:
        if not isinstance(owner, dict):
            raise ValueError(f"{HANDLER_MODULE_POLICY} contains an invalid capability owner")
        if owner.get("capability") in {"handler_registration", "packet_dispatcher"}:
            package = owner.get("package")
            if not isinstance(package, str) or not package:
                raise ValueError(f"{HANDLER_MODULE_POLICY} contains an invalid handler package")
            packages.add(package)
    for path in paths:
        for package in packages:
            prefix = f"crates/{package}/"
            if ((path.startswith(prefix) and path.endswith(".rs")) or path in {
                f"crates/{package}/Cargo.toml", f"crates/{package}/Cargo.lock",
            }):
                return True
    return False


def own_python_tests(paths: set[str], root: Path) -> list[tuple[str, list[str], str]]:
    tests = set()
    for path in paths:
        candidate = Path(path)
        if not path.startswith(ARCHITECTURE) or candidate.suffix != ".py":
            continue
        test = candidate if candidate.name.startswith("test_") else candidate.with_name(
            f"test_{candidate.stem}.py"
        )
        if test.name == "test_physical_files.py":
            continue  # check_architecture --self-test runs this complete suite.
        if (root / test).is_file():
            tests.add(test)
    tests.update(Path(ARCHITECTURE + TEST_OWNERS[Path(path).name])
                 for path in paths if Path(path).name in TEST_OWNERS)
    if paths & {
        "tools/architecture/session_method_index.py",
        "tools/architecture/hotspot_metrics.py",
        "tools/architecture/wow_world_test_census.py",
        "tools/architecture/wow_world_test_scan.py",
    }:
        tests.add(Path("tools/architecture/test_wow_world_test_census.py"))
    return [
        (f"Python owner tests: {test}", py("-m", "unittest", "discover", "-s", test.parent,
                                                  "-p", test.name), "command")
        for test in sorted(tests)
    ]


def select_checks(paths: set[str] | list[str], base: str, root: Path | None = None):
    """Select one fixed policy suite; the outer runner owns candidate and report state."""
    values = set(paths)
    root = root or Path.cwd()
    unsupported = sorted(path for path in values if path.startswith(UNSUPPORTED))
    handler_contract_changed = handler_contract_lane(values, root)
    if not policy_lane(values) and not handler_contract_changed:
        return [], unsupported

    checks: list[tuple[str, list[str], str]] = [
        ("Architecture, physical policy, and fixtures",
         py("tools/architecture/check_architecture.py", "check", "--self-test"), "command"),
        ("Ignored crate source inventory", py("tools/architecture/ignored_sources.py", "check"), "command"),
        ("wow-world structural closure", py("tools/architecture/wow_world_closure.py", "check"), "command"),
        ("wow-world test census and ceiling", py("tools/architecture/wow_world_test_census.py", "check"), "command"),
        ("WorldSession method ownership", py("tools/architecture/session_method_index.py", "check"), "command"),
        ("#1241 net-move policy", py("tools/architecture/net_move.py", "check", "--base", base), "command"),
    ]
    checks.extend(own_python_tests(values, root))
    for tool, owner in SELF_TESTS.items():
        path, owner_path = ARCHITECTURE + tool, ARCHITECTURE + owner
        if path in values:
            checks.append((f"Tool self-test: {path}", py(owner_path, "self-test"), "command"))

    if handler_contract_changed:
        checks.append(("Handler registration and composition contract", handler_checker("check"), "command"))

    checker_changed = any(path.startswith(CHECKER_SOURCE) for path in values)
    persistence_rules_changed = bool(values & {PERSISTENCE_POLICY, WORKFLOWS})
    exhaustive = checker_changed or persistence_rules_changed
    snapshot_changed = SNAPSHOT in values
    if snapshot_changed and not exhaustive:
        checks.append(("Snapshot source proof", py(SNAPSHOT_HELPER, "--base", base), "snapshot"))
    checks.append(("Session ownership syntax and persistence inventory",
                   checker("check") if exhaustive else checker("check", "--syntax-only"),
                   "exhaustive" if exhaustive else "syntax"))
    checks.append(("Snapshot-derived policy consistency",
                   checker("print-persistence-policy", "--from-snapshot", SNAPSHOT), "policy"))
    return checks, unsupported


def check_policy_shapes(root: Path) -> None:
    expected = {SNAPSHOT: 3, PERSISTENCE_POLICY: 1, WORKFLOWS: 2, SESSION_POLICY: 1}
    values = {}
    for path, version in expected.items():
        value = json.loads((root / path).read_text(encoding="utf-8"))
        if not isinstance(value, dict) or value.get("schema_version") != version:
            raise ValueError(f"{path} must have schema_version {version}")
        values[path] = value
    if not isinstance(values[SNAPSHOT].get("accesses"), list):
        raise ValueError(f"{SNAPSHOT} must contain an accesses array")
    if values[SESSION_POLICY].get("persistence_access_snapshot") != SNAPSHOT:
        raise ValueError(f"{SESSION_POLICY} must reference {SNAPSHOT}")
    if not isinstance(values[WORKFLOWS].get("workflows"), list):
        raise ValueError(f"{WORKFLOWS} must contain a workflows array")
    if not isinstance(values[PERSISTENCE_POLICY].get("groups"), list):
        raise ValueError(f"{PERSISTENCE_POLICY} must contain a groups array")


def run_checks(checks, unsupported, root: Path, run=None) -> int:
    invoke = run or subprocess.run
    exhaustive = False
    for name, command, kind in checks:
        if exhaustive and kind == "syntax":
            print(f"{name}: covered by exhaustive native checker", flush=True)
            continue
        print(f"{name}: {shlex.join(command)}", flush=True)
        if kind == "policy":
            result = invoke(command, cwd=root, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            code = result.returncode
            if not code:
                try:
                    if json.loads(result.stdout) != json.loads(
                        (root / PERSISTENCE_POLICY).read_text(encoding="utf-8")
                    ):
                        raise ValueError("checked persistence policy disagrees with the snapshot")
                except (json.JSONDecodeError, OSError, ValueError) as error:
                    print(f"Snapshot-derived policy consistency: {error}", file=sys.stderr, flush=True)
                    return 1
        else:
            result = invoke(command, cwd=root)
            code = result.returncode
        print(f"{name}: exit {code}", flush=True)
        if kind == "snapshot" and code == 2:
            fallback = checker("check")
            print("Snapshot proof is unsupported for this delta; running exhaustive native checker.", flush=True)
            result = invoke(fallback, cwd=root)
            code = result.returncode
            print(f"Exhaustive session and persistence inventory: exit {code}", flush=True)
            if code:
                return code
            exhaustive = True
            continue
        if code:
            return code
        exhaustive |= kind == "exhaustive"
    if unsupported:
        print("Coverage decision: UNSUPPORTED (exit 2)", flush=True)
        for path in unsupported:
            print(f"  {path}", flush=True)
        return 2
    if exhaustive:
        print("Coverage: exhaustive session and persistence inventory passed.", flush=True)
    elif checks:
        print("Coverage: syntax and snapshot-policy consistency passed; exhaustive inventory not claimed.",
              flush=True)
    else:
        print("Coverage decision: no owned policy inputs changed.", flush=True)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True, help="integration base commit or ref")
    parser.add_argument("paths", nargs="*", help="optional changed paths supplied by the outer runner")
    args = parser.parse_args()
    try:
        root = Path.cwd().resolve()
        paths = args.paths or changed_paths(args.base)
        checks, unsupported = select_checks(paths, args.base, root)
        if checks:
            check_policy_shapes(root)
    except (OSError, json.JSONDecodeError, subprocess.CalledProcessError, ValueError) as error:
        print(f"full-policy: {error}", file=sys.stderr)
        return 2
    return run_checks(checks, unsupported, root)


if __name__ == "__main__":
    sys.exit(main())
