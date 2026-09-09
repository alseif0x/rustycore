"""Fixtures checks.

Separated from check_architecture.py under #664; behaviour is preserved.
"""

from __future__ import annotations

from check_shared import *  # noqa: F401,F403
from check_policy import *  # noqa: F401,F403
from check_dependencies import *  # noqa: F401,F403
from check_runtime import *  # noqa: F401,F403

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

    # #140 retired the wow-network exceptions this fixture used to pin. Any live
    # issue-linked debt edge exercises the same classification path.
    exception_edge = ("wow-instances", "wow-data")
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
