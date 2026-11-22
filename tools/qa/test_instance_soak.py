"""Offline tests for multi-client scheduling and process identity."""

from contextlib import nullcontext
import json
import os
from pathlib import Path
import signal
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace

import death_scenario
import instance_scenario
import instance_soak


class SoakTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)

    def proc(self, starttime):
        process = self.root / "42"
        process.mkdir(exist_ok=True)
        fields = ["S", *(["0"] * 18), str(starttime)]
        (process / "stat").write_text("42 (world ) server) " + " ".join(fields))
        (process / "status").write_text("VmRSS:\t256 kB\nVmHWM:\t512 kB\n")

    def test_samples_only_the_guarded_process_and_records_linux_units(self):
        self.proc(2001)
        self.assertEqual(instance_soak.read_memory(42, 2001, self.root),
                         {"rss_kib": 256, "peak_rss_kib": 512})
        with self.assertRaises(death_scenario.ScenarioFailure):
            instance_soak.read_memory(42, 2000, self.root)
        (self.root / "42/status").write_text("VmRSS: 2 MB\nVmHWM: 3 MB\n")
        with self.assertRaises(death_scenario.ScenarioFailure):
            instance_soak.read_memory(42, 2001, self.root)

    def test_sampler_preserves_first_and_last_sample(self):
        with patch.object(instance_soak, "read_memory", return_value={"rss_kib": 19, "peak_rss_kib": 25}):
            with instance_soak.memory_samples(self.root, 42, 2001) as check:
                check()
        rows = [json.loads(row) for row in (self.root / "memory.jsonl").read_text().splitlines()]
        self.assertGreaterEqual(len(rows), 2)
        self.assertEqual([19] * len(rows), [row["rss_kib"] for row in rows])

    def test_queue_sample_ignores_old_journal_entries_and_preserves_distinct_outcomes(self):
        def entry(stamp):
            message = ("Instance soak command queue totals \x1b[32mcommand_full\x1b[0m=2 "
                       "command_disconnected=1 command_stale=4 retry_full=0")
            return SimpleNamespace(returncode=0, stdout=json.dumps({
                "__MONOTONIC_TIMESTAMP": str(stamp), "MESSAGE": message}))

        with (patch.object(instance_soak, "read_memory"),
              patch.object(instance_soak.time, "monotonic_ns", return_value=500_000),
              patch.object(instance_soak.time, "sleep"),
              patch.object(instance_soak.subprocess, "run", side_effect=[entry(400), entry(510)]) as query):
            sample = instance_soak.queue_sample(42, 2001)
        self.assertEqual(sample, {"command_full": 2, "command_disconnected": 1, "command_stale": 4,
                                  "retry_full": 0, "monotonic_us": 510})
        self.assertIn("_PID=42", query.call_args.args[0])

    def run_group_cycles(self, group_duration, fail_on=0, reset_counter=False):
        elapsed = 0
        count = 0

        def run_case(identity, journal, realm_id, db):
            nonlocal elapsed, count
            if identity["scenario"] == "instance-solo-reset":
                elapsed += 1000
            else:
                count += 1
                elapsed += group_duration
            status = "failed" if fail_on and count == fail_on else "passed"
            return {"scenario": identity["scenario"], "scenario_observation": status}

        baseline = dict.fromkeys(instance_soak.QUEUE_FIELDS, 1)
        end = {**baseline, "command_full": 0 if reset_counter else 3}
        with (patch.dict(os.environ, RUSTYCORE_CAPTURE_PID="42", RUSTYCORE_CAPTURE_STARTTIME="2001"),
              patch.object(instance_soak, "queue_sample", side_effect=[baseline, end]),
              patch.object(instance_soak, "memory_samples", return_value=nullcontext(lambda: None)),
              patch.object(instance_soak.time, "monotonic", side_effect=lambda: elapsed)):
            return instance_soak.run_workload({"capture": self.root}, {}, 1, None, run_case)

    def test_two_hours_exclude_setup_and_finish_only_after_a_whole_group_cycle(self):
        result = self.run_group_cycles(group_duration=700)
        self.assertEqual(result["group_cycles"], 11)
        self.assertEqual(result["group_elapsed_s"], 7700)
        self.assertEqual(result["elapsed_s"], 8700)
        self.assertEqual(result["command_queue_deltas"]["command_full"], 2)
        for cycle in result["workload"]:
            report = self.root / cycle["directory"] / "scenario-report.json"
            self.assertEqual(cycle["report_sha256"], death_scenario._sha256(report))

    def test_counter_reset_prevents_a_success_report(self):
        with self.assertRaisesRegex(death_scenario.ScenarioFailure, "backwards"):
            self.run_group_cycles(group_duration=3600, reset_counter=True)

    def test_failed_second_cycle_keeps_first_report_and_restores_signal_handler(self):
        handler = signal.getsignal(signal.SIGTERM)
        with self.assertRaises(death_scenario.ScenarioFailure):
            self.run_group_cycles(group_duration=8000, fail_on=2)
        self.assertIs(signal.getsignal(signal.SIGTERM), handler)
        self.assertTrue((self.root / "group-001/scenario-report.json").is_file())
        self.assertFalse((self.root / "group-002/scenario-report.json").exists())
        failure = json.loads((self.root / "soak-failure.json").read_text())
        self.assertEqual(failure["stage"], "group-002")
        self.assertEqual(failure["completed_cases"], 2)

    def test_guarded_entry_dispatches_to_soak(self):
        with patch.object(instance_soak, "run_workload", return_value={}) as soak:
            identity = {"scenario": "instance-stability-soak"}
            instance_scenario._run_case(identity, {}, 1, None)
            soak.assert_called_once_with(identity, {}, 1, None, instance_scenario._run_case)


if __name__ == "__main__":
    unittest.main()
