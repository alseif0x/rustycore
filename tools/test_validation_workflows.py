#!/usr/bin/env python3
"""Hermetic contract tests for the Validation V2 GitHub workflows."""

from __future__ import annotations

import os
from pathlib import Path
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[1]
RUST_CI = ROOT / ".github" / "workflows" / "rust-ci.yml"
DETERMINISM = ROOT / ".github" / "workflows" / "validation-determinism.yml"


def step_block(workflow: str, name: str) -> str:
    """Return one named Actions step without requiring a YAML dependency."""
    lines = workflow.splitlines()
    marker = f"      - name: {name}"
    try:
        start = next(index for index, line in enumerate(lines) if line == marker)
    except StopIteration as error:
        raise AssertionError(f"workflow step not found: {name}") from error
    end = len(lines)
    for index in range(start + 1, len(lines)):
        if lines[index].startswith("      - "):
            end = index
            break
    return "\n".join(lines[start:end]) + "\n"


def run_blocks(workflow: str) -> list[str]:
    """Extract shell bodies from run keys for interpolation checks."""
    lines = workflow.splitlines()
    blocks: list[str] = []
    for index, line in enumerate(lines):
        stripped = line.lstrip()
        if not stripped.startswith("run:"):
            continue
        value = stripped[len("run:"):].strip()
        indent = len(line) - len(stripped)
        if value == "|":
            body: list[str] = []
            for candidate in lines[index + 1:]:
                if candidate.strip() and len(candidate) - len(candidate.lstrip()) <= indent:
                    break
                body.append(candidate[indent + 2:] if candidate else "")
            blocks.append("\n".join(body) + "\n")
        else:
            blocks.append(value + "\n")
    return blocks


def git(repo: Path, *arguments: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *arguments],
        cwd=repo,
        check=check,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def git_output(repo: Path, *arguments: str) -> str:
    return git(repo, *arguments).stdout.strip()


def commit(repo: Path, message: str, filename: str, contents: str) -> str:
    (repo / filename).write_text(contents, encoding="utf-8")
    git(repo, "add", filename)
    git(repo, "commit", "-qm", message)
    return git_output(repo, "rev-parse", "HEAD")


def invoke_base_check(script: str, repo: Path, base: str, env_file: Path) -> subprocess.CompletedProcess[str]:
    environment = os.environ.copy()
    environment["MANUAL_BASE"] = base
    environment["GITHUB_ENV"] = str(env_file)
    return subprocess.run(
        ["bash", "-c", script],
        cwd=repo,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def test_manual_base_validation() -> None:
    workflow = RUST_CI.read_text(encoding="utf-8")
    script = next(
        block for block in run_blocks(step_block(workflow, "Validate manual final base"))
        if "git merge-base --is-ancestor" in block
    )
    assert "${{" not in script
    with tempfile.TemporaryDirectory(prefix="validation-workflow-base-") as raw_directory:
        directory = Path(raw_directory)
        repo = directory / "repo"
        repo.mkdir()
        git(repo, "init", "-q")
        git(repo, "config", "user.email", "validation-workflows@example.invalid")
        git(repo, "config", "user.name", "Validation workflows")
        first = commit(repo, "first", "state.txt", "first\n")
        head = commit(repo, "second", "state.txt", "second\n")

        valid_env = directory / "valid.env"
        valid = invoke_base_check(script, repo, first, valid_env)
        assert valid.returncode == 0, valid.stderr
        assert valid_env.read_text(encoding="utf-8") == f"VALIDATED_MANUAL_BASE_SHA={first}\n"

        same_env = directory / "same.env"
        same = invoke_base_check(script, repo, head, same_env)
        assert same.returncode != 0
        assert "must differ from HEAD" in same.stdout
        assert not same_env.exists()

        missing_env = directory / "missing.env"
        missing = invoke_base_check(script, repo, "", missing_env)
        assert missing.returncode != 0
        assert "requires an explicit base input" in missing.stdout
        assert not missing_env.exists()

        invalid_env = directory / "invalid.env"
        invalid = invoke_base_check(script, repo, "not-a-revision", invalid_env)
        assert invalid.returncode != 0
        assert "must resolve to a commit" in invalid.stdout
        assert not invalid_env.exists()

        git(repo, "checkout", "-qb", "side", first)
        side = commit(repo, "side", "side.txt", "side\n")
        git(repo, "checkout", "-q", "-")
        assert git_output(repo, "rev-parse", "HEAD") == head
        non_ancestor_env = directory / "non-ancestor.env"
        non_ancestor = invoke_base_check(script, repo, side, non_ancestor_env)
        assert non_ancestor.returncode != 0
        assert "must be an ancestor of HEAD" in non_ancestor.stdout
        assert not non_ancestor_env.exists()

        marker = directory / "command-substitution-ran"
        injection_env = directory / "injection.env"
        injection = invoke_base_check(
            script,
            repo,
            f"$(touch {marker})",
            injection_env,
        )
        assert injection.returncode != 0
        assert not marker.exists()
        assert not injection_env.exists()

        # Distinct commits can still have identical trees (e.g. a reverted change).
        identical = git_output(repo, "commit-tree", f"{head}^{{tree}}", "-p", head, "-m", "same tree")
        git(repo, "checkout", "-q", "--detach", identical)
        empty_env = directory / "empty.env"
        empty = invoke_base_check(script, repo, head, empty_env)
        assert empty.returncode != 0
        assert "no changed paths" in empty.stdout
        assert not empty_env.exists()


def test_rust_ci_contract() -> None:
    workflow = RUST_CI.read_text(encoding="utf-8")
    assert "login != 'alseif0x'" in workflow
    assert "uses: actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5" in workflow
    assert "uses: actions-rust-lang/setup-rust-toolchain@166cdcfd11aee3cb47222f9ddb555ce30ddb9659" in workflow
    assert "uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02" in workflow
    assert "base:\n        description: Explicit ancestor base required for a manual final run" in workflow
    assert "required: false\n        type: string" in workflow

    checkout = workflow.index("- name: Check out exact validation SHA")
    manual_base = workflow.index("- name: Validate manual final base")
    toolchain = workflow.index("- name: Install pinned Rust")
    fetch = workflow.index("- name: Prepare locked offline inputs")
    assert checkout < manual_base < toolchain < fetch

    validation_step = step_block(workflow, "Run canonical Validation V2 profile")
    validation_run = next(iter(run_blocks(validation_step)))
    assert "${{" not in validation_run
    assert "BASE_SHA: ${{ github.event.pull_request.base.sha || github.event.before || github.sha }}" in workflow
    assert "BASE_SHA=\"${VALIDATED_MANUAL_BASE_SHA:?manual final base was not validated}\"" in validation_run
    assert "args=(\"$PROFILE\" --base \"$BASE_SHA\" --timings --logs)" in validation_run
    assert 'if [ "$PROFILE" = "final" ]; then' in validation_step
    assert "args+=(--require-changes)" in validation_run

    manual_step = step_block(workflow, "Validate manual final base")
    manual_run = next(iter(run_blocks(manual_step)))
    assert "${{" not in manual_run
    assert "--end-of-options" in manual_run
    assert "git merge-base --is-ancestor \"$base_commit\" \"$head_commit\"" in manual_run

    verifier = step_block(workflow, "Verify Validation V2 manifest verdict")
    upload = step_block(workflow, "Upload Validation V2 manifest")
    assert "if: always()" in verifier
    assert "if: always()" in upload
    assert "validation-v2-manifest.json" in upload
    assert "validation-v2-manifest-logs/" in upload
    assert "target/cargo-timings/*.html" in upload
    assert "target/**" not in upload
    assert "if-no-files-found: error" in upload

    for block in run_blocks(workflow):
        assert "${{" not in block


def test_determinism_contract() -> None:
    workflow = DETERMINISM.read_text(encoding="utf-8")
    assert "workflow_dispatch:\n" in workflow
    assert "inputs:" not in workflow
    assert "inputs.profile" not in workflow
    assert "./tools/validation-v2 self-test" in workflow
    assert "--base" not in workflow
    assert "--timings" not in workflow
    assert "cargo fetch" not in workflow
    assert "quick" not in workflow
    assert "final" not in workflow
    assert workflow.count("          - ") >= 20
    assert "uses: actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5" in workflow
    assert "uses: actions-rust-lang/setup-rust-toolchain@166cdcfd11aee3cb47222f9ddb555ce30ddb9659" in workflow


def main() -> None:
    test_manual_base_validation()
    test_rust_ci_contract()
    test_determinism_contract()
    print("validation workflow tests passed")


if __name__ == "__main__":
    main()
