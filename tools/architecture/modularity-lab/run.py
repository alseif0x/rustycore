#!/usr/bin/env python3
"""Run paired, isolated lab samples; preserve negative results without selecting winners."""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import itertools
import json
import math
import os
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import time

LAB = Path(__file__).resolve().parent
REPO = LAB.parents[2]

STORAGE_CHECKS = {
    "non_clone_move_failed_attach_stale_replacement",
    "reciprocal_combat_atomic_admission",
    "independent_optional_families_conflict",
    "timer_partial_failure_synchronous_callback_read_reset",
    "invoked_map_barrier_callback_before_removal",
}
EXECUTION_CHECKS = {
    "actual_rust_guest_native_full_trace_equality",
    "phase_shield_before_action_same_guest_reentry_read_after_action",
    "fallible_summon_preserves_prior_effect_then_reset",
    "fuel_interrupts_actual_guest_infinite_loop", "memory_growth_limit",
    "oversize_payload_rejected_before_allocation", "out_of_bounds_payload_rejected",
    "forged_handle_rejected", "stale_handle_rejected", "unauthorized_action_rejected",
    "cumulative_hostcall_budget", "output_cap_does_not_rollback_prior_effect",
    "callback_depth_cap", "nested_fuel_exhaustion_before_depth_cap",
    "native_wasm_callback_failure_stops_before_followup_mutation",
    "trap_after_reward_does_not_undo_or_replay_effect",
    "actual_v2_binary_migrates_state_and_retains_mock_receipt_idempotency",
    "invalid_configuration_is_rejected",
}
EXECUTION_WORK = ["calls", "seed", "warmup_calls", "checksum", "final_observables", "calls_by_event"]


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def sources_digest() -> str:
    hashed = hashlib.sha256()
    paths = []
    for base, directories, names in os.walk(LAB):
        directories[:] = sorted(d for d in directories if d not in {"target", "target-v2", "results", "__pycache__"})
        paths.extend(Path(base) / n for n in names if n.endswith((".rs", ".toml", ".lock", ".json", ".py", ".md")))
    for path in sorted(paths):
        hashed.update(str(path.relative_to(LAB)).encode() + b"\0" + path.read_bytes() + b"\0")
    return hashed.hexdigest()


def command_json(command: list[str], timeout: int) -> dict:
    result = subprocess.run(command, capture_output=True, text=True, timeout=timeout, check=False)
    if result.returncode:
        raise RuntimeError(f"Command failed ({result.returncode}): {command}\n{result.stdout}\n{result.stderr}")
    value = json.loads(result.stdout)
    if not isinstance(value, dict) or value.get("schema_version") != 1:
        raise ValueError(f"Invalid result schema: {command}")
    return value


def check_result(value: dict, suite: str) -> None:
    success = value.get("ok", value.get("success"))
    checks = value.get("checks")
    if success is not True or not isinstance(checks, list) or not checks:
        raise ValueError("Functional check missing or not successful")
    if any(not isinstance(c, dict) or c.get("passed") is not True for c in checks):
        raise ValueError("A required functional check failed or was malformed")
    if suite == "storage":
        expected = Counter({(backend, name): 1 for backend in ["aggregate", "hecs"] for name in STORAGE_CHECKS})
        expected.update({("cross-backend", "seeded_sparse_exact_trace_and_state"): 3,
                         ("cross-backend", "seeded_dense_exact_trace_and_state"): 3})
        actual = Counter((c.get("backend"), c.get("name")) for c in checks)
    elif suite == "execution":
        expected = Counter({name: 1 for name in EXECUTION_CHECKS})
        actual = Counter(c.get("name") for c in checks)
    else:
        raise ValueError("Unknown functional suite")
    if expected - actual:
        raise ValueError(f"Missing required functional cases: {expected - actual}")


def check_storage_population(value: dict) -> None:
    entities, ticks = value["entities"], value["ticks"]
    optional = entities if value["density"] == "dense" else entities // 4
    operations = value.get("operations", {})
    if (value.get("final_entities") != entities or value.get("final_optional") != optional
            or operations.get("updates") != entities * ticks
            or numeric(operations, "churn") + numeric(operations, "transfers") != ticks * max(entities // 100, 1)):
        raise ValueError("Storage sample violates independent population/work-count oracle")


def numeric(value: dict, key: str, *, positive: bool = False) -> float:
    number = value.get(key)
    if isinstance(number, bool) or not isinstance(number, (int, float)) or not math.isfinite(number):
        raise ValueError(f"Missing/nonfinite numeric {key}: {number!r}")
    if number < 0 or (positive and number == 0):
        raise ValueError(f"Invalid numeric {key}: {number!r}")
    return number


def check_execution_work(value: dict) -> None:
    frequencies = value.get("calls_by_event", {})
    names = {"xp", "summon_success", "summon_failure", "reset", "reward"}
    if (not isinstance(frequencies, dict) or set(frequencies) != names
            or sum(numeric(frequencies, name) for name in names) != value["calls"]
            or value.get("warmup_calls") != 256
            or not isinstance(value.get("final_observables"), dict)
            or not value["final_observables"]):
        raise ValueError("Execution sample violates invocation-count/warmup/observables oracle")


def same_work(pair: dict, keys: list[str]) -> None:
    first, second = pair.values()
    for key in keys:
        if key not in first or key not in second or first[key] != second[key]:
            raise ValueError(f"Semantic/workload disagreement in {key}")


def spread(values: list[float]) -> dict:
    return {"min": min(values), "median": statistics.median(values), "max": max(values)}


def summarize_storage(rows: list[dict], config: dict) -> list[dict]:
    grouped = {}
    for row in rows:
        grouped.setdefault((row["entities"], row["density"]), []).append(row)
    summaries = []
    for (entities, density), runs in sorted(grouped.items()):
        ratios, memory = [], []
        for run in runs:
            base, candidate = run["samples"]["aggregate"], run["samples"]["hecs"]
            same_work(run["samples"], ["entities", "ticks", "seed", "density", "checksum", "final_entities", "final_optional", "operations"])
            ratios.append(numeric(candidate, "update_p99_ns", positive=True) / numeric(base, "update_p99_ns", positive=True))
            memory.append(numeric(candidate, "rss_kib", positive=True) <=
                          config["maximum_rss_baseline_multiplier"] * numeric(base, "rss_kib", positive=True) +
                          config["maximum_rss_additive_kib"])
        summaries.append({
            "entities": entities, "density": density, "paired_runs": len(runs),
            "update_p99_ratio": spread(ratios),
            "lab_tail_budget_pass": statistics.median(ratios) <= config["maximum_median_paired_update_p99_ratio"],
            "lab_rss_budget_pass": all(memory),
            "rss_budget_failed_runs": [i for i, passed in enumerate(memory) if not passed],
            "metrics": {backend: {key: spread([numeric(r["samples"][backend], key) for r in runs])
                                   for key in ["build_ns", "update_p50_ns", "update_p95_ns", "update_p99_ns", "sort_ns", "churn_ns", "transfer_ns", "rss_kib", "vmhwm_kib"]}
                        for backend in ["aggregate", "hecs"]},
        })
    return summaries


def summarize_execution(rows: list[dict], config: dict) -> list[dict]:
    summaries = []
    for calls in sorted({r["calls"] for r in rows}):
        runs = [r for r in rows if r["calls"] == calls]
        for run in runs:
            same_work(run["samples"], EXECUTION_WORK)
        tails = [numeric(r["samples"]["wasm"], "p99_ns", positive=True) for r in runs]
        memory = [numeric(r["samples"]["wasm"], "rss_kib", positive=True) for r in runs]
        ratios = [numeric(r["samples"]["wasm"], "total_ns", positive=True) /
                  numeric(r["samples"]["native"], "total_ns", positive=True) for r in runs]
        summaries.append({
            "calls": calls, "paired_runs": len(runs), "wasm_native_total_ratio": spread(ratios),
            "lab_tail_budget_pass": statistics.median(tails) <= config["maximum_median_wasm_p99_ns"],
            "lab_rss_budget_pass": max(memory) <= config["maximum_rss_kib"],
            "metrics": {backend: {key: spread([numeric(r["samples"][backend], key) for r in runs])
                                   for key in ["total_ns", "p50_ns", "p95_ns", "p99_ns", "rss_kib", "cold_compile_ns", "instantiate_ns"]}
                        for backend in ["native", "wasm"]},
        })
    return summaries


def text_command(command: list[str]) -> str:
    return subprocess.check_output(command, text=True, cwd=REPO, timeout=30).strip()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--suite", choices=["all", "storage", "execution"], default="all")
    parser.add_argument("--smoke", action="store_true")
    parser.add_argument("--storage-bin", type=Path, default=LAB / "storage/target/release/storage-lab")
    parser.add_argument("--execution-bin", type=Path, default=LAB / "execution/target/release/execution-lab")
    parser.add_argument("--guest", type=Path, default=LAB / "execution/guest/target/wasm32-unknown-unknown/release/execution_lab_guest.wasm")
    parser.add_argument("--guest-v2", type=Path, default=LAB / "execution/guest/target-v2/wasm32-unknown-unknown/release/execution_lab_guest.wasm")
    args = parser.parse_args()
    if args.output.exists():
        parser.error("Output already exists; use a new path to retain earlier evidence")
    protocol_path = LAB / "protocol.json"
    protocol = json.loads(protocol_path.read_text())
    repetitions = 2 if args.smoke else protocol["repetitions"]
    timeout = protocol["controls"]["command_timeout_seconds"]
    source_hash = sources_digest()
    report = {
        "schema_version": 1, "experiment": protocol["experiment"], "decision_eligible": False,
        "suite": args.suite, "protocol": protocol, "protocol_sha256": digest(protocol_path),
        "source_sha256": source_hash, "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "host": {"architecture": platform.machine(), "platform": platform.platform(), "cpu_count": os.cpu_count(),
                 "load_start": os.getloadavg(), "rustc": text_command(["rustc", "-Vv"])},
        "git_head": text_command(["git", "rev-parse", "HEAD"]),
        "git_status": text_command(["git", "status", "--short", "--", str(LAB)]),
        "checks": {}, "storage": [], "execution": [], "artifacts": {},
        "functional_checks_pass": False, "functional_pass": False, "resource_budgets_pass": False,
        "production_acceptance": False, "campaign_valid": False,
    }
    exit_code = 1
    try:
        active = []
        if args.suite in {"all", "storage"}:
            active.append(("storage", args.storage_bin, [str(args.storage_bin), "check"]))
        if args.suite in {"all", "execution"}:
            report["artifacts"]["guest_sha256"] = digest(args.guest)
            report["artifacts"]["guest_v2_sha256"] = digest(args.guest_v2)
            if report["artifacts"]["guest_sha256"] == report["artifacts"]["guest_v2_sha256"]:
                raise ValueError("The two guest versions must be different binaries")
            active.append(("execution", args.execution_bin, [str(args.execution_bin), "check", "--guest", str(args.guest), "--guest-v2", str(args.guest_v2)]))
        for name, binary, command in active:
            report["artifacts"][name + "_sha256"] = digest(binary)
            value = command_json(command, timeout)
            report["checks"][name] = value
            check_result(value, name)
        report["functional_checks_pass"] = True
        if args.suite in {"all", "storage"}:
            config = protocol["storage"]
            populations = [1000] if args.smoke else config["entities"]
            ticks = 30 if args.smoke else config["ticks"]
            for entities, density, repeat in itertools.product(populations, config["densities"], range(repetitions)):
                seed = protocol["first_seed"] + repeat
                row = {"entities": entities, "density": density, "repeat": repeat, "seed": seed, "samples": {}}
                report["storage"].append(row)
                order = ["aggregate", "hecs"] if repeat % 2 == 0 else ["hecs", "aggregate"]
                row["order"] = order
                for backend in order:
                    print(f"storage {entities}/{density} pair {repeat + 1}/{repetitions}: {backend}", flush=True)
                    value = command_json([str(args.storage_bin), "bench", "--backend", backend, "--entities", str(entities),
                                          "--ticks", str(ticks), "--seed", str(seed), "--density", density], timeout)
                    expected = {"backend": backend, "entities": entities, "density": density,
                                "seed": seed, "ticks": ticks, "warmup_ticks": config["warmup_ticks"]}
                    if any(value.get(key) != expected_value for key, expected_value in expected.items()):
                        raise ValueError("Storage sample does not match requested workload/warmup")
                    check_storage_population(value)
                    row["samples"][backend] = value
                same_work(row["samples"], ["entities", "ticks", "seed", "density", "checksum", "final_entities", "final_optional", "operations"])
            report["storage_summary"] = summarize_storage(report["storage"], config)
        if args.suite in {"all", "execution"}:
            config = protocol["execution"]
            calls_list = [1000] if args.smoke else config["calls"]
            for calls, repeat in itertools.product(calls_list, range(repetitions)):
                seed = protocol["first_seed"] + repeat
                row = {"calls": calls, "repeat": repeat, "seed": seed, "samples": {}}
                report["execution"].append(row)
                order = ["native", "wasm"] if repeat % 2 == 0 else ["wasm", "native"]
                row["order"] = order
                for backend in order:
                    print(f"execution {calls} pair {repeat + 1}/{repetitions}: {backend}", flush=True)
                    value = command_json([str(args.execution_bin), "bench", "--backend", backend, "--guest", str(args.guest),
                                          "--calls", str(calls), "--seed", str(seed)], timeout)
                    if value.get("backend") != backend or value.get("calls") != calls or value.get("seed") != seed:
                        raise ValueError("Execution sample does not match requested backend/workload")
                    check_execution_work(value)
                    row["samples"][backend] = value
                same_work(row["samples"], EXECUTION_WORK)
            report["execution_summary"] = summarize_execution(report["execution"], config)
        if sources_digest() != source_hash:
            raise ValueError("Lab source changed during measurement; discard this campaign for decisions")
        for name, binary, _ in active:
            if digest(binary) != report["artifacts"][name + "_sha256"]:
                raise ValueError("Executable changed during measurement")
        if args.suite in {"all", "execution"}:
            for key, guest in [("guest_sha256", args.guest), ("guest_v2_sha256", args.guest_v2)]:
                if digest(guest) != report["artifacts"][key]:
                    raise ValueError("Guest changed during measurement")
        report["campaign_valid"] = True
        report["functional_pass"] = True
        report["decision_eligible"] = not args.smoke
        summaries = report.get("storage_summary", []) + report.get("execution_summary", [])
        report["resource_budgets_pass"] = bool(summaries) and all(
            r["lab_tail_budget_pass"] and r["lab_rss_budget_pass"] for r in summaries)
        exit_code = 0 if report["resource_budgets_pass"] else 2
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        report["error"] = str(error)
        print(str(error), file=sys.stderr)
    finally:
        report["host"]["load_end"] = os.getloadavg()
        report["finished_utc"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        report["exit_code"] = exit_code
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with args.output.open("x") as output:
            json.dump(report, output, indent=2, sort_keys=True)
            output.write("\n")
        print(f"Report: {args.output}; functional={report['functional_pass']}; budgets={report['resource_budgets_pass']}")
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
