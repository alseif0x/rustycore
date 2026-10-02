"""Focused tests for the exact canonical-player extraction codemod."""

import contextlib
import io
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

import f4_canonical_extract as codemod


def sample_source() -> str:
    return '''//! Read one canonical `Player` by GUID and placement, without going through the
//! owning [`WorldSession`](crate::session::WorldSession).

use wow_core::ObjectGuid;

#[cfg(test)]
pub(crate) fn install_canonical_player_owner_for_test(
    session: &mut crate::session::WorldSession,
) -> ObjectGuid {
    let guid = session.player_guid().unwrap();
    session.set_player_guid(Some(guid));
    guid
}

pub(crate) fn canonical_unit_party_member_visible_auras_like_cpp() {}
pub(crate) fn with_canonical_player_at_like_cpp<R>() {}
pub(crate) fn with_canonical_player_at_mut_like_cpp<R>() {}
pub(crate) struct CanonicalPlayerVitalsLikeCpp;
pub(crate) struct CanonicalPlayerPartyStateLikeCpp;
pub(crate) type CanonicalPlayerPresentationLikeCpp = ();
pub(crate) fn canonical_player_presentation_like_cpp() {}
pub(crate) fn canonical_player_aggro_unit_state_like_cpp() {}
pub(crate) fn set_player_visible_item_values_like_cpp() {}
fn power_kind_from_u8_like_cpp() {}
fn power_to_u16_like_cpp() {}
pub(crate) fn canonical_player_vitals_like_cpp() {}
pub(crate) fn canonical_player_party_state_like_cpp() {}
pub(crate) fn canonical_player_honor_stats_like_cpp() {}

#[cfg(test)]
pub(crate) fn configure_canonical_player_vitals_for_test() {}

#[cfg(test)]
pub(crate) fn configure_canonical_player_party_flags_for_test() {}

pub type HonorStatsLikeCpp = ();
pub fn canonical_player_unit_flags2_like_cpp() {}
pub const PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP: u32 = 0;
pub fn canonical_player_is_contested_pvp_like_cpp() {}
pub fn canonical_player_reputation_state_flags_like_cpp() {}
pub fn canonical_player_forced_reputation_faction_ids_like_cpp() {}
pub fn canonical_player_reputation_standings_like_cpp() {}

pub(crate) fn unrelated_crate_helper() {}
'''


class CanonicalPlayerExtractTests(unittest.TestCase):
    def setUp(self):
        self.lexer = codemod._lexer(codemod.ROOT)

    def test_extracts_only_installer_and_keeps_its_exact_text_in_shell(self):
        original = sample_source()
        _, installer = codemod.extract_installer(original, self.lexer)
        core, shell, changed = codemod.transform_source(original, self.lexer)

        self.assertIn(installer, shell)
        self.assertNotIn(codemod.INSTALLER, core)
        self.assertIn("crate::session::WorldSession", shell)
        self.assertNotIn("crate::session::WorldSession", core)
        self.assertIn("//! owning WorldSession.", core)
        self.assertNotIn(codemod.OLD_SESSION_LINK, core)
        self.assertEqual(len(changed), 10)

    def test_only_allowlisted_visibility_and_fixture_gates_change(self):
        core, _, _ = codemod.transform_source(sample_source(), self.lexer)

        for name in codemod.PUBLIC_WORLD_FUNCTIONS:
            self.assertIn(f"pub fn {name}", core)
        self.assertIn("pub type CanonicalPlayerPresentationLikeCpp", core)
        for name in codemod.FIXTURE_FUNCTIONS:
            self.assertIn(f"#[cfg(any(test, feature = \"test-fixtures\"))]\npub fn {name}", core)
        for name in (
            "canonical_unit_party_member_visible_auras_like_cpp",
            "canonical_player_aggro_unit_state_like_cpp",
            "canonical_player_vitals_like_cpp",
            "canonical_player_party_state_like_cpp",
            "canonical_player_honor_stats_like_cpp",
            "unrelated_crate_helper",
        ):
            self.assertIn(f"pub(crate) fn {name}", core)
        self.assertIn("pub(crate) struct CanonicalPlayerVitalsLikeCpp", core)
        self.assertIn("pub(crate) struct CanonicalPlayerPartyStateLikeCpp", core)
        self.assertIn("fn power_kind_from_u8_like_cpp", core)
        self.assertIn("fn power_to_u16_like_cpp", core)
        self.assertIn("pub fn canonical_player_reputation_standings_like_cpp", core)

    def test_public_api_remains_exactly_the_original_seven_names(self):
        _, shell, _ = codemod.transform_source(sample_source(), self.lexer)
        exported = [name for kind, name in codemod.PUBLIC_API if f"{name}" in shell]

        self.assertEqual(tuple(exported), tuple(name for _, name in codemod.PUBLIC_API))
        self.assertNotIn("canonical_player_honor_stats_like_cpp", shell)
        self.assertNotIn("canonical_player_aggro_unit_state_like_cpp", shell)

    def test_missing_and_ambiguous_installer_are_rejected(self):
        source = sample_source()
        without_installer, _ = codemod.extract_installer(source, self.lexer)
        with self.assertRaisesRegex(codemod.CodemodError, "found 0"):
            codemod.extract_installer(without_installer, self.lexer)

        installer_start = source.index("#[cfg(test)]\npub(crate) fn install_")
        installer_end = source.index("\n}\n", installer_start) + 3
        duplicate = source + "\n" + source[installer_start:installer_end]
        with self.assertRaisesRegex(codemod.CodemodError, "found 2"):
            codemod.extract_installer(duplicate, self.lexer)

    def test_missing_and_ambiguous_visibility_targets_are_rejected(self):
        missing = sample_source().replace(
            "pub(crate) fn set_player_visible_item_values_like_cpp() {}\n", "", 1
        )
        with self.assertRaisesRegex(codemod.CodemodError, "set_player_visible_item_values_like_cpp"):
            codemod.transform_source(missing, self.lexer)

        duplicate = sample_source() + "\npub(crate) fn canonical_player_presentation_like_cpp() {}\n"
        with self.assertRaisesRegex(codemod.CodemodError, "found 2"):
            codemod.transform_source(duplicate, self.lexer)

    def test_plan_is_read_only_and_apply_is_idempotent(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            source_path = root / codemod.SOURCE
            source_path.parent.mkdir(parents=True)
            source_path.write_text(sample_source(), encoding="utf-8")

            with contextlib.redirect_stdout(io.StringIO()):
                codemod.run("plan", root)
            self.assertFalse((root / codemod.CORE).exists())

            with contextlib.redirect_stdout(io.StringIO()):
                codemod.run("apply", root)
            first_source = source_path.read_bytes()
            first_core = (root / codemod.CORE).read_bytes()
            with contextlib.redirect_stdout(io.StringIO()):
                codemod.run("apply", root)
                codemod.run("plan", root)

            self.assertEqual(source_path.read_bytes(), first_source)
            self.assertEqual((root / codemod.CORE).read_bytes(), first_core)


if __name__ == "__main__":
    unittest.main()
