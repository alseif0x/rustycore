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
    """Remove fields that are expected to vary between otherwise identical runs."""
    volatile = {
        "run_id",
        "started_at",
        "ended_at",
        "duration_seconds",
        "peak_child_rss_kib",
    }
    if isinstance(value, dict):
        return {
            key: stable_manifest(item)
            for key, item in value.items()
            if key not in volatile
        }
    if isinstance(value, list):
        return [stable_manifest(item) for item in value]
    return value


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
    direct_signal = runner.run_one(
        repo,
        [sys.executable, "-c", "import os,signal; os.kill(os.getpid(), signal.SIGTERM)"],
        base_env,
        30,
    )
    assert direct_signal["exit_code"] == 128 + signal.SIGTERM
    assert direct_signal["signal"] == signal.SIGTERM
    direct_timeout = runner.run_one(
        repo, [sys.executable, "-c", "import time; time.sleep(10)"], base_env, 1
    )
    assert direct_timeout["timed_out"] is True
    assert direct_timeout["exit_code"] == 128 + signal.SIGTERM

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

    deleted_groups = runner.grouped_paths(["crates/deleted/Cargo.toml"])
    try:
        runner.affected_workspace(repo, ["crates/deleted/Cargo.toml"], deleted_groups, metadata)
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
        (repo / "Cargo.toml").write_text('[workspace]\nresolver = "2"\n')
        subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
        subprocess.run(["git", "config", "user.email", "validation-v2@example.invalid"], cwd=repo, check=True)
        subprocess.run(["git", "config", "user.name", "Validation V2"], cwd=repo, check=True)
        subprocess.run(["git", "add", "."], cwd=repo, check=True)
        subprocess.run(["git", "commit", "-qm", "fixture"], cwd=repo, check=True)
        fake_tool(fake_bin / "rustc", 'printf "rustc 1.98.0 (fixture 1970-01-01)\\n"\n')
        environment = os.environ.copy()
        environment["PATH"] = f"{fake_bin}:{environment['PATH']}"
        environment["VALIDATION_V2_LOCK_DIR"] = str(directory / "locks")
        test_runner_contract(repo, tools, environment, directory)
        test_planner_contract(repo)
    print("validation-v2 self-test passed")


if __name__ == "__main__":
    main()
