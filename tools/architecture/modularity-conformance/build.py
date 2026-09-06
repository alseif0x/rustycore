#!/usr/bin/env python3
"""Build only laboratory sources, record exact artifacts; never install/restart a server."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tomllib

from freeze import ROOT, file_set, output_preflight, validate_cargo
import report

REPO = ROOT.parents[2]
ARTIFACTS = REPO / "target/modularity-conformance/artifacts"
LOCAL_TOOLCHAIN = REPO / "target/modularity-conformance/c-toolchain/root/usr"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(command, *, env=None):
    print("Building:", " ".join(str(part) for part in command), flush=True)
    subprocess.run([str(part) for part in command], cwd=REPO, env=env, check=True)


def expected_artifacts(root=ROOT):
    artifacts = {"driver": root / "target/release/conformance-driver"}
    c_directory = root.parents[2] / "target/modularity-conformance/artifacts"
    for manifest in sorted((root / "modules").glob("*/Cargo.toml")):
        name = manifest.parent.name
        package = tomllib.loads(manifest.read_text())["package"]["name"].replace("-", "_")
        artifacts[f"{name}:c-wasm"] = c_directory / f"{name}-c.wasm"
        artifacts[f"{name}:rust-wasm"] = root / "target/wasm32-unknown-unknown/release" / f"{package}.wasm"
    return artifacts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--clang", type=Path, default=LOCAL_TOOLCHAIN / "bin/clang-18")
    parser.add_argument("--wasm-ld", type=Path, default=LOCAL_TOOLCHAIN / "bin/wasm-ld-18")
    parser.add_argument("--library-path", type=Path, default=LOCAL_TOOLCHAIN / "lib/aarch64-linux-gnu")
    args = parser.parse_args()
    output_preflight(args.output)
    report.validate_protocol(json.loads((ROOT / "protocol.json").read_text()))
    before = validate_cargo(ROOT)
    source_modules = sorted((ROOT / "modules").glob("*/Cargo.toml"))
    if len(source_modules) < 2:
        raise ValueError("missing independent module sources")
    packages = []
    for manifest in source_modules:
        package = tomllib.loads(manifest.read_text())["package"]["name"]
        packages.extend(["-p", package])
    cargo_env = dict(os.environ, CARGO_BUILD_JOBS="2")
    # Explicit output path avoids silently measuring a binary from another workspace/cache.
    cargo_env["CARGO_TARGET_DIR"] = str(ROOT / "target")
    run(["cargo", "build", "--offline", "--locked", "--release", "--manifest-path",
         ROOT / "Cargo.toml", "-p", "conformance-driver", "--features", "wasm"], env=cargo_env)
    guest_env = dict(cargo_env, RUSTFLAGS="-C panic=abort")
    run(["cargo", "build", "--offline", "--locked", "--release", "--target", "wasm32-unknown-unknown",
         "--manifest-path", ROOT / "Cargo.toml", *packages], env=guest_env)
    artifacts = expected_artifacts(ROOT)
    c_env = dict(os.environ, LD_LIBRARY_PATH=str(args.library_path.resolve()))
    ARTIFACTS.mkdir(parents=True, exist_ok=True)
    for manifest in source_modules:
        name = manifest.parent.name
        source = ROOT / "c-guests" / f"{name}.c"
        if not source.is_file():
            raise ValueError(f"missing independent C producer for {name}")
        object_file = ARTIFACTS / f"{name}-c.o"
        artifact = ARTIFACTS / f"{name}-c.wasm"
        run([args.clang.absolute(), "--target=wasm32-unknown-unknown", "-std=c11", "-O2",
             "-Wall", "-Wextra", "-Werror", "-nostdlib", "-nostdinc", "-fno-builtin",
             "-c", "-o", object_file, source], env=c_env)
        # Preserve argv[0]: resolving wasm-ld's symlink to generic lld loses its driver flavor.
        run([args.wasm_ld.absolute(), "--no-entry", "--export-memory", "--max-memory=3145728",
             "-o", artifact, object_file], env=c_env)
    after = file_set(ROOT)
    if before != after:
        raise ValueError("source changed while building; do not certify a mixed tree")
    record = {
        "schema_version": 2, "kind": "source-built-conformance-artifacts", "source_files": after,
        "artifacts": {name: {"path": str(path), "sha256": sha(path)} for name, path in artifacts.items()},
        "toolchains": {
            "rustc": subprocess.check_output(["rustc", "-Vv"], text=True),
            "clang": subprocess.check_output([str(args.clang.absolute()), "--version"], env=c_env, text=True),
            "wasm_ld": subprocess.check_output([str(args.wasm_ld.absolute()), "--version"], env=c_env, text=True),
            "clang_sha256": sha(args.clang.resolve()), "wasm_ld_sha256": sha(args.wasm_ld.resolve()),
        },
        "build_environment": {name: os.environ.get(name) for name in
                              ["RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER",
                               "RUSTC_WORKSPACE_WRAPPER", "RUSTUP_TOOLCHAIN"]},
        "boundary": "exact laboratory files and loader paths; toolchain/dependency environment recorded, not a hermetic operating system",
    }
    with args.output.open("x") as stream:
        json.dump(record, stream, indent=2, sort_keys=True)
        stream.write("\n")
    print(f"Build record: {args.output}", flush=True)


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as exc:
        print(f"build failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
