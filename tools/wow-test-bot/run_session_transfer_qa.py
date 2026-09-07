#!/usr/bin/env python3
"""Guarded #585 portal fixture; fixed TESTBOT1/guid 14, no row deletion.

Normal usage requires --allow-position-fixture, --source and --world-exec.
Recovery: --allow-position-fixture --recover /absolute/private/journal.json.
Only map/instance/zone/XYZ/orientation are restored, never a stale whole-character
snapshot. The existing qa-runtime guard owns executable swap and restoration.
"""
import argparse
import contextlib
import decimal
import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
LIVE = ROOT / "target/deploy/live/world-server"
FIELDS = ("map", "instance_id", "zone", "position_x", "position_y", "position_z", "orientation")
WHERE = "guid=14 AND account=8"
SOURCE = ("0", "0", "1519", "-8346.46", "514.031", "96.5989", "0")


def command(args, **kwargs):
    return subprocess.run(args, check=True, text=True, **kwargs)


def sql(query):
    # Socket authentication: no credentials in argv, logs or journals.
    return command(["sudo", "-n", "mariadb", "--batch", "--skip-column-names"],
                   input=query, capture_output=True).stdout.strip()


def service(action):
    command(["sudo", "-n", "systemctl", action, "world-server"])


def digest(path):
    with open(path, "rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def numeric(values):
    if len(values) != len(FIELDS):
        raise RuntimeError("invalid location field count")
    for value in values:
        if not isinstance(value, str) or not decimal.Decimal(value).is_finite():
            raise RuntimeError("invalid location value")
    # Normalize, so SQL never receives executable text from a journal.
    return tuple(str(decimal.Decimal(value)) for value in values)


def location():
    return numeric(sql(f"SELECT {','.join(FIELDS)} FROM characters.characters WHERE {WHERE}").split("\t"))


def preflight():
    rows = sql("SELECT c.guid,c.account,c.online FROM characters.characters c "
               "JOIN auth.account a ON a.id=c.account "
               "JOIN auth.battlenet_accounts b ON b.id=a.battlenet_account "
               "WHERE b.email='TESTBOT1@bot.local'")
    if rows != "14\t8\t0" or sql("SELECT COUNT(*) FROM characters.characters WHERE online<>0") != "0":
        raise RuntimeError("requires exact sole offline TESTBOT1 and no online characters")
    dest = sql("SELECT w.MapID,w.LocX,w.LocY,w.LocZ FROM world.areatrigger_teleport a "
               "JOIN world.world_safe_locs w ON w.ID=a.PortLocID WHERE a.ID=2173")
    if dest != "369\t67.7607\t2490.98\t-4.29649":
        raise RuntimeError("portal destination differs from the reviewed fixture")


def write_location(values):
    values = numeric(values)
    assignments = ",".join(f"{field}={value}" for field, value in zip(FIELDS, values))
    sql(f"UPDATE characters.characters SET {assignments} WHERE {WHERE} AND online=0")
    if tuple(map(decimal.Decimal, location())) != tuple(map(decimal.Decimal, values)):
        raise RuntimeError("location write could not be verified")


@contextlib.contextmanager
def runtime_lock():
    with open("/tmp/rustycore-qa-runtime.lock", "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        yield


def wait_serving():
    deadline = time.monotonic() + 90
    while time.monotonic() < deadline:
        output = command(["sudo", "-n", "ss", "-lntp"], capture_output=True).stdout
        pid = command(["systemctl", "show", "world-server", "-p", "MainPID", "--value"],
                      capture_output=True).stdout.strip()
        if pid != "0" and all(any(f":{port} " in line and f"pid={pid}," in line
                                  for line in output.splitlines()) for port in (8085, 8086)):
            return
        time.sleep(0.5)
    raise RuntimeError("world-server did not resume serving")


def persist(path, value):
    # Initial journal exists before mutation. Atomic state replacement preserves it
    # across an interrupted later update; the directory is private (mkdtemp).
    temporary = path.with_suffix(".pending")
    with open(temporary, "w", encoding="utf8") as out:
        os.chmod(temporary, 0o600)
        json.dump(value, out, indent=2)
        out.flush()
        os.fsync(out.fileno())
    os.replace(temporary, path)
    fd = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def recover(path):
    journal = json.loads(path.read_text())
    if journal.get("schema") != "session-transfer-585-v1" or journal.get("identity") != [14, 8]:
        raise RuntimeError("unrecognized recovery journal")
    numeric(journal["original_location"])
    if journal.get("restored"):
        print("Fixture already restored; no writes")
        return
    with runtime_lock():
        # Never overwrite the bot's save while its process owner can still run.
        service("stop")
        if journal.get("admitted"):
            preflight()
            write_location(journal["original_location"])
        if digest(LIVE) != journal["original_executable"]:
            raise RuntimeError("runtime guard has not restored the original executable; server remains stopped")
        service("start")
        wait_serving()
        journal["restored"] = True
        persist(path, journal)
    print("Original character location and serving executable verified")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--allow-position-fixture", action="store_true", required=True)
    parser.add_argument("--source", type=Path)
    parser.add_argument("--world-exec", type=Path)
    parser.add_argument("--recover", type=Path)
    args = parser.parse_args()
    if args.recover:
        recover(args.recover.resolve(strict=True))
        return
    if not args.source or not args.world_exec:
        parser.error("--source and --world-exec required for a run")
    source = args.source.resolve(strict=True)
    candidate = args.world_exec.resolve(strict=True)
    if command(["git", "-C", str(source), "status", "--porcelain"], capture_output=True).stdout:
        raise RuntimeError("source checkout must be clean")
    command(["systemctl", "is-active", "--quiet", "world-server"])
    preflight()
    directory = Path(tempfile.mkdtemp(prefix="rustycore-session-transfer-585."))
    path = directory / "journal.json"
    journal = {"schema": "session-transfer-585-v1", "identity": [14, 8],
               "original_location": location(), "original_executable": digest(LIVE),
               "restored": False, "admitted": False}
    persist(path, journal)
    print(f"Recovery journal: {path}", flush=True)
    try:
        with runtime_lock():
            service("stop")
            preflight()
            if location() != tuple(journal["original_location"]):
                raise RuntimeError("location changed before fixture admission")
            journal["admitted"] = True
            persist(path, journal)
            write_location(SOURCE)
            service("start")
            wait_serving()
        env = os.environ.copy()
        env.update(BNET_HOST="127.0.0.1", BNET_PORT="8081",
                   QA_GIT_DIR=str(source), WOW_BOT_LOGIN_PORTAL_CHECK="1",
                   QA_SERVICE="world-server", QA_SYSTEMCTL="sudo -n systemctl",
                   QA_LIVE_DIR=str(LIVE.parent), QA_LIVE_NAME=LIVE.name,
                   QA_LOCK="/tmp/rustycore-qa-runtime.lock", QA_SKIP_PORT_GUARD="0",
                   QA_WORLD_PORT="8085", QA_INSTANCE_PORT="8086",
                   QA_BOT=str(ROOT / "tools/wow-test-bot/target/debug/wow-test-bot"),
                   QA_BOT_DIR=str(ROOT / "tools/wow-test-bot"),
                   QA_SMOKE=str(ROOT / "tools/wow-test-bot/run_login_disconnect_relog.sh"))
        command([str(ROOT / "tools/qa-runtime.sh"), "--allow-runtime-qa",
                 "--world-exec", str(candidate), "--report", str(directory / "runtime.json"), "login"], env=env)
    finally:
        recover(path)


if __name__ == "__main__":
    main()
