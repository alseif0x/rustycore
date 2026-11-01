"""Temp-repository tests for the ignored-source check (#1241 F3-6b)."""
import contextlib
import io
import pathlib
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import ignored_sources  # noqa: E402

SHIM = "crates/wow-world/unit_tests/session/progression/skills/f3_shims.rs"


class IgnoredSourcesTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="rustycore-ignored-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name)
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        (self.root / ".gitignore").write_text("skills/\ntarget/\n*.log\n")

    def write(self, rel, text="// source\n"):
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def run_check(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            status = ignored_sources.main(["check", "--root", str(self.root)])
        return status, out.getvalue()

    def test_f3_4_shim_hidden_by_skills_rule_fails(self):
        self.write(SHIM)
        status, text = self.run_check()
        self.assertEqual(status, 1, text)
        self.assertIn(f"{SHIM}  <- .gitignore:1:skills/", text)

    def test_relocated_shim_and_non_source_ignores_pass(self):
        self.write("crates/wow-world/unit_tests/session/progression/skills_f3_shims.rs")
        self.write("crates/wow-world/target/debug/build/out.rs")      # build output
        self.write("crates/wow-world/run.log", "log\n")               # not a source suffix
        self.write("crates/wow-world/skills/notes.md", "# notes\n")   # ignored, but not source
        self.assertEqual(ignored_sources.ignored_sources(self.root), [])
        status, text = self.run_check()
        self.assertEqual(status, 0, text)
        self.assertIn("PASS", text)

    def test_ignored_sources_outside_crates_are_out_of_scope(self):
        self.write("tools/skills/helper.py")
        self.assertEqual(self.run_check()[0], 0)

    def test_not_a_repository_is_a_runner_error(self):
        with tempfile.TemporaryDirectory() as bare, contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(ignored_sources.main(["check", "--root", bare]), 2)


if __name__ == "__main__":
    unittest.main()
