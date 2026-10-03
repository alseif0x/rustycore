"""Private read-only GT oracle guards; actual-file Rust diff requires opt-in."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class GameTableOracle(unittest.TestCase):
    acknowledgement = "--ack-private-initial-gt-oracle"
    names = ("BaseMp.txt", "HpPerSta.txt", "xp.txt")
    values = (tuple(range(1, 16)), (-0.0,), (100, 20, 3, 4, 5))
    fingerprints = (9000784081756505640, 12161821475553763397, 8930079713595422996)
    consumer_variable = "FOREVER_INITIALIZATION_TABLES_BIN"
    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).resolve().parents[3] / "target" / "forever-login"
        cls.binary = Path(os.environ["FOREVER_INITIAL_GT_ORACLE_BIN"]).resolve(strict=True)

    def run_oracle(self, root):
        return subprocess.run([str(self.binary), self.acknowledgement, str(root)],
                              capture_output=True, text=True, timeout=5)

    def rejected(self, root):
        result = self.run_oracle(root)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, "")
        self.assertEqual(result.stderr, "Private initial GT oracle rejected\n")

    def files(self, root):
        gt = root / "gt"
        gt.mkdir()
        contents = [(name, "Level\t" + "\t".join(f"C{i}" for i in range(len(values)))
                     + "\r\nignored\t" + "\t".join(str(i) for i in values) + "\r\n")
                    for name, values in zip(self.names, self.values)]
        for name, text in contents:
            path = gt / name
            path.write_bytes(text.encode("ascii"))
            path.chmod(0o600)

    def test_requires_explicit_ack(self):
        result = subprocess.run([str(self.binary)], capture_output=True, text=True, timeout=5)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, "")

    def test_outside_root_and_directory_symlink_are_rejected(self):
        with tempfile.TemporaryDirectory() as outside:
            self.rejected(outside)
            with tempfile.TemporaryDirectory(dir=self.root) as inside:
                link = Path(inside) / "link"
                link.symlink_to(outside, target_is_directory=True)
                self.rejected(link)

    def test_positive_synthetic_bits_match_independent_fixtures(self):
        with tempfile.TemporaryDirectory(dir=self.root) as directory:
            root = Path(directory)
            self.files(root)
            result = self.run_oracle(root)
            self.assertEqual(result.returncode, 0, result.stderr)
            rows = [json.loads(line) for line in result.stdout.splitlines()]
            self.assertEqual([r["source_linux_gt_rows_including_unused_zero"] for r in rows], [2, 2, 2])
            self.assertEqual([r["source_linux_gt_fnv64"] for r in rows],
                             list(self.fingerprints))

    def test_all_tables_required_before_any_output_and_permissions_are_private(self):
        with tempfile.TemporaryDirectory(dir=self.root) as directory:
            root = Path(directory)
            self.files(root)
            file = root / "gt" / self.names[2]
            file.chmod(0o644)
            self.rejected(root)
            file.unlink()  # this test owns the synthetic file, not a client asset
            self.rejected(root)

    def test_gt_and_file_symlinks_are_rejected(self):
        with tempfile.TemporaryDirectory(dir=self.root) as directory:
            root = Path(directory)
            self.files(root)
            file = root / "gt" / self.names[2]
            file.unlink()
            file.symlink_to(root / "gt" / self.names[1])
            self.rejected(root)
        with tempfile.TemporaryDirectory(dir=self.root) as directory:
            root = Path(directory)
            (root / "gt").symlink_to(root, target_is_directory=True)
            self.rejected(root)

    @unittest.skipUnless(os.environ.get("FOREVER_ACK_PRIVATE_DATA_TESTS") == "1", "actual-file comparison requires explicit private-data opt-in")
    def test_actual_files_match_rust_numeric_bits_and_physical_rows(self):
        directory = Path(os.environ["FOREVER_CLIENT_DATA_DIRECTORY"]).resolve(strict=True)
        self.assertTrue(directory.is_relative_to(self.root.resolve()))
        cpp = self.run_oracle(directory)
        self.assertEqual(cpp.returncode, 0, cpp.stderr)
        args = [os.environ[self.consumer_variable], "--ack-local-client-data", str(directory)]
        if self.consumer_variable == "FOREVER_INITIALIZATION_TABLES_BIN" and (directory / "Map.available.db2").is_file():
            args.append("--ack-available-initial-map")
        rust = subprocess.run(args, capture_output=True, text=True, timeout=30)
        self.assertEqual(rust.returncode, 0, rust.stderr)
        rows = [json.loads(line) for line in cpp.stdout.splitlines()]
        actual = json.loads(rust.stdout)
        self.assertEqual(actual["gt_row_counts_including_unused_zero"], [r["source_linux_gt_rows_including_unused_zero"] for r in rows])
        self.assertEqual(actual["gt_numeric_bit_fingerprints"], [r["source_linux_gt_fnv64"] for r in rows])

class SpellValueGameTableOracle(GameTableOracle):
    acknowledgement = "--ack-private-spell-value-gt-oracle"
    names = ("SpellScaling.txt", "CombatRatingsMultByILvl.txt", "StaminaMultByILvl.txt")
    values = (tuple(range(1, 25)), (1, 2, 3, 4), (-0.0, 1.5, -2, 3))
    fingerprints = (3895469710102651531, 3051449540718509784, 7524576399414174216)
    consumer_variable = "FOREVER_SPELL_VALUE_TABLES_BIN"


if __name__ == "__main__":
    unittest.main()
