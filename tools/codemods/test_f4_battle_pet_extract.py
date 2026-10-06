"""Focused self-tests for the exact battle-pet DTO extraction codemod."""

import unittest
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory

from f4_battle_pet_extract import (
    DTO_FIELDS,
    DTO_IMPLS,
    DTO_KINDS,
    DTO_METHODS,
    REPO,
    extract_dto_items,
    widen_moved_items,
)
import f4_battle_pet_extract as codemod


def sample_source():
    chunks = [
        "pub(crate) enum RepresentedBattlePetXpSourceLikeCpp { PetBattle }",
        "pub(crate) fn apply_battle_pet_calculated_stats_like_cpp() {}",
    ]
    for name, kind in DTO_KINDS.items():
        attributes = []
        if name == "RepresentedBattlePetCageItemLikeCpp":
            attributes.append('#[cfg(any(test, feature = "test-fixtures"))]')
        if name == "RepresentedBattlePetQueryCompanionLikeCpp":
            attributes.append("/// Retained DTO documentation.")
        if kind == "enum":
            attributes.append("#[derive(Clone, Copy)]")
            declaration = f"pub(crate) enum {name} {{ New }}"
        else:
            fields = DTO_FIELDS[name]
            field_text = "\n".join(f"    pub(crate) {field}: u8," for field in fields)
            attributes.append("#[derive(Clone)]")
            declaration = f"pub(crate) struct {name} {{\n{field_text}\n}}"
        chunks.append("\n".join((*attributes, declaration)))

    chunks.extend(
        [
            "impl RepresentedBattlePetSlotLikeCpp {\n"
            "    pub(crate) fn locked_empty(&self) {}\n"
            "    pub(crate) fn packet_slot_like_cpp(&self) {}\n"
            "}",
            "impl RepresentedBattlePetDataLikeCpp {\n"
            "    pub(crate) fn minimal_like_cpp(&self) {}\n"
            "    pub(crate) fn packet_info_like_cpp(&self) {}\n"
            "    pub(crate) fn private_guard_like_cpp(&self) {}\n"
            "}",
        ]
    )
    return "\n\n".join(chunks) + "\n"


def fixture_source():
    return codemod.SHELL_IMPORT_OLD + "\n\n" + sample_source()


def rustfmt_shell_import():
    return """#[cfg(test)]
use super::RepresentedBattlePetCageItemLikeCpp;
use super::{
    WorldSession,
    RepresentedBattlePetDataLikeCpp,
    ObjectGuid, Instant,
    RepresentedBattlePetCalculatedStatsLikeCpp,
    RepresentedAuraEffectLikeCpp, AuraApplication,
};"""


def fixture_session_mod():
    lines = []
    for name in codemod.SHELL_TYPES:
        if name == "RepresentedBattlePetCageItemLikeCpp":
            lines.append('#[cfg(any(test, feature = "test-fixtures"))]')
        lines.append(f"pub(crate) use battle_pet_adapter::{name};")
    return "\n".join(lines) + "\n"


def p4b_session_mod():
    lexer = codemod._item_support(Path(REPO))
    session_mod, _ = codemod._update_shell_reexports(fixture_session_mod(), lexer)
    test_only = (
        "RepresentedBattlePetCageItemLikeCpp",
        "RepresentedBattlePetLevelCriteriaLikeCpp",
        "RepresentedBattlePetQueryCompanionLikeCpp",
        "RepresentedBattlePetSaveInfoLikeCpp",
    )
    for name in test_only:
        reexport = codemod._root_reexport_statement(name, core=True)
        if name in {
            "RepresentedBattlePetCageItemLikeCpp",
            "RepresentedBattlePetSaveInfoLikeCpp",
        }:
            session_mod = session_mod.replace(
                codemod.ROOT_FIXTURE_GATE + "\n" + reexport,
                "#[cfg(test)]\n" + reexport,
                1,
            )
        else:
            session_mod = session_mod.replace(
                reexport, "#[cfg(test)]\n" + reexport, 1
            )
    return session_mod


def write_fixture(root, *, source=None, core=None, session_mod=None):
    for relative, contents in (
        (codemod.SOURCE, fixture_source() if source is None else source),
        (codemod.SESSION_MOD, fixture_session_mod() if session_mod is None else session_mod),
    ):
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(contents, encoding="utf-8")
    if core is not None:
        path = root / codemod.CORE
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(core, encoding="utf-8")


class ExactBattlePetCutTests(unittest.TestCase):
    def test_shell_reexports_add_exact_save_info_gate_to_complete_set(self):
        lexer = codemod._item_support(Path(REPO))
        old = fixture_session_mod()

        updated, changed = codemod._update_shell_reexports(old, lexer)

        self.assertEqual(changed, list(codemod.SHELL_TYPES))
        self.assertEqual(codemod._root_reexport_layout(old, lexer), "old")
        self.assertEqual(codemod._root_reexport_layout(updated, lexer), "new")
        self.assertEqual(updated.count(codemod.ROOT_FIXTURE_GATE), 2)
        self.assertIn(
            codemod.ROOT_FIXTURE_GATE
            + "\n"
            + codemod._root_reexport_statement(
                "RepresentedBattlePetSaveInfoLikeCpp", core=True
            ),
            updated,
        )
        self.assertIn(
            codemod._root_reexport_statement(
                "RepresentedBattlePetLevelCriteriaLikeCpp", core=True
            ),
            updated,
        )
        self.assertNotIn("pub(crate) use battle_pet_adapter::", updated)

    def test_shell_reexports_reject_incomplete_mixed_and_bad_old_sets(self):
        lexer = codemod._item_support(Path(REPO))
        old = fixture_session_mod()
        cage = "RepresentedBattlePetCageItemLikeCpp"
        save_info = "RepresentedBattlePetSaveInfoLikeCpp"
        other = "RepresentedBattlePetCalculatedStatsLikeCpp"
        cage_old = codemod._root_reexport_statement(cage, core=False)
        save_old = codemod._root_reexport_statement(save_info, core=False)
        other_old = codemod._root_reexport_statement(other, core=False)
        cage_gate = codemod.ROOT_FIXTURE_GATE + "\n" + cage_old
        malformed = (
            old.replace(other_old, "", 1),
            old + other_old + "\n",
            old.replace(other_old, codemod._root_reexport_statement(other, core=True), 1),
            old.replace(other_old, "fn helper() {\n" + other_old + "\n}", 1),
            old.replace(cage_gate, cage_old, 1),
            old.replace(cage_gate, "#[cfg(test)]\n" + cage_old, 1),
            old.replace(cage_gate, codemod.ROOT_FIXTURE_GATE + "\n" + cage_gate, 1),
            old.replace(save_old, codemod.ROOT_FIXTURE_GATE + "\n" + save_old, 1),
        )
        for candidate in malformed:
            with self.subTest(candidate=candidate):
                with self.assertRaises(codemod.CodemodError):
                    codemod._update_shell_reexports(candidate, lexer)

    def test_already_applied_requires_exact_save_info_and_cage_gates(self):
        lexer = codemod._item_support(Path(REPO))
        remaining, moved = extract_dto_items(fixture_source(), lexer)
        core, _ = widen_moved_items(codemod.CORE_IMPORTS + "\n" + moved + "\n", lexer)
        remaining, _ = codemod._update_shell_import(remaining, lexer)
        session_mod, _ = codemod._update_shell_reexports(fixture_session_mod(), lexer)
        remaining = remaining.replace(codemod.SHELL_IMPORT_NEW, rustfmt_shell_import(), 1)

        save_info_line = codemod._root_reexport_statement(
            "RepresentedBattlePetSaveInfoLikeCpp", core=True
        )
        save_info_gate = codemod.ROOT_FIXTURE_GATE + "\n" + save_info_line
        cage_line = codemod._root_reexport_statement(
            "RepresentedBattlePetCageItemLikeCpp", core=True
        )
        cage_gate = codemod.ROOT_FIXTURE_GATE + "\n" + cage_line
        other_line = codemod._root_reexport_statement(
            "RepresentedBattlePetCalculatedStatsLikeCpp", core=True
        )
        other_old_line = codemod._root_reexport_statement(
            "RepresentedBattlePetCalculatedStatsLikeCpp", core=False
        )
        malformed = (
            session_mod.replace(save_info_gate, save_info_line, 1),
            session_mod.replace(save_info_gate, "#[cfg(test)]\n" + save_info_line, 1),
            session_mod.replace(
                save_info_gate,
                codemod.ROOT_FIXTURE_GATE + "\n" + save_info_gate,
                1,
            ),
            session_mod.replace(cage_gate, cage_line, 1),
            session_mod.replace(cage_gate, "#[cfg(test)]\n" + cage_line, 1),
            session_mod.replace(cage_gate, codemod.ROOT_FIXTURE_GATE + "\n" + cage_gate, 1),
            session_mod.replace(other_line, other_old_line, 1),
            session_mod.replace(other_line, "", 1),
            session_mod + other_line + "\n",
            session_mod + save_info_line + "\n",
        )
        for candidate in malformed:
            with self.subTest(candidate=candidate):
                self.assertFalse(
                    codemod._already_applied(remaining, core, candidate, lexer)
                )

    def test_already_applied_accepts_only_the_exact_p4b_gates(self):
        lexer = codemod._item_support(Path(REPO))
        remaining, moved = extract_dto_items(fixture_source(), lexer)
        core, _ = widen_moved_items(codemod.CORE_IMPORTS + "\n" + moved + "\n", lexer)
        remaining, _ = codemod._update_shell_import(remaining, lexer)
        remaining = remaining.replace(codemod.SHELL_IMPORT_NEW, rustfmt_shell_import(), 1)
        session_mod = p4b_session_mod()

        self.assertEqual(codemod._root_reexport_layout(session_mod, lexer), "new-p4b")
        self.assertTrue(codemod._already_applied(remaining, core, session_mod, lexer))

        test_only = (
            "RepresentedBattlePetCageItemLikeCpp",
            "RepresentedBattlePetLevelCriteriaLikeCpp",
            "RepresentedBattlePetQueryCompanionLikeCpp",
            "RepresentedBattlePetSaveInfoLikeCpp",
        )
        malformed = []
        for name in test_only:
            reexport = codemod._root_reexport_statement(name, core=True)
            gated = "#[cfg(test)]\n" + reexport
            malformed.extend(
                (
                    session_mod.replace(reexport, "", 1),
                    session_mod.replace(gated, reexport, 1),
                    session_mod.replace(
                        gated,
                        codemod.ROOT_FIXTURE_GATE + "\n" + reexport,
                        1,
                    ),
                    session_mod.replace(
                        gated,
                        "#[cfg(test)]\n#[cfg(test)]\n" + reexport,
                        1,
                    ),
                )
            )

        malformed.append(
            session_mod
            + codemod._root_reexport_statement(
                "RepresentedBattlePetQueryCompanionLikeCpp", core=True
            )
            + "\n"
        )
        for candidate in malformed:
            with self.subTest(candidate=candidate):
                self.assertFalse(
                    codemod._already_applied(remaining, core, candidate, lexer)
                )

    def test_shell_import_layout_accepts_rustfmt_order_and_member_order(self):
        lexer = codemod._item_support(Path(REPO))
        source = (
            rustfmt_shell_import()
            + "\nuse super::{represented_aura_effect_amounts_like_cpp, warn};\n"
        )

        layout, imports = codemod._shell_import_layout(source, lexer)

        self.assertEqual(layout, "new")
        self.assertEqual(len(imports), 2)

        old_import = """use super::{
    WorldSession, ObjectGuid,
    RepresentedAuraEffectLikeCpp, Instant, AuraApplication,
};"""
        self.assertEqual(codemod._shell_import_layout(old_import, lexer)[0], "old")
        indented_root_imports = "\n".join(
            f"    {line}" if line else line for line in source.splitlines()
        )
        self.assertEqual(
            codemod._shell_import_layout(indented_root_imports, lexer)[0], "new"
        )

    def test_shell_import_layout_rejects_partial_duplicate_and_spoofed_shapes(self):
        lexer = codemod._item_support(Path(REPO))
        valid = rustfmt_shell_import()
        grouped = valid[valid.index("use super::{") :]
        nested_group = (
            valid[: valid.index("use super::{")]
            + "fn helper() {\n"
            + grouped
            + "\n}\n"
        )
        malformed = (
            valid.replace("RepresentedBattlePetDataLikeCpp,\n", ""),
            valid.replace("WorldSession,", "WorldSession, WorldSession,"),
            valid.replace("WorldSession,", "WorldSession, UnreviewedType,"),
            valid.replace("use super::{", "#[cfg(test)]\nuse super::{"),
            "#[cfg(test)]\n" + grouped,
            nested_group,
            valid.replace("#[cfg(test)]", '#[cfg(any(test, feature = "test-fixtures"))]'),
            valid.replace(
                "RepresentedBattlePetDataLikeCpp,",
                "/* RepresentedBattlePetDataLikeCpp, */",
            ),
            valid.replace(
                "RepresentedBattlePetDataLikeCpp,",
                '"RepresentedBattlePetDataLikeCpp",',
            ),
            valid + "\n" + valid,
            "/*\n" + valid + "\n*/",
            'const IMPORT_SPOOF: &str = r##"' + valid + '"##;\n',
        )
        for candidate in malformed:
            with self.subTest(candidate=candidate):
                with self.assertRaises(codemod.CodemodError):
                    codemod._shell_import_layout(candidate, lexer)

    def test_extracts_only_named_dtos_and_complete_impls(self):
        lexer = codemod._item_support(Path(REPO))
        source = sample_source()

        remaining, moved = extract_dto_items(source, lexer)

        for name in DTO_KINDS:
            self.assertNotIn(name, remaining)
            self.assertIn(name, moved)
        for name in DTO_IMPLS:
            self.assertIn(f"impl {name}", moved)
        self.assertIn("RepresentedBattlePetXpSourceLikeCpp", remaining)
        self.assertIn("apply_battle_pet_calculated_stats_like_cpp", remaining)
        self.assertIn('cfg(any(test, feature = "test-fixtures"))', moved)
        self.assertIn(
            '#[cfg(any(test, feature = "test-fixtures"))]\n'
            "#[derive(Clone)]\n"
            "pub(crate) struct RepresentedBattlePetCageItemLikeCpp",
            moved,
        )
        self.assertIn("/// Retained DTO documentation.", moved)

    def test_widens_only_reviewed_types_fields_and_methods(self):
        lexer = codemod._item_support(Path(REPO))
        _, moved = extract_dto_items(sample_source(), lexer)

        widened, changed = widen_moved_items(moved, lexer)

        expected = len(DTO_KINDS) + sum(map(len, DTO_FIELDS.values())) + len(DTO_METHODS)
        self.assertEqual(len(changed), expected)
        self.assertIn("pub(crate) fn private_guard_like_cpp", widened)
        self.assertNotIn("pub(crate) fn locked_empty", widened)
        self.assertNotIn("pub(crate) fn packet_info_like_cpp", widened)

    def test_duplicate_source_dto_is_rejected(self):
        lexer = codemod._item_support(Path(REPO))
        duplicate = (
            sample_source()
            + "\npub(crate) struct RepresentedBattlePetSlotLikeCpp { duplicate: u8 }\n"
        )

        with self.assertRaisesRegex(codemod.CodemodError, "expected one DTO declaration"):
            extract_dto_items(duplicate, lexer)

    def test_partial_destination_is_rejected_without_changing_sources(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            partial_core = (
                codemod.CORE_IMPORTS
                + "\npub enum RepresentedBattlePetSaveInfoLikeCpp { New }\n"
            )
            write_fixture(root, core=partial_core)
            source_path = root / codemod.SOURCE
            session_mod_path = root / codemod.SESSION_MOD
            source_before = source_path.read_bytes()
            session_mod_before = session_mod_path.read_bytes()

            with self.assertRaisesRegex(codemod.CodemodError, "partial or unexpected"):
                codemod.run("apply", root)

            self.assertEqual(source_path.read_bytes(), source_before)
            self.assertEqual(session_mod_path.read_bytes(), session_mod_before)

    def test_already_applied_requires_unique_core_items_and_impls(self):
        lexer = codemod._item_support(Path(REPO))
        remaining, moved = extract_dto_items(fixture_source(), lexer)
        core, _ = widen_moved_items(codemod.CORE_IMPORTS + "\n" + moved + "\n", lexer)
        remaining, _ = codemod._update_shell_import(remaining, lexer)
        session_mod, _ = codemod._update_shell_reexports(fixture_session_mod(), lexer)
        remaining = remaining.replace(codemod.SHELL_IMPORT_NEW, rustfmt_shell_import(), 1)

        self.assertTrue(codemod._already_applied(remaining, core, session_mod, lexer))
        duplicated_type = core + "\npub struct RepresentedBattlePetSlotLikeCpp {}\n"
        self.assertFalse(
            codemod._already_applied(remaining, duplicated_type, session_mod, lexer)
        )
        duplicated_impl = core + "\nimpl RepresentedBattlePetSlotLikeCpp {}\n"
        self.assertFalse(
            codemod._already_applied(remaining, duplicated_impl, session_mod, lexer)
        )
        wrong_root_gate = session_mod.replace('"test-fixtures"', '"fixture-spoof"', 1)
        self.assertFalse(
            codemod._already_applied(remaining, core, wrong_root_gate, lexer)
        )

    def test_plan_and_apply_accept_rustfmt_ordered_complete_destination(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_fixture(root)

            with redirect_stdout(StringIO()) as output:
                codemod.run("apply", root)
                source_path = root / codemod.SOURCE
                source = source_path.read_text(encoding="utf-8")
                self.assertEqual(source.count(codemod.SHELL_IMPORT_NEW), 1)
                source_path.write_text(
                    source.replace(codemod.SHELL_IMPORT_NEW, rustfmt_shell_import(), 1),
                    encoding="utf-8",
                )
                first = tuple(
                    (root / path).read_bytes()
                    for path in (codemod.SOURCE, codemod.CORE, codemod.SESSION_MOD)
                )
                codemod.run("plan", root)
                planned = tuple(
                    (root / path).read_bytes()
                    for path in (codemod.SOURCE, codemod.CORE, codemod.SESSION_MOD)
                )
                codemod.run("apply", root)
                second = tuple(
                    (root / path).read_bytes()
                    for path in (codemod.SOURCE, codemod.CORE, codemod.SESSION_MOD)
                )

            self.assertEqual(first, planned)
            self.assertEqual(first, second)
            self.assertIn("already applied", output.getvalue())


if __name__ == "__main__":
    unittest.main()
