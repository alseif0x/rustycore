"""Cargo environment and lifecycle contracts for Validation V2."""

import os
from pathlib import Path
import sys
import time
from unittest.mock import patch


def test_cargo_target_contract(
    runner, fake_tool, repo: Path, base_env: dict[str, str], directory: Path
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


def test_cargo_metadata_timeout(runner, repo: Path, base_env: dict[str, str], directory: Path) -> None:
    probe_bin = directory / "cargo-timeout-bin"
    probe_bin.mkdir()
    ready = directory / "metadata-child-ready"
    result = directory / "metadata-child-result"
    cargo = probe_bin / "cargo"
    cargo.write_text(
        f"#!{sys.executable}\n"
        + """import os
from pathlib import Path
import signal
import time

if os.fork() == 0:
    result = Path(os.environ["METADATA_CHILD_RESULT"])

    def report(state):
        temporary = result.with_suffix(".tmp")
        temporary.write_text(state)
        temporary.replace(result)

    def stop(_number, _frame):
        report("terminated")
        os._exit(0)

    signal.signal(signal.SIGTERM, stop)
    Path(os.environ["METADATA_CHILD_READY"]).touch()
    time.sleep(3)
    report("survived")
    os._exit(1)

os.wait()
"""
    )
    cargo.chmod(0o755)
    environment = {
        **base_env,
        "PATH": f"{probe_bin}:{base_env['PATH']}",
        "METADATA_CHILD_READY": str(ready),
        "METADATA_CHILD_RESULT": str(result),
    }
    try:
        runner.cargo_metadata(repo, environment, 2)
    except ValueError as error:
        assert str(error) == "Cargo metadata exceeded 2 seconds", error
    else:
        raise AssertionError("cargo metadata should have timed out")

    deadline = time.monotonic() + 4
    while not result.exists() and time.monotonic() < deadline:
        time.sleep(0.02)
    assert ready.exists(), "the metadata child must start before timeout"
    assert result.exists(), "the test child did not report its final state"
    assert result.read_text() == "terminated", "cargo metadata timeout left its child running"

    if sys.platform == "linux":
        _test_cargo_metadata_timeout_stubborn_descendant(runner, repo, base_env, directory)


def _test_cargo_metadata_timeout_stubborn_descendant(
    runner, repo: Path, base_env: dict[str, str], directory: Path
) -> None:
    probe_bin = directory / "cargo-timeout-stubborn-bin"
    probe_bin.mkdir()
    identity = directory / "metadata-stubborn-child-identity"
    leader_exit = directory / "metadata-leader-exited"
    cargo = probe_bin / "cargo"
    cargo.write_text(
        f"#!{sys.executable}\n"
        + """import os
from pathlib import Path
import signal
import time

def stop_leader(_number, _frame):
    Path(os.environ["METADATA_LEADER_EXIT"]).touch()
    os._exit(0)

signal.signal(signal.SIGTERM, stop_leader)

if os.fork() == 0:
    null_fd = os.open(os.devnull, os.O_RDWR)
    for descriptor in (0, 1, 2):
        os.dup2(null_fd, descriptor)
    if null_fd > 2:
        os.close(null_fd)
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    Path(os.environ["METADATA_CHILD_IDENTITY"]).write_text(
        f"{os.getpid()} {os.getpgrp()}"
    )
    time.sleep(8)
    os._exit(0)

os.wait()
"""
    )
    cargo.chmod(0o755)
    environment = {
        **base_env,
        "PATH": f"{probe_bin}:{base_env['PATH']}",
        "METADATA_CHILD_IDENTITY": str(identity),
        "METADATA_LEADER_EXIT": str(leader_exit),
    }
    with patch.object(runner, "TERMINATE_GRACE_SECONDS", 0.2):
        try:
            runner.cargo_metadata(repo, environment, 2)
        except ValueError as error:
            assert str(error) == "Cargo metadata exceeded 2 seconds", error
        else:
            raise AssertionError("cargo metadata should have timed out")

    assert identity.exists(), "the metadata child must start before timeout"
    child_pid, process_group = map(int, identity.read_text().split())

    def child_is_live() -> bool:
        try:
            stat = Path(f"/proc/{child_pid}/stat").read_bytes()
        except (FileNotFoundError, ProcessLookupError):
            return False
        state, _, group, _ = stat.rsplit(b") ", 1)[1].split(maxsplit=3)
        return group == str(process_group).encode() and state not in (b"Z", b"X")

    deadline = time.monotonic() + 2
    while child_is_live() and time.monotonic() < deadline:
        time.sleep(0.02)
    if child_is_live():
        # Let the bounded fixture finish before failing this regression.
        deadline = time.monotonic() + 7
        while child_is_live() and time.monotonic() < deadline:
            time.sleep(0.02)
        raise AssertionError("metadata timeout left a live group child")
    assert leader_exit.exists(), "the metadata leader must exit on SIGTERM"
