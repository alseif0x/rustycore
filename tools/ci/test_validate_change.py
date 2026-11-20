"""Exercise the pilot through its CLI with Git and local-only Cargo fixtures."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


RUNNER = Path(__file__).resolve().parents[1] / "validate-change"


class PilotTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.repo = Path(tmp.name)
        self.env = dict(os.environ, CARGO_TARGET_DIR=str(self.repo / "target"))
        self.command("git", "init", "-q")
        self.command("git", "config", "user.email", "pilot@example.invalid")
        self.command("git", "config", "user.name", "Pilot")
        self.write(".gitignore", "/target\n")
        self.write("Cargo.toml", '[workspace]\nmembers=["crates/*"]\nresolver="2"\n')
        self.package("core-lib", '[features]\nfixtures=[]\n',
                     '#[cfg(feature="fixtures")] pub fn fixture() {}\npub fn production() {}\n')
        self.package("adapter", '[dependencies]\ncore-lib={path="../core-lib"}\n'
                     '[features]\nfixtures=["core-lib/fixtures"]\n',
                     '#[cfg(feature="fixtures")] pub use core_lib::fixture;\n')
        self.package("consumer", '[dependencies]\nadapter={path="../adapter"}\n'
                     '[dev-dependencies]\nadapter={path="../adapter",features=["fixtures"]}\n',
                     '#[test] fn sees_fixture() { adapter::fixture(); }\n')
        self.write("docs/operations/pilot.md", "[Source](../../Cargo.toml)\n")
        self.command("cargo", "generate-lockfile", "--offline")
        self.command("cargo", "fmt", "--all")
        self.command("git", "add", ".")
        self.command("git", "commit", "-qm", "fixture")

    def command(self, *args, expected=0):
        r = subprocess.run(args, cwd=self.repo, env=self.env, text=True,
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        self.assertEqual(r.returncode, expected, r.stdout)
        return r.stdout

    def write(self, path, value):
        path = self.repo / path
        path.parent.mkdir(exist_ok=True, parents=True)
        path.write_text(value)

    def package(self, name, config, source):
        self.write(f"crates/{name}/Cargo.toml", f'[package]\nname="{name}"\n'
                   f'version="0.1.0"\nedition="2021"\n{config}')
        self.write(f"crates/{name}/src/lib.rs", source)

    def pilot(self, *args, expected=0):
        return self.command(RUNNER, "--base", "HEAD", "--working", *args, expected=expected)

    def test_docs_and_unknown_inputs(self):
        self.write("docs/operations/pilot.md", "[Source](../../Cargo.toml)\nEdited\n")
        self.assertNotIn("cargo test", self.pilot())
        self.write("docs/operations/pilot.md", "[Source](missing.md)\n")
        self.assertIn("missing.md", self.pilot(expected=1))
        self.write("new.sql", "SELECT 1;\n")
        self.assertIn("new.sql", self.pilot("--plan", expected=2))

    def test_forwarding_failure_in_downstream_test(self):
        path = self.repo / "crates/adapter/Cargo.toml"
        path.write_text(path.read_text().replace('fixtures=["core-lib/fixtures"]', "fixtures=[]"))
        self.assertIn("core_lib::fixture", self.pilot(expected=101))
        self.command("git", "restore", str(path))
        path.write_text(path.read_text() + "\n")
        self.assertIn("Additional acceptance required", self.pilot(expected=2))

    def test_downstream_and_unmatched_focus(self):
        self.write("crates/adapter/src/lib.rs", "// Removed fixture API\n")
        self.assertIn("cannot find function", self.pilot(expected=101))
        self.assertIn("matched no passing tests", self.pilot("--focus", "absent", expected=1))

    def test_scope_uses_the_candidate_actually_on_disk(self):
        self.assertIn("Empty change set", self.pilot(expected=1))
        path = "docs/operations/pilot.md"
        self.write(path, "[Broken](does-not-exist.md)\n")
        self.command("git", "add", path)
        self.assertIn("pass --working", self.command(RUNNER, "--base", "HEAD", expected=1))
        self.command("git", "commit", "-qm", "broken link")
        self.assertIn("does-not-exist.md", self.command(RUNNER, "--base", "HEAD^", expected=1))
        self.command("git", "restore", "--source=HEAD^", "--worktree", path)
        self.assertIn("Empty change set", self.command(RUNNER, "--base", "HEAD^", "--working", expected=1))

    def test_production_consumer_without_its_test_features(self):
        source = "crates/consumer/src/lib.rs"
        self.write(source, "pub fn needs_fixture() { adapter::fixture(); }\n")
        self.command("cargo", "test", "--offline", "--all-targets", "-p", "consumer")
        output = self.pilot(expected=101)
        self.assertIn("Production builds of owners and consumers: exit 101", output)
        self.assertIn("cannot find function", output)

    def test_focused_feedback_on_a_shared_diff(self):
        for pkg in ("core-lib", "adapter"):
            with (self.repo / f"crates/{pkg}/src/lib.rs").open("a") as source:
                source.write("// edit\n")
        output = self.pilot("--focus", "sees_fixture", "--package", "consumer")
        self.assertIn("1 passed", output)
        self.assertIn("Focused editing feedback only", output)
        self.assertIn("outside the affected packages", self.pilot("--focus", "x", "--package", "other", expected=1))


if __name__ == "__main__":
    unittest.main()
