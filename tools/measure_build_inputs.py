#!/usr/bin/env python3
"""Opt-in tiny Cargo diagnostic for build-script inputs and dev profiles.

This is evidence for invalidation/profile behavior, not a representative
workspace benchmark. It builds only isolated local fixtures and emits JSON.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import platform
import signal
import subprocess
import sys
import tempfile
import time
import tomllib
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
BUILD_SCRIPT = ROOT / "crates/world-server/build.rs"
ROOT_MANIFEST = ROOT / "Cargo.toml"
TOOLCHAIN = ROOT / "rust-toolchain.toml"
PROFILE_HEADERS = {"[profile.dev]", '[profile.dev.package."*"]', "[profile.dev.build-override]"}
RUST_OVERRIDE_KEYS = {
    "RUSTFLAGS", "RUSTDOCFLAGS", "RUSTC", "RUSTDOC", "RUSTC_WRAPPER",
    "RUSTC_WORKSPACE_WRAPPER", "RUSTC_BOOTSTRAP", "RUSTUP_TOOLCHAIN",
}
EXIT_FAILED = 1
EXIT_INCONCLUSIVE = 2


class DiagnosticError(RuntimeError):
    """A setup/command failure that remains red in the report."""


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="milliseconds")


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def profile_sections(text: str, include_package: bool = True) -> str:
    selected = set(PROFILE_HEADERS)
    if not include_package:
        selected.remove('[profile.dev.package."*"]')
    result: list[str] = []
    active = False
    for line in text.splitlines():
        if line.lstrip().startswith("["):
            active = line.strip() in selected
        if active:
            result.append(line)
    if not result:
        raise DiagnosticError("repository dev profile sections were not found")
    return "\n".join(result).rstrip() + "\n"


def toolchain_channel(path: Path = TOOLCHAIN) -> str:
    with path.open("rb") as stream:
        channel = tomllib.load(stream)["toolchain"]["channel"]
    if not isinstance(channel, str) or not channel:
        raise DiagnosticError("rust-toolchain.toml has no channel")
    return channel


def controlled_environment(
    target: Path, cargo_home: Path, base: dict[str, str] | None = None
) -> tuple[dict[str, str], list[str]]:
    """Remove inherited Cargo/Rust/Git/build-script overrides."""
    env = dict(os.environ if base is None else base)
    removed: list[str] = []
    for key in list(env):
        if (
            key.startswith(("CARGO_", "GIT_"))
            or key in RUST_OVERRIDE_KEYS
            or key in {"GIT_HASH", "VERGEN_GIT_SHA"}
        ):
            env.pop(key)
            removed.append(key)
    env.update(
        {
            "CARGO_TARGET_DIR": str(target.resolve()),
            "CARGO_HOME": str(cargo_home.resolve()),
            "CARGO_BUILD_JOBS": "1",
            "CARGO_NET_OFFLINE": "true",
            "CARGO_INCREMENTAL": "0",
            "CARGO_TERM_COLOR": "never",
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_CONFIG_GLOBAL": os.devnull,
            "GIT_CONFIG_SYSTEM": os.devnull,
            "GIT_TERMINAL_PROMPT": "0",
        }
    )
    return env, sorted(removed)


def build_command(channel: str) -> list[str]:
    return [
        "cargo", f"+{channel}", "build", "--offline", "--locked",
        "--jobs", "1", "--message-format=json-render-diagnostics",
    ]


def diagnostic_plan() -> list[str]:
    return ["cold", "unchanged-warm", "doc-only-commit", "profile-override-only"]


def parse_artifacts(stdout: bytes) -> dict[str, Any]:
    records: list[dict[str, Any]] = []
    parse_errors = 0
    for raw in stdout.splitlines():
        if not raw.strip():
            continue
        try:
            message = json.loads(raw)
        except (UnicodeDecodeError, json.JSONDecodeError):
            parse_errors += 1
            continue
        if not isinstance(message, dict) or message.get("reason") != "compiler-artifact":
            continue
        target = message.get("target")
        if not isinstance(target, dict):
            continue
        profile = message.get("profile")
        profile = profile if isinstance(profile, dict) else {}
        kinds = target.get("kind", [])
        kinds = [kind for kind in kinds if isinstance(kind, str)] if isinstance(kinds, list) else []
        records.append(
            {
                "target": target.get("name"),
                "kind": kinds,
                "fresh": message.get("fresh"),
                "profile": {
                    key: profile[key]
                    for key in ("opt_level", "debug", "debug_assertions", "overflow_checks", "test")
                    if key in profile
                },
                "artifact_files": [
                    Path(name).name for name in message.get("filenames", []) if isinstance(name, str)
                ],
            }
        )
    return {"records": records, "compiler_artifact_count": len(records), "json_parse_errors": parse_errors}


def artifact_paths(stdout: bytes, target: str, kind: str) -> list[Path]:
    paths: list[Path] = []
    for raw in stdout.splitlines():
        try:
            message = json.loads(raw)
        except (UnicodeDecodeError, json.JSONDecodeError):
            continue
        if not isinstance(message, dict) or message.get("reason") != "compiler-artifact":
            continue
        metadata = message.get("target")
        if not isinstance(metadata, dict) or metadata.get("name") != target:
            continue
        kinds = metadata.get("kind", [])
        if kind in kinds:
            paths.extend(Path(name) for name in message.get("filenames", []) if isinstance(name, str))
    return paths


def embedded_revision(stdout: bytes, target: str, expected: str) -> str | None:
    needle = expected.encode("ascii")
    for path in artifact_paths(stdout, target, "lib"):
        try:
            if needle in path.read_bytes():
                return expected
        except OSError:
            pass
    return None


def profile_for(records: list[dict[str, Any]], target: str, kind: str) -> dict[str, Any] | None:
    for record in records:
        if record.get("target") == target and kind in record.get("kind", []):
            profile = record.get("profile")
            return profile if isinstance(profile, dict) else None
    return None


class CommandRunner:
    def __init__(self, env: dict[str, str], timeout: int, results: list[dict[str, Any]]):
        self.env, self.timeout, self.results = env, timeout, results

    def run(
        self, command: list[str], cwd: Path, cargo: bool = False
    ) -> tuple[dict[str, Any], bytes, bytes]:
        started_at = utc_now()
        started = time.monotonic()
        stdout = stderr = b""
        timed_out = False
        process: subprocess.Popen[bytes] | None = None
        error_type: str | None = None
        try:
            process = subprocess.Popen(
                command, cwd=cwd, env=self.env, stdout=subprocess.PIPE,
                stderr=subprocess.PIPE, start_new_session=True,
            )
            try:
                stdout, stderr = process.communicate(timeout=self.timeout)
            except subprocess.TimeoutExpired:
                timed_out = True
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                try:
                    stdout, stderr = process.communicate(timeout=5)
                except subprocess.TimeoutExpired:
                    try:
                        os.killpg(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    stdout, stderr = process.communicate()
            exit_code = process.returncode
        except OSError as error:
            exit_code = None
            error_type = type(error).__name__
        except BaseException:
            if process is not None and process.poll() is None:
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    try:
                        os.killpg(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    process.wait()
            raise
        result: dict[str, Any] = {
            "command": command,
            "cwd": str(cwd),
            "started_at": started_at,
            "wall_time_seconds": round(time.monotonic() - started, 3),
            "exit_code": exit_code,
            "timed_out": timed_out,
            "status": "passed" if exit_code == 0 and not timed_out else "failed",
            "stdout_bytes": len(stdout),
            "stderr_lines": len(stderr.splitlines()),
            "stderr_sha256": digest(stderr),
        }
        if error_type:
            result["error_type"] = error_type
        if cargo:
            result.update(parse_artifacts(stdout))
        self.results.append(result)
        return result, stdout, stderr

    def require(self, command: list[str], cwd: Path, cargo: bool = False) -> tuple[dict[str, Any], bytes]:
        result, stdout, _ = self.run(command, cwd, cargo)
        if result["status"] != "passed":
            raise DiagnosticError(f"command failed: {command[0]} {command[1] if len(command) > 1 else ''}".strip())
        return result, stdout


def git_output(runner: CommandRunner, repo: Path, *args: str) -> str:
    _, stdout = runner.require(["git", *args], repo)
    return stdout.decode("utf-8", "replace").strip()


def write_fixture(fixture: Path, build_script: bytes, profiles: str, toolchain: bytes) -> None:
    # Keep dependencies physically outside the app workspace. Excluding paths
    # beneath a `members = ["."]` root did not prevent auto-membership on 1.98.
    for path in ("src", "../dependency/src", "../proc-macro/src"):
        (fixture / path).mkdir(parents=True)
    (fixture / "build.rs").write_bytes(build_script)
    (fixture / "rust-toolchain.toml").write_bytes(toolchain)
    (fixture / "README.md").write_text("initial diagnostic fixture\n", encoding="utf-8")
    manifest = "\n".join(
        [
            "[workspace]", 'members = ["."]',
            'resolver = "3"', "", "[package]", 'name = "fixture-app"',
            'version = "0.1.0"', 'edition = "2024"', 'rust-version = "1.98"',
            'build = "build.rs"', "", "[lib]", 'path = "src/lib.rs"', "",
            "[dependencies]", 'fixture-dependency = { path = "../dependency" }',
            'fixture-macro = { path = "../proc-macro" }', "",
        ]
    ) + profiles
    (fixture / "Cargo.toml").write_text(manifest, encoding="utf-8")
    (fixture / "src/lib.rs").write_text(
        'fixture_macro::fixture_marker!();\n\n'
        'pub fn dependency_value() -> u32 { fixture_dependency::value() }\n\n'
        'pub fn embedded_revision() -> &\'static str { env!("GIT_HASH") }\n',
        encoding="utf-8",
    )
    (fixture / "../dependency/Cargo.toml").write_text(
        "[package]\nname = \"fixture-dependency\"\nversion = \"0.1.0\"\n"
        "edition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
        encoding="utf-8",
    )
    (fixture / "../dependency/src/lib.rs").write_text("pub fn value() -> u32 { 7 }\n", encoding="utf-8")
    (fixture / "../proc-macro/Cargo.toml").write_text(
        "[package]\nname = \"fixture-macro\"\nversion = \"0.1.0\"\n"
        "edition = \"2024\"\n\n[lib]\nproc-macro = true\npath = \"src/lib.rs\"\n",
        encoding="utf-8",
    )
    (fixture / "../proc-macro/src/lib.rs").write_text(
        "extern crate proc_macro;\n\n"
        "#[proc_macro]\npub fn fixture_marker(_input: proc_macro::TokenStream) "
        "-> proc_macro::TokenStream { proc_macro::TokenStream::new() }\n",
        encoding="utf-8",
    )


def prepare_fixture(
    fixture: Path, runner: CommandRunner, source_packed_refs: bool, channel: str
) -> dict[str, Any]:
    runner.require(["cargo", f"+{channel}", "generate-lockfile", "--offline"], fixture)
    _, raw_metadata = runner.require([
        "cargo", f"+{channel}", "metadata", "--locked", "--offline", "--format-version", "1",
    ], fixture)
    metadata = json.loads(raw_metadata)
    members = sorted(package["name"] for package in metadata["packages"]
                     if package["id"] in metadata["workspace_members"])
    if members != ["fixture-app"]:
        raise DiagnosticError(f"fixture dependencies are not external: {members}")
    for command in (
        ["git", "init", "-q"],
        ["git", "config", "user.email", "build-inputs@example.invalid"],
        ["git", "config", "user.name", "Build input diagnostic"],
        ["git", "config", "core.hooksPath", os.devnull],
        ["git", "add", "."],
        ["git", "commit", "-qm", "initial fixture"],
    ):
        runner.require(command, fixture)
    if source_packed_refs:
        runner.require(["git", "pack-refs", "--all", "--no-prune"], fixture)
    common = Path(git_output(runner, fixture, "rev-parse", "--git-common-dir"))
    if not common.is_absolute():
        common = fixture / common
    return {
        "initial_sha": git_output(runner, fixture, "rev-parse", "HEAD"),
        "packed_refs_present": (common / "packed-refs").is_file(),
        "packed_refs_setup": "git pack-refs --all --no-prune" if source_packed_refs else "not applied",
        "workspace_members": members,
    }


def cargo_build(
    runner: CommandRunner, fixture: Path, channel: str, expected: str | None = None
) -> dict[str, Any]:
    result, stdout = runner.require(build_command(channel), fixture, cargo=True)
    if expected:
        result["embedded_revision"] = embedded_revision(stdout, "fixture_app", expected)
    return result


def revision_scenario(
    fixture: Path, runner: CommandRunner, setup: dict[str, Any], channel: str
) -> dict[str, Any]:
    initial = setup["initial_sha"]
    cold = cargo_build(runner, fixture, channel, initial)
    warm = cargo_build(runner, fixture, channel, initial)
    (fixture / "README.md").write_text("documentation-only change\n", encoding="utf-8")
    runner.require(["git", "add", "README.md"], fixture)
    runner.require(["git", "commit", "-qm", "documentation-only change"], fixture)
    doc_sha = git_output(runner, fixture, "rev-parse", "HEAD")
    doc = cargo_build(runner, fixture, channel, doc_sha)
    return {
        "initial_sha": initial,
        "doc_only_sha": doc_sha,
        "cold": cold,
        "unchanged_warm": warm,
        "doc_only_commit": {
            **doc,
            "working_tree_clean": git_output(runner, fixture, "status", "--porcelain") == "",
        },
        "packed_refs": setup["packed_refs_present"],
        "packed_refs_setup": setup["packed_refs_setup"],
        "workspace_members": setup["workspace_members"],
    }


def revision_observation(scenario: dict[str, Any]) -> dict[str, Any]:
    def fresh(step: dict[str, Any]) -> Any:
        return next(
            (record["fresh"] for record in step.get("records", [])
             if record.get("target") == "fixture_app" and "lib" in record.get("kind", [])),
            None,
        )

    cold, warm, doc = scenario["cold"], scenario["unchanged_warm"], scenario["doc_only_commit"]
    cold_rev, warm_rev, doc_rev = cold.get("embedded_revision"), warm.get("embedded_revision"), doc.get("embedded_revision")
    return {
        "cold_root_lib_fresh": fresh(cold),
        "warm_root_lib_fresh": fresh(warm),
        "doc_only_root_lib_fresh": fresh(doc),
        "cold_embedded_revision": cold_rev,
        "warm_embedded_revision": warm_rev,
        "doc_only_embedded_revision": doc_rev,
        "warm_reused_root": fresh(warm) is True,
        "doc_only_rebuilt_root": fresh(doc) is False,
        "doc_only_revision_changed": cold_rev is not None and doc_rev == scenario["doc_only_sha"] and doc_rev != cold_rev,
    }


def profile_observation(effective: dict[str, Any], controlled: dict[str, Any]) -> dict[str, Any]:
    e, c = effective.get("records", []), controlled.get("records", [])
    values = {
        "effective_dependency": profile_for(e, "fixture_dependency", "lib"),
        "controlled_dependency": profile_for(c, "fixture_dependency", "lib"),
        "effective_proc_macro": profile_for(e, "fixture_macro", "proc-macro"),
        "controlled_proc_macro": profile_for(c, "fixture_macro", "proc-macro"),
    }
    return {
        **values,
        "dependency_wildcard_precedence": (
            values["effective_dependency"] is not None
            and values["controlled_dependency"] is not None
            and values["effective_dependency"].get("opt_level") == "2"
            and values["controlled_dependency"].get("opt_level") == "0"
        ),
        "proc_macro_package_precedence": (
            values["effective_proc_macro"] is not None
            and values["controlled_proc_macro"] is not None
            and values["effective_proc_macro"].get("opt_level") == "2"
            and values["controlled_proc_macro"].get("opt_level") == "1"
        ),
    }


def report_status(revision: dict[str, Any], profiles: dict[str, Any], results: list[dict[str, Any]]) -> str:
    if any(result.get("status") != "passed" for result in results):
        return "failed"
    if (
        revision.get("warm_reused_root") and revision.get("doc_only_rebuilt_root")
        and revision.get("doc_only_revision_changed")
        and profiles.get("dependency_wildcard_precedence")
        and profiles.get("proc_macro_package_precedence")
    ):
        return "passed"
    return "inconclusive"


def write_report(path: Path | None, report: dict[str, Any]) -> None:
    serialized = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if path is None:
        sys.stdout.write(serialized)
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    descriptor = os.open(path, flags, 0o600)
    with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
        stream.write(serialized)


def run_diagnostic(output: Path | None, timeout: int) -> int:
    started = time.monotonic()
    report: dict[str, Any] = {
        "schema": 1,
        "tool": "rustycore-build-input-diagnostic",
        "status": "failed",
        "started_at": utc_now(),
        "timeout_seconds": timeout,
        "plan": diagnostic_plan(),
        "host": {
            "platform": platform.platform(),
            "machine": platform.machine(),
            "python": platform.python_version(),
            "cpu_count": os.cpu_count(),
        },
        "limitations": [
            "Tiny local fixtures establish input/profile behavior, not representative workspace speedups.",
            "No network dependencies or full-workspace build are measured.",
            "Embedded revision detection scans the tiny lib artifact and is best effort.",
            "Wall times are recorded for context only and are not an optimization claim.",
        ],
    }
    results: list[dict[str, Any]] = []
    try:
        build_bytes = BUILD_SCRIPT.read_bytes()
        manifest_text = ROOT_MANIFEST.read_text(encoding="utf-8")
        channel = toolchain_channel()
        with tempfile.TemporaryDirectory(prefix="rustycore-build-inputs-") as raw:
            temp_root = Path(raw)
            source_home = temp_root / "cargo-home-source"
            source_home.mkdir(mode=0o700)
            source_env, removed_source = controlled_environment(
                temp_root / "target-source", source_home
            )
            source_runner = CommandRunner(source_env, timeout, results)
            source_sha = git_output(source_runner, ROOT, "rev-parse", "HEAD")
            common = Path(git_output(source_runner, ROOT, "rev-parse", "--git-common-dir"))
            if not common.is_absolute():
                common = (ROOT / common).resolve()
            packed = (common / "packed-refs").is_file()
            effective = temp_root / "effective/app"
            controlled = temp_root / "build-override-only/app"
            for home in (temp_root / "cargo-home-effective", temp_root / "cargo-home-controlled"):
                home.mkdir(mode=0o700)
            write_fixture(effective, build_bytes, profile_sections(manifest_text), TOOLCHAIN.read_bytes())
            write_fixture(
                controlled, build_bytes, profile_sections(manifest_text, False), TOOLCHAIN.read_bytes()
            )
            effective_env, removed_effective = controlled_environment(
                temp_root / "target-effective", temp_root / "cargo-home-effective"
            )
            controlled_env, removed_controlled = controlled_environment(
                temp_root / "target-controlled", temp_root / "cargo-home-controlled"
            )
            effective_runner = CommandRunner(effective_env, timeout, results)
            controlled_runner = CommandRunner(controlled_env, timeout, results)
            rust, rust_out, _ = effective_runner.run(
                ["rustc", f"+{channel}", "--version"], effective
            )
            cargo, cargo_out, _ = effective_runner.run(
                ["cargo", f"+{channel}", "--version"], effective
            )
            rust_text = rust_out.decode("utf-8", "replace").strip()
            cargo_text = cargo_out.decode("utf-8", "replace").strip()
            active = rust_text.split()[1] if len(rust_text.split()) > 1 else None
            report["toolchain"] = {
                "required_channel": channel,
                "active_rust": active,
                "rustc_version": rust_text,
                "cargo_version": cargo_text,
                "pinned_match": active == channel and rust["status"] == "passed" and cargo["status"] == "passed",
            }
            if not report["toolchain"]["pinned_match"]:
                raise DiagnosticError("pinned rust/cargo version probe failed")
            effective_setup = prepare_fixture(effective, effective_runner, packed, channel)
            controlled_setup = prepare_fixture(controlled, controlled_runner, packed, channel)
            scenario = revision_scenario(effective, effective_runner, effective_setup, channel)
            controlled_build = cargo_build(controlled_runner, controlled, channel, controlled_setup["initial_sha"])
            revision = revision_observation(scenario)
            profiles = profile_observation(scenario["cold"], controlled_build)
            report["repository"] = {
                "source_repo_sha": source_sha,
                "build_script": {
                    "path": "crates/world-server/build.rs",
                    "sha256": digest(build_bytes),
                    "source_repo_sha": source_sha,
                },
                "profile_source": {
                    "path": "Cargo.toml",
                    "sha256": digest(ROOT_MANIFEST.read_bytes()),
                    "sections": sorted(PROFILE_HEADERS),
                },
                "source_packed_refs_present": packed,
            }
            report["isolation"] = {
                "fixture_root": str(temp_root),
                "cargo_target_dirs": [str((temp_root / name).resolve()) for name in ("target-effective", "target-controlled")],
                "cargo_homes": [str((temp_root / name).resolve()) for name in ("cargo-home-effective", "cargo-home-controlled")],
                "offline": True,
                "cargo_jobs": 1,
                "cargo_incremental": 0,
                "sanitized_env_keys": sorted(set(removed_source + removed_effective + removed_controlled)),
                "target_dirs_private": True,
            }
            report["scenarios"] = {
                "revision_invalidation": scenario,
                "profile_precedence": {
                    "effective": scenario["cold"],
                    "build_override_only": controlled_build,
                    "observation": profiles,
                },
            }
            if not packed:
                report["limitations"].append(
                    "Source Git common dir has no packed-refs; the missing watched path can make warm freshness inconclusive."
                )
            if not revision.get("doc_only_revision_changed"):
                report["limitations"].append(
                    "The embedded GIT_HASH string was not found in the tiny lib artifact."
                )
            if not revision.get("warm_reused_root"):
                report["limitations"].append(
                    "Cargo did not report the unchanged warm root lib as fresh."
                )
            report["observations"] = {
                "revision_invalidation": revision,
                "profile_precedence": profiles,
            }
            report["status"] = report_status(revision, profiles, results)
    except KeyboardInterrupt:
        report["error"] = "KeyboardInterrupt"
    except (OSError, DiagnosticError, ValueError, IndexError, KeyError) as error:
        report["error"] = f"{type(error).__name__}: {error}"
    report["ended_at"] = utc_now()
    report["wall_time_seconds"] = round(time.monotonic() - started, 3)
    report["command_results"] = results
    report["commands"] = [result["command"] for result in results]
    write_report(output, report)
    return 0 if report["status"] == "passed" else EXIT_INCONCLUSIVE if report["status"] == "inconclusive" else EXIT_FAILED


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, help="write JSON here instead of stdout")
    parser.add_argument("--timeout", type=int, default=300, help="per-command timeout in seconds")
    args = parser.parse_args(argv)
    if args.timeout < 30:
        parser.error("--timeout must be at least 30 seconds")
    return run_diagnostic(args.output, args.timeout)


if __name__ == "__main__":
    raise SystemExit(main())
