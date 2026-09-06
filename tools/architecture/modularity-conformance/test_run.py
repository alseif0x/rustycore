"""Synthetic supervisor oracles only; these tests never run a laboratory benchmark."""
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

import freeze
import report
import run


def unit_record(names):
    names = sorted(names)
    return {"returncode": 0, "stdout": "\n".join(f"test {name} ... ok" for name in names)
            + f"\ntest result: ok. {len(names)} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n"}


def functional_fixture(ids=(1, 2)):
    values = [[module, module * 10] for module in ids]
    trace = []
    for module, result in values:
        trace += [{"kind": "enter", "module": module, "event": 7, "argument": 31},
                  {"kind": "leave", "module": module, "event": 7, "argument": 31,
                   "result": {"ok": result}}]
    registration = {"values": values, "oracle": {"states": [{"module": module} for module in ids],
                                                  "trace": trace}}
    results = {}
    for mode in report.MODES:
        cases = report.COMMON | (report.NATIVE if mode == "native" else set())
        results[mode] = {"schema_version": 2, "kind": "mode-contract-checks", "mode": mode,
                         "passed": True, "checks": [{"name": name, "passed": True,
                         "detail": copy.deepcopy(registration) if name == "registration_order" else {}}
                         for name in sorted(cases)]}
    return results


class SupervisorTests(unittest.TestCase):
    def test_required_native_return_gate_is_present(self):
        self.assertIn("wasm::tests::registration::wasm_runtime_native_returns_share_portable_fault_semantics_and_leave_trace",
                      run.WASM_GATES)

    def test_required_test_set_rejects_missing_duplicate_and_ignored_tests(self):
        valid = unit_record(run.WASM_GATES | {"tests::core"})
        self.assertEqual(run.test_gate(valid, run.WASM_GATES, wasm=True), sorted(run.WASM_GATES))
        for record in [unit_record(run.WASM_GATES - {next(iter(run.WASM_GATES))}),
                       {**valid, "stdout": valid["stdout"].replace("... ok", "... ignored", 1)},
                       {**valid, "stdout": valid["stdout"] + "test tests::core ... ok\n"},
                       {**valid, "returncode": 1}]:
            with self.subTest(record=record), self.assertRaises(ValueError):
                run.test_gate(record, run.WASM_GATES, wasm=True)

    def test_third_challenge_requires_exact_four_lifecycle_tests(self):
        run.test_gate(unit_record(report.CHALLENGE_TESTS), report.CHALLENGE_TESTS)
        for names in [{"trivial"}, report.CHALLENGE_TESTS - {next(iter(report.CHALLENGE_TESTS))},
                      report.CHALLENGE_TESTS | {"unreviewed_extra"}]:
            with self.subTest(names=names), self.assertRaises(ValueError):
                run.test_gate(unit_record(names), report.CHALLENGE_TESTS)

    def test_third_crate_must_be_registered_and_really_executed(self):
        self.assertEqual(report.registration_ids(functional_fixture((1, 2, 7)), 3), [1, 2, 7])
        with self.assertRaises(ValueError):
            report.registration_ids(functional_fixture(), 3)
        missing_execution = functional_fixture((1, 2, 7))
        for value in missing_execution.values():
            case = next(c for c in value["checks"] if c["name"] == "registration_order")
            case["detail"]["oracle"]["trace"] = case["detail"]["oracle"]["trace"][:-2]
        with self.assertRaises(ValueError):
            report.registration_ids(missing_execution, 3)

    def test_decision_needs_actual_review_and_measurement_success(self):
        passing = {"passed": True}
        self.assertTrue(run.decision_eligible(True, [], passing, passing))
        self.assertFalse(run.decision_eligible(True, [], {}, passing))
        self.assertFalse(run.decision_eligible(True, [], passing, {}))
        self.assertFalse(run.decision_eligible(False, [], passing, passing))
        self.assertFalse(run.decision_eligible(True, ["failed sample"], passing, passing))

    def test_semantic_review_is_tied_to_exact_freeze_source_module_and_deltas(self):
        value = {"schema_version": 1, "kind": "independent-module-semantic-review", "verdict": "pass",
                 "freeze_sha256": "a" * 64, "source_files": {"source": "b" * 64},
                 "module": "challenge", "module_id": 7, "changed_declarative_paths": ["Cargo.toml"],
                 "reviewer": "independent reviewer", "rationale": "Reviewed the real source and four lifecycles",
                 "checks": {name: True for name in run.REVIEW_CHECKS}}
        arguments = ("a" * 64, value["source_files"], "challenge", 7, ["Cargo.toml"])
        self.assertTrue(run.semantic_review(value, *arguments)["passed"])
        for key, replacement in [("freeze_sha256", "c" * 64), ("source_files", {}),
                                 ("module_id", 9), ("changed_declarative_paths", []),
                                 ("reviewer", ""), ("checks", {name: 1 for name in run.REVIEW_CHECKS})]:
            with self.subTest(key=key), self.assertRaises(ValueError):
                run.semantic_review({**value, key: replacement}, *arguments)

    def test_output_parent_is_checked_before_work_and_gzip_can_be_read(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            freeze.output_preflight(root / "new.json")
            with self.assertRaises(ValueError):
                freeze.output_preflight(root / "missing/report.json")
            path = root / "report.json.gz"
            run.write_new(path, {"retained": "output"})
            self.assertEqual(freeze.read_record(path), {"retained": "output"})
            with self.assertRaises(ValueError):
                freeze.output_preflight(path)

    def test_artifact_record_cannot_point_to_an_unloaded_copy_or_accept_replacement(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            actual, unrelated = root / "driver", root / "copy"
            actual.write_bytes(b"binary")
            unrelated.write_bytes(b"binary")
            digest = hashlib.sha256(b"binary").hexdigest()
            value = {"artifacts": {"driver": {"path": str(actual), "sha256": digest}}}
            with patch.object(run.builder, "expected_artifacts", return_value={"driver": actual}):
                run.verify_artifacts(value)
                with self.assertRaises(ValueError):
                    run.verify_artifacts({"artifacts": {"driver": {"path": str(unrelated), "sha256": digest}}})
                actual.write_bytes(b"rebuilt")
                with self.assertRaises(ValueError):
                    run.verify_artifacts(value)

    def test_failed_and_malformed_commands_retain_original_output(self):
        failed = run.command([sys.executable, "-c", "print('retained failure'); raise SystemExit(3)"], 5)
        self.assertEqual(failed["returncode"], 3)
        self.assertIn("retained failure", failed["stdout"])
        malformed = run.command([sys.executable, "-c", "print('not JSON')"], 5)
        self.assertIn("malformed JSON", malformed["error"])
        self.assertEqual(malformed["stdout"], "not JSON\n")

    def test_timeout_kills_only_its_process_group_and_retains_partial_output(self):
        process = Mock(pid=4567)
        process.communicate.side_effect = [subprocess.TimeoutExpired("fixture", 1), ("partial out", "partial err")]
        with patch.object(run.subprocess, "Popen", return_value=process) as spawn, patch.object(run.os, "killpg") as kill:
            value = run.command(["fixture"], 1)
        self.assertTrue(spawn.call_args.kwargs["start_new_session"])
        kill.assert_called_once_with(4567, run.signal.SIGKILL)
        self.assertEqual(value["error"], "timeout")
        self.assertEqual(value["stdout"], "partial out")
        self.assertEqual(value["stderr"], "partial err")
        self.assertIn("elapsed_seconds", value)


class PrefreezeEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.files = {name: "a" * 64 for name in freeze.REQUIRED}
        protocol = freeze.read_record(freeze.ROOT / "protocol.json")
        functional = functional_fixture()
        unit = {**unit_record(run.WASM_GATES), "role": "host-unit"}
        commands = [unit] + [{"role": f"functional:{mode}", "returncode": 0,
                              "value": copy.deepcopy(functional[mode])} for mode in report.MODES]
        self.value = {"schema_version": 2, "kind": "two-module-conformance", "passed": True,
                      "decision_eligible": False, "errors": [], "samples": [], "source_files": self.files,
                      "build": {"kind": "source-built-conformance-artifacts", "schema_version": 2,
                                "source_files": self.files, "artifacts": {}},
                      "protocol": protocol, "protocol_sha256": self.files["protocol.json"],
                      "commands": commands, "functional": functional,
                      "functional_verdict": report.functional(functional, report.MODES)}

    def test_complete_structural_prefreeze_evidence_passes(self):
        with patch.object(run, "verify_artifacts"):
            self.assertEqual(run.validate_prefreeze_report(self.value, self.files), [1, 2])

    def test_green_summary_alone_is_not_a_freeze(self):
        with self.assertRaises(ValueError):
            run.validate_prefreeze_report({"kind": "two-module-conformance", "passed": True,
                                           "source_files": self.files}, self.files)

    def test_missing_command_or_failed_host_cannot_be_hidden_by_summary(self):
        for mutation in ["missing", "failed"]:
            value = copy.deepcopy(self.value)
            if mutation == "missing":
                value["commands"].pop()
            else:
                value["commands"][0]["returncode"] = 1
            with self.subTest(mutation=mutation), patch.object(run, "verify_artifacts"), self.assertRaises(ValueError):
                run.validate_prefreeze_report(value, self.files)


if __name__ == "__main__":
    unittest.main()
