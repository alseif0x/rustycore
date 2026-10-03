"""Opt-in private-copy QA of the production numeric birth reader.

No source-client mutation, account, SQL or network access. Requires an explicitly
built Rust consumer; acquisition/header tests are not this acceptance evidence.
"""
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import unittest


class BirthCatalogGuards(unittest.TestCase):
    names = ["SkillLine.db2", "SkillRaceClassInfo.db2", "SkillLineAbility.available.db2",
             "CharacterLoadout.db2", "CharacterLoadoutItem.db2"]

    @classmethod
    def setUpClass(cls):
        if os.environ.get("FOREVER_ACK_PRIVATE_DATA_TESTS") != "1":
            raise RuntimeError("Explicit private-data QA acknowledgement required")
        cls.root = (Path(__file__).resolve().parents[3] / "target" / "forever-login").resolve(strict=True)
        cls.source = Path(os.environ["FOREVER_CLIENT_DATA_DIRECTORY"]).resolve(strict=True)
        if not cls.source.is_relative_to(cls.root) or not cls.source.is_dir():
            raise RuntimeError("Only acquired private fixture data is admitted")
        cls.binary = Path(os.environ["FOREVER_BIRTH_TABLES_BIN"]).resolve(strict=True)

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(dir=self.root, prefix="birth-negative-")
        self.addCleanup(self.temporary.cleanup)
        self.data = Path(self.temporary.name)
        for name in self.names:
            shutil.copy2(self.source / name, self.data / name)
            (self.data / name).chmod(0o600)

    def call(self, *args):
        return subprocess.run([str(self.binary), *map(str, args)],
                              capture_output=True, text=True, timeout=10)

    def read(self):
        return self.call("--ack-local-client-data", self.data, "--ack-available-birth-abilities")

    def rejected(self, result=None):
        result = self.read() if result is None else result
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        self.assertEqual(result.stderr.strip(), "Private Forever birth catalog rejected")

    def modify(self, name, offset, value, fmt="<I"):
        path = self.data / name
        data = bytearray(path.read_bytes())
        struct.pack_into(fmt, data, offset, value)
        path.write_bytes(data)

    def test_actual_numeric_batch_reports_known_and_unknown_counts(self):
        result = self.read()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, "")
        self.assertEqual(json.loads(result.stdout), {
            "build": 70170, "skill_lines": 154, "skill_race_class": 186,
            "skill_abilities": 7833, "unknown_ability_records": 5,
            "loadouts": 114, "loadout_items": 846, "player_admitted": False,
        })

    def test_acknowledgement_and_option_name_are_required(self):
        self.rejected(self.call())
        self.rejected(self.call("--ack-local-client-data", self.data, "--wrong"))

    def test_outside_fixture_is_rejected_before_asset_access(self):
        with tempfile.TemporaryDirectory() as outside:
            self.rejected(self.call("--ack-local-client-data", outside,
                                    "--ack-available-birth-abilities"))

    def test_complete_mode_never_falls_back_to_available_file(self):
        self.rejected(self.call("--ack-local-client-data", self.data))

    def test_all_five_table_hashes_are_checked(self):
        for name in self.names:
            with self.subTest(name=name):
                original = (self.data / name).read_bytes()
                self.modify(name, 152, 0)
                self.rejected()
                (self.data / name).write_bytes(original)

    def test_all_five_layout_hashes_are_checked(self):
        for name in self.names:
            with self.subTest(name=name):
                original = (self.data / name).read_bytes()
                self.modify(name, 156, 0)
                self.rejected()
                (self.data / name).write_bytes(original)

    def test_sparse_data_is_not_silently_regular(self):
        self.modify("SkillLine.db2", 172, 1, "<H")
        self.rejected()

    def test_truncated_plaintext_parent_data_is_rejected(self):
        path = self.data / "SkillLineAbility.available.db2"
        path.write_bytes(path.read_bytes()[:-1])
        self.rejected()

    def test_unknown_section_cannot_be_promoted(self):
        self.modify("SkillLineAbility.available.db2", 244, 0, "<Q")
        self.rejected()

    def test_parent_skill_id_cannot_overflow_uint16(self):
        # First source-verified parent lookup entry, changed only in this copy.
        self.modify("SkillLineAbility.available.db2", 127580 + 7833 * 20 + 2 + 12, 65536)
        self.rejected()


if __name__ == "__main__":
    unittest.main()
