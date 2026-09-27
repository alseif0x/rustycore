#!/usr/bin/env python3
"""Hermetic unit tests for the opt-in build-input diagnostic (no Cargo runs)."""

from __future__ import annotations

import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, patch

import measure_build_inputs as tool


class MeasureBuildInputsTests(unittest.TestCase):
    def test_command_contract_and_failures(self) -> None:
        command = tool.build_command("1.98.0")
        self.assertEqual(command, ["cargo", "+1.98.0", "build", "--offline", "--locked",
                                   "--jobs", "1", "--message-format=json-render-diagnostics"])
        for timed_out in (False, True):
            process = Mock(pid=123, returncode=0 if timed_out else 17)
            if timed_out:
                process.communicate.side_effect = [tool.subprocess.TimeoutExpired(command, 30), (b"", b"")]
            else:
                process.communicate.return_value = (b"", b"fixture failure")
            results = []
            runner = tool.CommandRunner({}, 30, results)
            with patch.object(tool.subprocess, "Popen", return_value=process) as popen, \
                 patch.object(tool.os, "killpg") as killpg:
                with self.assertRaises(tool.DiagnosticError):
                    runner.require(command, Path("/fixture"))
            self.assertTrue(popen.call_args.kwargs["start_new_session"])
            self.assertEqual(results[0]["status"], "failed")
            self.assertEqual(results[0]["timed_out"], timed_out)
            self.assertEqual(results[0]["exit_code"], process.returncode)
            if timed_out:
                killpg.assert_called_once_with(123, tool.signal.SIGTERM)
            else:
                killpg.assert_not_called()

    def test_profile_selection_and_toolchain_parser(self) -> None:
        text = (
            '[profile.dev]\ndebug = 1\n'
            '[profile.dev.package."*"]\nopt-level = 2\n'
            '[profile.dev.build-override]\nopt-level = 1\n'
            '[profile.release]\nopt-level = 3\n'
        )
        effective = tool.profile_sections(text)
        controlled = tool.profile_sections(text, include_package=False)
        self.assertIn('[profile.dev.package."*"]', effective)
        self.assertNotIn('[profile.dev.package."*"]', controlled)
        self.assertIn('[profile.dev.build-override]', controlled)
        self.assertNotIn('[profile.release]', effective)
        with tempfile.TemporaryDirectory() as raw:
            path = Path(raw) / "rust-toolchain.toml"
            path.write_text('[toolchain]\nchannel = "1.98.0"\n', encoding="utf-8")
            self.assertEqual(tool.toolchain_channel(path), "1.98.0")

    def test_environment_isolation(self) -> None:
        base = {
            "PATH": "/bin",
            "CARGO_HOME": "private-secret",
            "CARGO_TARGET_DIR": "other-target",
            "GIT_HASH": "wrong-revision",
            "VERGEN_GIT_SHA": "another-revision",
            "GIT_DIR": "other-repository",
            "RUSTFLAGS": "-C target-cpu=native",
            "RUSTUP_TOOLCHAIN": "nightly",
        }
        with tempfile.TemporaryDirectory() as raw:
            target, home = Path(raw) / "target", Path(raw) / "cargo-home"
            env, removed = tool.controlled_environment(target, home, base)
        self.assertEqual(env["CARGO_TARGET_DIR"], str(target.resolve()))
        self.assertEqual(env["CARGO_HOME"], str(home.resolve()))
        self.assertEqual(env["CARGO_BUILD_JOBS"], "1")
        self.assertEqual(env["CARGO_NET_OFFLINE"], "true")
        for key in ("GIT_HASH", "VERGEN_GIT_SHA", "GIT_DIR", "RUSTFLAGS", "RUSTUP_TOOLCHAIN"):
            self.assertNotIn(key, env)
            self.assertIn(key, removed)
        self.assertNotIn("private-secret", env.values())
        self.assertEqual(env["GIT_CONFIG_NOSYSTEM"], "1")
        self.assertEqual(env["GIT_CONFIG_GLOBAL"], tool.os.devnull)

    def test_fixture_declares_non_workspace_dependencies(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            fixture = Path(raw) / "fixture"
            tool.write_fixture(fixture, b"fn main() {}\n", "[profile.dev]\n", b"[toolchain]\nchannel='1.98.0'\n")
            manifest = (fixture / "Cargo.toml").read_text(encoding="utf-8")
        self.assertIn('members = ["."]', manifest)
        self.assertIn("fixture-dependency = { path = \"../dependency\" }", manifest)
        self.assertIn("fixture-macro = { path = \"../proc-macro\" }", manifest)

    def test_rejects_auto_workspace_members_before_compilation(self) -> None:
        runner = Mock()
        metadata = {"packages": [{"id": "root", "name": "fixture-app"},
                                 {"id": "dep", "name": "fixture-dependency"}],
                    "workspace_members": ["root", "dep"]}
        runner.require.side_effect = [({}, b""), ({}, json.dumps(metadata).encode())]
        with self.assertRaisesRegex(tool.DiagnosticError, "dependencies are not external"):
            tool.prepare_fixture(Path("/fixture"), runner, True, "1.98.0")
        self.assertEqual(runner.require.call_count, 2)

    def test_artifact_parser_and_embedded_revision(self) -> None:
        revision = "a" * 40
        with tempfile.TemporaryDirectory() as raw:
            artifact = Path(raw) / "libfixture_app.rlib"
            artifact.write_bytes(b"metadata\0" + revision.encode() + b"\0")
            message = {
                "reason": "compiler-artifact",
                "target": {"name": "fixture_app", "kind": ["lib"]},
                "profile": {"opt_level": "2", "debug": 1},
                "fresh": False,
                "filenames": [str(artifact)],
            }
            stdout = (json.dumps(message) + "\n").encode()
            parsed = tool.parse_artifacts(stdout)
            self.assertEqual(parsed["compiler_artifact_count"], 1)
            self.assertFalse(parsed["records"][0]["fresh"])
            self.assertEqual(tool.profile_for(parsed["records"], "fixture_app", "lib")["opt_level"], "2")
            self.assertEqual(tool.embedded_revision(stdout, "fixture_app", revision), revision)

    def test_status_requires_all_observations(self) -> None:
        revision = {
            "warm_reused_root": True,
            "doc_only_rebuilt_root": True,
            "doc_only_revision_changed": True,
        }
        profiles = {
            "dependency_wildcard_precedence": True,
            "proc_macro_package_precedence": True,
        }
        self.assertEqual(tool.report_status(revision, profiles, [{"status": "passed"}]), "passed")
        revision["doc_only_revision_changed"] = False
        self.assertEqual(tool.report_status(revision, profiles, [{"status": "passed"}]), "inconclusive")
        self.assertEqual(tool.report_status(revision, profiles, [{"status": "failed"}]), "failed")

    def test_report_is_exclusive(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            path = Path(raw) / "report.json"
            tool.write_report(path, {"status": "inconclusive"})
            self.assertEqual(json.loads(path.read_text(encoding="utf-8"))["status"], "inconclusive")
            with self.assertRaises(FileExistsError):
                tool.write_report(path, {"status": "failed"})


if __name__ == "__main__":
    unittest.main()
