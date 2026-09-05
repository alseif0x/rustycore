#!/usr/bin/env python3
"""Freeze a tested two-module tree; never silently rebase the extension challenge."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parent
IGNORED_DIRS = {"target", "__pycache__", ".git"}
DECLARATIVE = {"Cargo.toml", "Cargo.lock", "driver/Cargo.toml", "driver/src/composition.rs"}
REQUIRED = {
    "contract/src/lib.rs", "contract/src/guest.rs", "host/src/lib.rs",
    "modules/encounter/src/lib.rs", "modules/policy/src/lib.rs",
    "c-guests/contract.h", "c-guests/encounter.c", "c-guests/policy.c",
    "driver/src/main.rs", "driver/src/composition.rs", "protocol.json", "freeze.py",
}


def file_set(root: Path) -> dict[str, str]:
    """All files count, not just selected suffixes that can hide an included helper."""
    found = {}
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root)
        if any(part in IGNORED_DIRS for part in relative.parts):
            continue
        if path.is_symlink():
            raise ValueError(f"symlink in frozen source tree: {relative}")
        if path.is_file():
            found[relative.as_posix()] = hashlib.sha256(path.read_bytes()).hexdigest()
    return found


def new_module_name(name: str) -> None:
    if (not name or name in {"encounter", "policy"}
            or not all(c in "abcdefghijklmnopqrstuvwxyz0123456789_" for c in name)):
        raise ValueError("challenge requires a new lowercase module name")


def extension_path(path: str, name: str) -> bool:
    return (path.startswith(f"modules/{name}/")
            or path == f"c-guests/{name}.c"
            or path == f"driver/tests/{name}.rs")


def compare(frozen: dict, current: dict[str, str], name: str) -> dict:
    new_module_name(name)
    if frozen.get("schema_version") != 1 or frozen.get("kind") != "two-module-source-freeze":
        raise ValueError("invalid freeze schema")
    before = frozen.get("files")
    if not isinstance(before, dict) or REQUIRED - before.keys():
        raise ValueError("incomplete freeze; required host/adapter/driver sources absent")
    if any(extension_path(path, name) for path in before):
        raise ValueError("third module existed before the challenge freeze")
    removed = sorted(before.keys() - current.keys())
    changed = sorted(path for path in before.keys() & current.keys()
                     if before[path] != current[path])
    added = sorted(current.keys() - before.keys())
    forbidden = sorted(set(removed) | (set(changed) - DECLARATIVE)
                       | {path for path in added if not extension_path(path, name)})
    required_extension = {f"modules/{name}/Cargo.toml", f"modules/{name}/src/lib.rs"}
    missing = sorted(required_extension - current.keys())
    return {
        "schema_version": 1,
        "source_freeze_pass": not forbidden and not missing,
        "frozen_file_count": len(before),
        "unchanged_file_count": len(before) - len(changed) - len(removed),
        "changed_declarative_paths": sorted(set(changed) & DECLARATIVE),
        "added_extension_paths": added,
        "forbidden_paths": forbidden,
        "missing_extension_paths": missing,
        "manual_review_required": "Review declarative deltas and independent module semantics; hashes alone are not conformance.",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    create = sub.add_parser("create")
    create.add_argument("--output", type=Path, required=True)
    create.add_argument("--validation-report", type=Path, required=True)
    check = sub.add_parser("check")
    check.add_argument("--freeze", type=Path, required=True)
    check.add_argument("--module", required=True)
    args = parser.parse_args()
    files = file_set(ROOT)
    if args.command == "check":
        report = compare(json.loads(args.freeze.read_text()), files, args.module)
        print(json.dumps(report, indent=2))
        return 0 if report["source_freeze_pass"] else 1

    if args.output.resolve().is_relative_to(ROOT):
        raise ValueError("freeze output must be outside its hashed source tree")
    if REQUIRED - files.keys():
        raise ValueError(f"cannot freeze incomplete host: {sorted(REQUIRED - files.keys())}")
    validation_bytes = args.validation_report.read_bytes()
    validation = json.loads(validation_bytes)
    # The executable campaign produces this pre-freeze report after both real guests run.
    if (validation.get("kind") != "two-module-conformance"
            or validation.get("passed") is not True
            or validation.get("source_files") != files):
        raise ValueError("need passing two-module validation for this exact tree")
    sha = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    record = {
        "schema_version": 1, "kind": "two-module-source-freeze",
        "created_utc": datetime.now(timezone.utc).isoformat(), "git_head": sha,
        "validation_sha256": hashlib.sha256(validation_bytes).hexdigest(), "files": files,
        "allowed_declarative_changes": sorted(DECLARATIVE),
    }
    with args.output.open("x") as stream:
        json.dump(record, stream, indent=2, sort_keys=True)
        stream.write("\n")
    print(json.dumps({"frozen_files": len(files), "output": str(args.output)}))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, subprocess.CalledProcessError) as exc:
        print(f"freeze rejected: {exc}", file=sys.stderr)
        raise SystemExit(2)
