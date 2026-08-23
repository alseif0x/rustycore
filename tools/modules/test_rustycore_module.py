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


if __name__ == "__main__":
    unittest.main(verbosity=2)
