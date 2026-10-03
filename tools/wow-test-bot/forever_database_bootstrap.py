#!/usr/bin/env python3
"""Create NEW, isolated Forever databases from pinned upstream SQL. Never reset.

Source: advocaite/TrinityCore 02245dcd README database setup; TrinityCore
TDB1210.26091 release asset with GitHub's SHA-256. No source/SQL/error dump.
The disposable auth container/account is the only permitted destination.
"""

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

SOURCE_SHA = "02245dcd245e7433e524577656177723d3e4992e"
ARCHIVE_SHA = "fc5513334d7534a19f533124a95910193c8150379e5ed8d0db3e547ac2c5a3f5"
CONTAINER = "rustycore-forever-auth-70170"
SCHEMAS = {kind: f"{kind}_forever_70170" for kind in ("characters", "world", "hotfixes")}
# The first local World bootstrap was stopped before a qualified-schema patch,
# after discovering a filename-order bug. Preserve that diagnostic copy; the
# corrected source-ordered bootstrap has a NEW destination, never an overwrite.
SCHEMAS["world"] = "world_forever_70170_02245"
MYSQL = 'export MYSQL_PWD="$MARIADB_ROOT_PASSWORD"; exec mariadb -uroot --batch --skip-column-names'


def command(args, data=None):
    result = subprocess.run(args, input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if result.returncode:
        # MariaDB errors can contain an entire SQL statement. Do not render it.
        metadata = re.search(rb"ERROR ([0-9]+) \([A-Z0-9]+\)(?: at line ([0-9]+))?", result.stderr)
        details = f"exit={result.returncode}"
        if metadata:
            details += f", mysql_code={metadata[1].decode()}, line={(metadata[2] or b'0').decode()}"
        raise RuntimeError(f"isolated operation failed ({details}); no SQL/error text rendered")
    return result.stdout


def query(sql, schema=None):
    destination = MYSQL + (f" {schema}" if schema else "")
    return command(["docker", "exec", "-i", CONTAINER, "sh", "-c", destination], sql.encode())


def source(reference, path):
    return command(["git", "-C", str(reference), "show", f"{SOURCE_SHA}:{path}"])


def validate_sql(sql):
    # Ignore comments and values, not quoted identifiers. Otherwise ordinary
    # descriptions containing 'world.' falsely look like database references.
    sql = re.sub(rb"'(?:''|\\.|[^'])*'|\"(?:\"\"|\\.|[^\"])*\"", b"''", sql)
    sql = re.sub(rb"(?m)--[^\n]*|\#[^\n]*", b"", sql)
    sql = re.sub(rb"/\*(?!\!).*?\*/", b"", sql, flags=re.S)
    sql = re.sub(rb"/\*![0-9]*\s*|\*/", b"", sql)
    # Do not let an upstream dump switch out of the explicitly selected schema.
    if re.search(rb"(?im)(?:^|;)\s*(?:USE\b|(?:DROP|CREATE)\s+DATABASE\b|SOURCE\b)", sql):
        raise RuntimeError("cross-schema SQL directive rejected")
    if re.search(rb"(?i)\b(?:FROM|JOIN|INTO|UPDATE|TABLE)\s+`?(?:auth|characters|world|hotfixes)`?\s*\.", sql):
        raise RuntimeError("cross-schema SQL reference rejected")


def adapt_schema(sql, kind):
    """Rebind this dump's explicit schema ONLY in SQL code, never string values."""
    protected = rb"'(?:''|\\.|[^'])*'|\"(?:\"\"|\\.|[^\"])*\"|--[^\n]*|\#[^\n]*|/\*.*?\*/"
    pieces = re.split(b"(" + protected + b")", sql, flags=re.S)
    qualifier = re.compile(rb"(?i)(?<![A-Za-z0-9_])`?" + kind.encode() + rb"`?\s*\.")
    for index in range(0, len(pieces), 2):
        pieces[index] = qualifier.sub(b"`" + SCHEMAS[kind].encode() + b"`.", pieces[index])
    result = b"".join(pieces)
    validate_sql(result)
    return result


def schema_exists(kind):
    return query(f"SELECT COUNT(*) FROM information_schema.schemata WHERE schema_name='{SCHEMAS[kind]}';").strip() != b"0"


def update_paths(paths, kind):
    selected = [path for path in paths if path.endswith(".sql") and (
        path.startswith(f"sql/updates/{kind}/master/") or path.startswith(f"sql/custom/{kind}/")
    )]
    names = [Path(path).name for path in selected]
    if len(names) != len(set(names)):
        raise RuntimeError("duplicate update filename rejected")
    return sorted(selected, key=lambda path: Path(path).name)


def update_needed(name, sql, applied):
    if name not in applied:
        return True
    if applied[name].lower() != hashlib.sha1(sql).hexdigest():
        raise RuntimeError("already applied target update has a different source hash")
    return False


def validate_target(reference):
    root = Path(__file__).resolve().parents[2]
    runtime = root / "target" / "forever-login"
    if reference.resolve() != root / "target" / "forever-cpp-reference":
        raise RuntimeError("reference must be this checkout's pinned target reference")
    head = command(["git", "-C", str(reference), "rev-parse", "HEAD"]).decode().strip()
    if head != SOURCE_SHA:
        raise RuntimeError("target reference revision mismatch")
    ports = json.loads(command(["docker", "inspect", "--format", "{{json .NetworkSettings.Ports}}", CONTAINER]))
    if ports.get("3306/tcp") != [{"HostIp": "127.0.0.1", "HostPort": "13316"}]:
        raise RuntimeError("container is not the loopback-only isolated target")
    if query("SELECT COUNT(*) FROM auth_forever_70170.account WHERE id=1 AND username='1#1' AND battlenet_account=1;").strip() != b"1":
        raise RuntimeError("disposable auth identity missing")
    return runtime


def add_auth_session_tables(reference):
    validate_target(reference)
    names = (
        "battlenet_account_transmog_outfits", "battlenet_account_warband_scenes",
        "battlenet_account_player_data_element", "battlenet_account_player_data_flag",
    )
    sql = source(reference, "sql/base/auth_database.sql")
    # Never import the auth dump: it would overwrite the fixture's credentials.
    statements = []
    for name in names:
        if query(f"SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='auth_forever_70170' AND table_name='{name}';").strip() != b"0":
            raise RuntimeError("auth addition already exists; no overwrite mode")
        match = re.search(rb"CREATE TABLE `" + name.encode() + rb"` \(.*?;", sql, re.S)
        if not match:
            raise RuntimeError("pinned auth table definition missing")
        validate_sql(match.group())
        statements.append(match.group())
    for statement in statements:
        query(statement.decode(), "auth_forever_70170")
    print("Added four pinned session-only auth tables; existing account/auth rows untouched.")


def bootstrap(reference, archive, kinds, resume_empty=False):
    runtime = validate_target(reference)
    if archive.resolve().parent != runtime or archive.is_symlink():
        raise RuntimeError("archive must be a regular file inside the isolated runtime")
    with archive.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    if digest != ARCHIVE_SHA:
        raise RuntimeError("release archive digest mismatch")
    # Check every destination before creating any. No DROP/reuse mode is offered.
    existing = {kind for kind in kinds if schema_exists(kind)}
    if existing:
        if not resume_empty:
            raise RuntimeError("destination already exists; refusing overwrite or automatic recovery")
        for kind in existing:
            if query(f"SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='{SCHEMAS[kind]}';").strip() != b"0":
                raise RuntimeError("resume target contains tables; refusing overwrite")
    paths = command(["git", "-C", str(reference), "ls-tree", "-r", "--name-only", SOURCE_SHA, "sql/updates", "sql/custom"]).decode().splitlines()
    report = {"source_sha": SOURCE_SHA, "tdb_sha256": digest, "schemas": {}}
    for kind in kinds:
        schema = SCHEMAS[kind]
        if kind not in existing:
            query(f"CREATE DATABASE `{schema}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;")
        print(f"Creating isolated {kind} schema (new destination only).", flush=True)
        if kind == "characters":
            base = source(reference, "sql/base/characters_database.sql")
        else:
            name = f"TDB_full_{kind}_1210.26091_2026_09_09.sql"
            base = command(["7z", "x", "-so", str(archive), name])
        query(adapt_schema(base, kind).decode(), schema)
        applied = dict(line.split("\t") for line in query("SELECT name,hash FROM updates;", schema).decode().splitlines())
        changes = []
        # UpdateFetcher::PathCompare orders the unique FILENAME, not directory.
        # sql/custom must not run before earlier sql/updates prerequisites.
        for path in update_paths(paths, kind):
            name = Path(path).name
            sql = source(reference, path)
            if not update_needed(name, sql, applied):
                continue
            print(f"Applying {path}", flush=True)
            query(adapt_schema(sql, kind).decode(), schema)
            sha1 = hashlib.sha1(sql).hexdigest()
            query(f"REPLACE INTO updates (name,hash,state,speed) VALUES ('{name}','{sha1}','RELEASED',0);", schema)
            changes.append({"path": path, "sha1": sha1})
        query(f"GRANT ALL PRIVILEGES ON `{schema}`.* TO 'forever'@'%';")
        tables = int(query(f"SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='{schema}';").strip())
        report["schemas"][schema] = {"tables": tables, "updates": changes}
        print(f"Isolated {kind} ready: {tables} tables, {len(changes)} target updates.", flush=True)
    print(json.dumps(report, sort_keys=True))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ack-new-isolated-databases", action="store_true", required=True)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--kind", choices=SCHEMAS, action="append")
    parser.add_argument("--add-session-auth-tables", action="store_true")
    parser.add_argument("--resume-empty-schema", action="store_true", help="only accept a destination with zero tables; never reset")
    args = parser.parse_args()
    try:
        kinds = list(dict.fromkeys(args.kind or []))
        if bool(kinds) != bool(args.archive) or (not kinds and not args.add_session_auth_tables):
            raise RuntimeError("select new schemas with archive, or the explicit session-only auth addition")
        if args.add_session_auth_tables:
            add_auth_session_tables(args.reference)
        if kinds:
            bootstrap(args.reference, args.archive, kinds, args.resume_empty_schema)
    except RuntimeError as error:
        print(str(error))  # Only messages constructed here, never child error text.
        print("Existing data retained; no automatic reset. No secrets printed.")
        return 1
    except (OSError, ValueError, json.JSONDecodeError):
        print("Forever bootstrap rejected/failed. Existing data retained; no automatic reset. No secrets printed.")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
