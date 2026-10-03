"""Pre-acquisition guards. No assets, WoW account, database or network access."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class AcquisitionGuards(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.repo = Path(__file__).resolve().parents[3]
        cls.root = cls.repo / "target" / "forever-login"
        cls.binary = Path(os.environ["FOREVER_CLIENT_DATA_PROBE_BIN"]).resolve(strict=True)

    def rejected(self, args, message):
        result = subprocess.run([str(self.binary), *map(str, args)],
                                capture_output=True, text=True, timeout=5)
        self.assertEqual(result.returncode, 1)
        self.assertIn(message, result.stderr)
        self.assertEqual(result.stdout, "")

    def test_explicit_acquisition_ack_required(self):
        self.rejected([], "Usage:")
        self.rejected(["--wrong", self.repo, self.root / "unused", "esES"], "Usage:")

    def test_unknown_locale_before_storage_access(self):
        self.rejected(["--ack-local-client-data", "/does-not-exist",
                       self.root / "unused", "xxXX"], "Unsupported acquisition locale")

    def test_outside_private_root_is_rejected_before_storage_access(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "new-data"
            self.rejected(["--ack-local-client-data", self.repo, output, "esES"],
                          "Output must remain")
            self.assertFalse(output.exists())

    def test_similar_marker_outside_checkout_is_not_authority(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "target" / "forever-login" / "new-data"
            self.rejected(["--ack-local-client-data", self.repo, output, "esES"],
                          "Output must remain")
            self.assertFalse(output.exists())

    def test_existing_output_cannot_be_overwritten(self):
        with tempfile.TemporaryDirectory(dir=self.root) as directory:
            self.rejected(["--ack-local-client-data", self.repo, directory, "esES"],
                          "Refusing to reuse")

    def test_symlink_escape_is_rejected(self):
        with tempfile.TemporaryDirectory(dir=self.root) as inside:
            with tempfile.TemporaryDirectory() as outside:
                link = Path(inside) / "outside"
                link.symlink_to(outside, target_is_directory=True)
                output = Path(outside) / "new-data"
                self.rejected(["--ack-local-client-data", self.repo,
                               link / "new-data", "esES"], "Output must remain")
                self.assertFalse(output.exists())

    def test_optional_modes_are_explicit_and_not_repeatable(self):
        args = ["--ack-local-client-data", "/does-not-exist", self.root / "unused", "esES"]
        self.rejected([*args, "--unknown"], "Unknown optional")
        self.rejected([*args, "--ack-spell-value-game-tables", "--ack-spell-value-game-tables"],
                      "may appear once")
        self.rejected([*args, "--ack-available-achievements", "--ack-available-achievements"], "may appear once")
        self.rejected([*args, "--ack-public-tact-keys"], "requires one private file")
        self.rejected([*args, "--ack-tact-key-table", "--ack-tact-key-table"], "may appear once")
        self.rejected([*args, "--ack-character-customization-tables",
                       "--ack-character-customization-tables"], "may appear once")
        self.rejected([*args, "--ack-name-validation-tables", "--ack-name-validation-tables"],
                      "may appear once")
        self.rejected([*args, "--ack-character-initialization-tables",
                       "--ack-character-initialization-tables"], "may appear once")
        self.rejected([*args, "--ack-available-initial-map", "--ack-available-initial-map"],
                      "may appear once")
        self.rejected([*args, "--ack-character-birth-tables", "--ack-character-birth-tables"],
                      "may appear once")
        self.rejected([*args, "--ack-available-birth-abilities", "--ack-available-birth-abilities"],
                      "may appear once")
        self.rejected([*args, "--ack-available-birth-abilities"], "require birth-table")
        self.rejected([*args, "--ack-character-item-tables", "--ack-character-item-tables"],
                      "may appear once")
        self.rejected([*args, "--ack-item-template-tables", "--ack-item-template-tables"],
                      "may appear once")
        self.rejected([*args, "--ack-item-table-metadata-only", "--ack-item-table-metadata-only"],
                      "may appear once")
        self.rejected([*args, "--ack-item-table-metadata-only"], "requires item-table")
        self.rejected([*args, "--ack-available-item-tables", "--ack-available-item-tables"],
                      "may appear once")
        self.rejected([*args, "--ack-available-item-tables"], "require item-table")
        self.rejected([*args, "--ack-character-item-tables", "--ack-available-item-tables",
                       "--ack-item-table-metadata-only"], "mutually exclusive")
        self.rejected([*args, "--ack-available-initial-map"], "requires initialization-table")
        self.rejected([*args, "--ack-tact-key-table", "--ack-public-tact-keys"], "requires one private file")
        self.rejected([*args, "--ack-spell-info-tables", "--ack-spell-info-tables"],
                      "may appear once")
        self.rejected([*args, "--ack-spell-table-metadata-only", "--ack-spell-table-metadata-only"],
                      "may appear once")
        self.rejected([*args, "--ack-spell-table-metadata-only"], "requires SpellInfo-table")
        self.rejected([*args, "--ack-character-item-tables", "--ack-spell-table-metadata-only"],
                      "requires SpellInfo-table")
        self.rejected([*args, "--ack-spell-info-tables", "--ack-item-table-metadata-only"],
                      "requires item-table")
        self.rejected([*args, "--ack-available-spell-info-tables", "--ack-available-spell-info-tables"],
                      "may appear once")
        self.rejected([*args, "--ack-available-spell-info-tables"], "require SpellInfo-table")
        self.rejected([*args, "--ack-spell-info-tables", "--ack-available-spell-info-tables",
                       "--ack-spell-table-metadata-only"], "mutually exclusive")

    def test_optional_table_flags_accept_any_option_order(self):
        with tempfile.TemporaryDirectory(dir=self.root) as directory:
            directory = Path(directory)
            key_file = directory / "synthetic.txt"
            key_file.write_text("synthetic, not key material")
            key_file.chmod(0o600)
            result = subprocess.run([
                str(self.binary), "--ack-local-client-data", "/does-not-exist",
                directory / "output", "esES", "--ack-tact-key-table",
                "--ack-available-achievements", "--ack-character-customization-tables",
                "--ack-name-validation-tables", "--ack-character-initialization-tables",
                "--ack-available-initial-map",
                "--ack-character-birth-tables",
                "--ack-available-birth-abilities",
                "--ack-character-item-tables",
                "--ack-item-table-metadata-only",
                "--ack-item-template-tables",
                "--ack-spell-info-tables",
                "--ack-spell-value-game-tables",
                "--ack-spell-table-metadata-only",
                "--ack-public-tact-keys", key_file,
            ], capture_output=True, text=True, timeout=5)
            self.assertEqual(result.returncode, 1)
            self.assertNotIn("Usage:", result.stderr)
            self.assertNotIn("Unknown optional", result.stderr)
            self.assertNotIn("requires one private file", result.stderr)

    def test_key_file_outside_private_root_is_not_imported(self):
        with tempfile.TemporaryDirectory() as directory:
            key_file = Path(directory) / "synthetic.txt"
            key_file.write_text("synthetic, not key material")
            key_file.chmod(0o600)
            self.rejected(["--ack-local-client-data", "/does-not-exist", self.root / "unused", "esES",
                           "--ack-public-tact-keys", key_file], "must remain inside")

    def test_available_item_options_accept_both_orders_before_storage(self):
        with tempfile.TemporaryDirectory(dir=self.root) as directory:
            args = ["--ack-local-client-data", self.repo, directory, "esES"]
            for flags in [("--ack-character-item-tables", "--ack-available-item-tables"),
                          ("--ack-available-item-tables", "--ack-character-item-tables")]:
                self.rejected([*args, *flags], "Refusing to reuse")

    def test_template_metadata_options_accept_both_orders_before_storage(self):
        with tempfile.TemporaryDirectory(dir=self.root) as directory:
            args = ["--ack-local-client-data", self.repo, directory, "esES"]
            for flags in [("--ack-item-template-tables", "--ack-item-table-metadata-only"),
                          ("--ack-item-table-metadata-only", "--ack-item-template-tables")]:
                self.rejected([*args, *flags], "Refusing to reuse")

    def test_key_file_permissions_size_and_symlink_guards(self):
        with tempfile.TemporaryDirectory(dir=self.root) as directory:
            key_file = Path(directory) / "synthetic.txt"
            args = ["--ack-local-client-data", "/does-not-exist", self.root / "unused", "esES", "--ack-public-tact-keys"]
            key_file.write_text("synthetic, not key material")
            key_file.chmod(0o644)
            self.rejected([*args, key_file], "mode 0600")
            key_file.write_text(""); key_file.chmod(0o600)
            self.rejected([*args, key_file], "size is outside")
            link = Path(directory) / "link.txt"; link.symlink_to(key_file)
            self.rejected([*args, link], "regular non-symlink")

    def test_spell_metadata_options_accept_both_orders_before_storage(self):
        with tempfile.TemporaryDirectory(dir=self.root) as directory:
            args = ["--ack-local-client-data", self.repo, directory, "esES"]
            for flags in [("--ack-spell-info-tables", "--ack-spell-table-metadata-only"),
                          ("--ack-spell-table-metadata-only", "--ack-spell-info-tables"),
                          ("--ack-spell-info-tables",)]:
                self.rejected([*args, *flags], "Refusing to reuse")

    def test_available_spells_require_exact_observed_locale_before_storage(self):
        args = ["--ack-local-client-data", "/does-not-exist", self.root / "unused", "enUS",
                "--ack-spell-info-tables", "--ack-available-spell-info-tables"]
        self.rejected(args, "require observed esES")
        with tempfile.TemporaryDirectory(dir=self.root) as directory:
            for flags in [("--ack-spell-info-tables", "--ack-available-spell-info-tables"),
                          ("--ack-available-spell-info-tables", "--ack-spell-info-tables")]:
                self.rejected(["--ack-local-client-data", self.repo, directory, "esES", *flags],
                              "Refusing to reuse")


if __name__ == "__main__":
    unittest.main()
