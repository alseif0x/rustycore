# SPDX-License-Identifier: GPL-3.0-or-later
# Added 2026-10-20: exact reviewed sibling-library source identity.
"""Exact, relocatable sibling-library allowances; never a wildcard path source."""

from __future__ import annotations

import os
import pathlib
import re
from typing import Any

from architecture_common import ArchitectureError, REPO_ROOT


def path_identity(entry: dict[str, str]) -> str:
    return f"reviewed-path:{entry['path']}#{entry['to']}@{entry['version']}"


def validate_path_allowances(
    entries: Any, protected: set[str], workspace: set[str]
) -> dict[tuple[str, str, str], dict[str, str]]:
    if not isinstance(entries, list):
        raise ArchitectureError("external_dependencies.allowed_paths must be an array")
    result: dict[tuple[str, str, str], dict[str, str]] = {}
    fields = {"from", "to", "kind", "path", "version", "reason"}
    for entry in entries:
        if not isinstance(entry, dict) or set(entry) != fields:
            raise ArchitectureError("external path allowance has invalid fields")
        if not all(isinstance(value, str) and value.strip() for value in entry.values()):
            raise ArchitectureError("external path allowance needs non-empty strings")
        if entry["from"] not in protected or entry["to"] in workspace:
            raise ArchitectureError("external path allowance has invalid package scope")
        if not re.fullmatch(r"[A-Za-z0-9_-]+", entry["to"]):
            raise ArchitectureError("external path allowance has invalid package name")
        if entry["kind"] not in {"normal", "build"}:
            raise ArchitectureError("external path allowance has invalid kind")
        if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", entry["version"]):
            raise ArchitectureError("external path allowance needs an exact release version")
        # Exactly one sibling checkout, then ordinary canonical components.
        if not re.fullmatch(r"\.\./[A-Za-z0-9_-]+(?:/[A-Za-z0-9_-]+)+", entry["path"]):
            raise ArchitectureError("external path allowance needs a canonical sibling path")
        key = (entry["from"], entry["to"], entry["kind"])
        if key in result:
            raise ArchitectureError("duplicate external path allowance")
        result[key] = entry
    return result


def admitted_path_source(
    entry: dict[str, str], package: dict[str, Any], package_id: str,
    *, repo_root: pathlib.Path = REPO_ROOT,
) -> str:
    """Called only after Cargo name/version/source/PackageId consistency checks.

    Registry/git substitutes never gain a path allowance. The declared lexical
    directory, Cargo file URL, manifest and exact package version must agree;
    neither the checkout nor any library path component may traverse a symlink.
    """
    if package.get("source") is not None:
        return str(package.get("source"))
    expected = pathlib.Path(os.path.abspath(repo_root / entry["path"]))
    manifest = expected / "Cargo.toml"
    base, _, _ = package_id.rpartition("#")
    if (package.get("version") != entry["version"]
            or package.get("name") != entry["to"]
            or package.get("manifest_path") != str(manifest)
            or base != "path+" + expected.as_uri()):
        raise ArchitectureError("reviewed external path identity does not match Cargo metadata")
    try:
        for path in (repo_root, manifest):
            for component in (path, *path.parents):
                if component.is_symlink():
                    raise ArchitectureError("reviewed external path traverses a symlink")
        if not manifest.is_file() or expected.resolve(strict=True) != expected:
            raise ArchitectureError("reviewed external path manifest is unavailable or noncanonical")
    except OSError as exc:
        raise ArchitectureError("cannot validate reviewed external path") from exc
    return path_identity(entry)
