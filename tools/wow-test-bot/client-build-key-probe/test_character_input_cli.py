"""Native negative CLI guards; never enter names or touch an official client."""
import os
import subprocess
import unittest
from pathlib import Path


class CharacterInputCliTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.binary = os.environ["FOREVER_CHARACTER_INPUT_EXE"]
        cls.wine = os.environ["FOREVER_PROBE_WINE"]
        prefix = Path(os.environ["WINEPREFIX"]).resolve(strict=True)
        if "target/forever-login/wine-prefix" not in prefix.as_posix():
            raise ValueError("only the isolated prefix is permitted")

    def reject(self, arguments, expected):
        result = subprocess.run([self.wine, self.binary, "--character-name-input", *arguments],
                                capture_output=True, text=True, timeout=15)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(expected, result.stdout + result.stderr)
        self.assertNotIn("Fixture names entered", result.stdout)

    def test_wrong_arity(self):
        self.reject([], "Usage: --character-name-input")

    def test_non_fixture_executable(self):
        self.reject([r"C:\not-isolated\WowB.exe", "Test", "Surname"], "Only the isolated")

    def test_invalid_first_name(self):
        for value in ("", "A", "Ab1", "A" * 13, "Áb"):
            with self.subTest(length=len(value)):
                self.reject([r"C:\not-isolated\WowB.exe", value, "Surname"], "Fixture names must")

    def test_invalid_surname(self):
        for value in ("A", "Ab1", "A" * 13, "Áb"):
            with self.subTest(length=len(value)):
                self.reject([r"C:\not-isolated\WowB.exe", "Test", value], "Fixture names must")

    def test_empty_surname_does_not_skip_isolation_guard(self):
        self.reject([r"C:\not-isolated\WowB.exe", "Test", ""], "Only the isolated")


if __name__ == "__main__":
    unittest.main()
