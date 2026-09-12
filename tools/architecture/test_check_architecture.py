"""Routing and failure contracts for combined architecture acceptance; no Cargo."""

from contextlib import ExitStack, redirect_stderr, redirect_stdout
import io
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import check_architecture as checker
import hotspot_metrics
import test_physical_files


class ArchitectureCommandTests(unittest.TestCase):
    def invoke(self, arguments, failure=None):
        policy = {"migration_issue": 584, "exceptions": []}
        ledger = {"issues": [{"number": 584}]}
        runtime = {"world_session_responsibility_families": {"families": []}}
        syntax = {
            "syntax_baseline": {
                "world_session": {"fields": []},
                "session_resources": {"fields": []},
                "session_command": {"variants": []},
            }
        }
        payloads = {
            checker.DEFAULT_PHYSICAL_POLICY: policy,
            checker.DEFAULT_ISSUE_LEDGER: ledger,
            checker.DEFAULT_RUNTIME_OWNERSHIP_LEDGER: runtime,
            checker.DEFAULT_SESSION_OWNERSHIP_POLICY: syntax,
        }
        calls = {}
        output = io.StringIO()
        with ExitStack() as stack:
            stack.enter_context(patch("sys.argv", ["check_architecture.py", *arguments]))
            stack.enter_context(redirect_stdout(output))
            stack.enter_context(redirect_stderr(output))
            stack.enter_context(patch.object(checker, "load_json", side_effect=lambda path: payloads.get(path, {})))
            stack.enter_context(patch.object(checker.physical_files, "inventory", return_value=[]))
            physical = stack.enter_context(patch.object(checker.physical_files, "evaluate", return_value={
                "files": [], "legacy_files": 0, "generated_files": 0,
                "review_required": 0, "mode": "migration",
            }))
            calls["physical"] = physical
            for name, value in {
                "validate_policy": {}, "validate_issue_ledger": ledger,
                "validate_handler_module_policy": {}, "validate_debt_ownership": None,
                "validate_runtime_ownership_ledger": runtime,
                "validate_hotspot_non_growth": 8, "validate_runtime_syntax_coverage": None,
                "validate_runtime_clock_phase_trace": 6, "validate_documented_sequence": None,
                "run_fixture_self_tests": None, "run_handler_module_policy_self_tests": 6,
                "run_debt_ownership_fixture_tests": 14, "run_runtime_ownership_self_tests": 9,
                "run_path_module_scanner_self_tests": 1, "run_hotspot_classifier_self_tests": 1,
                "run_hotspot_view_self_tests": 3, "run_hotspot_ratchet_self_tests": (4, 1),
                "cargo_metadata": {}, "check_dependencies": (38, 101, 15, 57, 2),
                "print_hotspots": None,
            }.items():
                calls[name] = stack.enter_context(patch.object(checker, name, return_value=value))
            calls["physical_fixtures"] = stack.enter_context(
                patch.object(test_physical_files, "run_self_tests", return_value=True)
            )
            if failure == "physical_fixtures":
                calls[failure].return_value = False
            elif failure:
                calls[failure].side_effect = checker.ArchitectureError("fixture rejection")
            result = checker.main()
        return result, calls, output.getvalue()

    def test_combined_runs_the_union_once(self):
        result, calls, output = self.invoke(["check", "--self-test"])
        self.assertEqual(result, 0, output)
        for name, call in calls.items():
            self.assertEqual(call.call_count, 1, name)
        self.assertIn("Architecture dependencies: PASS", output)
        self.assertIn("Architecture self-test: PASS", output)

    def test_original_commands_retain_their_coverage(self):
        for command in ("check", "self-test"):
            with self.subTest(command=command):
                result, calls, output = self.invoke([command])
                self.assertEqual(result, 0, output)
                self.assertEqual(calls["physical"].call_count, 1)
                self.assertEqual(calls["validate_hotspot_non_growth"].call_count, 1)
                self.assertEqual(calls["check_dependencies"].call_count, int(command == "check"))
                self.assertEqual(calls["print_hotspots"].call_count, int(command == "check"))
                self.assertEqual(calls["physical_fixtures"].call_count, int(command == "self-test"))

    def test_combined_fails_closed_in_each_phase(self):
        for failure in ("physical", "validate_hotspot_non_growth", "physical_fixtures",
                        "run_path_module_scanner_self_tests", "check_dependencies"):
            with self.subTest(failure=failure):
                result, calls, output = self.invoke(["check", "--self-test"], failure)
                self.assertEqual(result, 1, output)
                self.assertIn("architecture check failed", output)
                self.assertEqual(calls["print_hotspots"].call_count, 0)


class HotspotSnapshotTests(unittest.TestCase):
    def setUp(self):
        hotspot_metrics.physical_hotspot_row.cache_clear()
        self.addCleanup(hotspot_metrics.physical_hotspot_row.cache_clear)

    def test_repeated_views_reuse_one_measurement_per_file(self):
        root = hotspot_metrics.REPO_ROOT
        a, b = root / "a.rs", root / "b.rs"
        with patch.object(Path, "read_text", return_value="fn a() {}\n") as read:
            first = hotspot_metrics.physical_hotspot_row(a)
            self.assertEqual(first, (1, 1, 0, "a.rs"))
            self.assertEqual(hotspot_metrics.physical_hotspot_row(a), first)
            self.assertEqual(hotspot_metrics.physical_hotspot_row(b), (1, 1, 0, "b.rs"))
            self.assertEqual(read.call_count, 2)

    def test_read_failure_is_not_cached(self):
        path = hotspot_metrics.REPO_ROOT / "temporarily-missing.rs"
        with patch.object(Path, "read_text", side_effect=[OSError("missing"), "fn a() {}\n"]):
            with self.assertRaises(hotspot_metrics.ArchitectureError):
                hotspot_metrics.physical_hotspot_row(path)
            self.assertEqual(hotspot_metrics.physical_hotspot_row(path)[:3], (1, 1, 0))

    def test_new_invocation_observes_source_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "input.rs"
            program = (
                "import sys; from pathlib import Path; "
                "sys.path.insert(0, sys.argv[1]); import hotspot_metrics as h; "
                "h.REPO_ROOT=Path(sys.argv[2]); "
                "print(h.physical_hotspot_row(h.REPO_ROOT/'input.rs')[0])"
            )
            for lines in (1, 2):
                source.write_text("// fixture\n" * lines)
                output = subprocess.check_output([
                    sys.executable, "-c", program, str(Path(__file__).parent), directory,
                ], text=True)
                self.assertEqual(int(output.strip()), lines)


if __name__ == "__main__":
    unittest.main()
