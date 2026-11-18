"""Focused path-selection contracts for the Validation V2 runner."""

from pathlib import Path
import subprocess
import tempfile
from unittest.mock import patch


def _git(repo: Path, *arguments: str) -> bytes:
    return subprocess.check_output(["git", *arguments], cwd=repo)


def _assert_relocation_paths(runner, repo: Path, base_commit: str, old: str, new: str) -> None:
    paths = runner.changed_paths(repo, base_commit)
    assert paths == sorted((old, new)), paths
    assert runner.grouped_paths(paths) == {"workspace-rust": [old], "documentation": [new]}


def _assert_diverged_base_contract(runner) -> None:
    with tempfile.TemporaryDirectory(prefix="validation-v2-diverged-base-") as raw_repo:
        repo = Path(raw_repo)
        cleanup = repo / "docs" / "cleanup.txt"
        manifest = repo / "crates" / "consumer" / "Cargo.toml"
        source = repo / "crates" / "consumer" / "src" / "lib.rs"
        dependency = repo / "crates" / "dependency" / "Cargo.toml"
        for path in (cleanup, manifest, source, dependency):
            path.parent.mkdir(parents=True, exist_ok=True)
        cleanup.write_text("line with trailing space  \n")
        manifest.write_text(
            '[package]\nname = "consumer"\nversion = "0.1.0"\nedition = "2024"\n'
        )
        source.write_text("pub fn base() {}\n")
        dependency.write_text(
            '[package]\nname = "dependency"\nversion = "0.1.0"\nedition = "2024"\n'
        )
        _git(repo, "init", "-q")
        _git(repo, "config", "user.email", "validation-v2@example.invalid")
        _git(repo, "config", "user.name", "Validation V2")
        _git(repo, "add", "-A")
        _git(repo, "commit", "-qm", "common base")
        common = _git(repo, "rev-parse", "HEAD").decode().strip()

        dependency_manifest = (
            '[package]\nname = "consumer"\nversion = "0.1.0"\nedition = "2024"\n\n'
            '[dependencies]\ndependency = { path = "../dependency" }\n'
        )
        _git(repo, "checkout", "-qb", "target")
        cleanup.write_text("line with trailing space\n")
        manifest.write_text(dependency_manifest)
        _git(repo, "add", "-A")
        _git(repo, "commit", "-qm", "target updates")
        target = _git(repo, "rev-parse", "HEAD").decode().strip()

        _git(repo, "checkout", "-qb", "candidate", common)
        manifest.write_text(dependency_manifest)
        source.write_text("pub fn base() {}\npub fn candidate() {}\n")
        _git(repo, "add", "-A")
        _git(repo, "commit", "-qm", "candidate updates")
        metadata = {
            "workspace_members": ["consumer", "dependency"],
            "packages": [
                {
                    "id": name,
                    "name": name,
                    "manifest_path": str(repo / "crates" / name / "Cargo.toml"),
                    "targets": [{"kind": ["lib"]}],
                }
                for name in ("consumer", "dependency")
            ],
            "resolve": {
                "nodes": [
                    {"id": "consumer", "dependencies": ["dependency"]},
                    {"id": "dependency", "dependencies": []},
                ]
            },
        }
        with patch.object(
            runner,
            "cargo_metadata",
            return_value=(metadata, {"status": "passed", "exit_code": 0}),
        ):
            environment = {"PROTOC": "/fixture/protoc"}
            quick = runner.build_plan(repo, "quick", 1, target, environment, 30)
            final = runner.build_plan(repo, "final", 1, target, environment, 30)
            audit = runner.build_plan(repo, "audit", 1, target, environment, 30)

        for plan in (quick, final, audit):
            assert plan["base_commit"] == target
            assert plan["merge_base"] == common
            assert "crates/consumer/Cargo.toml" in plan["changed_paths"]
        expected_diff_check = ["git", "diff", "--check", common]
        assert expected_diff_check in quick["planned_commands"]
        assert expected_diff_check in final["planned_commands"]
        audit_hygiene = next(
            step["argv"] for step in audit["planned_steps"]
            if step["section"] == "diff-hygiene"
        )
        assert audit_hygiene == expected_diff_check

        checker_test = [
            "cargo", "test", "--locked", *runner.CHECKER_CARGO, "--jobs", "1",
            "--lib", "--no-fail-fast",
        ]
        assert checker_test not in quick["planned_commands"]
        assert checker_test in final["planned_commands"]
        assert not runner.handler_graph_changed(repo, target, ["crates/consumer/Cargo.toml"])
        assert runner.handler_graph_changed(repo, common, ["crates/consumer/Cargo.toml"])

        target_relative = subprocess.run(
            ["git", "diff", "--check", target], cwd=repo, capture_output=True
        )
        assert target_relative.returncode != 0, target_relative
        common_relative = subprocess.run(expected_diff_check, cwd=repo, capture_output=True)
        assert common_relative.returncode == 0, common_relative

        source.write_text("pub fn base() {}\npub fn candidate() {}  \n")
        candidate_whitespace = subprocess.run(
            expected_diff_check, cwd=repo, capture_output=True
        )
        assert candidate_whitespace.returncode != 0, candidate_whitespace

        current_manifest = runner.normalise_manifest(
            {"plan": {"base_commit": target, "merge_base": common}}
        )
        assert current_manifest["plan"]["merge_base"] == common
        legacy_manifest = runner.normalise_manifest({"plan": {"base_commit": target}})
        assert legacy_manifest["plan"] == {"base_commit": target}


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

    _assert_diverged_base_contract(runner)
