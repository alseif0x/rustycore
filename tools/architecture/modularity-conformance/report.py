"""Fail-closed campaign oracles; no absent sample or numerical field becomes zero."""
from __future__ import annotations

from collections import Counter, defaultdict
import math
import re
import statistics

MODES = ["native", "rust-wasm", "c-wasm", "mixed"]
CHALLENGE_TESTS = {
    "independent_module_native_lifecycle",
    "independent_module_rust_wasm_lifecycle",
    "independent_module_c_wasm_lifecycle",
    "independent_module_mixed_lifecycle",
}


def validate_protocol(value):
    """Reject accidental matrix reductions; this is the finite preregistered V2, not a framework."""
    if (not isinstance(value, dict) or value.get("schema_version") != 2
            or value.get("experiment") != "modularity-conformance-v2"):
        raise ValueError("invalid V2 protocol identity")
    fixed = {"modes": MODES, "populations": [1000, 10000],
             "densities": ["sparse", "dense"], "repetitions": 10, "first_seed": 42,
             "sparse_fraction": {"numerator": 1, "denominator": 4}}
    for name, expected in fixed.items():
        if value.get(name) != expected or type(value.get(name)) is not type(expected):
            raise ValueError(f"V2 protocol matrix changed or incomplete: {name}")
    sections = {
        "storage": {"warmup_ticks": 25, "ticks": 200,
                    "churn_fraction_per_tick": 0.01, "transfer_fraction_per_tick": 0.01},
        "dispatch": {"warmup_calls": 256, "calls": [10000, 100000],
                     "workloads": ["policy", "reentry"]},
        "resources": {"max_modules": 8, "max_state_bytes_per_module_entity": 256,
                      "max_host_calls_per_root": 256, "max_callback_depth": 8,
                      "max_trace_entries_per_root": 4096, "wasm_fuel_per_root": 1000000,
                      "wasm_memory_bytes_per_instance": 3145728, "maximum_wasm_instances": 8,
                      "maximum_wasm_memories": 8, "maximum_wasm_linear_memory_total_bytes": 25165824,
                      "maximum_wasm_artifact_bytes": 4194304},
    }
    for name, fields in sections.items():
        section = value.get(name)
        if not isinstance(section, dict):
            raise ValueError(f"missing protocol section: {name}")
        for key, expected in fields.items():
            if section.get(key) != expected or type(section.get(key)) is not type(expected):
                raise ValueError(f"V2 contract/matrix changed: {name}.{key}")
    for name, keys in {
        "storage": ["maximum_median_update_p99_ns_per_entity_batch_amortized",
                    "maximum_median_churn_ns_per_operation", "maximum_median_transfer_ns_per_operation"],
        "dispatch": ["maximum_median_root_p99_ns"],
        "resources": ["maximum_rss_kib", "maximum_cold_compile_ns_per_artifact",
                      "maximum_instantiate_ns_per_artifact"],
        "controls": ["command_timeout_seconds"],
    }.items():
        section = value.get(name)
        if not isinstance(section, dict):
            raise ValueError(f"missing protocol section: {name}")
        for key in keys:
            numeric(section, key, positive=True)
    for key in ["fresh_process_per_sample", "rotate_mode_order_by_seed",
                "same_seed_and_workload_across_modes", "no_concurrent_lab_builds_or_benchmarks",
                "correctness_before_timing", "retain_failed_samples",
                "retain_raw_output_for_malformed_or_failed_commands"]:
        if value["controls"].get(key) is not True:
            raise ValueError(f"missing preregistered control: {key}")
    return value

COMMON = {
    "registration_order", "zero_optional_neutrality", "policy_and_state_isolation",
    "summon_reentry_order", "nullable_summon_partial_effects", "stale_outer_write",
    "fallible_action_partial_effects", "active_detached_transfer", "failed_attach_retains_state",
    "stale_and_forged_handles", "reset_scoped_contribution", "unload_preserves_other_module",
    "versioned_snapshot_replay", "stale_snapshot_rejected", "unsupported_executor_switch",
    "cumulative_host_calls", "bounded_reentry_depth", "output_limit_partial_effects",
    "mixed_reverse_reentry", "callback_result_oracle",
}
NATIVE = {
    "invalid_registration_identity", "invalid_registration_versions",
    "invalid_registration_capabilities", "registration_conflicts", "module_count_limit",
    "registration_state_limit", "malformed_state_write", "oversized_state_write",
    "shared_state_type_isolation",
}


def numeric(value, key, *, positive=False):
    result = value.get(key)
    if (isinstance(result, bool) or not isinstance(result, (int, float))
            or not math.isfinite(result) or result < 0 or (positive and result == 0)):
        raise ValueError(f"invalid numeric field {key}: {result!r}")
    return result


def observations(value, key, expected, quantiles):
    if not isinstance(expected, int) or isinstance(expected, bool) or expected <= 0:
        raise ValueError("observation count must be a positive integer")
    if not isinstance(quantiles, dict):
        raise ValueError("missing quantiles")
    raw = value.get(key)
    if (not isinstance(raw, list) or len(raw) != expected
            or any(isinstance(v, bool) or not isinstance(v, int) or v < 0 for v in raw)):
        raise ValueError(f"missing/invalid raw observations: {key}")
    ordered = sorted(raw)
    for percentile in (50, 95, 99):
        rank = math.ceil(len(ordered) * percentile / 100) - 1
        if quantiles.get(f"p{percentile}_ns") != ordered[rank]:
            raise ValueError(f"quantile disagrees with retained raw observations: {key}")
    return sum(raw)


def functional(results, modes):
    if modes != MODES:
        raise ValueError("functional conformance requires all four exact execution modes")
    if set(results) != set(modes):
        raise ValueError("missing or unexpected execution mode")
    by_mode = {}
    for mode, value in results.items():
        if (value.get("schema_version") != 2 or value.get("kind") != "mode-contract-checks"
                or value.get("mode") != mode or value.get("passed") is not True):
            raise ValueError(f"failed or malformed functional result: {mode}")
        checks = value.get("checks")
        if not isinstance(checks, list) or any(not isinstance(c, dict) for c in checks):
            raise ValueError("malformed checks")
        expected = COMMON | (NATIVE if mode == "native" else set())
        counts = Counter(check.get("name") for check in checks)
        if counts != Counter({name: 1 for name in expected}):
            raise ValueError(f"missing, duplicate or unexpected checks: {mode}: {counts}")
        if any(check.get("passed") is not True or not isinstance(check.get("detail"), dict)
               for check in checks):
            raise ValueError(f"failed/missing oracle: {mode}")
        by_mode[mode] = {check["name"]: check["detail"] for check in checks}
    reference = by_mode["native"]
    differences = []
    for mode, checks in by_mode.items():
        for name in COMMON:
            if checks[name] != reference[name]:
                differences.append({"mode": mode, "case": name})
    if differences:
        raise ValueError(f"semantic disagreement, including ordered trace/results/state: {differences}")
    return {"passed": True, "modes": len(modes), "common_cases_per_mode": len(COMMON),
            "native_negative_cases": len(NATIVE), "exact_oracles_equal": True}


def spread(values):
    if not values:
        raise ValueError("empty sample population")
    return {"min": min(values), "median": statistics.median(values), "max": max(values)}


def cold(value, resources):
    measurements = value.get("cold")
    if not isinstance(measurements, dict) or not isinstance(measurements.get("artifacts"), list):
        raise ValueError("cold observations absent")
    artifacts = measurements["artifacts"]
    expected = measurements.get("expected_wasm_modules")
    if (not isinstance(expected, list)
            or any(type(module) is not int or module <= 0 for module in expected)
            or len(set(expected)) != len(expected)
            or any(not isinstance(row, dict) for row in artifacts)):
        raise ValueError("missing/invalid declared executor composition")
    if {row.get("module_id") for row in artifacts} != set(expected):
        raise ValueError("cold artifact observations do not cover the exact Wasm composition")
    if value["mode"] == "native":
        if artifacts or measurements.get("engine_creation_ns") is not None:
            raise ValueError("native compile cost cannot be inferred as Wasm cold cost")
        return True
    numeric(measurements, "engine_creation_ns", positive=True)
    if not artifacts or len({row.get("module_id") for row in artifacts}) != len(artifacts):
        raise ValueError("missing/duplicate cold artifact observations")
    passing = True
    for row in artifacts:
        if row.get("fault") is not None:
            passing = False
        compile_ns = numeric(row, "compile_ns", positive=True)
        instantiate_ns = numeric(row, "instantiate_ns", positive=True)
        numeric(row, "metadata_ns", positive=True)
        passing &= compile_ns <= resources["maximum_cold_compile_ns_per_artifact"]
        passing &= instantiate_ns <= resources["maximum_instantiate_ns_per_artifact"]
    return passing


def same(rows, fields):
    reference = rows[0]
    for row in rows[1:]:
        for field in fields:
            if field not in reference or row.get(field) != reference[field]:
                raise ValueError(f"sample semantic/work-count disagreement in {field}")


def sample(value, protocol):
    if value.get("schema_version") != 2 or value.get("mode") not in protocol["modes"]:
        raise ValueError("invalid sample schema/mode")
    if not isinstance(value.get("final_digest"), str) or not re.fullmatch("[0-9a-f]{64}", value["final_digest"]):
        raise ValueError("missing/invalid full-state digest")
    rss = numeric(value, "rss_kib", positive=True)
    peak = numeric(value, "rss_hwm_kib", positive=True)
    memory = max(rss, peak) <= protocol["resources"]["maximum_rss_kib"]
    cold_pass = cold(value, protocol["resources"])
    if value.get("kind") == "storage-sample":
        population, ticks = numeric(value, "population", positive=True), numeric(value, "ticks", positive=True)
        if population not in protocol["populations"] or ticks != protocol["storage"]["ticks"]:
            raise ValueError("unexpected storage population/ticks")
        if value.get("density") not in protocol["densities"]:
            raise ValueError("unexpected density")
        optional = population if value["density"] == "dense" else population // 4
        count = population // 100 * ticks
        if (value.get("warmup_ticks") != protocol["storage"]["warmup_ticks"]
                or value.get("final_entities") != population
                or value.get("optional_entities") != optional
                or value.get("final_optional_entities") != optional
                or value.get("final_module_states") != optional * numeric(value, "module_count", positive=True)
                or value.get("operations") != {"updates": population * ticks, "churn": count,
                    "transfer": count, "cross_map_transfers": count, "same_map_transfers": 0}):
            raise ValueError("storage population/lifetime/work-count oracle failed")
        observations(value, "raw_update_ns", ticks, value["update"])
        if observations(value, "raw_churn_ns", count, value["churn"]) != value.get("churn_total_ns"):
            raise ValueError("churn total disagrees with raw observations")
        if observations(value, "raw_transfer_ns", count, value["transfer"]) != value.get("transfer_total_ns"):
            raise ValueError("transfer total disagrees with raw observations")
        update = numeric(value["update"], "p99_ns", positive=True) / population
        churn = numeric(value, "churn_total_ns", positive=True) / count
        transfer = numeric(value, "transfer_total_ns", positive=True) / count
        return {"memory": memory, "cold": cold_pass, "update_amortized_ns": update,
                "churn_mean_ns": churn, "transfer_mean_ns": transfer}
    if value.get("kind") == "dispatch-sample":
        calls = numeric(value, "calls", positive=True)
        if (calls not in protocol["dispatch"]["calls"]
                or value.get("workload") not in protocol["dispatch"]["workloads"]
                or value.get("warmup_calls") != protocol["dispatch"]["warmup_calls"]
                or value.get("expected_rejections") != (calls if value["workload"] == "reentry" else 0)
                or not value.get("result_digest") or not value.get("invocations_by_module_event")):
            raise ValueError("dispatch result/work-count oracle failed")
        numeric(value, "admitted_calls", positive=True)
        if not isinstance(value.get("result_digest"), str) or not re.fullmatch("[0-9a-f]{64}", value["result_digest"]):
            raise ValueError("missing/invalid result digest")
        observations(value, "raw_latencies_ns", calls, value["latency"])
        return {"memory": memory, "cold": cold_pass,
                "root_p99_ns": numeric(value["latency"], "p99_ns", positive=True)}
    raise ValueError("unknown sample kind")


def summarize(rows, protocol):
    validate_protocol(protocol)
    expected = protocol["repetitions"]
    seeds = set(range(protocol["first_seed"], protocol["first_seed"] + expected))
    grouped = defaultdict(list)
    for row in rows:
        kind = row.get("kind")
        if kind == "storage-sample":
            key = (kind, row.get("population"), row.get("density"))
        elif kind == "dispatch-sample":
            key = (kind, row.get("calls"), row.get("workload"))
        else:
            raise ValueError("unknown/malformed result row")
        grouped[key].append(row)
    expected_groups = (len(protocol["populations"]) * len(protocol["densities"])
                       + len(protocol["dispatch"]["calls"]) * len(protocol["dispatch"]["workloads"]))
    if len(grouped) != expected_groups:
        raise ValueError("missing measurement configurations")
    summaries = []
    for key, values in sorted(grouped.items()):
        counts = Counter((r["mode"], r["seed"]) for r in values)
        if counts != Counter({(mode, seed): 1 for mode in protocol["modes"] for seed in seeds}):
            raise ValueError(f"missing/duplicate samples for {key}")
        for seed in seeds:
            pair = [r for r in values if r["seed"] == seed]
            fields = ["final_digest"]
            fields += (["operations", "final_entities", "final_optional_entities", "final_module_states"]
                       if key[0] == "storage-sample" else
                       ["result_digest", "invocations_by_module_event", "admitted_calls", "expected_rejections"])
            same(pair, fields)
        for mode in protocol["modes"]:
            evaluated = [sample(row, protocol) for row in values if row["mode"] == mode]
            metrics = {name: spread([row[name] for row in evaluated])
                       for name in evaluated[0] if name not in {"memory", "cold"}}
            passes = all(row["memory"] and row["cold"] for row in evaluated)
            if key[0] == "storage-sample":
                config = protocol["storage"]
                passes &= metrics["update_amortized_ns"]["median"] <= config["maximum_median_update_p99_ns_per_entity_batch_amortized"]
                passes &= metrics["churn_mean_ns"]["median"] <= config["maximum_median_churn_ns_per_operation"]
                passes &= metrics["transfer_mean_ns"]["median"] <= config["maximum_median_transfer_ns_per_operation"]
            else:
                passes &= metrics["root_p99_ns"]["median"] <= protocol["dispatch"]["maximum_median_root_p99_ns"]
            summaries.append({"configuration": key, "mode": mode, "runs": expected,
                              "passed": bool(passes), "metrics": metrics,
                              "all_memory_samples_pass": all(r["memory"] for r in evaluated),
                              "all_cold_samples_pass": all(r["cold"] for r in evaluated)})
    return {"passed": all(row["passed"] for row in summaries), "summaries": summaries}


def registration_ids(functional_results, expected_count):
    """Actual registration and root execution, not a count of Cargo manifests on disk."""
    functional(functional_results, MODES)
    all_ids = []
    for mode in MODES:
        case = next(check for check in functional_results[mode]["checks"]
                    if check["name"] == "registration_order")["detail"]
        values = case.get("values")
        if (not isinstance(values, list) or len(values) != expected_count
                or any(not isinstance(row, list) or len(row) != 2
                       or type(row[0]) is not int or row[0] <= 0 for row in values)):
            raise ValueError(f"registered/executed module population is not {expected_count}: {mode}")
        ids = [row[0] for row in values]
        if len(set(ids)) != len(ids):
            raise ValueError("duplicate executed module")
        oracle = case.get("oracle", {})
        states = oracle.get("states", [])
        if sorted(state.get("module") for state in states) != sorted(ids):
            raise ValueError("module registration does not have all canonical state components")
        entered = [entry.get("module") for entry in oracle.get("trace", [])
                   if entry.get("kind") == "enter" and entry.get("event") == 7
                   and entry.get("argument") == 31]
        returned = [(entry.get("module"), entry.get("result")) for entry in oracle.get("trace", [])
                    if entry.get("kind") == "leave" and entry.get("event") == 7
                    and entry.get("argument") == 31]
        if entered != ids or returned != [(module, {"ok": value}) for module, value in values]:
            raise ValueError("declared modules did not all enter and return through the real dispatcher")
        all_ids.append(sorted(ids))
    if any(ids != all_ids[0] for ids in all_ids):
        raise ValueError("module population differs between execution modes")
    return all_ids[0]
