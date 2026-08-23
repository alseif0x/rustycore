#!/usr/bin/env python3
"""Compose trusted linked modules into the world server (issue #229).

`sync` reads every `modules/*/module.toml`, validates it, writes
`modules.lock.toml`, and regenerates the `world-modules` compositor crate.
`check` verifies the checked-in lock and generated crate still match the
checkouts without writing anything.

The build never runs this: generation is an explicit operator step, so cargo
never fetches, discovers or rewrites the source tree behind your back.
"""
from __future__ import annotations

import argparse
import hashlib
import pathlib
import re
import sys
import tomllib

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
MODULES_DIR = REPO_ROOT / "modules"
LOCK_PATH = REPO_ROOT / "modules.lock.toml"
COMPOSITOR = REPO_ROOT / "crates" / "world-modules"
SUPPORTED_SOURCE_API = "1"
# Source APIs this server still accepts. A module asking for anything else
# fails at composition, long before a player logs in.
COMPATIBLE_SOURCE_APIS = {"1"}
CONFIG_OVERRIDE_DIR = REPO_ROOT / "conf" / "modules"

ID_RE = re.compile(r"^[a-z][a-z0-9_.]{0,63}$")
PACKAGE_RE = re.compile(r"^[a-z][a-z0-9_-]{0,63}$")
PATH_RE = re.compile(r"^[A-Za-z0-9_./-]+$")
REGISTRAR_RE = re.compile(r"^[a-z][a-z0-9_]*(::[a-z][a-z0-9_]*)+$")
VERSION_RE = re.compile(r"^\d+\.\d+\.\d+$")


class ComposeError(Exception):
    pass


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ComposeError(message)


def _digest(path: pathlib.Path) -> str:
    """Content digest of a checkout: every tracked-looking file, sorted."""
    h = hashlib.sha256()
    for f in sorted(p for p in path.rglob("*") if p.is_file()):
        if "target" in f.relative_to(path).parts or ".git" in f.relative_to(path).parts:
            continue
        h.update(str(f.relative_to(path)).encode())
        h.update(f.read_bytes())
    return h.hexdigest()


def read_modules() -> list[dict]:
    if not MODULES_DIR.is_dir():
        return []
    found = []
    for entry in sorted(MODULES_DIR.iterdir()):
        manifest = entry / "module.toml"
        if not entry.is_dir() or not manifest.is_file():
            continue
        found.append(parse_manifest(entry, manifest))
    reject_collisions(found)
    return found


def parse_manifest(root: pathlib.Path, manifest: pathlib.Path) -> dict:
    raw = tomllib.loads(manifest.read_text(encoding="utf-8"))
    where = manifest.relative_to(REPO_ROOT)
    for section in ("module", "build", "compatibility"):
        _require(section in raw, f"{where}: missing [{section}] section")

    module, build, compat = raw["module"], raw["build"], raw["compatibility"]
    identifier = module.get("id", "")
    _require(bool(ID_RE.match(identifier)), f"{where}: invalid module id {identifier!r}")
    version = module.get("version", "")
    _require(bool(VERSION_RE.match(version)), f"{where}: invalid version {version!r}")
    display = module.get("display_name", "")
    _require(bool(display.strip()) and len(display) <= 128, f"{where}: invalid display_name")

    package = build.get("package", "")
    _require(bool(PACKAGE_RE.match(package)), f"{where}: invalid package {package!r}")
    crate_path = build.get("crate_path", ".")
    _require(bool(PATH_RE.match(crate_path)), f"{where}: invalid crate_path {crate_path!r}")
    resolved = (root / crate_path).resolve()
    _require(
        resolved.is_relative_to(root.resolve()),
        f"{where}: crate_path escapes the module checkout",
    )
    _require((resolved / "Cargo.toml").is_file(), f"{where}: {crate_path} has no Cargo.toml")
    registrar = build.get("registrar", "")
    _require(bool(REGISTRAR_RE.match(registrar)), f"{where}: invalid registrar {registrar!r}")

    source_api = str(compat.get("source_api", ""))
    _require(
        source_api in COMPATIBLE_SOURCE_APIS,
        f"{where}: source_api {source_api!r} is not supported; this server provides "
        f"{sorted(COMPATIBLE_SOURCE_APIS)}. Update the module or pin an older server.",
    )

    config = load_config(identifier, raw.get("config", {}), where)

    return {
        "id": identifier,
        "version": version,
        "display_name": display,
        "package": package,
        "source_path": str(resolved.relative_to(REPO_ROOT)),
        "registrar": registrar,
        "source_api": source_api,
        "requested_ref": module.get("ref", ""),
        "resolved_commit": module.get("commit", ""),
        "digest": _digest(root),
        "config": config,
        "config_digest": config_digest(config),
        "order": module.get("order", 0),
    }


CONFIG_KEY_RE = re.compile(r"^[a-z][a-z0-9_]{0,63}$")


def load_config(module_id: str, defaults: dict, where) -> dict:
    """Package defaults, then operator overrides from outside the repository.

    Overrides live in `conf/modules/<id>.toml`, never inside the module
    checkout, so updating a module never clobbers operator settings and a
    module repository never carries a secret.
    """
    merged: dict[str, object] = {}
    for source, values in (("default", defaults), ("override", _override_for(module_id))):
        for key, value in values.items():
            _require(
                bool(CONFIG_KEY_RE.match(key)),
                f"{where}: invalid {source} configuration key {key!r}",
            )
            _require(
                isinstance(value, (bool, int, str)) and not isinstance(value, float),
                f"{where}: configuration key {key!r} must be a boolean, integer or string",
            )
            merged[key] = value
    return dict(sorted(merged.items()))


def _override_for(module_id: str) -> dict:
    path = CONFIG_OVERRIDE_DIR / f"{module_id}.toml"
    if not path.is_file():
        return {}
    return tomllib.loads(path.read_text(encoding="utf-8"))


def config_digest(config: dict) -> str:
    """Mirror `ModuleConfig::digest` exactly so both sides agree."""
    state = 0xCBF29CE484222325
    for key, value in sorted(config.items()):
        if isinstance(value, bool):
            rendered = f"b:{'true' if value else 'false'}"
        elif isinstance(value, int):
            rendered = f"i:{value}"
        else:
            rendered = f"s:{len(value)}:{value}"
        for byte in f"{key}={rendered};".encode():
            state ^= byte
            state = (state * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"fnv1a64:{state:016x}"


def reject_collisions(modules: list[dict]) -> None:
    for key, label in (("id", "module id"), ("package", "Cargo package")):
        seen: dict[str, str] = {}
        for m in modules:
            if m[key] in seen:
                raise ComposeError(
                    f"duplicate {label} {m[key]!r} in {m['source_path']} and {seen[m[key]]}"
                )
            seen[m[key]] = m["source_path"]


def ordered(modules: list[dict]) -> list[dict]:
    """Operator order first, then module id, so composition is reproducible."""
    return sorted(modules, key=lambda m: (m["order"], m["id"]))


def render_lock(modules: list[dict]) -> str:
    lines = [
        "# Generated by tools/modules/compose.py. Do not edit by hand.",
        "#",
        "# Records exactly what was composed into the server: identity, source,",
        "# requested ref, resolved commit, crate, source API, order and digest.",
        "# It holds no credentials and no URLs with secrets.",
        f'source_api = "{SUPPORTED_SOURCE_API}"',
        "",
    ]
    for index, m in enumerate(ordered(modules)):
        lines += [
            "[[module]]",
            f'id = "{m["id"]}"',
            f'version = "{m["version"]}"',
            f'package = "{m["package"]}"',
            f'source_path = "{m["source_path"]}"',
            f'requested_ref = "{m["requested_ref"]}"',
            f'resolved_commit = "{m["resolved_commit"]}"',
            f'registrar = "{m["registrar"]}"',
            f'source_api = "{m["source_api"]}"',
            f"enabled_order = {index}",
            f'config_digest = "{m["config_digest"]}"',
            f'digest = "sha256:{m["digest"]}"',
            "",
        ]
    return "\n".join(lines).rstrip("\n") + "\n"


def render_manifest(modules: list[dict]) -> str:
    deps = "\n".join(
        f'{m["package"]} = {{ path = "../../{m["source_path"]}" }}' for m in ordered(modules)
    )
    return f'''# Generated by tools/modules/compose.py. Do not edit by hand.
[package]
name = "world-modules"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[[bin]]
name = "world-modules"
path = "src/main.rs"
test = false

[dependencies]
anyhow = {{ workspace = true }}
tokio = {{ workspace = true }}
world-server = {{ workspace = true }}
wow-module-api = {{ workspace = true }}
{deps}
'''.replace("\n\n\n", "\n\n").rstrip("\n") + "\n"


def _rust_config(config: dict) -> str:
    if not config:
        return "        std::collections::BTreeMap::new()"
    rows = []
    for key, value in sorted(config.items()):
        if isinstance(value, bool):
            rendered = f"wow_module_api::ModuleConfigValue::Bool({'true' if value else 'false'})"
        elif isinstance(value, int):
            rendered = f"wow_module_api::ModuleConfigValue::Integer({value})"
        else:
            escaped = value.replace("\\", "\\\\").replace('"', '\\"')
            rendered = f'wow_module_api::ModuleConfigValue::Text("{escaped}".to_owned())'
        rows.append(f'            ("{key}".to_owned(), {rendered}),')
    body = "\n".join(rows)
    return "        std::collections::BTreeMap::from([\n" + body + "\n        ])"


def render_main(modules: list[dict]) -> str:
    rows = ordered(modules)
    if rows:
        blocks = []
        for m in rows:
            crate = m["registrar"].split("::")[0]
            blocks.append(
                f"    let config = wow_module_api::ModuleConfig::new(\n"
                f'        &wow_module_api::ModuleId::new("{m["id"]}")\n'
                f'            .expect("composed module ids are validated at sync"),\n'
                f"{_rust_config(m['config'])},\n"
                f"    );\n"
                f"    {m['registrar']}(&mut modules, config)\n"
                f'        .map_err(|error| anyhow::anyhow!("module {m["id"]}: {{error}}"))?;'
            )
        calls = "\n".join(blocks)
        summary = "\n".join(
            f"//! {i}. `{m['id']}` {m['version']} (config {m['config_digest']})"
            for i, m in enumerate(rows)
        )
    else:
        calls = "    // No modules are installed; this is the no-op compositor."
        summary = "//! No modules are installed."
    return f'''// Generated by tools/modules/compose.py. Do not edit by hand.
// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Composed world-server entry point for trusted linked modules.
//!
//! Registrars run in the operator's declared order, recorded in
//! `modules.lock.toml`. Ordering is explicit here and never relies on linker
//! inventory. Configuration is validated at sync and embedded, so no callback
//! reads a file. Installed modules, in composition order:
//!
{summary}

use std::process::ExitCode;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<ExitCode> {{
    let mut modules = wow_module_api::ModuleRegistry::new();
{calls}
    world_server::run_with_modules(std::env::args().skip(1).collect(), modules).await
}}
'''


def sync(quiet: bool = False) -> list[dict]:
    """Regenerate the lock and compositor. Returns the composed modules."""
    modules = read_modules()
    COMPOSITOR.joinpath("src").mkdir(parents=True, exist_ok=True)
    LOCK_PATH.write_text(render_lock(modules), encoding="utf-8")
    COMPOSITOR.joinpath("Cargo.toml").write_text(render_manifest(modules), encoding="utf-8")
    COMPOSITOR.joinpath("src/main.rs").write_text(render_main(modules), encoding="utf-8")
    if not quiet:
        names = ", ".join(m["id"] for m in ordered(modules)) or "none"
        print(f"composed {len(modules)} module(s): {names}")
    return modules


def check(quiet: bool = False) -> list[dict]:
    """Verify the tree matches the lock. Returns the composed modules."""
    modules = read_modules()
    stale = [
        name
        for name, path, expected in (
            ("modules.lock.toml", LOCK_PATH, render_lock(modules)),
            ("crates/world-modules/Cargo.toml", COMPOSITOR / "Cargo.toml", render_manifest(modules)),
            ("crates/world-modules/src/main.rs", COMPOSITOR / "src/main.rs", render_main(modules)),
        )
        if not path.is_file() or path.read_text(encoding="utf-8") != expected
    ]
    if stale:
        raise ComposeError(
            "composition is stale; run `python3 tools/modules/compose.py sync`:\n  "
            + "\n  ".join(stale)
        )
    if not quiet:
        print(f"composition is current for {len(modules)} module(s)")
    return modules


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("sync", "check"))
    args = parser.parse_args()
    try:
        sync() if args.command == "sync" else check()
        return 0
    except ComposeError as error:
        print(f"module composition failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
