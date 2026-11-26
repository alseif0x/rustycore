#!/usr/bin/env python3
"""Run the #46 four-client Northshire movement stability workload.

The normal mode keeps four pre-provisioned QA characters connected in parallel
for two-hour waves.  ``--preflight`` runs one short, explicitly non-acceptance
wave to provide startup feedback.  Runtime capture and restoration belong to
the caller; this runner only writes inside its private capture directory.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import signal
import stat
import subprocess
import sys
import time
from typing import Any

import instance_soak
import verify_four_client_movement


SCENARIO = "four-client-movement-stability"
PREFLIGHT_SCENARIO = "four-client-movement-preflight"
CHARACTERS = (15, 16, 17, 18)
BNET_HOST = "127.0.0.1"
BNET_PORT = 14881
WORLD_HOST = "127.0.0.1"
WORLD_PORT = 14885
INSTANCE_HOST = "127.0.0.1"
INSTANCE_PORT = 14886
REALM_ID = 1
CLIENT_BUILD = 54261
NORTHSHIRE_ORIGIN = (-8948.0, -80.0, 88.78)
NORTHSHIRE_LANE_GAP = 2.5
MOVE_HEARTBEAT = 0x3A10
REQUEST_RAID_INFO = 0x36D2
INSTANCE_INFO = 0x2634
LOGOUT_COMPLETE = 0x2684
QUEUE_FIELDS = instance_soak.QUEUE_FIELDS
QUEUE_MESSAGE = instance_soak.QUEUE_MESSAGE
PACKET_NAME = re.compile(
    r"^rust-(?P<direction>c2s|s2c)(?:-unencrypted)?-"
    r"(?P<seq>[0-9]+)-counter[0-9]+-0x[0-9A-Fa-f]{4}-"
    r"(?P<name>[A-Za-z0-9_-]+)-len[0-9]+\.(?P<extension>bin|meta)$"
)
KEEP_PACKETS = {
    ("s2c", "LoginVerifyWorld"),
    ("s2c", "PhaseShiftChange"),
    ("c2s", "MoveHeartbeat"),
    ("s2c", "MoveUpdate"),
}
PAIR_SETTLE_SECONDS = 2.0
INCOMPLETE_PAIR_SECONDS = 30.0
CAPTURE_SCAN_SECONDS = 2.0
ScenarioFailure = instance_soak.death_scenario.ScenarioFailure


def _fail(message: str) -> None:
    raise ScenarioFailure(message)


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _write_private(path: Path, value: Any, *, exclusive: bool = True) -> None:
    flags = os.O_WRONLY | os.O_CREAT | (os.O_EXCL if exclusive else os.O_TRUNC)
    descriptor = os.open(path, flags, 0o600)
    with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
        json.dump(value, stream, sort_keys=True, indent=2)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())


def _canonical_file(path: Path, description: str) -> Path:
    if path.is_symlink() or not path.is_file():
        _fail(f"{description} is missing or unsafe")
    resolved = path.resolve(strict=True)
    if resolved != path:
        _fail(f"{description} is not canonical")
    return path


def _capture_directory(path: Path) -> Path:
    if (not path.is_absolute() or path.is_symlink() or not path.is_dir()
            or path.resolve(strict=True) != path or path.stat().st_mode & 0o777 != 0o700):
        _fail("capture directory must be canonical, absolute, and mode 0700")
    return path


def _read_json(path: Path, description: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        _fail(f"{description} is missing or malformed")
    if not isinstance(value, dict):
        _fail(f"{description} has an unexpected shape")
    return value


def _account_password_name(account: str) -> str:
    suffix = "".join(
        character.upper() if character.isascii() and character.isalnum() else "_"
        for character in account
    )
    return f"WOW_BOT_PASSWORD_{suffix}"


def _configured_bots(config_path: Path, env: dict[str, str]) -> dict[int, dict[str, Any]]:
    config = _read_json(config_path, "prepared bot config")
    bots = config.get("bots")
    if not isinstance(bots, list):
        _fail("prepared bot config has no bot list")
    selected: dict[int, dict[str, Any]] = {}
    accounts: set[str] = set()
    enabled_accounts: dict[str, int] = {}
    for bot in bots:
        if not isinstance(bot, dict) or bot.get("enabled") is not True:
            continue
        account_name = bot.get("account")
        if isinstance(account_name, str) and account_name.strip():
            key = account_name.casefold()
            enabled_accounts[key] = enabled_accounts.get(key, 0) + 1
        guid = bot.get("character_guid")
        if type(guid) is not int or guid not in CHARACTERS:
            continue
        if guid in selected:
            _fail("prepared bot config contains a duplicate selected character")
        account = bot.get("account")
        if not isinstance(account, str) or not account.strip():
            _fail("prepared bot config has a selected character without an account")
        account_key = account.casefold()
        if account_key in accounts:
            _fail("the four selected QA characters must use four distinct accounts")
        accounts.add(account_key)
        password = bot.get("password")
        if not isinstance(password, str):
            password = ""
        password_available = bool(password.strip()) or bool(env.get("WOW_BOT_PASSWORD", "").strip())
        password_available |= bool(env.get(_account_password_name(account), "").strip())
        if not password_available:
            _fail("prepared bot environment is missing a selected account credential")
        selected[guid] = {"account": account}
    if set(selected) != set(CHARACTERS):
        _fail("prepared bot config must enable exactly QA character GUIDs 15, 16, 17, and 18")
    if any(enabled_accounts.get(account.casefold(), 0) != 1
           for account in (bot["account"] for bot in selected.values())):
        _fail("each selected QA account must identify exactly one enabled bot config")
    return selected


def _bot_environment(base: dict[str, str], account: str, config_path: Path,
                     report_path: Path) -> dict[str, str]:
    allowed = {
        "WOW_BOT_EXEC", "WOW_BOT_EXEC_SHA256", "WOW_BOT_DB_CONF",
        "WOW_BOT_AUTH_DB_URL", "WOW_BOT_CHARACTERS_DB_URL", "WOW_BOT_WORLD_DB_URL",
    }
    env = {key: value for key, value in base.items() if key in allowed}
    for key in ("WOW_BOT_PASSWORD", _account_password_name(account)):
        value = base.get(key)
        if value:
            env[key] = value
    env.update({
        "WOW_BOT_CONFIG": str(config_path),
        "WOW_BOT_ENV_FILE": "/dev/null",
        "WOW_BOT_ENSURE_TEST_ACCOUNTS": "0",
        "WOW_BOT_GENERATE_LOCAL_PASSWORD": "0",
        "WOW_BOT_CLIENT_BUILD": str(CLIENT_BUILD),
        "WOW_BOT_REPORT": str(report_path),
        "BNET_HOST": BNET_HOST,
        "BNET_PORT": str(BNET_PORT),
        "WORLD_HOST": WORLD_HOST,
        "WORLD_PORT": str(WORLD_PORT),
        "INSTANCE_HOST": INSTANCE_HOST,
        "INSTANCE_PORT": str(INSTANCE_PORT),
        "REALM_ID": str(REALM_ID),
    })
    return env


def _process_identity_from_env(env: dict[str, str]) -> tuple[int, int]:
    try:
        pid = int(env["RUSTYCORE_CAPTURE_PID"])
        starttime = int(env["RUSTYCORE_CAPTURE_STARTTIME"])
    except (KeyError, ValueError):
        _fail("runner is missing the captured world PID/starttime identity")
    if pid <= 0 or starttime <= 0:
        _fail("captured world PID/starttime identity is invalid")
    instance_soak.read_memory(pid, starttime)
    return pid, starttime


def _packet_paths(capture: Path) -> dict[str, dict[str, Any]]:
    groups: dict[str, dict[str, Any]] = {}
    for path in capture.iterdir():
        if not path.name.startswith("rust-"):
            continue
        match = PACKET_NAME.fullmatch(path.name)
        if not match or path.is_symlink() or not path.is_file():
            _fail("capture directory contains an unsafe or unrecognized Rust packet artifact")
        fields = match.groupdict()
        stem = path.name.rsplit(".", 1)[0]
        row = groups.setdefault(stem, {
            "seq": int(fields["seq"]), "direction": fields["direction"],
            "name": fields["name"], "paths": {},
        })
        if row["seq"] != int(fields["seq"]) or fields["extension"] in row["paths"]:
            _fail("capture packet file naming is ambiguous")
        row["paths"][fields["extension"]] = path
    return groups


class ProcessStdoutQueueSampler:
    """Read fresh queue counters from the captured server's private stdout file."""

    def __init__(self, path: Path, pid: int, starttime: int):
        if not path.is_absolute() or path.is_symlink() or not path.is_file():
            _fail("server stdout log must be a canonical absolute regular file")
        self.path = path.resolve(strict=True)
        if self.path != path:
            _fail("server stdout log path is not canonical")
        self.pid = pid
        self.starttime = starttime
        try:
            self.fd = os.open(path, os.O_RDONLY | getattr(os, "O_CLOEXEC", 0)
                              | getattr(os, "O_NOFOLLOW", 0))
        except OSError:
            _fail("cannot open the captured server stdout log read-only")
        info = os.fstat(self.fd)
        if not stat.S_ISREG(info.st_mode) or stat.S_IMODE(info.st_mode) & 0o077:
            os.close(self.fd)
            _fail("server stdout log must be private and regular")
        self.device = info.st_dev
        self.inode = info.st_ino
        self.offset = info.st_size
        self.pending = b""
        try:
            self._check_identity()
        except BaseException:
            os.close(self.fd)
            raise

    def _check_identity(self) -> None:
        instance_soak.read_memory(self.pid, self.starttime)
        try:
            descriptor = os.fstat(self.fd)
            named = os.stat(self.path)
            process_stdout = os.stat(f"/proc/{self.pid}/fd/1")
        except OSError:
            _fail("captured server stdout log or fd 1 is no longer readable")
        identities = {(row.st_dev, row.st_ino) for row in (descriptor, named, process_stdout)}
        if len(identities) != 1 or descriptor.st_dev != self.device or descriptor.st_ino != self.inode:
            _fail("server stdout log no longer matches the captured process fd 1 inode")
        if not stat.S_ISREG(descriptor.st_mode) or descriptor.st_size < self.offset:
            _fail("server stdout log was truncated or replaced during sampling")

    @staticmethod
    def _decode_queue_line(line: bytes) -> dict[str, int] | None:
        text = line.decode("utf-8", errors="replace")
        if QUEUE_MESSAGE not in text:
            return None
        text = re.sub(r"\x1b\[[0-9;]*m", "", text)
        fields = {name: int(value) for name, value in
                  re.findall(r"\b([a-z_]+)=([0-9]+)\b", text)}
        if any(name not in fields for name in QUEUE_FIELDS):
            _fail("server stdout queue line is missing a required counter")
        return {name: fields[name] for name in QUEUE_FIELDS}

    def sample(self) -> dict[str, Any]:
        deadline = time.monotonic() + 75
        while time.monotonic() < deadline:
            self._check_identity()
            info = os.fstat(self.fd)
            if info.st_size > self.offset:
                data = os.pread(self.fd, min(256 * 1024, info.st_size - self.offset), self.offset)
                if not data:
                    _fail("server stdout log stopped advancing while queue counters were due")
                self.offset += len(data)
                chunks = (self.pending + data).splitlines(keepends=True)
                self.pending = b""
                if chunks and not chunks[-1].endswith((b"\n", b"\r")):
                    self.pending = chunks.pop()
                latest = None
                for line in chunks:
                    counters = self._decode_queue_line(line)
                    if counters is not None:
                        latest = counters
                if latest is not None:
                    return {**latest, "monotonic_us": time.monotonic_ns() // 1000}
            time.sleep(1)
        raise ScenarioFailure("no fresh queue counters in captured server stdout")

    def close(self) -> None:
        os.close(self.fd)


class CapturePruner:
    """Keep this run's movement and relogin-position streams in the active capture directory."""

    def __init__(self, capture: Path, max_bytes: int, first_sequence: int | None = None):
        self.capture = capture
        self.max_bytes = max_bytes
        self.owned_stems: set[str] = set()
        existing = _packet_paths(capture)
        if first_sequence is not None and first_sequence < 1:
            _fail("explicit first capture sequence must be positive")
        self.first_sequence = first_sequence or 1
        self.preexisting_stems = set(existing)
        if existing and first_sequence is None:
            _fail("capture directory contains pre-existing Rust packet files; supply an explicit sequence boundary")
        if any(row["seq"] >= self.first_sequence for row in existing.values()):
            _fail("explicit first sequence does not follow all existing capture files")
        if any(set(row["paths"]) != {"bin", "meta"} for row in existing.values()):
            _fail("pre-existing capture has an incomplete Rust packet file pair")
        self.max_sequence = self.first_sequence - 1
        existing_bytes = sum(path.stat().st_size for row in existing.values()
                             for path in row["paths"].values())
        if existing_bytes > max_bytes:
            _fail("pre-existing capture files exceed the configured disk bound")

    def scan_and_prune(self) -> None:
        now = time.time()
        groups = _packet_paths(self.capture)
        total = 0
        for stem, row in groups.items():
            if stem in self.preexisting_stems:
                total += sum(path.stat().st_size for path in row["paths"].values())
                continue
            if row["seq"] < self.first_sequence:
                _fail("packet capture produced a frame before the explicit sequence boundary")
            self.owned_stems.add(stem)
            self.max_sequence = max(self.max_sequence, row["seq"])
            paths: dict[str, Path] = row["paths"]
            age = now - min(path.stat().st_mtime for path in paths.values())
            keep = (row["direction"], row["name"]) in KEEP_PACKETS
            if not keep and set(paths) == {"bin", "meta"} and age >= PAIR_SETTLE_SECONDS:
                for path in paths.values():
                    if stem not in self.owned_stems or path.parent != self.capture:
                        _fail("refusing to recycle a packet file not owned by this run")
                    path.unlink()
                continue
            if set(paths) != {"bin", "meta"} and age >= INCOMPLETE_PAIR_SECONDS:
                _fail("packet capture left an incomplete file pair")
            for path in paths.values():
                total += path.stat().st_size
        if total > self.max_bytes:
            _fail("retained packet captures exceeded the configured disk bound")

    def current_range(self) -> tuple[int, int]:
        return self.max_sequence + 1, self.max_sequence

    def artifacts(self, first: int, last: int) -> list[dict[str, Any]]:
        rows = []
        for stem, row in _packet_paths(self.capture).items():
            if first <= row["seq"] <= last:
                for path in row["paths"].values():
                    rows.append({"name": path.name, "size": path.stat().st_size,
                                 "sha256": _sha256(path)})
        return sorted(rows, key=lambda item: item["name"])


def build_plan(character: int, wave: int, origin: tuple[float, float, float],
               active_seconds: int) -> dict[str, Any]:
    if character not in CHARACTERS or active_seconds < 20:
        raise ValueError("plan requires one selected character and at least 20 active seconds")
    active_ms = active_seconds * 1000
    slots = max(2, min(8, (active_ms - 10_000) // 35_000))
    initial_ms = min(10_000, active_ms // 5)
    raid_wait_ms = min(5_000, active_ms // 10)
    spacing_ms = max(1_000, (active_ms - initial_ms - raid_wait_ms) // slots)
    final_ms = max(1_000, active_ms - initial_ms - slots * spacing_ms)
    lane = (character - CHARACTERS[0]) * NORTHSHIRE_LANE_GAP
    cycle_shift = (wave % 4) * 0.2
    x, y, z = origin
    route = ((x + cycle_shift, y + lane, z),
             (x + cycle_shift + 1.25, y + lane, z))
    actions: list[dict[str, Any]] = [{"action": "wait", "milliseconds": initial_ms}]
    raid_slot = slots // 2
    for slot in range(slots):
        px, py, pz = route[slot % len(route)]
        actions.append({
            "action": "movement", "kind": "heartbeat", "time_ms": 1_000 + slot * 35_000,
            "x": px, "y": py, "z": pz, "orientation": 0.0,
        })
        if slot == raid_slot:
            actions.extend((
                {"action": "request_raid_info"},
                {"action": "wait_for_packet", "connection": "realm",
                 "opcode": INSTANCE_INFO, "timeout_ms": raid_wait_ms},
            ))
        actions.append({"action": "wait", "milliseconds": spacing_ms})
    actions.append({"action": "wait", "milliseconds": final_ms})
    return {
        "character_guid": character,
        "timeout_ms": active_ms + 15_000,
        "actions": actions,
        "expect": [{"event": "packet", "connection": "realm",
                    "opcode": LOGOUT_COMPLETE, "body_hex": ""}],
        "finish_policy": "logout",
    }


def _raid_info_count(body_hex: Any) -> int:
    if not isinstance(body_hex, str) or len(body_hex) % 2:
        _fail("RaidInfo response body is malformed")
    try:
        body = bytes.fromhex(body_hex)
    except ValueError:
        _fail("RaidInfo response body is malformed")
    if len(body) < 4:
        _fail("RaidInfo response is shorter than its lock count")
    count = int.from_bytes(body[:4], "little", signed=True)
    if count < 0 or len(body) != 4 + count * 25:
        _fail("RaidInfo response count does not match its rows")
    return count


def validate_bot_report(report_path: Path, character: int,
                        expected_position: tuple[float, float, float]) -> dict[str, Any]:
    report = _read_json(report_path, "bot report")
    results = report.get("results")
    if not isinstance(results, list):
        _fail("bot report has no result list")
    matches = [row for row in results if isinstance(row, dict)
               and row.get("character_guid") == character]
    if len(matches) != 1:
        _fail("bot report does not contain exactly the selected character")
    evidence = matches[0].get("cast_lifecycle")
    if not isinstance(evidence, dict) or evidence.get("failure") is not None:
        _fail("cast-lifecycle evidence is missing or failed")
    if (evidence.get("passed") is not True or evidence.get("finish_policy") != "logout"
            or evidence.get("finish_completed") is not True
            or evidence.get("logout_request_completed") is not True
            or evidence.get("logout_confirmed") is not True
            or evidence.get("logout_route") != "realm"):
        _fail("bot did not complete the planned realm logout")
    if evidence.get("server_disconnect_connection") is not None:
        _fail("server disconnected the bot before normal logout completed")
    sent = [row for row in evidence.get("sent_packets", [])
            if isinstance(row, dict) and row.get("action") == "request_raid_info"]
    if (len(sent) != 1 or sent[0].get("connection") != "realm"
            or sent[0].get("opcode") != REQUEST_RAID_INFO
            or sent[0].get("body_hex") != "" or sent[0].get("write_completed") is not True):
        _fail("bot did not send exactly one typed realm RaidInfo request")
    request_observation = sent[0].get("observation_sequence")
    responses = [row for row in evidence.get("events", [])
                 if isinstance(row, dict) and row.get("connection") == "realm"
                 and row.get("opcode") == INSTANCE_INFO
                 and isinstance(request_observation, int)
                 and isinstance(row.get("observation_sequence"), int)
                 and row["observation_sequence"] > request_observation]
    if len(responses) != 1:
        _fail("RaidInfo request did not receive exactly one later realm response")
    lock_count = _raid_info_count(responses[0].get("body_hex"))
    state = evidence.get("movement_state")
    if not isinstance(state, dict) or state.get("map_id") != 0:
        _fail("selected character was not in Northshire's world map")
    actual = (state.get("x"), state.get("y"), state.get("z"))
    if any(not isinstance(value, (int, float)) or not math.isfinite(value)
           for value in actual):
        _fail("bot report lacks a finite Northshire movement position")
    if any(not math.isclose(float(actual_value), expected, abs_tol=0.001)
           for actual_value, expected in zip(actual, expected_position, strict=True)):
        _fail("bot's final movement position differs from the planned Northshire point")
    return {"character_guid": character, "raid_info_lock_count": lock_count,
            "movement_position": list(expected_position),
            "logout_confirmed": True, "disconnected": True}


def _terminate_children(children: dict[int, subprocess.Popen[Any]]) -> None:
    for process in children.values():
        if process.poll() is None:
            process.terminate()
    deadline = time.monotonic() + 5
    for process in children.values():
        if process.poll() is None:
            try:
                process.wait(timeout=max(0.0, deadline - time.monotonic()))
            except subprocess.TimeoutExpired:
                process.kill()
    for process in children.values():
        if process.poll() is None:
            process.wait()


def run_wave(*, wave: int, active_seconds: int, wave_seconds: int,
             capture: Path, run_dir: Path, config_path: Path, bot_path: Path,
             configured: dict[int, dict[str, Any]], base_env: dict[str, str],
             origin: tuple[float, float, float], pruner: CapturePruner,
             pid: int, starttime: int, preflight: bool) -> dict[str, Any]:
    wave_dir = run_dir / f"wave-{wave:03d}"
    wave_dir.mkdir(mode=0o700)
    first, _ = pruner.current_range()
    wave_started = time.monotonic()
    children: dict[int, subprocess.Popen[Any]] = {}
    log_streams = []
    last_capture_scan = 0.0
    try:
        for character in CHARACTERS:
            account = configured[character]["account"]
            plan = build_plan(character, wave, origin, active_seconds)
            plan_path = wave_dir / f"plan-{character}.json"
            report_path = wave_dir / f"bot-{character}.json"
            log_path = wave_dir / f"bot-{character}.log"
            _write_private(plan_path, plan)
            log_fd = os.open(log_path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            log_stream = os.fdopen(log_fd, "wb")
            log_streams.append(log_stream)
            command = [
                str(bot_path), "--config", str(config_path), "--single", account,
                "--cast-lifecycle", "--cast-lifecycle-plan", str(plan_path),
                "--no-auto-teleport", "--report", str(report_path),
            ]
            children[character] = subprocess.Popen(
                command, cwd=bot_path.parent,
                env=_bot_environment(base_env, account, config_path, report_path),
                stdin=subprocess.DEVNULL, stdout=log_stream, stderr=subprocess.STDOUT,
                close_fds=True,
            )

        timeout = active_seconds + 90
        check_at = time.monotonic()
        while any(process.poll() is None for process in children.values()):
            for character, process in children.items():
                if process.poll() not in (None, 0):
                    _fail(f"QA bot for GUID {character} exited unsuccessfully")
            if time.monotonic() >= last_capture_scan:
                pruner.scan_and_prune()
                last_capture_scan = time.monotonic() + CAPTURE_SCAN_SECONDS
            if time.monotonic() - wave_started > timeout:
                _fail("four-client wave exceeded its bounded bot timeout")
            if time.monotonic() >= check_at:
                instance_soak.read_memory(pid, starttime)
                check_at = time.monotonic() + 10
            time.sleep(0.5)
        return_codes = {character: process.wait() for character, process in children.items()}
        if any(code != 0 for code in return_codes.values()):
            _fail("one or more QA bots failed")

        # Let the final packet metadata write settle before choosing the verifier window.
        stable_since = time.monotonic()
        prior_max = pruner.max_sequence
        while time.monotonic() - stable_since < PAIR_SETTLE_SECONDS:
            pruner.scan_and_prune()
            if pruner.max_sequence != prior_max:
                stable_since = time.monotonic()
                prior_max = pruner.max_sequence
            instance_soak.read_memory(pid, starttime)
            time.sleep(0.25)
        last = pruner.max_sequence
        if first > last:
            _fail("capture contains no packets for this four-client wave")
        movement = verify_four_client_movement.verify(capture, CHARACTERS, first, last)
        expected_pairs = {f"{actor}->{peer}" for actor in CHARACTERS for peer in CHARACTERS
                          if actor != peer}
        if set(movement["peer_deliveries"]) != expected_pairs:
            _fail("movement verifier did not cover all twelve peer streams")

        clients = []
        for character in CHARACTERS:
            plan = build_plan(character, wave, origin, active_seconds)
            last_move = next(action for action in reversed(plan["actions"])
                             if action["action"] == "movement")
            final_point = (last_move["x"], last_move["y"], last_move["z"])
            report_path = wave_dir / f"bot-{character}.json"
            clients.append(validate_bot_report(report_path, character, final_point))

        pruner.scan_and_prune()
        artifacts = pruner.artifacts(first, last)
        retained_size = sum(row["size"] for row in artifacts)
        if retained_size > pruner.max_bytes:
            _fail("selected four-client traces exceeded the configured disk bound")
        result = {
            "wave": wave,
            "preflight": preflight,
            "start_sequence": first,
            "end_sequence": last,
            "elapsed_s": round(time.monotonic() - wave_started, 3),
            "wave_seconds": wave_seconds,
            "active_plan_seconds": active_seconds,
            "characters": clients,
            "movement": movement,
            "retained_trace_count": len(artifacts),
            "retained_trace_bytes": retained_size,
            "trace_files": artifacts,
        }
        _write_private(wave_dir / "wave-report.json", result)
        return result
    except BaseException:
        _terminate_children(children)
        raise
    finally:
        for stream in log_streams:
            stream.close()


def _resolve_config(cli_path: str | None, env: dict[str, str]) -> Path:
    source = cli_path or env.get("RUSTYCORE_FOUR_CLIENT_CONFIG") or env.get("WOW_BOT_CONFIG")
    if not source:
        _fail("set --config or WOW_BOT_CONFIG to the prepared private bot config")
    path = Path(source)
    if not path.is_absolute():
        path = Path(__file__).resolve().parents[1] / "wow-test-bot" / path
    return _canonical_file(path, "prepared bot config")


def _parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--capture-dir", default=os.environ.get("RUSTYCORE_CAPTURE_DIR"),
                        help="fresh private Rust packet capture directory")
    parser.add_argument("--config", help="prepared private bot JSON config; defaults to WOW_BOT_CONFIG")
    parser.add_argument("--bot", default=os.environ.get("WOW_BOT_EXEC"),
                        help="prepared wow-test-bot binary; defaults to WOW_BOT_EXEC")
    parser.add_argument("--duration-seconds", type=int, default=7_200,
                        help="normal workload minimum (default: 7200; minimum: 7200)")
    parser.add_argument("--wave-seconds", type=int, default=300,
                        help="connected client wave duration (default: 300; range: 240..600)")
    parser.add_argument("--preflight", action="store_true",
                        help="run one 60-second startup wave; never reports full acceptance")
    parser.add_argument("--northshire-origin", type=float, nargs=3,
                        default=NORTHSHIRE_ORIGIN, metavar=("X", "Y", "Z"),
                        help="known Northshire position anchor (default: -8948 -80 88.78)")
    parser.add_argument("--capture-limit-mib", type=int, default=128,
                        help="maximum retained native packet bytes (default: 128 MiB)")
    parser.add_argument("--first-sequence", type=int,
                        default=os.environ.get("RUSTYCORE_CAPTURE_FIRST_SEQUENCE"),
                        help="first packet sequence owned by this workload; required when capture files already exist")
    return parser.parse_args(argv)


def run(cli: argparse.Namespace, env: dict[str, str] | None = None) -> dict[str, Any]:
    env = dict(os.environ if env is None else env)
    if not cli.capture_dir:
        _fail("runner is missing RUSTYCORE_CAPTURE_DIR")
    capture = _capture_directory(Path(cli.capture_dir))
    report_value = env.get("RUSTYCORE_CAPTURE_REPORT", str(capture / "scenario-report.json"))
    report_path = Path(report_value)
    if report_path != capture / "scenario-report.json" or report_path.exists() or report_path.is_symlink():
        _fail("scenario report must be a fresh scenario-report.json in the capture directory")
    if env.get("RUSTYCORE_CAPTURE_BACKEND", "rust") != "rust":
        _fail("four-client movement verification requires Rust native packet capture")
    if env.get("WORLD_PORT", str(WORLD_PORT)) != str(WORLD_PORT) \
            or env.get("INSTANCE_PORT", str(INSTANCE_PORT)) != str(INSTANCE_PORT):
        _fail("runtime capture must target loopback world/instance ports 14885 and 14886")

    config_path = _resolve_config(cli.config, env)
    configured = _configured_bots(config_path, env)
    bot_value = cli.bot or env.get("WOW_BOT_EXEC")
    if not bot_value:
        _fail("set --bot or WOW_BOT_EXEC to the already prepared QA bot binary")
    bot_path = _canonical_file(Path(bot_value), "QA bot executable")
    if not os.access(bot_path, os.X_OK):
        _fail("QA bot is not executable")
    bot_sha = _sha256(bot_path)
    expected_bot_sha = env.get("WOW_BOT_EXEC_SHA256", "").lower()
    if expected_bot_sha and (not re.fullmatch(r"[0-9a-f]{64}", expected_bot_sha)
                             or expected_bot_sha != bot_sha):
        _fail("QA bot binary differs from its SHA-256 pin")
    candidate_sha = env.get("RUSTYCORE_CAPTURE_CANDIDATE_SHA256", "")
    if not re.fullmatch(r"[0-9a-fA-F]{64}", candidate_sha):
        _fail("runtime capture candidate SHA-256 is missing or invalid")
    pid, starttime = _process_identity_from_env(env)

    if cli.capture_limit_mib <= 0 or cli.wave_seconds < 240 or cli.wave_seconds > 600:
        _fail("capture limit must be positive and wave duration must be 240..600 seconds")
    if cli.preflight:
        wave_seconds = 60
        active_seconds = wave_seconds - 10
        required_duration = wave_seconds
        wave_limit = 1
    else:
        if cli.duration_seconds < 7_200:
            _fail("normal stability acceptance must run at least 7200 seconds")
        wave_seconds = cli.wave_seconds
        active_seconds = wave_seconds - 10
        required_duration = cli.duration_seconds
        wave_limit = None
    origin = tuple(float(value) for value in cli.northshire_origin)
    if not all(math.isfinite(value) for value in origin):
        _fail("Northshire origin coordinates must be finite")

    run_dir = capture / "four-client-stability"
    try:
        run_dir.mkdir(mode=0o700)
    except FileExistsError:
        _fail("four-client output directory already exists")
    pruner = CapturePruner(capture, cli.capture_limit_mib * 1024 * 1024,
                           cli.first_sequence)
    server_log = env.get("RUSTYCORE_CAPTURE_SERVER_LOG")
    queue_sampler = (ProcessStdoutQueueSampler(Path(server_log), pid, starttime)
                     if server_log else None)

    def queue_sample() -> dict[str, Any]:
        return queue_sampler.sample() if queue_sampler else instance_soak.queue_sample(pid, starttime)

    waves: list[dict[str, Any]] = []
    started = time.monotonic()
    campaign_started: float | None = None
    stage = "queue-before"
    baseline: dict[str, Any] | None = None
    end: dict[str, Any] | None = None
    queue_samples: list[dict[str, Any]] = []
    next_capture_scan = 0.0
    signal_before = signal.getsignal(signal.SIGTERM)

    def interrupted(_signum: int, _frame: Any) -> None:
        raise InterruptedError("four-client stability run interrupted")

    signal.signal(signal.SIGTERM, interrupted)
    try:
        with instance_soak.memory_samples(run_dir, pid, starttime) as check_samples:
            baseline = queue_sample()
            _write_private(run_dir / "queue-before.json", baseline)
            queue_samples.append({"wave": 0, **baseline})
            campaign_started = time.monotonic()
            wave = 0
            while wave_limit is None or wave < wave_limit:
                if wave and time.monotonic() - campaign_started >= required_duration:
                    break
                wave += 1
                stage = f"wave-{wave:03d}"
                wave_started = time.monotonic()
                result = run_wave(
                    wave=wave, active_seconds=active_seconds, wave_seconds=wave_seconds,
                    capture=capture, run_dir=run_dir, config_path=config_path,
                    bot_path=bot_path, configured=configured, base_env=env,
                    origin=origin, pruner=pruner, pid=pid, starttime=starttime,
                    preflight=cli.preflight,
                )
                if waves:
                    result["reloaded_positions"] = verify_four_client_movement.verify_relogin_positions(
                        capture, result["start_sequence"], result["end_sequence"],
                        {client["character_guid"]: client["movement_position"]
                         for client in waves[-1]["characters"]},
                    )
                end = queue_sample()
                _write_private(run_dir / f"queue-after-wave-{wave:03d}.json", end)
                queue_samples.append({"wave": wave, **end})
                result["command_queue_counters"] = {name: end[name] for name in QUEUE_FIELDS}
                waves.append(result)
                _write_private(run_dir / f"wave-{wave:03d}-summary.json", {
                    "wave": wave, "start_elapsed_s": round(wave_started - started, 3),
                    "result": result,
                })
                check_samples()
                target_end = wave_started + wave_seconds
                while time.monotonic() < target_end:
                    if time.monotonic() >= next_capture_scan:
                        pruner.scan_and_prune()
                        next_capture_scan = time.monotonic() + CAPTURE_SCAN_SECONDS
                    check_samples()
                    time.sleep(min(1.0, max(0.0, target_end - time.monotonic())))
            stage = "queue-after"
            check_samples()
            if end is None or baseline is None:
                _fail("four-client run completed without queue counter samples")
            _write_private(run_dir / "queue-after.json", end)
        deltas = {name: end[name] - baseline[name] for name in QUEUE_FIELDS}
        if any(delta < 0 for delta in deltas.values()):
            _fail("server process queue counters moved backwards")
    except BaseException as error:
        _write_private(run_dir / "failure.json", {
            "stage": stage, "completed_waves": len(waves),
            "error_type": type(error).__name__, "elapsed_s": round(time.monotonic() - started, 3),
        })
        raise
    finally:
        signal.signal(signal.SIGTERM, signal_before)
        if queue_sampler:
            queue_sampler.close()

    elapsed = time.monotonic() - started
    report = {
        "scenario": PREFLIGHT_SCENARIO if cli.preflight else SCENARIO,
        "scenario_observation": "preflight_passed" if cli.preflight else "passed",
        "preflight_only": cli.preflight,
        "required_duration_seconds": required_duration,
        "elapsed_seconds": round(elapsed, 3),
        "wave_count": len(waves),
        "wave_seconds": wave_seconds,
        "characters": list(CHARACTERS),
        "server_pid": pid,
        "server_starttime_ticks": starttime,
        "candidate_server_sha256": candidate_sha.lower(),
        "bot_sha256": bot_sha,
        "endpoints": {"bnet": f"{BNET_HOST}:{BNET_PORT}",
                      "world": f"{WORLD_HOST}:{WORLD_PORT}",
                      "instance": f"{INSTANCE_HOST}:{INSTANCE_PORT}"},
        "command_queue_deltas": deltas,
        "command_queue_samples": queue_samples,
        "queue_sample_interval": "baseline and after every completed wave",
        "queue_source": "captured_server_stdout_fd1" if queue_sampler else "systemd_journal",
        "queue_before": "four-client-stability/queue-before.json",
        "queue_after": "four-client-stability/queue-after.json",
        "memory_samples": "four-client-stability/memory.jsonl",
        "workload": waves,
        "parity_assessed": False,
    }
    _write_private(report_path, report)
    return report


def main(argv: list[str] | None = None) -> int:
    os.umask(0o077)
    cli = _parse_args(argv)
    try:
        report = run(cli)
    except (ScenarioFailure, OSError, ValueError, KeyError, json.JSONDecodeError,
            subprocess.SubprocessError, InterruptedError, KeyboardInterrupt) as error:
        capture_value = os.environ.get("RUSTYCORE_CAPTURE_DIR")
        if capture_value:
            capture = Path(capture_value)
            report_path = capture / "scenario-report.json"
            if (capture.is_dir() and not capture.is_symlink()
                    and not report_path.exists() and not report_path.is_symlink()):
                try:
                    _write_private(report_path, {
                        "scenario": PREFLIGHT_SCENARIO if cli.preflight else SCENARIO,
                        "scenario_observation": "preflight_failed" if cli.preflight else "failed",
                        "error_type": type(error).__name__,
                        "error": str(error),
                        "parity_assessed": False,
                    })
                except OSError:
                    pass
        print(f"four-client stability failed: {error}", file=sys.stderr)
        return 130 if isinstance(error, (InterruptedError, KeyboardInterrupt)) else 1
    print(json.dumps({key: report[key] for key in (
        "scenario", "scenario_observation", "wave_count", "elapsed_seconds",
        "server_pid", "server_starttime_ticks", "command_queue_deltas",
    )}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
