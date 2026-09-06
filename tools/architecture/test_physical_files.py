"""Adversarial tests for the physical branch of the existing architecture checker."""
import copy
import datetime as dt
import hashlib
import json
import pathlib
import subprocess
import tempfile
import unittest

import physical_files as physical


class PhysicalFilesTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="rustycore-physical-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name)
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        self.policy = {
            "schema_version": 1, "baseline_commit": "8f5caedc", "migration_issue": 578,
            "completed_checkpoints": [], "legacy": [], "exceptions": [], "generated": [],
        }
        self.today = dt.date(2026, 9, 5)

    def source(self, path="crates/a/src/lib.rs", lines=2500):
        destination = self.root / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text("// physical line, including comments\n" * lines, encoding="utf-8")
        return destination

    def legacy(self, path="crates/a/src/lib.rs", ceiling=2500):
        self.policy["legacy"].append({
            "path": path, "observed_lines": ceiling, "ceiling_lines": ceiling,
            "split": "separate the a aggregate's rules and scenario tests", "review_checkpoint": "578:C4",
        })

    def check(self, terminal=False):
        return physical.evaluate(self.root, self.policy, physical.inventory(self.root),
                                 terminal=terminal, today=self.today)

    def test_production_integration_tools_and_extensionless_sources_are_counted(self):
        for path in ("crates/a/src/lib.rs", "crates/a/tests/lifetime.rs", "tools/bot/test.py", "tools/guest/src/guest.c"):
            self.source(path, 20)
        self.source("tools/runner", 0).write_text("#!/usr/bin/env python3\n\n# a comment\n", encoding="utf-8")
        self.source("docs/report.md", 9000)
        result = self.check()
        self.assertEqual(len(result["files"]), 5)
        self.assertEqual(next(row["lines"] for row in result["files"] if row["path"] == "tools/runner"), 3)

    def test_new_source_or_oversized_test_is_not_grandfathered(self):
        for path in ("crates/a/src/new.rs", "crates/a/tests/new.rs", "tools/new.py", "tools/new.c"):
            with self.subTest(path=path):
                candidate = self.source(path, 2001)
                with self.assertRaisesRegex(physical.PhysicalFileError, "ceiling 2000"):
                    self.check()
                candidate.unlink()

    def test_legacy_growth_fails_but_reduction_and_tightening_pass(self):
        self.source()
        self.legacy()
        self.check()
        self.source(lines=2501)
        with self.assertRaisesRegex(physical.PhysicalFileError, "ceiling 2500"):
            self.check()
        self.source(lines=2300)
        self.check()
        self.policy["legacy"][0]["ceiling_lines"] = 2300
        self.check()
        self.source(lines=2301)
        with self.assertRaisesRegex(physical.PhysicalFileError, "ceiling 2300"):
            self.check()

    def test_renaming_or_moving_does_not_transfer_a_legacy_ceiling(self):
        for target in ("crates/a/src/new.rs", "tools/moved.rs", "tools/moved.txt"):
            with self.subTest(target=target):
                source = self.source()
                self.policy["legacy"] = []
                self.legacy()
                destination = self.root / target
                destination.parent.mkdir(parents=True, exist_ok=True)
                source.rename(destination)
                with self.assertRaisesRegex(physical.PhysicalFileError, "audited physical path missing"):
                    self.check()
                destination.unlink()

    def test_terminal_is_not_a_migration_pass(self):
        self.source()
        self.legacy()
        self.check()
        with self.assertRaisesRegex(physical.PhysicalFileError, "unfinished physical migration"):
            self.check(terminal=True)
        self.source(lines=800)
        self.check(terminal=True)

    def test_completed_checkpoint_expires_a_legacy_entry(self):
        self.source()
        self.legacy()
        self.policy["completed_checkpoints"] = ["578:C4"]
        with self.assertRaisesRegex(physical.PhysicalFileError, "expired physical review checkpoint"):
            self.check()

    def exception(self):
        self.source()
        self.policy["exceptions"] = [{
            "path": "crates/a/src/lib.rs", "observed_lines": 2500, "ceiling_lines": 2500,
            "responsibility": "pinned upstream numerical kernel", "reason": "retain upstream patch alignment",
            "tracking_issue": 578, "review_checkpoint": "next-upstream-update",
            "reviewed_at": "2026-09-05", "expires_on": "2026-10-05",
        }]

    def test_concrete_bounded_exception_can_pass_terminal(self):
        self.exception()
        self.check(terminal=True)

    def test_expired_or_future_review_cannot_pass(self):
        self.exception()
        for field, value in (("expires_on", "2026-09-05"), ("reviewed_at", "2026-09-06"), ("expires_on", "bad")):
            with self.subTest(field=field, value=value):
                original = copy.deepcopy(self.policy)
                self.policy["exceptions"][0][field] = value
                with self.assertRaises(physical.PhysicalFileError):
                    self.check(terminal=True)
                self.policy = original

    def test_duplicate_policy_path_cannot_hide_growth(self):
        self.source()
        self.legacy()
        self.legacy(ceiling=5000)
        with self.assertRaisesRegex(physical.PhysicalFileError, "duplicate/overlapping"):
            self.check()

    def test_blank_split_or_boolean_ceiling_is_rejected(self):
        self.source()
        self.legacy()
        for field, value in (("split", ""), ("ceiling_lines", True), ("ceiling_lines", 2501)):
            original = copy.deepcopy(self.policy)
            self.policy["legacy"][0][field] = value
            with self.assertRaises(physical.PhysicalFileError):
                self.check()
            self.policy = original

    def test_tracked_source_cannot_hide_behind_new_ignore_rule(self):
        self.source()
        subprocess.run(["git", "-C", str(self.root), "add", "crates/a/src/lib.rs"], check=True)
        (self.root / ".gitignore").write_text("crates/\n", encoding="utf-8")
        with self.assertRaisesRegex(physical.PhysicalFileError, "ceiling 2000"):
            self.check()

    def test_ignored_build_products_are_not_repository_source(self):
        (self.root / ".gitignore").write_text("target/\n", encoding="utf-8")
        self.source("target/out/generated.rs", 9000)
        self.assertEqual(self.check()["files"], [])

    def test_symlink_cannot_bypass_source_identity(self):
        self.source("tools/real.txt", 3000)
        (self.root / "tools/link.rs").symlink_to("real.txt")
        with self.assertRaisesRegex(physical.PhysicalFileError, "symlink"):
            self.check()

    def test_non_source_symlink_is_not_a_new_code_approval_gate(self):
        self.source("docs/real.txt", 3000)
        (self.root / "docs/readme.md").symlink_to("real.txt")
        self.assertEqual(self.check()["files"], [])

    def test_utf8_error_is_not_silently_dropped(self):
        self.source(lines=0).write_bytes(b"\xff")
        with self.assertRaisesRegex(physical.PhysicalFileError, "cannot count source"):
            self.check()

    def generated(self):
        def record(path):
            return {"path": path, "sha256": hashlib.sha256((self.root / path).read_bytes()).hexdigest()}
        self.source("crates/a/src/generated.rs", 3000)
        self.source("tools/generate.py", 5)
        self.source("data/input.txt", 2)
        self.source("docs/reproduction.json", 2)
        self.policy["generated"] = [{
            "path": "crates/a/src/generated.rs",
            "output_sha256": record("crates/a/src/generated.rs")["sha256"],
            "generator": record("tools/generate.py"), "inputs": [record("data/input.txt")],
            "reproduction_command": "python3 tools/generate.py data/input.txt",
            "evidence": record("docs/reproduction.json"),
        }]
        entry = self.policy["generated"][0]
        (self.root / "docs/reproduction.json").write_text(json.dumps({
            "schema_version": 1, "command": entry["reproduction_command"],
            "generator": entry["generator"], "inputs": entry["inputs"],
            "output": {"path": entry["path"], "sha256": entry["output_sha256"]},
            "reproduced_sha256": entry["output_sha256"],
        }), encoding="utf-8")
        entry["evidence"] = record("docs/reproduction.json")

    def test_generated_name_alone_is_not_provenance(self):
        self.source("crates/a/src/generated.rs", 3000)
        with self.assertRaisesRegex(physical.PhysicalFileError, "ceiling 2000"):
            self.check()

    def test_reviewed_generated_provenance_is_reported_separately(self):
        self.generated()
        result = self.check(terminal=True)
        self.assertEqual(result["generated_files"], 1)
        self.assertEqual(result["files"][0]["kind"], "generated")

    def test_generated_output_generator_input_and_evidence_drift_fail(self):
        for path in ("crates/a/src/generated.rs", "tools/generate.py", "data/input.txt", "docs/reproduction.json"):
            with self.subTest(path=path):
                self.generated()
                with (self.root / path).open("a", encoding="utf-8") as output:
                    output.write("changed\n")
                with self.assertRaisesRegex(physical.PhysicalFileError, "hash drift"):
                    self.check()

    def test_generated_output_cannot_certify_itself(self):
        self.generated()
        entry = self.policy["generated"][0]
        entry["generator"] = {"path": entry["path"], "sha256": entry["output_sha256"]}
        with self.assertRaisesRegex(physical.PhysicalFileError, "certify itself"):
            self.check()

    def test_rehashed_but_unrelated_reproduction_evidence_is_rejected(self):
        self.generated()
        evidence = self.root / "docs/reproduction.json"
        evidence.write_text('{}', encoding="utf-8")
        self.policy["generated"][0]["evidence"]["sha256"] = hashlib.sha256(evidence.read_bytes()).hexdigest()
        with self.assertRaisesRegex(physical.PhysicalFileError, "reproduction record"):
            self.check()


def run_self_tests():
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(PhysicalFilesTests)
    return unittest.TextTestRunner(verbosity=0).run(suite).wasSuccessful()


if __name__ == "__main__":
    unittest.main()
