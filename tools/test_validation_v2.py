#!/usr/bin/env python3
"""Hermetic contract tests for the shadow Validation V2 runner."""

from __future__ import annotations

import fcntl
import hashlib
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


def test_runner_contract(repo: Path, tools: Path, base_env: dict[str, str], directory: Path) -> None:
    (repo / "docs").mkdir()
    (repo / "docs" / "guide.md").write_text("fixture\n")

    def invoke(mode: str, manifest: Path, extra: dict[str, str] | None = None):
        environment = base_env.copy()
        environment["VALIDATION_V2_MANIFEST"] = str(manifest)
        if extra:
            environment.update(extra)
        return subprocess.run(
            [str(tools / "validation-v2"), mode, "--base", "HEAD"],
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
    assert success["resources"]["cargo_jobs"] == 2
    assert "memory_limit_kib" in success["resources"]
    assert success["profile"] == "quick"
    assert len(success["run_id"]) == 20
    assert success["provenance"]["rust"] == {"active": "1.98.0", "pinned": "1.98.0"}
    assert success["plan"]["changed_paths"] == ["docs/guide.md"]
    assert success["plan"]["workspace"] is None
    assert len(success["commands"]) == 2
    assert all(command["status"] == "passed" for command in success["commands"])
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
    invoke("audit", audit_manifest)
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
        [str(tools / "validation-v2"), "quick", "--base", "HEAD"],
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
    assert any(command[:5] == ["cargo", "check", "--locked", "--tests", "--jobs"] for command in quick)
    assert not any(command[:2] == ["cargo", "test"] for command in quick)
    final_check = next(command for command in final if command[:2] == ["cargo", "check"])
    assert all(package in final_check for package in ("a", "b", "c"))
    final_test = next(command for command in final if command[:2] == ["cargo", "test"])
    assert "a" in final_test and "b" not in final_test and "c" not in final_test
    ratchet = ["python3", "tools/architecture/check_architecture.py", "hotspot-ratchet"]
    assert ratchet in final and ratchet not in quick
    assert len({tuple(command) for command in final}) == len(final)

    root_groups = runner.grouped_paths(["Cargo.lock"])
    root_workspace = runner.affected_workspace(repo, ["Cargo.lock"], root_groups, metadata)
    assert root_workspace["root_wide"] is True
    assert root_workspace["direct_packages"] == ["a", "b", "c"]
    assert root_workspace["direct_library_packages"] == []
    docs_commands, _ = runner.validation_commands(
        repo, "quick", 2, "base", runner.grouped_paths(["docs/guide.md"]), None
    )
    assert not any(command and command[0] == "cargo" for command in docs_commands)
    assert runner.validation_commands(repo, "quick", 2, "base", {}, None)[0] == []

    audit = runner.audit_steps("base", 2)
    for _ in range(10):
        assert audit == runner.audit_steps("base", 2)
    assert len({tuple(step["argv"]) for step in audit}) == len(audit)
    assert [step["section"] for step in audit] == [
        "diff-hygiene",
        "architecture-policy-self-test",
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


def main() -> None:
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
        environment["VALIDATION_V2_LOCK_DIR"] = str(directory / "locks")
        test_runner_contract(repo, tools, environment, directory)
        test_interrupt_and_verdict_contract(repo, tools, environment, directory, fake_bin)
        test_planner_contract(repo)
    print("validation-v2 self-test passed")


if __name__ == "__main__":
    main()
