#!/usr/bin/env python3
"""Executable architecture guardrails for the RustyCore workspace."""


from check_shared import *  # noqa: F401,F403
from check_policy import *  # noqa: F401,F403
from check_runtime import *  # noqa: F401,F403
from check_dependencies import *  # noqa: F401,F403
from check_fixtures import *  # noqa: F401,F403
from check_issue_state import *  # noqa: F401,F403

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--physical-policy", type=pathlib.Path, default=DEFAULT_PHYSICAL_POLICY,
        help="reviewed physical source ceilings and bounded exceptions",
    )
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
    subparsers.add_parser(
        "hotspot-ratchet",
        help="enforce only the curated hotspot LOC ceilings",
    )
    refresh_parser = subparsers.add_parser(
        "refresh-issue-state",
        help="derive the mirrored issue state/title fields from GitHub (networked)",
    )
    refresh_parser.add_argument(
        "--check",
        action="store_true",
        help="report drift and fail instead of rewriting the ledgers",
    )
    hotspots_parser = subparsers.add_parser(
        "hotspots", help="report source hotspots without enforcing a line limit"
    )
    hotspots_parser.add_argument("--limit", type=int, default=10)
    physical_parser = subparsers.add_parser(
        "physical-files", help="check all repository source/test/tool files without Cargo scans",
    )
    physical_parser.add_argument("--terminal", action="store_true", help="reject unfinished oversized migrations")
    physical_parser.add_argument("--json", action="store_true", help="emit the complete measured inventory")
    args = parser.parse_args()

    try:
        if args.command == "refresh-issue-state":
            return refresh_issue_state(args.ledger, args.runtime_ledger, not args.check)
        if args.command in {"physical-files", "check", "self-test"}:
            physical_policy = load_json(args.physical_policy)
            physical_result = physical_files.evaluate(
                REPO_ROOT, physical_policy, physical_files.inventory(REPO_ROOT),
                terminal=getattr(args, "terminal", False),
            )
            physical_issues = {entry["number"] for entry in load_json(args.ledger)["issues"]}
            for issue in [physical_policy["migration_issue"], *[
                entry["tracking_issue"] for entry in physical_policy["exceptions"]
            ]]:
                if issue not in physical_issues:
                    raise ArchitectureError(f"physical policy references unknown issue #{issue}")
            if args.command == "physical-files" and args.json:
                print(json.dumps(physical_result, indent=2))
            else:
                print(f"Physical source ratchet: PASS ({len(physical_result['files'])} files; "
                      f"{physical_result['legacy_files']} legacy ceilings; "
                      f"{physical_result['generated_files']} verified-provenance generated files; "
                      f"{physical_result['review_required']} cohesion reviews; {physical_result['mode']})")
            if args.command == "physical-files":
                return 0
        policy = validate_policy(load_json(args.policy))
        if args.command == "hotspot-ratchet":
            # The one ceiling an ordinary Rust diff can move. Kept separate from
            # `check` so the pre-merge gate does not pull the whole scanner into
            # the normal path.
            ledger = validate_issue_ledger(load_json(args.ledger))
            runtime_ledger = validate_runtime_ownership_ledger(
                load_json(args.runtime_ledger), ledger
            )
            audited_hotspots = validate_hotspot_non_growth(runtime_ledger)
            print(f"Hotspot LOC ratchet: PASS ({audited_hotspots} audited paths)")
            return 0
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
            traced_clocks = validate_runtime_clock_phase_trace(REPO_ROOT)
            validate_documented_sequence(ledger)
        if args.command == "self-test":
            from test_physical_files import run_self_tests
            if not run_self_tests():
                raise ArchitectureError("physical source self-tests failed")
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
            hotspot_view_assertions = run_hotspot_view_self_tests(runtime_ledger)
            hotspot_ratchet_rejections, hotspot_reduction_acceptances = (
                run_hotspot_ratchet_self_tests(runtime_ledger)
            )
            print(
                "Architecture self-test: PASS "
                f"({len(list(FIXTURES_DIR.glob('*.json')))} fixtures, "
                f"{debt_ownership_fixtures} debt-ownership rejections, "
                f"{runtime_ownership_rejections} runtime-ownership rejections, "
                f"{hotspot_classifier_fixtures} hotspot-classifier fixture, "
                f"{hotspot_view_assertions} physical/logical view assertions, "
                f"{hotspot_ratchet_rejections} hotspot-ratchet rejections, "
                f"{hotspot_reduction_acceptances} hotspot-reduction acceptance, "
                f"{handler_module_policy_rejections} handler-module-policy rejections)"
            )
        elif args.command == "hotspots":
            if args.limit <= 0:
                raise ArchitectureError("--limit must be positive")
            runtime_ledger = load_json(args.runtime_ledger)
            print_hotspots(runtime_ledger, args.limit)
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
                f"({traced_clocks} traced runtime clocks, "
                f"{len(syntax['world_session']['fields'])} WorldSession fields, "
                f"{len(syntax['session_resources']['fields'])} SessionResources fields, "
                f"{len(syntax['session_command']['variants'])} command variants, "
                f"{len(runtime_ledger['world_session_responsibility_families']['families'])} "
                "semantic responsibility families)"
            )
            print(
                "Architecture hotspot ratchet: PASS "
                f"({audited_hotspots} audited logical owners; production/test/total "
                "metrics are at or below baseline)"
            )
            print_hotspots(runtime_ledger)
    except (ArchitectureError, physical_files.PhysicalFileError, OSError, subprocess.CalledProcessError) as exc:
        print(f"architecture check failed: {exc}", file=sys.stderr)
        return 1
    return 0



if __name__ == "__main__":
    raise SystemExit(main())
