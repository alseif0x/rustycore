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
ARCHITECTURE_DOC = REPO_ROOT / "docs" / "architecture" / "ownership-and-boundaries.md"
FIXTURES_DIR = ARCHITECTURE_DIR / "fixtures"
DEBT_OWNERSHIP_FIXTURES_DIR = FIXTURES_DIR / "debt-ownership"
LEDGER_ISSUE_STATES = {"open", "closed"}
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
    if not isinstance(ledger, dict) or ledger.get("schema_version") != 1:
        raise ArchitectureError(
            "architecture issue ledger must be a schema_version 1 object"
        )
    parent_issue = ledger.get("parent_issue")
    reaudit_issue = ledger.get("reaudit_issue")
    issues = ledger.get("issues")
    sequence = ledger.get("sequence")
    if type(parent_issue) is not int or parent_issue <= 0:
        raise ArchitectureError("issue ledger needs a positive parent_issue")
    if type(reaudit_issue) is not int or reaudit_issue <= 0:
        raise ArchitectureError("issue ledger needs a positive reaudit_issue")
    if not isinstance(issues, list) or not issues:
        raise ArchitectureError("issue ledger issues must be a non-empty array")

    entries: dict[int, dict[str, Any]] = {}
    for index, entry in enumerate(issues):
        if not isinstance(entry, dict):
            raise ArchitectureError(f"issue ledger entry {index} must be an object")
        number = entry.get("number")
        state = entry.get("state")
        title = entry.get("title")
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
        if number in entries:
            raise ArchitectureError(f"duplicate issue ledger entry: #{number}")
        entries[number] = entry

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
    unsequenced = sorted(set(entries) - set(sequence) - {parent_issue})
    if unsequenced:
        raise ArchitectureError(
            f"issue ledger issues missing from the sequence: {unsequenced}"
        )

    ledger["_entries"] = entries
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
        if args.command in {"check", "self-test"}:
            ledger = validate_issue_ledger(load_json(args.ledger))
            validate_debt_ownership(policy, ledger)
            validate_documented_sequence(ledger)
        if args.command == "self-test":
            run_fixture_self_tests(policy)
            debt_ownership_fixtures = run_debt_ownership_fixture_tests()
            print(
                "Architecture self-test: PASS "
                f"({len(list(FIXTURES_DIR.glob('*.json')))} fixtures, "
                f"{debt_ownership_fixtures} debt-ownership rejections)"
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
            print_hotspots()
    except ArchitectureError as exc:
        print(f"architecture check failed: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
