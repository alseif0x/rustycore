"""Hermetic selector and fallback tests for full_policy.py."""

from contextlib import redirect_stdout
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from types import SimpleNamespace

import full_policy


class FullPolicyTests(unittest.TestCase):
    def test_world_rust_uses_the_fixed_policy_suite(self):
        checks, unsupported = full_policy.select_checks(
            ["crates/wow-world/src/session/identity.rs"], "base-sha", Path.cwd()
        )
        names = [name for name, _, _ in checks]
        self.assertEqual(unsupported, [])
        self.assertEqual(names[:6], [
            "Architecture, physical policy, and fixtures",
            "Ignored crate source inventory",
            "wow-world structural closure",
            "wow-world test census and ceiling",
            "WorldSession method ownership",
            "#1241 net-move policy",
        ])
        self.assertIn("Session ownership syntax and persistence inventory", names)
        self.assertIn("Snapshot-derived policy consistency", names)
        self.assertFalse(any("test_physical_files.py" in " ".join(command)
                             for _, command, _ in checks))
        self.assertTrue(all(command[0] == sys.executable for _, command, _ in checks
                            if Path(command[0]).name.startswith("python")))

    def test_workspace_and_world_manifests_keep_closure_and_census(self):
        for path in ("Cargo.lock", "Cargo.toml", "crates/wow-world-character/Cargo.toml"):
            with self.subTest(path=path):
                checks, _ = full_policy.select_checks([path], "base-sha", Path.cwd())
                names = {name for name, _, _ in checks}
                self.assertIn("wow-world structural closure", names)
                self.assertIn("wow-world test census and ceiling", names)
                self.assertIn("Architecture, physical policy, and fixtures", names)

    def test_handler_contract_check_tracks_registry_policy_and_module_inputs(self):
        relevant = (
            "crates/wow-world-inventory/src/handlers/auction.rs",
            "crates/wow-world-inventory/src/lib.rs",
            "crates/wow-world-inventory/Cargo.toml",
            "crates/world-server/src/handler_registry.rs",
            "crates/wow-core/src/unexpected_handler.rs",
            "tools/architecture/world-handler-contract.tsv",
            "tools/architecture/handler-module-policy.json",
            "tools/architecture/handler-contract-check/src/lib.rs",
        )
        for path in relevant:
            with self.subTest(path=path):
                checks, _ = full_policy.select_checks([path], "base-sha", Path.cwd())
                handler = [command for name, command, _ in checks
                           if name == "Handler registration and composition contract"]
                self.assertEqual(handler, [full_policy.checker("check", binary="handler-contract-check")])
                self.assertTrue(any(name == "Session ownership syntax and persistence inventory"
                                    for name, _, _ in checks))

        checks, _ = full_policy.select_checks(
            ["tools/architecture/session-ownership-policy.json"], "base-sha", Path.cwd()
        )
        self.assertFalse(any(name == "Handler registration and composition contract"
                             for name, _, _ in checks))
        self.assertTrue(any(name == "Session ownership syntax and persistence inventory"
                            for name, _, _ in checks))

    def test_owned_architecture_document_triggers_the_fixed_lane(self):
        checks, unsupported = full_policy.select_checks(
            [full_policy.ARCHITECTURE_DOC], "base-sha", Path.cwd()
        )
        self.assertEqual(unsupported, [])
        self.assertEqual(checks[0][0], "Architecture, physical policy, and fixtures")

    def test_changed_architecture_python_discovers_adjacent_owner_test(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            test_file = root / "tools/architecture/test_custom_check.py"
            test_file.parent.mkdir(parents=True)
            test_file.write_text("", encoding="utf-8")
            checks, _ = full_policy.select_checks(
                ["tools/architecture/custom_check.py"], "base-sha", root
            )
            owner_tests = [command for name, command, _ in checks if name.startswith("Python owner tests")]
            self.assertEqual(len(owner_tests), 1)
            self.assertEqual(owner_tests[0][0], sys.executable)
            self.assertEqual(owner_tests[0][-1], "test_custom_check.py")

    def test_physical_suite_is_not_duplicated_and_checker_self_tests_are_kept(self):
        checks, _ = full_policy.select_checks(
            ["tools/architecture/physical_files.py", "tools/architecture/net_move.py",
             "tools/architecture/session_method_index.py",
             "tools/architecture/net-move-policy.json"], "base-sha", Path.cwd()
        )
        commands = [command for _, command, _ in checks]
        self.assertTrue(any("--self-test" in command and "check_architecture.py" in " ".join(command)
                            for command in commands))
        self.assertFalse(any("test_physical_files.py" in command for command in commands))
        self.assertTrue(any("test_net_move.py" in command for command in commands))
        self.assertTrue(any("session_method_index.py" in " ".join(command) and "self-test" in command
                            for command in commands))

    def test_persistence_policy_edit_uses_exhaustive_checker(self):
        checks, _ = full_policy.select_checks(
            [full_policy.PERSISTENCE_POLICY], "base-sha", Path.cwd()
        )
        syntax = [kind for _, command, kind in checks
                  if "--syntax-only" in command]
        exhaustive = [kind for _, _, kind in checks if kind == "exhaustive"]
        self.assertEqual(syntax, [])
        self.assertEqual(exhaustive, ["exhaustive"])

    def test_policy_input_schemas_and_snapshot_reference_are_checked(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            values = {
                full_policy.SNAPSHOT: {"schema_version": 3, "accesses": []},
                full_policy.PERSISTENCE_POLICY: {"schema_version": 1, "groups": []},
                full_policy.WORKFLOWS: {"schema_version": 2, "workflows": []},
                full_policy.SESSION_POLICY: {
                    "schema_version": 1,
                    "persistence_access_snapshot": full_policy.SNAPSHOT,
                },
            }
            for name, value in values.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(json.dumps(value), encoding="utf-8")
            full_policy.check_policy_shapes(root)
            values[full_policy.SESSION_POLICY]["persistence_access_snapshot"] = "other.json"
            (root / full_policy.SESSION_POLICY).write_text(
                json.dumps(values[full_policy.SESSION_POLICY]), encoding="utf-8"
            )
            with self.assertRaisesRegex(ValueError, "must reference"):
                full_policy.check_policy_shapes(root)

    def test_snapshot_exit_two_escalates_and_skips_duplicate_syntax(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            policy = root / full_policy.PERSISTENCE_POLICY
            policy.parent.mkdir(parents=True)
            policy.write_text('{"schema_version":1,"groups":[]}', encoding="utf-8")
            checks, unsupported = full_policy.select_checks(
                [full_policy.SNAPSHOT], "base-sha", root
            )
            calls = []

            def run(command, **kwargs):
                calls.append(command)
                if command[0] == sys.executable and str(full_policy.SNAPSHOT_HELPER) in command:
                    return SimpleNamespace(returncode=2, stdout="", stderr="")
                if "print-persistence-policy" in command:
                    return SimpleNamespace(returncode=0, stdout=policy.read_text(), stderr="")
                return SimpleNamespace(returncode=0, stdout="", stderr="")

            with redirect_stdout(io.StringIO()):
                result = full_policy.run_checks(checks, unsupported, root, run=run)
            self.assertEqual(result, 0)
            self.assertTrue(any(command[0] == "cargo" and "check" in command
                                and "--syntax-only" not in command
                                for command in calls))
            self.assertFalse(any("--syntax-only" in command for command in calls))

    def test_snapshot_proof_failure_does_not_fall_back(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            policy = root / full_policy.PERSISTENCE_POLICY
            policy.parent.mkdir(parents=True)
            policy.write_text('{"schema_version":1,"groups":[]}', encoding="utf-8")
            checks, unsupported = full_policy.select_checks(
                [full_policy.SNAPSHOT], "base-sha", root
            )
            calls = []

            def run(command, **kwargs):
                calls.append(command)
                if command[0] == sys.executable and str(full_policy.SNAPSHOT_HELPER) in command:
                    return SimpleNamespace(returncode=1, stdout="", stderr="")
                return SimpleNamespace(returncode=0, stdout="", stderr="")

            with redirect_stdout(io.StringIO()):
                result = full_policy.run_checks(checks, unsupported, root, run=run)
            self.assertEqual(result, 1)
            self.assertFalse(any(command[0] == "cargo" for command in calls))

    def test_unsupported_owner_stays_blocked_after_green_checks(self):
        for prefix in full_policy.UNSUPPORTED:
            with self.subTest(prefix=prefix):
                path = f"{prefix}run.py"
                checks, unsupported = full_policy.select_checks([path], "base-sha", Path.cwd())

                def green(command, cwd, **kwargs):
                    output = ""
                    if "print-persistence-policy" in command:
                        output = (cwd / full_policy.PERSISTENCE_POLICY).read_text(encoding="utf-8")
                    return SimpleNamespace(returncode=0, stdout=output, stderr="")

                with redirect_stdout(io.StringIO()):
                    result = full_policy.run_checks(checks, unsupported, Path.cwd(), run=green)
                self.assertEqual(result, 2)
                self.assertEqual(unsupported, [path])


if __name__ == "__main__":
    unittest.main()
