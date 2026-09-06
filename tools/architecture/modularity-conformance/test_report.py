import copy
import json
from pathlib import Path
import unittest

import report


class FunctionalReportTests(unittest.TestCase):
    def setUp(self):
        self.modes = ["native", "rust-wasm", "c-wasm", "mixed"]
        self.rows = {}
        for mode in self.modes:
            names = report.COMMON | (report.NATIVE if mode == "native" else set())
            self.rows[mode] = {"schema_version": 2, "kind": "mode-contract-checks", "mode": mode,
                               "passed": True, "checks": [
                {"name": name, "passed": True, "detail": {"trace": [1, 2], "state": [3, 4]}}
                for name in sorted(names)]}

    def test_complete_equivalent_matrix(self):
        self.assertTrue(report.functional(self.rows, self.modes)["passed"])

    def test_missing_mode_rejected(self):
        del self.rows["c-wasm"]
        with self.assertRaises(ValueError):
            report.functional(self.rows, self.modes)

    def test_native_only_caller_cannot_reduce_required_modes(self):
        with self.assertRaises(ValueError):
            report.functional({"native": self.rows["native"]}, ["native"])

    def test_missing_or_duplicate_case_rejected(self):
        for duplicate in [False, True]:
            with self.subTest(duplicate=duplicate):
                rows = copy.deepcopy(self.rows)
                check = rows["mixed"]["checks"].pop()
                if duplicate:
                    rows["mixed"]["checks"].extend([check, check])
                with self.assertRaises(ValueError):
                    report.functional(rows, self.modes)

    def test_green_flags_do_not_hide_order_or_state_difference(self):
        for field, value in [("trace", [2, 1]), ("state", [3, 5])]:
            with self.subTest(field=field):
                rows = copy.deepcopy(self.rows)
                rows["c-wasm"]["checks"][0]["detail"][field] = value
                with self.assertRaises(ValueError):
                    report.functional(rows, self.modes)

    def test_failed_check_cannot_hide_behind_top_level_pass(self):
        self.rows["rust-wasm"]["checks"][0]["passed"] = False
        with self.assertRaises(ValueError):
            report.functional(self.rows, self.modes)


class NumericReportTests(unittest.TestCase):
    def test_peak_rss_above_budget_fails_even_when_final_rss_is_small(self):
        protocol = json.loads(Path(__file__).with_name("protocol.json").read_text())
        value = {"schema_version": 2, "mode": "native", "kind": "dispatch-sample",
                 "final_digest": "a" * 64, "result_digest": "b" * 64,
                 "rss_kib": 100, "rss_hwm_kib": protocol["resources"]["maximum_rss_kib"] + 1,
                 "cold": {"artifacts": [], "engine_creation_ns": None, "expected_wasm_modules": []},
                 "calls": 10000, "workload": "policy", "warmup_calls": 256,
                 "expected_rejections": 0, "admitted_calls": 50000,
                 "invocations_by_module_event": {"1:7": 10000, "2:7": 10000},
                 "latency": {"p50_ns": 1, "p95_ns": 1, "p99_ns": 1}, "raw_latencies_ns": [1] * 10000}
        self.assertFalse(report.sample(value, protocol)["memory"])
        value["rss_hwm_kib"] = 100
        self.assertTrue(report.sample(value, protocol)["memory"])

    def test_protocol_rejects_empty_reduced_duplicate_or_nonfinite_matrix(self):
        original = json.loads(Path(__file__).with_name("protocol.json").read_text())
        mutations = [
            ("modes", ["native"]), ("modes", ["native"] * 4), ("populations", []),
            ("repetitions", 0), ("repetitions", True), ("first_seed", -1),
        ]
        for key, value in mutations:
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                report.validate_protocol({**original, key: value})
        for section, key, value in [("dispatch", "calls", []),
                                    ("dispatch", "workloads", []),
                                    ("controls", "command_timeout_seconds", float("nan")),
                                    ("resources", "maximum_rss_kib", True),
                                    ("controls", "retain_failed_samples", False)]:
            changed = copy.deepcopy(original)
            changed[section][key] = value
            with self.subTest(section=section, key=key), self.assertRaises(ValueError):
                report.validate_protocol(changed)

    def test_empty_protocol_cannot_make_an_empty_campaign_pass(self):
        protocol = json.loads(Path(__file__).with_name("protocol.json").read_text())
        protocol["populations"] = []
        protocol["dispatch"]["calls"] = []
        with self.assertRaises(ValueError):
            report.summarize([], protocol)

    def test_missing_nonfinite_and_boolean_numbers_rejected(self):
        for value in [None, float("nan"), float("inf"), True, -1]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                report.numeric({"rss": value}, "rss", positive=True)

    def test_raw_observations_must_agree_with_quantile(self):
        value = {"raw": [30, 10, 20]}
        correct = {"p50_ns": 20, "p95_ns": 30, "p99_ns": 30}
        self.assertEqual(report.observations(value, "raw", 3, correct), 60)
        with self.assertRaises(ValueError):
            report.observations(value, "raw", 3, {**correct, "p99_ns": 10})
        with self.assertRaises(ValueError):
            report.observations(value, "raw", 4, correct)

    def test_empty_campaign_does_not_pass(self):
        protocol = json.loads(Path(__file__).with_name("protocol.json").read_text())
        with self.assertRaises(ValueError):
            report.summarize([], protocol)

    def test_wasm_missing_cold_phase_does_not_become_zero(self):
        protocol = json.loads(Path(__file__).with_name("protocol.json").read_text())
        value = {"mode": "rust-wasm", "cold": {"engine_creation_ns": 1, "expected_wasm_modules": [1],
                 "artifacts": [{"module_id": 1, "compile_ns": None,
                                "instantiate_ns": 1, "metadata_ns": 1, "fault": None}]}}
        with self.assertRaises(ValueError):
            report.cold(value, protocol["resources"])

    def test_cold_declared_module_ids_must_be_positive_integer_identities(self):
        protocol = json.loads(Path(__file__).with_name("protocol.json").read_text())
        for module in [None, True, 0, -1, "1"]:
            value = {"mode": "rust-wasm", "cold": {"engine_creation_ns": 1,
                     "expected_wasm_modules": [module], "artifacts": [{"module_id": module,
                         "compile_ns": 1, "instantiate_ns": 1, "metadata_ns": 1, "fault": None}]}}
            with self.subTest(module=module), self.assertRaises(ValueError):
                report.cold(value, protocol["resources"])

    def test_missing_cold_artifact_rejected_even_if_remaining_artifact_passes(self):
        protocol = json.loads(Path(__file__).with_name("protocol.json").read_text())
        value = {"mode": "rust-wasm", "cold": {"engine_creation_ns": 1, "expected_wasm_modules": [1, 2],
                 "artifacts": [{"module_id": 1, "compile_ns": 1, "instantiate_ns": 1,
                                "metadata_ns": 1, "fault": None}]}}
        with self.assertRaises(ValueError):
            report.cold(value, protocol["resources"])


if __name__ == "__main__":
    unittest.main()
