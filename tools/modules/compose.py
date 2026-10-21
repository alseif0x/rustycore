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

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import module_sql  # noqa: E402

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
MODULES_DIR = REPO_ROOT / "modules"
LOCK_PATH = REPO_ROOT / "modules.lock.toml"
COMPOSITOR = REPO_ROOT / "crates" / "world-modules"
# Mirrors `wow_module_api::SOURCE_API` / `COMPATIBLE_SOURCE_APIS`; the module
# manager tests assert both sides agree. API 2 (#1445) is additive over API 1.
SUPPORTED_SOURCE_API = "2"
# Source APIs this server still accepts. A module asking for anything else
# fails at composition, long before a player logs in.
COMPATIBLE_SOURCE_APIS = {"1", "2"}
# The one workspace crate a module may depend on (ADR-002, #1445 S0).
MODULE_API_PACKAGE = "wow-module-api"
CONFIG_OVERRIDE_DIR = REPO_ROOT / "conf" / "modules"
# The core migration manifest a module's `min_core_schema` is checked against.
CORE_MIGRATIONS = REPO_ROOT / "database" / "migrations" / "manifest.toml"
# Per-module SQL (#1444 H6): `data/sql/manifest.toml` inside the checkout.
MODULE_SQL_DIR = pathlib.PurePosixPath("data/sql")
# Databases a module may change. auth and hotfixes are closed to modules.
MODULE_DATABASES = ("characters", "world")
# The lock's tombstone block starts at this line. `rustycore-db module remove`
# writes it and `module purge` rewrites it; compose keeps it byte for byte.
TOMBSTONE_MARKER = "# --- tombstones: written by rustycore-db module remove; kept as is by compose.py ---"

ID_RE = re.compile(r"^[a-z][a-z0-9_.]{0,63}$")
PACKAGE_RE = re.compile(r"^[a-z][a-z0-9_-]{0,63}$")
PATH_RE = re.compile(r"^[A-Za-z0-9_./-]+$")
REGISTRAR_RE = re.compile(r"^[a-z][a-z0-9_]*(::[a-z][a-z0-9_]*)+$")
VERSION_RE = re.compile(r"^\d+\.\d+\.\d+$")
# A migration or core schema version: dot-separated numbers, compared as such.
SCHEMA_VERSION_RE = re.compile(r"^\d{1,9}(\.\d{1,9}){0,7}$")
SQL_FILE_RE = re.compile(r"^[A-Za-z0-9_][A-Za-z0-9_./-]*\.sql$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")


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
    reject_server_dependencies(root, resolved, where)

    source_api = str(compat.get("source_api", ""))
    _require(
        source_api in COMPATIBLE_SOURCE_APIS,
        f"{where}: source_api {source_api!r} is not supported; this server provides "
        f"{sorted(COMPATIBLE_SOURCE_APIS)}. Update the module or pin an older server.",
    )

    config = load_config(identifier, raw.get("config", {}), where)
    sql = read_module_sql(identifier, root)

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
        "sql": sql,
    }


def schema_version(text: str) -> tuple[int, ...]:
    return tuple(int(part) for part in text.split("."))


def table_prefix(module_id: str) -> str:
    """The table prefix a module owns; mirrors `module_sql::table_prefix`."""
    return "mod_" + module_id.replace(".", "_") + "_"


def sql_sha256(path: pathlib.Path) -> str:
    """SHA-256 of the normalized text, as `migration::normalized_source` hashes it."""
    text = path.read_text(encoding="utf-8").replace("\r\n", "\n").replace("\r", "\n")
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def core_schema_versions() -> list[tuple[int, ...]]:
    _require(CORE_MIGRATIONS.is_file(), f"core migration manifest {CORE_MIGRATIONS} is missing")
    core = tomllib.loads(CORE_MIGRATIONS.read_text(encoding="utf-8"))
    return [
        schema_version(m["version"])
        for m in core.get("migrations", [])
        if m.get("component") == "core" and SCHEMA_VERSION_RE.match(str(m.get("version", "")))
    ]


def read_module_sql(module_id: str, root: pathlib.Path) -> dict | None:
    """Validate `data/sql/manifest.toml`; `None` when the module ships no SQL.

    Format 1:
      format = 1
      min_core_schema = "2026.08.02.00"   # the core migration it builds on
      [[migrations]] database, version, description, file
      [[purge]]      database, file       # reviewed DROP scripts for `purge`
    Files are relative to `data/sql`. The lock records each file's SHA-256.
    """
    sql_dir = root / MODULE_SQL_DIR
    manifest = sql_dir / "manifest.toml"
    if not manifest.is_file():
        return None
    where = manifest.relative_to(REPO_ROOT)
    raw = tomllib.loads(manifest.read_text(encoding="utf-8"))
    _require(raw.get("format") == 1, f"{where}: unsupported format {raw.get('format')!r}")
    unknown = set(raw) - {"format", "min_core_schema", "migrations", "purge"}
    _require(not unknown, f"{where}: unknown keys {sorted(unknown)}")
    minimum = str(raw.get("min_core_schema", ""))
    _require(
        bool(SCHEMA_VERSION_RE.match(minimum)),
        f"{where}: min_core_schema {minimum!r} must be a core migration version such as 2026.08.02.00",
    )
    core = core_schema_versions()
    _require(
        any(version >= schema_version(minimum) for version in core),
        f"{where}: min_core_schema {minimum} is newer than this server's core schema",
    )

    prefix = table_prefix(module_id)

    def entry_file(entry: dict, label: str) -> tuple[str, str]:
        file = str(entry.get("file", ""))
        _require(
            bool(SQL_FILE_RE.match(file)) and ".." not in pathlib.PurePosixPath(file).parts,
            f"{where}: {label} file {file!r} must be a relative .sql path inside data/sql",
        )
        path = sql_dir / file
        _require(path.is_file(), f"{where}: {label} file {file} is missing")
        # The static guard: only the module's own prefixed tables (#1444 H6).
        problems = module_sql.check_script(
            path.read_text(encoding="utf-8"), prefix, purge=label == "purge"
        )
        _require(
            not problems,
            f"{where}: {file} is refused by the module SQL guard (prefix {prefix}): "
            + "; ".join(problems),
        )
        return file, sql_sha256(path)

    def database_of(entry: dict, label: str) -> str:
        database = str(entry.get("database", ""))
        _require(
            database in MODULE_DATABASES,
            f"{where}: {label} database {database!r}: a module may change only "
            f"{' and '.join(MODULE_DATABASES)}",
        )
        return database

    migrations, seen = [], set()
    for entry in raw.get("migrations", []):
        database = database_of(entry, "migration")
        version = str(entry.get("version", ""))
        _require(
            bool(SCHEMA_VERSION_RE.match(version)),
            f"{where}: migration version {version!r} must be dot-separated numbers",
        )
        _require((database, version) not in seen, f"{where}: duplicate migration {database} {version}")
        seen.add((database, version))
        description = str(entry.get("description", "")).strip()
        _require(0 < len(description) <= 255, f"{where}: migration {version} needs a description")
        _require(
            '"' not in description
            and "\\" not in description
            and not any(ch.isspace() and ch != " " or not ch.isprintable() for ch in description),
            f"{where}: description of {version} may not contain quotes, backslashes, newlines "
            f"or control characters",
        )
        file, sha = entry_file(entry, "migration")
        migrations.append(
            {"database": database, "version": version, "description": description, "file": file, "sha256": sha}
        )
    _require(bool(migrations), f"{where}: declares no migrations")
    migrations.sort(key=lambda m: (MODULE_DATABASES.index(m["database"]), schema_version(m["version"])))
    purge = []
    for entry in raw.get("purge", []):
        database = database_of(entry, "purge")
        file, sha = entry_file(entry, "purge")
        purge.append({"database": database, "file": file, "sha256": sha})
    purge.sort(key=lambda p: (MODULE_DATABASES.index(p["database"]), p["file"]))
    return {
        "min_core_schema": minimum,
        "source": str((root / MODULE_SQL_DIR).relative_to(REPO_ROOT)),
        "migrations": migrations,
        "purge": purge,
    }


def workspace_packages() -> dict[str, pathlib.Path]:
    """Every package the server workspace provides, by name, with its directory.

    Reads the root `[workspace] members` and, so a partial checkout is still
    covered, every `crates/*/Cargo.toml`.
    """
    found: dict[str, pathlib.Path] = {}
    candidates: list[pathlib.Path] = []
    root_manifest = REPO_ROOT / "Cargo.toml"
    if root_manifest.is_file():
        members = tomllib.loads(root_manifest.read_text(encoding="utf-8"))
        candidates += [
            REPO_ROOT / member
            for member in members.get("workspace", {}).get("members", [])
            if "*" not in member
        ]
    candidates += sorted(p.parent for p in (REPO_ROOT / "crates").glob("*/Cargo.toml"))
    for directory in candidates:
        manifest = directory / "Cargo.toml"
        if not manifest.is_file():
            continue
        name = tomllib.loads(manifest.read_text(encoding="utf-8")).get("package", {}).get("name")
        if name:
            found.setdefault(name, directory.resolve())
    return found


def _normal_dependency_tables(cargo: dict) -> list[dict]:
    """`[dependencies]` plus every `[target.<cfg>.dependencies]`."""
    tables = [cargo.get("dependencies", {})]
    tables += [t.get("dependencies", {}) for t in cargo.get("target", {}).values()]
    return tables


def reject_server_dependencies(root: pathlib.Path, crate_dir: pathlib.Path, where) -> None:
    """A module may depend on no workspace crate except `wow-module-api`.

    This is ADR-002's real invariant (#1445 S0): the public contract is the
    only server surface a module compiles against, so it cannot obtain a
    session, a map or a packet writer through a type. The rule follows every
    path dependency that stays inside the module checkout, so a helper crate
    there cannot carry a server dependency in on the module's behalf.
    Development dependencies are not checked; `wow-module-api` re-exports
    `ObjectGuid`, so a module's tests need no other server crate either.
    """
    workspace = workspace_packages()
    api_dir = workspace.get(MODULE_API_PACKAGE)
    root_deps = {}
    root_manifest = REPO_ROOT / "Cargo.toml"
    if root_manifest.is_file():
        root_deps = (
            tomllib.loads(root_manifest.read_text(encoding="utf-8"))
            .get("workspace", {})
            .get("dependencies", {})
        )
    module_root = root.resolve()
    pending = [crate_dir.resolve()]
    visited: set[pathlib.Path] = set()
    while pending:
        current = pending.pop()
        if current in visited:
            continue
        visited.add(current)
        manifest = current / "Cargo.toml"
        if not manifest.is_file():
            continue
        cargo = tomllib.loads(manifest.read_text(encoding="utf-8"))
        for table in _normal_dependency_tables(cargo):
            for key, spec in table.items():
                spec = spec if isinstance(spec, dict) else {}
                package = spec.get("package", key)
                base = current
                if spec.get("workspace") is True:
                    inherited = root_deps.get(key, {})
                    spec = inherited if isinstance(inherited, dict) else {}
                    package = spec.get("package", key)
                    base = REPO_ROOT
                target = (base / spec["path"]).resolve() if "path" in spec else None
                inside_server = target is not None and target.is_relative_to(REPO_ROOT.resolve())
                inside_module = target is not None and target.is_relative_to(module_root)
                is_api = package == MODULE_API_PACKAGE and (target is None or target == api_dir)
                if is_api:
                    continue
                _require(
                    package not in workspace and not (inside_server and not inside_module),
                    f"{where}: the module crate depends on server crate {package!r}; a module "
                    f"may depend only on {MODULE_API_PACKAGE} among the server's crates "
                    f"(ADR-002)",
                )
                if inside_module:
                    pending.append(target)


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


def render_lock(modules: list[dict], tombstone_block: str = "") -> str:
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
        if m.get("sql"):
            lines += render_sql_entries("module", m["sql"])
    text = "\n".join(lines).rstrip("\n") + "\n"
    if tombstone_block:
        text += "\n" + tombstone_block
    return text


def render_sql_entries(table: str, sql: dict) -> list[str]:
    """The SQL lines of one `[[module]]` (or tombstone) entry."""
    lines = [
        f"[{table}.sql]",
        f'min_core_schema = "{sql["min_core_schema"]}"',
        f'source = "{sql["source"]}"',
        "",
    ]
    for m in sql["migrations"]:
        lines += [
            f"[[{table}.sql.migration]]",
            f'database = "{m["database"]}"',
            f'version = "{m["version"]}"',
            f'description = "{m["description"]}"',
            f'file = "{m["file"]}"',
            f'sha256 = "{m["sha256"]}"',
            "",
        ]
    for p in sql["purge"]:
        lines += [
            f"[[{table}.sql.purge]]",
            f'database = "{p["database"]}"',
            f'file = "{p["file"]}"',
            f'sha256 = "{p["sha256"]}"',
            "",
        ]
    return lines


def split_lock(text: str) -> tuple[str, str]:
    """The generated part and the preserved tombstone block of a lock.

    The marker counts only as a whole line: a quoted value that happens to
    contain its text (a description, say) is not the block.
    """
    if text.startswith(TOMBSTONE_MARKER + "\n"):
        return "", text
    index = text.find("\n" + TOMBSTONE_MARKER + "\n")
    if index < 0:
        return text, ""
    return text[: index + 1], text[index + 1 :]


def read_previous_lock() -> tuple[dict, str]:
    """The current lock, parsed, and its tombstone block verbatim.

    The block is kept byte for byte, so it is validated on its own first: it
    may hold nothing but `[[tombstone]]` entries with SQL.
    """
    if not LOCK_PATH.is_file():
        return {}, ""
    text = LOCK_PATH.read_text(encoding="utf-8")
    _, block = split_lock(text)
    try:
        parsed = tomllib.loads(text)
        tombstones = tomllib.loads(block) if block else {}
    except tomllib.TOMLDecodeError as error:
        raise ComposeError(f"modules.lock.toml is not valid TOML: {error}") from error
    _require(
        set(tombstones) <= {"tombstone"},
        f"the tombstone block of modules.lock.toml may hold only [[tombstone]] entries, "
        f"not {sorted(set(tombstones) - {'tombstone'})}",
    )
    for entry in tombstones.get("tombstone", []):
        _require(
            isinstance(entry, dict)
            and bool(ID_RE.match(str(entry.get("id", ""))))
            and isinstance(entry.get("sql"), dict)
            and isinstance(entry["sql"].get("migration", []), list),
            f"modules.lock.toml: invalid tombstone entry {entry!r:.80}",
        )
    _require(
        parsed.get("tombstone", []) == tombstones.get("tombstone", []),
        "modules.lock.toml: [[tombstone]] entries must all be inside the tombstone block",
    )
    return parsed, block


def check_sql_history(modules: list[dict], previous: dict) -> None:
    """Module SQL history is append-only and never silently forgotten.

    - A migration recorded in the lock keeps its file checksum: a change needs
      a new migration file, because the old one may already be applied.
    - A module whose SQL is in the lock cannot disappear without a tombstone
      (`rustycore-db module remove`), which keeps its history acceptable.
    - Two modules, live or tombstoned, cannot share a table prefix.
    """
    tombstones = {t.get("id"): t for t in previous.get("tombstone", [])}
    current = {m["id"]: m for m in modules}
    for old in previous.get("module", []):
        old_sql = old.get("sql")
        if not old_sql:
            continue
        identifier = old.get("id")
        now = current.get(identifier)
        if now is None:
            _require(
                identifier in tombstones,
                f"module {identifier} has SQL in modules.lock.toml but its checkout is gone; "
                f"restore it and run `rustycore-db module remove {identifier}` first, so its "
                f"history and data stay accepted (a tombstone)",
            )
            continue
        recorded = {(m["database"], m["version"]): m["sha256"] for m in old_sql.get("migration", [])}
        new_sql = now.get("sql") or {"migrations": []}
        present = {(m["database"], m["version"]): m["sha256"] for m in new_sql["migrations"]}
        for key, sha in recorded.items():
            _require(
                key in present,
                f"module {identifier}: migration {key[0]} {key[1]} was removed from data/sql; "
                f"recorded migrations are permanent",
            )
            _require(
                present[key] == sha,
                f"module {identifier}: migration {key[0]} {key[1]} changed after it was recorded; "
                f"add a new migration file instead",
            )
    for identifier, tombstone in tombstones.items():
        now = current.get(identifier)
        if now is None:
            continue
        # A re-installed module continues its tombstoned history.
        present = {
            (m["database"], m["version"]): m["sha256"]
            for m in (now.get("sql") or {"migrations": []})["migrations"]
        }
        for m in tombstone.get("sql", {}).get("migration", []):
            _require(
                present.get((m["database"], m["version"])) == m["sha256"],
                f"module {identifier} is re-installed but its data/sql does not continue the "
                f"tombstoned history ({m['database']} {m['version']})",
            )
    prefixes: dict[str, str] = {}
    with_sql = {identifier for identifier, m in current.items() if m.get("sql")}
    for identifier in sorted(with_sql | set(tombstones)):
        prefix = table_prefix(identifier)
        _require(
            prefix not in prefixes or prefixes[prefix] == identifier,
            f"modules {prefixes.get(prefix)} and {identifier} share the table prefix {prefix}",
        )
        prefixes[prefix] = identifier


def has_sql(modules: list[dict], tombstone_block: str) -> bool:
    return bool(tombstone_block) or any(m.get("sql") for m in modules)


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

[lints]
workspace = true
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


def render_main(modules: list[dict], with_sql: bool = False) -> str:
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
    if with_sql:
        # The lock carries every module's SQL history, tombstones included;
        # the startup schema check accepts exactly that history (#1444 H6).
        run = (
            "    world_server::run_with_composition(\n"
            "        std::env::args().skip(1).collect(),\n"
            "        modules,\n"
            '        include_str!("../../../modules.lock.toml"),\n'
            "    )\n"
            "    .await"
        )
    else:
        run = "    world_server::run_with_modules(std::env::args().skip(1).collect(), modules).await"
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
{run}
}}
'''


def sync(quiet: bool = False) -> list[dict]:
    """Regenerate the lock and compositor. Returns the composed modules."""
    modules = read_modules()
    previous, tombstones = read_previous_lock()
    check_sql_history(modules, previous)
    COMPOSITOR.joinpath("src").mkdir(parents=True, exist_ok=True)
    LOCK_PATH.write_text(render_lock(modules, tombstones), encoding="utf-8")
    COMPOSITOR.joinpath("Cargo.toml").write_text(render_manifest(modules), encoding="utf-8")
    COMPOSITOR.joinpath("src/main.rs").write_text(
        render_main(modules, has_sql(modules, tombstones)), encoding="utf-8"
    )
    if not quiet:
        names = ", ".join(m["id"] for m in ordered(modules)) or "none"
        print(f"composed {len(modules)} module(s): {names}")
    return modules


def check(quiet: bool = False) -> list[dict]:
    """Verify the tree matches the lock. Returns the composed modules."""
    modules = read_modules()
    previous, tombstones = read_previous_lock()
    check_sql_history(modules, previous)
    with_sql = has_sql(modules, tombstones)
    stale = [
        name
        for name, path, expected in (
            ("modules.lock.toml", LOCK_PATH, render_lock(modules, tombstones)),
            ("crates/world-modules/Cargo.toml", COMPOSITOR / "Cargo.toml", render_manifest(modules)),
            ("crates/world-modules/src/main.rs", COMPOSITOR / "src/main.rs", render_main(modules, with_sql)),
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
