#!/usr/bin/env python3
"""Integration tests for the module manager (issue #230).

Each test drives the real CLI in a temporary copy of the repository skeleton,
so nothing here touches the developer's own `modules/` checkout and no test
reaches the network.
"""
from __future__ import annotations

import json
import pathlib
import shutil
import subprocess
import sys
import tempfile
import tomllib
import unittest

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
CLI = REPO_ROOT / "tools/modules/rustycore-module"


class Workspace:
    """A throwaway repo root with just enough structure for the CLI."""

    def __init__(self, root: pathlib.Path):
        self.root = root
        (root / "tools/modules").mkdir(parents=True)
        (root / "crates").mkdir()
        for name in ("compose.py", "rustycore-module"):
            shutil.copy2(REPO_ROOT / "tools/modules" / name, root / "tools/modules" / name)
        shutil.copytree(REPO_ROOT / "tools/modules/skeleton", root / "tools/modules/skeleton")
        shutil.copytree(REPO_ROOT / "crates/wow-module-api", root / "crates/wow-module-api")
        shutil.copytree(REPO_ROOT / "crates/wow-core", root / "crates/wow-core")
        (root / "modules").mkdir()
        (root / "Cargo.toml").write_text("[workspace]\nmembers = []\n", encoding="utf-8")

    def run(self, *args: str) -> subprocess.CompletedProcess:
        return subprocess.run(
            [sys.executable, str(self.root / "tools/modules/rustycore-module"), *args],
            cwd=self.root, capture_output=True, text=True, check=False,
        )


class ModuleManagerTests(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.ws = Workspace(pathlib.Path(self._tmp.name))

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_full_new_check_sync_remove_workflow(self) -> None:
        created = self.ws.run("new", "demo.greeter", "--json")
        self.assertEqual(created.returncode, 0, created.stderr)
        self.assertEqual(json.loads(created.stdout)["path"], "modules/demo_greeter")

        listed = json.loads(self.ws.run("list", "--json").stdout)
        self.assertEqual([m["id"] for m in listed["modules"]], ["demo.greeter"])

        self.assertEqual(self.ws.run("sync").returncode, 0)
        self.assertEqual(self.ws.run("check").returncode, 0)
        lock = (self.ws.root / "modules.lock.toml").read_text(encoding="utf-8")
        self.assertIn('id = "demo.greeter"', lock)
        self.assertIn("enabled_order = 0", lock)

        removed = self.ws.run("remove", "demo.greeter", "--json")
        self.assertEqual(removed.returncode, 0, removed.stderr)
        self.assertEqual(self.ws.run("sync").returncode, 0)
        self.assertNotIn("demo.greeter", (self.ws.root / "modules.lock.toml").read_text())
        self.assertFalse((self.ws.root / "modules/demo_greeter").exists())

    def test_two_modules_compose_in_declared_operator_order(self) -> None:
        self.ws.run("new", "zulu.module")
        self.ws.run("new", "alpha.module")
        # Declared order must beat alphabetical order.
        manifest = self.ws.root / "modules/zulu_module/module.toml"
        manifest.write_text(
            manifest.read_text().replace('id = "zulu.module"', 'id = "zulu.module"\norder = -1'),
            encoding="utf-8",
        )
        self.assertEqual(self.ws.run("sync").returncode, 0)
        main = (self.ws.root / "crates/world-modules/src/main.rs").read_text(encoding="utf-8")
        self.assertLess(main.index("zulu_module::register"), main.index("alpha_module::register"))

    def test_documented_exit_codes(self) -> None:
        self.ws.run("new", "demo.greeter")
        for args, expected in (
            (("new", "BAD.Id"), 1),
            (("install", "--path", "x", "--git", "y"), 1),
            (("remove", "nope.module"), 2),
            (("update", "demo_greeter"), 3),
            (("install", "--path", "modules/demo_greeter", "--dir", "demo_greeter"), 4),
        ):
            with self.subTest(args=args):
                self.assertEqual(self.ws.run(*args).returncode, expected)

    def test_errors_are_machine_readable(self) -> None:
        failure = self.ws.run("remove", "nope.module", "--json")
        payload = json.loads(failure.stderr)
        self.assertEqual(payload["exit_code"], 2)
        self.assertEqual(payload["command"], "remove")
        self.assertIn("nope.module", payload["error"])

    def test_install_from_a_local_path_and_reject_a_duplicate_id(self) -> None:
        self.ws.run("new", "demo.greeter", "--dir", "source_copy")
        installed = self.ws.run("install", "--path", "modules/source_copy", "--dir", "second")
        self.assertEqual(installed.returncode, 1, installed.stdout)
        self.assertIn("duplicate module id", installed.stderr)
        self.assertFalse((self.ws.root / "modules/second").exists(),
                         "a rejected install must leave nothing behind")

    def test_remove_refuses_to_escape_the_modules_directory(self) -> None:
        escaped = self.ws.run("remove", "../crates")
        self.assertNotEqual(escaped.returncode, 0)
        self.assertTrue((self.ws.root / "crates").is_dir())

    def test_doctor_reports_a_missing_workspace_member(self) -> None:
        self.ws.run("new", "demo.greeter")
        self.ws.run("sync")
        report = json.loads(self.ws.run("doctor", "--json").stdout)
        self.assertTrue(
            any("workspace member" in f for f in report["findings"]),
            report,
        )


    def test_operator_overrides_live_outside_the_module_and_change_the_digest(self) -> None:
        self.ws.run("new", "demo.greeter")
        before = json.loads(self.ws.run("list", "--json").stdout)["modules"][0]["config_digest"]

        conf = self.ws.root / "conf/modules"
        conf.mkdir(parents=True)
        (conf / "demo.greeter.toml").write_text(
            'welcome_text = "Overridden"\n', encoding="utf-8"
        )
        after = json.loads(self.ws.run("list", "--json").stdout)["modules"][0]["config_digest"]
        self.assertNotEqual(before, after, "an override must change the digest")

        self.assertEqual(self.ws.run("sync").returncode, 0)
        main = (self.ws.root / "crates/world-modules/src/main.rs").read_text(encoding="utf-8")
        self.assertIn("Overridden", main, "the override must reach the compositor")
        self.assertNotIn(
            "Overridden",
            (self.ws.root / "modules/demo_greeter/module.toml").read_text(encoding="utf-8"),
            "an override must never be written into the module checkout",
        )

    def test_the_digest_matches_the_rust_implementation(self) -> None:
        """Pinned in `wow-module-api`'s own tests; both sides must agree."""
        sys.path.insert(0, str(REPO_ROOT / "tools/modules"))
        import compose  # noqa: PLC0415

        self.assertEqual(compose.config_digest({}), "fnv1a64:cbf29ce484222325")
        self.assertEqual(
            compose.config_digest(
                {"enabled": True, "welcome_text": "Overridden by the operator"}
            ),
            "fnv1a64:1144d896121f347b",
        )

    def test_an_incompatible_source_api_is_refused_before_activation(self) -> None:
        shutil.copytree(
            REPO_ROOT / "tools/modules/fixtures/incompatible_api",
            self.ws.root / "modules/incompatible_api",
        )
        refused = self.ws.run("check")
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn("source_api", refused.stderr)
        self.assertIn("Update the module or pin an older server", refused.stderr)

    def test_a_module_depending_on_a_server_crate_is_refused(self) -> None:
        """ADR-002 / #1445 S0: `wow-module-api` is the only server crate allowed."""
        shutil.copytree(
            REPO_ROOT / "tools/modules/fixtures/forbidden_dependency",
            self.ws.root / "modules/forbidden_dependency",
        )
        refused = self.ws.run("check")
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn("depends on server crate 'wow-core'", refused.stderr)
        self.assertIn("only on wow-module-api", refused.stderr)

    def test_a_workspace_inherited_server_crate_is_refused_too(self) -> None:
        self.ws.run("new", "demo.greeter")
        root = self.ws.root / "Cargo.toml"
        root.write_text(
            root.read_text(encoding="utf-8")
            + '\n[workspace.dependencies]\nwow-core = { path = "crates/wow-core" }\n',
            encoding="utf-8",
        )
        cargo = self.ws.root / "modules/demo_greeter/Cargo.toml"
        cargo.write_text(
            cargo.read_text(encoding="utf-8").replace(
                "[dependencies]\n", "[dependencies]\nwow-core = { workspace = true }\n", 1
            ),
            encoding="utf-8",
        )
        refused = self.ws.run("sync")
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn("'wow-core'", refused.stderr)

    def test_a_server_crate_named_without_a_path_is_refused(self) -> None:
        self.ws.run("new", "demo.greeter")
        cargo = self.ws.root / "modules/demo_greeter/Cargo.toml"
        cargo.write_text(
            cargo.read_text(encoding="utf-8").replace(
                "[dependencies]\n", '[dependencies]\nwow-core = "0.1"\n', 1
            ),
            encoding="utf-8",
        )
        refused = self.ws.run("check")
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn("depends on server crate 'wow-core'", refused.stderr)

    def test_a_helper_crate_inside_the_module_cannot_smuggle_a_server_crate(self) -> None:
        self.ws.run("new", "demo.greeter")
        module = self.ws.root / "modules/demo_greeter"
        helper = module / "helper"
        (helper / "src").mkdir(parents=True)
        (helper / "src/lib.rs").write_text("", encoding="utf-8")
        (helper / "Cargo.toml").write_text(
            '[package]\nname = "demo-helper"\nversion = "0.1.0"\nedition = "2024"\n\n'
            '[dependencies]\nnot-a-server-name = { package = "wow-core", '
            'path = "../../../crates/wow-core" }\n',
            encoding="utf-8",
        )
        cargo = module / "Cargo.toml"
        cargo.write_text(
            cargo.read_text(encoding="utf-8").replace(
                "[dependencies]\n", '[dependencies]\ndemo-helper = { path = "helper" }\n', 1
            ),
            encoding="utf-8",
        )
        refused = self.ws.run("check")
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn("'wow-core'", refused.stderr)

        # The same helper without the server dependency is fine.
        (helper / "Cargo.toml").write_text(
            '[package]\nname = "demo-helper"\nversion = "0.1.0"\nedition = "2024"\n',
            encoding="utf-8",
        )
        self.assertEqual(self.ws.run("sync").returncode, 0)

    def test_dev_dependencies_and_api_one_modules_still_compose(self) -> None:
        """The skeleton is a source API 1 module; a dev-dependency is not checked."""
        self.ws.run("new", "demo.greeter")
        cargo = self.ws.root / "modules/demo_greeter/Cargo.toml"
        cargo.write_text(
            cargo.read_text(encoding="utf-8")
            + '\n[dev-dependencies]\nwow-core = { path = "../../crates/wow-core" }\n',
            encoding="utf-8",
        )
        self.assertEqual(self.ws.run("sync").returncode, 0)
        lock = tomllib.loads((self.ws.root / "modules.lock.toml").read_text(encoding="utf-8"))
        self.assertEqual(lock["source_api"], "2", "the server provides API 2")
        self.assertEqual(lock["module"][0]["source_api"], "1", "the module keeps its own API")

    def test_the_example_milestones_module_installs_and_composes(self) -> None:
        """#1444 H5: the shipped example is a valid source API 2 checkout."""
        example = self.ws.root / "examples/milestones"
        shutil.copytree(REPO_ROOT / "tools/modules/examples/milestones", example)
        installed = self.ws.run("install", "--path", "examples/milestones", "--json")
        self.assertEqual(installed.returncode, 0, installed.stderr)
        self.assertEqual(json.loads(installed.stdout)["id"], "example.milestones")
        self.assertEqual(self.ws.run("sync").returncode, 0)
        lock = tomllib.loads((self.ws.root / "modules.lock.toml").read_text(encoding="utf-8"))
        self.assertEqual(lock["module"][0]["source_api"], "2")
        compositor = (self.ws.root / "crates/world-modules/src/main.rs").read_text(encoding="utf-8")
        self.assertIn("example_milestones::register(&mut modules, config)", compositor)
        self.assertEqual(self.ws.run("check").returncode, 0)

    def test_the_composer_and_the_module_api_agree_on_source_apis(self) -> None:
        sys.path.insert(0, str(REPO_ROOT / "tools/modules"))
        import compose  # noqa: PLC0415

        lib = (REPO_ROOT / "crates/wow-module-api/src/lib.rs").read_text(encoding="utf-8")
        self.assertIn(f'pub const SOURCE_API: &str = "{compose.SUPPORTED_SOURCE_API}";', lib)
        rendered = ", ".join(f'"{api}"' for api in sorted(compose.COMPATIBLE_SOURCE_APIS))
        self.assertIn(f"pub const COMPATIBLE_SOURCE_APIS: &[&str] = &[{rendered}];", lib)

    def test_zero_modules_and_no_config_keep_the_base_behaviour(self) -> None:
        self.assertEqual(self.ws.run("sync").returncode, 0)
        main = (self.ws.root / "crates/world-modules/src/main.rs").read_text(encoding="utf-8")
        manifest = (self.ws.root / "crates/world-modules/Cargo.toml").read_text(encoding="utf-8")
        self.assertIn("No modules are installed", main)
        self.assertNotIn("ModuleConfig::new", main)
        self.assertIn(
            "test = false",
            manifest,
            "the generated launcher has no unit tests and must not link a libtest harness",
        )

    def test_workspace_lint_inheritance_is_checked_and_restored(self) -> None:
        self.assertEqual(self.ws.run("sync").returncode, 0)
        manifest_path = self.ws.root / "crates/world-modules/Cargo.toml"
        manifest_text = manifest_path.read_text(encoding="utf-8")
        self.assertIs(tomllib.loads(manifest_text)["lints"]["workspace"], True)

        without_lints = manifest_text.replace("\n[lints]\nworkspace = true\n", "\n", 1)
        self.assertNotEqual(without_lints, manifest_text)
        manifest_path.write_text(without_lints, encoding="utf-8")
        stale = self.ws.run("check")
        self.assertNotEqual(stale.returncode, 0)
        self.assertIn("crates/world-modules/Cargo.toml", stale.stderr)

        self.assertEqual(self.ws.run("sync").returncode, 0)
        restored = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
        self.assertIs(restored["lints"]["workspace"], True)


if __name__ == "__main__":
    unittest.main(verbosity=2)
