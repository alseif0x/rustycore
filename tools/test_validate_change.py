"""Focused contracts for the experimental scoped editing helper."""

from __future__ import annotations

import importlib.machinery
import importlib.util
from pathlib import Path
import tempfile
from unittest.mock import patch


SCRIPT = Path(__file__).with_name("validate-change").resolve()
loader = importlib.machinery.SourceFileLoader("validate_change", str(SCRIPT))
spec = importlib.util.spec_from_loader(loader.name, loader)
runner = importlib.util.module_from_spec(spec)
loader.exec_module(runner)


def synthetic_metadata(repo: Path) -> dict[str, object]:
    packages = []
    for package_id, name, directory, dependencies in (
        ("a-id", "package-a", "a-member", []),
        ("b-id", "package-b", "b-member", ["../a-member"]),
        ("c-id", "package-c", "c-member", ["../b-member"]),
    ):
        root = repo / "crates" / directory
        (root / "src").mkdir(parents=True, exist_ok=True)
        (root / "Cargo.toml").write_text("[package]\n", encoding="utf-8")
        package_dependencies = [
            {"name": "dependency", "path": path}
            for path in dependencies
        ]
        packages.append(
            {
                "id": package_id,
                "name": name,
                "manifest_path": str(root / "Cargo.toml"),
                "dependencies": package_dependencies,
            }
        )
    return {"workspace_members": ["a-id", "b-id", "c-id"], "packages": packages}


def test_workspace_scope_tracks_reverse_path_consumers_and_package_names() -> None:
    with tempfile.TemporaryDirectory(prefix="validate-change-scope-") as raw_directory:
        repo = Path(raw_directory).resolve()
        metadata = synthetic_metadata(repo)
        direct, consumers, removed, manifests = runner.workspace_scope(
            repo, ["crates/a-member/src/new.rs"], metadata
        )

        assert direct == ["package-a"]
        assert consumers == ["package-a", "package-b", "package-c"]
        assert removed is False
        assert manifests["package-a"] == "crates/a-member/Cargo.toml"


def test_plan_keeps_independent_package_and_feature_scopes() -> None:
    with tempfile.TemporaryDirectory(prefix="validate-change-plan-") as raw_directory:
        repo = Path(raw_directory).resolve()
        metadata = synthetic_metadata(repo)
        (repo / "crates/a-member/src/lib.rs").write_text("pub fn edit() {}\n", encoding="utf-8")
        (repo / "crates/a-member/src/new.rs").write_text("pub fn next() {}\n", encoding="utf-8")
        (repo / "README.md").write_text("[guide](docs/guide.md)\n", encoding="utf-8")
        (repo / "Cargo.lock").write_text("# fixture\n", encoding="utf-8")
        (repo / "docs").mkdir()
        (repo / "docs/guide.md").write_text("guide\n", encoding="utf-8")

        with patch.object(runner, "cargo_metadata", return_value=metadata):
            source_steps, source_needs = runner.plan(
                repo, ["crates/a-member/src/lib.rs"], "base"
            )
            manifest_steps, manifest_needs = runner.plan(
                repo, ["crates/a-member/Cargo.toml"], "base"
            )
            focused_steps, focused_needs = runner.plan(
                repo,
                ["crates/a-member/src/new.rs"],
                "base",
                focus="my_case",
                features=["test-fixtures"],
                no_default_features=True,
            )

        source_commands = [command for _, command in source_steps]
        assert ["cargo", "fmt", "--manifest-path", "crates/a-member/Cargo.toml", "--check"] in source_commands
        assert ["cargo", "test", "--locked", "--all-targets", "--jobs", "1", "-p", "package-a"] in source_commands
        assert ["cargo", "check", "--locked", "--all-targets", "--jobs", "1", "-p", "package-b"] in source_commands
        assert ["cargo", "check", "--locked", "--all-targets", "--jobs", "1", "-p", "package-c"] in source_commands
        assert source_needs == []

        manifest_commands = [command for _, command in manifest_steps]
        assert ["cargo", "check", "--locked", "--all-targets", "--jobs", "1", "-p", "package-a"] in manifest_commands
        assert ["cargo", "check", "--locked", "--all-targets", "--jobs", "1", "-p", "package-b"] in manifest_commands
        assert ["cargo", "check", "--locked", "--all-targets", "--jobs", "1", "-p", "package-c"] in manifest_commands
        assert "crates/a-member/Cargo.toml" in manifest_needs

        focused_commands = [command for _, command in focused_steps]
        assert any(
            command[:8] == [
                "cargo", "test", "--locked", "--jobs", "1", "-p", "package-a", "--lib",
            ]
            and "--features" in command
            and "test-fixtures" in command
            and "--no-default-features" in command
            and command[-2:] == ["--", "my_case"]
            for command in focused_commands
        )
        assert focused_needs == []


def test_docs_scan_current_tree_and_root_changes_stay_broad() -> None:
    with tempfile.TemporaryDirectory(prefix="validate-change-docs-") as raw_directory:
        repo = Path(raw_directory).resolve()
        (repo / "README.md").write_text("[guide](docs/guide.md)\n", encoding="utf-8")
        with patch.object(runner, "markdown_files", return_value=["README.md"]):
            docs_steps, docs_needs = runner.plan(repo, ["README.md"], "base")
        docs_commands = [command for _, command in docs_steps]
        assert [runner.sys.executable, runner.DOC_LINK_CHECKER, "README.md"] in docs_commands
        assert docs_needs == []

        with patch.object(runner, "markdown_files", return_value=["README.md"]):
            root_steps, root_needs = runner.plan(repo, ["Cargo.lock"], "base")
        root_commands = [command for _, command in root_steps]
        assert [
            "cargo", "check", "--locked", "--workspace", "--all-targets", "--jobs", "1",
        ] in root_commands
        assert "Cargo.lock" in root_needs


def main() -> None:
    test_workspace_scope_tracks_reverse_path_consumers_and_package_names()
    test_plan_keeps_independent_package_and_feature_scopes()
    test_docs_scan_current_tree_and_root_changes_stay_broad()
    print("validate-change tests passed")


if __name__ == "__main__":
    main()
