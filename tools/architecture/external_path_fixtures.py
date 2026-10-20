# SPDX-License-Identifier: GPL-3.0-or-later
# Added 2026-10-20: synthetic reviewed-path identity and substitution cases.
"""Synthetic source-identity cases for reviewed sibling path dependencies."""

from __future__ import annotations

import pathlib
import tempfile

from architecture_common import ArchitectureError
from external_path_policy import (
    admitted_path_source, path_identity, validate_path_allowances,
)


def run_external_path_self_tests() -> None:
    # Imported during self-test execution, after the checker modules are loaded.
    from check_dependencies import check_dependencies
    from check_policy import validate_policy
    from check_fixtures import synthetic_metadata

    entry = {
        "from": "application", "to": "library", "kind": "normal",
        "path": "../sibling/crates/library", "version": "0.1.0",
        "reason": "Synthetic reviewed adapter",
    }
    def reject(operation, label):
        try:
            operation()
        except ArchitectureError:
            return
        raise ArchitectureError(f"external path fixture accepted {label}")

    def validate(value):
        return validate_path_allowances(value, {"application"}, {"application"})

    if len(validate([entry])) != 1:
        raise ArchitectureError("external path fixture lost its allowance")
    reject(lambda: validate([entry, entry]), "duplicate allowance")
    for field, value in (
        ("from", "unprotected"), ("to", "application"), ("kind", "dev"),
        ("version", "^0.1"), ("path", "/absolute/path"),
        ("path", "../sibling/../substitute"), ("path", "../sibling//library"),
        ("path", "../sibling/library/"), ("reason", ""),
    ):
        changed = {**entry, field: value}
        reject(lambda: validate([changed]), f"invalid {field}={value!r}")
    reject(lambda: validate([{**entry, "unexpected": True}]), "unknown field")

    with tempfile.TemporaryDirectory(prefix="architecture-path-") as temporary:
        root = pathlib.Path(temporary).resolve() / "core"
        root.mkdir()
        library = root.parent / "sibling/crates/library"
        library.mkdir(parents=True)
        manifest = library / "Cargo.toml"
        manifest.write_text('[package]\nname="library"\nversion="0.1.0"\n')
        package_id = "path+" + library.as_uri() + "#0.1.0"
        package = {
            "name": "library", "version": "0.1.0", "source": None,
            "manifest_path": str(manifest),
        }
        def admit(value=package, identity=package_id):
            return admitted_path_source(entry, value, identity, repo_root=root)

        if admit() != path_identity(entry):
            raise ArchitectureError("external path fixture lost canonical identity")
        # Exercise classification and the unused-allowance ratchet without
        # invoking Cargo or weakening the checker's actual repository root.
        fixture_policy = validate_policy({
            "schema_version": 2,
            "categories": {"application": ["application"]},
            "allowed_category_dependencies": {"application": []},
            "restricted_packages": {}, "exceptions": [],
            "external_dependencies": {
                "canonical_registry_source": "registry+https://github.com/rust-lang/crates.io-index",
                "protected_categories": ["application"], "protected_packages": [],
                "allowed": {"application": {"normal": [], "build": []}},
                "allowed_paths": [entry], "exceptions": [],
            },
        })
        graph = synthetic_metadata({"application"}, {("application", "library", "normal")})
        graph["packages"][1].update(package, id=package_id)
        graph["resolve"]["nodes"][0]["deps"][0]["pkg"] = package_id
        # check_dependencies deliberately has no filesystem-root override, so
        # test the source-sensitive production graph through its portable
        # identity directly and keep manifest validation covered above.
        from check_dependencies import production_edges, classify_external_dependency
        _, _, edges = production_edges(graph)
        if ("application", "library", package_id, "normal") not in edges:
            raise ArchitectureError("external path fixture lost Cargo package identity")
        if classify_external_dependency(fixture_policy, "application", "library",
                                        admit(), "normal")[0] != "allowed":
            raise ArchitectureError("external path fixture rejected reviewed identity")
        if classify_external_dependency(fixture_policy, "application", "library",
                                        package_id, "normal")[0] != "forbidden":
            raise ArchitectureError("external path fixture allowed unchecked identity")
        empty_graph = synthetic_metadata({"application"}, set())
        reject(lambda: check_dependencies(fixture_policy, empty_graph), "unused allowance")
        for field, value in (
            ("version", "0.1.1"), ("name", "substitute"),
            ("manifest_path", str(root / "Cargo.toml")),
            ("manifest_path", str(library / "sub/../Cargo.toml")),
        ):
            altered = {**package, field: value}
            reject(lambda: admit(altered), f"substituted {field}")
        reject(lambda: admit(identity="path+file:///substitute#library@0.1.0"),
               "substituted package URL")
        # Registry and Git identities remain distinct from the allowed path.
        for source in ("registry+https://example.test/index", "git+https://example.test/lib#abc"):
            if admit({**package, "source": source}) == path_identity(entry):
                raise ArchitectureError("external path fixture admitted sourced substitute")
        original = library / "original.toml"
        manifest.rename(original)
        manifest.symlink_to(original)
        reject(admit, "symlinked manifest")
        manifest.unlink()
        original.rename(manifest)
        original_dir = library.with_name("original-library")
        library.rename(original_dir)
        library.symlink_to(original_dir, target_is_directory=True)
        reject(admit, "symlinked directory")
        library.unlink()
        original_dir.rename(library)
        manifest.unlink()
        reject(admit, "missing manifest")
