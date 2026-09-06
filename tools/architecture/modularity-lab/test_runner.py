import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location("lab_runner", Path(__file__).with_name("run.py"))
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class RunnerChecks(unittest.TestCase):
    def test_checks_cannot_be_empty_or_silently_skipped(self):
        for value in [{"ok": True, "checks": []}, {"success": True, "checks": [{"passed": False}]},
                      {"ok": True, "checks": [{"passed": 1}]}]:
            with self.assertRaises(ValueError):
                runner.check_result(value, "execution")

    def test_deleting_a_required_case_fails_even_if_returned_checks_pass(self):
        checks = [{"name": name, "passed": True} for name in runner.EXECUTION_CHECKS]
        runner.check_result({"success": True, "checks": checks}, "execution")
        with self.assertRaises(ValueError):
            runner.check_result({"success": True, "checks": checks[:-1]}, "execution")

    def test_shared_backend_workload_bug_fails_independent_oracle(self):
        sample = {"entities": 1000, "ticks": 200, "density": "sparse", "final_entities": 1000,
                  "final_optional": 250, "operations": {"updates": 200000, "churn": 1300, "transfers": 700}}
        runner.check_storage_population(sample)
        sample["final_optional"] = 0
        with self.assertRaises(ValueError):
            runner.check_storage_population(sample)

    def test_numeric_rejects_missing_bool_nan_and_negative(self):
        for value in [None, True, float("nan"), -1]:
            with self.assertRaises(ValueError):
                runner.numeric({"rss_kib": value}, "rss_kib")

    def test_execution_requires_observables_and_complete_work(self):
        sample = {"calls": 1000, "warmup_calls": 256, "final_observables": {"money": 1},
                  "calls_by_event": {"xp": 750, "summon_success": 62, "summon_failure": 63,
                                     "reset": 63, "reward": 62}}
        runner.check_execution_work(sample)
        sample["calls_by_event"]["xp"] = 749
        with self.assertRaises(ValueError):
            runner.check_execution_work(sample)
        sample["calls_by_event"]["xp"] = 750
        sample["final_observables"] = {}
        with self.assertRaises(ValueError):
            runner.check_execution_work(sample)

    def test_semantic_disagreement_is_not_a_performance_win(self):
        with self.assertRaises(ValueError):
            runner.same_work({"aggregate": {"checksum": "1"}, "hecs": {"checksum": "2"}}, ["checksum"])

    def test_spread_keeps_variation_instead_of_best_run(self):
        self.assertEqual(runner.spread([1, 10, 3]), {"min": 1, "median": 3, "max": 10})


if __name__ == "__main__":
    unittest.main()
