"""Runtime checks.

Separated from check_architecture.py under #664; behaviour is preserved.
"""

from __future__ import annotations

from check_shared import *  # noqa: F401,F403
from check_policy import *  # noqa: F401,F403
from check_policy import _repository_defines_test

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
        if inventory_name == "session_resources":
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
                logical_scope = entry.get("logical_scope", "module")
                if logical_scope not in {"module", "crate"}:
                    raise ArchitectureError(
                        f"runtime ownership hotspot {entry_id} logical_scope must be "
                        "module or crate"
                    )
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


def validate_runtime_clock_phase_trace(repository_root: Path) -> int:
    """Check the #188 trace's declarations and existing source/test anchors.

    Identifier-shaped entries must name a function in their source; descriptive
    entries are not resolved and empty regression-anchor lists are accepted.
    Cadence, diff and resolution guards are declarations, not analyzed behavior.
    Reject a declared map-guard delivery, but do not claim to prove spawn
    reachability, phase order, lock safety or single resolution from metadata.
    Those require affected source review and production integration evidence.
    """
    trace_path = repository_root / "tools/architecture/runtime-clock-phase-trace.json"
    trace = load_json(trace_path)
    if trace.get("schema_version") != 1:
        raise ArchitectureError("runtime clock/phase trace has an unsupported schema version")
    clocks = trace.get("clocks")
    if not isinstance(clocks, list) or not clocks:
        raise ArchitectureError("runtime clock/phase trace records no clocks")

    required = (
        "id", "source", "entry", "cpp_anchor", "availability", "cadence",
        "diff_source", "owns", "delivers_under_map_guard", "resolution_guard",
        "regression_anchors",
    )
    seen: set[str] = set()
    for clock in clocks:
        missing = [field for field in required if field not in clock]
        if missing:
            raise ArchitectureError(
                f"runtime clock {clock.get('id', '<unnamed>')} is missing {missing}"
            )
        identifier = clock["id"]
        if identifier in seen:
            raise ArchitectureError(f"runtime clock {identifier} is recorded twice")
        seen.add(identifier)
        if clock["availability"] not in {"production", "diagnostic"}:
            raise ArchitectureError(
                f"runtime clock {identifier} must be production or diagnostic"
            )
        if clock["delivers_under_map_guard"] is not False:
            raise ArchitectureError(
                f"runtime clock {identifier} claims delivery under a map guard, "
                "which the campaign invariants forbid"
            )
        source = repository_root / clock["source"]
        if not source.is_file():
            raise ArchitectureError(
                f"runtime clock {identifier} names missing source {clock['source']}"
            )
        body = source.read_text(encoding="utf-8")
        entry = clock["entry"]
        if entry.isidentifier() and f"fn {entry}" not in body:
            raise ArchitectureError(
                f"runtime clock {identifier} names entry point {entry} "
                f"that {clock['source']} does not define"
            )
        for field in ("cadence", "diff_source", "resolution_guard"):
            if not str(clock[field]).strip():
                raise ArchitectureError(f"runtime clock {identifier} has an empty {field}")
        for anchor in clock["regression_anchors"]:
            if not _repository_defines_test(repository_root, anchor):
                raise ArchitectureError(
                    f"runtime clock {identifier} names regression anchor {anchor} "
                    "that no test defines"
                )

    owner = trace.get("tick_owner", {})
    for field in ("constructed_default", "production_default",
                  "production_default_source", "production_default_mechanism"):
        if not str(owner.get(field, "")).strip():
            raise ArchitectureError(f"runtime clock/phase trace tick_owner lacks {field}")
    return len(clocks)


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
