#!/usr/bin/env python3
"""Hermetic contract tests for the shadow Validation V2 runner."""

from __future__ import annotations

import fcntl
import hashlib
import io
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
from contextlib import redirect_stdout
from unittest.mock import patch


RUNNER_PATH = Path(__file__).with_name("validation-v2").resolve()
loader = importlib.machinery.SourceFileLoader("validation_v2_runner", str(RUNNER_PATH))
spec = importlib.util.spec_from_loader(loader.name, loader)
runner = importlib.util.module_from_spec(spec)
loader.exec_module(runner)


def fake_tool(path: Path, body: str) -> None:
    path.write_text("#!/usr/bin/env bash\nset -eu\n" + body)
    path.chmod(0o755)


def synthetic_metadata(repo: Path) -> dict[str, object]:
    packages = []
    nodes = []
    for package, dependencies in (("a", []), ("b", ["a"]), ("c", ["b"])):
        package_root = repo / "crates" / package
        (package_root / "src").mkdir(parents=True)
        (package_root / "Cargo.toml").write_text(
            f'[package]\nname = "{package}"\nversion = "0.1.0"\nedition = "2024"\n'
        )
        (package_root / "src" / "lib.rs").write_text("pub fn fixture() {}\n")
        packages.append(
            {
                "id": package,
                "name": package,
                "manifest_path": str(package_root / "Cargo.toml"),
                "targets": [{"kind": ["lib"]}],
            }
        )
        nodes.append({"id": package, "dependencies": dependencies})
    return {
        "workspace_members": ["a", "b", "c"],
        "packages": packages,
        "resolve": {"nodes": nodes},
    }


def stable_manifest(value: object) -> object:
    """The runner owns the comparison form; the fixture must not keep a rival copy."""
    return runner.normalise_manifest(value)


def test_no_validation_level() -> None:
    for arguments in ([], ["none"], ["1"]):
        captured = io.StringIO()
        with patch.object(sys, "argv", ["validation-v2", *arguments]), \
             patch.object(runner, "provenance", side_effect=AssertionError("level 1 must not inspect Git/Rust")), \
             patch.object(runner, "command_environment", side_effect=AssertionError("level 1 must not probe tools")), \
             patch.object(runner, "acquire_lock", side_effect=AssertionError("level 1 must not acquire locks")), \
             patch.object(runner, "write_manifest", side_effect=AssertionError("level 1 must not produce evidence")), \
             redirect_stdout(captured):
            assert runner.main() == 0
        assert "NOT VALIDATED" in captured.getvalue()
        assert "no acceptance manifest" in captured.getvalue()
    for option in ("--logs", "--require-changes", "--keep-going", "--architecture", "--timings"):
        with patch.object(sys, "argv", ["validation-v2", "1", option]):
            assert runner.main() == runner.USAGE_ERROR


def test_runner_contract(repo: Path, tools: Path, base_env: dict[str, str], directory: Path) -> None:
    (repo / "docs").mkdir()
    (repo / "docs" / "guide.md").write_text("fixture\n")

    def invoke(mode: str, manifest: Path, extra: dict[str, str] | None = None,
               flags: list[str] | None = None):
        environment = base_env.copy()
        environment["VALIDATION_V2_MANIFEST"] = str(manifest)
        if extra:
            environment.update(extra)
        return subprocess.run(
            [str(tools / "validation-v2"), mode, "--base", "HEAD", *(flags or [])],
            cwd=repo,
            env=environment,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

    success_manifest = directory / "success.json"
    result = invoke("quick", success_manifest)
    assert result.returncode == 0, result.stderr
    success = json.loads(success_manifest.read_text())
    assert success["schema"] == runner.MANIFEST_SCHEMA
    assert success["runner_signal"] is None
    assert success["resources"]["cargo_jobs"] == runner.DEFAULT_JOBS == 1
    timed_result = invoke("quick", directory / "timed-docs.json", flags=["--timings"])
    assert timed_result.returncode == 0, timed_result.stderr
    timed_docs = json.loads((directory / "timed-docs.json").read_text())
    assert timed_docs["plan"]["planned_commands"] == success["plan"]["planned_commands"]
    invalid_timings = invoke("verify", directory / "invalid-timings.json", flags=["--timings"])
    assert invalid_timings.returncode == runner.USAGE_ERROR
    assert "--timings is only valid" in invalid_timings.stderr
    invalid_architecture = invoke("quick", directory / "invalid-architecture.json", flags=["--architecture"])
    assert invalid_architecture.returncode == runner.USAGE_ERROR
    assert "--architecture is only valid" in invalid_architecture.stderr
    assert (
        f"validation-v2: cargo target={repo.resolve() / 'target'} jobs={runner.DEFAULT_JOBS}"
        in result.stdout
    )
    assert "memory_limit_kib" in success["resources"]
    assert success["profile"] == "quick"
    assert len(success["run_id"]) == 20
    assert success["provenance"]["rust"] == {"active": "1.98.0", "pinned": "1.98.0"}
    assert success["plan"]["changed_paths"] == ["docs/guide.md"]
    assert success["plan"]["workspace"] is None
    assert len(success["commands"]) == 2
    assert all(command["status"] == "passed" for command in success["commands"])
    alias_manifest = directory / "level-two.json"
    alias_result = invoke("2", alias_manifest, {"PROTOC": "/must-not-be-probed"})
    assert alias_result.returncode == 0, alias_result.stderr
    assert "ITERATION ONLY" in alias_result.stdout
    assert json.loads(alias_manifest.read_text())["profile"] == "quick"
    logged_manifest = directory / "logged.json"
    logged_result = invoke("quick", logged_manifest, flags=["--logs"])
    assert logged_result.returncode == 0, logged_result.stderr
    logged = json.loads(logged_manifest.read_text())
    assert stable_manifest(logged) == stable_manifest(success)
    log_directory = directory / "logged-logs"
    assert log_directory.stat().st_mode & 0o777 == 0o700
    logs = sorted(log_directory.glob("*.log"))
    assert len(logs) == len(logged["commands"])
    for log, command in zip(logs, logged["commands"]):
        assert log.stat().st_mode & 0o777 == 0o600
        assert json.loads(log.read_text().splitlines()[0])["argv"] == command["argv"]
    retained = [log.read_bytes() for log in logs]
    repeated_log = invoke("quick", logged_manifest, flags=["--logs"])
    assert repeated_log.returncode != 0  # Never overwrite earlier evidence.
    assert [log.read_bytes() for log in logs] == retained
    assert "command logs" in logged_result.stdout
    for mode, flag in (("quick", "--keep-going"), ("audit", "--require-changes"), ("verify", "--logs")):
        invalid_option = invoke(mode, directory / "invalid-option.json", flags=[flag])
        assert invalid_option.returncode == runner.USAGE_ERROR
    stable_success = stable_manifest(success)
    for run in range(2, 11):
        repeated_manifest = directory / f"success-{run}.json"
        result = invoke("quick", repeated_manifest)
        assert result.returncode == 0, result.stderr
        repeated = json.loads(repeated_manifest.read_text())
        assert stable_manifest(repeated) == stable_success

    broken = repo / "broken.sh"
    broken.write_text("if\n")
    failure_manifest = directory / "failure.json"
    result = invoke("quick", failure_manifest)
    assert result.returncode != 0
    failure = json.loads(failure_manifest.read_text())
    assert failure["exit_code"] == result.returncode
    assert failure["commands"][-1]["status"] == "failed"
    broken.unlink()

    direct_failure = runner.run_one(
        repo, [sys.executable, "-c", "raise SystemExit(23)"], base_env, 30
    )
    assert direct_failure["exit_code"] == 23
    assert direct_failure["failure_kind"] == "exit"
    assert direct_failure["status"] == "failed"
    direct_signal = runner.run_one(
        repo,
        [sys.executable, "-c", "import os,signal; os.kill(os.getpid(), signal.SIGTERM)"],
        base_env,
        30,
    )
    assert direct_signal["exit_code"] == 128 + signal.SIGTERM
    assert direct_signal["signal"] == signal.SIGTERM
    assert direct_signal["failure_kind"] == "signal"
    direct_timeout = runner.run_one(
        repo, [sys.executable, "-c", "import time; time.sleep(10)"], base_env, 1
    )
    assert direct_timeout["timed_out"] is True
    assert direct_timeout["exit_code"] == 128 + signal.SIGTERM
    assert direct_timeout["failure_kind"] == "timeout"

    # A child that traps SIGTERM and exits zero during the grace window used to
    # be recorded as {"timed_out": true, "exit_code": 0, "status": "passed"}.
    trapped_timeout = runner.run_one(
        repo,
        [
            sys.executable,
            "-c",
            "import signal,sys,time; signal.signal(signal.SIGTERM, lambda *_: sys.exit(0));"
            " time.sleep(30)",
        ],
        base_env,
        1,
    )
    assert trapped_timeout["timed_out"] is True
    assert trapped_timeout["exit_code"] == 0
    assert trapped_timeout["failure_kind"] == "timeout"
    assert trapped_timeout["status"] == "failed"

    # Cargo hides a signalled test binary behind its own exit 101.
    child_signal = runner.run_one(
        repo,
        [
            sys.executable,
            "-c",
            "import sys; print('error: test failed, to rerun pass `-p wow-world --lib`');"
            " print('  process didn\\'t exit successfully: wow_world-df42 (signal: 6,"
            " SIGABRT: process abort signal)'); sys.exit(101)",
        ],
        base_env,
        30,
    )
    assert child_signal["exit_code"] == 101
    assert child_signal["signal"] is None
    assert child_signal["failure_kind"] == "child-signal"
    assert child_signal["child_signal_reports"] == [{"signal": 6, "name": "SIGABRT"}]

    # The audit's own budget must outlast its longest step.
    assert runner.DEFAULT_TIMEOUT == 900 and runner.AUDIT_TIMEOUT == 3600
    # A fixture repository cannot satisfy a real audit, and whether the host
    # heavy lock is free decides how far it gets. Only the recorded budget is
    # this test's business.
    audit_manifest = directory / "audit-timeout.json"
    audit_bin = directory / "audit-bin"
    audit_bin.mkdir()
    fake_tool(audit_bin / "cargo", "exit 0\n")
    invoke("audit", audit_manifest, {"PATH": f"{audit_bin}:{base_env['PATH']}"})
    if audit_manifest.exists():
        recorded = json.loads(audit_manifest.read_text())
        assert recorded["resources"]["command_timeout_seconds"] == runner.AUDIT_TIMEOUT
    assert (
        json.loads(success_manifest.read_text())["resources"]["command_timeout_seconds"]
        == runner.DEFAULT_TIMEOUT
    )

    resolved = runner.resolve_protoc(repo, base_env)
    assert resolved is not None and resolved.endswith("protoc")
    assert runner.command_environment(repo, 2)["PROTOC"] == resolved

    wrong_version = dict(base_env)
    wrong_version["PROTOC"] = str(directory / "bin" / "protoc-wrong")
    fake_tool(directory / "bin" / "protoc-wrong", 'printf "libprotoc 27.0\n"\n')
    try:
        runner.resolve_protoc(repo, wrong_version)
    except ValueError as error:
        assert "expected 'libprotoc 28.3'" in str(error), error
    else:
        raise AssertionError("a mismatched protoc was accepted")

    absent = {key: value for key, value in base_env.items() if key != "PROTOC"}
    absent["PATH"] = "/nonexistent"
    absent["HOME"] = str(directory / "empty-home")
    assert runner.resolve_protoc(repo, absent) is None
    cargo_step = [{"section": "compile", "argv": ["cargo", "check"]}]
    try:
        runner.require_protoc_for(cargo_step, absent)
    except ValueError as error:
        assert "no pinned protoc was found" in str(error), error
    else:
        raise AssertionError("a Cargo plan was allowed without protoc")
    runner.require_protoc_for([{"section": "docs", "argv": ["git", "diff"]}], absent)

    # The comparison form is an allowlist: a new field cannot slip through it.
    for mutation, expected in (
        ({"invented_at_top": 1}, "manifest carries unknown field(s): invented_at_top"),
        (
            {"provenance": {**success["provenance"], "hostname": "x"}},
            "provenance carries unknown field(s): hostname",
        ),
        (
            {"resources": {**success["resources"], "swap_kib": 0}},
            "resources carries unknown field(s): swap_kib",
        ),
        (
            {"plan": {**success["plan"], "future_field": []}},
            "plan carries unknown field(s): future_field",
        ),
        (
            {"commands": [{**success["commands"][0], "cpu_seconds": 1}]},
            "command 0 carries unknown field(s): cpu_seconds",
        ),
    ):
        try:
            runner.normalise_manifest({**success, **mutation})
        except ValueError as error:
            assert str(error) == expected, (str(error), expected)
        else:
            raise AssertionError(f"the contract accepted {sorted(mutation)}")

    # Host-shaped values are placeheld, never compared literally.
    comparison = runner.normalise_manifest(success)
    assert comparison["provenance"]["repository_root"] == runner.PLACEHOLDER
    assert comparison["provenance"]["kernel"] == runner.PLACEHOLDER
    assert comparison["locks"]["repository"] == runner.PLACEHOLDER
    assert comparison["locks"]["heavy"] is None
    assert comparison["provenance"]["head"] == success["provenance"]["head"]
    assert "run_id" not in comparison and "started_at" not in comparison
    assert all(
        "duration_seconds" not in command for command in comparison["commands"]
    )

    normalised = subprocess.run(
        [str(tools / "validation-v2"), "normalize", "--manifest", str(success_manifest)],
        cwd=repo,
        env=base_env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert normalised.returncode == 0, normalised.stderr
    assert json.loads(normalised.stdout) == comparison

    # Classification order, including the OOM case a host cannot be forced into.
    assert runner.classify_failure(0, None, False, 0, []) is None
    assert runner.classify_failure(0, None, False, None, []) is None
    assert runner.classify_failure(0, None, False, 1, []) == "oom"
    assert runner.classify_failure(137, None, True, 1, []) == "oom"
    assert runner.classify_failure(137, 9, True, 0, []) == "timeout"
    assert runner.classify_failure(137, 9, False, 0, []) == "signal"
    assert runner.classify_failure(101, None, False, 0, [{"signal": 6}]) == "child-signal"
    assert runner.classify_failure(1, None, False, None, []) == "exit"
    for probe in (runner.oom_kill_count(), runner.memory_limit_kib()):
        assert probe is None or isinstance(probe, int)

    steps = [
        {"section": "pass", "argv": [sys.executable, "-c", "pass"]},
        {"section": "fail", "argv": [sys.executable, "-c", "raise SystemExit(19)"]},
        {
            "section": "must-not-run",
            "argv": [sys.executable, "-c", "raise SystemExit(99)"],
        },
    ]
    outcomes, exit_code = runner.run_steps(repo, steps, base_env, 30)
    assert exit_code == 19
    assert [outcome["section"] for outcome in outcomes] == ["pass", "fail"]

    # The combined audit policy check must stop before expensive Cargo and
    # persistence scans on failure, without dropping any green-path coverage.
    audit = runner.audit_steps("base", 1)
    for red_section, expected_sections in (
        ("architecture-policy-check", ["diff-hygiene", "architecture-policy-check"]),
        (None, [step["section"] for step in audit]),
    ):
        simulated = [
            {**step, "argv": [sys.executable, "-c",
                "raise SystemExit(19)" if step["section"] == red_section else "pass"]}
            for step in audit
        ]
        outcomes, exit_code = runner.run_steps(repo, simulated, base_env, 30)
        assert exit_code == (19 if red_section else 0)
        assert [outcome["section"] for outcome in outcomes] == expected_sections

    # An independent acceptance named by a red step still runs, the run keeps
    # the first failure's exit code, and steps after the named one do not run.
    paired = [
        {
            "section": "policy",
            "argv": [sys.executable, "-c", "raise SystemExit(19)"],
            "continue_to_section_on_failure": "ownership",
        },
        {"section": "intermediate", "argv": [sys.executable, "-c", "pass"]},
        {"section": "ownership", "argv": [sys.executable, "-c", "pass"]},
        {"section": "must-not-run", "argv": [sys.executable, "-c", "raise SystemExit(99)"]},
    ]
    outcomes, exit_code = runner.run_steps(repo, paired, base_env, 30)
    assert exit_code == 19
    assert [outcome["section"] for outcome in outcomes] == ["policy", "intermediate", "ownership"]
    both_red = [
        {
            "section": "policy",
            "argv": [sys.executable, "-c", "raise SystemExit(19)"],
            "continue_to_section_on_failure": "ownership",
        },
        {"section": "ownership", "argv": [sys.executable, "-c", "raise SystemExit(23)"]},
        {"section": "must-not-run", "argv": [sys.executable, "-c", "pass"]},
    ]
    outcomes, exit_code = runner.run_steps(repo, both_red, base_env, 30)
    assert exit_code == 19
    assert [outcome["section"] for outcome in outcomes] == ["policy", "ownership"]
    missing_target = [
        {
            "section": "policy",
            "argv": [sys.executable, "-c", "raise SystemExit(19)"],
            "continue_to_section_on_failure": "absent",
        },
        {"section": "sibling", "argv": [sys.executable, "-c", "pass"]},
    ]
    outcomes, exit_code = runner.run_steps(repo, missing_target, base_env, 30)
    assert exit_code == 19  # A target that never arrives cannot yield a green run.
    assert [outcome["section"] for outcome in outcomes] == ["policy", "sibling"]

    zero_exit_timeout = [
        {
            "section": "trapped-timeout",
            "argv": [
                sys.executable,
                "-c",
                "import signal,sys,time; signal.signal(signal.SIGTERM, lambda *_: sys.exit(0));"
                " time.sleep(30)",
            ],
        },
        {"section": "must-not-run", "argv": [sys.executable, "-c", "pass"]},
    ]
    outcomes, exit_code = runner.run_steps(repo, zero_exit_timeout, base_env, 1)
    assert exit_code == runner.RUNNER_ERROR
    assert [outcome["section"] for outcome in outcomes] == ["trapped-timeout"]

    # A terminating signal becomes an exception so the manifest is still written.
    previous = {number: signal.getsignal(number) for number in runner.INTERRUPT_SIGNALS}
    try:
        runner.install_interrupt_handlers()
        try:
            os.kill(os.getpid(), signal.SIGTERM)
        except runner.RunnerInterrupted as interrupted:
            assert interrupted.number == signal.SIGTERM
        else:
            raise AssertionError("SIGTERM did not raise RunnerInterrupted")
    finally:
        for number, handler in previous.items():
            signal.signal(number, handler)

    result = invoke(
        "final", directory / "invalid.json", {"VALIDATION_V2_CARGO_JOBS": "0"}
    )
    assert result.returncode == runner.USAGE_ERROR
    assert "must be between" in result.stderr
    unavailable = subprocess.run(
        [str(tools / "validation-v2"), "quick", "--base", "missing-ref"],
        cwd=repo,
        env=base_env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert unavailable.returncode == runner.USAGE_ERROR
    assert "is unavailable" in unavailable.stderr

    lock_dir = Path(base_env["VALIDATION_V2_LOCK_DIR"])
    lock_dir.mkdir(exist_ok=True)
    digest = hashlib.sha256(str(repo.resolve()).encode()).hexdigest()[:16]
    lock = lock_dir / f"rustycore-validation-v2-{digest}.lock"
    with lock.open("w+") as stream:
        stream.write('{"pid":999,"profile":"fixture"}')
        stream.flush()
        fcntl.flock(stream, fcntl.LOCK_EX | fcntl.LOCK_NB)
        result = invoke("quick", directory / "locked.json")
    assert result.returncode == runner.LOCKED_ERROR
    assert "lock contention" in result.stderr

    heavy_lock = directory / "host-heavy.lock"
    base_env["VALIDATION_V2_HEAVY_LOCK"] = str(heavy_lock)
    with heavy_lock.open("w+") as stream:
        stream.write(
            json.dumps(
                {
                    "run_id": "other-clone-run",
                    "repository": str(directory / "different-clone"),
                    "profile": "audit",
                }
            )
        )
        stream.flush()
        fcntl.flock(stream, fcntl.LOCK_EX | fcntl.LOCK_NB)
        result = invoke("quick", directory / "quick-with-heavy-lock.json")
        assert result.returncode == 0, result.stderr
        result = invoke("audit", directory / "audit-locked.json")
    assert result.returncode == runner.LOCKED_ERROR
    assert "other-clone-run" in result.stderr
    assert "different-clone" in result.stderr


def test_interrupt_and_verdict_contract(
    repo: Path, tools: Path, base_env: dict[str, str], directory: Path, fake_bin: Path
) -> None:
    """A killed run must leave a failed manifest, and a consumer must reject it."""
    fake_tool(fake_bin / "actionlint", 'echo VALIDATION_V2_CHILD_READY\nsleep 30\n')
    workflow = repo / ".github" / "workflows" / "slow.yml"
    workflow.parent.mkdir(parents=True, exist_ok=True)
    workflow.write_text("name: fixture\n")
    manifest = directory / "interrupted.json"
    environment = base_env.copy()
    environment["VALIDATION_V2_MANIFEST"] = str(manifest)
    environment["VALIDATION_V2_TIMEOUT_SECONDS"] = "60"
    process = subprocess.Popen(
        [str(tools / "validation-v2"), "quick", "--base", "HEAD", "--logs"],
        cwd=repo,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    watchdog = threading.Timer(120, process.kill)
    watchdog.start()
    try:
        assert process.stdout is not None
        for line in process.stdout:
            if line.strip() == "VALIDATION_V2_CHILD_READY":
                time.sleep(0.2)
                process.send_signal(signal.SIGINT)
                break
        else:
            raise AssertionError("the runner never reached the slow command")
        trailing = process.stdout.read()
        returncode = process.wait()
    finally:
        watchdog.cancel()
        if process.stdout is not None:
            process.stdout.close()
        if process.poll() is None:
            process.kill()
            process.wait()
    assert returncode == 128 + signal.SIGINT, (returncode, trailing)
    interrupted = json.loads(manifest.read_text())
    assert interrupted["status"] == "failed"
    assert interrupted["exit_code"] == 128 + signal.SIGINT
    assert interrupted["runner_signal"] == signal.SIGINT
    assert interrupted["commands"], "the interrupted command must still be recorded"
    last = interrupted["commands"][-1]
    assert last["failure_kind"] == "interrupted"
    assert last["status"] == "failed"
    assert last["argv"][0] == "actionlint"
    interrupted_logs = directory / "interrupted-logs"
    assert any("VALIDATION_V2_CHILD_READY" in log.read_text() for log in interrupted_logs.glob("*.log"))
    workflow.unlink()
    (fake_bin / "actionlint").unlink()

    def verify(target: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [str(tools / "validation-v2"), "verify", "--manifest", str(target)],
            cwd=repo,
            env=base_env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

    missing = verify(directory / "never-written.json")
    assert missing.returncode == runner.VERDICT_ERROR
    assert "is missing" in missing.stderr
    rejected = verify(manifest)
    assert rejected.returncode == runner.VERDICT_ERROR
    assert "runner died by signal" in rejected.stderr

    green_manifest = directory / "verified-green.json"
    environment["VALIDATION_V2_MANIFEST"] = str(green_manifest)
    result = subprocess.run(
        [str(tools / "validation-v2"), "quick", "--base", "HEAD"],
        cwd=repo,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert result.returncode == 0, result.stderr
    accepted = verify(green_manifest)
    assert accepted.returncode == 0, accepted.stderr
    assert runner.verify_manifest(green_manifest, "quick") == 0
    assert runner.verify_manifest(green_manifest, "final") == runner.VERDICT_ERROR
    guarded = subprocess.run(
        [str(tools / "validation-v2"), "verify", "--manifest", str(green_manifest), "--require-profile", "final"],
        cwd=repo, env=base_env, text=True, capture_output=True,
    )
    assert guarded.returncode == runner.VERDICT_ERROR
    assert "required 'final'" in guarded.stderr

    green = json.loads(green_manifest.read_text())
    unreadable = directory / "unreadable.json"
    unreadable.write_text("{not json")
    assert verify(unreadable).returncode == runner.VERDICT_ERROR

    tampered = directory / "tampered.json"
    document = json.loads(json.dumps(green))
    document["commands"][-1]["status"] = "failed"
    document["commands"][-1]["failure_kind"] = "oom"
    tampered.write_text(json.dumps(document))
    tampered_result = verify(tampered)
    assert tampered_result.returncode == runner.VERDICT_ERROR
    assert "oom" in tampered_result.stderr

    truncated = directory / "truncated.json"
    document = json.loads(json.dumps(green))
    document["commands"] = document["commands"][:-1]
    truncated.write_text(json.dumps(document))
    truncated_result = verify(truncated)
    assert truncated_result.returncode == runner.VERDICT_ERROR
    assert "planned steps were executed" in truncated_result.stderr

    stale_schema = directory / "stale-schema.json"
    document = json.loads(json.dumps(green))
    document["schema"] = runner.MANIFEST_SCHEMA - 1
    stale_schema.write_text(json.dumps(document))
    assert verify(stale_schema).returncode == runner.VERDICT_ERROR

    usage = subprocess.run(
        [str(tools / "validation-v2"), "verify"],
        cwd=repo,
        env=base_env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert usage.returncode == runner.USAGE_ERROR
    assert "requires --manifest" in usage.stderr


def test_planner_contract(repo: Path) -> None:
    metadata = synthetic_metadata(repo)
    one_crate_paths = ["crates/a/src/lib.rs"]
    one_crate_groups = runner.grouped_paths(one_crate_paths)
    workspace = runner.affected_workspace(repo, one_crate_paths, one_crate_groups, metadata)
    assert workspace == {
        "root_wide": False,
        "direct_packages": ["a"],
        "reverse_closure_packages": ["a", "b", "c"],
        "direct_library_packages": ["a"],
    }
    quick, _ = runner.validation_commands(repo, "quick", 2, "base", one_crate_groups, workspace)
    final, _ = runner.validation_commands(repo, "final", 2, "base", one_crate_groups, workspace)
    assert ["cargo", "fmt", "--all", "--check"] in quick
    assert all(command[0] != "cargo" or command[1] == "fmt" for command in quick)
    final_check = next(command for command in final if command[:2] == ["cargo", "check"])
    assert all(package in final_check for package in ("a", "b", "c"))
    final_test = next(command for command in final if command[:2] == ["cargo", "test"])
    assert "a" in final_test and "b" not in final_test and "c" not in final_test
    assert "--no-fail-fast" in final_test  # Same package/feature batch after a test failure.
    ratchet = ["python3", "tools/architecture/check_architecture.py", "hotspot-ratchet"]
    assert ratchet in final and ratchet not in quick
    physical = ["python3", "tools/architecture/check_architecture.py", "physical-files"]
    assert physical in final and physical not in quick
    # Not conditional on a workspace crate: source deletions, integrated tools
    # and generation inputs/policy changes all retain the physical gate.
    for changed in ("tools/qa.py", "tools/bot/src/main.rs", "tools/guest/test.c",
                    "crates/a/tests/deleted.rs", "data/generator-input.txt", "docs/state.md"):
        only_groups = runner.grouped_paths([changed])
        only_final, _ = runner.validation_commands(repo, "final", 2, "base", only_groups, None)
        assert physical in only_final, changed
        assert ratchet not in only_final, changed
    physical_groups = runner.grouped_paths(["tools/architecture/physical-file-policy.json"])
    physical_final, _ = runner.validation_commands(repo, "final", 2, "base", physical_groups, None)
    assert ["python3", "-m", "unittest", "discover", "-s", "tools/architecture", "-p", "test_physical_files.py"] in physical_final
    assert len({tuple(command) for command in final}) == len(final)

    # A root-wide path widens the check to the whole workspace. It must not
    # narrow the tests to nothing: the broader the change, the weaker that made
    # the gate (#364). With nothing else changed, every library is tested.
    root_groups = runner.grouped_paths(["Cargo.lock"])
    root_workspace = runner.affected_workspace(repo, ["Cargo.lock"], root_groups, metadata)
    assert root_workspace["root_wide"] is True
    assert root_workspace["direct_packages"] == ["a", "b", "c"]
    assert root_workspace["direct_library_packages"] == ["a", "b", "c"]

    # Both root config spellings affect every package and standalone manifest,
    # including deletion and mixed config/source diffs. No on-disk existence
    # check may turn a deleted configuration into text-only acceptance.
    for config_path in sorted(runner.CARGO_CONFIG_PATHS):
        for source_paths in ([], one_crate_paths):
            paths = [config_path, *source_paths]
            groups = runner.grouped_paths(paths)
            assert groups["workspace-root"] == [config_path]
            config_workspace = runner.affected_workspace(repo, paths, groups, metadata)
            assert config_workspace == {**root_workspace, "direct_library_packages": ["a", "b", "c"]}
            for mode in ("quick", "final"):
                planned, _ = runner.validation_commands(repo, mode, 1, "base", groups, config_workspace)
                assert (["cargo", "check", "--locked", "--workspace", "--all-targets", "--jobs", "1"] in planned) == (mode == "final")
                suites = [command for command in planned if command[:2] == ["cargo", "test"]]
                if mode == "final":
                    workspace_suite = next(command for command in suites if "--manifest-path" not in command)
                    assert workspace_suite == ["cargo", "test", "--no-fail-fast", "--locked", "--lib",
                                               "--jobs", "1", "-p", "a", "-p", "b", "-p", "c"]
                else:
                    assert not suites
                checker_mode = "test" if mode == "final" else "fmt"
                assert any(command[:2] == ["cargo", checker_mode] and runner.CHECKER_MANIFEST in command
                           for command in planned)
                assert any(command[:2] == ["cargo", "check" if mode == "final" else "fmt"] and runner.BOT_MANIFEST in command
                           for command in planned)
                if mode == "quick":
                    assert all(command[0] != "cargo" or command[1] == "fmt" for command in planned)
                assert len({tuple(command) for command in planned}) == len(planned)
                overlapping = runner.grouped_paths([
                    *paths, "tools/architecture/handler-contract-check/src/lib.rs",
                    "tools/wow-test-bot/src/main.rs",
                ])
                assert runner.validation_commands(repo, mode, 1, "base", overlapping, config_workspace)[0] == planned

    assert runner.classify_path("docs/config.toml") == "documentation"
    assert runner.classify_path(".cargo/unrelated-note") == "other"

    # With a crate changed alongside it, the root-wide path must not lose the
    # tests that crate would have got on its own.
    lock_and_crate = ["Cargo.lock", "crates/a/src/lib.rs"]
    lock_and_crate_groups = runner.grouped_paths(lock_and_crate)
    lock_and_crate_workspace = runner.affected_workspace(
        repo, lock_and_crate, lock_and_crate_groups, metadata
    )
    assert lock_and_crate_workspace["root_wide"] is True
    assert lock_and_crate_workspace["direct_library_packages"] == ["a"]

    # The property both cases exist for: a plan that compiles the workspace
    # must also run tests. A green gate that ran none is the defect #364 names.
    for planned_workspace, planned_groups in (
        (root_workspace, root_groups),
        (lock_and_crate_workspace, lock_and_crate_groups),
    ):
        planned, _ = runner.validation_commands(
            repo, "final", 2, "base", planned_groups, planned_workspace
        )
        assert any(
            command[:5] == ["cargo", "check", "--locked", "--workspace", "--all-targets"]
            for command in planned
        )
        assert any(command[:2] == ["cargo", "test"] for command in planned), planned
    # Changing the gate must run the gate's own contract suite, not just a
    # syntax check (#364).
    harness_suite = repo / "tools" / "test_validation_v2.py"
    harness_suite.parent.mkdir(parents=True, exist_ok=True)
    harness_suite.write_text("fixture\n")
    for changed in (["tools/validation-v2"], ["tools/test_validation_v2.py"]):
        harness_groups = runner.grouped_paths(changed)
        harness_commands, _ = runner.validation_commands(
            repo, "final", 2, "base", harness_groups, None
        )
        assert ["python3", "tools/test_validation_v2.py"] in harness_commands, harness_commands
    workflow_suite = repo / runner.WORKFLOW_CONTRACT_SUITE
    workflow_suite.write_text("fixture\n")
    for changed in ([runner.WORKFLOW_CONTRACT_SUITE], [runner.RUNNER_PATH],
                    [".github/workflows/rust-ci.yml"], [".github/workflows/validation-determinism.yml"]):
        planned, _ = runner.validation_commands(repo, "final", 1, "base", runner.grouped_paths(changed), None)
        assert ["python3", runner.WORKFLOW_CONTRACT_SUITE] in planned
    for path in (runner.BUILD_INPUT_DIAGNOSTIC, runner.BUILD_INPUT_CONTRACT_SUITE):
        planned, _ = runner.validation_commands(repo, "final", 1, "base", runner.grouped_paths([path]), None)
        assert ["python3", runner.BUILD_INPUT_CONTRACT_SUITE] in planned
        assert not any(command[0] == "cargo" for command in planned)

    docs_commands, _ = runner.validation_commands(
        repo, "quick", 2, "base", runner.grouped_paths(["docs/guide.md"]), None
    )
    assert not any(command and command[0] == "cargo" for command in docs_commands)
    assert runner.validation_commands(repo, "quick", 2, "base", {}, None)[0] == []
    quick_paths = [
        "Cargo.toml", "crates/a/src/lib.rs", "tools/validation-v2", runner.RUNNER_CONTRACT_SUITE,
        runner.WORKFLOW_CONTRACT_SUITE, runner.BUILD_INPUT_DIAGNOSTIC,
        "tools/architecture/physical-file-policy.json", "tools/architecture/check_architecture.py",
        "tools/architecture/handler-contract-check/src/lib.rs", "tools/wow-test-bot/src/main.rs",
    ]
    with patch.object(runner, "resolve_base", return_value="base"), \
         patch.object(runner, "changed_paths", return_value=quick_paths), \
         patch.object(runner, "cargo_metadata", side_effect=AssertionError("quick must not resolve dependencies")), \
         patch.object(runner, "require_protoc_for", side_effect=AssertionError("quick must not require protobuf")):
        quick_plan = runner.build_plan(repo, "quick", 1, "base", {}, 30)
    assert quick_plan["workspace"] is None and quick_plan["metadata"] is None
    for command in quick_plan["planned_commands"]:
        assert command[0] != "cargo" or command[1] == "fmt", command
        assert command[:2] != ["python3", runner.RUNNER_CONTRACT_SUITE], command
        assert command[:2] != ["python3", runner.WORKFLOW_CONTRACT_SUITE], command
        assert command[:2] != ["python3", runner.BUILD_INPUT_CONTRACT_SUITE], command
        assert "unittest" not in command and "self-test" not in command, command
    assert runner.validation_commands(repo, "none", 1, "base", runner.grouped_paths(quick_paths), workspace)[0] == []

    # No profile may silence a checker test by name. A skipped test is a test
    # that rots: #363 was nine stale literals hiding behind `--skip`.
    checker_groups = runner.grouped_paths(
        ["tools/architecture/handler-contract-check/src/lib.rs"]
    )
    checker_commands, _ = runner.validation_commands(
        repo, "final", 2, "base", checker_groups, None
    )
    checker_test = next(
        command
        for command in checker_commands
        if command[:2] == ["cargo", "test"] and "handler-contract-check" in " ".join(command)
    )
    assert "--skip" not in checker_test, checker_test

    audit = runner.audit_steps("base", 2)
    for _ in range(10):
        assert audit == runner.audit_steps("base", 2)
    assert len({tuple(step["argv"]) for step in audit}) == len(audit)
    assert not any("--skip" in step["argv"] for step in audit), audit
    assert [step["section"] for step in audit] == [
        "diff-hygiene",
        "architecture-policy-check",
        "workspace-format",
        "handler-contract-format",
        "qa-bot-format",
        "handler-contract-unit-tests",
        "handler-contract-repository-check",
        "session-persistence-ratchet",
        "qa-bot-tests",
        "workspace-all-target-tests",
        "world-modules-launcher-check",
        "capture-loot-contract",
        "capture-creature-spell-contract",
    ]
    policy_step = next(step for step in audit if step["section"] == "architecture-policy-check")
    assert all("continue_to_section_on_failure" not in step for step in audit)
    assert policy_step["argv"] == ["python3", "tools/architecture/check_architecture.py", "check", "--self-test"]
    assert sum("check_architecture.py" in " ".join(step["argv"]) for step in audit) == 1
    assert all("--no-fail-fast" in step["argv"] for step in audit if step["argv"][:2] == ["cargo", "test"])
    assert not any("check_architecture.py" in " ".join(command) for command in quick)
    assert not any("session-ownership-check" in " ".join(command) for command in final)

    mixed_paths = [
        "tools/check.sh",
        "tools/second.sh",
        "tools/policy.json",
        "tools/check.py",
        ".github/workflows/check.yml",
        "tools/architecture/handler-contract-check/src/lib.rs",
        "tools/architecture/handler-contract-check/policy.json",
        "tools/wow-test-bot/src/main.rs",
        "unclassified.asset",
    ]
    for path in mixed_paths[:-1]:
        fixture = repo / path
        fixture.parent.mkdir(parents=True, exist_ok=True)
        fixture.write_text("{}\n" if fixture.suffix == ".json" else "fixture\n")
    groups = runner.grouped_paths(mixed_paths)
    assert set(groups) == {
        "architecture-checker", "json", "other", "python", "shell", "workflow", "wow-test-bot"
    }
    commands, _ = runner.validation_commands(repo, "final", 2, "base", groups, None)
    assert len([command for command in commands if command[:2] == ["bash", "-n"]]) == 2
    json_command = next(
        command
        for command in commands
        if command[:2] == ["python3", "-c"] and "import json" in command[2]
    )
    assert "tools/policy.json" in json_command
    assert "tools/architecture/handler-contract-check/policy.json" in json_command
    assert any(command[:2] == ["cargo", "fmt"] and "handler-contract-check" in " ".join(command) for command in commands)
    assert any(command[:2] == ["cargo", "fmt"] and "wow-test-bot" in " ".join(command) for command in commands)

    # A crate that is gone from disk and from the resolved workspace was removed,
    # and a removal can affect anything: plan it root-wide rather than refuse.
    removed_paths = ["crates/removed/Cargo.toml", "crates/removed/src/lib.rs"]
    removed_groups = runner.grouped_paths(removed_paths)
    removed_workspace = runner.affected_workspace(repo, removed_paths, removed_groups, metadata)
    assert removed_workspace["root_wide"] is True
    assert removed_workspace["direct_packages"] == ["a", "b", "c"]
    removed_commands, _ = runner.validation_commands(
        repo, "final", 2, "base", removed_groups, removed_workspace
    )
    assert any(
        command[:5] == ["cargo", "check", "--locked", "--workspace", "--all-targets"]
        for command in removed_commands
    )
    # A removal resolves to no package, so there is nothing narrower to keep:
    # it tests every library rather than none (#364).
    assert removed_workspace["direct_library_packages"] == ["a", "b", "c"]
    assert any(command[:2] == ["cargo", "test"] for command in removed_commands)

    # A manifest deleted while its package still resolves is an inconsistent
    # tree, and still fails closed.
    # A source file that resolves to no package but is still on disk stays an
    # error: that is an unclassified path, not a removal.
    present_unclassified = repo / "crates" / "stray.rs"
    present_unclassified.write_text("// not part of any package\n")
    try:
        runner.affected_workspace(
            repo,
            ["crates/stray.rs"],
            runner.grouped_paths(["crates/stray.rs"]),
            metadata,
        )
    except ValueError as error:
        assert "maps to 0 packages" in str(error), error
    else:
        raise AssertionError("an unclassified present path did not fail closed")
    present_unclassified.unlink()

    deleted_paths = ["crates/a/Cargo.toml"]
    (repo / "crates" / "a" / "Cargo.toml").unlink()
    deleted_groups = runner.grouped_paths(deleted_paths)
    try:
        runner.affected_workspace(repo, deleted_paths, deleted_groups, metadata)
    except ValueError as error:
        assert "deleted workspace manifest" in str(error)
    else:
        raise AssertionError("deleted workspace manifest did not fail closed")


def test_cargo_target_contract(
    repo: Path, base_env: dict[str, str], directory: Path
) -> None:
    """Cargo targets are worktree-local by default and explicit paths are resolved once."""
    repository = repo.resolve()

    probe_bin = directory / "cargo-probe-bin"
    probe_bin.mkdir()
    fake_tool(
        probe_bin / "cargo",
        'printf "%s|%s\\n" "$CARGO_TARGET_DIR" "$CARGO_BUILD_JOBS" >> "$VALIDATION_V2_CARGO_ENV_LOG"\n'
        'if [ "${1-}" = "metadata" ]; then\n'
        '    printf \'{"workspace_members": [], "packages": [], "resolve": {"nodes": []}}\\n\'\n'
        "fi\n",
    )

    def effective(configured: str | None) -> dict[str, str]:
        environment = base_env.copy()
        environment["PATH"] = f"{probe_bin}:{base_env['PATH']}"
        if configured is None:
            environment.pop("CARGO_TARGET_DIR", None)
        else:
            environment["CARGO_TARGET_DIR"] = configured
        with patch.dict(os.environ, environment, clear=True):
            return runner.command_environment(repo, runner.DEFAULT_JOBS)

    def probe(configured: str | None, expected: Path, name: str) -> None:
        environment = effective(configured)
        log = directory / f"cargo-env-{name}.log"
        environment["VALIDATION_V2_CARGO_ENV_LOG"] = str(log)
        metadata, metadata_outcome = runner.cargo_metadata(repo, environment, 30)
        assert metadata == {"workspace_members": [], "packages": [], "resolve": {"nodes": []}}
        assert metadata_outcome["status"] == "passed"
        metadata_observed = log.read_text().strip()
        command_outcome = runner.run_one(repo, ["cargo", "check"], environment, 30)
        assert command_outcome["status"] == "passed"
        command_observed = log.read_text().splitlines()[-1]
        assert metadata_observed == command_observed == f"{expected}|1"

    default = effective(None)
    assert default["CARGO_TARGET_DIR"] == str(repository / "target")
    assert default["CARGO_BUILD_JOBS"] == "1"
    with patch.dict(os.environ, base_env, clear=True), patch.object(
        runner, "resolve_protoc", side_effect=AssertionError("self-test must not probe host protoc")
    ):
        hermetic = runner.command_environment(repo, 1, resolve_protobuf=False)
    assert "PROTOC" not in hermetic
    probe(None, repository / "target", "default")

    for blank_value in ("", "   "):
        blank = effective(blank_value)
        assert blank["CARGO_TARGET_DIR"] == default["CARGO_TARGET_DIR"]

    absolute_target = directory / "explicit-target"
    absolute = effective(str(absolute_target))
    assert absolute["CARGO_TARGET_DIR"] == str(absolute_target.resolve())
    probe(str(absolute_target), absolute_target.resolve(), "absolute")

    relative = effective("explicit-target")
    assert relative["CARGO_TARGET_DIR"] == str((repository / "explicit-target").resolve())
    probe("explicit-target", (repository / "explicit-target").resolve(), "relative")

    worktree_a = directory / "worktree-a"
    worktree_b = directory / "worktree-b"
    default_a = runner.resolve_cargo_target_dir(worktree_a)
    default_b = runner.resolve_cargo_target_dir(worktree_b)
    assert default_a == worktree_a.resolve() / "target"
    assert default_b == worktree_b.resolve() / "target"
    assert default_a != default_b

    relative_a = runner.resolve_cargo_target_dir(worktree_a, "validation-target")
    relative_b = runner.resolve_cargo_target_dir(worktree_b, "validation-target")
    assert relative_a == worktree_a.resolve() / "validation-target"
    assert relative_b == worktree_b.resolve() / "validation-target"
    assert relative_a != relative_b


def test_architecture_plan_contract() -> None:
    checker = ["python3", "tools/architecture/check_architecture.py"]
    retained = [
        ["git", "diff", "--check", "base"],
        ["cargo", "check", "--tests", "-p", "consumer"],
        ["cargo", "test", "--lib", "-p", "owner"],
        [*checker, "physical-files", "--terminal"],
        ["cargo", "run", "--", "check"],  # Exhaustive/other checks are not waived.
    ]
    commands = [*retained, *[[*checker, mode] for mode in (
        "physical-files", "hotspot-ratchet", "self-test", "check",
    )]]
    for original in (commands, []):
        plan = {"planned_steps": [
            {"section": str(index), "argv": command.copy()}
            for index, command in enumerate(original)
        ], "planned_commands": []}
        runner.enable_architecture_acceptance(plan, 2)
        first = [step.copy() for step in plan["planned_steps"]]
        runner.enable_architecture_acceptance(plan, 2)
        assert plan["planned_steps"] == first
        actual = plan["planned_commands"]
        assert actual == [step["argv"] for step in first]
        # The policy step names the ownership step, so a ratchet breach cannot
        # hide the ownership verdict.
        assert first[0]["continue_to_section_on_failure"] == "session-syntax-acceptance"
        assert all("continue_to_section_on_failure" not in step for step in first[1:])
        assert actual[0] == [*checker, "check", "--self-test"]
        assert actual[1] == [
            "cargo", "run", "--release", "--locked", "--manifest-path", runner.CHECKER_MANIFEST,
            "--bin", "session-ownership-check", "--jobs", "2", "--", "check", "--syntax-only",
        ]
        assert actual[2:] == (retained if original else [])
        runner.enable_cargo_timings(plan)
        assert "--timings" in plan["planned_commands"][1]


def test_timing_plan_contract() -> None:
    commands = [
        ["cargo", "check", "--locked", "--tests", "-p", "consumer"],
        ["cargo", "test", "--lib", "-p", "owner", "--", "--exact", "case"],
        ["cargo", "run", "--release", "--", "--timings"],
        ["cargo", "build", "--timings"],
        ["cargo", "fmt", "--all", "--check"],
        ["python3", "checker.py"],
    ]
    originals = [command.copy() for command in commands]
    plan = {"planned_steps": [
        {"section": str(index), "argv": command}
        for index, command in enumerate(commands)
    ], "planned_commands": [command.copy() for command in commands]}
    runner.enable_cargo_timings(plan)
    runner.enable_cargo_timings(plan)  # Idempotent; no duplicate runs or Cargo flags.
    assert len(plan["planned_steps"]) == len(originals)
    assert plan["planned_commands"] == [step["argv"] for step in plan["planned_steps"]]
    for before, after in zip(originals, plan["planned_commands"]):
        if before[:2] in (["cargo", "check"], ["cargo", "test"], ["cargo", "run"], ["cargo", "build"]):
            boundary = after.index("--") if "--" in after else len(after)
            assert after[:boundary].count("--timings") == 1
            restored = after.copy()
            if "--timings" not in before[:before.index("--") if "--" in before else len(before)]:
                restored.pop(restored.index("--timings"))
            assert restored == before  # Targets/features and program arguments are retained.
        else:
            assert after == before


def test_empty_scope_contract(repo: Path, tools: Path, environment: dict[str, str], directory: Path) -> None:
    for profile in ("quick", "final", "2", "3"):
        manifest = directory / f"empty-{profile}.json"
        result = subprocess.run(
            [str(tools / "validation-v2"), profile, "--base", "HEAD", "--require-changes"],
            cwd=repo, env={**environment, "VALIDATION_V2_MANIFEST": str(manifest)},
            capture_output=True, text=True,
        )
        assert result.returncode == runner.USAGE_ERROR, (result.stdout, result.stderr)
        recorded = json.loads(manifest.read_text())
        assert recorded["profile"] == runner.LEVEL_PROFILES.get(profile, profile)
        assert "empty validation scope" in recorded["runner_error"]
        assert recorded["commands"] == []
        assert recorded["plan"]["require_changes"] is True
        assert runner.manifest_problems(recorded)
    result = subprocess.run(
        [str(tools / "validation-v2"), "quick", "--base", "HEAD"], cwd=repo,
        env={**environment, "VALIDATION_V2_MANIFEST": str(directory / "empty-allowed.json")},
        capture_output=True, text=True,
    )
    assert result.returncode == 0
    assert "NO CHANGED PATHS" in result.stdout


def test_evidence_contract(repo: Path, environment: dict[str, str], directory: Path) -> None:
    logs = runner.create_log_directory(directory / "evidence.json")
    steps = [
        {"section": "first", "argv": [sys.executable, "-c", "print('first output'); raise SystemExit(19)"]},
        {"section": "second", "argv": [sys.executable, "-c", "print('second output'); raise SystemExit(23)"]},
        {"section": "third", "argv": [sys.executable, "-c", "print('last output')"]},
    ]
    results, code = runner.run_steps(repo, steps, environment, 30, log_directory=logs, keep_going=True)
    assert code == 19
    assert [r["status"] for r in results] == ["failed", "failed", "passed"]
    for index, (step, message) in enumerate(zip(steps, ("first output", "second output", "last output")), 1):
        log = logs / f"{index:02d}-{step['section']}.log"
        contents = log.read_text()
        assert message in contents
        assert json.loads(contents.splitlines()[0])["argv"] == step["argv"]
    # No caller may turn a diagnostic red run into a green manifest.
    assert runner.manifest_problems({"schema": runner.MANIFEST_SCHEMA, "runner": "rustycore-validation-v2",
                                    "status": "passed", "exit_code": 0, "commands": results,
                                    "plan": {"planned_steps": steps}})
    for reason in ("oom", "timeout", "signal", "child-signal", "interrupted", "output-log"):
        def fatal(*args, sink=None, **kwargs):
            outcome = {"status": "failed", "exit_code": 137, "failure_kind": reason, "duration_seconds": 0}
            sink.append(outcome)
            return outcome
        with patch.object(runner, "run_one", side_effect=fatal):
            results, code = runner.run_steps(repo, steps, environment, 30, keep_going=True)
        assert code == 137 and len(results) == 1, reason
    # Logs reject symlink/stale targets rather than truncating arbitrary files.
    sentinel = directory / "sentinel.txt"
    sentinel.write_text("preserve")
    link = logs / "symlink.log"
    link.symlink_to(sentinel)
    try:
        runner.run_one(repo, [sys.executable, "-c", "pass"], environment, 30, log_path=link)
    except FileExistsError:
        pass
    else:
        raise AssertionError("log symlink was accepted")
    assert sentinel.read_text() == "preserve"
    # A logging I/O failure is non-green even when the child succeeds.
    original_tee = runner.tee_output
    def broken_log(stream, reports, log, errors):
        original_tee(stream, reports, None, errors)
        errors.append("fixture log full")
    with patch.object(runner, "tee_output", side_effect=broken_log):
        result = runner.run_one(repo, [sys.executable, "-c", "pass"], environment, 30,
                                log_path=logs / "full.log")
    assert result["status"] == "failed" and result["failure_kind"] == "output-log"
    assert runner.failed_exit_code(result) == runner.RUNNER_ERROR


def main() -> None:
    test_no_validation_level()
    test_architecture_plan_contract()
    test_timing_plan_contract()
    with tempfile.TemporaryDirectory(prefix="validation-v2-self-test-") as raw_directory:
        directory = Path(raw_directory)
        repo = directory / "repo"
        tools = repo / "tools"
        fake_bin = directory / "bin"
        tools.mkdir(parents=True)
        fake_bin.mkdir()
        shutil.copy2(RUNNER_PATH, tools / "validation-v2")
        (repo / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "1.98.0"\n')
        (repo / ".protoc-version").write_text("28.3\n")
        (repo / "Cargo.toml").write_text('[workspace]\nresolver = "2"\n')
        subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
        subprocess.run(["git", "config", "user.email", "validation-v2@example.invalid"], cwd=repo, check=True)
        subprocess.run(["git", "config", "user.name", "Validation V2"], cwd=repo, check=True)
        subprocess.run(["git", "add", "."], cwd=repo, check=True)
        subprocess.run(["git", "commit", "-qm", "fixture"], cwd=repo, check=True)
        fake_tool(fake_bin / "rustc", 'printf "rustc 1.98.0 (fixture 1970-01-01)\\n"\n')
        fake_tool(fake_bin / "protoc", 'printf "libprotoc 28.3\\n"\n')
        environment = os.environ.copy()
        environment["PATH"] = f"{fake_bin}:{environment['PATH']}"
        environment.pop("CARGO_TARGET_DIR", None)
        environment.pop("VALIDATION_V2_CARGO_JOBS", None)
        environment.pop("PROTOC", None)
        environment["VALIDATION_V2_LOCK_DIR"] = str(directory / "locks")
        test_empty_scope_contract(repo, tools, environment, directory)
        test_evidence_contract(repo, environment, directory)
        test_cargo_target_contract(repo, environment, directory)
        test_runner_contract(repo, tools, environment, directory)
        test_interrupt_and_verdict_contract(repo, tools, environment, directory, fake_bin)
        test_planner_contract(repo)
    print("validation-v2 self-test passed")


if __name__ == "__main__":
    main()
