"""Explicit local-data negative QA for the production Rust ID catalog.

Only private disposable copies are changed. No source installation, account,
database or network access. This is not part of asset-free regression discovery.
"""
import os
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import unittest


class TargetCatalogGuards(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if os.environ.get("FOREVER_ACK_PRIVATE_DATA_TESTS") != "1":
            raise RuntimeError("Explicit private-data QA acknowledgement required")
        cls.root = Path(__file__).resolve().parents[3] / "target" / "forever-login"
        cls.source = Path(os.environ["FOREVER_CLIENT_DATA_DIRECTORY"]).resolve(strict=True)
        if not cls.source.is_relative_to(cls.root.resolve(strict=True)):
            raise RuntimeError("Only acquired private fixture data is admitted")
        cls.binary = Path(os.environ["FOREVER_CHARACTER_TABLES_BIN"]).resolve(strict=True)

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(dir=self.root, prefix="catalog-negative-")
        self.addCleanup(self.temporary.cleanup)
        self.data = Path(self.temporary.name)
        for name in ["ChrClasses.db2", "ChrRaces.db2"]:
            shutil.copy2(self.source / name, self.data / name)
            (self.data / name).chmod(0o600)

    def modify(self, name, offset, value, fmt="<I"):
        path = self.data / (name + ".db2")
        data = bytearray(path.read_bytes())
        struct.pack_into(fmt, data, offset, value)
        path.write_bytes(data)

    def rejected(self):
        result = subprocess.run([str(self.binary), "--ack-local-client-data", str(self.data)],
                                capture_output=True, text=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")

    def test_wrong_table(self):
        self.modify("ChrClasses", 152, 0)
        self.rejected()

    def test_wrong_layout(self):
        self.modify("ChrRaces", 156, 0)
        self.rejected()

    def test_wrong_inline_id_source(self):
        self.modify("ChrClasses", 174, 30, "<H")
        self.rejected()

    def test_truncated_header(self):
        path = self.data / "ChrClasses.db2"
        path.write_bytes(path.read_bytes()[:203])
        self.rejected()

    def test_old_empty_wdc4_is_not_target_data(self):
        data = bytearray(72)
        data[:4] = b"WDC4"
        (self.data / "ChrClasses.db2").write_bytes(data)
        self.rejected()

    def test_sparse_is_not_silently_read_as_regular(self):
        self.modify("ChrClasses", 172, 1, "<H")
        self.rejected()

    def test_encrypted_section_is_not_silently_zero_filled(self):
        self.modify("ChrClasses", 204, 1, "<Q")
        self.rejected()

    def test_external_id_list_truncated(self):
        self.modify("ChrRaces", 204 + 24, 228)
        self.rejected()

    def test_missing_copy_source_is_not_silently_discarded(self):
        self.modify("ChrClasses", 204 + 36, 1)
        path = self.data / "ChrClasses.db2"
        path.write_bytes(path.read_bytes() + struct.pack("<II", 99, 0))
        self.rejected()


if __name__ == "__main__":
    unittest.main()
