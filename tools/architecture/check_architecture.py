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
from urllib.parse import urlsplit


REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
ARCHITECTURE_DIR = pathlib.Path(__file__).resolve().parent
DEFAULT_POLICY = ARCHITECTURE_DIR / "dependency-policy.json"
DEFAULT_ISSUE_LEDGER = ARCHITECTURE_DIR / "architecture-issue-ledger.json"
DEFAULT_RUNTIME_OWNERSHIP_LEDGER = ARCHITECTURE_DIR / "runtime-ownership-ledger.json"
DEFAULT_SESSION_OWNERSHIP_POLICY = ARCHITECTURE_DIR / "session-ownership-policy.json"
DEFAULT_HANDLER_MODULE_POLICY = ARCHITECTURE_DIR / "handler-module-policy.json"
ARCHITECTURE_DOC = REPO_ROOT / "docs" / "architecture" / "ownership-and-boundaries.md"
HANDLER_SNAPSHOT = ARCHITECTURE_DIR / "world-handler-contract.tsv"
FIXTURES_DIR = ARCHITECTURE_DIR / "fixtures"
DEBT_OWNERSHIP_FIXTURES_DIR = FIXTURES_DIR / "debt-ownership"
LEDGER_ISSUE_STATES = {"open", "closed"}
LEDGER_ISSUE_KINDS = {"epic", "slice"}
HANDLER_MODULE_CAPABILITIES = {"handler_registration", "packet_dispatcher"}
PRODUCT_DEPENDENCY_KINDS = {"normal", "build"}
IGNORED_DEPENDENCY_KINDS = {"dev"}
CRATES_IO_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"
CARGO_METADATA_COMMAND = (
    "cargo",
    "metadata",
    "--locked",
    "--all-features",
    "--format-version",
    "1",
)


class ArchitectureError(RuntimeError):
    """A policy, metadata, or architecture-contract error."""


def reject_duplicate_json_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ArchitectureError(f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def parse_json(text: str, source: str) -> Any:
    try:
        return json.loads(text, object_pairs_hook=reject_duplicate_json_keys)
    except json.JSONDecodeError as exc:
        raise ArchitectureError(f"invalid JSON in {source}: {exc}") from exc
    except ArchitectureError as exc:
        raise ArchitectureError(f"invalid JSON in {source}: {exc}") from exc


def load_json(path: pathlib.Path) -> Any:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ArchitectureError(f"cannot read {path}: {exc}") from exc
    return parse_json(text, str(path))


def validate_handler_module_policy(
    policy: Any, ledger: dict[str, Any]
) -> dict[str, Any]:
    """Validate logical handler capability owners and their open retirement issues."""
    root_keys = {"schema_version", "introduced_by_issue", "capability_owners"}
    if not isinstance(policy, dict) or policy.get("schema_version") != 1:
        raise ArchitectureError("handler module policy must be a schema_version 1 object")
    if set(policy) != root_keys:
        raise ArchitectureError(
            "handler module policy must contain exactly " + ", ".join(sorted(root_keys))
        )
    introduced_by = policy.get("introduced_by_issue")
    owners = policy.get("capability_owners")
    if not isinstance(introduced_by, int) or isinstance(introduced_by, bool) or introduced_by <= 0:
        raise ArchitectureError("handler module policy introduced_by_issue must be positive")
    if not isinstance(owners, list):
        raise ArchitectureError("handler module policy capability_owners must be an array")

    issues = {entry["number"]: entry for entry in ledger["issues"]}
    if introduced_by not in issues:
        raise ArchitectureError(
            f"handler module policy introduced_by_issue #{introduced_by} is absent from the architecture issue ledger"
        )

    owner_keys = {
        "capability",
        "package",
        "module",
        "allow_descendants",
        "tracking_issue",
    }
    seen_capabilities: set[str] = set()
    declared_owners: list[dict[str, Any]] = []
    logical_module = re.compile(r"^crate(?:::[A-Za-z_][A-Za-z0-9_]*)*$")
    for index, owner in enumerate(owners):
        if not isinstance(owner, dict) or set(owner) != owner_keys:
            raise ArchitectureError(
                f"handler module policy owner {index} must contain exactly "
                + ", ".join(sorted(owner_keys))
            )
        capability = owner["capability"]
        package = owner["package"]
        module = owner["module"]
        allow_descendants = owner["allow_descendants"]
        tracking_issue = owner["tracking_issue"]
        if not isinstance(capability, str) or capability not in HANDLER_MODULE_CAPABILITIES:
            raise ArchitectureError(
                f"handler module policy owner {index} has unknown capability {capability!r}"
            )
        if capability in seen_capabilities:
            raise ArchitectureError(
                f"handler module policy declares duplicate capability {capability}"
            )
        seen_capabilities.add(capability)
        if not isinstance(package, str) or not package:
            raise ArchitectureError(
                f"handler module policy capability {capability} needs a package"
            )
        if not isinstance(module, str) or not logical_module.fullmatch(module):
            raise ArchitectureError(
                f"handler module policy capability {capability} has invalid logical module {module!r}"
            )
        if not isinstance(allow_descendants, bool):
            raise ArchitectureError(
                f"handler module policy capability {capability} allow_descendants must be boolean"
            )
        for previous in declared_owners:
            same_package = package == previous["package"]
            this_below_previous = module == previous["module"] or (
                previous["allow_descendants"]
                and module.startswith(previous["module"] + "::")
            )
            previous_below_this = previous["module"] == module or (
                allow_descendants
                and previous["module"].startswith(module + "::")
            )
            if same_package and (this_below_previous or previous_below_this):
                raise ArchitectureError(
                    "handler module policy capabilities "
                    f"{previous['capability']} and {capability} have overlapping logical owners"
                )
        declared_owners.append(owner)
        if (
            not isinstance(tracking_issue, int)
            or isinstance(tracking_issue, bool)
            or tracking_issue <= 0
        ):
            raise ArchitectureError(
                f"handler module policy capability {capability} needs a positive tracking_issue"
            )
        tracked = issues.get(tracking_issue)
        if tracked is None:
            raise ArchitectureError(
                f"handler module policy capability {capability} tracking issue #{tracking_issue} is absent from the architecture issue ledger"
            )
        if tracked["state"] != "open":
            raise ArchitectureError(
                f"handler module policy capability {capability} has stale closed tracking issue #{tracking_issue}"
            )

    if seen_capabilities != HANDLER_MODULE_CAPABILITIES:
        missing = sorted(HANDLER_MODULE_CAPABILITIES - seen_capabilities)
        raise ArchitectureError(
            f"handler module policy is missing required capabilities: {missing}"
        )
    return policy


def run_handler_module_policy_self_tests(
    policy: dict[str, Any], ledger: dict[str, Any]
) -> int:
    mutations: list[tuple[str, dict[str, Any], str]] = []

    duplicate = json.loads(json.dumps(policy))
    duplicate["capability_owners"].append(duplicate["capability_owners"][0])
    mutations.append(("duplicate-capability", duplicate, "duplicate capability"))

    invalid_module = json.loads(json.dumps(policy))
    invalid_module["capability_owners"][0]["module"] = "handlers"
    mutations.append(("invalid-module", invalid_module, "invalid logical module"))

    unknown_field = json.loads(json.dumps(policy))
    unknown_field["unexpected"] = True
    mutations.append(("unknown-field", unknown_field, "must contain exactly"))

    stale = json.loads(json.dumps(policy))
    stale["capability_owners"][0]["tracking_issue"] = 134
    mutations.append(("stale-issue", stale, "stale closed tracking issue #134"))

    absent = json.loads(json.dumps(policy))
    absent["capability_owners"][0]["tracking_issue"] = 999999
    mutations.append(("absent-issue", absent, "absent from the architecture issue ledger"))

    overlap = json.loads(json.dumps(policy))
    overlap["capability_owners"][0]["module"] = "crate::session::handlers"
    mutations.append(("overlapping-owners", overlap, "overlapping logical owners"))

    for name, mutant, expected_error in mutations:
        try:
            validate_handler_module_policy(mutant, ledger)
        except ArchitectureError as exc:
            if expected_error not in str(exc):
                raise ArchitectureError(
                    f"handler module policy self-test {name} returned the wrong failure: {exc}"
                ) from exc
        else:
            raise ArchitectureError(
                f"handler module policy self-test {name} was not rejected"
            )
    return len(mutations)


def validate_policy(policy: Any) -> dict[str, Any]:
    if not isinstance(policy, dict) or policy.get("schema_version") != 2:
        raise ArchitectureError("dependency policy must be a schema_version 2 object")

    categories = policy.get("categories")
    allowed = policy.get("allowed_category_dependencies")
    restricted = policy.get("restricted_packages")
    exceptions = policy.get("exceptions")
    external = policy.get("external_dependencies")
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
    if not isinstance(external, dict):
        raise ArchitectureError("external_dependencies must be an object")

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

    restricted_allowed_edges: set[tuple[str, str]] = set()
    for package, direct_dependencies in restricted.items():
        if package not in package_categories:
            raise ArchitectureError(f"restricted package {package} is not classified")
        if not isinstance(direct_dependencies, list):
            raise ArchitectureError(
                f"restricted package {package} must contain an array"
            )
        if not all(
            isinstance(dependency, str) and dependency
            for dependency in direct_dependencies
        ):
            raise ArchitectureError(
                f"restricted package {package} dependencies must be non-empty strings"
            )
        if len(direct_dependencies) != len(set(direct_dependencies)):
            raise ArchitectureError(
                f"restricted package {package} dependencies contain duplicates"
            )
        unknown = set(direct_dependencies) - set(package_categories)
        if unknown:
            raise ArchitectureError(
                f"{package} directly allows unknown packages: {sorted(unknown)}"
            )
        restricted_allowed_edges.update(
            (package, dependency) for dependency in direct_dependencies
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
        if type(tracking_issue) is not int or tracking_issue <= 0:
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

    protected_categories = external.get("protected_categories")
    explicitly_protected_packages = external.get("protected_packages")
    canonical_registry_source = external.get("canonical_registry_source")
    external_allowed = external.get("allowed")
    external_exceptions = external.get("exceptions")
    if not isinstance(protected_categories, list):
        raise ArchitectureError(
            "external_dependencies.protected_categories must be an array"
        )
    if not all(
        isinstance(category, str) and category for category in protected_categories
    ):
        raise ArchitectureError(
            "external_dependencies.protected_categories must contain non-empty strings"
        )
    if len(protected_categories) != len(set(protected_categories)):
        raise ArchitectureError(
            "external_dependencies.protected_categories contains duplicates"
        )
    unknown_protected_categories = set(protected_categories) - known_categories
    if unknown_protected_categories:
        raise ArchitectureError(
            "external_dependencies protects unknown categories: "
            f"{sorted(unknown_protected_categories)}"
        )
    if not isinstance(explicitly_protected_packages, list):
        raise ArchitectureError(
            "external_dependencies.protected_packages must be an array"
        )
    if not all(
        isinstance(package, str) and package
        for package in explicitly_protected_packages
    ):
        raise ArchitectureError(
            "external_dependencies.protected_packages must contain "
            "non-empty strings"
        )
    if len(explicitly_protected_packages) != len(
        set(explicitly_protected_packages)
    ):
        raise ArchitectureError(
            "external_dependencies.protected_packages contains duplicates"
        )
    unknown_protected_packages = (
        set(explicitly_protected_packages) - set(package_categories)
    )
    if unknown_protected_packages:
        raise ArchitectureError(
            "external_dependencies protects unknown packages: "
            f"{sorted(unknown_protected_packages)}"
        )
    if not isinstance(external_allowed, dict):
        raise ArchitectureError("external_dependencies.allowed must be an object")
    if not isinstance(external_exceptions, list):
        raise ArchitectureError("external_dependencies.exceptions must be an array")
    if (
        not isinstance(canonical_registry_source, str)
        or not canonical_registry_source
    ):
        raise ArchitectureError(
            "external_dependencies.canonical_registry_source must be a non-empty string"
        )

    protected_packages = {
        package
        for package, category in package_categories.items()
        if category in protected_categories
    } | set(explicitly_protected_packages)
    unprotected_restricted_packages = set(restricted) - protected_packages
    if unprotected_restricted_packages:
        raise ArchitectureError(
            "restricted workspace packages must also guard their external surface: "
            f"{sorted(unprotected_restricted_packages)}"
        )
    missing_external_surfaces = protected_packages - set(external_allowed)
    stale_external_surfaces = set(external_allowed) - protected_packages
    if missing_external_surfaces:
        raise ArchitectureError(
            "external dependency surface missing protected packages: "
            f"{sorted(missing_external_surfaces)}"
        )
    if stale_external_surfaces:
        raise ArchitectureError(
            "external dependency surface contains unprotected packages: "
            f"{sorted(stale_external_surfaces)}"
        )

    external_allowed_edges: set[tuple[str, str, str, str]] = set()
    for package, kinds in external_allowed.items():
        if not isinstance(kinds, dict) or set(kinds) != PRODUCT_DEPENDENCY_KINDS:
            raise ArchitectureError(
                f"external dependency surface for {package} must define "
                "normal and build arrays exactly once"
            )
        for kind, dependencies in kinds.items():
            if not isinstance(dependencies, list):
                raise ArchitectureError(
                    f"external {kind} dependencies for {package} must be an array"
                )
            if not all(
                isinstance(dependency, str) and dependency
                for dependency in dependencies
            ):
                raise ArchitectureError(
                    f"external {kind} dependencies for {package} "
                    "must be non-empty strings"
                )
            if len(dependencies) != len(set(dependencies)):
                raise ArchitectureError(
                    f"external {kind} dependencies for {package} contain duplicates"
                )
            for dependency in dependencies:
                if dependency in package_categories:
                    raise ArchitectureError(
                        "external dependency policy references workspace package "
                        f"{dependency}: {kind} {package} -> {dependency}"
                    )
                external_allowed_edges.add(
                    (package, dependency, canonical_registry_source, kind)
                )

    external_exception_map: dict[
        tuple[str, str, str, str], dict[str, Any]
    ] = {}
    for index, exception in enumerate(external_exceptions):
        if not isinstance(exception, dict):
            raise ArchitectureError(f"external exception {index} must be an object")
        source = exception.get("from")
        target = exception.get("to")
        kind = exception.get("kind")
        tracking_issue = exception.get("tracking_issue")
        reason = exception.get("reason")
        if source not in protected_packages:
            raise ArchitectureError(
                f"external exception {index} references an unprotected package: {source}"
            )
        if not isinstance(target, str) or not target:
            raise ArchitectureError(
                f"external exception {index} needs a non-empty dependency name"
            )
        if target in package_categories:
            raise ArchitectureError(
                "external dependency policy references workspace package "
                f"{target}: {kind} {source} -> {target}"
            )
        if kind not in PRODUCT_DEPENDENCY_KINDS:
            raise ArchitectureError(
                f"external exception {source} -> {target} needs kind normal or build"
            )
        if type(tracking_issue) is not int or tracking_issue <= 0:
            raise ArchitectureError(
                f"external exception {source} -> {target} "
                "needs a positive tracking_issue"
            )
        if not isinstance(reason, str) or not reason.strip():
            raise ArchitectureError(
                f"external exception {source} -> {target} needs a non-empty reason"
            )
        key = (source, target, canonical_registry_source, kind)
        if key in external_allowed_edges:
            raise ArchitectureError(
                f"external dependency {kind} {source} -> {target} "
                "cannot be both allowed and exceptional"
            )
        if key in external_exception_map:
            raise ArchitectureError(
                f"duplicate external exception: {kind} {source} -> {target}"
            )
        external_exception_map[key] = exception

    policy["_package_categories"] = package_categories
    policy["_restricted_allowed_edges"] = restricted_allowed_edges
    policy["_exception_map"] = exception_map
    policy["_external_protected_categories"] = set(protected_categories)
    policy["_external_protected_packages"] = protected_packages
    policy["_external_canonical_registry_source"] = canonical_registry_source
    policy["_external_allowed_edges"] = external_allowed_edges
    policy["_external_exception_map"] = external_exception_map
    return policy


def validate_issue_ledger(ledger: Any) -> dict[str, Any]:
    """Validate the checked-in architecture issue ledger.

    The ledger is the offline source of truth for which GitHub issues may own
    dependency debt: it is committed to the repository, so the guardrails never
    depend on live GitHub availability.
    """
    if not isinstance(ledger, dict) or ledger.get("schema_version") != 2:
        raise ArchitectureError(
            "architecture issue ledger must be a schema_version 2 object"
        )
    parent_issue = ledger.get("parent_issue")
    reaudit_issue = ledger.get("reaudit_issue")
    issues = ledger.get("issues")
    sequence = ledger.get("sequence")
    external_prerequisites = ledger.get("external_prerequisites")
    if type(parent_issue) is not int or parent_issue <= 0:
        raise ArchitectureError("issue ledger needs a positive parent_issue")
    if type(reaudit_issue) is not int or reaudit_issue <= 0:
        raise ArchitectureError("issue ledger needs a positive reaudit_issue")
    if not isinstance(issues, list) or not issues:
        raise ArchitectureError("issue ledger issues must be a non-empty array")
    if not isinstance(external_prerequisites, list):
        raise ArchitectureError(
            "issue ledger external_prerequisites must be an array"
        )

    external_entries: dict[int, dict[str, Any]] = {}
    for index, entry in enumerate(external_prerequisites):
        if not isinstance(entry, dict):
            raise ArchitectureError(
                f"issue ledger external prerequisite {index} must be an object"
            )
        number = entry.get("number")
        state = entry.get("state")
        title = entry.get("title")
        if type(number) is not int or number <= 0:
            raise ArchitectureError(
                f"issue ledger external prerequisite {index} needs a positive number"
            )
        if state not in LEDGER_ISSUE_STATES:
            raise ArchitectureError(
                f"issue ledger external prerequisite #{number} needs state open or closed"
            )
        if not isinstance(title, str) or not title.strip():
            raise ArchitectureError(
                f"issue ledger external prerequisite #{number} needs a non-empty title"
            )
        if number in external_entries:
            raise ArchitectureError(
                f"duplicate issue ledger external prerequisite: #{number}"
            )
        external_entries[number] = entry

    entries: dict[int, dict[str, Any]] = {}
    for index, entry in enumerate(issues):
        if not isinstance(entry, dict):
            raise ArchitectureError(f"issue ledger entry {index} must be an object")
        number = entry.get("number")
        state = entry.get("state")
        title = entry.get("title")
        kind = entry.get("kind")
        parents = entry.get("parents")
        dependencies = entry.get("depends_on")
        if type(number) is not int or number <= 0:
            raise ArchitectureError(f"issue ledger entry {index} needs a positive number")
        if state not in LEDGER_ISSUE_STATES:
            raise ArchitectureError(
                f"issue ledger entry #{number} needs state open or closed"
            )
        if not isinstance(title, str) or not title.strip():
            raise ArchitectureError(
                f"issue ledger entry #{number} needs a non-empty title"
            )
        if kind not in LEDGER_ISSUE_KINDS:
            raise ArchitectureError(
                f"issue ledger entry #{number} needs kind epic or slice"
            )
        for label, values in (("parents", parents), ("depends_on", dependencies)):
            if not isinstance(values, list) or any(
                type(value) is not int or value <= 0 for value in values
            ):
                raise ArchitectureError(
                    f"issue ledger entry #{number} {label} must contain positive issue numbers"
                )
            if len(values) != len(set(values)):
                raise ArchitectureError(
                    f"issue ledger entry #{number} {label} contains duplicates"
                )
            if number in values:
                raise ArchitectureError(
                    f"issue ledger entry #{number} cannot reference itself in {label}"
                )
        if number in entries:
            raise ArchitectureError(f"duplicate issue ledger entry: #{number}")
        entries[number] = entry

    overlap = sorted(set(entries) & set(external_entries))
    if overlap:
        raise ArchitectureError(
            f"issue ledger issues also declared as external prerequisites: {overlap}"
        )

    known_dependencies = set(entries) | set(external_entries)
    for number, entry in entries.items():
        unknown_parents = sorted(set(entry["parents"]) - set(entries))
        if unknown_parents:
            raise ArchitectureError(
                f"issue ledger entry #{number} references unknown parents: {unknown_parents}"
            )
        non_epic_parents = sorted(
            parent
            for parent in entry["parents"]
            if entries[parent]["kind"] != "epic"
        )
        if non_epic_parents:
            raise ArchitectureError(
                f"issue ledger entry #{number} has non-epic parents: {non_epic_parents}"
            )
        unknown_dependencies = sorted(
            set(entry["depends_on"]) - known_dependencies
        )
        if unknown_dependencies:
            raise ArchitectureError(
                f"issue ledger entry #{number} references undeclared dependencies: "
                f"{unknown_dependencies}"
            )
        if entry["state"] == "closed":
            unresolved = sorted(
                dependency
                for dependency in entry["depends_on"]
                if (entries.get(dependency) or external_entries[dependency])["state"]
                != "closed"
            )
            if unresolved:
                raise ArchitectureError(
                    f"closed issue #{number} depends on open issues: {unresolved}"
                )

    for label, designated in (
        ("parent_issue", parent_issue),
        ("reaudit_issue", reaudit_issue),
    ):
        if designated not in entries:
            raise ArchitectureError(
                f"issue ledger {label} #{designated} is absent from issues"
            )
        if entries[designated]["state"] != "open":
            raise ArchitectureError(
                f"issue ledger {label} #{designated} must be open while the "
                "ledger tracks unresolved debt"
            )
    if entries[parent_issue]["kind"] != "epic":
        raise ArchitectureError("issue ledger parent_issue must be an epic")
    if entries[reaudit_issue]["kind"] != "slice":
        raise ArchitectureError("issue ledger reaudit_issue must be a slice")

    if not isinstance(sequence, list) or not sequence:
        raise ArchitectureError("issue ledger sequence must be a non-empty array")
    if any(type(number) is not int or number <= 0 for number in sequence):
        raise ArchitectureError(
            "issue ledger sequence must contain positive issue numbers"
        )
    if len(sequence) != len(set(sequence)):
        raise ArchitectureError("issue ledger sequence contains duplicates")
    unknown = sorted(set(sequence) - set(entries))
    if unknown:
        raise ArchitectureError(
            f"issue ledger sequence references absent issues: {unknown}"
        )
    epics = {number for number, entry in entries.items() if entry["kind"] == "epic"}
    sequenced_epics = sorted(set(sequence) & epics)
    if sequenced_epics:
        raise ArchitectureError(
            f"issue ledger sequence must not contain epics: {sequenced_epics}"
        )
    slices = set(entries) - epics
    unsequenced = sorted(slices - set(sequence))
    if unsequenced:
        raise ArchitectureError(
            f"issue ledger issues missing from the sequence: {unsequenced}"
        )

    positions = {number: index for index, number in enumerate(sequence)}
    ordering_errors: list[str] = []
    graph: dict[int, list[int]] = {number: [] for number in entries}
    for number, entry in entries.items():
        for dependency in entry["depends_on"]:
            if dependency not in entries:
                continue
            graph[number].append(dependency)
            if (
                entry["kind"] == "slice"
                and entries[dependency]["kind"] == "slice"
                and positions[dependency] >= positions[number]
            ):
                ordering_errors.append(f"#{number} before dependency #{dependency}")
    visiting: set[int] = set()
    visited: set[int] = set()

    def visit(number: int, trail: list[int]) -> None:
        if number in visiting:
            cycle_start = trail.index(number)
            cycle = trail[cycle_start:]
            raise ArchitectureError(
                "issue ledger dependency cycle: "
                + " -> ".join(f"#{item}" for item in cycle)
            )
        if number in visited:
            return
        visiting.add(number)
        for dependency in graph[number]:
            visit(dependency, trail + [dependency])
        visiting.remove(number)
        visited.add(number)

    for number in entries:
        visit(number, [number])
    if ordering_errors:
        raise ArchitectureError(
            "issue ledger sequence is not topological: " + ", ".join(ordering_errors)
        )

    ledger["_entries"] = entries
    ledger["_external_entries"] = external_entries
    return ledger


def validate_debt_ownership(policy: dict[str, Any], ledger: dict[str, Any]) -> None:
    """Every policy exception must be owned by an open ledger issue.

    A completed issue must not remain the supposed owner of unresolved debt,
    and an exception may never reference an issue the ledger does not track.
    """
    entries = ledger["_entries"]
    problems: list[str] = []
    workspace_exceptions = policy.get("exceptions", [])
    external_exceptions = policy.get("external_dependencies", {}).get("exceptions", [])
    for exception in workspace_exceptions:
        edge = f"{exception.get('from')} -> {exception.get('to')}"
        _validate_debt_owner(entries, edge, exception.get("tracking_issue"), problems)
    for exception in external_exceptions:
        edge = (
            f"{exception.get('kind')} {exception.get('from')} -> {exception.get('to')}"
        )
        _validate_debt_owner(entries, edge, exception.get("tracking_issue"), problems)
    if problems:
        raise ArchitectureError("\n".join(problems))


def validate_runtime_ownership_ledger(
    runtime: Any, issue_ledger: dict[str, Any]
) -> dict[str, Any]:
    """Validate the curated owner/writer/mirror and retirement baseline."""
    if not isinstance(runtime, dict) or runtime.get("schema_version") != 1:
        raise ArchitectureError(
            "runtime ownership ledger must be a schema_version 1 object"
        )
    baseline_commit = runtime.get("baseline_commit")
    if not isinstance(baseline_commit, str) or re.fullmatch(
        r"[0-9a-f]{40}", baseline_commit
    ) is None:
        raise ArchitectureError(
            "runtime ownership ledger needs a full lowercase baseline_commit"
        )
    commit_check = subprocess.run(
        ("git", "cat-file", "-e", f"{baseline_commit}^{{commit}}"),
        cwd=REPO_ROOT,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if commit_check.returncode != 0:
        raise ArchitectureError(
            f"runtime ownership baseline_commit {baseline_commit} is not a local commit"
        )
    ancestor_check = subprocess.run(
        ("git", "merge-base", "--is-ancestor", baseline_commit, "HEAD"),
        cwd=REPO_ROOT,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if ancestor_check.returncode != 0:
        raise ArchitectureError(
            f"runtime ownership baseline_commit {baseline_commit} is not an ancestor of HEAD"
        )

    external_tracking = runtime.get("external_tracking_issues")
    if not isinstance(external_tracking, list):
        raise ArchitectureError(
            "runtime ownership ledger external_tracking_issues must be an array"
        )
    known_issues = dict(issue_ledger["_entries"])
    known_issues.update(issue_ledger["_external_entries"])
    for index, entry in enumerate(external_tracking):
        if not isinstance(entry, dict):
            raise ArchitectureError(
                f"runtime external tracking issue {index} must be an object"
            )
        number = entry.get("number")
        state = entry.get("state")
        title = entry.get("title")
        role = entry.get("role")
        if type(number) is not int or number <= 0:
            raise ArchitectureError(
                f"runtime external tracking issue {index} needs a positive number"
            )
        if number in known_issues:
            raise ArchitectureError(
                f"runtime external tracking issue #{number} duplicates the architecture ledger"
            )
        if state not in LEDGER_ISSUE_STATES:
            raise ArchitectureError(
                f"runtime external tracking issue #{number} needs state open or closed"
            )
        if not isinstance(title, str) or not title.strip():
            raise ArchitectureError(
                f"runtime external tracking issue #{number} needs a non-empty title"
            )
        if not isinstance(role, str) or not role.strip():
            raise ArchitectureError(
                f"runtime external tracking issue #{number} needs a non-empty role"
            )
        known_issues[number] = entry

    ownership = runtime.get("world_session_responsibility_families")
    if not isinstance(ownership, dict):
        raise ArchitectureError(
            "runtime ownership ledger needs world_session_responsibility_families"
        )
    families = ownership.get("families")
    if not isinstance(families, list) or not families:
        raise ArchitectureError(
            "runtime ownership WorldSession families must be a non-empty array"
        )
    family_ids: set[str] = set()
    covered_field_names: set[str] = set()
    semantic_keys = {
        "current_storage",
        "sole_writer",
        "readers",
        "clock_lifetime",
        "persistence_publication_order",
        "mirror_direction",
        "target_owner",
        "rollback_condition",
        "retirement_condition",
    }
    for index, family in enumerate(families):
        if not isinstance(family, dict):
            raise ArchitectureError(
                f"runtime ownership WorldSession family {index} must be an object"
            )
        family_id = family.get("id")
        if not isinstance(family_id, str) or not family_id.strip():
            raise ArchitectureError(
                f"runtime ownership WorldSession family {index} needs a non-empty id"
            )
        if family_id in family_ids:
            raise ArchitectureError(
                f"duplicate runtime ownership WorldSession family: {family_id}"
            )
        family_ids.add(family_id)
        field_names = family.get("field_names")
        expected_field_count = family.get("expected_field_count")
        if not isinstance(field_names, list) or not field_names or any(
            not isinstance(field_name, str) or not field_name.strip()
            for field_name in field_names
        ):
            raise ArchitectureError(
                f"runtime ownership family {family_id} needs exact field_names"
            )
        if len(field_names) != len(set(field_names)):
            raise ArchitectureError(
                f"runtime ownership family {family_id} field_names contains duplicates"
            )
        if expected_field_count != len(field_names):
            raise ArchitectureError(
                f"runtime ownership family {family_id} expected_field_count does not "
                "match field_names"
            )
        overlap = sorted(covered_field_names & set(field_names))
        if overlap:
            raise ArchitectureError(
                f"runtime ownership family {family_id} overlaps fields: {overlap}"
            )
        covered_field_names.update(field_names)
        for key in semantic_keys:
            value = family.get(key)
            if not isinstance(value, str) or not value.strip():
                raise ArchitectureError(
                    f"runtime ownership family {family_id} needs non-empty {key}"
                )
        _validate_open_retirement_issues(
            family.get("cutover_issues"),
            f"runtime ownership family {family_id}",
            known_issues,
        )
    coverage_source = ownership.get("coverage_source")
    expected_total = ownership.get("expected_field_count")
    if expected_total is None and isinstance(coverage_source, dict):
        expected_total = coverage_source.get("expected_field_count")
    if expected_total != len(covered_field_names):
        raise ArchitectureError(
            "runtime ownership WorldSession family coverage does not match expected total"
        )

    inventories = runtime.get("inventories")
    required_inventories = {
        "session_resources",
        "player_broadcast_info",
        "session_command",
        "legacy_canonical_bridges",
        "sql_pool_access",
        "handler_ownership",
        "hotspots",
    }
    if not isinstance(inventories, dict) or set(inventories) != required_inventories:
        raise ArchitectureError(
            "runtime ownership inventories must define exactly: "
            + ", ".join(sorted(required_inventories))
        )
    for inventory_name, inventory in inventories.items():
        if not isinstance(inventory, dict):
            raise ArchitectureError(
                f"runtime ownership inventory {inventory_name} must be an object"
            )
        entries = inventory.get("entries")
        if not isinstance(entries, list) or not entries:
            raise ArchitectureError(
                f"runtime ownership inventory {inventory_name} needs entries"
            )
        entry_ids: set[str] = set()
        exact_member_key = None
        exact_count_key = None
        if inventory_name in {"session_resources", "player_broadcast_info"}:
            exact_member_key = "field_names"
            exact_count_key = "expected_field_count"
        elif inventory_name == "session_command":
            exact_member_key = "variants"
            exact_count_key = "expected_variant_count"
        covered_members: set[str] = set()
        for index, entry in enumerate(entries):
            if not isinstance(entry, dict):
                raise ArchitectureError(
                    f"runtime ownership inventory {inventory_name} entry {index} "
                    "must be an object"
                )
            if inventory_name == "hotspots":
                entry_id = entry.get("path")
                if not isinstance(entry_id, str) or not entry_id.strip():
                    raise ArchitectureError(
                        f"runtime ownership hotspot entry {index} needs a non-empty path"
                    )
            else:
                entry_id = entry.get("id", entry.get("path"))
            if not isinstance(entry_id, str) or not entry_id.strip():
                raise ArchitectureError(
                    f"runtime ownership inventory {inventory_name} entry {index} "
                    "needs an id or path"
                )
            if entry_id in entry_ids:
                raise ArchitectureError(
                    f"duplicate runtime ownership inventory {inventory_name} entry: "
                    f"{entry_id}"
                )
            entry_ids.add(entry_id)
            if exact_member_key is not None and exact_count_key is not None:
                members = entry.get(exact_member_key)
                if not isinstance(members, list) or not members or any(
                    not isinstance(member, str) or not member.strip()
                    for member in members
                ):
                    raise ArchitectureError(
                        f"runtime ownership inventory {inventory_name}/{entry_id} "
                        f"needs exact {exact_member_key}"
                    )
                if len(members) != len(set(members)):
                    raise ArchitectureError(
                        f"runtime ownership inventory {inventory_name}/{entry_id} "
                        f"has duplicate {exact_member_key}"
                    )
                if entry.get(exact_count_key) != len(members):
                    raise ArchitectureError(
                        f"runtime ownership inventory {inventory_name}/{entry_id} "
                        f"{exact_count_key} does not match {exact_member_key}"
                    )
                overlap = sorted(covered_members & set(members))
                if overlap:
                    raise ArchitectureError(
                        f"runtime ownership inventory {inventory_name}/{entry_id} "
                        f"overlaps members: {overlap}"
                    )
                covered_members.update(members)
            owner = entry.get("owner")
            retirement = entry.get("retirement_condition")
            if not isinstance(owner, str) or not owner.strip():
                raise ArchitectureError(
                    f"runtime ownership inventory {inventory_name}/{entry_id} needs an owner"
                )
            if not isinstance(retirement, str) or not retirement.strip():
                raise ArchitectureError(
                    f"runtime ownership inventory {inventory_name}/{entry_id} needs a "
                    "retirement_condition"
                )
            _validate_open_retirement_issues(
                entry.get("open_retirement_issues"),
                f"runtime ownership inventory {inventory_name}/{entry_id}",
                known_issues,
            )
            if inventory_name == "hotspots":
                production = entry.get("production_lines")
                tests = entry.get("test_lines")
                total = entry.get("total_lines")
                if any(type(value) is not int or value < 0 for value in (production, tests, total)):
                    raise ArchitectureError(
                        f"runtime ownership hotspot {entry_id} needs non-negative line counts"
                    )
                if production + tests != total:
                    raise ArchitectureError(
                        f"runtime ownership hotspot {entry_id} line counts do not add up"
                    )
        if exact_count_key is not None and inventory.get(exact_count_key) != len(
            covered_members
        ):
            raise ArchitectureError(
                f"runtime ownership inventory {inventory_name} exact coverage does not "
                "match its expected total"
            )
        if inventory_name == "handler_ownership":
            try:
                snapshot_lines = HANDLER_SNAPSHOT.read_text(encoding="utf-8").splitlines()
            except OSError as exc:
                raise ArchitectureError(
                    f"cannot read handler snapshot {HANDLER_SNAPSHOT}: {exc}"
                ) from exc
            data_lines = [
                line for line in snapshot_lines if line and not line.startswith("#")
            ]
            if not data_lines or not data_lines[0].startswith("opcode_value\t"):
                raise ArchitectureError("handler snapshot is missing its TSV header")
            actual_entries = len(data_lines) - 1
            coverage_source = inventory.get("coverage_source")
            audited_entries = (
                coverage_source.get("audited_entry_count")
                if isinstance(coverage_source, dict)
                else None
            )
            if audited_entries != actual_entries:
                raise ArchitectureError(
                    "runtime ownership handler audited_entry_count differs from the "
                    f"snapshot: ledger={audited_entries}, snapshot={actual_entries}"
                )
    return runtime


def _validate_open_retirement_issues(
    issue_numbers: Any,
    label: str,
    known_issues: dict[int, dict[str, Any]],
) -> None:
    if not isinstance(issue_numbers, list) or not issue_numbers or any(
        type(number) is not int or number <= 0 for number in issue_numbers
    ):
        raise ArchitectureError(f"{label} needs positive retirement issue numbers")
    if len(issue_numbers) != len(set(issue_numbers)):
        raise ArchitectureError(f"{label} has duplicate retirement issues")
    unknown = sorted(set(issue_numbers) - set(known_issues))
    if unknown:
        raise ArchitectureError(f"{label} references unknown issues: {unknown}")
    closed = sorted(
        number for number in issue_numbers if known_issues[number]["state"] != "open"
    )
    if closed:
        raise ArchitectureError(f"{label} references closed issues: {closed}")


def run_runtime_ownership_self_tests(
    runtime: dict[str, Any], issue_ledger: dict[str, Any]
) -> int:
    """Exercise semantic-ledger ratchets without copying the large baseline."""

    def clone() -> dict[str, Any]:
        return json.loads(json.dumps(runtime))

    cases: list[tuple[str, dict[str, Any], str]] = []

    duplicate_family = clone()
    duplicate_family["world_session_responsibility_families"]["families"].insert(
        -1,
        duplicate_family["world_session_responsibility_families"]["families"][0],
    )
    cases.append(
        ("duplicate-family", duplicate_family, "duplicate runtime ownership")
    )

    unknown_issue = clone()
    unknown_issue["inventories"]["session_resources"]["entries"][0][
        "open_retirement_issues"
    ] = [999999]
    cases.append(("unknown-retirement", unknown_issue, "references unknown issues"))

    missing_writer = clone()
    missing_writer["world_session_responsibility_families"]["families"][0][
        "sole_writer"
    ] = ""
    cases.append(("missing-writer", missing_writer, "needs non-empty sole_writer"))

    invalid_coverage = clone()
    invalid_coverage["world_session_responsibility_families"]["expected_field_count"] += 1
    cases.append(
        (
            "invalid-field-coverage",
            invalid_coverage,
            "family coverage does not match expected total",
        )
    )

    invalid_hotspot = clone()
    invalid_hotspot["inventories"]["hotspots"]["entries"][0]["total_lines"] += 1
    cases.append(("hotspot-arithmetic", invalid_hotspot, "line counts do not add up"))

    missing_hotspot_path = clone()
    missing_hotspot_path["inventories"]["hotspots"]["entries"][0].pop("path")
    cases.append(
        (
            "missing-hotspot-path",
            missing_hotspot_path,
            "needs a non-empty path",
        )
    )

    duplicate_hotspot_path = clone()
    duplicate_hotspot_path["inventories"]["hotspots"]["entries"][1]["path"] = (
        duplicate_hotspot_path["inventories"]["hotspots"]["entries"][0]["path"]
    )
    cases.append(
        (
            "duplicate-hotspot-path",
            duplicate_hotspot_path,
            "duplicate runtime ownership inventory hotspots entry",
        )
    )

    invented_commit = clone()
    invented_commit["baseline_commit"] = "f" * 40
    cases.append(("invented-commit", invented_commit, "is not a local commit"))

    stale_handler_count = clone()
    stale_handler_count["inventories"]["handler_ownership"]["coverage_source"][
        "audited_entry_count"
    ] += 1
    cases.append(
        (
            "stale-handler-count",
            stale_handler_count,
            "audited_entry_count differs from the snapshot",
        )
    )

    for name, candidate, expected in cases:
        try:
            validate_runtime_ownership_ledger(candidate, issue_ledger)
        except ArchitectureError as exc:
            if expected not in str(exc):
                raise ArchitectureError(
                    f"runtime ownership self-test {name} returned the wrong failure: {exc}"
                ) from exc
        else:
            raise ArchitectureError(
                f"runtime ownership self-test {name} was not rejected"
            )
    return len(cases)


def validate_runtime_syntax_coverage(
    runtime: dict[str, Any], syntax_policy: Any
) -> None:
    """Prove that the curated families cover the exact checked-in AST members."""
    if not isinstance(syntax_policy, dict) or syntax_policy.get("schema_version") != 1:
        raise ArchitectureError(
            "session ownership policy must be a schema_version 1 object"
        )
    syntax = syntax_policy.get("syntax_baseline")
    if not isinstance(syntax, dict):
        raise ArchitectureError("session ownership policy needs syntax_baseline")

    def syntax_names(section: str, member: str) -> list[str]:
        value = syntax.get(section)
        rows = value.get(member) if isinstance(value, dict) else None
        if not isinstance(rows, list) or any(
            not isinstance(row, dict)
            or not isinstance(row.get("name"), str)
            or not row["name"].strip()
            for row in rows
        ):
            raise ArchitectureError(
                f"session ownership syntax baseline {section}.{member} is malformed"
            )
        names = [row["name"] for row in rows]
        if len(names) != len(set(names)):
            raise ArchitectureError(
                f"session ownership syntax baseline {section}.{member} has duplicate names"
            )
        return names

    world_syntax = syntax_names("world_session", "fields")
    world_curated = [
        name
        for family in runtime["world_session_responsibility_families"]["families"]
        for name in family["field_names"]
    ]
    comparisons = [
        ("WorldSession fields", world_syntax, world_curated),
        (
            "SessionResources fields",
            syntax_names("session_resources", "fields"),
            [
                name
                for entry in runtime["inventories"]["session_resources"]["entries"]
                for name in entry["field_names"]
            ],
        ),
        (
            "PlayerBroadcastInfo fields",
            syntax_names("player_broadcast_info", "fields"),
            [
                name
                for entry in runtime["inventories"]["player_broadcast_info"]["entries"]
                for name in entry["field_names"]
            ],
        ),
        (
            "SessionCommand variants",
            syntax_names("session_command", "variants"),
            [
                name
                for entry in runtime["inventories"]["session_command"]["entries"]
                for name in entry["variants"]
            ],
        ),
    ]
    for label, actual, curated in comparisons:
        if set(actual) != set(curated):
            raise ArchitectureError(
                f"runtime ownership {label} do not match syntax baseline: "
                f"missing={sorted(set(actual) - set(curated))}, "
                f"stale={sorted(set(curated) - set(actual))}"
            )

    world_rows = syntax["world_session"]["fields"]
    production = sum(row.get("source_class") == "production" for row in world_rows)
    test_fixtures = sum(row.get("source_class") == "test_fixture" for row in world_rows)
    ownership = runtime["world_session_responsibility_families"]
    if production != ownership.get("expected_production_field_count"):
        raise ArchitectureError(
            "runtime ownership WorldSession production count differs from syntax baseline"
        )
    if test_fixtures != ownership.get("expected_test_fixture_field_count"):
        raise ArchitectureError(
            "runtime ownership WorldSession test-fixture count differs from syntax baseline"
        )


def _validate_debt_owner(
    entries: dict[int, dict[str, Any]],
    edge: str,
    tracking_issue: Any,
    problems: list[str],
) -> None:
    if type(tracking_issue) is not int:
        return  # validate_policy already rejects malformed ownership
    entry = entries.get(tracking_issue)
    if entry is None:
        problems.append(
            f"exception {edge} tracks issue #{tracking_issue}, which is absent "
            "from the architecture issue ledger"
        )
    elif entry["state"] != "open":
        problems.append(
            f"exception {edge} is still owned by completed issue "
            f"#{tracking_issue}; retarget it to the slice that can actually "
            "remove the edge"
        )


def validate_documented_sequence(
    ledger: dict[str, Any], doc_path: pathlib.Path = ARCHITECTURE_DOC
) -> None:
    """The human refactor sequence in the architecture doc must be the ledger's."""
    try:
        text = doc_path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ArchitectureError(f"cannot read {doc_path}: {exc}") from exc
    section = re.search(
        r"^## Refactor sequence\s*$([\s\S]*?)(?:^## |\Z)", text, re.MULTILINE
    )
    if section is None:
        raise ArchitectureError(
            f"{doc_path} has no '## Refactor sequence' section to reconcile "
            "with the issue ledger"
        )
    documented: list[int] = []
    for line in section.group(1).splitlines():
        if re.match(r"^\d+\. ", line):
            documented.extend(int(number) for number in re.findall(r"#(\d+)", line))
    if not documented:
        raise ArchitectureError(
            f"{doc_path} refactor sequence lists no issues to reconcile with "
            "the issue ledger"
        )
    if documented != ledger["sequence"]:
        raise ArchitectureError(
            "documented refactor sequence does not match the architecture issue "
            f"ledger: doc lists {documented}, ledger lists {ledger['sequence']}"
        )


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


def classify_external_dependency(
    policy: dict[str, Any],
    source: str,
    target: str,
    target_source: str,
    kind: str,
) -> tuple[str, str]:
    package_categories = policy["_package_categories"]
    source_category = package_categories.get(source)
    if source not in policy["_external_protected_packages"]:
        return "allowed", f"{source_category} does not have a guarded external surface"

    edge = (source, target, target_source, kind)
    if edge in policy["_external_allowed_edges"]:
        return "allowed", "explicit direct external dependency"

    exception = policy["_external_exception_map"].get(edge)
    if exception is not None:
        return (
            "exception",
            f"baseline external exception tracked by #{exception['tracking_issue']}",
        )

    return (
        "forbidden",
        f"{source_category} package {source} has an explicit direct external-dependency "
        f"surface; {target} comes from {target_source!r}. Review and declare legitimate "
        "libraries, or add an issue-linked exception for temporary infrastructure debt",
    )


def cargo_metadata() -> dict[str, Any]:
    command = list(CARGO_METADATA_COMMAND)
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
    metadata = parse_json(completed.stdout, "cargo metadata output")
    if not isinstance(metadata, dict):
        raise ArchitectureError("cargo metadata did not return an object")
    return metadata


def package_id_url_basename(package_id_base: str, prefix: str) -> str | None:
    # Cargo 1.88 PackageIdSpec::fmt omits the name only when the URL's final
    # path segment equals it exactly. Do not decode, trim, or strip suffixes.
    if not package_id_base.startswith(prefix):
        return None
    path = urlsplit(package_id_base[len(prefix) :]).path
    if not path:
        return None
    basename = path.rsplit("/", 1)[-1]
    return basename or None


def package_id_fragment_matches(
    package_id_base: str,
    package_id_fragment: str,
    package_name: str,
    package_version: str,
    prefix: str,
) -> bool:
    if package_id_fragment == f"{package_name}@{package_version}":
        return True
    if package_id_fragment != package_version:
        return False
    return package_id_url_basename(package_id_base, prefix) == package_name


def external_package_source(package: dict[str, Any], package_id: str) -> str:
    package_name = package.get("name")
    package_version = package.get("version")
    if not isinstance(package_name, str) or not package_name:
        raise ArchitectureError(
            f"external package {package_id} has invalid name metadata {package_name!r}"
        )
    if not isinstance(package_version, str) or not package_version:
        raise ArchitectureError(
            f"external package {package_id} has invalid version metadata "
            f"{package_version!r}"
        )
    package_id_base, separator, package_id_fragment = package_id.rpartition("#")
    if "source" not in package:
        raise ArchitectureError(
            f"external package {package_id} is missing source metadata"
        )
    source = package["source"]
    if source is None:
        if not package_id_base.startswith("path+"):
            raise ArchitectureError(
                f"external package {package_id} has null source but is not a path package"
            )
        if not separator or not package_id_fragment_matches(
            package_id_base,
            package_id_fragment,
            package_name,
            package_version,
            "path+",
        ):
            raise ArchitectureError(
                f"external path package id {package_id!r} does not identify "
                f"{package_name}@{package_version}"
            )
        return package_id
    if not isinstance(source, str) or not source:
        raise ArchitectureError(
            f"external package {package_id} has invalid source metadata {source!r}"
        )
    if source.startswith("registry+"):
        source_base = source
        fragment_matches = package_id_fragment_matches(
            package_id_base,
            package_id_fragment,
            package_name,
            package_version,
            "registry+",
        )
    elif source.startswith("git+"):
        source_base, precise_separator, precise = source.rpartition("#")
        if not precise_separator or not precise:
            raise ArchitectureError(
                f"external git package {package_id} has no precise revision in "
                f"source metadata {source!r}"
            )
        fragment_matches = package_id_fragment_matches(
            package_id_base,
            package_id_fragment,
            package_name,
            package_version,
            "git+",
        )
    else:
        raise ArchitectureError(
            f"external package {package_id} has unsupported source metadata {source!r}"
        )
    if not separator or not fragment_matches:
        raise ArchitectureError(
            f"external sourced package id {package_id!r} does not identify "
            f"{package_name}@{package_version}"
        )
    if package_id_base != source_base:
        raise ArchitectureError(
            f"external package {package_id} is inconsistent with source metadata "
            f"{source!r}"
        )
    return source


def production_edges(
    metadata: dict[str, Any],
) -> tuple[
    set[str],
    set[tuple[str, str, str]],
    set[tuple[str, str, str, str]],
]:
    packages = metadata.get("packages")
    workspace_members = metadata.get("workspace_members")
    workspace_default_members = metadata.get("workspace_default_members")
    resolve = metadata.get("resolve")
    if (
        not isinstance(packages, list)
        or not isinstance(workspace_members, list)
        or not isinstance(workspace_default_members, list)
        or not isinstance(resolve, dict)
    ):
        raise ArchitectureError(
            "cargo metadata is missing packages, workspace members/default members, "
            "or resolve"
        )

    package_by_id: dict[str, dict[str, Any]] = {}
    for package in packages:
        if not isinstance(package, dict):
            raise ArchitectureError("cargo metadata packages contains a non-object entry")
        package_id = package.get("id")
        if not isinstance(package_id, str) or not package_id:
            raise ArchitectureError("cargo metadata package has an invalid id")
        if package_id in package_by_id:
            raise ArchitectureError(f"cargo metadata has duplicate package id {package_id}")
        package_by_id[package_id] = package

    def unique_id_set(values: list[Any], label: str) -> set[str]:
        ids: set[str] = set()
        for value in values:
            if not isinstance(value, str) or not value:
                raise ArchitectureError(f"cargo metadata {label} has an invalid id")
            if value in ids:
                raise ArchitectureError(
                    f"cargo metadata {label} has duplicate id {value}"
                )
            ids.add(value)
        return ids

    member_ids = unique_id_set(workspace_members, "workspace_members")
    default_member_ids = unique_id_set(
        workspace_default_members, "workspace_default_members"
    )
    if default_member_ids != member_ids:
        raise ArchitectureError(
            "workspace_default_members differs from workspace_members; "
            "cargo metadata --all-features would not prove every optional direct "
            "dependency, so update the checker before narrowing default-members"
        )
    missing_members = member_ids - set(package_by_id)
    if missing_members:
        raise ArchitectureError(
            f"cargo metadata omitted workspace members: {sorted(missing_members)}"
        )
    for package_id in member_ids:
        package = package_by_id[package_id]
        if (
            "source" not in package
            or package["source"] is not None
            or not package_id.startswith("path+")
        ):
            raise ArchitectureError(
                f"workspace member {package_id} must be a source-null path package"
            )
        external_package_source(package, package_id)

    member_names = {
        package_by_id[package_id].get("name") for package_id in member_ids
    }
    if None in member_names or len(member_names) != len(member_ids):
        raise ArchitectureError("workspace package names are missing or duplicated")

    resolve_nodes = resolve.get("nodes")
    if not isinstance(resolve_nodes, list):
        raise ArchitectureError("cargo metadata resolve is missing nodes")
    node_by_id: dict[str, dict[str, Any]] = {}
    for node in resolve_nodes:
        if not isinstance(node, dict):
            raise ArchitectureError(
                "cargo metadata resolve nodes contains a non-object entry"
            )
        node_id = node.get("id")
        if not isinstance(node_id, str) or not node_id:
            raise ArchitectureError("cargo metadata resolve node has an invalid id")
        if node_id in node_by_id:
            raise ArchitectureError(
                f"cargo metadata resolve has duplicate node id {node_id}"
            )
        node_by_id[node_id] = node
    missing_member_nodes = member_ids - set(node_by_id)
    if missing_member_nodes:
        raise ArchitectureError(
            "cargo metadata resolve omitted workspace members: "
            f"{sorted(missing_member_nodes)}"
        )

    workspace_edges: set[tuple[str, str, str]] = set()
    external_edges: set[tuple[str, str, str, str]] = set()
    external_identity_ids: dict[
        tuple[str, str, str, str], set[str]
    ] = {}
    for package_id in member_ids:
        package = package_by_id[package_id]
        source = package["name"]
        dependencies = node_by_id[package_id].get("deps")
        if not isinstance(dependencies, list):
            raise ArchitectureError(
                f"package {source} has invalid resolved dependency metadata"
            )
        for dependency in dependencies:
            if not isinstance(dependency, dict):
                raise ArchitectureError(
                    f"package {source} has a non-object resolved dependency entry"
                )
            target_id = dependency.get("pkg")
            dependency_kinds = dependency.get("dep_kinds")
            if not isinstance(target_id, str) or not isinstance(
                dependency_kinds, list
            ) or not dependency_kinds:
                raise ArchitectureError(
                    f"package {source} has an invalid resolved dependency entry"
                )
            target_package = package_by_id.get(target_id)
            if target_package is None or not isinstance(
                target_package.get("name"), str
            ):
                raise ArchitectureError(
                    f"resolved dependency of {source} references unknown package "
                    f"{target_id}"
                )
            target = target_package["name"]
            for dependency_kind in dependency_kinds:
                if not isinstance(dependency_kind, dict):
                    raise ArchitectureError(
                        f"dependency {source} -> {target} has invalid kind metadata"
                    )
                if "kind" not in dependency_kind:
                    raise ArchitectureError(
                        f"dependency {source} -> {target} is missing kind metadata"
                    )
                raw_kind = dependency_kind["kind"]
                if raw_kind is None:
                    kind = "normal"
                elif isinstance(raw_kind, str):
                    kind = raw_kind
                else:
                    raise ArchitectureError(
                        f"dependency {source} -> {target} has unsupported kind "
                        f"{raw_kind!r}; update the architecture checker explicitly"
                    )
                if kind in IGNORED_DEPENDENCY_KINDS:
                    continue
                if kind not in PRODUCT_DEPENDENCY_KINDS:
                    raise ArchitectureError(
                        f"dependency {source} -> {target} has unsupported kind "
                        f"{kind!r}; update the architecture checker explicitly"
                    )
                if target_id in member_ids:
                    workspace_edges.add((source, target, kind))
                else:
                    edge = (
                        source,
                        target,
                        external_package_source(target_package, target_id),
                        kind,
                    )
                    external_edges.add(edge)
                    external_identity_ids.setdefault(edge, set()).add(target_id)
    ambiguous_external_identities = [
        (identity, package_ids)
        for identity, package_ids in sorted(external_identity_ids.items())
        if len(package_ids) > 1
    ]
    if ambiguous_external_identities:
        details = "; ".join(
            f"{kind} {source} -> {target} from {target_source!r} resolves to "
            f"multiple package ids {sorted(package_ids)}"
            for (
                source,
                target,
                target_source,
                kind,
            ), package_ids in ambiguous_external_identities
        )
        raise ArchitectureError(
            "ambiguous direct external dependency identity: " + details
        )
    return member_names, workspace_edges, external_edges


def check_dependencies(
    policy: dict[str, Any], metadata: dict[str, Any]
) -> tuple[int, int, int, int, int]:
    workspace_packages, workspace_edges, external_edges = production_edges(metadata)
    policy_packages = set(policy["_package_categories"])
    missing = sorted(workspace_packages - policy_packages)
    stale = sorted(policy_packages - workspace_packages)
    errors: list[str] = []
    if missing:
        errors.append(f"unclassified workspace packages: {', '.join(missing)}")
    if stale:
        errors.append(f"classified packages no longer in workspace: {', '.join(stale)}")

    used_restricted_allowed: set[tuple[str, str]] = set()
    used_exceptions: set[tuple[str, str]] = set()
    for source, target, kind in sorted(workspace_edges):
        decision, reason = classify_edge(policy, source, target)
        edge = (source, target)
        if decision == "allowed" and edge in policy["_restricted_allowed_edges"]:
            used_restricted_allowed.add(edge)
        elif decision == "exception":
            used_exceptions.add((source, target))
        elif decision == "forbidden":
            errors.append(f"forbidden {kind} edge {source} -> {target}: {reason}")

    stale_restricted_allowed = sorted(
        policy["_restricted_allowed_edges"] - used_restricted_allowed
    )
    for source, target in stale_restricted_allowed:
        errors.append(
            f"obsolete restricted-package allowance {source} -> {target}; "
            "remove it from the policy"
        )

    stale_exceptions = obsolete_exception_edges(policy, used_exceptions)
    for source, target in stale_exceptions:
        errors.append(
            f"obsolete baseline exception {source} -> {target}; remove it from the policy"
        )

    external_policy_targets = {
        target
        for _, target, _, _ in (
            policy["_external_allowed_edges"]
            | set(policy["_external_exception_map"])
        )
    }
    external_targets_now_in_workspace = sorted(
        external_policy_targets & workspace_packages
    )
    if external_targets_now_in_workspace:
        errors.append(
            "external dependency policy references workspace packages: "
            f"{', '.join(external_targets_now_in_workspace)}"
        )

    used_external_allowed: set[tuple[str, str, str, str]] = set()
    used_external_exceptions: set[tuple[str, str, str, str]] = set()
    for source, target, target_source, kind in sorted(external_edges):
        decision, reason = classify_external_dependency(
            policy, source, target, target_source, kind
        )
        edge = (source, target, target_source, kind)
        if decision == "allowed" and edge in policy["_external_allowed_edges"]:
            used_external_allowed.add(edge)
        elif decision == "exception":
            used_external_exceptions.add(edge)
        elif decision == "forbidden":
            errors.append(
                f"forbidden direct external {kind} dependency "
                f"{source} -> {target} from {target_source!r}: {reason}"
            )

    stale_external_allowed = sorted(
        policy["_external_allowed_edges"] - used_external_allowed
    )
    for source, target, target_source, kind in stale_external_allowed:
        errors.append(
            f"obsolete allowed external {kind} dependency {source} -> {target} "
            f"from {target_source!r}; remove it from the policy"
        )

    stale_external_exceptions = sorted(
        set(policy["_external_exception_map"]) - used_external_exceptions
    )
    for source, target, target_source, kind in stale_external_exceptions:
        errors.append(
            f"obsolete baseline external exception {kind} {source} -> {target} "
            f"from {target_source!r}; remove it from the policy"
        )

    if errors:
        raise ArchitectureError("\n".join(errors))
    guarded_external_edges = used_external_allowed | used_external_exceptions
    return (
        len(workspace_packages),
        len(workspace_edges),
        len(used_exceptions),
        len(guarded_external_edges),
        len(used_external_exceptions),
    )


def obsolete_exception_edges(
    policy: dict[str, Any], used_exceptions: set[tuple[str, str]]
) -> list[tuple[str, str]]:
    return sorted(set(policy["_exception_map"]) - used_exceptions)


RUST_WHITESPACE = b" \t\r\n"
RUST_IDENTIFIER_BYTES = frozenset(
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_"
)
RUST_LITERAL_OR_COMMENT_TRANSLATION = bytes(
    byte if byte in (ord("\n"), ord("\r")) else ord(" ")
    for byte in range(256)
)
EXACT_CFG_TEST_ATTRIBUTE = re.compile(
    rb"\s*cfg\s*\(\s*test\s*\)\s*\Z"
)


def blank_rust_noncode(masked: bytearray, start: int, end: int) -> None:
    """Blank a comment/literal while preserving offsets and line boundaries."""
    masked[start:end] = masked[start:end].translate(
        RUST_LITERAL_OR_COMMENT_TRANSLATION
    )


def rust_raw_string_end(source: bytes, start: int) -> int | None:
    """Return the end of a raw string beginning at start, if one begins there."""
    if start > 0 and source[start - 1] in RUST_IDENTIFIER_BYTES:
        return None
    prefix_length = 0
    for prefix in (b"br", b"cr", b"r"):
        if source.startswith(prefix, start):
            prefix_length = len(prefix)
            break
    if not prefix_length:
        return None

    cursor = start + prefix_length
    while cursor < len(source) and source[cursor] == ord("#"):
        cursor += 1
    hashes = source[start + prefix_length : cursor]
    if cursor >= len(source) or source[cursor] != ord('"'):
        return None

    terminator = b'"' + hashes
    closing = source.find(terminator, cursor + 1)
    return len(source) if closing < 0 else closing + len(terminator)


def rust_char_literal_end(source: bytes, start: int) -> int | None:
    """Distinguish a Rust character literal from a lifetime or loop label."""
    cursor = start + 1
    if cursor >= len(source) or source[cursor] in (ord("\n"), ord("\r")):
        return None
    if source[cursor] == ord("\\"):
        cursor += 1
        if cursor >= len(source):
            return None
        if source[cursor] == ord("x"):
            cursor += 3
        elif source[cursor] == ord("u") and cursor + 1 < len(source):
            if source[cursor + 1] != ord("{"):
                return None
            closing = source.find(b"}", cursor + 2)
            if closing < 0:
                return None
            cursor = closing + 1
        else:
            cursor += 1
    else:
        first = source[cursor]
        if first < 0x80:
            cursor += 1
        elif first & 0xE0 == 0xC0:
            cursor += 2
        elif first & 0xF0 == 0xE0:
            cursor += 3
        elif first & 0xF8 == 0xF0:
            cursor += 4
        else:
            return None
    if cursor < len(source) and source[cursor] == ord("'"):
        return cursor + 1
    return None


def mask_rust_noncode(source: str) -> bytes:
    """Lexically blank Rust comments and literals without changing byte offsets."""
    original = source.encode("utf-8")
    masked = bytearray(original)
    cursor = 0
    while cursor < len(original):
        if original.startswith(b"//", cursor):
            end = original.find(b"\n", cursor + 2)
            end = len(original) if end < 0 else end
            blank_rust_noncode(masked, cursor, end)
            cursor = end
            continue
        if original.startswith(b"/*", cursor):
            depth = 1
            end = cursor + 2
            while end < len(original) and depth:
                if original.startswith(b"/*", end):
                    depth += 1
                    end += 2
                elif original.startswith(b"*/", end):
                    depth -= 1
                    end += 2
                else:
                    end += 1
            blank_rust_noncode(masked, cursor, end)
            cursor = end
            continue
        raw_end = None
        if original[cursor] in (ord("b"), ord("c"), ord("r")):
            raw_end = rust_raw_string_end(original, cursor)
        if raw_end is not None:
            blank_rust_noncode(masked, cursor, raw_end)
            cursor = raw_end
            continue
        if original[cursor] == ord('"'):
            end = cursor + 1
            while end < len(original):
                if original[end] == ord("\\"):
                    end = min(end + 2, len(original))
                elif original[end] == ord('"'):
                    end += 1
                    break
                else:
                    end += 1
            blank_rust_noncode(masked, cursor, end)
            cursor = end
            continue
        if original[cursor] == ord("'"):
            char_end = rust_char_literal_end(original, cursor)
            if char_end is not None:
                blank_rust_noncode(masked, cursor, char_end)
                cursor = char_end
                continue
        cursor += 1
    return bytes(masked)


def skip_rust_whitespace(source: bytes, cursor: int) -> int:
    while cursor < len(source) and source[cursor] in RUST_WHITESPACE:
        cursor += 1
    return cursor


def matching_rust_delimiter(
    source: bytes, start: int, opening: int, closing: int
) -> int | None:
    depth = 0
    for cursor in range(start, len(source)):
        if source[cursor] == opening:
            depth += 1
        elif source[cursor] == closing:
            depth -= 1
            if depth == 0:
                return cursor
    return None


def rust_identifier(source: bytes, cursor: int) -> tuple[str | None, int]:
    if cursor >= len(source) or source[cursor] not in RUST_IDENTIFIER_BYTES:
        return None, cursor
    end = cursor + 1
    while end < len(source) and source[end] in RUST_IDENTIFIER_BYTES:
        end += 1
    return source[cursor:end].decode("ascii"), end


def skip_outer_rust_attributes(source: bytes, cursor: int) -> int:
    """Skip attributes following the cfg(test) attribute on the same item."""
    while True:
        candidate = skip_rust_whitespace(source, cursor)
        if candidate >= len(source) or source[candidate] != ord("#"):
            return candidate
        bracket = skip_rust_whitespace(source, candidate + 1)
        if bracket >= len(source) or source[bracket] != ord("["):
            return candidate
        closing = matching_rust_delimiter(
            source, bracket, ord("["), ord("]")
        )
        if closing is None:
            raise ArchitectureError("unterminated Rust attribute in hotspot source")
        cursor = closing + 1


def cfg_test_item_kind(source: bytes, cursor: int) -> tuple[str, int]:
    """Classify the top-level Rust item attached to an exact cfg(test)."""
    cursor = skip_outer_rust_attributes(source, cursor)
    token, end = rust_identifier(source, cursor)
    if token == "pub":
        cursor = skip_rust_whitespace(source, end)
        if cursor < len(source) and source[cursor] == ord("("):
            closing = matching_rust_delimiter(
                source, cursor, ord("("), ord(")")
            )
            if closing is None:
                raise ArchitectureError("unterminated Rust visibility in hotspot source")
            cursor = closing + 1
        cursor = skip_rust_whitespace(source, cursor)

    modifiers: set[str] = set()
    while True:
        token, end = rust_identifier(source, cursor)
        if token not in {"async", "unsafe", "default", "auto", "extern"}:
            break
        modifiers.add(token)
        cursor = skip_rust_whitespace(source, end)

    token, end = rust_identifier(source, cursor)
    if token == "const":
        lookahead = skip_rust_whitespace(source, end)
        while True:
            following, following_end = rust_identifier(source, lookahead)
            if following not in {"async", "unsafe", "extern"}:
                break
            lookahead = skip_rust_whitespace(source, following_end)
        if following == "fn":
            return "body-or-semicolon", following_end
        return "semicolon", end
    if token in {"fn", "mod", "struct", "enum", "union", "trait", "impl"}:
        return "body-or-semicolon", end
    if token in {"use", "type", "static"}:
        return "semicolon", end
    if token in {"macro", "macro_rules"}:
        return "body-or-semicolon", end
    if "extern" in modifiers:
        if token in {"crate", "static", "type"}:
            return "semicolon", end
        if token == "fn" or token is None:
            return "body-or-semicolon", end
    raise ArchitectureError(
        "unsupported top-level Rust item following exact #[cfg(test)]"
    )


def cfg_test_item_end(source: bytes, cursor: int, kind: str) -> int:
    round_depth = 0
    square_depth = 0
    curly_depth = 0
    angle_depth = 0
    while cursor < len(source):
        byte = source[cursor]
        if byte == ord("("):
            round_depth += 1
        elif byte == ord(")"):
            round_depth -= 1
        elif byte == ord("["):
            square_depth += 1
        elif byte == ord("]"):
            square_depth -= 1
        elif (
            kind == "body-or-semicolon"
            and byte == ord("<")
            and not round_depth
            and not square_depth
        ):
            angle_depth += 1
        elif (
            kind == "body-or-semicolon"
            and byte == ord(">")
            and angle_depth
            and (cursor == 0 or source[cursor - 1] != ord("-"))
        ):
            angle_depth -= 1
        elif byte == ord("{"):
            if (
                kind == "body-or-semicolon"
                and not round_depth
                and not square_depth
                and not curly_depth
                and not angle_depth
            ):
                closing = matching_rust_delimiter(
                    source, cursor, ord("{"), ord("}")
                )
                if closing is None:
                    raise ArchitectureError(
                        "unterminated cfg(test) Rust item body in hotspot source"
                    )
                return closing + 1
            curly_depth += 1
        elif byte == ord("}"):
            curly_depth -= 1
        elif (
            byte == ord(";")
            and not round_depth
            and not square_depth
            and not curly_depth
            and not angle_depth
        ):
            return cursor + 1
        cursor += 1
    raise ArchitectureError("unterminated cfg(test) Rust item in hotspot source")


def top_level_cfg_test_item_spans(source: str) -> list[tuple[int, int]]:
    """Return exact byte spans of test-only top-level items in a Rust file."""
    masked = mask_rust_noncode(source)
    spans: list[tuple[int, int]] = []
    brace_depth = 0
    cursor = 0
    while cursor < len(masked):
        byte = masked[cursor]
        if byte == ord("{"):
            brace_depth += 1
        elif byte == ord("}"):
            brace_depth -= 1
        elif byte == ord("#") and brace_depth == 0:
            bracket = skip_rust_whitespace(masked, cursor + 1)
            if bracket < len(masked) and masked[bracket] == ord("["):
                closing = matching_rust_delimiter(
                    masked, bracket, ord("["), ord("]")
                )
                if closing is None:
                    raise ArchitectureError(
                        "unterminated top-level Rust attribute in hotspot source"
                    )
                if EXACT_CFG_TEST_ATTRIBUTE.fullmatch(
                    masked[bracket + 1 : closing]
                ):
                    item_start = closing + 1
                    kind, search_start = cfg_test_item_kind(masked, item_start)
                    item_end = cfg_test_item_end(masked, search_start, kind)
                    spans.append((cursor, item_end))
        cursor += 1
    return spans


def top_level_cfg_test_line_indexes(source: str) -> set[int]:
    """Return physical line indexes touched by top-level cfg(test) items."""
    test_line_indexes: set[int] = set()
    encoded = source.encode("utf-8")
    for start, end in top_level_cfg_test_item_spans(source):
        first_line = encoded.count(b"\n", 0, start)
        last_line = encoded.count(b"\n", 0, max(start, end - 1))
        test_line_indexes.update(range(first_line, last_line + 1))
    return test_line_indexes


def file_is_entirely_cfg_test(source: str) -> bool:
    """Whether an inner `#![cfg(test)]` makes the whole file test-only.

    A test module extracted to its own file carries its `#[cfg(test)]` on the
    `mod` declaration in the parent, not inside the file, so the extent scan
    below finds nothing and counts every line as production. That inverts the
    metric: moving 94k lines of tests out of a hotspot would report the hotspot
    unchanged and a new 94k production file appearing.

    Only inner attributes and inner doc comments may precede the first item, so
    reading until the first other line is enough to decide.
    """
    for line in source.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("//"):
            continue
        if stripped == "#![cfg(test)]":
            return True
        if stripped.startswith("#!["):
            continue
        return False
    return False


def hotspot_line_counts(source: str) -> tuple[int, int, int]:
    """Partition physical lines by exact top-level cfg(test) item extents."""
    lines = source.splitlines()
    if file_is_entirely_cfg_test(source):
        return len(lines), 0, len(lines)
    test_lines = len(top_level_cfg_test_line_indexes(source))
    production_lines = len(lines) - test_lines
    return len(lines), production_lines, test_lines


def run_path_module_scanner_self_tests() -> int:
    """Prove a `#[path]` counts as a mount only where it is code."""
    source = "\n".join(
        [
            '/// Uses `#[path = "prose.rs"]` in prose',
            '// #[path = "commented.rs"]',
            '/* #[path = "blocked.rs"] */',
            'const SQL: &str = "#[path = \\"quoted.rs\\"]";',
            "#[cfg(test)]",
            '#[path = "real_tests.rs"]',
            "mod tests;",
            '#[path = "plain.rs"]',
            "mod plain;",
            '#[path = r"rawly.rs"]',
            "mod rawly;",
            '#[path = r#"hashed.rs"#]',
            "mod hashed;",
            '#[path = "escaped\\x2ers"]',
            "mod escaped;",
        ]
    )
    is_code = rust_code_offsets(source)
    # Every `#[path` in the fixture, classified by whether it starts in code.
    # Located by scanning rather than by exact-string lookup, because the decoy
    # inside the string literal is written with escaped quotes and would not
    # match the plain form.
    found = set()
    for match in re.finditer(r"#\[path", source):
        if not is_code[match.start()]:
            continue
        declaration = PATH_MODULE_DECLARATION.search(source[match.start() :])
        found.add(path_module_target(declaration) if declaration else "?")
    expected = {"real_tests.rs", "plain.rs", "rawly.rs", "hashed.rs", "escaped.rs"}
    if found != expected:
        raise ArchitectureError(
            "path scanner self-test failed: declarations counted as code were "
            f"{sorted(found)}, expected {sorted(expected)}"
        )
    return 1


def run_hotspot_classifier_self_tests() -> int:
    """Prove cfg(test) extents survive lexical traps and stop at item ends."""
    lines = [
        "pub fn production_before() {}",
        "#[cfg(test)]",
        "mod first {",
        '    const COOKED: &str = "} // not syntax";',
        '    const RAW: &str = r###"} /* not syntax */"###;',
        "    const CLOSE: char = '}' ;",
        "    // } #[cfg(test)]",
        "    /* { outer /* } */ } */",
        "    fn still_inside<'a>(value: &'a str) -> &'a str { value }",
        "}",
        "pub fn production_between() {}",
        "#[cfg(test)]",
        "fn helper() {",
        "    let _ = '{';",
        "}",
        "#[cfg(test)]",
        "use crate::{",
        "    alpha,",
        "    beta,",
        "};",
        "#[cfg(test)]",
        "#[allow(dead_code)]",
        "pub(crate) mod second {",
        '    const NOTE: &str = "}";',
        "}",
        "#[cfg(not(test))]",
        "mod production_not_test {}",
        '#[cfg(any(test, feature = "fixture"))]',
        "fn production_possible() {}",
        'const FAKE: &str = "#[cfg(test)] mod fake { }";',
        "pub fn production_after() {}",
    ]
    source = "\n".join(lines) + "\n"
    expected = (
        set(range(1, 10))
        | set(range(11, 15))
        | set(range(15, 20))
        | set(range(20, 25))
    )
    actual = top_level_cfg_test_line_indexes(source)
    if actual != expected:
        raise ArchitectureError(
            "hotspot classifier adversarial self-test failed: "
            f"expected lines {sorted(expected)}, got {sorted(actual)}"
        )
    total, production, tests = hotspot_line_counts(source)
    expected_counts = (len(lines), len(lines) - len(expected), len(expected))
    if (total, production, tests) != expected_counts:
        raise ArchitectureError(
            "hotspot classifier line partition self-test failed: "
            f"got {(total, production, tests)}"
        )
    return 1


HOTSPOT_ROW_CACHE: dict[pathlib.Path, tuple[int, int, int, str]] = {}


# Both string forms Rust accepts here. `#[path = r"child.rs"]` is valid and
# rustfmt-stable, and a pattern that only knew ordinary literals left that child
# as a row of its own -- its lines uncharged to the hotspot that mounts it, which
# is the ceiling bypass this aggregation exists to close.
PATH_MODULE_DECLARATION = re.compile(
    r'#\[path\s*=\s*(?:r(?P<hashes>\#*)"(?P<raw>.*?)"(?P=hashes)'
    r'|"(?P<plain>(?:[^"\\]|\\.)*)")\s*\]'
)

RUST_STRING_ESCAPE = re.compile(r"\\(x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f]{1,6}\}|.)", re.DOTALL)

RUST_SIMPLE_ESCAPES = {
    "n": "\n",
    "r": "\r",
    "t": "\t",
    "0": "\0",
    "\\": "\\",
    '"': '"',
    "'": "'",
}


def decode_rust_string(literal: str) -> str:
    """Resolve the escapes in an ordinary Rust string literal.

    `#[path = "foo\\x2ers"]` is a valid spelling of `foo.rs` and rustc mounts
    the same file either way. Returning the undecoded text meant the child was
    never found, so its lines went uncharged to the hotspot that mounts it --
    the same ceiling bypass as the raw-string form, in a different disguise.
    """

    def replace(match: re.Match[str]) -> str:
        body = match.group(1)
        if body.startswith("x"):
            return chr(int(body[1:], 16))
        if body.startswith("u{"):
            return chr(int(body[2:-1], 16))
        # A line continuation swallows the newline and the indent after it.
        if body == "\n":
            return ""
        return RUST_SIMPLE_ESCAPES.get(body, body)

    return RUST_STRING_ESCAPE.sub(replace, literal)


def path_module_target(match: re.Match[str]) -> str:
    """The file a `#[path]` match names, whichever string form it used.

    Raw strings have no escapes by definition, so only the ordinary form is
    decoded.
    """
    raw = match.group("raw")
    return raw if raw is not None else decode_rust_string(match.group("plain"))


def rust_code_offsets(source: str) -> list[bool]:
    """Mark which byte offsets are code rather than comment or string interior.

    A `#[path = "x.rs"]` written inside a doc comment or a string literal is not
    a mount, and matching one would aggregate an unrelated file into a hotspot
    and drop it from the report -- a sentence about the attribute would move a
    ceiling. The attribute's own value *is* a string, so the interiors cannot
    simply be blanked; instead a match is accepted only when it *starts* in
    code, and then read from the original text. Rust's side of this uses `syn`
    and is immune; this is the text-scanning half catching up.
    """
    is_code = [True] * len(source)
    index = 0
    length = len(source)
    while index < length:
        pair = source[index : index + 2]
        if pair == "//":
            end = source.find("\n", index)
            end = length if end == -1 else end
            for offset in range(index, end):
                is_code[offset] = False
            index = end
        elif pair == "/*":
            depth = 1
            scan = index + 2
            while scan < length and depth:
                if source[scan : scan + 2] == "/*":
                    depth += 1
                    scan += 2
                elif source[scan : scan + 2] == "*/":
                    depth -= 1
                    scan += 2
                else:
                    scan += 1
            for offset in range(index, scan):
                is_code[offset] = False
            index = scan
        elif source[index] == '"':
            scan = index + 1
            while scan < length:
                if source[scan] == "\\":
                    scan += 2
                    continue
                if source[scan] == '"':
                    scan += 1
                    break
                scan += 1
            # The quotes stay code; only the interior is masked, so an attribute
            # keeps its own value while a `#[path ..]` written inside a string
            # starts in masked text and is refused.
            for offset in range(index + 1, min(scan - 1, length)):
                is_code[offset] = False
            index = scan
        else:
            index += 1
    return is_code


def path_module_children(path: pathlib.Path, source: str) -> list[tuple[pathlib.Path, bool]]:
    """Files this one mounts with `#[path]`, and whether the mount is test-only.

    `#[path]` does not change the module tree, so the child is part of this
    module and not a file of its own. Counting it separately would let a
    module's ceiling be evaded by moving code across the boundary, which is
    exactly what extracting a `mod tests` does.

    The cfg can sit on the declaration rather than inside the child --
    `character_vendor_atomicity_tests.rs` has no inner attribute because its
    parent's `#[cfg(test)]` already supplies one -- so test-only-ness is read
    from the mount, not only from the file.
    """
    children = []
    is_code = rust_code_offsets(source)
    line_starts = []
    offset = 0
    for line in source.splitlines(keepends=True):
        line_starts.append(offset)
        offset += len(line)
    lines = source.splitlines()
    for index, line in enumerate(lines):
        match = PATH_MODULE_DECLARATION.search(line)
        if not match:
            continue
        # Accepted only if the declaration begins in code.
        if not is_code[line_starts[index] + match.start()]:
            continue
        child = (path.parent / path_module_target(match)).resolve()
        if not (child.is_file() and child.suffix == ".rs"):
            continue
        # Attributes preceding the #[path] on the same declaration.
        test_only = False
        for previous in reversed(lines[max(0, index - 4) : index]):
            stripped = previous.strip()
            if not stripped.startswith("#["):
                break
            if stripped.replace(" ", "") == "#[cfg(test)]":
                test_only = True
                break
        children.append((child, test_only))
    return children


def hotspot_row(path: pathlib.Path) -> tuple[int, int, int, str]:
    cached = HOTSPOT_ROW_CACHE.get(path)
    if cached is not None:
        return cached
    try:
        source = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ArchitectureError(f"cannot read Rust source {path}: {exc}") from exc
    total_lines, production_lines, test_lines = hotspot_line_counts(source)
    # A `#[path]` child counts against the module that mounts it. Without this
    # the ceiling applies to whatever stayed behind: moving 94,000 test lines
    # into a sibling would leave the parent capped at what remains and the
    # sibling capped by nothing, so the extraction would silently retire the
    # ratchet it was supposed to leave intact.
    for child, mounted_under_cfg_test in path_module_children(path, source):
        child_total, child_production, child_tests = hotspot_row(child)[:3]
        total_lines += child_total
        if mounted_under_cfg_test:
            # The mount supplies the cfg, so none of the child is production
            # however the file itself reads.
            test_lines += child_total
        else:
            production_lines += child_production
            test_lines += child_tests
    row = (
        total_lines,
        production_lines,
        test_lines,
        path.relative_to(REPO_ROOT).as_posix(),
    )
    HOTSPOT_ROW_CACHE[path] = row
    return row


def hotspot_rows(limit: int | None = 10) -> list[tuple[int, int, int, str]]:
    crates_root = REPO_ROOT / "crates"
    sources = sorted(crates_root.glob("*/src/**/*.rs"))
    mounted: set[pathlib.Path] = set()
    for path in sources:
        try:
            source = path.read_text(encoding="utf-8")
        except OSError as exc:
            raise ArchitectureError(f"cannot read Rust source {path}: {exc}") from exc
        mounted.update(child for child, _ in path_module_children(path, source))
    # Mounted children are already inside their parent's row; listing them again
    # would double-count them in the report.
    rows = [hotspot_row(path) for path in sources if path.resolve() not in mounted]
    rows.sort(key=lambda row: (-row[0], row[3]))
    return rows if limit is None else rows[:limit]


def validate_hotspot_non_growth(
    runtime: dict[str, Any],
    live_rows: list[tuple[int, int, int, str]] | None = None,
) -> int:
    """Reject growth in any audited hotspot production/test/total metric.

    The checked-in ledger values are independent upper bounds, not exact
    snapshots: a hotspot may shrink without editing the baseline. An audited
    path must remain present until its ledger row is deliberately retired, so
    renaming a file cannot silently evade its ceiling. Reporting remains
    broader; only paths explicitly curated in the runtime ledger are ratcheted
    here.
    """
    if live_rows is None:
        live_rows = hotspot_rows(limit=None)

    live_by_path: dict[str, tuple[int, int, int]] = {}
    for total, production, tests, path in live_rows:
        if (
            type(total) is not int
            or type(production) is not int
            or type(tests) is not int
            or not isinstance(path, str)
            or not path
            or min(total, production, tests) < 0
            or production + tests != total
        ):
            raise ArchitectureError(
                f"live hotspot metrics for {path!r} must be non-negative and add up"
            )
        if path in live_by_path:
            raise ArchitectureError(f"duplicate live hotspot metrics for {path}")
        live_by_path[path] = (production, tests, total)

    entries = runtime["inventories"]["hotspots"]["entries"]
    violations: list[str] = []
    for entry in entries:
        path = entry["path"]
        live = live_by_path.get(path)
        if live is None:
            violations.append(
                f"audited hotspot path {path} is missing; retire or replace "
                "its ledger row explicitly"
            )
            continue
        for index, key in enumerate(
            ("production_lines", "test_lines", "total_lines")
        ):
            baseline_value = entry[key]
            live_value = live[index]
            if live_value > baseline_value:
                violations.append(
                    f"{path} {key} grew from {baseline_value} to {live_value} "
                    f"(+{live_value - baseline_value})"
                )
    if violations:
        raise ArchitectureError(
            "runtime ownership hotspot LOC ratchet failed:\n- "
            + "\n- ".join(violations)
        )
    return len(entries)


def run_hotspot_ratchet_self_tests(runtime: dict[str, Any]) -> tuple[int, int]:
    """Prove independent growth rejection and below-baseline reduction acceptance."""
    baseline_rows = [
        (
            entry["total_lines"],
            entry["production_lines"],
            entry["test_lines"],
            entry["path"],
        )
        for entry in runtime["inventories"]["hotspots"]["entries"]
    ]
    validate_hotspot_non_growth(runtime, baseline_rows)

    total, production, tests, path = baseline_rows[0]
    growth_cases = [
        (
            "production-growth-with-stable-total",
            (total, production + 1, tests - 1, path),
            "production_lines grew",
        ),
        (
            "test-growth-with-stable-total",
            (total, production - 1, tests + 1, path),
            "test_lines grew",
        ),
        (
            "total-growth",
            (total + 1, production + 1, tests, path),
            "total_lines grew",
        ),
    ]
    for name, replacement, expected in growth_cases:
        candidate = [replacement, *baseline_rows[1:]]
        try:
            validate_hotspot_non_growth(runtime, candidate)
        except ArchitectureError as exc:
            if expected not in str(exc):
                raise ArchitectureError(
                    f"hotspot ratchet self-test {name} returned the wrong failure: {exc}"
                ) from exc
        else:
            raise ArchitectureError(
                f"hotspot ratchet self-test {name} was not rejected"
            )

    try:
        validate_hotspot_non_growth(runtime, baseline_rows[1:])
    except ArchitectureError as exc:
        if "audited hotspot path" not in str(exc) or "is missing" not in str(exc):
            raise ArchitectureError(
                f"hotspot ratchet self-test missing-path returned the wrong failure: {exc}"
            ) from exc
    else:
        raise ArchitectureError(
            "hotspot ratchet self-test missing-path was not rejected"
        )

    reduced = [
        (total - 2, production - 1, tests - 1, path),
        *baseline_rows[1:],
    ]
    validate_hotspot_non_growth(runtime, reduced)
    return len(growth_cases) + 1, 1


def print_hotspots(limit: int = 10) -> None:
    print(
        "Architecture hotspots (reporting; exact top-level #[cfg(test)] item extents):"
    )
    print(f"{'total':>8} {'prod':>8} {'tests':>8}  path")
    for total, production, tests, path in hotspot_rows(limit):
        print(f"{total:8d} {production:8d} {tests:8d}  {path}")


def synthetic_metadata(
    package_names: set[str], edges: set[tuple[str, str, str]]
) -> dict[str, Any]:
    package_ids = {
        package: f"path+file:///architecture-fixture/{package}#0.0.0"
        for package in package_names
    }
    external_names = {
        target for _, target, _ in edges if target not in package_names
    }
    external_ids = {
        package: f"{CRATES_IO_SOURCE}#{package}@9.9.9"
        for package in external_names
    }
    dependency_ids = package_ids | external_ids
    return {
        "workspace_members": [package_ids[package] for package in sorted(package_names)],
        "workspace_default_members": [
            package_ids[package] for package in sorted(package_names)
        ],
        "packages": [
            {
                "id": package_ids[package],
                "name": package,
                "version": "0.0.0",
                "source": None,
            }
            for package in sorted(package_names)
        ]
        + [
            {
                "id": external_ids[package],
                "name": package,
                "version": "9.9.9",
                "source": CRATES_IO_SOURCE,
            }
            for package in sorted(external_names)
        ],
        "resolve": {
            "nodes": [
                {
                    "id": package_ids[package],
                    "deps": [
                        {
                            "name": target.replace("-", "_"),
                            "pkg": dependency_ids[target],
                            "dep_kinds": [
                                {
                                    "kind": (
                                        None if kind == "normal" else kind
                                    ),
                                    "target": None,
                                }
                            ],
                        }
                        for source, target, kind in sorted(edges)
                        if source == package
                    ],
                }
                for package in sorted(package_names)
            ]
        },
    }


def fixture_policy(
    policy: dict[str, Any],
    source: str,
    target: str,
    scope: str,
    kind: str,
) -> dict[str, Any]:
    package_categories = policy["_package_categories"]
    categories: dict[str, list[str]] = {}
    scenario_packages = {source}
    if scope == "workspace":
        scenario_packages.add(target)
    for package in sorted(scenario_packages):
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
    restricted_packages = {}
    if source in policy["restricted_packages"]:
        restricted_packages[source] = [
            dependency
            for dependency in policy["restricted_packages"][source]
            if dependency in scenario_packages
        ]

    protected_categories = sorted(
        set(categories) & policy["_external_protected_categories"]
    )
    explicitly_protected_packages = sorted(
        scenario_packages
        & set(policy["external_dependencies"]["protected_packages"])
    )
    protected_packages = {
        package
        for package in scenario_packages
        if package_categories[package] in protected_categories
    } | set(explicitly_protected_packages)
    external_allowed = {
        package: {"normal": [], "build": []}
        for package in protected_packages
    }
    external_edge = (source, target, CRATES_IO_SOURCE, kind)
    if scope == "external" and external_edge in policy["_external_allowed_edges"]:
        external_allowed[source][kind].append(target)

    return validate_policy(
        {
            "schema_version": 2,
            "categories": categories,
            "allowed_category_dependencies": allowed_categories,
            "restricted_packages": restricted_packages,
            "exceptions": [],
            "external_dependencies": {
                "canonical_registry_source": CRATES_IO_SOURCE,
                "protected_categories": protected_categories,
                "protected_packages": explicitly_protected_packages,
                "allowed": external_allowed,
                "exceptions": [],
            },
        }
    )


def run_fixture_self_tests(policy: dict[str, Any]) -> None:
    expected_metadata_command = (
        "cargo",
        "metadata",
        "--locked",
        "--all-features",
        "--format-version",
        "1",
    )
    if CARGO_METADATA_COMMAND != expected_metadata_command:
        raise ArchitectureError(
            "cargo metadata command self-test failed; the architecture graph must "
            "remain locked and include all features"
        )

    try:
        parse_json(
            '{"normal": [], "normal": ["hidden-last-value"]}',
            "duplicate-key self-test",
        )
    except ArchitectureError as exc:
        if "duplicate JSON key 'normal'" not in str(exc):
            raise ArchitectureError(
                f"duplicate JSON key returned the wrong failure: {exc}"
            ) from exc
    else:
        raise ArchitectureError("duplicate JSON key fail-closed self-test failed")

    fixtures = sorted(FIXTURES_DIR.glob("*.json"))
    if not fixtures:
        raise ArchitectureError(f"no architecture fixtures found in {FIXTURES_DIR}")
    for fixture_path in fixtures:
        fixture = load_json(fixture_path)
        if not isinstance(fixture, dict):
            raise ArchitectureError(f"fixture {fixture_path.name} must be an object")
        required_keys = {"name", "source", "target", "expected"}
        optional_keys = {"scope", "kind", "target_cfg"}
        missing_keys = required_keys - set(fixture)
        unknown_keys = set(fixture) - required_keys - optional_keys
        if missing_keys or unknown_keys:
            raise ArchitectureError(
                f"fixture {fixture_path.name} has missing keys "
                f"{sorted(missing_keys)} and unknown keys {sorted(unknown_keys)}"
            )
        if not isinstance(fixture["name"], str) or not fixture["name"].strip():
            raise ArchitectureError(
                f"fixture {fixture_path.name} needs a non-empty name"
            )
        source = fixture.get("source")
        target = fixture.get("target")
        expected = fixture.get("expected")
        scope = fixture.get("scope", "workspace")
        kind = fixture.get("kind", "normal")
        target_cfg = fixture.get("target_cfg")
        if not all(
            isinstance(value, str)
            for value in (source, target, expected, scope, kind)
        ):
            raise ArchitectureError(
                f"fixture {fixture_path.name} needs string source, target, expected, "
                "scope, and kind"
            )
        if target_cfg is not None and not isinstance(target_cfg, str):
            raise ArchitectureError(
                f"fixture {fixture_path.name} target_cfg must be a string"
            )
        if source not in policy["_package_categories"]:
            raise ArchitectureError(
                f"fixture {fixture_path.name} references unclassified source {source}"
            )
        if scope not in {"workspace", "external"}:
            raise ArchitectureError(
                f"fixture {fixture_path.name} has unknown scope {scope!r}"
            )
        target_is_workspace = target in policy["_package_categories"]
        if scope == "workspace" and not target_is_workspace:
            raise ArchitectureError(
                f"fixture {fixture_path.name} references unclassified workspace "
                f"target {target}"
            )
        if kind not in PRODUCT_DEPENDENCY_KINDS:
            raise ArchitectureError(
                f"fixture {fixture_path.name} has unknown dependency kind {kind!r}"
            )
        if expected not in {"allowed", "forbidden"}:
            raise ArchitectureError(
                f"fixture {fixture_path.name} has unknown expected result {expected!r}"
            )
        scenario_policy = fixture_policy(policy, source, target, scope, kind)
        scenario_packages = {source}
        if scope == "workspace":
            scenario_packages.add(target)
        metadata = synthetic_metadata(
            scenario_packages, {(source, target, kind)}
        )
        if target_cfg is not None:
            metadata["resolve"]["nodes"][0]["deps"][0]["dep_kinds"][0][
                "target"
            ] = target_cfg
        try:
            check_dependencies(scenario_policy, metadata)
        except ArchitectureError as exc:
            if expected != "forbidden" or "forbidden" not in str(exc):
                raise ArchitectureError(
                    f"fixture {fixture_path.name}: expected {expected}, got error: {exc}"
                ) from exc
        else:
            if expected != "allowed":
                raise ArchitectureError(
                    f"fixture {fixture_path.name}: expected {expected}, got allowed"
                )

    _, dev_workspace_edges, dev_external_edges = production_edges(
        synthetic_metadata(
            {"fixture-domain"},
            {("fixture-domain", "fixture-test-helper", "dev")},
        )
    )
    if dev_workspace_edges or dev_external_edges:
        raise ArchitectureError("development dependency exclusion self-test failed")

    strict_kind_metadata = synthetic_metadata(
        {"fixture-domain"},
        {("fixture-domain", "fixture-runtime", "normal")},
    )
    strict_kind_entry = strict_kind_metadata["resolve"]["nodes"][0]["deps"][0]
    for label, dependency_kind, expected_error in [
        (
            "missing",
            {"target": None},
            "missing kind metadata",
        ),
        (
            "empty",
            {"kind": "", "target": None},
            "unsupported kind ''",
        ),
        (
            "integer",
            {"kind": 0, "target": None},
            "unsupported kind 0",
        ),
        (
            "boolean",
            {"kind": False, "target": None},
            "unsupported kind False",
        ),
        (
            "unknown",
            {"kind": "future-kind", "target": None},
            "unsupported kind 'future-kind'",
        ),
    ]:
        strict_kind_entry["dep_kinds"] = [dependency_kind]
        try:
            production_edges(strict_kind_metadata)
        except ArchitectureError as exc:
            if expected_error not in str(exc):
                raise ArchitectureError(
                    f"{label} dependency kind returned the wrong failure: {exc}"
                ) from exc
        else:
            raise ArchitectureError(
                f"{label} dependency kind fail-closed self-test failed"
            )

    external_origin_policy = fixture_policy(
        policy,
        "wow-map",
        "rand",
        "external",
        "normal",
    )

    def external_origin_metadata(
        package_id: str, package_source: str | None
    ) -> dict[str, Any]:
        metadata = synthetic_metadata(
            {"wow-map"},
            {("wow-map", "rand", "normal")},
        )
        dependency = metadata["resolve"]["nodes"][0]["deps"][0]
        old_package_id = dependency["pkg"]
        external_package = next(
            package
            for package in metadata["packages"]
            if package["id"] == old_package_id
        )
        external_package["id"] = package_id
        external_package["source"] = package_source
        dependency["pkg"] = package_id
        return metadata

    canonical_rand_id = f"{CRATES_IO_SOURCE}#rand@9.9.9"
    check_dependencies(
        external_origin_policy,
        external_origin_metadata(canonical_rand_id, CRATES_IO_SOURCE),
    )
    for package_id, package_source in [
        (
            "registry+https://example.invalid/rand#9.9.9",
            "registry+https://example.invalid/rand",
        ),
        (
            "git+https://example.invalid/dependency?rev=fixture#rand@9.9.9",
            "git+https://example.invalid/dependency?rev=fixture#0123456789",
        ),
        (
            "git+https://example.invalid/rand#9.9.9",
            "git+https://example.invalid/rand#0123456789",
        ),
    ]:
        production_edges(
            external_origin_metadata(package_id, package_source)
        )
    for label, package_id, package_source in [
        (
            "path",
            "path+file:///architecture-fixture/rand#9.9.9",
            None,
        ),
        (
            "git",
            "git+https://example.invalid/rand?rev=fixture#rand@9.9.9",
            "git+https://example.invalid/rand?rev=fixture#0123456789",
        ),
        (
            "alternate registry",
            "registry+https://example.invalid/index#rand@9.9.9",
            "registry+https://example.invalid/index",
        ),
    ]:
        try:
            check_dependencies(
                external_origin_policy,
                external_origin_metadata(package_id, package_source),
            )
        except ArchitectureError as exc:
            if "forbidden direct external normal dependency" not in str(exc):
                raise ArchitectureError(
                    f"{label} origin mutant returned the wrong failure: {exc}"
                ) from exc
        else:
            raise ArchitectureError(
                f"{label} origin mutant bypassed the external dependency policy"
            )

    for label, package_id, package_source, expected_error in [
        (
            "inconsistent canonical source",
            "path+file:///architecture-fixture/rand#rand@9.9.9",
            CRATES_IO_SOURCE,
            "is inconsistent with source metadata",
        ),
        (
            "null non-path source",
            "fixture-rand#rand@9.9.9",
            None,
            "has null source but is not a path package",
        ),
        (
            "mismatched package name",
            f"{CRATES_IO_SOURCE}#not-rand@9.9.9",
            CRATES_IO_SOURCE,
            "does not identify rand@9.9.9",
        ),
        (
            "mismatched package version",
            f"{CRATES_IO_SOURCE}#rand@1.2.3",
            CRATES_IO_SOURCE,
            "does not identify rand@9.9.9",
        ),
        (
            "registry id without package name",
            f"{CRATES_IO_SOURCE}#9.9.9",
            CRATES_IO_SOURCE,
            "does not identify rand@9.9.9",
        ),
        (
            "git suffix normalized unlike Cargo",
            "git+https://example.invalid/rand.git#9.9.9",
            "git+https://example.invalid/rand.git#0123456789",
            "does not identify rand@9.9.9",
        ),
    ]:
        try:
            production_edges(
                external_origin_metadata(package_id, package_source)
            )
        except ArchitectureError as exc:
            if expected_error not in str(exc):
                raise ArchitectureError(
                    f"{label} metadata mutant returned the wrong failure: {exc}"
                ) from exc
        else:
            raise ArchitectureError(
                f"{label} metadata mutant bypassed source validation"
            )

    ambiguous_metadata = external_origin_metadata(
        canonical_rand_id, CRATES_IO_SOURCE
    )
    second_rand_id = f"{CRATES_IO_SOURCE}#rand@8.5.0"
    ambiguous_metadata["packages"].append(
        {
            "id": second_rand_id,
            "name": "rand",
            "version": "8.5.0",
            "source": CRATES_IO_SOURCE,
        }
    )
    ambiguous_metadata["resolve"]["nodes"][0]["deps"].append(
        {
            "name": "rand_legacy",
            "pkg": second_rand_id,
            "dep_kinds": [{"kind": None, "target": None}],
        }
    )
    try:
        production_edges(ambiguous_metadata)
    except ArchitectureError as exc:
        if "ambiguous direct external dependency identity" not in str(exc):
            raise ArchitectureError(
                f"duplicate external identity returned the wrong failure: {exc}"
            ) from exc
    else:
        raise ArchitectureError(
            "duplicate external package ids bypassed identity validation"
        )

    duplicate_cases: list[tuple[str, dict[str, Any], str]] = []

    duplicate_package_metadata = synthetic_metadata(
        {"fixture-domain"}, set()
    )
    duplicate_package_metadata["packages"].append(
        dict(duplicate_package_metadata["packages"][0])
    )
    duplicate_cases.append(
        (
            "package id",
            duplicate_package_metadata,
            "duplicate package id",
        )
    )

    duplicate_node_metadata = synthetic_metadata(
        {"fixture-domain"},
        {("fixture-domain", "fixture-runtime", "normal")},
    )
    duplicate_node_metadata["resolve"]["nodes"].append(
        {
            "id": duplicate_node_metadata["resolve"]["nodes"][0]["id"],
            "deps": [],
        }
    )
    duplicate_cases.append(
        (
            "resolve node id",
            duplicate_node_metadata,
            "duplicate node id",
        )
    )

    duplicate_member_metadata = synthetic_metadata(
        {"fixture-domain"}, set()
    )
    duplicate_member_metadata["workspace_members"].append(
        duplicate_member_metadata["workspace_members"][0]
    )
    duplicate_cases.append(
        (
            "workspace member id",
            duplicate_member_metadata,
            "workspace_members has duplicate id",
        )
    )

    duplicate_default_member_metadata = synthetic_metadata(
        {"fixture-domain"}, set()
    )
    duplicate_default_member_metadata["workspace_default_members"].append(
        duplicate_default_member_metadata["workspace_default_members"][0]
    )
    duplicate_cases.append(
        (
            "workspace default member id",
            duplicate_default_member_metadata,
            "workspace_default_members has duplicate id",
        )
    )

    sourced_member_metadata = synthetic_metadata(
        {"fixture-domain"}, set()
    )
    old_member_id = sourced_member_metadata["workspace_members"][0]
    sourced_member_id = f"{CRATES_IO_SOURCE}#fixture-domain@0.0.0"
    sourced_member_package = sourced_member_metadata["packages"][0]
    sourced_member_package["id"] = sourced_member_id
    sourced_member_package["source"] = CRATES_IO_SOURCE
    sourced_member_metadata["workspace_members"][0] = sourced_member_id
    sourced_member_metadata["workspace_default_members"][0] = sourced_member_id
    sourced_member_node = sourced_member_metadata["resolve"]["nodes"][0]
    if sourced_member_node["id"] != old_member_id:
        raise ArchitectureError("sourced workspace-member self-test setup failed")
    sourced_member_node["id"] = sourced_member_id
    duplicate_cases.append(
        (
            "sourced workspace member",
            sourced_member_metadata,
            "must be a source-null path package",
        )
    )

    for label, metadata, expected_error in duplicate_cases:
        try:
            production_edges(metadata)
        except ArchitectureError as exc:
            if expected_error not in str(exc):
                raise ArchitectureError(
                    f"duplicate {label} returned the wrong failure: {exc}"
                ) from exc
        else:
            raise ArchitectureError(
                f"duplicate {label} bypassed metadata validation"
            )

    collision_policy = validate_policy(
        {
            "schema_version": 2,
            "categories": {"domain-runtime": ["wow-map"]},
            "allowed_category_dependencies": {
                "domain-runtime": ["domain-runtime"]
            },
            "restricted_packages": {},
            "exceptions": [],
            "external_dependencies": {
                "canonical_registry_source": CRATES_IO_SOURCE,
                "protected_categories": ["domain-runtime"],
                "protected_packages": [],
                "allowed": {
                    "wow-map": {"normal": ["wow-math"], "build": []}
                },
                "exceptions": [],
            },
        }
    )
    collision_metadata = synthetic_metadata(
        {"wow-map", "wow-math"},
        set(),
    )
    external_wow_math_id = f"{CRATES_IO_SOURCE}#wow-math@9.9.9"
    collision_metadata["packages"].append(
        {
            "id": external_wow_math_id,
            "name": "wow-math",
            "version": "9.9.9",
            "source": CRATES_IO_SOURCE,
        }
    )
    wow_map_id = next(
        package_id
        for package_id in collision_metadata["workspace_members"]
        if package_id.endswith("/wow-map#0.0.0")
    )
    wow_map_node = next(
        node
        for node in collision_metadata["resolve"]["nodes"]
        if node["id"] == wow_map_id
    )
    wow_map_node["deps"].append(
        {
            "name": "external_wow_math",
            "pkg": external_wow_math_id,
            "dep_kinds": [{"kind": None, "target": None}],
        }
    )
    try:
        check_dependencies(collision_policy, collision_metadata)
    except ArchitectureError as exc:
        if "external dependency policy references workspace packages: wow-math" not in str(
            exc
        ):
            raise ArchitectureError(
                f"workspace/external name collision returned the wrong failure: {exc}"
            ) from exc
    else:
        raise ArchitectureError(
            "workspace/external package name collision bypassed policy validation"
        )

    narrowed_default_members = synthetic_metadata(
        {"fixture-domain-a", "fixture-domain-b"}, set()
    )
    narrowed_default_members["workspace_default_members"] = [
        narrowed_default_members["workspace_members"][0]
    ]
    try:
        production_edges(narrowed_default_members)
    except ArchitectureError as exc:
        if "workspace_default_members differs from workspace_members" not in str(exc):
            raise ArchitectureError(
                f"default-members coverage returned the wrong failure: {exc}"
            ) from exc
    else:
        raise ArchitectureError(
            "narrowed workspace default-members fail-closed self-test failed"
        )

    exception_edge = ("wow-network", "wow-data")
    if classify_edge(policy, *exception_edge)[0] != "exception":
        raise ArchitectureError("baseline exception classification self-test failed")

    external_exception_edge = (
        "wow-loot",
        "tokio",
        CRATES_IO_SOURCE,
        "normal",
    )
    if (
        classify_external_dependency(policy, *external_exception_edge)[0]
        != "exception"
    ):
        raise ArchitectureError(
            "baseline external exception classification self-test failed"
        )

    stale_policy = validate_policy(
        {
            "schema_version": 2,
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
            "external_dependencies": {
                "canonical_registry_source": CRATES_IO_SOURCE,
                "protected_categories": ["adapter-platform"],
                "protected_packages": [],
                "allowed": {
                    "fixture-data": {"normal": [], "build": []},
                    "fixture-network": {"normal": [], "build": []},
                },
                "exceptions": [],
            },
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

    stale_restricted_policy = validate_policy(
        {
            "schema_version": 2,
            "categories": {
                "adapter-platform": ["fixture-network", "fixture-core"]
            },
            "allowed_category_dependencies": {
                "adapter-platform": ["adapter-platform"]
            },
            "restricted_packages": {
                "fixture-network": ["fixture-core"]
            },
            "exceptions": [],
            "external_dependencies": {
                "canonical_registry_source": CRATES_IO_SOURCE,
                "protected_categories": ["adapter-platform"],
                "protected_packages": [],
                "allowed": {
                    "fixture-core": {"normal": [], "build": []},
                    "fixture-network": {"normal": [], "build": []},
                },
                "exceptions": [],
            },
        }
    )
    active_restricted_metadata = synthetic_metadata(
        {"fixture-network", "fixture-core"},
        {("fixture-network", "fixture-core", "normal")},
    )
    check_dependencies(stale_restricted_policy, active_restricted_metadata)
    try:
        check_dependencies(
            stale_restricted_policy,
            synthetic_metadata({"fixture-network", "fixture-core"}, set()),
        )
    except ArchitectureError as exc:
        expected_error = (
            "obsolete restricted-package allowance "
            "fixture-network -> fixture-core"
        )
        if expected_error not in str(exc):
            raise ArchitectureError(
                f"restricted-package allowance ratchet returned the wrong failure: {exc}"
            ) from exc
    else:
        raise ArchitectureError(
            "restricted-package allowance ratchet self-test failed"
        )

    stale_external_policy = validate_policy(
        {
            "schema_version": 2,
            "categories": {"domain-runtime": ["fixture-domain"]},
            "allowed_category_dependencies": {
                "domain-runtime": ["domain-runtime"]
            },
            "restricted_packages": {},
            "exceptions": [],
            "external_dependencies": {
                "canonical_registry_source": CRATES_IO_SOURCE,
                "protected_categories": ["domain-runtime"],
                "protected_packages": [],
                "allowed": {
                    "fixture-domain": {"normal": ["rand"], "build": []}
                },
                "exceptions": [
                    {
                        "from": "fixture-domain",
                        "to": "tokio",
                        "kind": "normal",
                        "tracking_issue": 135,
                        "reason": "Synthetic exception used to test the removal ratchet.",
                    }
                ],
            },
        }
    )
    try:
        check_dependencies(
            stale_external_policy,
            synthetic_metadata({"fixture-domain"}, set()),
        )
    except ArchitectureError as exc:
        expected_errors = (
            "obsolete allowed external normal dependency fixture-domain -> rand",
            "obsolete baseline external exception normal fixture-domain -> tokio",
        )
        if not all(expected in str(exc) for expected in expected_errors):
            raise ArchitectureError(
                f"external dependency ratchets returned the wrong failure: {exc}"
            ) from exc
    else:
        raise ArchitectureError("external dependency ratchet self-test failed")


def run_debt_ownership_fixture_tests() -> int:
    """Reject malformed/duplicate debt ownership and ledger violations."""
    fixtures = sorted(DEBT_OWNERSHIP_FIXTURES_DIR.glob("*.json"))
    if not fixtures:
        raise ArchitectureError(
            f"no debt-ownership fixtures found in {DEBT_OWNERSHIP_FIXTURES_DIR}"
        )
    for fixture_path in fixtures:
        fixture = load_json(fixture_path)
        if not isinstance(fixture, dict):
            raise ArchitectureError(
                f"debt-ownership fixture {fixture_path.name} must be an object"
            )
        required_keys = {"name", "expect", "error_substring"}
        optional_keys = {"policy", "ledger"}
        missing_keys = required_keys - set(fixture)
        unknown_keys = set(fixture) - required_keys - optional_keys
        if missing_keys or unknown_keys:
            raise ArchitectureError(
                f"debt-ownership fixture {fixture_path.name} has missing keys "
                f"{sorted(missing_keys)} and unknown keys {sorted(unknown_keys)}"
            )
        if not isinstance(fixture["name"], str) or not fixture["name"].strip():
            raise ArchitectureError(
                f"debt-ownership fixture {fixture_path.name} needs a non-empty name"
            )
        if fixture["expect"] != "reject":
            raise ArchitectureError(
                f"debt-ownership fixture {fixture_path.name} must expect reject"
            )
        if not isinstance(fixture["error_substring"], str) or not fixture[
            "error_substring"
        ].strip():
            raise ArchitectureError(
                f"debt-ownership fixture {fixture_path.name} needs a non-empty "
                "error_substring"
            )
        if "policy" not in fixture and "ledger" not in fixture:
            raise ArchitectureError(
                f"debt-ownership fixture {fixture_path.name} needs a policy or "
                "ledger payload"
            )
        try:
            policy = fixture.get("policy")
            if policy is not None:
                policy = validate_policy(policy)
            ledger = fixture.get("ledger")
            if ledger is not None:
                ledger = validate_issue_ledger(ledger)
            if policy is not None and ledger is not None:
                validate_debt_ownership(policy, ledger)
        except ArchitectureError as exc:
            if fixture["error_substring"] not in str(exc):
                raise ArchitectureError(
                    f"debt-ownership fixture {fixture_path.name} returned the "
                    f"wrong failure: {exc}"
                ) from exc
        else:
            raise ArchitectureError(
                f"debt-ownership fixture {fixture_path.name} was not rejected"
            )
    return len(fixtures)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--policy",
        type=pathlib.Path,
        default=DEFAULT_POLICY,
        help="dependency policy JSON (default: repository policy)",
    )
    parser.add_argument(
        "--ledger",
        type=pathlib.Path,
        default=DEFAULT_ISSUE_LEDGER,
        help="architecture issue ledger JSON (default: repository ledger)",
    )
    parser.add_argument(
        "--runtime-ledger",
        type=pathlib.Path,
        default=DEFAULT_RUNTIME_OWNERSHIP_LEDGER,
        help="runtime ownership ledger JSON (default: repository ledger)",
    )
    parser.add_argument(
        "--session-ownership-policy",
        type=pathlib.Path,
        default=DEFAULT_SESSION_OWNERSHIP_POLICY,
        help="session ownership syntax policy JSON (default: repository policy)",
    )
    parser.add_argument(
        "--handler-module-policy",
        type=pathlib.Path,
        default=DEFAULT_HANDLER_MODULE_POLICY,
        help="handler logical-module ownership policy JSON (default: repository policy)",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser(
        "check", help="check architecture ratchets and report source hotspots"
    )
    subparsers.add_parser("self-test", help="validate policy and focused fixtures")
    hotspots_parser = subparsers.add_parser(
        "hotspots", help="report source hotspots without enforcing a line limit"
    )
    hotspots_parser.add_argument("--limit", type=int, default=10)
    args = parser.parse_args()

    try:
        policy = validate_policy(load_json(args.policy))
        if args.command in {"check", "self-test"}:
            ledger = validate_issue_ledger(load_json(args.ledger))
            handler_module_policy = validate_handler_module_policy(
                load_json(args.handler_module_policy), ledger
            )
            validate_debt_ownership(policy, ledger)
            runtime_ledger = validate_runtime_ownership_ledger(
                load_json(args.runtime_ledger), ledger
            )
            audited_hotspots = validate_hotspot_non_growth(runtime_ledger)
            session_ownership_policy = load_json(args.session_ownership_policy)
            validate_runtime_syntax_coverage(
                runtime_ledger, session_ownership_policy
            )
            validate_documented_sequence(ledger)
        if args.command == "self-test":
            run_fixture_self_tests(policy)
            handler_module_policy_rejections = run_handler_module_policy_self_tests(
                handler_module_policy, ledger
            )
            debt_ownership_fixtures = run_debt_ownership_fixture_tests()
            runtime_ownership_rejections = run_runtime_ownership_self_tests(
                runtime_ledger, ledger
            )
            run_path_module_scanner_self_tests()
            hotspot_classifier_fixtures = run_hotspot_classifier_self_tests()
            hotspot_ratchet_rejections, hotspot_reduction_acceptances = (
                run_hotspot_ratchet_self_tests(runtime_ledger)
            )
            print(
                "Architecture self-test: PASS "
                f"({len(list(FIXTURES_DIR.glob('*.json')))} fixtures, "
                f"{debt_ownership_fixtures} debt-ownership rejections, "
                f"{runtime_ownership_rejections} runtime-ownership rejections, "
                f"{hotspot_classifier_fixtures} hotspot-classifier fixture, "
                f"{hotspot_ratchet_rejections} hotspot-ratchet rejections, "
                f"{hotspot_reduction_acceptances} hotspot-reduction acceptance, "
                f"{handler_module_policy_rejections} handler-module-policy rejections)"
            )
        elif args.command == "hotspots":
            if args.limit <= 0:
                raise ArchitectureError("--limit must be positive")
            print_hotspots(args.limit)
        else:
            (
                packages,
                workspace_edges,
                workspace_exceptions,
                guarded_external_edges,
                external_exceptions,
            ) = check_dependencies(policy, cargo_metadata())
            print(
                "Architecture dependencies: "
                f"PASS ({packages} packages, {workspace_edges} workspace edges, "
                f"{guarded_external_edges} guarded external dependencies, "
                f"{workspace_exceptions} workspace baseline exceptions, "
                f"{external_exceptions} external baseline exceptions)"
            )
            syntax = session_ownership_policy["syntax_baseline"]
            print(
                "Architecture ownership: PASS "
                f"({len(syntax['world_session']['fields'])} WorldSession fields, "
                f"{len(syntax['session_resources']['fields'])} SessionResources fields, "
                f"{len(syntax['player_broadcast_info']['fields'])} broadcast fields, "
                f"{len(syntax['session_command']['variants'])} command variants, "
                f"{len(runtime_ledger['world_session_responsibility_families']['families'])} "
                "semantic responsibility families)"
            )
            print(
                "Architecture hotspot ratchet: PASS "
                f"({audited_hotspots} audited files; production/test/total "
                "metrics are at or below baseline)"
            )
            print_hotspots()
    except ArchitectureError as exc:
        print(f"architecture check failed: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
