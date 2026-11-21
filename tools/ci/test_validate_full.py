"""Exercise the full build checks using local Git and Cargo packages."""
import json
import fcntl
import os
from pathlib import Path
import runpy
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

    def scan_initializer(self, source, function="run_inner", name="session_resources"):
        manifest = RUNNER.parent / "full-validation/Cargo.toml"
        result = subprocess.run(
            ["cargo", "run", "--quiet", "--locked", "--manifest-path", manifest,
             "--", function, name],
            cwd=self.root,
            env=self.env,
            input=source,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        self.assertEqual(result.returncode, 0, result.stdout)
        fields = result.stdout.split("\0")
        self.assertTrue(fields and fields[-1] == "", result.stdout)
        fields.pop()
        self.assertEqual(len(fields), 2, result.stdout)
        return fields[0]

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
        report = self.root / "target/report.json"
        self.assertIn("new-format", self.build("--report", report, expected=2))
        self.assertEqual(json.loads(report.read_text())["uncovered"], ["new-format"])
        self.assertIn("Write the report outside", self.build("--report", "new-format", expected=1))
        self.write("new-format", "dirty\n")
        self.assertIn("clean, committed checkout", self.build("--report", report, expected=1))
        self.assertEqual(json.loads(report.read_text())["status"], "incomplete")

    def test_concurrent_execution_refuses_the_same_checkout(self):
        with (self.root / ".git/validate-full.lock").open("w") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            self.assertIn("already running", self.build(expected=1))

    def test_global_inventory_runs_even_on_an_unchanged_tree(self):
        args = [RUNNER, "--base", "HEAD", "--plan"]
        self.assertIn("Empty candidate diff", self.run_cmd(*args, expected=1))
        all_checks = self.run_cmd(*args, "--workspace")
        for command in ("--workspace --all-targets", "verify-required loot-single-item-claim",
                        "verify-required creature-spell-casting", "--bin handler-contract-check",
                        "--bin session-ownership-check -- check"):
            self.assertIn(command, all_checks)
        inventory = self.run_cmd(*args, "--workspace", "--part", "inventory")
        self.assertIn("Full persistence inventory", inventory)
        self.assertNotIn("Workspace test targets", inventory)
        compositor = self.run_cmd(*args, "--workspace", "--part", "compositor")
        self.assertLess(compositor.index("cargo fetch --locked"), compositor.index("check_sql_compositor.py"))
        self.assertIn("requires --workspace", self.run_cmd(*args, "--part", "inventory", expected=2))

    def test_modules_lock_only_compositor_fetches_before_offline_sql(self):
        self.write("modules.lock.toml", "")
        self.commit()
        plan = self.run_cmd(RUNNER, "--base", self.base, "--part", "compositor", "--plan")
        self.assertLess(plan.index("cargo fetch --locked"), plan.index("check_sql_compositor.py"))

    def test_workspace_production_checks_independent_binary_without_test_cfg(self):
        self.write("crates/launcher/Cargo.toml",
                   '[package]\nname="launcher"\nversion="0.1.0"\nedition="2021"\n')
        self.write("crates/launcher/src/main.rs",
                   '#[cfg(not(test))]\ncompile_error!("production-only launcher failure");\nfn main() {}\n')
        self.write("tools/wow-test-bot/Cargo.toml",
                   '[package]\nname="fixture-bot"\nversion="0.1.0"\nedition="2021"\n\n[workspace]\n')
        self.write("tools/wow-test-bot/src/main.rs", "fn main() {}\n")
        self.run_cmd("cargo", "generate-lockfile", "--offline")
        self.commit()

        plan = self.run_cmd(RUNNER, "--base", self.base, "--workspace", "--part", "production", "--plan")
        self.assertIn("cargo check --locked --workspace", plan)
        self.run_cmd("cargo", "test", "--locked", "-p", "launcher", "--all-targets")
        result = self.run_cmd(RUNNER, "--base", self.base, "--workspace", "--part", "production", expected=101)
        self.assertIn("Workspace production targets: exit 101", result)
        self.assertIn("production-only launcher failure", result)

    def test_hosted_partitions_cover_the_serial_commands_once(self):
        path = self.root / "crates/domain/src/lib.rs"
        path.write_text(path.read_text() + "// Test the same candidate on each worker.\n")
        self.commit()
        def planned(part):
            result = self.run_cmd(RUNNER, "--base", self.base, "--part", part, "--plan")
            return set(result.splitlines()[1:])
        tests, production, compositor = (planned(part) for part in ("tests", "production", "compositor"))
        self.assertTrue(any("cargo test --locked" in step for step in tests))
        self.assertTrue(any("cargo check --locked" in step for step in production))
        self.assertTrue(tests.isdisjoint(production))
        self.assertEqual(tests | production | compositor, planned("build"))
        self.write("modules.lock.toml", "")
        self.commit()
        self.assertTrue(any("check_sql_compositor.py" in row for row in planned("compositor")))
        self.assertTrue(planned("production").isdisjoint(planned("compositor")))

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

    def test_config_bool_fields_project_out_but_preserve_sql_inputs(self):
        checker = RUNNER.parent / "ci/full_snapshot.py"
        path = "crates/world-server/src/app.rs"
        self.write("crates/world-server/src/bootstrap/config.rs",
                   "pub fn world_config_bool(_: &(), _: &str, default: bool) -> bool { default }\n")
        snapshot = "tools/architecture/persistence-access-snapshot.json"
        targets = ("LoginDatabase", "WorldDatabase", "CharacterDatabase", "HotfixDatabase")

        def write_rows(fingerprint):
            rows = [{"source": path, "enclosing": "fn run_inner", "symbol": "session_resources",
                     "operation": "value_alias", "target": target, "fingerprint": fingerprint}
                    for target in targets]
            self.write(snapshot, json.dumps({"schema_version": 3, "accesses": rows}))

        before = '''fn run_inner() {
            let session_resources = SessionRuntimePolicyCapabilitiesLikeCpp {
                persistence: world_db.execute("SELECT id FROM characters"),
            };
        }
        '''
        self.write(path, before)
        write_rows(self.scan_initializer(before))
        self.commit()
        base = self.run_cmd("git", "rev-parse", "HEAD").strip()

        after = '''fn run_inner() {
            let session_resources = SessionRuntimePolicyCapabilitiesLikeCpp {
                persistence: world_db.execute("SELECT id FROM characters"),
                allow_two_side_interaction_auction: world_config_bool(
                    &world_configs,
                    "CONFIG_ALLOW_TWO_SIDE_INTERACTION_AUCTION",
                    true,
                ),
                use_new_feature: world_config_bool(
                    &world_configs,
                    "CONFIG_UNRELATED_FEATURE",
                    false,
                ),
            };
        }
        '''
        self.write(path, after)
        write_rows(self.scan_initializer(after))
        self.commit()
        self.assertIn("Verified 4", self.run_cmd(sys.executable, checker, "--base", base))

    def test_matching_spelling_cannot_hide_a_stale_database_target(self):
        checker = RUNNER.parent / "ci/full_snapshot.py"
        path = "crates/world-server/src/app.rs"
        snapshot = "tools/architecture/persistence-access-snapshot.json"

        def write_row(fingerprint):
            row = {"source": path, "enclosing": "fn run_inner", "symbol": "session_resources",
                   "operation": "value_alias", "target": "WorldDatabase", "fingerprint": fingerprint}
            self.write(snapshot, json.dumps({"schema_version": 3, "accesses": [row]}))

        before = "fn run_inner() { let session_resources = SessionRuntimePolicyCapabilitiesLikeCpp { persistence: world_db, }; }\n"
        self.write(path, before)
        write_row(self.scan_initializer(before))
        self.commit()
        base = self.run_cmd("git", "rev-parse", "HEAD").strip()
        after = "fn run_inner() { let session_resources = SessionRuntimePolicyCapabilitiesLikeCpp { persistence: hotfix_db, }; }\n"
        self.write(path, after)
        write_row(self.scan_initializer(after))
        self.commit()
        result = self.run_cmd(sys.executable, checker, "--base", base, expected=2)
        self.assertIn("persistence producer or operand changed", result)

    def test_sql_operand_change_requires_exhaustive_inventory(self):
        checker = RUNNER.parent / "ci/full_snapshot.py"
        path = "crates/world-server/src/app.rs"
        snapshot = "tools/architecture/persistence-access-snapshot.json"

        def write_row(fingerprint):
            row = {"source": path, "enclosing": "fn run_inner", "symbol": "session_resources",
                   "operation": "value_alias", "target": "WorldDatabase", "fingerprint": fingerprint}
            self.write(snapshot, json.dumps({"schema_version": 3, "accesses": [row]}))

        before = '''fn run_inner() {
            let session_resources = SessionRuntimePolicyCapabilitiesLikeCpp {
                persistence: world_db.execute("SELECT id FROM characters"),
            };
        }
        '''
        self.write(path, before)
        write_row(self.scan_initializer(before))
        self.commit()
        base = self.run_cmd("git", "rev-parse", "HEAD").strip()
        after = before.replace("SELECT id FROM characters", "DELETE FROM characters")
        self.write(path, after)
        write_row(self.scan_initializer(after))
        self.commit()
        result = self.run_cmd(sys.executable, checker, "--base", base, expected=2)
        self.assertIn("persistence producer or operand changed", result)


    def test_capture_artifacts_require_their_own_contract(self):
        flow = "crates/capture-diff/flows/sample"
        self.package("capture-diff", "", "pub fn fixture() {}\n")
        self.run_cmd("cargo", "generate-lockfile", "--offline")
        self.write(f"{flow}/sample.pkt", "fixture")
        self.commit()
        self.assertIn("Coverage required", self.build("--plan", expected=2))
        self.write(f"{flow}/requirement.json", "{}")
        self.commit()
        self.assertIn("verify-required sample", self.build("--plan"))

    def test_unclassified_markdown_cannot_get_a_links_only_verdict(self):
        self.write("new-inputs/fixture.md", "build input\n")
        self.commit()
        self.assertIn("Coverage required", self.build("--plan", expected=2))

    def test_established_markdown_path_coverage_is_complete(self):
        repository = RUNNER.parent.parent
        paths = subprocess.check_output(
            ["git", "ls-files", "-z", "--", "*.md"], cwd=repository
        ).split(b"\0")
        is_established = runpy.run_path(str(RUNNER))["established_markdown"]
        unowned = [path.decode() for path in paths if path and not is_established(path.decode())]
        self.assertEqual(unowned, [])

    def test_established_markdown_uses_repository_wide_link_check(self):
        self.write("README.md", "# Root\n")
        self.write("docs/architecture/new-contract.md", "[Guide](../README.md)\n")
        self.commit()

        plan = self.build("--plan")
        self.assertIn("Repository Markdown links:", plan)
        self.assertIn("README.md", plan)
        self.assertIn("docs/architecture/new-contract.md", plan)
        self.assertNotIn("Coverage required", plan)

    def test_workflow_contract_test_is_covered_when_it_changes_alone(self):
        self.write("tools/test_validation_workflows.py", "# workflow contract fixture\n")
        self.commit()

        plan = self.build("--plan")
        self.assertIn("Workflow contracts:", plan)
        self.assertNotIn("Coverage required", plan)

    def test_document_link_checker_tests_are_selected_with_the_checker(self):
        self.write("tools/ci/check_doc_links.py", "# checker fixture\n")
        self.commit()

        plan = self.build("--plan")
        self.assertIn("Markdown link checker regression tests:", plan)
        self.assertNotIn("Coverage required", plan)


if __name__ == "__main__":
    unittest.main()
