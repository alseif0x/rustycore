"""Negative controls: extra helpers, moves, rewritten baselines and false third modules."""
import unittest
from pathlib import Path
import tempfile

import freeze


class FrozenContractTests(unittest.TestCase):
    def setUp(self):
        self.before = {path: "abc" for path in freeze.REQUIRED | freeze.DECLARATIVE}
        self.current = dict(self.before)
        self.current.update({"modules/challenge/Cargo.toml": "def",
                             "modules/challenge/src/lib.rs": "def",
                             "c-guests/challenge.c": "def", "driver/tests/challenge.rs": "def"})
        self.frozen = {"schema_version": 1, "kind": "two-module-source-freeze",
                       "files": self.before}

    def check(self):
        return freeze.compare(self.frozen, self.current, "challenge")

    def test_new_independent_module_and_declarative_registration_allowed(self):
        self.current["driver/src/composition.rs"] = "new registration"
        result = self.check()
        self.assertTrue(result["source_freeze_pass"])
        self.assertEqual(result["changed_declarative_paths"], ["driver/src/composition.rs"])

    def test_changed_host_rejected(self):
        self.current["host/src/lib.rs"] = "module-specific central arm"
        self.assertFalse(self.check()["source_freeze_pass"])

    def test_new_helper_cannot_escape_hash_set(self):
        self.current["host/src/hidden.inc"] = "module-specific callback"
        self.assertFalse(self.check()["source_freeze_pass"])

    def test_renamed_original_module_rejected(self):
        self.current["modules/challenge/src/copied.rs"] = self.current.pop("modules/policy/src/lib.rs")
        self.assertFalse(self.check()["source_freeze_pass"])

    def test_missing_third_crate_is_not_a_pass(self):
        del self.current["modules/challenge/Cargo.toml"]
        self.assertFalse(self.check()["source_freeze_pass"])

    def test_missing_c_frontend_or_integration_target_is_not_a_pass(self):
        for path in ["c-guests/challenge.c", "driver/tests/challenge.rs"]:
            with self.subTest(path=path):
                saved = self.current.pop(path)
                self.assertFalse(self.check()["source_freeze_pass"])
                self.current[path] = saved

    def test_existing_third_module_is_not_independent_challenge(self):
        self.before["modules/challenge/src/lib.rs"] = "already anticipated"
        with self.assertRaises(ValueError):
            self.check()

    def test_incomplete_frozen_adapter_set_rejected(self):
        del self.before["contract/src/guest.rs"]
        with self.assertRaises(ValueError):
            self.check()

    def test_driver_logic_and_protocol_are_frozen(self):
        for path in ["driver/src/main.rs", "protocol.json", "freeze.py"]:
            with self.subTest(path=path):
                self.current[path] = "silently relaxed after results"
                self.assertFalse(self.check()["source_freeze_pass"])
                self.current[path] = self.before[path]

    def test_traversal_or_initial_module_name_is_rejected(self):
        for name in ["../host", "encounter", "policy", "", "challenge/else"]:
            with self.subTest(name=name), self.assertRaises(ValueError):
                freeze.compare(self.frozen, self.current, name)


class SourceTraversalTests(unittest.TestCase):
    def test_only_known_root_outputs_are_excluded(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "target").mkdir()
            (root / "target/binary").write_bytes(b"generated")
            (root / "source.rs").write_text("source")
            self.assertEqual(set(freeze.file_set(root)), {"source.rs"})
            nested = root / "modules/challenge/src/target"
            nested.mkdir(parents=True)
            (nested / "logic.rs").write_text("hidden implementation")
            with self.assertRaises(ValueError):
                freeze.file_set(root)

    def test_source_and_ignored_output_symlinks_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "target").symlink_to("/tmp", target_is_directory=True)
            with self.assertRaises(ValueError):
                freeze.file_set(root)

    def test_current_cargo_paths_stay_inside_lab_and_modules_depend_only_on_contract(self):
        self.assertIn("Cargo.toml", freeze.validate_cargo(freeze.ROOT))

    def fixture(self, root, dependency):
        for folder in ["contract", "host", "driver", "modules/challenge"]:
            (root / folder).mkdir(parents=True)
        (root / "Cargo.toml").write_text(
            '[workspace]\nmembers=["contract","host","driver","modules/challenge"]\n'
            '[workspace.dependencies]\nconformance-contract={path="contract"}\n')
        (root / "modules/challenge/Cargo.toml").write_text(
            '[package]\nname="challenge"\n[dependencies]\n' + dependency)

    def test_external_or_noncontract_dependency_is_rejected(self):
        for dependency in ['conformance-contract={path="../../../outside"}\n',
                           'conformance-host={path="../../host"}\n']:
            with self.subTest(dependency=dependency), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                self.fixture(root, dependency)
                with self.assertRaises(ValueError):
                    freeze.validate_cargo(root)

    def test_inherited_contract_path_is_admitted_but_module_build_script_is_not(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root, "conformance-contract.workspace=true\n")
            freeze.validate_cargo(root)
            (root / "modules/challenge/build.rs").write_text("fn main() {}")
            with self.assertRaises(ValueError):
                freeze.validate_cargo(root)


if __name__ == "__main__":
    unittest.main()
