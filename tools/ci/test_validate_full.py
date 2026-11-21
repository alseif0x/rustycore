"""Exercise the full build checks using local Git and Cargo packages."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


RUNNER = Path(__file__).resolve().parents[1] / "validate-full"


class FullValidationTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.env = dict(os.environ, CARGO_TARGET_DIR=str(self.root / "target"), CARGO_NET_OFFLINE="true")
        self.run_cmd("git", "init", "--quiet")
        self.run_cmd("git", "config", "user.email", "validation@example.invalid")
        self.run_cmd("git", "config", "user.name", "Validation")
        self.write(".gitignore", "/target/\n")
        self.write("Cargo.toml", '[workspace]\nresolver="2"\nmembers=["crates/*"]\n')
        self.package("domain", '[features]\nfixtures=[]\n',
                     'pub fn answer() -> u8 { 2 }\n#[cfg(feature="fixtures")] pub fn setup() {}\n')
        self.package("server", '[dependencies]\ndomain={path="../domain"}\n'
                     '[dev-dependencies]\ndomain={path="../domain",features=["fixtures"]}\n',
                     '#[test] fn consumes_domain() { assert_eq!(domain::answer(), 2); }\n')
        self.run_cmd("cargo", "generate-lockfile", "--offline")
        self.run_cmd("cargo", "fmt", "--all")
        self.commit()
        self.base = self.run_cmd("git", "rev-parse", "HEAD").strip()

    def run_cmd(self, *args, expected=0):
        result = subprocess.run(args, cwd=self.root, env=self.env, text=True,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        self.assertEqual(result.returncode, expected, result.stdout)
        return result.stdout

    def write(self, name, value):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(value)

    def package(self, name, manifest, source):
        self.write(f"crates/{name}/Cargo.toml", f'[package]\nname="{name}"\nversion="0.1.0"\n'
                   f'edition="2021"\n{manifest}')
        self.write(f"crates/{name}/src/lib.rs", source)

    def commit(self):
        self.run_cmd("git", "add", ".")
        self.run_cmd("git", "commit", "-qm", "candidate")

    def build(self, *args, expected=0):
        return self.run_cmd(RUNNER, "--base", self.base, "--part", "build", *args, expected=expected)

    def test_consumer_tests_run_for_changed_dependency(self):
        path = self.root / "crates/domain/src/lib.rs"
        healthy = path.read_text()
        path.write_text(healthy.replace("    2", "    3"))
        self.commit()
        self.assertIn("consumes_domain ... FAILED", self.build(expected=101))
        path.write_text(healthy + "// Regression repaired.\n")
        self.commit()
        report = self.root / "target/result.json"
        self.build("--report", report)
        result = json.loads(report.read_text())
        self.assertEqual(result["head"], self.run_cmd("git", "rev-parse", "HEAD").strip())
        self.assertEqual(result["exit"], 0)
        self.assertIn("server", result["commands"][1]["argv"])

    def test_separate_production_configuration(self):
        self.write("crates/server/src/lib.rs", "pub fn boot() { domain::setup(); }\n")
        self.commit()
        result = self.build(expected=101)
        self.assertIn("Affected test targets: exit 0", result)
        self.assertIn("Production feature configuration: exit 101", result)

    def test_unknown_paths_are_not_accepted(self):
        self.write("new-format", "data\n")
        self.commit()
        self.assertIn("new-format", self.build(expected=2))
        self.assertIn("Write the report outside", self.build("--report", "new-format", expected=1))
        self.write("new-format", "dirty\n")
        self.assertIn("clean, committed checkout", self.build(expected=1))

    def test_source_bound_snapshot_rejects_stale_and_forged_rows(self):
        checker = RUNNER.parent / "ci/full_snapshot.py"
        path = "crates/domain/src/lib.rs"
        snapshot = "tools/architecture/persistence-access-snapshot.json"
        def write_rows(first, second):
            rows = [{"source": path, "enclosing": "fn boot", "symbol": "store", "operation": "value_alias",
                     "target": target, "fingerprint": value}
                    for target, value in (("auth", first), ("world", second))]
            self.write(snapshot, json.dumps({"schema_version": 3, "accesses": rows}))
        self.write(path, "fn boot() { let store = 1; let store = wrap(store); }\n")
        write_rows("store=1", "store=1")
        self.commit()
        base = self.run_cmd("git", "rev-parse", "HEAD").strip()
        self.write(path, "fn boot() { let store = 2; let store = wrap(store); }\n")
        write_rows("store=2", "store=2")
        self.commit()
        self.assertIn("Verified 2", self.run_cmd(sys.executable, checker, "--base", base))
        write_rows("store=2", "store=1")
        self.commit()
        self.assertIn("source-derived delta", self.run_cmd(sys.executable, checker, "--base", base, expected=1))
        write_rows("store=3", "store=3")
        self.commit()
        self.assertIn("fingerprint differs", self.run_cmd(sys.executable, checker, "--base", base, expected=1))


if __name__ == "__main__":
    unittest.main()
