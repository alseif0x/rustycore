"""Private physical-source inventory/ratchet used by check_architecture.py.

Independent of the logical Rust owner scanner. No policy-writing or standalone CLI.
"""
from __future__ import annotations

import datetime as dt
import hashlib
import json
import pathlib
import subprocess
from typing import Any


SOURCE_SUFFIXES = frozenset({
    ".rs", ".py", ".sh", ".bash", ".c", ".h", ".cc", ".cpp", ".cxx",
    ".hpp", ".hxx", ".js", ".jsx", ".mjs", ".ts", ".tsx", ".proto",
})
REVIEW_LIMIT = 1000
TERMINAL_LIMIT = 2000


class PhysicalFileError(ValueError):
    """A physical inventory or reviewed-policy violation."""


def checked_path(root: pathlib.Path, relative: str) -> pathlib.Path:
    if (not isinstance(relative, str) or not relative
            or pathlib.PurePosixPath(relative).is_absolute()
            or ".." in pathlib.PurePosixPath(relative).parts
            or pathlib.PurePosixPath(relative).as_posix() != relative):
        raise PhysicalFileError(f"invalid repository-relative source path: {relative!r}")
    path = root / relative
    if path.is_symlink() or path.resolve() != root.resolve() / relative:
        raise PhysicalFileError(f"source/provenance symlink is not accepted: {relative}")
    return path


def inventory(root: pathlib.Path) -> list[dict[str, Any]]:
    """Tracked + untracked nonignored sources, including tests/tools/build scripts.

    Git enumerates source inputs without traversing target/, private configurations,
    or temporary build products. Tracked files remain visible even if ignored later.
    """
    result = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        check=True, capture_output=True,
    )
    rows = []
    for relative in sorted(set(result.stdout.decode("utf-8").split("\0")) - {""}):
        path = root / relative
        # Documentation/assets may legitimately be symlinks. Never follow them
        # just to discover a shebang; source and extensionless links fail closed.
        if path.is_symlink() and path.suffix and path.suffix not in SOURCE_SUFFIXES:
            continue
        path = checked_path(root, relative)
        if not path.exists():
            # A deleted audited file is rejected by evaluate; ordinary deletions
            # are not fabricated zero-line source files.
            continue
        if not path.is_file():
            continue
        with path.open("rb") as source:
            prefix = source.read(256)
        if path.suffix not in SOURCE_SUFFIXES and not prefix.startswith(b"#!"):
            continue
        try:
            contents = path.read_text(encoding="utf-8")
        except (UnicodeError, OSError) as exc:
            raise PhysicalFileError(f"cannot count source {relative}: {type(exc).__name__}") from exc
        rows.append({"path": relative, "lines": len(contents.splitlines()), "kind": "handwritten"})
    return rows


def nonempty(value: Any, label: str) -> None:
    if not isinstance(value, str) or not value.strip():
        raise PhysicalFileError(f"{label} must be nonempty")


def positive(value: Any, label: str) -> None:
    if type(value) is not int or value <= 0:
        raise PhysicalFileError(f"{label} must be a positive integer")


def exact_keys(value: Any, keys: set[str], label: str) -> None:
    if not isinstance(value, dict) or set(value) != keys:
        raise PhysicalFileError(f"{label} requires exactly {sorted(keys)}")


def provenance(root: pathlib.Path, item: Any, label: str) -> None:
    exact_keys(item, {"path", "sha256"}, label)
    path = checked_path(root, item["path"])
    digest = item["sha256"]
    if not isinstance(digest, str) or len(digest) != 64:
        raise PhysicalFileError(f"{label} requires SHA-256")
    if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
        raise PhysicalFileError(f"{label} provenance hash drift: {item['path']}")


def evaluate(
    root: pathlib.Path, policy: Any, rows: list[dict[str, Any]], *,
    terminal: bool = False, today: dt.date | None = None,
) -> dict[str, Any]:
    exact_keys(policy, {
        "schema_version", "baseline_commit", "migration_issue", "completed_checkpoints",
        "legacy", "exceptions", "generated",
    }, "physical policy")
    if type(policy["schema_version"]) is not int or policy["schema_version"] != 1:
        raise PhysicalFileError("unsupported physical policy schema")
    nonempty(policy["baseline_commit"], "baseline_commit")
    positive(policy["migration_issue"], "migration_issue")
    for key in ("completed_checkpoints", "legacy", "exceptions", "generated"):
        if not isinstance(policy[key], list):
            raise PhysicalFileError(f"{key} must be a list")
    for checkpoint in policy["completed_checkpoints"]:
        nonempty(checkpoint, "completed checkpoint")
    today = today or dt.datetime.now(dt.timezone.utc).date()
    live = {row["path"]: dict(row) for row in rows}
    if len(live) != len(rows):
        raise PhysicalFileError("duplicate physical inventory path")
    for path, row in live.items():
        exact_keys(row, {"path", "lines", "kind"}, "physical inventory row")
        if row["kind"] != "handwritten":
            raise PhysicalFileError(f"generated attribution requires reviewed provenance: {path}")
        checked_path(root, path)
        if type(row["lines"]) is not int or row["lines"] < 0:
            raise PhysicalFileError(f"invalid physical line count: {path}")
    declared: set[str] = set()
    ceilings: dict[str, int] = {}
    violations = []

    def register(entry: dict[str, Any]) -> str:
        path = entry["path"]
        checked_path(root, path)
        if path in declared:
            raise PhysicalFileError(f"duplicate/overlapping physical policy path: {path}")
        declared.add(path)
        if path not in live:
            violations.append(f"audited physical path missing: {path}; explicitly retire/replace its row")
        return path

    def checkpoint(entry: dict[str, Any], path: str) -> None:
        nonempty(entry["review_checkpoint"], f"{path} review checkpoint")
        if entry["review_checkpoint"] in policy["completed_checkpoints"]:
            violations.append(f"expired physical review checkpoint: {path}")

    for entry in policy["legacy"]:
        exact_keys(entry, {"path", "observed_lines", "ceiling_lines", "split", "review_checkpoint"}, "legacy physical entry")
        path = register(entry)
        positive(entry["observed_lines"], f"{path} observed_lines")
        positive(entry["ceiling_lines"], f"{path} ceiling_lines")
        if entry["ceiling_lines"] > entry["observed_lines"]:
            raise PhysicalFileError(f"legacy ceiling exceeds reviewed observation: {path}")
        nonempty(entry["split"], f"{path} split")
        checkpoint(entry, path)
        ceilings[path] = entry["ceiling_lines"]
        if terminal and live.get(path, {}).get("lines", 0) > TERMINAL_LIMIT:
            violations.append(f"unfinished physical migration at terminal acceptance: {path}")

    for entry in policy["exceptions"]:
        exact_keys(entry, {"path", "observed_lines", "ceiling_lines", "responsibility", "reason",
                           "tracking_issue", "review_checkpoint", "reviewed_at", "expires_on"}, "physical exception")
        path = register(entry)
        for field in ("responsibility", "reason"):
            nonempty(entry[field], f"{path} {field}")
        for field in ("observed_lines", "ceiling_lines", "tracking_issue"):
            positive(entry[field], f"{path} {field}")
        checkpoint(entry, path)
        try:
            reviewed = dt.date.fromisoformat(entry["reviewed_at"])
            expires = dt.date.fromisoformat(entry["expires_on"])
        except (ValueError, TypeError) as exc:
            raise PhysicalFileError(f"invalid exception review dates: {path}") from exc
        if reviewed > today or reviewed >= expires or expires <= today:
            violations.append(f"expired/invalid physical exception dates: {path}")
        ceilings[path] = entry["ceiling_lines"]

    for entry in policy["generated"]:
        exact_keys(entry, {"path", "output_sha256", "generator", "inputs", "reproduction_command", "evidence"}, "generated source")
        path = register(entry)
        nonempty(entry["reproduction_command"], f"{path} reproduction_command")
        if not isinstance(entry["inputs"], list) or not entry["inputs"]:
            raise PhysicalFileError(f"generated source needs exact input provenance: {path}")
        for item in [entry["generator"], entry["evidence"], *entry["inputs"]]:
            provenance(root, item, f"{path} generator/input/reproduction evidence")
            if item["path"] == path:
                raise PhysicalFileError(f"generated source cannot certify itself: {path}")
        provenance(root, {"path": path, "sha256": entry["output_sha256"]}, "generated output")
        try:
            reproduction = json.loads((root / entry["evidence"]["path"]).read_text(encoding="utf-8"))
        except (ValueError, OSError) as exc:
            raise PhysicalFileError(f"generated reproduction evidence is not JSON: {path}") from exc
        expected = {
            "schema_version": 1, "command": entry["reproduction_command"],
            "generator": entry["generator"], "inputs": entry["inputs"],
            "output": {"path": path, "sha256": entry["output_sha256"]},
            "reproduced_sha256": entry["output_sha256"],
        }
        if reproduction != expected:
            raise PhysicalFileError(f"generated reproduction record does not match pinned provenance: {path}")
        if path in live:
            live[path]["kind"] = "generated"

    for path, row in live.items():
        if row["kind"] == "generated":
            continue
        ceiling = ceilings.get(path, TERMINAL_LIMIT)
        if row["lines"] > ceiling:
            violations.append(f"physical source {path} has {row['lines']} lines, ceiling {ceiling}")
    if violations:
        raise PhysicalFileError("physical source ratchet failed:\n- " + "\n- ".join(violations))
    return {
        "mode": "terminal" if terminal else "migration",
        "files": sorted(live.values(), key=lambda row: (-row["lines"], row["path"])),
        "review_required": sum(row["lines"] > REVIEW_LIMIT and row["kind"] == "handwritten" for row in live.values()),
        "legacy_files": len(policy["legacy"]),
        "generated_files": len(policy["generated"]),
    }
