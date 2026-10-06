"""Focused fixtures for the pure, hash-checked hub impl extraction."""

import copy
import unittest
from pathlib import Path

import f4_hub_extract as codemod
import f4_test_fixtures_gate as item_tools


SOURCE_FILE = "crates/wow-world/src/session/mixed.rs"


def sample_source():
    return r'''// impl crate::session::HubRef<'x> { fn fake() {} }
const TEXT: &str = r#" impl crate::session::HubMut<'x> { fn fake() { } } "#;

#[cfg(feature = "previous-item")]
/// Documentation for the preceding item must stay behind.
struct PreviousItem;

/// HubRef documentation stays attached to its impl.
#[cfg(
    any(
        test,
        feature = "test-fixtures"
    )
)]
/// Documentation interleaved between the cfg and allow attributes.
#[allow(dead_code)]
impl<'a> crate::session::HubRef<'a> {
    fn selected(&self) {
        let braces = r#" } impl WorldSession { { "#;
        /* nested-looking braces are comments: { } */
    }
}

/// Trait implementation docs stay attached too.
impl Default for crate::session::HubMut<'_> {
    fn default() -> Self { todo!() }
}

impl crate::session::WorldSession {
    fn shell_only(&self) { let brace = "}"; }
}

impl crate::session::CharacterCx {
    fn cx_only(&self) { /* } */ }
}

impl crate::session::Unrelated {
    fn unrelated(&self) {}
}
'''


class HubExtractTests(unittest.TestCase):
    def setUp(self):
        self.lexer = item_tools.lib(codemod.REPO)

    def records(self, raw=None):
        raw = sample_source() if raw is None else raw
        code = self.lexer.blank_noncode(raw)
        return codemod._block_records(
            Path(SOURCE_FILE), raw, code, "crate::session::mixed_hint",
            self.lexer, {"HubMut", "HubRef"},
        )

    def test_plan_records_mirrored_destination_without_promoting_hint(self):
        records = self.records()
        self.assertTrue(records)
        for record in records:
            self.assertEqual(
                record["destination_path"],
                "crates/wow-world-core/src/session/mixed.rs",
            )
            self.assertEqual(record["module_path_hint"], "crate::session::mixed_hint")
            self.assertTrue(record["source_sha256"])

    def test_multiline_cfg_and_doc_comments_are_inside_exact_moved_span(self):
        raw = sample_source()
        record = next(r for r in self.records(raw) if r["owner"]["name"] == "HubRef")

        self.assertEqual(len(record["cfg_prefix"]), 1)
        self.assertIn("any(", record["cfg_prefix"][0])
        self.assertIn('feature = "test-fixtures"', record["cfg_prefix"][0])
        self.assertTrue(raw[record["span"]["char_start"]:].startswith(
            "/// HubRef documentation stays attached to its impl.\n#[cfg("
        ))
        retained, moved = codemod.extract_recorded_blocks(raw, [record], self.lexer)
        self.assertEqual(moved[0].source, raw[record["span"]["char_start"]:record["span"]["char_end"]])
        self.assertNotIn("Documentation for the preceding item", moved[0].source)
        self.assertIn("#[allow(dead_code)]", moved[0].source)
        self.assertIn("Documentation interleaved between the cfg and allow", moved[0].source)
        self.assertNotIn("HubRef documentation stays attached", retained)

    def test_qualified_lifetime_and_trait_default_are_censused_exactly(self):
        records = self.records()
        inherent = next(r for r in records if r["owner"]["name"] == "HubRef")
        trait = next(r for r in records if r["owner"]["trait"] == "Default")

        self.assertEqual(inherent["owner"]["type"], "crate::session::HubRef<'a>")
        self.assertEqual(inherent["owner"]["kind"], "inherent")
        self.assertEqual(trait["owner"]["type"], "crate::session::HubMut<'_>")
        self.assertEqual(trait["owner"]["kind"], "trait")
        self.assertEqual(trait["function_names"], ["default"])

    def test_mixed_file_keeps_worldsession_cx_and_other_impl_bytes_exact(self):
        raw = sample_source()
        records = self.records(raw)
        selected_spans = sorted(
            (r["span"]["char_start"], r["span"]["char_end"]) for r in records
        )
        expected = []
        cursor = 0
        for start, end in selected_spans:
            expected.append(raw[cursor:start])
            cursor = end
        expected.append(raw[cursor:])

        retained, moved = codemod.extract_recorded_blocks(raw, records, self.lexer)
        self.assertEqual(retained, "".join(expected))
        self.assertEqual(len(moved), 2)
        for exact_shell_impl in (
            "impl crate::session::WorldSession {\n    fn shell_only(&self) { let brace = \"}\"; }\n}",
            "impl crate::session::CharacterCx {\n    fn cx_only(&self) { /* } */ }\n}",
            "impl crate::session::Unrelated {\n    fn unrelated(&self) {}\n}",
        ):
            self.assertIn(exact_shell_impl, retained)

    def test_braces_in_strings_and_comments_do_not_change_recorded_block(self):
        raw = sample_source()
        records = self.records(raw)
        retained, moved = codemod.extract_recorded_blocks(raw, records, self.lexer)

        self.assertIn('const TEXT: &str = r#" impl crate::session::HubMut', retained)
        self.assertEqual(
            [r["owner"]["name"] for r in (item.record for item in moved)],
            ["HubRef", "HubMut"],
        )

    def test_module_inner_docs_and_attrs_remain_outside_impl_span(self):
        raw = '''//! Module documentation belongs to the source module.\n#![allow(dead_code)]\nimpl crate::session::HubRef<'_> {}\n'''
        record = self.records(raw)[0]
        retained, moved = codemod.extract_recorded_blocks(raw, [record], self.lexer)

        self.assertTrue(retained.startswith("//! Module documentation"))
        self.assertIn("#![allow(dead_code)]", retained)
        self.assertEqual(moved[0].source, "impl crate::session::HubRef<'_> {}")

    def test_duplicate_and_overlapping_spans_are_rejected(self):
        raw = sample_source()
        records = self.records(raw)
        with self.assertRaisesRegex(codemod.PlanError, "duplicate span"):
            codemod.extract_recorded_blocks(raw, records + [copy.deepcopy(records[0])], self.lexer)

        overlapping = copy.deepcopy(records)
        overlapping[1]["span"]["char_start"] = overlapping[0]["span"]["char_end"] - 1
        with self.assertRaisesRegex(codemod.PlanError, "overlapping spans"):
            codemod.extract_recorded_blocks(raw, overlapping, self.lexer)

    def test_stale_block_hash_is_rejected(self):
        raw = sample_source()
        record = copy.deepcopy(self.records(raw)[0])
        record["span"]["sha256"] = "0" * 64

        with self.assertRaisesRegex(codemod.PlanError, "stale block sha256"):
            codemod.extract_recorded_blocks(raw, [record], self.lexer)

    def test_source_drift_and_reapply_are_rejected(self):
        raw = sample_source()
        records = self.records(raw)
        with self.assertRaisesRegex(codemod.PlanError, "source drift or reapply"):
            codemod.extract_recorded_blocks(raw.replace("const TEXT", "const NOTE", 1), records, self.lexer)

        retained, _ = codemod.extract_recorded_blocks(raw, records, self.lexer)
        with self.assertRaisesRegex(codemod.PlanError, "source drift or reapply"):
            codemod.extract_recorded_blocks(retained, records, self.lexer)


if __name__ == "__main__":
    unittest.main()
