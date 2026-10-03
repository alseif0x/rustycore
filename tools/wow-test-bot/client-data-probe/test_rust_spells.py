"""Opt-in production-linked QA of all 49 private build-70170 spell tables.

Requires the explicitly built forever_spell_tables example. Only disposable
copies are mutated; no client, account, SQL, keys, network or runtime access.
Counts do not establish SpellInfo assembly or learned Player state.
"""
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import unittest


# Captured plaintext direct rows, unknown direct rows and readable copy entries.
# Copies may overwrite/skip source IDs: a header copy count is not an expected
# exact count of materialized new rows. No client record/text value is retained.
COUNTS = {
    "SpellName": (17565, 569, 14173),
    "SpellEffect": (42409, 1261, 0),
    "SpellMisc": (31720, 906, 0),
    "SpellAuraOptions": (12886, 67, 0),
    "SpellAuraRestrictions": (316, 17, 0),
    "SpellCastingRequirements": (3258, 44, 0),
    "SpellCategories": (10587, 344, 0),
    "SpellClassOptions": (6049, 8, 0),
    "SpellCooldowns": (4436, 65, 0),
    "SpellEmpower": (0, 0, 0),
    "SpellEmpowerStage": (0, 0, 0),
    "SpellEquippedItems": (1888, 17, 0),
    "SpellInterrupts": (9858, 312, 0),
    "SpellLabel": (4631, 31, 0),
    "SpellLevels": (12829, 157, 0),
    "SpellPower": (3429, 37, 0),
    "SpellPowerDifficulty": (0, 0, 0),
    "SpellReagents": (3300, 38, 0),
    "SpellReagentsCurrency": (0, 0, 0),
    "SpellScaling": (0, 0, 0),
    "SpellShapeshift": (3468, 0, 0),
    "SpellTargetRestrictions": (4442, 136, 0),
    "SpellTotems": (1079, 2, 0),
    "SpellXSpellVisual": (20248, 657, 0),
    "Difficulty": (17, 0, 5),
    "SpellCastTimes": (68, 0, 4),
    "SpellDuration": (123, 0, 11),
    "SpellRange": (58, 0, 0),
    "SpellRadius": (49, 0, 4),
    "SpellProcsPerMinute": (11, 0, 0),
    "SpellProcsPerMinuteMod": (0, 0, 0),
    "SpellLearnSpell": (52, 0, 0),
    "SpellShapeshiftForm": (21, 0, 11),
    "SummonProperties": (89, 1, 34),
    "BattlePetSpecies": (113, 2, 0),
    "SpellCategory": (263, 0, 1),
    "Talent": (432, 0, 0),
    "SpellItemEnchantment": (2199, 1, 17),
    "SpellVisual": (2273, 19, 3786),
    "SpellVisualMissile": (722, 1, 0),
    "SpellVisualEffectName": (2683, 6, 240),
    "LiquidType": (53, 0, 0),
    "ExpectedStat": (133, 0, 0),
    "ExpectedStatMod": (4, 0, 1),
    "ContentTuning": (100, 0, 0),
    "ContentTuningXExpected": (29, 0, 0),
    "RandPropPoints": (290, 0, 10),
    "MythicPlusSeason": (1, 0, 0),
    "UnitCondition": (346, 0, 7),
}


class SpellTableGuards(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if os.environ.get("FOREVER_ACK_PRIVATE_DATA_TESTS") != "1":
            raise RuntimeError("Explicit private-data QA acknowledgement required")
        cls.root = (Path(__file__).resolve().parents[3] / "target" / "forever-login").resolve(strict=True)
        cls.source = Path(os.environ["FOREVER_CLIENT_DATA_DIRECTORY"]).resolve(strict=True)
        if not cls.source.is_relative_to(cls.root) or not cls.source.is_dir():
            raise RuntimeError("Only acquired private fixture data is admitted")
        cls.binary = Path(os.environ["FOREVER_SPELL_TABLES_BIN"]).resolve(strict=True)

    @staticmethod
    def filename(name):
        return name + (".available.db2" if COUNTS[name][1] else ".db2")

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(dir=self.root, prefix="spell-negative-")
        self.addCleanup(self.temporary.cleanup)
        self.data = Path(self.temporary.name)
        for name in COUNTS:
            filename = self.filename(name)
            original = (self.source / filename).resolve(strict=True)
            if not original.is_relative_to(self.source) or not original.is_file():
                raise RuntimeError("Outside private fixture file")
            shutil.copy2(original, self.data / filename)
            (self.data / filename).chmod(0o600)

    def call(self, *args):
        return subprocess.run([str(self.binary), *map(str, args)],
                              capture_output=True, text=True, timeout=30)

    def read(self):
        return self.call("--ack-local-client-data", self.data,
                         "--ack-available-spell-info-tables")

    def rejected(self, result=None):
        result = self.read() if result is None else result
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        self.assertEqual(result.stderr.strip(), "Private Forever spell tables rejected")

    def changed(self, name, mutate):
        """Restore only this test's disposable copy, including on rejection failure."""
        path = self.data / self.filename(name)
        original = path.read_bytes()
        try:
            altered = bytearray(original)
            mutate(altered)
            path.write_bytes(altered)
            self.rejected()
        finally:
            path.write_bytes(original)

    def test_actual_batch_preserves_known_unknown_and_copy_count_boundaries(self):
        result = self.read()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, "")
        report = json.loads(result.stdout)
        self.assertEqual(set(report), {"build", "tables", "player_admitted", "spell_info_assembled"})
        self.assertEqual(report["build"], 70170)
        self.assertIs(report["player_admitted"], False)
        self.assertIs(report["spell_info_assembled"], False)
        self.assertEqual([table["name"] for table in report["tables"]], list(COUNTS))
        for table in report["tables"]:
            self.assertEqual(set(table), {"name", "known_materialized_records",
                                         "unknown_direct_baseline_records"})
            direct, unknown, copies = COUNTS[table["name"]]
            self.assertEqual(table["unknown_direct_baseline_records"], unknown)
            self.assertGreaterEqual(table["known_materialized_records"], direct)
            self.assertLessEqual(table["known_materialized_records"], direct + copies)

    def test_acknowledgement_and_exact_option_are_required(self):
        self.rejected(self.call())
        self.rejected(self.call("--ack-local-client-data", self.data, "--wrong"))
        self.rejected(self.call("--ack-local-client-data", self.data,
                                "--ack-available-spell-info-tables", "--extra"))

    def test_outside_fixture_and_symlink_escape_are_rejected(self):
        with tempfile.TemporaryDirectory() as outside:
            self.rejected(self.call("--ack-local-client-data", outside,
                                    "--ack-available-spell-info-tables"))
            alias = self.data / "outside"
            alias.symlink_to(outside, target_is_directory=True)
            self.rejected(self.call("--ack-local-client-data", alias,
                                    "--ack-available-spell-info-tables"))

    def test_complete_mode_never_falls_back_to_a_prefix(self):
        self.rejected(self.call("--ack-local-client-data", self.data))

    def test_strict_mode_rejects_a_prefix_renamed_as_complete(self):
        # The name is not a completeness witness; original unavailable-section
        # headers remain and ordinary open must still reject the first table.
        shutil.copy2(self.data / "SpellName.available.db2", self.data / "SpellName.db2")
        self.rejected(self.call("--ack-local-client-data", self.data))

    def test_all_table_hash_layout_and_native_locale_gates(self):
        for name in COUNTS:
            for offset in (152, 156, 168):
                with self.subTest(name=name, header_offset=offset):
                    def drift(data, at=offset):
                        value, = struct.unpack_from("<I", data, at)
                        struct.pack_into("<I", data, at, value ^ 1)
                    self.changed(name, drift)

    def test_missing_and_truncated_tables_do_not_publish_a_partial_batch(self):
        for name in COUNTS:
            with self.subTest(name=name):
                self.changed(name, lambda data: data.pop())
                path = self.data / self.filename(name)
                held = path.with_suffix(".held")
                path.rename(held)
                try:
                    self.rejected()
                finally:
                    held.rename(path)

    def test_each_unknown_first_section_cannot_be_promoted_to_plaintext(self):
        for name, (_, unknown, _) in COUNTS.items():
            if unknown:
                with self.subTest(name=name):
                    # WDC5 fixed header 204, then 40-byte section headers.
                    self.changed(name, lambda data: struct.pack_into("<Q", data, 244, 0))

    def test_empty_tables_require_all_primitive_field_metadata(self):
        for name, (direct, unknown, copies) in COUNTS.items():
            if not (direct or unknown or copies):
                with self.subTest(name=name):
                    self.changed(name, lambda data: data.__delitem__(slice(204, None)))

    def test_complete_spell_file_read_is_bounded(self):
        # First required full table after nine prefixes, not a generic prefix test.
        self.changed("SpellEmpower", lambda data: data.extend(bytes(4 * 1024 * 1024 + 1)))


if __name__ == "__main__":
    unittest.main()
