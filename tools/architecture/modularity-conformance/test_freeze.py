"""Negative controls: extra helpers, moves, rewritten baselines and false third modules."""
import unittest

import freeze


class FrozenContractTests(unittest.TestCase):
    def setUp(self):
        self.before = {path: "abc" for path in freeze.REQUIRED | freeze.DECLARATIVE}
        self.current = dict(self.before)
        self.current.update({"modules/challenge/Cargo.toml": "def",
                             "modules/challenge/src/lib.rs": "def"})
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


if __name__ == "__main__":
    unittest.main()
