"""Focused fixtures for the exact phasing packet/party extraction codemod."""

import unittest
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory

import f4_phasing_extract as codemod


def party_function():
    return """/// C++ `PhasingHandler::FillPartyMemberPhase`.
pub fn party_member_phase_states_like_cpp(
    phase_shift: &PhaseShift,
) -> Result<PartyMemberPhaseStates, PhaseShiftPacketBuildError> {
    let phases = phase_shift
        .phases_like_cpp()
        .map(|phase| {
            Ok(PartyMemberPhase {
                flags: u32::from(phase.flags().bits()),
                id: u16::try_from(phase.id())
                    .map_err(|_| PhaseShiftPacketBuildError::PhaseIdOutOfRange(phase.id()))?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(PartyMemberPhaseStates {
        phase_shift_flags: phase_shift.flags_like_cpp().bits(),
        personal_guid: phase_shift.personal_guid_like_cpp(),
        phases,
    })
}
"""


def sample_source():
    return f"""use std::{{collections::HashSet, error::Error, fmt}};
use std::fmt::Write as _;
use wow_entities::PhaseShift;
use wow_packet::packets::misc::{{PhaseShiftChange, PhaseShiftDataPhase}};
use wow_packet::packets::party::{{PartyMemberPhase, PartyMemberPhaseStates}};

/// Kept enum docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseShiftPacketBuildError {{
    PhaseIdOutOfRange(u32),
    VisibleMapIdOutOfRange(u32),
    UiMapPhaseIdOutOfRange(u32),
}}

impl fmt::Display for PhaseShiftPacketBuildError {{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {{
        match self {{
            Self::PhaseIdOutOfRange(id) => write!(f, "phase id {{id}} does not fit"),
            Self::VisibleMapIdOutOfRange(id) => write!(f, "visible map id {{id}} does not fit"),
            Self::UiMapPhaseIdOutOfRange(id) => write!(f, "UI map phase id {{id}} does not fit"),
        }}
    }}
}}

impl Error for PhaseShiftPacketBuildError {{}}

pub struct RetainedHelper {{ _set: HashSet<u32> }}

{party_function()}

pub fn phase_shift_change_for_player_like_cpp() -> Result<(), PhaseShiftPacketBuildError> {{
    Ok(())
}}
"""


def write_fixture(root, *, source=None, core=None):
    source_path = root / codemod.SOURCE
    source_path.parent.mkdir(parents=True, exist_ok=True)
    source_path.write_text(sample_source() if source is None else source, encoding="utf-8")
    if core is not None:
        core_path = root / codemod.CORE
        core_path.parent.mkdir(parents=True, exist_ok=True)
        core_path.write_text(core, encoding="utf-8")


class ExactPhasingCutTests(unittest.TestCase):
    def test_extracts_only_named_items_and_preserves_attrs_docs_and_helper(self):
        lexer = codemod._item_support(codemod.REPO)
        source = sample_source()

        remaining, core = codemod._build_core_module(source, lexer)

        self.assertNotIn("pub enum PhaseShiftPacketBuildError", remaining)
        self.assertNotIn("impl fmt::Display for PhaseShiftPacketBuildError", remaining)
        self.assertNotIn("impl Error for PhaseShiftPacketBuildError", remaining)
        self.assertNotIn("pub fn party_member_phase_states_like_cpp", remaining)
        self.assertIn("pub struct RetainedHelper", remaining)
        self.assertIn("phase_shift_change_for_player_like_cpp", remaining)
        self.assertIn("/// Kept enum docs.", core)
        self.assertIn("#[derive(Debug, Clone, Copy, PartialEq, Eq)]", core)
        self.assertIn("/// C++ `PhasingHandler::FillPartyMemberPhase`.", core)
        self.assertIn("pub fn party_member_phase_states_like_cpp", core)
        self.assertIn(codemod.CORE_IMPORTS, core)

    def test_plan_is_read_only_and_apply_is_idempotent(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_fixture(root)
            source_path = root / codemod.SOURCE
            before = source_path.read_bytes()

            with redirect_stdout(StringIO()):
                codemod.run("plan", root)
            self.assertEqual(source_path.read_bytes(), before)
            self.assertFalse((root / codemod.CORE).exists())

            with redirect_stdout(StringIO()) as output:
                codemod.run("apply", root)
                first = tuple(
                    (root / relative).read_bytes()
                    for relative in (codemod.SOURCE, codemod.CORE)
                )
                codemod.run("apply", root)
                second = tuple(
                    (root / relative).read_bytes()
                    for relative in (codemod.SOURCE, codemod.CORE)
                )
            self.assertEqual(first, second)
            self.assertIn("already applied", output.getvalue())

            result = source_path.read_text(encoding="utf-8")
            self.assertIn("use std::collections::HashSet;", result)
            self.assertIn("use std::fmt::Write as _;", result)
            self.assertNotIn("error::Error, fmt", result)
            self.assertIn("#[cfg(test)]\nuse wow_packet::packets::party::PartyMemberPhase;", result)
            self.assertNotIn("PartyMemberPhaseStates};", result)
            self.assertIn("pub use wow_world_core::phasing::", result)

    def test_rustfmt_formatted_reexport_is_idempotent(self):
        formatted = (
            "pub use wow_world_core::phasing::{PhaseShiftPacketBuildError, "
            "party_member_phase_states_like_cpp};"
        )
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_fixture(root)
            with redirect_stdout(StringIO()):
                codemod.run("apply", root)

            source_path = root / codemod.SOURCE
            source = source_path.read_text(encoding="utf-8")
            self.assertEqual(source.count(codemod.PARTY_REEXPORT), 1)
            party_import = "#[cfg ( test )]\nuse wow_packet :: packets :: party :: PartyMemberPhase ;"
            rendered_source = source.replace(codemod.PARTY_REEXPORT, formatted).replace(
                codemod.PARTY_IMPORT_NEW, party_import
            )
            source_path.write_text(
                rendered_source, encoding="utf-8"
            )
            before = source_path.read_bytes()
            with redirect_stdout(StringIO()) as output:
                codemod.run("plan", root)

            self.assertIn("already applied", output.getvalue())
            self.assertEqual(source_path.read_bytes(), before)
            self.assertIn(party_import, rendered_source)
            self.assertIn(formatted, rendered_source)

    def test_wrong_party_import_cfg_is_rejected(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_fixture(root)
            with redirect_stdout(StringIO()):
                codemod.run("apply", root)

            source_path = root / codemod.SOURCE
            source = source_path.read_text(encoding="utf-8").replace(
                "#[cfg(test)]\nuse wow_packet::packets::party::PartyMemberPhase;",
                "#[cfg(not(test))]\nuse wow_packet::packets::party::PartyMemberPhase;",
            )
            source_path.write_text(source, encoding="utf-8")
            before = source_path.read_bytes()
            with self.assertRaisesRegex(codemod.CodemodError, "partial or unexpected"):
                codemod.run("plan", root)
            self.assertEqual(source_path.read_bytes(), before)

    def test_comments_do_not_satisfy_core_reexport(self):
        formatted = (
            "pub use wow_world_core::phasing::{PhaseShiftPacketBuildError, "
            "party_member_phase_states_like_cpp};"
        )
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_fixture(root)
            with redirect_stdout(StringIO()):
                codemod.run("apply", root)

            source_path = root / codemod.SOURCE
            source = source_path.read_text(encoding="utf-8")
            source = source.replace(codemod.PARTY_REEXPORT, formatted)
            self.assertEqual(source.count(formatted), 1)
            source = source.replace(formatted, "// " + formatted, 1)
            source_path.write_text(source, encoding="utf-8")
            before = source_path.read_bytes()
            with self.assertRaisesRegex(codemod.CodemodError, "partial or unexpected"):
                codemod.run("plan", root)
            self.assertEqual(source_path.read_bytes(), before)

    def test_duplicate_core_reexports_are_rejected(self):
        formatted = (
            "pub use wow_world_core::phasing::{PhaseShiftPacketBuildError, "
            "party_member_phase_states_like_cpp};"
        )
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_fixture(root)
            with redirect_stdout(StringIO()):
                codemod.run("apply", root)

            source_path = root / codemod.SOURCE
            source = source_path.read_text(encoding="utf-8")
            source_path.write_text(source + "\n" + formatted + "\n", encoding="utf-8")
            before = source_path.read_bytes()
            with self.assertRaisesRegex(codemod.CodemodError, "partial or unexpected"):
                codemod.run("plan", root)
            self.assertEqual(source_path.read_bytes(), before)

    def test_missing_function_is_rejected_without_writes(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = sample_source().replace(party_function(), "")
            write_fixture(root, source=source)
            source_path = root / codemod.SOURCE
            before = source_path.read_bytes()

            with self.assertRaisesRegex(codemod.CodemodError, "expected exactly one fn"):
                codemod.run("apply", root)

            self.assertEqual(source_path.read_bytes(), before)
            self.assertFalse((root / codemod.CORE).exists())

    def test_duplicate_enum_or_trait_impl_is_rejected_without_writes(self):
        duplicate_enum = "\n#[derive(Debug)]\npub enum PhaseShiftPacketBuildError { Duplicate }\n"
        duplicate_impl = "\nimpl Error for PhaseShiftPacketBuildError {}\n"
        for suffix, message in (
            (duplicate_enum, "expected exactly one enum declaration"),
            (duplicate_impl, "expected exactly one `Error` impl"),
        ):
            with self.subTest(message=message), TemporaryDirectory() as temporary:
                root = Path(temporary)
                source = sample_source() + suffix
                write_fixture(root, source=source)
                source_path = root / codemod.SOURCE
                before = source_path.read_bytes()

                with self.assertRaisesRegex(codemod.CodemodError, message):
                    codemod.run("apply", root)

                self.assertEqual(source_path.read_bytes(), before)
                self.assertFalse((root / codemod.CORE).exists())

    def test_partial_core_destination_is_rejected_without_world_changes(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_fixture(root, core=codemod.CORE_IMPORTS + "\n\npub enum Incomplete {}\n")
            source_path = root / codemod.SOURCE
            source_before = source_path.read_bytes()
            core_path = root / codemod.CORE
            core_before = core_path.read_bytes()

            with self.assertRaisesRegex(codemod.CodemodError, "partial or unexpected"):
                codemod.run("apply", root)

            self.assertEqual(source_path.read_bytes(), source_before)
            self.assertEqual(core_path.read_bytes(), core_before)


if __name__ == "__main__":
    unittest.main()
