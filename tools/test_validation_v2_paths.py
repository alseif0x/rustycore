"""Focused path-selection contracts for the Validation V2 runner."""

from pathlib import Path
import subprocess
import tempfile


def _git(repo: Path, *arguments: str) -> bytes:
    return subprocess.check_output(["git", *arguments], cwd=repo)


def _assert_relocation_paths(runner, repo: Path, base_commit: str, old: str, new: str) -> None:
    paths = runner.changed_paths(repo, base_commit)
    assert paths == sorted((old, new)), paths
    assert runner.grouped_paths(paths) == {"workspace-rust": [old], "documentation": [new]}


def test_changed_path_rename_contract(runner) -> None:
    old = "crates/example space/src/lib.rs"
    new = "docs/example renamed.txt"
    with tempfile.TemporaryDirectory(prefix="validation-v2-paths-") as raw_repo:
        repo = Path(raw_repo)
        old_file = repo / old
        new_file = repo / new
        old_file.parent.mkdir(parents=True)
        new_file.parent.mkdir(parents=True)
        old_file.write_text("workspace source\n")
        _git(repo, "init", "-q")
        _git(repo, "config", "user.email", "validation-v2@example.invalid")
        _git(repo, "config", "user.name", "Validation V2")
        _git(repo, "config", "diff.renames", "true")
        _git(repo, "add", old)
        _git(repo, "commit", "-qm", "base")
        base_commit = _git(repo, "rev-parse", "HEAD").decode().strip()

        old_file.rename(new_file)
        _git(repo, "add", "--intent-to-add", new)
        assert _git(repo, "diff", "--cached", "--name-only", "-z") == b""
        assert _git(repo, "ls-files", "--others", "--exclude-standard", "-z") == b""
        unstaged_status = _git(repo, "diff", "--name-status")
        assert unstaged_status == f"R100\t{old}\t{new}\n".encode(), unstaged_status
        _assert_relocation_paths(runner, repo, base_commit, old, new)

        _git(repo, "add", "-A")
        staged_names = _git(repo, "diff", "--cached", "--name-only", "-z")
        assert staged_names == f"{new}\0".encode(), staged_names
        _assert_relocation_paths(runner, repo, base_commit, old, new)

        _git(repo, "commit", "-qm", "relocate source into docs")
        committed_names = _git(repo, "diff", "--name-only", "-z", f"{base_commit}...HEAD")
        assert committed_names == f"{new}\0".encode(), committed_names
        _assert_relocation_paths(runner, repo, base_commit, old, new)

    with tempfile.TemporaryDirectory(prefix="validation-v2-path-plan-") as raw_repo:
        repo = Path(raw_repo)
        helper_path = runner.RUNNER_PATHS_TEST
        helper_file = repo / helper_path
        helper_file.parent.mkdir(parents=True)
        helper_file.write_text("# fixture\n")
        suite_file = repo / runner.RUNNER_CONTRACT_SUITE
        suite_file.write_text("# fixture\n")
        suite_command = ["python3", runner.RUNNER_CONTRACT_SUITE]
        groups = runner.grouped_paths([helper_path])
        final, _ = runner.validation_commands(repo, "final", 1, "base", groups, None)
        quick, _ = runner.validation_commands(repo, "quick", 1, "base", groups, None)
        assert suite_command in final, final
        assert suite_command not in quick, quick
