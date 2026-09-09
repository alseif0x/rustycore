"""Dependencies checks.

Separated from check_architecture.py under #664; behaviour is preserved.
"""

from __future__ import annotations

from check_shared import *  # noqa: F401,F403
from check_policy import *  # noqa: F401,F403

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
    # Cargo PackageIdSpec::fmt omits the name only when the URL's final
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

    reserved_packages = policy["_reserved_packages"]
    for name in sorted(reserved_packages):
        if name not in workspace_packages:
            errors.append(f"reserved package {name} is no longer a workspace member")
    for source, target, kind in sorted(workspace_edges):
        if target in reserved_packages:
            errors.append(
                f"reserved package {target} has acquired a dependent: {kind} edge "
                f"{source} -> {target}; it is a real boundary now, so give it an owner"
            )
        if source in reserved_packages:
            errors.append(
                f"reserved package {source} has acquired a dependency: {kind} edge "
                f"{source} -> {target}; an empty name must not grow edges"
            )

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
