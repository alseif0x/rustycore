#!/usr/bin/env python3
"""Freeze a tested two-module tree; never silently rebase the extension challenge."""
from __future__ import annotations

import argparse
import hashlib
import gzip
import json
import os
from pathlib import Path
import subprocess
import sys
import tomllib
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parent
IGNORED_ROOT_DIRS = {"target", "__pycache__"}
RESERVED_DIRS = IGNORED_ROOT_DIRS | {".git"}
DECLARATIVE = {"Cargo.toml", "Cargo.lock", "driver/Cargo.toml", "driver/src/composition.rs"}
REQUIRED = {
    "contract/src/lib.rs", "contract/src/guest.rs", "host/src/lib.rs",
    "modules/encounter/src/lib.rs", "modules/policy/src/lib.rs",
    "c-guests/contract.h", "c-guests/encounter.c", "c-guests/policy.c",
    "driver/src/main.rs", "driver/src/composition.rs", "driver/src/harness.rs",
    "host/src/wasm/mod.rs", "protocol.json", "freeze.py", "build.py", "run.py", "report.py",
    "test_freeze.py", "test_report.py", "test_run.py",
}


def file_set(root: Path) -> dict[str, str]:
    """All files count, not just selected suffixes that can hide an included helper."""
    found = {}
    for directory, directories, files in os.walk(root, followlinks=False):
        directory = Path(directory)
        for name in list(directories):
            path = directory / name
            if path.is_symlink():
                raise ValueError(f"symlink in frozen source tree: {path.relative_to(root)}")
            if directory == root and name in IGNORED_ROOT_DIRS:
                directories.remove(name)
            elif name in RESERVED_DIRS:
                raise ValueError(f"nested ignored-directory source escape: {path.relative_to(root)}")
        for name in files:
            path = directory / name
            relative = path.relative_to(root)
            if path.is_symlink() or not path.is_file():
                raise ValueError(f"non-regular frozen source: {relative}")
            found[relative.as_posix()] = hashlib.sha256(path.read_bytes()).hexdigest()
    return dict(sorted(found.items()))


def read_record(path: Path):
    raw = path.read_bytes()
    return json.loads(gzip.decompress(raw) if raw.startswith(b"\x1f\x8b") else raw)


def output_preflight(path: Path):
    if (path.exists() or path.is_symlink() or path.resolve().is_relative_to(ROOT)
            or not path.parent.is_dir() or not os.access(path.parent, os.W_OK)):
        raise ValueError("output must be new, outside source, with an existing writable parent")


def validate_cargo(root: Path):
    """This experiment admits internal paths and contract-only modules, not external core replacements."""
    sources = file_set(root)
    workspace = tomllib.loads((root / "Cargo.toml").read_text()).get("workspace", {})
    modules = {str(path.parent.relative_to(root)) for path in (root / "modules").glob("*/Cargo.toml")}
    members = workspace.get("members")
    if (not isinstance(members, list) or len(set(members)) != len(members)
            or set(members) != {"contract", "host", "driver"} | modules):
        raise ValueError("workspace must contain exactly the core/driver and independent module crates")
    inherited = workspace.get("dependencies", {})

    def admitted_path(manifest, raw):
        if not isinstance(raw, str):
            raise ValueError("dependency path must be literal")
        path = (manifest.parent / raw).resolve()
        if (not path.is_relative_to(root.resolve())
                or any(part in RESERVED_DIRS for part in path.relative_to(root.resolve()).parts)):
            raise ValueError(f"external/ignored Cargo source path: {manifest}: {raw}")
        return path

    def visit(table, manifest, independent):
        if not isinstance(table, dict):
            return
        if "path" in table:
            admitted_path(manifest, table["path"])
        for key, value in table.items():
            if key in {"dependencies", "dev-dependencies", "build-dependencies"}:
                if not isinstance(value, dict):
                    raise ValueError("invalid Cargo dependency table")
                for name, dependency in value.items():
                    if independent and name != "conformance-contract":
                        raise ValueError(f"independent module may depend only on contract: {manifest}: {name}")
                    if independent:
                        dependency = inherited.get(name) if isinstance(dependency, dict) and dependency.get("workspace") else dependency
                        if (not isinstance(dependency, dict) or dependency.get("package", name) != "conformance-contract"
                                or "path" not in dependency):
                            raise ValueError("module contract dependency must use the frozen internal package")
                        owner = root / "Cargo.toml" if value[name].get("workspace") else manifest
                        if admitted_path(owner, dependency["path"]) != (root / "contract").resolve():
                            raise ValueError("module contract dependency points to a replacement")
            if isinstance(value, dict):
                visit(value, manifest, independent)
            elif isinstance(value, list):
                for item in value:
                    visit(item, manifest, independent)

    for name in sources:
        if Path(name).name != "Cargo.toml":
            continue
        manifest = root / name
        parsed = tomllib.loads(manifest.read_text())
        independent = name.startswith("modules/")
        if independent and ((manifest.parent / "build.rs").exists()
                            or parsed.get("package", {}).get("build") not in {None, False}):
            raise ValueError("independent challenge cannot replace core through a build script")
        visit(parsed, manifest, independent)
    return sources


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
    required_extension |= {f"c-guests/{name}.c", f"driver/tests/{name}.rs"}
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
    files = validate_cargo(ROOT)
    if args.command == "check":
        report = compare(read_record(args.freeze), files, args.module)
        print(json.dumps(report, indent=2))
        return 0 if report["source_freeze_pass"] else 1

    output_preflight(args.output)
    if REQUIRED - files.keys():
        raise ValueError(f"cannot freeze incomplete host: {sorted(REQUIRED - files.keys())}")
    validation_bytes = args.validation_report.read_bytes()
    validation = read_record(args.validation_report)
    # Import only for this CLI operation, avoiding a module-import cycle.
    from run import validate_prefreeze_report
    baseline_ids = validate_prefreeze_report(validation, files)
    sha = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    record = {
        "schema_version": 1, "kind": "two-module-source-freeze",
        "created_utc": datetime.now(timezone.utc).isoformat(), "git_head": sha,
        "validation_sha256": hashlib.sha256(validation_bytes).hexdigest(), "files": files,
        "baseline_module_ids": baseline_ids,
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
