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


def fixture_session_mod():
    lines = []
    for name in codemod.SHELL_TYPES:
        if name == "RepresentedBattlePetCageItemLikeCpp":
            lines.append('#[cfg(any(test, feature = "test-fixtures"))]')
        lines.append(f"pub(crate) use battle_pet_adapter::{name};")
    return "\n".join(lines) + "\n"


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
        session_mod, _ = codemod._update_shell_reexports(fixture_session_mod())

        self.assertTrue(codemod._already_applied(remaining, core, session_mod, lexer))
        duplicated_type = core + "\npub struct RepresentedBattlePetSlotLikeCpp {}\n"
        self.assertFalse(
            codemod._already_applied(remaining, duplicated_type, session_mod, lexer)
        )
        duplicated_impl = core + "\nimpl RepresentedBattlePetSlotLikeCpp {}\n"
        self.assertFalse(
            codemod._already_applied(remaining, duplicated_impl, session_mod, lexer)
        )

    def test_apply_is_idempotent_for_complete_destination_fixture(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_fixture(root)

            with redirect_stdout(StringIO()) as output:
                codemod.run("apply", root)
                first = tuple(
                    (root / path).read_bytes()
                    for path in (codemod.SOURCE, codemod.CORE, codemod.SESSION_MOD)
                )
                codemod.run("apply", root)
                second = tuple(
                    (root / path).read_bytes()
                    for path in (codemod.SOURCE, codemod.CORE, codemod.SESSION_MOD)
                )

            self.assertEqual(first, second)
            self.assertIn("already applied", output.getvalue())


if __name__ == "__main__":
    unittest.main()
