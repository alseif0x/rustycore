"""Focused self-tests for the #1263 F4a P4a visibility codemod."""

import unittest

from f4_extract_core import (
    PUBLIC_MAP_MANAGER_METHODS,
    CodemodError,
    transform_map_manager,
    widen_declaration,
)


class WidenDeclarationTests(unittest.TestCase):
    def test_widens_only_the_named_field_and_is_idempotent(self):
        source = "pub(crate) used: Option<u8>,\npub(crate) private: Option<u8>,\n"

        widened, changed = widen_declaration(source, "field", "used")
        repeated, changed_again = widen_declaration(widened, "field", "used")

        self.assertTrue(changed)
        self.assertFalse(changed_again)
        self.assertEqual(repeated, "pub used: Option<u8>,\npub(crate) private: Option<u8>,\n")

    def test_rejects_missing_or_ambiguous_declarations(self):
        with self.assertRaises(CodemodError):
            widen_declaration("pub(crate) keep: u8,\n", "field", "missing")
        with self.assertRaises(CodemodError):
            widen_declaration(
                "pub(crate) duplicate: u8,\npub(crate) duplicate: u16,\n",
                "field",
                "duplicate",
            )

    def test_widens_const_method_declaration(self):
        source = "pub(crate) const fn runtime_elapsed_ms_like_cpp(&self) -> u64 { 0 }\n"

        widened, changed = widen_declaration(
            source, "method", "runtime_elapsed_ms_like_cpp"
        )

        self.assertTrue(changed)
        self.assertEqual(
            widened,
            "pub const fn runtime_elapsed_ms_like_cpp(&self) -> u64 { 0 }\n",
        )


class MapManagerStageTests(unittest.TestCase):
    def test_widens_only_the_per_file_allowlist_and_is_idempotent(self):
        filename = "runtime/creature.rs"
        declarations = PUBLIC_MAP_MANAGER_METHODS[filename]
        source = "\n".join(
            f"pub(crate) {'const ' if name == 'runtime_elapsed_ms_like_cpp' else ''}"
            f"fn {name}(&self) {{}}"
            for name in declarations
        )
        source += "\npub(crate) fn runtime_private_helper(&self) {}\n"

        widened, changed = transform_map_manager(filename, source)
        repeated, changed_again = transform_map_manager(filename, widened)

        self.assertEqual(len(changed), len(declarations))
        self.assertFalse(changed_again)
        self.assertEqual(repeated, widened)
        self.assertIn("pub const fn runtime_elapsed_ms_like_cpp", widened)
        self.assertIn("pub(crate) fn runtime_private_helper", widened)

    def test_rejects_paths_outside_the_reviewed_allowlist(self):
        with self.assertRaises(CodemodError):
            transform_map_manager("mod.rs", "")


if __name__ == "__main__":
    unittest.main()
