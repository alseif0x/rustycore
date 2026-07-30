#!/usr/bin/env python3
"""Executable architecture guardrails for the RustyCore workspace."""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess
import sys
from typing import Any


REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
ARCHITECTURE_DIR = pathlib.Path(__file__).resolve().parent
DEFAULT_POLICY = ARCHITECTURE_DIR / "dependency-policy.json"
FIXTURES_DIR = ARCHITECTURE_DIR / "fixtures"
PRODUCT_DEPENDENCY_KINDS = {"normal", "build"}


class ArchitectureError(RuntimeError):
    """A policy, metadata, or architecture-contract error."""


def load_json(path: pathlib.Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise ArchitectureError(f"cannot read {path}: {exc}") from exc
    except json.JSONDecodeError as exc:
        raise ArchitectureError(f"invalid JSON in {path}: {exc}") from exc


def validate_policy(policy: Any) -> dict[str, Any]:
    if not isinstance(policy, dict) or policy.get("schema_version") != 1:
        raise ArchitectureError("dependency policy must be a schema_version 1 object")

    categories = policy.get("categories")
    allowed = policy.get("allowed_category_dependencies")
    restricted = policy.get("restricted_packages")
    exceptions = policy.get("exceptions")
    if not isinstance(categories, dict) or not categories:
        raise ArchitectureError("dependency policy categories must be a non-empty object")
    if not isinstance(allowed, dict) or set(allowed) != set(categories):
        raise ArchitectureError(
            "allowed_category_dependencies must define every category exactly once"
        )
    if not isinstance(restricted, dict):
        raise ArchitectureError("restricted_packages must be an object")
    if not isinstance(exceptions, list):
        raise ArchitectureError("exceptions must be an array")

    package_categories: dict[str, str] = {}
    for category, packages in categories.items():
        if not isinstance(category, str) or not isinstance(packages, list):
            raise ArchitectureError("every category must contain a package-name array")
        for package in packages:
            if not isinstance(package, str) or not package:
                raise ArchitectureError("category package names must be non-empty strings")
            previous = package_categories.setdefault(package, category)
            if previous != category:
                raise ArchitectureError(
                    f"package {package} is classified as both {previous} and {category}"
                )

    known_categories = set(categories)
    for source_category, target_categories in allowed.items():
        if not isinstance(target_categories, list):
            raise ArchitectureError(
                f"allowed targets for {source_category} must be an array"
            )
        unknown = set(target_categories) - known_categories
        if unknown:
            raise ArchitectureError(
                f"{source_category} allows unknown categories: {sorted(unknown)}"
            )

    for package, direct_dependencies in restricted.items():
        if package not in package_categories:
            raise ArchitectureError(f"restricted package {package} is not classified")
        if not isinstance(direct_dependencies, list):
            raise ArchitectureError(
                f"restricted package {package} must contain an array"
            )
        unknown = set(direct_dependencies) - set(package_categories)
        if unknown:
            raise ArchitectureError(
                f"{package} directly allows unknown packages: {sorted(unknown)}"
            )

    exception_map: dict[tuple[str, str], dict[str, Any]] = {}
    for index, exception in enumerate(exceptions):
        if not isinstance(exception, dict):
            raise ArchitectureError(f"exception {index} must be an object")
        source = exception.get("from")
        target = exception.get("to")
        tracking_issue = exception.get("tracking_issue")
        reason = exception.get("reason")
        if source not in package_categories or target not in package_categories:
            raise ArchitectureError(
                f"exception {index} references an unclassified package: {source} -> {target}"
            )
        if not isinstance(tracking_issue, int) or tracking_issue <= 0:
            raise ArchitectureError(
                f"exception {source} -> {target} needs a positive tracking_issue"
            )
        if not isinstance(reason, str) or not reason.strip():
            raise ArchitectureError(
                f"exception {source} -> {target} needs a non-empty reason"
            )
        key = (source, target)
        if key in exception_map:
            raise ArchitectureError(f"duplicate exception: {source} -> {target}")
        exception_map[key] = exception

    policy["_package_categories"] = package_categories
    policy["_exception_map"] = exception_map
    return policy


def classify_edge(
    policy: dict[str, Any], source: str, target: str
) -> tuple[str, str]:
    package_categories = policy["_package_categories"]
    exception_map = policy["_exception_map"]
    if source not in package_categories or target not in package_categories:
        return "forbidden", "one or both packages are not classified"

    restricted = policy["restricted_packages"].get(source)
    if restricted is not None:
        if target in restricted:
            return "allowed", "explicit direct dependency of a restricted package"
        exception = exception_map.get((source, target))
        if exception is not None:
            return (
                "exception",
                f"baseline exception tracked by #{exception['tracking_issue']}",
            )
        return "forbidden", f"{source} has a restricted direct-dependency surface"

    source_category = package_categories[source]
    target_category = package_categories[target]
    if target_category in policy["allowed_category_dependencies"][source_category]:
        return "allowed", f"{source_category} may depend on {target_category}"

    exception = exception_map.get((source, target))
    if exception is not None:
        return (
            "exception",
            f"baseline exception tracked by #{exception['tracking_issue']}",
        )
    return (
        "forbidden",
        f"{source_category} may not depend on {target_category}",
    )


def cargo_metadata() -> dict[str, Any]:
    command = [
        "cargo",
        "metadata",
        "--locked",
        "--no-deps",
        "--format-version",
        "1",
    ]
    try:
        completed = subprocess.run(
            command,
            cwd=REPO_ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
    except OSError as exc:
        raise ArchitectureError(f"cannot run cargo metadata: {exc}") from exc
    if completed.returncode != 0:
        detail = completed.stderr.strip() or f"exit status {completed.returncode}"
        raise ArchitectureError(f"cargo metadata failed: {detail}")
    try:
        metadata = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise ArchitectureError(f"cargo metadata returned invalid JSON: {exc}") from exc
    if not isinstance(metadata, dict):
        raise ArchitectureError("cargo metadata did not return an object")
    return metadata


def production_edges(
    metadata: dict[str, Any],
) -> tuple[set[str], set[tuple[str, str, str]]]:
    packages = metadata.get("packages")
    workspace_members = metadata.get("workspace_members")
    if not isinstance(packages, list) or not isinstance(workspace_members, list):
        raise ArchitectureError("cargo metadata is missing packages or workspace_members")

    package_by_id = {
        package.get("id"): package
        for package in packages
        if isinstance(package, dict) and isinstance(package.get("id"), str)
    }
    member_ids = set(workspace_members)
    missing_members = member_ids - set(package_by_id)
    if missing_members:
        raise ArchitectureError(
            f"cargo metadata omitted workspace members: {sorted(missing_members)}"
        )

    member_names = {
        package_by_id[package_id].get("name") for package_id in member_ids
    }
    if None in member_names or len(member_names) != len(member_ids):
        raise ArchitectureError("workspace package names are missing or duplicated")

    edges: set[tuple[str, str, str]] = set()
    for package_id in member_ids:
        package = package_by_id[package_id]
        source = package["name"]
        dependencies = package.get("dependencies")
        if not isinstance(dependencies, list):
            raise ArchitectureError(f"package {source} has invalid dependency metadata")
        for dependency in dependencies:
            if not isinstance(dependency, dict):
                continue
            target = dependency.get("name")
            kind = dependency.get("kind") or "normal"
            if target in member_names and kind in PRODUCT_DEPENDENCY_KINDS:
                edges.add((source, target, kind))
    return member_names, edges


def check_dependencies(
    policy: dict[str, Any], metadata: dict[str, Any]
) -> tuple[int, int, int]:
    workspace_packages, edges = production_edges(metadata)
    policy_packages = set(policy["_package_categories"])
    missing = sorted(workspace_packages - policy_packages)
    stale = sorted(policy_packages - workspace_packages)
    errors: list[str] = []
    if missing:
        errors.append(f"unclassified workspace packages: {', '.join(missing)}")
    if stale:
        errors.append(f"classified packages no longer in workspace: {', '.join(stale)}")

    used_exceptions: set[tuple[str, str]] = set()
    for source, target, kind in sorted(edges):
        decision, reason = classify_edge(policy, source, target)
        if decision == "exception":
            used_exceptions.add((source, target))
        elif decision == "forbidden":
            errors.append(f"forbidden {kind} edge {source} -> {target}: {reason}")

    stale_exceptions = obsolete_exception_edges(policy, used_exceptions)
    for source, target in stale_exceptions:
        errors.append(
            f"obsolete baseline exception {source} -> {target}; remove it from the policy"
        )

    if errors:
        raise ArchitectureError("\n".join(errors))
    return len(workspace_packages), len(edges), len(used_exceptions)


def obsolete_exception_edges(
    policy: dict[str, Any], used_exceptions: set[tuple[str, str]]
) -> list[tuple[str, str]]:
    return sorted(set(policy["_exception_map"]) - used_exceptions)


def test_module_start(lines: list[str]) -> int | None:
    cfg_test = re.compile(r"^\s*#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]\s*$")
    tests_module = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+tests\s*\{")
    for index, line in enumerate(lines):
        if not cfg_test.match(line):
            continue
        for candidate in lines[index + 1 : index + 5]:
            if tests_module.match(candidate):
                return index
    return None


def hotspot_rows(limit: int = 10) -> list[tuple[int, int, int, str]]:
    rows: list[tuple[int, int, int, str]] = []
    crates_root = REPO_ROOT / "crates"
    for path in crates_root.glob("*/src/**/*.rs"):
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except OSError as exc:
            raise ArchitectureError(f"cannot read Rust source {path}: {exc}") from exc
        test_start = test_module_start(lines)
        test_lines = len(lines) - test_start if test_start is not None else 0
        production_lines = len(lines) - test_lines
        rows.append(
            (
                len(lines),
                production_lines,
                test_lines,
                path.relative_to(REPO_ROOT).as_posix(),
            )
        )
    rows.sort(key=lambda row: (-row[0], row[3]))
    return rows[:limit]


def print_hotspots(limit: int = 10) -> None:
    print(
        "Architecture hotspots (informational; inline #[cfg(test)] module split is approximate):"
    )
    print(f"{'total':>8} {'prod':>8} {'tests':>8}  path")
    for total, production, tests, path in hotspot_rows(limit):
        print(f"{total:8d} {production:8d} {tests:8d}  {path}")


def synthetic_metadata(
    package_names: set[str], edges: set[tuple[str, str]]
) -> dict[str, Any]:
    package_ids = {
        package: f"path+file:///architecture-fixture/{package}#0.0.0"
        for package in package_names
    }
    return {
        "workspace_members": [package_ids[package] for package in sorted(package_names)],
        "packages": [
            {
                "id": package_ids[package],
                "name": package,
                "dependencies": [
                    {"name": target, "kind": None}
                    for source, target in sorted(edges)
                    if source == package
                ],
            }
            for package in sorted(package_names)
        ],
    }


def fixture_policy(
    policy: dict[str, Any], source: str, target: str
) -> dict[str, Any]:
    package_categories = policy["_package_categories"]
    categories: dict[str, list[str]] = {}
    for package in (source, target):
        category = package_categories[package]
        categories.setdefault(category, []).append(package)
    allowed_categories = {
        category: [
            allowed
            for allowed in policy["allowed_category_dependencies"][category]
            if allowed in categories
        ]
        for category in categories
    }
    scenario_packages = {source, target}
    restricted_packages = {}
    if source in policy["restricted_packages"]:
        restricted_packages[source] = [
            dependency
            for dependency in policy["restricted_packages"][source]
            if dependency in scenario_packages
        ]
    return validate_policy(
        {
            "schema_version": 1,
            "categories": categories,
            "allowed_category_dependencies": allowed_categories,
            "restricted_packages": restricted_packages,
            "exceptions": [],
        }
    )


def run_fixture_self_tests(policy: dict[str, Any]) -> None:
    fixtures = sorted(FIXTURES_DIR.glob("*.json"))
    if not fixtures:
        raise ArchitectureError(f"no architecture fixtures found in {FIXTURES_DIR}")
    for fixture_path in fixtures:
        fixture = load_json(fixture_path)
        if not isinstance(fixture, dict):
            raise ArchitectureError(f"fixture {fixture_path.name} must be an object")
        source = fixture.get("source")
        target = fixture.get("target")
        expected = fixture.get("expected")
        if not all(isinstance(value, str) for value in (source, target, expected)):
            raise ArchitectureError(
                f"fixture {fixture_path.name} needs string source, target, and expected"
            )
        if source not in policy["_package_categories"] or target not in policy[
            "_package_categories"
        ]:
            raise ArchitectureError(
                f"fixture {fixture_path.name} references an unclassified package"
            )
        if expected not in {"allowed", "forbidden"}:
            raise ArchitectureError(
                f"fixture {fixture_path.name} has unknown expected result {expected!r}"
            )
        scenario_policy = fixture_policy(policy, source, target)
        metadata = synthetic_metadata({source, target}, {(source, target)})
        try:
            check_dependencies(scenario_policy, metadata)
        except ArchitectureError as exc:
            if expected != "forbidden" or "forbidden normal edge" not in str(exc):
                raise ArchitectureError(
                    f"fixture {fixture_path.name}: expected {expected}, got error: {exc}"
                ) from exc
        else:
            if expected != "allowed":
                raise ArchitectureError(
                    f"fixture {fixture_path.name}: expected {expected}, got allowed"
                )

    exception_edge = ("wow-network", "wow-data")
    if classify_edge(policy, *exception_edge)[0] != "exception":
        raise ArchitectureError("baseline exception classification self-test failed")

    stale_policy = validate_policy(
        {
            "schema_version": 1,
            "categories": {
                "adapter-platform": ["fixture-network", "fixture-data"]
            },
            "allowed_category_dependencies": {
                "adapter-platform": ["adapter-platform"]
            },
            "restricted_packages": {"fixture-network": []},
            "exceptions": [
                {
                    "from": "fixture-network",
                    "to": "fixture-data",
                    "tracking_issue": 135,
                    "reason": "Synthetic exception used to test the removal ratchet.",
                }
            ],
        }
    )
    try:
        check_dependencies(
            stale_policy,
            synthetic_metadata({"fixture-network", "fixture-data"}, set()),
        )
    except ArchitectureError as exc:
        if "obsolete baseline exception fixture-network -> fixture-data" not in str(exc):
            raise ArchitectureError(
                f"obsolete-exception ratchet returned the wrong failure: {exc}"
            ) from exc
    else:
        raise ArchitectureError("obsolete-exception ratchet self-test failed")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--policy",
        type=pathlib.Path,
        default=DEFAULT_POLICY,
        help="dependency policy JSON (default: repository policy)",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("check", help="check dependencies and report source hotspots")
    subparsers.add_parser("self-test", help="validate policy and focused fixtures")
    hotspots_parser = subparsers.add_parser(
        "hotspots", help="report source hotspots without enforcing a line limit"
    )
    hotspots_parser.add_argument("--limit", type=int, default=10)
    args = parser.parse_args()

    try:
        policy = validate_policy(load_json(args.policy))
        if args.command == "self-test":
            run_fixture_self_tests(policy)
            print(
                f"Architecture self-test: PASS ({len(list(FIXTURES_DIR.glob('*.json')))} fixtures)"
            )
        elif args.command == "hotspots":
            if args.limit <= 0:
                raise ArchitectureError("--limit must be positive")
            print_hotspots(args.limit)
        else:
            packages, edges, exceptions = check_dependencies(policy, cargo_metadata())
            print(
                "Architecture dependencies: "
                f"PASS ({packages} packages, {edges} production edges, "
                f"{exceptions} baseline exceptions)"
            )
            print_hotspots()
    except ArchitectureError as exc:
        print(f"architecture check failed: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
