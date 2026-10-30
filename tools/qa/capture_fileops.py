#!/usr/bin/env python3
"""Privileged, compare-and-swap file operations for qa-runtime capture runs."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import sys
import tempfile
import uuid


JOURNAL_NAME = ".qa-capture-journal.json"
BACKUP_NAME = ".qa-original-worldserver.conf"
CONFIG_DIGEST_KEYS = (
    "DataDir",
    "Locale",
    "DBC.Locale",
    "RealmID",
    "WorldServerPort",
    "InstanceServerPort",
    "PlayerSaveInterval",
    "DurabilityLoss.InPvP",
    "DurabilityLoss.OnDeath",
    "Corpse.Decay.Normal",
    "Corpse.Decay.Elite",
    "Corpse.Decay.RareElite",
    "Corpse.Decay.Obsolete",
    "Corpse.Decay.Rare",
    "Corpse.Decay.Trivial",
    "Corpse.Decay.MinusMob",
    "Rate.Corpse.Decay.Looted",
    "Rate.Damage.Fall",
    "Death.SicknessLevel",
    "Death.CorpseReclaimDelay.PvP",
    "Death.CorpseReclaimDelay.PvE",
    "Death.Bones.World",
    "Death.Bones.BattlegroundOrArena",
    "LogsDir",
    "PacketLogFile",
)
# Selected nonsecret lifecycle settings are shared with the active Rust config
# loader and the target TrinityCore World/Player/Unit paths documented for capture.


def fail(message: str) -> None:
    raise RuntimeError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def file_digest(path: Path) -> str:
    return digest(path.read_bytes())


def sync_directory(path: Path) -> None:
    fd = os.open(path, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def verify_output_dir(path: Path) -> None:
    if not path.is_absolute() or path.is_symlink() or not path.is_dir():
        fail("capture output directory is not a real absolute directory")
    if not re.fullmatch(r"/[A-Za-z0-9_./-]+", str(path)):
        fail("capture output directory contains unsupported path characters")
    if path.resolve(strict=True) != path:
        fail("capture output directory is not canonical")
    if stat.S_IMODE(path.stat().st_mode) != 0o700:
        fail("capture output directory mode is not 0700")


def verify_canonical_directory(path: Path, description: str) -> None:
    if (
        not path.is_absolute()
        or path.is_symlink()
        or not path.is_dir()
        or path.resolve(strict=True) != path
    ):
        fail(f"{description} is missing or unsafe")


def verify_canonical_file(path: Path, description: str) -> None:
    if (
        not path.is_absolute()
        or path.is_symlink()
        or not path.is_file()
        or path.resolve(strict=True) != path
    ):
        fail(f"{description} is missing or unsafe")


def effective_config_values(data: bytes) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in data.decode("utf-8", errors="replace").splitlines():
        if line.lstrip().startswith(("#", ";")) or "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key.strip()] = value.strip()
    return values


def effective_config_digest(data: bytes, keys: tuple[str, ...], extra: str) -> str:
    values = effective_config_values(data)
    canonical = []
    for key in keys:
        value = values.get(key, "<unset>")
        if re.search(r"DatabaseInfo|Password|Secret|Token|PrivateKey|SessionKey", key):
            value = "<redacted-present>" if key in values else "<unset>"
        canonical.append(f"{key}={value}")
    canonical.append(extra)
    return digest(("\n".join(canonical) + "\n").encode())


def require_cpp_packet_log_absent(data: bytes) -> None:
    value = effective_config_values(data).get("PacketLogFile", "<unset>")
    value = value.split("#", 1)[0].strip()
    if value not in ("", '""', "''", "<unset>"):
        fail("active C++ PacketLogFile is already configured; refusing to replace it")


def write_journal(path: Path, record: dict[str, object], *, create: bool) -> None:
    encoded = (json.dumps(record, sort_keys=True) + "\n").encode()
    if create:
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(fd, "wb") as stream:
            stream.write(encoded)
            stream.flush()
            os.fsync(stream.fileno())
        sync_directory(path.parent)
        return
    fd, temporary = tempfile.mkstemp(prefix=".qa-journal-", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as stream:
            stream.write(encoded)
            stream.flush()
            os.fsync(stream.fileno())
        os.chmod(temporary, 0o600)
        os.replace(temporary, path)
        sync_directory(path.parent)
    finally:
        if os.path.lexists(temporary):
            os.unlink(temporary)


def read_journal(path: Path) -> dict[str, object]:
    verify_output_dir(path.parent)
    if path.is_symlink() or not path.is_file():
        fail("capture recovery journal is missing or unsafe")
    record = json.loads(path.read_text(encoding="utf-8"))
    if record.get("journal") != str(path):
        fail("capture recovery journal path does not match its record")
    return record


def ensure_real_parents(path: Path) -> list[str]:
    missing: list[Path] = []
    current = path
    while not current.exists():
        if current.is_symlink():
            fail("systemd drop-in parent contains a symlink")
        missing.append(current)
        current = current.parent
    if current.is_symlink() or not current.is_dir():
        fail("systemd drop-in parent is not a real directory")
    for item in reversed(missing):
        item.mkdir(mode=0o755)
        sync_directory(item.parent)
    return [str(item) for item in missing]


def preflight_rust(args: argparse.Namespace) -> None:
    output = Path(args.output_dir)
    verify_output_dir(output)
    if not re.fullmatch(r"[A-Za-z0-9_.@:-]+", args.unit):
        fail("unsafe systemd unit name")
    if not re.fullmatch(r"/[A-Za-z0-9_./-]+", args.output_dir):
        fail("capture output path contains unsupported systemd characters")
    root = Path(args.systemd_root)
    verify_canonical_directory(root, "systemd runtime directory")
    dropin_dir = root / f"{args.unit}.service.d"
    if os.path.lexists(dropin_dir / "43-qa-capture.conf"):
        fail("capture drop-in already exists; refusing to overwrite it")
    current = dropin_dir
    while not current.exists():
        if current.is_symlink():
            fail("systemd drop-in parent contains a symlink")
        current = current.parent
    if current.is_symlink() or not current.is_dir():
        fail("systemd drop-in parent is not a real directory")
    if os.path.lexists(output / JOURNAL_NAME):
        fail("capture recovery journal already exists")
    config = Path(args.config)
    verify_canonical_file(config, "configured world config")
    print("ready")


def preflight_cpp(args: argparse.Namespace) -> None:
    output = Path(args.output_dir)
    verify_output_dir(output)
    config = Path(args.config)
    verify_canonical_file(config, "active C++ config")
    current = config.read_bytes()
    require_cpp_packet_log_absent(current)
    update_config(current, str(output))
    if os.path.lexists(output / JOURNAL_NAME) or os.path.lexists(output / BACKUP_NAME):
        fail("capture recovery path already exists")
    print("ready")


def atomic_replace(path: Path, data: bytes, uid: int, gid: int, mode: int, mtime_ns: int | None = None) -> None:
    fd, temporary = tempfile.mkstemp(prefix=".qa-capture-", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.chown(temporary, uid, gid)
        os.chmod(temporary, mode)
        if mtime_ns is not None:
            os.utime(temporary, ns=(mtime_ns, mtime_ns))
        os.replace(temporary, path)
        sync_directory(path.parent)
    finally:
        if os.path.lexists(temporary):
            os.unlink(temporary)


def prepare_rust(args: argparse.Namespace) -> None:
    creature_melee_trace = os.environ.get("RUSTYCORE_CREATURE_MELEE_TRACE")
    if creature_melee_trace not in (None, "1"):
        fail("RUSTYCORE_CREATURE_MELEE_TRACE must be exactly 1 when set")
    output = Path(args.output_dir)
    verify_output_dir(output)
    if not re.fullmatch(r"[A-Za-z0-9_.@:-]+", args.unit):
        fail("unsafe systemd unit name")
    if not re.fullmatch(r"/[A-Za-z0-9_./-]+", args.output_dir):
        fail("capture output path contains unsupported systemd characters")
    systemd_root = Path(args.systemd_root)
    verify_canonical_directory(systemd_root, "systemd runtime directory")
    config = Path(args.config)
    verify_canonical_file(config, "configured world config")
    journal = output / JOURNAL_NAME
    dropin_dir = systemd_root / f"{args.unit}.service.d"
    dropin = dropin_dir / "43-qa-capture.conf"
    if os.path.lexists(dropin):
        fail("capture drop-in already exists; refusing to overwrite it")
    run_id = uuid.uuid4().hex
    expected = (
        f"# RustyCore QA capture run {run_id}\n"
        f"[Service]\nEnvironment=RUSTYCORE_PACKET_DUMP_DIR={output}\n"
    ).encode()
    if creature_melee_trace == "1":
        expected += b"Environment=RUSTYCORE_CREATURE_MELEE_TRACE=1\n"
    created_dirs = [str(dropin_dir)] if not dropin_dir.exists() else []
    record: dict[str, object] = {
        "schema": 1,
        "backend": "rust",
        "journal": str(journal),
        "dropin": str(dropin),
        "dropin_sha256": digest(expected),
        "dropin_uid": os.geteuid(),
        "dropin_gid": os.getegid(),
        "dropin_mode": 0o644,
        "dropin_original_absent": True,
        "run_id": run_id,
        "created_dirs": created_dirs,
        "output_dir": str(output),
        "files_restored": False,
    }
    write_journal(journal, record, create=True)
    for directory in ensure_real_parents(dropin_dir):
        if directory not in created_dirs:
            created_dirs.append(directory)
    record["created_dirs"] = created_dirs
    write_journal(journal, record, create=False)
    verify_canonical_directory(systemd_root, "systemd runtime directory")
    verify_canonical_directory(dropin_dir, "systemd drop-in directory")
    fd = os.open(dropin, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o644)
    with os.fdopen(fd, "wb") as stream:
        stream.write(expected)
        stream.flush()
        os.fsync(stream.fileno())
    os.chown(dropin, int(record["dropin_uid"]), int(record["dropin_gid"]))
    os.chmod(dropin, int(record["dropin_mode"]))
    sync_directory(dropin.parent)
    if file_digest(dropin) != record["dropin_sha256"]:
        fail("installed capture drop-in hash mismatch")
    verify_canonical_file(config, "configured world config")
    config_data = config.read_bytes()
    before = effective_config_digest(
        config_data,
        CONFIG_DIGEST_KEYS,
        f"capture.backend=rust\ncapture.world_port={args.world_port}\n"
        f"capture.instance_port={args.instance_port}\n"
        "capture.RUSTYCORE_PACKET_DUMP_DIR=<unset>",
    )
    captured = effective_config_digest(
        config_data,
        CONFIG_DIGEST_KEYS,
        f"capture.backend=rust\ncapture.world_port={args.world_port}\n"
        f"capture.instance_port={args.instance_port}\n"
        "capture.RUSTYCORE_PACKET_DUMP_DIR=<private>",
    )
    print(f"{before}\t{captured}")


def update_config(data: bytes, output_dir: str) -> bytes:
    text = data.decode("utf-8", errors="strict")
    replacements = {"LogsDir": f'"{output_dir}"', "PacketLogFile": '"cpp.pkt"'}
    counts = {key: 0 for key in replacements}
    updated: list[str] = []
    for line in text.splitlines(keepends=True):
        body = line.rstrip("\r\n")
        ending = line[len(body) :]
        for key, value in replacements.items():
            match = re.match(rf"^(\s*{key}\s*=\s*)(.*)$", body)
            if match:
                counts[key] += 1
                current = match.group(2)
                comment = ""
                if "#" in current:
                    current, comment_tail = current.split("#", 1)
                    comment = " #" + comment_tail
                body = match.group(1) + value + comment
                break
        updated.append(body + ending)
    if any(count != 1 for count in counts.values()):
        fail("active C++ config must contain exactly one LogsDir and PacketLogFile key")
    return "".join(updated).encode("utf-8")


def prepare_cpp(args: argparse.Namespace) -> None:
    output = Path(args.output_dir)
    verify_output_dir(output)
    config = Path(args.config)
    verify_canonical_file(config, "active C++ config")
    original_stat = config.stat()
    original = config.read_bytes()
    require_cpp_packet_log_absent(original)
    updated = update_config(original, str(output))
    journal = output / JOURNAL_NAME
    backup = output / BACKUP_NAME
    if os.path.lexists(journal) or os.path.lexists(backup):
        fail("capture recovery path already exists")
    record: dict[str, object] = {
        "schema": 1,
        "backend": "cpp",
        "journal": str(journal),
        "config": str(config),
        "backup": str(backup),
        "original_sha256": digest(original),
        "installed_sha256": digest(updated),
        "original_uid": original_stat.st_uid,
        "original_gid": original_stat.st_gid,
        "original_mode": stat.S_IMODE(original_stat.st_mode),
        "original_mtime_ns": original_stat.st_mtime_ns,
        "installed_uid": original_stat.st_uid,
        "installed_gid": original_stat.st_gid,
        "installed_mode": stat.S_IMODE(original_stat.st_mode),
        "output_dir": str(output),
        "files_restored": False,
    }
    write_journal(journal, record, create=True)
    verify_canonical_file(config, "active C++ config")
    shutil.copy2(config, backup, follow_symlinks=False)
    os.chown(backup, original_stat.st_uid, original_stat.st_gid)
    os.chmod(backup, stat.S_IMODE(original_stat.st_mode))
    with backup.open("rb") as backup_stream:
        os.fsync(backup_stream.fileno())
    sync_directory(output)
    if file_digest(backup) != record["original_sha256"]:
        fail("active C++ config backup hash mismatch")
    verify_canonical_file(config, "active C++ config")
    current_data = config.read_bytes()
    current_stat = config.stat()
    if (
        digest(current_data) != record["original_sha256"]
        or current_stat.st_uid != original_stat.st_uid
        or current_stat.st_gid != original_stat.st_gid
        or stat.S_IMODE(current_stat.st_mode) != stat.S_IMODE(original_stat.st_mode)
        or current_stat.st_mtime_ns != original_stat.st_mtime_ns
    ):
        fail("active C++ config drifted after backup; refusing to install capture settings")
    require_cpp_packet_log_absent(current_data)
    verify_canonical_file(config, "active C++ config")
    atomic_replace(
        config,
        updated,
        original_stat.st_uid,
        original_stat.st_gid,
        stat.S_IMODE(original_stat.st_mode),
    )
    if file_digest(config) != record["installed_sha256"]:
        fail("installed C++ capture config hash mismatch")
    before_extra = (
        f"capture.backend=cpp\ncapture.world_port={args.world_port}\n"
        f"capture.instance_port={args.instance_port}\n"
        "capture.RUSTYCORE_PACKET_DUMP_DIR=<unset>"
    )
    capture_extra = (
        f"capture.backend=cpp\ncapture.world_port={args.world_port}\n"
        f"capture.instance_port={args.instance_port}\n"
        "capture.RUSTYCORE_PACKET_DUMP_DIR=<unset>"
    )
    print(
        f"{effective_config_digest(original, CONFIG_DIGEST_KEYS, before_extra)}\t"
        f"{effective_config_digest(updated, CONFIG_DIGEST_KEYS, capture_extra)}"
    )


def restore_files(args: argparse.Namespace) -> None:
    journal = Path(args.journal)
    record = read_journal(journal)
    if record.get("files_restored") is True:
        if record.get("backend") == "rust":
            dropin = Path(str(record["dropin"]))
            verify_canonical_directory(dropin.parent.parent, "systemd runtime directory")
            if dropin.parent.is_symlink():
                fail("systemd drop-in directory became a symlink")
            if dropin.parent.exists():
                verify_canonical_directory(dropin.parent, "systemd drop-in directory")
            if os.path.lexists(dropin):
                fail("capture drop-in reappeared after restoration")
        elif record.get("backend") == "cpp":
            config = Path(str(record["config"]))
            verify_canonical_file(config, "active C++ config")
            info = config.stat()
            if (
                file_digest(config) != record["original_sha256"]
                or info.st_uid != record["original_uid"]
                or info.st_gid != record["original_gid"]
                or stat.S_IMODE(info.st_mode) != record["original_mode"]
                or info.st_mtime_ns != record["original_mtime_ns"]
            ):
                fail("active C++ config changed after restoration")
        else:
            fail("unknown capture journal backend")
        print("already-restored")
        return
    if record.get("backend") == "rust":
        dropin = Path(str(record["dropin"]))
        verify_canonical_directory(dropin.parent.parent, "systemd runtime directory")
        if dropin.parent.is_symlink():
            fail("systemd drop-in directory became a symlink")
        if dropin.parent.exists():
            verify_canonical_directory(dropin.parent, "systemd drop-in directory")
        if record.get("dropin_original_absent") is not True:
            fail("capture journal does not prove the drop-in was originally absent")
        if dropin.is_symlink():
            fail("run-owned capture drop-in became a symlink; refusing restoration")
        if not os.path.lexists(dropin):
            for directory in reversed(record.get("created_dirs", [])):
                try:
                    Path(str(directory)).rmdir()
                    sync_directory(Path(str(directory)).parent)
                except OSError:
                    pass
            record["files_restored"] = True
            write_journal(journal, record, create=False)
            print("restored-absent")
            return
        if not dropin.is_file():
            fail("run-owned capture drop-in is not a regular file")
        verify_canonical_directory(dropin.parent, "systemd drop-in directory")
        verify_canonical_file(dropin, "run-owned capture drop-in")
        info = dropin.stat()
        if (
            file_digest(dropin) != record["dropin_sha256"]
            or info.st_uid != record["dropin_uid"]
            or info.st_gid != record["dropin_gid"]
            or stat.S_IMODE(info.st_mode) != record["dropin_mode"]
        ):
            fail("run-owned capture drop-in drifted; refusing restoration")
        dropin.unlink()
        sync_directory(dropin.parent)
        for directory in reversed(record.get("created_dirs", [])):
            try:
                Path(str(directory)).rmdir()
                sync_directory(Path(str(directory)).parent)
            except OSError:
                pass
    elif record.get("backend") == "cpp":
        config = Path(str(record["config"]))
        backup = Path(str(record["backup"]))
        verify_canonical_file(config, "active C++ config")
        info = config.stat()
        current = file_digest(config)
        if current == record["installed_sha256"]:
            if (
                info.st_uid != record["installed_uid"]
                or info.st_gid != record["installed_gid"]
                or stat.S_IMODE(info.st_mode) != record["installed_mode"]
            ):
                fail("active C++ config metadata drifted; refusing restoration")
            verify_canonical_file(backup, "active C++ config backup")
            if file_digest(backup) != record["original_sha256"]:
                fail("active C++ config backup has drifted")
            backup_info = backup.stat()
            if (
                backup_info.st_uid != record["original_uid"]
                or backup_info.st_gid != record["original_gid"]
                or stat.S_IMODE(backup_info.st_mode) != record["original_mode"]
            ):
                fail("active C++ config backup metadata has drifted")
            verify_canonical_file(config, "active C++ config")
            atomic_replace(
                config,
                backup.read_bytes(),
                int(record["original_uid"]),
                int(record["original_gid"]),
                int(record["original_mode"]),
                int(record["original_mtime_ns"]),
            )
        elif current != record["original_sha256"]:
            fail("active C++ config drifted; refusing restoration")
        restored = config.stat()
        if (
            file_digest(config) != record["original_sha256"]
            or restored.st_uid != record["original_uid"]
            or restored.st_gid != record["original_gid"]
            or stat.S_IMODE(restored.st_mode) != record["original_mode"]
            or restored.st_mtime_ns != record["original_mtime_ns"]
        ):
            fail("active C++ config did not restore to its original hash and metadata")
    else:
        fail("unknown capture journal backend")
    record["files_restored"] = True
    write_journal(journal, record, create=False)
    print("restored")


def complete(args: argparse.Namespace) -> None:
    journal = Path(args.journal)
    record = read_journal(journal)
    if record.get("files_restored") is not True:
        fail("capture config has not been restored")
    if record.get("backend") == "rust":
        dropin = Path(str(record["dropin"]))
        verify_canonical_directory(dropin.parent.parent, "systemd runtime directory")
        if dropin.parent.is_symlink():
            fail("systemd drop-in directory became a symlink")
        if dropin.parent.exists():
            verify_canonical_directory(dropin.parent, "systemd drop-in directory")
        if os.path.lexists(dropin):
            fail("capture drop-in still exists after restoration")
    elif record.get("backend") == "cpp":
        config = Path(str(record["config"]))
        verify_canonical_file(config, "active C++ config")
        info = config.stat()
        if (
            file_digest(config) != record["original_sha256"]
            or info.st_uid != record["original_uid"]
            or info.st_gid != record["original_gid"]
            or stat.S_IMODE(info.st_mode) != record["original_mode"]
            or info.st_mtime_ns != record["original_mtime_ns"]
        ):
            fail("active C++ config changed after restoration")
    else:
        fail("unknown capture journal backend")
    backup = record.get("backup")
    if backup:
        backup_path = Path(str(backup))
        if backup_path.is_symlink():
            fail("capture config backup became a symlink")
        if backup_path.exists():
            backup_path.unlink()
            sync_directory(backup_path.parent)
    journal.unlink()
    sync_directory(journal.parent)
    print("complete")


def inventory(args: argparse.Namespace) -> None:
    output = Path(args.output_dir)
    verify_output_dir(output)
    suffixes = {"rust": {".bin", ".meta"}, "cpp": {".pkt"}}[args.backend]
    artifacts = []
    for path in sorted(output.iterdir(), key=lambda item: item.name):
        if path.suffix.lower() not in suffixes:
            continue
        if path.is_symlink() or not path.is_file():
            fail("native capture artifact is not a regular file")
        data = path.read_bytes()
        artifacts.append({"name": path.name, "size": len(data), "sha256": digest(data)})
    print(json.dumps(artifacts, sort_keys=True, separators=(",", ":")))


def main() -> int:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="action", required=True)
    rust = sub.add_parser("prepare-rust")
    rust.add_argument("--output-dir", required=True)
    rust.add_argument("--systemd-root", required=True)
    rust.add_argument("--unit", required=True)
    rust.add_argument("--world-port", required=True)
    rust.add_argument("--instance-port", required=True)
    rust.add_argument("--config", required=True)
    rust.set_defaults(func=prepare_rust)
    rust_check = sub.add_parser("preflight-rust")
    rust_check.add_argument("--output-dir", required=True)
    rust_check.add_argument("--systemd-root", required=True)
    rust_check.add_argument("--unit", required=True)
    rust_check.add_argument("--config", required=True)
    rust_check.set_defaults(func=preflight_rust)
    cpp = sub.add_parser("prepare-cpp")
    cpp.add_argument("--output-dir", required=True)
    cpp.add_argument("--config", required=True)
    cpp.add_argument("--world-port", required=True)
    cpp.add_argument("--instance-port", required=True)
    cpp.set_defaults(func=prepare_cpp)
    cpp_check = sub.add_parser("preflight-cpp")
    cpp_check.add_argument("--output-dir", required=True)
    cpp_check.add_argument("--config", required=True)
    cpp_check.set_defaults(func=preflight_cpp)
    restore = sub.add_parser("restore")
    restore.add_argument("--journal", required=True)
    restore.set_defaults(func=restore_files)
    finish = sub.add_parser("complete")
    finish.add_argument("--journal", required=True)
    finish.set_defaults(func=complete)
    listing = sub.add_parser("inventory")
    listing.add_argument("--output-dir", required=True)
    listing.add_argument("--backend", choices=("rust", "cpp"), required=True)
    listing.set_defaults(func=inventory)
    args = parser.parse_args()
    try:
        args.func(args)
    except (OSError, ValueError, RuntimeError, json.JSONDecodeError) as error:
        print(f"capture file operation failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
