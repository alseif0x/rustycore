#!/usr/bin/env python3
"""Run all declared cells, preserve failures, require a frozen third-module challenge for timing."""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import gzip
import hashlib
import itertools
import json
import os
from pathlib import Path
import platform
import re
import signal
import subprocess
import sys
import time

import freeze
import report
import build as builder

ROOT = freeze.ROOT
REPO = ROOT.parents[2]
WASM_GATES = {
    "wasm::tests::validation::hidden_second_memory_is_rejected_before_instantiation",
    "wasm::tests::validation::initial_codec_rejection_does_not_activate_or_advance_authority",
    "wasm::tests::validation::real_rust_and_c_codecs_reject_malformed_replay_before_mutation",
    "wasm::tests::validation::real_rust_and_c_live_writes_validate_shape_without_consuming_revisions",
    "wasm::tests::validation::semantic_imports_are_forbidden_inside_codec_even_when_error_is_ignored",
    "wasm::tests::validation::validation_reads_have_scoped_separate_budget_and_sticky_failures",
    "wasm::tests::validation::codec_traps_clear_the_phase_and_preserve_previous_state",
    "wasm::tests::validation::codec_uses_remaining_root_fuel_instead_of_refilling",
    "wasm::tests::validation::live_write_preflight_rejects_stale_or_oversize_before_codec",
    "wasm::tests::validation::replay_validates_all_codec_candidates_before_any_installation",
    "wasm::tests::registration::wasm_runtime_native_returns_share_portable_fault_semantics_and_leave_trace",
    "wasm::tests::resources::real_rust_and_c_guests_cannot_grow_past_their_memory_cap_or_spin_forever",
    "wasm::tests::resources::cumulative_guest_fuel_fails_after_effect_and_the_same_probe_finishes_with_injected_refill",
    "wasm::tests::resources::real_guest_recursive_dispatch_hits_host_depth_and_keeps_prior_nested_writes",
    "wasm::tests::imports::hostile_pointer_length_revision_and_operation_are_rejected_before_state_changes",
    "wasm::tests::imports::malformed_imports_cannot_bypass_a_depleted_host_call_budget",
    "wasm::tests::imports::missing_capability_and_detached_actions_do_not_mutate_authority",
    "wasm::tests::imports::hostile_unreachable_after_shield_is_a_trap_without_rollback",
    "wasm::tests::registration::duplicate_executor_rejection_preserves_native_or_wasm_authority",
    "wasm::tests::registration::hostile_metadata_mismatches_never_activate_or_add_entity_state",
    "wasm::tests::registration::real_frontends_admit_matching_metadata_and_reject_duplicate_or_mismatched_identity",
    "wasm::tests::registration::bad_manifest_and_oversize_binary_fail_before_guest_compilation",
    "wasm::tests::registration::start_functions_have_no_invocation_authority_over_existing_entities",
    "wasm::tests::registration::instance_count_and_total_linear_memory_are_bounded_in_one_store",
}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def command(argv, timeout, *, parse_json=True):
    started = time.monotonic()
    record = {"argv": [str(part) for part in argv], "started_utc": datetime.now(timezone.utc).isoformat()}
    try:
        process = subprocess.Popen(record["argv"], cwd=REPO, stdout=subprocess.PIPE,
                                   stderr=subprocess.PIPE, text=True, errors="replace",
                                   start_new_session=True,
                                   env=dict(os.environ, CARGO_BUILD_JOBS="2", CARGO_TARGET_DIR=str(ROOT / "target")))
        try:
            stdout, stderr = process.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            # Kill only this command's session, including cargo/rustc/test descendants.
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            stdout, stderr = process.communicate(timeout=10)
            record.update(error="timeout", stdout=stdout, stderr=stderr, returncode=None)
            return record
        record.update(returncode=process.returncode, stderr=stderr)
        if parse_json:
            try:
                record["value"] = json.loads(stdout)
            except (json.JSONDecodeError, ValueError) as exc:
                record.update(error=f"malformed JSON: {exc}", stdout=stdout)
        else:
            record["stdout"] = stdout
        if process.returncode:
            record["stdout"] = stdout
            record["error"] = f"command exited {process.returncode}"
    except OSError as exc:
        record.update(error=str(exc), stdout="", stderr="", returncode=None)
    finally:
        record["elapsed_seconds"] = time.monotonic() - started
    return record


def verify_build(path):
    value = freeze.read_record(path)
    if (value.get("schema_version") != 2 or value.get("kind") != "source-built-conformance-artifacts"
            or value.get("source_files") != freeze.validate_cargo(ROOT)):
        raise ValueError("build record does not describe the exact current source tree")
    verify_artifacts(value)
    return value


def verify_artifacts(value):
    artifacts = value.get("artifacts")
    expected = builder.expected_artifacts(ROOT)
    if not isinstance(artifacts, dict) or set(artifacts) != set(expected):
        raise ValueError("not every module has both actual Rust/C artifacts")
    for name, path in expected.items():
        artifact = artifacts[name]
        if (not isinstance(artifact, dict) or not isinstance(artifact.get("path"), str)
                or Path(artifact["path"]).resolve() != path.resolve()
                or path.is_symlink() or not path.is_file()
                or not isinstance(artifact.get("sha256"), str)
                or not re.fullmatch("[0-9a-f]{64}", artifact["sha256"])
                or sha(path) != artifact["sha256"]):
            raise ValueError(f"artifact path/content differs from the actual loader input: {name}")


def test_gate(record, expected, *, wasm=False):
    if record.get("returncode") != 0 or record.get("error"):
        raise ValueError("test command failed")
    stdout = record.get("stdout", "")
    passed = re.findall(r"^test (\S+) \.\.\. ok$", stdout, re.MULTILINE)
    summary = re.search(r"test result: ok\. (\d+) passed; 0 failed;", stdout)
    if not summary or int(summary[1]) != len(passed) or len(set(passed)) != len(passed):
        raise ValueError("malformed, duplicate or incomplete test output")
    admitted = {name for name in passed if name.startswith("wasm::")} if wasm else set(passed)
    if admitted != set(expected):
        raise ValueError(f"required tests absent/ignored/changed: {sorted(set(expected) - admitted)}; unexpected: {sorted(admitted - set(expected))}")
    return sorted(admitted)


def validate_prefreeze_report(value, files):
    protocol = report.validate_protocol(freeze.read_record(ROOT / "protocol.json"))
    if (not isinstance(value, dict) or value.get("schema_version") != 2
            or value.get("kind") != "two-module-conformance" or value.get("passed") is not True
            or value.get("decision_eligible") is not False or value.get("errors") != []
            or value.get("samples") != [] or value.get("source_files") != files
            or value.get("protocol") != protocol
            or value.get("protocol_sha256") != files["protocol.json"]):
        raise ValueError("need complete passing prefreeze evidence for the exact source/protocol")
    build = value.get("build", {})
    if (build.get("kind") != "source-built-conformance-artifacts" or build.get("schema_version") != 2
            or build.get("source_files") != files):
        raise ValueError("prefreeze evidence lacks exact source-built artifact record")
    verify_artifacts(build)
    commands = value.get("commands")
    if not isinstance(commands, list) or any(not isinstance(row, dict) for row in commands):
        raise ValueError("missing prefreeze commands")
    units = [row for row in commands if row.get("role") == "host-unit"]
    if len(units) != 1:
        raise ValueError("missing/duplicate host gate evidence")
    test_gate(units[0], WASM_GATES, wasm=True)
    functional = value.get("functional", {})
    verdict = report.functional(functional, report.MODES)
    if value.get("functional_verdict") != verdict:
        raise ValueError("prefreeze functional summary disagrees with its retained cases")
    for mode in report.MODES:
        retained = [row for row in commands if row.get("role") == f"functional:{mode}"]
        if (len(retained) != 1 or retained[0].get("returncode") != 0
                or retained[0].get("error") or retained[0].get("value") != functional[mode]):
            raise ValueError("prefreeze case values are missing from successful command evidence")
    return report.registration_ids(functional, 2)


REVIEW_CHECKS = {"declarative_changes_only", "contract_only_dependencies",
                 "new_independent_state_and_lifecycle", "all_four_modes_exercised",
                 "no_core_specific_changes"}


def semantic_review(value, frozen_hash, files, name, module_id, changes):
    checks = value.get("checks") if isinstance(value, dict) else None
    if (not isinstance(value, dict) or value.get("schema_version") != 1
            or value.get("kind") != "independent-module-semantic-review"
            or value.get("verdict") != "pass" or value.get("freeze_sha256") != frozen_hash
            or value.get("source_files") != files or value.get("module") != name
            or value.get("module_id") != module_id or type(value.get("module_id")) is not int
            or value.get("changed_declarative_paths") != changes
            or not isinstance(value.get("reviewer"), str) or not value["reviewer"].strip()
            or not isinstance(value.get("rationale"), str) or not value["rationale"].strip()
            or not isinstance(checks, dict) or set(checks) != REVIEW_CHECKS
            or any(verdict is not True for verdict in checks.values())):
        raise ValueError("missing/stale/incomplete independent-module semantic review")
    return {"passed": True, "reviewer": value["reviewer"], "module_id": module_id}


def decision_eligible(measure, errors, review, measurements):
    return bool(measure and not errors and review.get("passed") is True
                and measurements.get("passed") is True)


def write_new(path, value):
    encoded = json.dumps(value, indent=2, sort_keys=True).encode() + b"\n"
    with path.open("xb") as stream:
        stream.write(gzip.compress(encoded, mtime=0) if path.suffix == ".gz" else encoded)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-record", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--freeze", type=Path)
    parser.add_argument("--module")
    parser.add_argument("--measure", action="store_true")
    parser.add_argument("--review-record", type=Path)
    args = parser.parse_args()
    freeze.output_preflight(args.output)
    if bool(args.freeze) != bool(args.module) or (args.measure and not args.freeze):
        raise ValueError("measurement requires --freeze and --module after the independent extension")
    if ((args.measure and not args.review_record) or (args.review_record and not args.freeze)
            or (args.review_record and args.review_record.resolve().is_relative_to(ROOT))):
        raise ValueError("measurement requires the real semantic review record outside source")
    build = verify_build(args.build_record)
    protocol = report.validate_protocol(freeze.read_record(ROOT / "protocol.json"))
    output = {
        "schema_version": 2,
        "kind": "three-module-conformance" if args.freeze else "two-module-conformance",
        "passed": False, "decision_eligible": False,
        "created_utc": datetime.now(timezone.utc).isoformat(),
        "git_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "source_files": build["source_files"], "build": build,
        "protocol_sha256": sha(ROOT / "protocol.json"), "protocol": protocol,
        "machine": {"architecture": platform.machine(), "platform": platform.platform(),
                    "load_before": os.getloadavg(), "cpu_tick_hz": os.sysconf("SC_CLK_TCK")},
        "commands": [], "functional": {}, "samples": [], "errors": [],
    }
    timeout = protocol["controls"]["command_timeout_seconds"]
    driver = build["artifacts"]["driver"]["path"]
    try:
        count = len(list((ROOT / "modules").glob("*/Cargo.toml")))
        if count != (3 if args.freeze else 2):
            raise ValueError("wrong module population for this challenge phase")
        if args.freeze:
            frozen = freeze.read_record(args.freeze)
            baseline_ids = frozen.get("baseline_module_ids")
            if (not isinstance(baseline_ids, list) or len(baseline_ids) != 2
                    or any(type(number) is not int or number <= 0 for number in baseline_ids)
                    or len(set(baseline_ids)) != 2):
                raise ValueError("freeze lacks verified two-module baseline identities")
            output["freeze_check"] = freeze.compare(frozen, freeze.file_set(ROOT), args.module)
            output["freeze_sha256"] = sha(args.freeze)
            if not output["freeze_check"]["source_freeze_pass"]:
                raise ValueError("frozen host/adapter/driver changed")
        unit = command(["cargo", "test", "--offline", "--locked", "--manifest-path", ROOT / "Cargo.toml",
                        "-p", "conformance-host", "--features", "wasm", "--lib", "--", "--test-threads=1"],
                       timeout, parse_json=False)
        unit["role"] = "host-unit"
        output["commands"].append(unit)
        try:
            output["host_gate"] = test_gate(unit, WASM_GATES, wasm=True)
        except ValueError as exc:
            output["errors"].append(str(exc))
        for mode in protocol["modes"]:
            verify_artifacts(build)
            result = command([driver, "checks", mode], timeout)
            result["role"] = f"functional:{mode}"
            output["commands"].append(result)
            print(f"Functional {mode}: exit={result.get('returncode')}", flush=True)
            if result.get("error"):
                output["errors"].append(f"functional command failed: {mode}")
            if "value" in result:
                output["functional"][mode] = result["value"]
        try:
            output["functional_verdict"] = report.functional(output["functional"], protocol["modes"])
            module_ids = report.registration_ids(output["functional"], count)
            output["registered_and_executed_module_ids"] = module_ids
            if args.freeze and (not set(baseline_ids).issubset(module_ids)
                                or len(set(module_ids) - set(baseline_ids)) != 1):
                raise ValueError("third module is not independently registered/executed in every mode")
        except (ValueError, KeyError, TypeError) as exc:
            output["errors"].append(str(exc))
        if args.freeze:
            challenge = command(["cargo", "test", "--offline", "--locked", "--manifest-path", ROOT / "Cargo.toml",
                                 "-p", "conformance-driver", "--features", "wasm", "--test", args.module,
                                 "--", "--test-threads=1"], timeout, parse_json=False)
            output["commands"].append(challenge)
            try:
                output["challenge_tests"] = test_gate(challenge, report.CHALLENGE_TESTS)
            except ValueError as exc:
                output["errors"].append(str(exc))
            if args.review_record and not output["errors"]:
                new_id = next(iter(set(module_ids) - set(baseline_ids)))
                output["semantic_review"] = semantic_review(freeze.read_record(args.review_record),
                    output["freeze_sha256"], output["source_files"], args.module, new_id,
                    output["freeze_check"]["changed_declarative_paths"])
                output["semantic_review_sha256"] = sha(args.review_record)
        if args.measure and not output["errors"]:
            configurations = [
                ("storage", [str(n), density, str(protocol["storage"]["ticks"])])
                for n, density in itertools.product(protocol["populations"], protocol["densities"])
            ] + [
                ("dispatch", [workload, str(calls)])
                for calls, workload in itertools.product(protocol["dispatch"]["calls"], protocol["dispatch"]["workloads"])
            ]
            for repetition in range(protocol["repetitions"]):
                seed = protocol["first_seed"] + repetition
                modes = protocol["modes"]
                order = modes[repetition % len(modes):] + modes[:repetition % len(modes)]
                for kind, params in configurations:
                    for mode in order:
                        verify_artifacts(build)
                        result = command([driver, kind, mode, *params, str(seed)], timeout)
                        output["commands"].append(result)
                        print(f"Sample seed={seed} {kind} {params} {mode}: exit={result.get('returncode')}", flush=True)
                        if result.get("error"):
                            output["errors"].append(f"failed sample: {seed} {kind} {params} {mode}")
                        if "value" in result:
                            output["samples"].append(result["value"])
            try:
                output["measurement_verdict"] = report.summarize(output["samples"], protocol)
                if not output["measurement_verdict"]["passed"]:
                    output["errors"].append("one or more predeclared resource/cost budgets failed")
            except (ValueError, KeyError, TypeError) as exc:
                output["errors"].append(str(exc))
        if freeze.file_set(ROOT) != output["source_files"]:
            output["errors"].append("source changed during campaign")
        verify_artifacts(build)
        output["passed"] = not output["errors"]
        output["decision_eligible"] = decision_eligible(args.measure, output["errors"],
            output.get("semantic_review", {}), output.get("measurement_verdict", {}))
    except (ValueError, KeyError, TypeError, OSError) as exc:
        output["errors"].append(str(exc))
        output["passed"] = False
        output["decision_eligible"] = False
    finally:
        output["machine"]["load_after"] = os.getloadavg()
        output["finished_utc"] = datetime.now(timezone.utc).isoformat()
        write_new(args.output, output)
    print(json.dumps({"passed": output["passed"], "decision_eligible": output["decision_eligible"],
                      "report": str(args.output), "errors": output["errors"]}), flush=True)
    return 0 if output["passed"] else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, KeyError, OSError) as exc:
        print(f"campaign rejected: {exc}", file=sys.stderr)
        raise SystemExit(2)
