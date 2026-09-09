"""Policy checks.

Separated from check_architecture.py under #664; behaviour is preserved.
"""

from __future__ import annotations

from check_shared import *  # noqa: F401,F403

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

    reserved = policy.get("reserved_packages", [])
    if not isinstance(reserved, list):
        raise ArchitectureError("reserved_packages must be an array")
    seen_reserved: set[str] = set()
    for entry in reserved:
        if not isinstance(entry, dict) or set(entry) != {
            "package",
            "issue",
            "state",
            "reason",
        }:
            raise ArchitectureError(
                "each reserved_packages entry needs exactly package, issue, state and reason"
            )
        name = entry["package"]
        if name not in package_categories:
            raise ArchitectureError(f"reserved package {name} is not a classified package")
        if name in seen_reserved:
            raise ArchitectureError(f"duplicate reserved package {name}")
        seen_reserved.add(name)
        if type(entry["issue"]) is not int or entry["issue"] <= 0:
            raise ArchitectureError(f"reserved package {name} needs a positive owning issue")
        if entry["state"] not in {"open", "closed"}:
            raise ArchitectureError(f"reserved package {name} has an invalid mirrored state")
        if not isinstance(entry["reason"], str) or not entry["reason"].strip():
            raise ArchitectureError(f"reserved package {name} needs a reason")
    policy["_reserved_packages"] = {entry["package"]: entry for entry in reserved}
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
    for entry in policy.get("reserved_packages", []):
        if entry["state"] != "open":
            problems.append(
                f"reserved package {entry['package']} is held for completed issue "
                f"#{entry['issue']}; fill the crate or remove the reservation"
            )
    if problems:
        raise ArchitectureError("\n".join(problems))


def _repository_defines_test(repository_root: Path, name: str) -> bool:
    needle = f"fn {name}("
    for path in (repository_root / "crates").rglob("*.rs"):
        try:
            if needle in path.read_text(encoding="utf-8"):
                return True
        except OSError:
            continue
    return False


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
