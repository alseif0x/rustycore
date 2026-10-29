#!/usr/bin/env python3
"""One guarded death-lifecycle scenario pass, driven by the outer capture wrapper."""

from __future__ import annotations

import hashlib
import json
import math
import os
from pathlib import Path
import re
import subprocess
import threading
import time
from typing import Any

import death_fixture
from death_corpse_target import select_offer_corpse


SCENARIOS = {
    "fall-release": 1,
    "reclaim": 1,
    "healer": 2,
    "wisp": 2,
    "reincarnation": 1,
    "mob": 1,
    "automatic-release": 1,
    "offer": 2,
    "recovery-dead": 2,
    "recovery-ghost": 2,
    "stuck-death": 2,
    "stuck-home": 1,
}
LOGOUT_COMPLETE = 0x2684
PRE_RESURRECT = 0x276B
DURABILITY_DAMAGE_DEATH = 0x2745
MOVE_ROOT = 0x2DF9
MOVE_TELEPORT = 0x2E04
MOVE_TELEPORT_ACK = 0x39FA
DEATH_RELEASE_LOC = 0x26D3
SPELL_GO = 0x2C36
COOLDOWN_EVENT = 0x26B9
RESURRECT_REQUEST = 0x257E
ATTACKER_STATE_UPDATE = 0x2952
AURA_UPDATE = 0x2C1F
UPDATE_OBJECT = 0x27CB
PLAYER_GHOST = 0x10
READY_MARKER = "CAST_LIFECYCLE_READY"
# Native socket loss retains the session for 60s (WorldSession.cpp:137,514-537),
# then LogoutPlayer still needs bounded time to release, save and go offline.
OFFLINE_SECONDS = 90
ACTIVE_BOTS: list[BotPass] = []


class ScenarioFailure(RuntimeError):
    """A report-safe scenario failure with a fixed, nonsecret message."""


class BotPass:
    def __init__(self, actor: dict[str, Any], plan: dict[str, Any], label: str):
        self.actor = actor
        self.plan = plan
        self.label = label
        self.plan_path: Path | None = None
        self.report_path: Path | None = None
        self.log_path: Path | None = None
        self.process: subprocess.Popen[str] | None = None
        self.ready = threading.Event()
        self.reader: threading.Thread | None = None
        self.report: dict[str, Any] | None = None
        self.report_hash: str | None = None


def packed_guid(low: int, high: int) -> bytes:
    """Match the bot's lowmask, highmask, low bytes, high bytes encoding."""
    low_bytes = low.to_bytes(8, "little")
    high_bytes = high.to_bytes(8, "little")
    low_mask = sum(1 << i for i, byte in enumerate(low_bytes) if byte)
    high_mask = sum(1 << i for i, byte in enumerate(high_bytes) if byte)
    return bytes((low_mask, high_mask)) + bytes(byte for byte in low_bytes if byte) + bytes(
        byte for byte in high_bytes if byte
    )


def player_guid(actor: dict[str, Any], realm_id: int) -> dict[str, int]:
    if not 1 <= realm_id <= 0x1FFF or not 1 <= int(actor["guid"]) < (1 << 40):
        raise ScenarioFailure("invalid journal player GUID or realm ID")
    return {"low": int(actor["guid"]), "high": (2 << 58) | ((realm_id & 0x1FFF) << 42)}


def _packet(connection: str, opcode: int, body: bytes = b"") -> dict[str, Any]:
    return {"event": "packet", "connection": connection, "opcode": opcode,
            "body_hex": body.hex()}


def _wait(connection: str, opcode: int, timeout_ms: int) -> dict[str, Any]:
    return {"action": "wait_for_packet", "connection": connection,
            "opcode": opcode, "timeout_ms": timeout_ms}


def _fall_actions(actor: dict[str, Any], *, release: bool) -> list[dict[str, Any]]:
    home = actor["home"]
    start = [home[0], home[1], home[2] + 100.0]
    actions: list[dict[str, Any]] = [
        {"action": "movement", "kind": "heartbeat", "time_ms": 1,
         "x": start[0], "y": start[1], "z": start[2], "orientation": 0.0,
         "fall": {"fall_time_ms": 1, "z_speed": 0.0, "sin_angle": 0.0,
                  "cos_angle": 1.0, "xy_speed": 0.0}},
        {"action": "wait", "milliseconds": 3500},
        {"action": "movement", "kind": "fall_land", "time_ms": 3501,
         "x": home[0], "y": home[1], "z": home[2], "orientation": 0.0,
         "fall": {"fall_time_ms": 3500, "z_speed": -7.0, "sin_angle": 0.0,
                  "cos_angle": 1.0, "xy_speed": 0.0}},
        _wait("instance", MOVE_ROOT, 30_000),
    ]
    if release:
        actions.extend([
            {"action": "release_spirit", "check_instance": False},
            {"action": "wait", "milliseconds": 2000},
        ])
    return actions


def plan_for(scenario: str, epoch: int, actor: dict[str, Any], realm_id: int,
             durability_percent: int,
             actors: dict[str, dict[str, Any]] | None = None) -> dict[str, Any]:
    """Build one plain existing cast-lifecycle plan; there is no plan DSL."""
    if scenario not in SCENARIOS or not 1 <= epoch <= SCENARIOS[scenario]:
        raise ScenarioFailure("scenario epoch is outside the bounded case set")
    guid = player_guid(actor, realm_id)
    actions: list[dict[str, Any]] = []
    expected: list[dict[str, Any]] = []
    timeout_ms = 10_000
    finish = "logout"
    observe_only = False
    death = scenario in {"fall-release", "reclaim", "healer", "wisp", "reincarnation",
                         "automatic-release", "offer", "recovery-dead", "recovery-ghost"}
    # Player.cpp sends the configured death percentage even if no equipment is present.
    if death and epoch == 1 and durability_percent == 10:
        expected.append(_packet("realm", DURABILITY_DAMAGE_DEATH,
                                (10).to_bytes(4, "little", signed=True)))
    expects_pre_res = scenario in {"fall-release", "reclaim", "healer", "wisp",
                                   "automatic-release", "recovery-ghost"} and epoch == 1
    if expects_pre_res:
        expected.append(_packet("realm", PRE_RESURRECT,
                                packed_guid(guid["low"], guid["high"])))

    if scenario == "fall-release":
        actions = _fall_actions(actor, release=True) + [
            {"action": "release_spirit", "check_instance": False},
        ]
        timeout_ms = 10_000
    elif scenario == "reclaim":
        actions = _fall_actions(actor, release=True) + [
            {"action": "reclaim_corpse", "corpse": {"low": 0, "high": 0}},
            {"action": "wait", "milliseconds": 35_000},
            {"action": "reclaim_corpse", "corpse": guid},
            {"action": "reclaim_corpse", "corpse": guid},
        ]
        timeout_ms = 45_000
    elif scenario == "healer":
        if epoch == 1:
            actions = _fall_actions(actor, release=True) + [
                {"action": "spirit_healer_activate", "healer": {
                    "entry": 6491, "map_id": 0, "near_xyz": [-9340.7, 165.021, 61.6412],
                    "radius": 40.0}},
            ]
            timeout_ms = 10_000
    elif scenario == "wisp":
        if epoch == 1:
            actions = _fall_actions(actor, release=True)
            timeout_ms = 10_000
        else:
            actions = [{"action": "spirit_healer_activate", "healer": {
                "entry": 6491, "map_id": 0, "near_xyz": [-9340.7, 165.021, 61.6412],
                "radius": 40.0}}]
            timeout_ms = 5_000
    elif scenario == "reincarnation":
        actions = _fall_actions(actor, release=False) + [
            {"action": "self_res", "spell_id": 21169},
            {"action": "wait", "milliseconds": 2000},
            {"action": "self_res", "spell_id": 21169},
        ]
        timeout_ms = 10_000
    elif scenario == "mob":
        actions = [
            {"action": "attack_swing", "target": {"entry": 6, "map_id": 0,
             "near_xyz": [-8783.05, -161.565, 82.0352], "radius": 40.0}},
            {"action": "wait", "milliseconds": 30_000},
        ]
        timeout_ms = 30_000
    elif scenario == "automatic-release":
        actions = _fall_actions(actor, release=False) + [_wait("realm", PRE_RESURRECT, 370_000)]
        timeout_ms = 390_000
    elif scenario == "offer":
        if epoch == 1:
            actions = _fall_actions(actor, release=False)
            timeout_ms = 10_000
        else:
            if actors is None or "Qadeathcast" not in actors or "Qadeathmob" not in actors:
                raise ScenarioFailure("offer plan requires validated journal actor bindings")
            cast = actors["Qadeathcast"]
            wrong = actors["Qadeathmob"]
            caster_guid = player_guid(cast, realm_id)
            wrong_guid = player_guid(wrong, realm_id)
            actions = [
                _wait("realm", RESURRECT_REQUEST, 40_000),
                {"action": "resurrect_response", "offerer": wrong_guid, "response": 0},
                {"action": "resurrect_response", "offerer": wrong_guid, "response": 1},
                _wait("realm", RESURRECT_REQUEST, 90_000),
                {"action": "resurrect_response", "offerer": caster_guid, "response": 0},
                {"action": "resurrect_response", "offerer": caster_guid, "response": 0},
                {"action": "wait", "milliseconds": 3000},
            ]
            timeout_ms = 105_000
    elif scenario in {"recovery-dead", "recovery-ghost"}:
        if epoch == 1:
            actions = _fall_actions(actor, release=(scenario == "recovery-ghost"))
            finish = "disconnect"
            timeout_ms = 10_000
        else:
            expected = []
            actions = []
            timeout_ms = 3_000
    elif scenario == "stuck-death":
        self_guid = player_guid(actor, realm_id)
        if epoch == 1:
            cast_hearthstone = {"low": 8690001, "high": 0}
            cast_stuck = {"low": 7355001, "high": 0}
            actions = [
                {"action": "cast", "spell_id": 8690, "cast_id": cast_hearthstone,
                 "target": self_guid},
                _wait("instance", MOVE_TELEPORT, 30_000),
                {"action": "wait", "milliseconds": 2000},
                {"action": "cast", "spell_id": 7355, "cast_id": cast_stuck,
                 "target": self_guid},
                _wait("instance", MOVE_ROOT, 30_000),
            ]
            expected.extend([
                # Native InitExplicitTargets removes the supplied self target;
                # Hearthstone Go retains UNIT|DEST_LOCATION with an empty Unit.
                {"event": "prepare", "client_cast_id": cast_hearthstone},
                {"event": "start", "client_cast_id": cast_hearthstone, "spell_id": 8690,
                 "target": "empty"},
                {"event": "go", "client_cast_id": cast_hearthstone, "spell_id": 8690,
                 "target": {"unit": {"low": 0, "high": 0}}},
                {"event": "prepare", "client_cast_id": cast_stuck},
                {"event": "start", "client_cast_id": cast_stuck, "spell_id": 7355,
                 "target": "empty"},
                {"event": "go", "client_cast_id": cast_stuck, "spell_id": 7355,
                 "target": "empty"},
            ])
            timeout_ms = 45_000
        else:
            cast_stuck = {"low": 7355002, "high": 0}
            actions = [
                {"action": "cast", "spell_id": 7355, "cast_id": cast_stuck,
                 "target": self_guid},
                {"action": "wait", "milliseconds": 3000},
            ]
            expected.extend([
                {"event": "prepare", "client_cast_id": cast_stuck},
                {"event": "start", "client_cast_id": cast_stuck,
                 "spell_id": 7355, "target": "empty"},
                {"event": "go", "client_cast_id": cast_stuck,
                 "spell_id": 7355, "target": "empty"},
            ])
            timeout_ms = 15_000
    elif scenario == "stuck-home":
        self_guid = player_guid(actor, realm_id)
        cast_stuck = {"low": 7355003, "high": 0}
        actions = [
            {"action": "cast", "spell_id": 7355, "cast_id": cast_stuck,
             "target": self_guid},
            _wait("instance", MOVE_TELEPORT, 30_000),
        ]
        expected.extend([
            {"event": "prepare", "client_cast_id": cast_stuck},
            {"event": "start", "client_cast_id": cast_stuck,
             "spell_id": 7355, "target": "empty"},
            {"event": "go", "client_cast_id": cast_stuck,
             "spell_id": 7355, "target": "empty"},
        ])
        timeout_ms = 40_000
    if scenario == "automatic-release":
        actions = _fall_actions(actor, release=False) + [_wait("realm", PRE_RESURRECT, 370_000)]
    if scenario == "healer" and epoch == 2:
        actions = []
        timeout_ms = 3_000
    if finish == "logout":
        expected.append(_packet("realm", LOGOUT_COMPLETE))
    return {"character_guid": int(actor["guid"]), "observe_only": observe_only,
            "timeout_ms": timeout_ms, "actions": actions, "expect": expected,
            "finish_policy": finish}


def observer_plan(timeout_ms: int, character_guid: int) -> dict[str, Any]:
    return {"character_guid": character_guid, "observe_only": True,
            "timeout_ms": timeout_ms, "actions": [], "expect": [],
            "finish_policy": "logout"}


def wisp_expiry_plan(actor: dict[str, Any]) -> dict[str, Any]:
    return {"character_guid": int(actor["guid"]), "timeout_ms": 78_000,
            "actions": [{"action": "wait", "milliseconds": 75_000}],
            "expect": [_packet("realm", LOGOUT_COMPLETE)], "finish_policy": "logout"}


def _snapshot_path(output: Path, scenario: str, epoch: int,
                   actor: dict[str, Any], label: str) -> Path:
    return output / f"epoch-{epoch:03d}-{scenario}-{actor['name']}-{label}-snapshot.json"


def _has_ready_marker(line: str) -> bool:
    return READY_MARKER in line


def _observer_exit_allowed(status: int) -> bool:
    return status in {0, 1}


def _write_private(path: Path, value: Any) -> None:
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    fd = os.open(path, flags, 0o600)
    with os.fdopen(fd, "w", encoding="utf-8") as stream:
        json.dump(value, stream, sort_keys=True, indent=2)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _read_world_setting(path: Path, key: str) -> str | None:
    found: str | None = None
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.lstrip()
        if not stripped or stripped.startswith(("#", ";")) or "=" not in line:
            continue
        name, value = line.split("=", 1)
        if name.strip() == key:
            value = value.split("#", 1)[0].split(";", 1)[0].strip().strip("\"'")
            found = value
    return found


def _runtime_identity() -> dict[str, Any]:
    required = ("RUSTYCORE_DEATH_JOURNAL", "RUSTYCORE_DEATH_EPOCH",
                "RUSTYCORE_DEATH_EPOCH_INDEX", "RUSTYCORE_DEATH_BOT_CONFIG",
                "RUSTYCORE_DEATH_WORLD_CONFIG", "RUSTYCORE_DEATH_SCENARIO",
                "RUSTYCORE_CAPTURE_DIR", "RUSTYCORE_CAPTURE_REPORT",
                "RUSTYCORE_CAPTURE_BACKEND", "WOW_BOT_EXEC")
    if any(not os.environ.get(key) for key in required):
        raise ScenarioFailure("required guarded scenario environment is incomplete")
    scenario = os.environ["RUSTYCORE_DEATH_SCENARIO"]
    if scenario not in SCENARIOS:
        raise ScenarioFailure("scenario name is not in the bounded case set")
    try:
        epoch = int(os.environ["RUSTYCORE_DEATH_EPOCH_INDEX"])
    except ValueError:
        raise ScenarioFailure("epoch index is invalid") from None
    if not 1 <= epoch <= SCENARIOS[scenario]:
        raise ScenarioFailure("epoch index does not match the selected scenario")
    if os.environ["RUSTYCORE_DEATH_EPOCH"] != f"epoch-{epoch:03d}":
        raise ScenarioFailure("epoch name and index do not agree")
    capture = Path(os.environ["RUSTYCORE_CAPTURE_DIR"])
    if (not capture.is_absolute() or capture.is_symlink() or not capture.is_dir()
            or capture.resolve(strict=True) != capture or capture.stat().st_mode & 0o777 != 0o700):
        raise ScenarioFailure("capture directory is not canonical and private")
    report = Path(os.environ["RUSTYCORE_CAPTURE_REPORT"])
    if report != capture / "scenario-report.json" or report.exists() or report.is_symlink():
        raise ScenarioFailure("scenario report path is not fresh inside the capture directory")
    for name in ("RUSTYCORE_DEATH_JOURNAL", "RUSTYCORE_DEATH_BOT_CONFIG",
                 "RUSTYCORE_DEATH_WORLD_CONFIG", "WOW_BOT_EXEC"):
        path = Path(os.environ[name])
        if (not path.is_absolute() or path.is_symlink() or not path.exists()
                or path.resolve(strict=True) != path):
            raise ScenarioFailure("scenario input path is not canonical")
    if not Path(os.environ["WOW_BOT_EXEC"]).is_file() or not os.access(os.environ["WOW_BOT_EXEC"], os.X_OK):
        raise ScenarioFailure("candidate bot is not an executable file")
    candidate_sha = os.environ.get("RUSTYCORE_CAPTURE_CANDIDATE_SHA256", "")
    bot_sha = os.environ.get("WOW_BOT_EXEC_SHA256", "")
    if not re.fullmatch(r"[0-9a-fA-F]{64}", candidate_sha):
        raise ScenarioFailure("capture candidate SHA-256 is missing or invalid")
    if not re.fullmatch(r"[0-9a-fA-F]{64}", bot_sha) or \
            _sha256(Path(os.environ["WOW_BOT_EXEC"])) != bot_sha.lower():
        raise ScenarioFailure("candidate bot executable differs from its capture hash pin")
    if os.environ["RUSTYCORE_CAPTURE_BACKEND"] not in {"rust", "cpp"}:
        raise ScenarioFailure("capture backend is invalid")
    return {"scenario": scenario, "epoch": epoch, "capture": capture,
            "report_path": report, "journal_path": Path(os.environ["RUSTYCORE_DEATH_JOURNAL"]),
            "bot_config": Path(os.environ["RUSTYCORE_DEATH_BOT_CONFIG"]),
            "world_config": Path(os.environ["RUSTYCORE_DEATH_WORLD_CONFIG"]),
            "bot_exec": Path(os.environ["WOW_BOT_EXEC"])}


def _read_report(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        raise ScenarioFailure("bot report is missing or malformed") from None
    if not isinstance(value, dict) or not isinstance(value.get("results"), list):
        raise ScenarioFailure("bot report has an unexpected structure")
    return value


def _result_for(report: dict[str, Any], actor: dict[str, Any], *, observer: bool = False) -> dict[str, Any]:
    matches = [result for result in report["results"]
               if isinstance(result, dict) and result.get("character_guid") == actor["guid"]]
    if len(matches) != 1:
        raise ScenarioFailure("bot report actor GUID does not match the fixture journal")
    result = matches[0]
    evidence = result.get("cast_lifecycle")
    if not isinstance(evidence, dict) or evidence.get("failure") is not None:
        raise ScenarioFailure("bot lifecycle report is missing or failed")
    if observer:
        if (evidence.get("observe_only") is not True or evidence.get("passed") is not False
                or evidence.get("finish_completed") is not True):
            raise ScenarioFailure("observer collection did not finish as observe-only evidence")
    else:
        if evidence.get("passed") is not True:
            raise ScenarioFailure("bot lifecycle plan or exact expectations did not pass")
        facts = evidence.get("expected_rules")
        if not isinstance(facts, list) or not facts or any(fact.get("matched") is not True for fact in facts):
            raise ScenarioFailure("bot lifecycle exact expectations are incomplete")
    return evidence


def _read_stream(pass_: BotPass) -> None:
    assert pass_.process is not None and pass_.process.stdout is not None and pass_.log_path is not None
    try:
        with pass_.log_path.open("x", encoding="utf-8") as log:
            os.chmod(pass_.log_path, 0o600)
            for line in pass_.process.stdout:
                log.write(line)
                log.flush()
                if _has_ready_marker(line):
                    pass_.ready.set()
    finally:
        pass_.process.stdout.close()


def start_bot(pass_: BotPass, *, bot_exec: Path, bot_config: Path, output: Path,
              readiness_seconds: int = 180) -> None:
    pass_.plan_path = output / f"{pass_.label}.plan.json"
    pass_.report_path = output / f"{pass_.label}.bot-report.json"
    pass_.log_path = output / f"{pass_.label}.bot.log"
    _write_private(pass_.plan_path, pass_.plan)
    account = next((item["email"] for item in death_fixture.ACCOUNTS
                    if item["account_id"] == pass_.actor["account_id"]), None)
    if account is None:
        raise ScenarioFailure("journal actor has no reviewed account binding")
    timeout_secs = max(30, math.ceil(pass_.plan["timeout_ms"] / 1000) + 30)
    command = [str(bot_exec), "--config", str(bot_config), "--single", account,
               "--cast-lifecycle-plan", str(pass_.plan_path), "--report",
               str(pass_.report_path), "--timeout", str(timeout_secs)]
    try:
        pass_.process = subprocess.Popen(command, cwd=str(bot_config.parent), text=True,
                                         stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                         bufsize=1, start_new_session=True)
    except Exception:
        raise ScenarioFailure("candidate bot could not be started") from None
    pass_.reader = threading.Thread(target=_read_stream, args=(pass_,), daemon=True)
    pass_.reader.start()
    ACTIVE_BOTS.append(pass_)
    deadline = time.monotonic() + readiness_seconds
    while time.monotonic() < deadline:
        if pass_.ready.wait(0.2):
            if pass_.process.poll() is not None:
                raise ScenarioFailure("candidate bot exited at the login readiness boundary")
            return
        if pass_.process.poll() is not None:
            raise ScenarioFailure("candidate bot exited before the login readiness boundary")
    raise ScenarioFailure("candidate bot did not reach CAST_LIFECYCLE_READY before the deadline")


def _stop_owned(pass_: BotPass) -> None:
    process = pass_.process
    if process is None or process.poll() is not None:
        return
    process.terminate()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=5)


def finish_bot(pass_: BotPass, *, timeout_seconds: int, actor: dict[str, Any],
               observer: bool = False) -> dict[str, Any]:
    assert pass_.process is not None and pass_.report_path is not None and pass_.reader is not None
    try:
        status = pass_.process.wait(timeout=timeout_seconds)
    except subprocess.TimeoutExpired:
        _stop_owned(pass_)
        raise ScenarioFailure("candidate bot exceeded its bounded pass deadline") from None
    pass_.reader.join(timeout=5)
    if pass_ in ACTIVE_BOTS:
        ACTIVE_BOTS.remove(pass_)
    if pass_.reader.is_alive() or (status != 0 and not (observer and _observer_exit_allowed(status))):
        raise ScenarioFailure("candidate bot failed or did not close its private log")
    if pass_.report_path.is_symlink() or not pass_.report_path.is_file():
        raise ScenarioFailure("candidate bot report is missing or unsafe")
    pass_.report_hash = _sha256(pass_.report_path)
    pass_.report = _read_report(pass_.report_path)
    return _result_for(pass_.report, actor, observer=observer)


def _actor_snapshot(snapshot: dict[str, Any], actor: dict[str, Any]) -> dict[str, Any]:
    characters = snapshot.get("characters", {})
    columns = [str(name).casefold() for name in characters.get("columns", [])]
    rows = characters.get("rows", [])
    guid_index = columns.index("guid") if "guid" in columns else -1
    if guid_index < 0:
        raise ScenarioFailure("database snapshot has no character GUID column")
    matches = [row for row in rows if len(row) == len(columns) and row[guid_index] == str(actor["guid"])]
    if len(matches) != 1:
        raise ScenarioFailure("database snapshot does not contain exactly one journal actor")
    return {column: matches[0][index] for index, column in enumerate(columns)}


def validate_state(scenario: str, epoch: int, actor: dict[str, Any],
                   snapshot: dict[str, Any]) -> dict[str, Any]:
    row = _actor_snapshot(snapshot, actor)
    try:
        health = int(row["health"])
        flags = int(row["playerflags"])
    except (KeyError, ValueError):
        raise ScenarioFailure("database character snapshot lacks health or player flags") from None
    corpse_data = snapshot.get("corpse", {})
    corpse_columns = [str(name).casefold() for name in corpse_data.get("columns", [])]
    owner_index = corpse_columns.index("guid") if "guid" in corpse_columns else -1
    corpse_rows = corpse_data.get("rows", [])
    owned_corpses = [item for item in corpse_rows
                     if owner_index >= 0 and len(item) == len(corpse_columns)
                     and item[owner_index] == str(actor["guid"])]
    # WorldSession.cpp:558-565 releases a running death timer during LogoutPlayer,
    # including disconnect cleanup, before the offline character save.
    ghost_required = scenario in {"fall-release", "automatic-release", "recovery-ghost",
                                   "mob", "recovery-dead", "stuck-death"} or (
        scenario in {"wisp", "offer"} and epoch == 1)
    alive_required = scenario in {"reclaim", "healer", "reincarnation", "stuck-home"} or (
        (scenario == "wisp" or scenario == "offer") and epoch == 2)
    if ghost_required and (health != 1 or not flags & PLAYER_GHOST or len(owned_corpses) != 1):
        raise ScenarioFailure("released ghost must persist health one, the ghost flag, and one owned corpse")
    if alive_required and (health <= 0 or flags & PLAYER_GHOST or owned_corpses):
        raise ScenarioFailure("required alive state or corpse cleanup was not persisted")
    return {"guid": actor["guid"], "health": health, "player_flags": flags,
            "ghost": bool(flags & PLAYER_GHOST), "owned_corpse_count": len(owned_corpses),
            "position": {key: row.get(key) for key in
                         ("map", "position_x", "position_y", "position_z", "orientation")}}


def _poll_actor_offline(db: death_fixture.MariaDB, journal: dict[str, Any],
                        actor: dict[str, Any]) -> None:
    deadline = time.monotonic() + OFFLINE_SECONDS
    while time.monotonic() < deadline:
        rows = db.character_identities(journal)
        matches = [row for row in rows if len(row) == 4 and row[0] == str(actor["guid"])
                   and row[1] == str(actor["account_id"]) and row[2] == actor["name"]]
        if len(matches) == 1 and matches[0][3] == "0":
            return
        time.sleep(1)
    raise ScenarioFailure("character did not reach verified offline state before snapshot")


def _save_snapshot(db: death_fixture.MariaDB, journal: dict[str, Any], actor: dict[str, Any],
                   scenario: str, epoch: int, output: Path,
                   label: str) -> tuple[dict[str, Any], dict[str, Any]]:
    _poll_actor_offline(db, journal, actor)
    snapshot = db.snapshot(journal)
    path = _snapshot_path(output, scenario, epoch, actor, label)
    _write_private(path, snapshot)
    return snapshot, validate_state(scenario, epoch, actor, snapshot)


def _events(evidence: dict[str, Any], opcode: int) -> list[dict[str, Any]]:
    return [event for event in evidence.get("events", [])
            if isinstance(event, dict) and event.get("opcode") == opcode]


def _validate_death_shutdown_evidence(scenario: str, epoch: int, plan: dict[str, Any],
                                      evidence: dict[str, Any], observation: dict[str, Any],
                                      reload_input: dict[str, Any] | None = None) -> None:
    new_death = epoch == 1 and scenario in {"mob", "offer", "recovery-dead", "stuck-death"}
    reload_ghost = epoch == 2 and scenario in {"recovery-dead", "stuck-death"}
    if not new_death and not reload_ghost:
        return
    trace = evidence.get("trace", [])
    if (any(action.get("action") == "release_spirit" for action in plan.get("actions", []))
            or any(packet.get("action") == "release_spirit"
                   for packet in evidence.get("sent_packets", []))
            or any(item.get("action") == "release_spirit" for item in trace)):
        raise ScenarioFailure("unreleased-death plan sent or scheduled an explicit manual release")
    finish = plan.get("finish_policy")
    starts = [item for item in trace if item.get("kind") == "action_started"
              and item.get("action") == f"finish_{finish}"
              and type(item.get("sequence")) is int and item["sequence"] >= 0]
    if finish not in {"logout", "disconnect"} or len(starts) != 1:
        raise ScenarioFailure("death plan lacks its exact native shutdown boundary")
    boundary = starts[0]["sequence"]
    packets = [item for item in trace
               if item.get("kind") in {"packet_observed", "packet_redacted"}
               and type(item.get("sequence")) is int and item["sequence"] >= 0]
    pre_res = [item["sequence"] for item in packets if item.get("opcode") == PRE_RESURRECT]
    if new_death:
        for event in _events(evidence, PRE_RESURRECT):
            if not any(item.get("opcode") == PRE_RESURRECT
                       and item.get("event_sequence") == event.get("sequence")
                       and event.get("sequence") is not None for item in packets):
                raise ScenarioFailure("PreResurrect evidence lacks its ordered packet trace")
        roots = [item["sequence"] for item in packets if item.get("opcode") == MOVE_ROOT
                 and item.get("connection") == "instance" and item["sequence"] < boundary]
        if not roots:
            raise ScenarioFailure("native death/root was not observed before finish began")
        if any(sequence < boundary for sequence in pre_res):
            raise ScenarioFailure("unreleased death received PreResurrect before finish began")
        observation["pre_shutdown"] = {
            "phase": "PreShutdown", "state": "unreleased_death", "proof": "wire",
            "root_trace_sequences": roots, "finish_started_trace_sequence": boundary,
            "manual_release_sent": False, "pre_resurrect_before_finish": False,
        }
    else:
        state = (reload_input or {}).get("state", {})
        if (state.get("guid") != plan.get("character_guid") or state.get("health") != 1
                or state.get("ghost") is not True or state.get("owned_corpse_count") != 1):
            raise ScenarioFailure("reload epoch lacks its owned persisted ghost/corpse input")
        observation["pre_shutdown"] = {
            "phase": "PreShutdown", "state": "persisted_ghost", "proof": "pre_login_snapshot",
            "reload_input": reload_input, "finish_started_trace_sequence": boundary,
            "manual_release_sent": False,
        }
    observation["shutdown_persistence_phase"] = {
        "phase": "ShutdownPersistencePhase", "finish_policy": finish,
        "offline_state": observation["state"],
        "pre_resurrect_during_finish_trace_sequences": [sequence for sequence in pre_res
                                                        if sequence > boundary],
    }


def _owned_spell_cooldowns(snapshot: dict[str, Any], actor: dict[str, Any]) -> list[dict[str, int]]:
    table = snapshot.get("character_spell_cooldown", {})
    columns = [str(value).casefold() for value in table.get("columns", [])]
    required = ("guid", "spell", "time", "categoryid", "categoryend")
    if any(name not in columns for name in required):
        raise ScenarioFailure("spell cooldown snapshot has an unexpected schema")
    rows = []
    for row in table.get("rows", []):
        if len(row) != len(columns) or row[columns.index("guid")] != str(actor["guid"]):
            continue
        try:
            rows.append({name: int(row[columns.index(name)]) for name in required[1:]})
        except ValueError:
            raise ScenarioFailure("spell cooldown snapshot contains a malformed owner row") from None
    return rows


def _has_native_8690_category_cooldown(rows: list[dict[str, int]]) -> bool:
    return (len(rows) == 1 and rows[0]["categoryid"] == 1176
            and rows[0]["time"] > 0 and rows[0]["categoryend"] > 0)


def _cooldown_event_spell_ids(evidence: dict[str, Any]) -> list[int]:
    spell_ids = []
    for event in _events(evidence, COOLDOWN_EVENT):
        body_hex = event.get("body_hex")
        try:
            body = bytes.fromhex(body_hex) if isinstance(body_hex, str) else b""
        except ValueError:
            body = b""
        if len(body) < 4:
            raise ScenarioFailure("cooldown event has no complete spell ID")
        spell_ids.append(int.from_bytes(body[:4], "little", signed=True))
    return spell_ids


def _spell_go_events(evidence: dict[str, Any], spell_id: int) -> list[dict[str, Any]]:
    return [event for event in _events(evidence, SPELL_GO)
            if event.get("spell_id") == spell_id]


def _completed_teleport_ack_indexes(evidence: dict[str, Any]) -> list[int]:
    return [index for index, packet in enumerate(evidence.get("sent_packets", []))
            if isinstance(packet, dict) and packet.get("action") == "ack_move_teleport"
            and packet.get("opcode") == MOVE_TELEPORT_ACK
            and packet.get("connection") == "instance"
            and packet.get("write_attempted") is True
            and packet.get("write_completed") is True
            and packet.get("write_error") is None]


def _validate_stuck_graveyard_position(snapshot: dict[str, Any],
                                       actor: dict[str, Any]) -> dict[str, float | int]:
    row = _actor_snapshot(snapshot, actor)
    try:
        map_id = int(row["map"])
        position = tuple(float(row[key]) for key in ("position_x", "position_y", "position_z"))
    except (KeyError, ValueError):
        raise ScenarioFailure("Stuck repop snapshot lacks a valid map position") from None
    expected = (-9339.72, 170.2, 61.5622)
    distance = math.dist(position, expected)
    if map_id != 0 or not math.isfinite(distance) or distance > 0.25:
        raise ScenarioFailure("Stuck repop did not persist the Alliance graveyard 106 position")
    return {"map": map_id, "x": position[0], "y": position[1], "z": position[2],
            "distance_from_graveyard_106": distance}


def _validate_stuck_evidence(scenario: str, epoch: int, actor: dict[str, Any],
                             snapshot: dict[str, Any], evidence: dict[str, Any],
                             observation: dict[str, Any]) -> None:
    if scenario not in {"stuck-death", "stuck-home"}:
        return
    cooldown_rows = _owned_spell_cooldowns(snapshot, actor)
    cooldown_8690 = [row for row in cooldown_rows if row["spell"] == 8690]
    cooldown_events = _cooldown_event_spell_ids(evidence)
    stuck_go = _spell_go_events(evidence, 7355)
    if len(stuck_go) != 1:
        raise ScenarioFailure("Stuck plan must record exactly one successful 7355 SpellGo")
    if any(event.get("opcode") == DURABILITY_DAMAGE_DEATH
           for event in evidence.get("events", []) if isinstance(event, dict)):
        raise ScenarioFailure("Stuck self-death emitted a death durability packet")
    observation["stuck_spell_cooldowns"] = [row for row in cooldown_rows
                                            if row["spell"] in {7355, 8690}]
    if scenario == "stuck-home":
        if cooldown_8690 or 8690 in cooldown_events or _spell_go_events(evidence, 8690):
            raise ScenarioFailure("home Stuck case unexpectedly used or saved Hearthstone cooldown")
        if not _completed_teleport_ack_indexes(evidence):
            raise ScenarioFailure("home Stuck case lacks a completed native teleport ACK")
        return
    if epoch == 1:
        hearthstone_go = _spell_go_events(evidence, 8690)
        if len(hearthstone_go) != 1:
            raise ScenarioFailure("Stuck death setup lacks its one normal Hearthstone SpellGo")
        target = hearthstone_go[0].get("target") or {}
        destination = target.get("dst_location") or {}
        empty = {"low": 0, "high": 0}
        try:
            position = tuple(float(destination[key]) for key in ("x", "y", "z"))
        except (KeyError, TypeError, ValueError):
            raise ScenarioFailure("Hearthstone SpellGo lacks its native home destination") from None
        if (target.get("flags") != 0x42 or target.get("unit") != empty
                or target.get("item") != empty or destination.get("transport") != empty
                or not all(math.isfinite(value) for value in position)
                or math.dist(position, actor["home"]) > 0.25
                or target.get("map_id") not in (None, actor["map"])):
            raise ScenarioFailure("Hearthstone SpellGo target does not match the native home route")
        observation["hearthstone_target"] = target
        if not _has_native_8690_category_cooldown(cooldown_8690):
            raise ScenarioFailure("Stuck death setup did not persist the native 8690 category cooldown")
        if 8690 in cooldown_events:
            raise ScenarioFailure("Stuck death setup emitted an unexpected 8690 cooldown event")
        acknowledgements = _completed_teleport_ack_indexes(evidence)
        sent = evidence.get("sent_packets", [])
        casts = [index for index, packet in enumerate(sent)
                 if isinstance(packet, dict) and packet.get("action") == "cast"]
        if (len(casts) != 2 or not acknowledgements
                or not casts[0] < acknowledgements[0] < casts[1]):
            raise ScenarioFailure("Hearthstone teleport ACK did not precede the lethal 7355 cast")
        if not _events(evidence, MOVE_ROOT):
            raise ScenarioFailure("Stuck self-death did not observe the native movement-root packet")
        return
    if cooldown_events.count(8690) > 0:
        raise ScenarioFailure("dead-player repop emitted an unrelated 8690 cooldown event")
    if not _has_native_8690_category_cooldown(cooldown_8690):
        raise ScenarioFailure("dead-player repop did not retain the saved 8690 category cooldown")
    graveyard_position = _validate_stuck_graveyard_position(snapshot, actor)
    observation["graveyard_position"] = graveyard_position
    if not _completed_teleport_ack_indexes(evidence) and not _events(evidence, DEATH_RELEASE_LOC):
        raise ScenarioFailure("Stuck repop lacks teleport ACK or DeathReleaseLoc evidence")


def _require_stuck_death_config(scenario: str, world_config: Path) -> None:
    if scenario == "stuck-death" and \
            _read_world_setting(world_config, "DurabilityLoss.InPvP") != "0":
        raise ScenarioFailure("Stuck self-death requires DurabilityLoss.InPvP = 0")


def _evidence_digest(evidence: dict[str, Any]) -> dict[str, Any]:
    return {"passed": evidence.get("passed"), "failure": evidence.get("failure"),
            "finish_completed": evidence.get("finish_completed"),
            "logout_confirmed": evidence.get("logout_confirmed"),
            "expected_rules": evidence.get("expected_rules", []),
            "events": evidence.get("events", []), "sent_packets": evidence.get("sent_packets", []),
            "trace": evidence.get("trace", [])}


def _hashes(identity: dict[str, Any]) -> dict[str, str]:
    return {key: _sha256(identity[key]) for key in
            ("journal_path", "bot_config", "world_config", "bot_exec")}


def _run_single(pass_: BotPass, identity: dict[str, Any], db: death_fixture.MariaDB,
                journal: dict[str, Any], observations: list[dict[str, Any]], *,
                observer: bool = False) -> dict[str, Any]:
    start_bot(pass_, bot_exec=identity["bot_exec"], bot_config=identity["bot_config"],
              output=identity["capture"])
    timeout = math.ceil(pass_.plan["timeout_ms"] / 1000) + 60
    evidence = finish_bot(pass_, timeout_seconds=timeout, actor=pass_.actor, observer=observer)
    snapshot, state = _save_snapshot(db, journal, pass_.actor, identity["scenario"],
                                     identity["epoch"], identity["capture"], pass_.label)
    observations.append({"pass": pass_.label, "actor": pass_.actor["name"],
                         "report_sha256": pass_.report_hash, "state": state,
                         "lifecycle": _evidence_digest(evidence),
                         **_snapshot_facts(snapshot, pass_.actor),
                         "snapshot_sha256": _sha256(_snapshot_path(
                             identity["capture"], identity["scenario"], identity["epoch"],
                             pass_.actor, pass_.label))})
    return evidence


def _run_offer(identity: dict[str, Any], journal: dict[str, Any], actors: dict[str, dict[str, Any]],
               realm_id: int, db: death_fixture.MariaDB, observations: list[dict[str, Any]]) -> None:
    victim = actors["Qadeathfall"]
    caster = actors["Qadeathcast"]
    probe = BotPass(caster, observer_plan(3000, caster["guid"]), "offer-corpse-probe")
    evidence = _run_single(probe, identity, db, journal, observations, observer=True)
    try:
        target = select_offer_corpse(evidence, victim, player_guid(victim, realm_id))
    except ValueError as error:
        raise ScenarioFailure(str(error)) from None
    observations[-1]["offer_corpse_target"] = target
    victim_pass = BotPass(victim, plan_for("offer", 2, victim, realm_id, 10, actors), "offer-victim")
    target_guid = target["guid"]
    cast_a = {"low": 7328001, "high": 0}
    cast_b = {"low": 7328002, "high": 0}
    caster_plan = {"character_guid": caster["guid"], "timeout_ms": 95_000,
        "actions": [
            {"action": "cast", "spell_id": 7328, "cast_id": cast_a, "target": target_guid},
            {"action": "wait", "milliseconds": 70_000},
            {"action": "cast", "spell_id": 7328, "cast_id": cast_b, "target": target_guid},
        ],
        "expect": [
            {"event": "prepare", "client_cast_id": cast_a},
            {"event": "start", "client_cast_id": cast_a, "spell_id": 7328,
             "target": {"corpse_ally": target_guid}},
            {"event": "go", "client_cast_id": cast_a, "spell_id": 7328,
             "target": {"corpse_ally": target_guid}},
            {"event": "prepare", "client_cast_id": cast_b},
            {"event": "start", "client_cast_id": cast_b, "spell_id": 7328,
             "target": {"corpse_ally": target_guid}},
            {"event": "go", "client_cast_id": cast_b, "spell_id": 7328,
             "target": {"corpse_ally": target_guid}},
            _packet("realm", LOGOUT_COMPLETE),
        ], "finish_policy": "logout"}
    caster_pass = BotPass(caster, caster_plan, "offer-caster")
    start_bot(victim_pass, bot_exec=identity["bot_exec"], bot_config=identity["bot_config"],
              output=identity["capture"])
    start_bot(caster_pass, bot_exec=identity["bot_exec"], bot_config=identity["bot_config"],
              output=identity["capture"])
    caster_evidence = finish_bot(caster_pass, timeout_seconds=150, actor=caster)
    caster_snapshot, caster_state = _save_snapshot(db, journal, caster, "offer", 2,
                                                    identity["capture"], caster_pass.label)
    if 7328 not in _spell_rows(caster_snapshot, caster):
        raise ScenarioFailure("offer caster lacks the journal-seeded known-spell proof for 7328")
    observations.append({"pass": caster_pass.label, "actor": caster["name"],
                         "report_sha256": caster_pass.report_hash, "state": caster_state,
                         "lifecycle": _evidence_digest(caster_evidence)})
    victim_evidence = finish_bot(victim_pass, timeout_seconds=150, actor=victim)
    _, victim_state = _save_snapshot(db, journal, victim, "offer", 2,
                                     identity["capture"], victim_pass.label)
    observations.append({"pass": victim_pass.label, "actor": victim["name"],
                         "report_sha256": victim_pass.report_hash, "state": victim_state,
                         "lifecycle": _evidence_digest(victim_evidence)})


def _run_wisp(identity: dict[str, Any], journal: dict[str, Any], actors: dict[str, dict[str, Any]],
              realm_id: int, db: death_fixture.MariaDB, observations: list[dict[str, Any]]) -> None:
    observer = actors["Qadeathcast"]
    wisp = actors["Qadeathwisp"]
    observer_pass = BotPass(observer, observer_plan(150_000, observer["guid"]),
                             "wisp-paladin-observer")
    start_bot(observer_pass, bot_exec=identity["bot_exec"], bot_config=identity["bot_config"],
              output=identity["capture"])
    heal = BotPass(wisp, plan_for("wisp", 2, wisp, realm_id, 10), "wisp-healer")
    heal_evidence = _run_single(heal, identity, db, journal, observations)
    healer_observation = observations[-1]
    # Player::_SaveSpells (20405-20416) omits dependent spells. The seeded
    # runtime HasSpell(20585) proof is the first epoch's actual Wisp aura.
    expiry_plan = wisp_expiry_plan(wisp)
    expiry = BotPass(wisp, expiry_plan, "wisp-expiry-relogin")
    expiry_evidence = _run_single(expiry, identity, db, journal, observations)
    expiry_observation = observations[-1]
    observer_evidence = finish_bot(observer_pass, timeout_seconds=180, actor=observer, observer=True)
    observer_snapshot, observer_state = _save_snapshot(db, journal, observer, "wisp", 2,
                                                        identity["capture"], observer_pass.label)
    observations.append({"pass": observer_pass.label, "actor": observer["name"],
                         "report_sha256": observer_pass.report_hash, "state": observer_state,
                         "lifecycle": _evidence_digest(observer_evidence)})
    sickness_facts = _validate_wisp_sickness(healer_observation, expiry_observation)
    observer_events = [event for opcode in (AURA_UPDATE, UPDATE_OBJECT)
                       for event in _events(observer_evidence, opcode)]
    owner_events = [event for item in (heal_evidence, expiry_evidence)
                    for opcode in (AURA_UPDATE, UPDATE_OBJECT)
                    for event in _events(item, opcode)]
    observations.append({"pass": "wisp-sickness-expiry", **sickness_facts})
    observations.append({"pass": "wisp-aura-evidence", "owner_aura_updates": owner_events,
                         "observer_aura_updates": observer_events,
                         "observer_snapshot_sha256": _sha256(_snapshot_path(
                             identity["capture"], "wisp", identity["epoch"], observer,
                             observer_pass.label))})


def _find_aura_rows(observation: dict[str, Any], spell: int) -> list[dict[str, Any]]:
    # The durable snapshot is retained privately; this lightweight summary is populated below.
    return [row for row in observation.get("auras", []) if row.get("spell") == spell]


def _validate_wisp_sickness(healer_observation: dict[str, Any],
                            expiry_observation: dict[str, Any]) -> dict[str, Any]:
    initial = _find_aura_rows(healer_observation, 15007)
    expired = _find_aura_rows(expiry_observation, 15007)
    # Player.cpp:4394-4402 changes only remaining duration at level 11;
    # the Spell's ten-minute maximum remains intact.
    if (not initial or int(initial[0].get("maxduration", 0)) != 600_000
            or not 0 < int(initial[0].get("remaintime", 0)) <= 60_000):
        raise ScenarioFailure("wisp healer pass lacks sickness with 600000ms maximum and live remaining time up to 60000ms")
    if expired:
        raise ScenarioFailure("wisp sickness aura remained after the bounded 75-second relog")
    return {"initial_max_duration_ms": int(initial[0]["maxduration"]),
            "healer_aura_rows": initial, "expiry_aura_rows": expired,
            "expiry_relogin_wait_ms": 75_000}


def _snapshot_facts(snapshot: dict[str, Any], actor: dict[str, Any]) -> dict[str, Any]:
    aura_table = snapshot.get("character_aura", {})
    columns = [str(name).casefold() for name in aura_table.get("columns", [])]
    rows = aura_table.get("rows", [])
    guid_index = columns.index("guid") if "guid" in columns else -1
    spell_index = columns.index("spell") if "spell" in columns else -1
    if guid_index >= 0 and spell_index >= 0:
        owner_rows = [row for row in rows if len(row) == len(columns)
                      and row[guid_index] == str(actor["guid"])]
        auras = []
        for row in owner_rows:
            fact = {name: row[index] for index, name in enumerate(columns)}
            try:
                fact["spell"] = int(fact["spell"])
                if "maxduration" in fact:
                    fact["maxduration"] = int(fact["maxduration"])
                if "remaintime" in fact:
                    fact["remaintime"] = int(fact["remaintime"])
            except ValueError:
                continue
            auras.append(fact)
    else:
        auras = []
    return {"auras": auras}


def _load_snapshot(path: Path) -> dict[str, Any]:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        raise ScenarioFailure("private database snapshot could not be read") from None


def _check_special_state(scenario: str, epoch: int, actor: dict[str, Any],
                         snapshot: dict[str, Any], observations: list[dict[str, Any]]) -> None:
    if scenario == "wisp" and epoch == 1:
        facts = _snapshot_facts(snapshot, actor)
        observations[-1]["auras"] = facts["auras"]
        # BuildPlayerRepop (Player.cpp:4269-4272) casts Wisp only when
        # HasSpell(20585), then Ghost. Persisted actor auras prove both casts.
        spells = {row["spell"] for row in facts["auras"]}
        if not {8326, 20584} <= spells:
            raise ScenarioFailure("wisp release did not persist both Ghost8326 and Wisp20584 auras")
    if scenario == "reincarnation":
        inventory = snapshot.get("character_inventory", {})
        item_rows = snapshot.get("item_instance", {})
        item_by_id = {row[0]: row for row in item_rows.get("rows", []) if row}
        inv_cols = [str(value).casefold() for value in inventory.get("columns", [])]
        item_col = inv_cols.index("item") if "item" in inv_cols else -1
        bag_rows = inventory.get("rows", [])
        ankh_ids = [row[item_col] for row in bag_rows if item_col >= 0 and len(row) == len(inv_cols)
                    and row[inv_cols.index("guid")] == str(actor["guid"])]
        count_index = [str(value).casefold() for value in item_rows.get("columns", [])].index("count")
        entry_index = [str(value).casefold() for value in item_rows.get("columns", [])].index("itementry")
        ankh_count = sum(int(item_by_id[item][count_index]) for item in ankh_ids if item in item_by_id
                         and int(item_by_id[item][entry_index]) == 17030)
        if ankh_count != 9:
            raise ScenarioFailure("reincarnation pass did not persist exactly nine Ankh items")
        known_spells = _spell_rows(snapshot, actor)
        if 20608 not in known_spells or 21169 in known_spells:
            raise ScenarioFailure("reincarnation passive proof or known-spell boundary is invalid")
        cooldowns = snapshot.get("character_spell_cooldown", {})
        cols = [str(value).casefold() for value in cooldowns.get("columns", [])]
        if not ("spell" in cols and any(len(row) == len(cols)
                and row[cols.index("guid")] == str(actor["guid"])
                and row[cols.index("spell")] == "21169" for row in cooldowns.get("rows", []))):
            raise ScenarioFailure("reincarnation pass lacks the saved 21169 cooldown row")
    if scenario == "healer":
        facts = _snapshot_facts(snapshot, actor)
        observations[-1]["auras"] = facts["auras"]
        sickness = [row for row in facts["auras"] if row.get("spell") == 15007]
        if not sickness or sickness[0].get("maxduration") != 600_000:
            raise ScenarioFailure("healer resurrection did not save sickness with 600000ms max duration")
        actor_row = _actor_snapshot(snapshot, actor)
        observations[-1]["health"] = actor_row.get("health")
        observations[-1]["mana"] = actor_row.get("power1")
        durability = _durability_by_slot(snapshot, actor)
        observations[-1]["durability_by_slot"] = durability
        # ObjectMgr::FillMaxDurability gives entry2121 max60, independently of
        # raw Sparse40. Player::DurabilityLoss removes six on the fall and
        # fifteen on healer resurrection (Player.cpp:4546-4562/4320-4376).
        if durability.get(4) != {"entry": 2121, "durability": 19} or \
                durability.get(35) != {"entry": 2121, "durability": 25}:
            raise ScenarioFailure("healer pass did not persist the reviewed 2121 durability results")
    if actor["name"] == "Qadeathfall" and scenario in {
            "fall-release", "reclaim", "automatic-release", "recovery-dead", "recovery-ghost"}:
        durability = _durability_by_slot(snapshot, actor)
        observations[-1]["durability_by_slot"] = durability
        if durability.get(4) != {"entry": 2121, "durability": 34} or \
                durability.get(35) != {"entry": 2121, "durability": 40}:
            raise ScenarioFailure("fall/recovery pass did not persist equipped34 and backpack40")


def _spell_rows(snapshot: dict[str, Any], actor: dict[str, Any]) -> set[int]:
    table = snapshot.get("character_spell", {})
    columns = [str(name).casefold() for name in table.get("columns", [])]
    if "guid" not in columns or "spell" not in columns:
        raise ScenarioFailure("database spell snapshot has an unexpected schema")
    return {int(row[columns.index("spell")]) for row in table.get("rows", [])
            if len(row) == len(columns) and row[columns.index("guid")] == str(actor["guid"])}


def _durability_by_slot(snapshot: dict[str, Any], actor: dict[str, Any]) -> dict[int, dict[str, int]]:
    inventory = snapshot.get("character_inventory", {})
    items = snapshot.get("item_instance", {})
    inv_columns = [str(value).casefold() for value in inventory.get("columns", [])]
    item_columns = [str(value).casefold() for value in items.get("columns", [])]
    if any(name not in inv_columns for name in ("guid", "slot", "item")) or any(
            name not in item_columns for name in ("guid", "itementry", "durability")):
        raise ScenarioFailure("durability snapshot has an unexpected inventory schema")
    item_rows = {row[item_columns.index("guid")]: row for row in items.get("rows", [])
                 if len(row) == len(item_columns)}
    result = {}
    for row in inventory.get("rows", []):
        if len(row) != len(inv_columns) or row[inv_columns.index("guid")] != str(actor["guid"]):
            continue
        item = item_rows.get(row[inv_columns.index("item")])
        if item is not None:
            result[int(row[inv_columns.index("slot")])] = {
                "entry": int(item[item_columns.index("itementry")]),
                "durability": int(item[item_columns.index("durability")]),
            }
    return result


def _run_case(identity: dict[str, Any], journal: dict[str, Any], realm_id: int,
              durability_percent: int, db: death_fixture.MariaDB) -> dict[str, Any]:
    scenario, epoch = identity["scenario"], identity["epoch"]
    actors = {item["name"]: item for item in journal["actors"]}
    observations: list[dict[str, Any]] = []
    if scenario == "offer" and epoch == 2:
        _run_offer(identity, journal, actors, realm_id, db, observations)
    elif scenario == "wisp" and epoch == 2:
        _run_wisp(identity, journal, actors, realm_id, db, observations)
    else:
        actor_name = {"fall-release": "Qadeathfall", "reclaim": "Qadeathfall",
            "healer": "Qadeathfall", "wisp": "Qadeathwisp", "reincarnation": "Qadeathreinc",
            "mob": "Qadeathmob", "automatic-release": "Qadeathfall", "offer": "Qadeathfall",
            "recovery-dead": "Qadeathfall", "recovery-ghost": "Qadeathfall",
            "stuck-death": "Qadeathcast", "stuck-home": "Qadeathcast"}[scenario]
        actor = actors[actor_name]
        pass_ = BotPass(actor, plan_for(scenario, epoch, actor, realm_id, durability_percent),
                        f"{scenario}-{identity['epoch']:03d}")
        reload_input = None
        if epoch == 2 and scenario in {"recovery-dead", "stuck-death"}:
            label = f"{pass_.label}-before-login"
            _, state = _save_snapshot(db, journal, actor, scenario, epoch, identity["capture"], label)
            reload_input = {"state": state, "snapshot_sha256": _sha256(_snapshot_path(
                identity["capture"], scenario, epoch, actor, label))}
        evidence = _run_single(pass_, identity, db, journal, observations)
        snapshot_path = _snapshot_path(identity["capture"], scenario, epoch, actor, pass_.label)
        snapshot = _load_snapshot(snapshot_path)
        _check_special_state(scenario, epoch, actor, snapshot, observations)
        _validate_stuck_evidence(scenario, epoch, actor, snapshot, evidence, observations[-1])
        _validate_death_shutdown_evidence(scenario, epoch, pass_.plan, evidence,
                                          observations[-1], reload_input)
        if scenario == "automatic-release":
            sent = evidence.get("sent_packets", [])
            if any(packet.get("action") == "release_spirit" for packet in sent):
                raise ScenarioFailure("automatic-release plan sent an explicit release packet")
        if scenario == "mob":
            _validate_mob_evidence(evidence, observations[-1])
        if scenario in {"fall-release", "reclaim"}:
            sent_actions = [packet.get("action") for packet in evidence.get("sent_packets", [])]
            needed = {"fall-release": ("release_spirit", 2), "reclaim": ("reclaim_corpse", 3)}[scenario]
            if sent_actions.count(needed[0]) != needed[1]:
                raise ScenarioFailure("duplicate release or three-step reclaim sequence was not recorded")
        if (scenario == "healer" and epoch == 1) or (scenario == "wisp" and epoch == 2):
            selections = evidence.get("target_selections", [])
            if not any(item.get("action") == "spirit_healer_activate"
                       and item.get("selector", {}).get("entry") == 6491
                       and item.get("selector", {}).get("map_id") == 0 for item in selections):
                raise ScenarioFailure("spirit healer was not selected from observed entry-6491 evidence")
        if scenario == "reincarnation":
            starts = [item.get("action") for item in evidence.get("trace", [])
                      if item.get("kind") == "action_started"]
            if starts.count("self_res") != 2:
                raise ScenarioFailure("reincarnation plan did not issue both active 21169 requests")
    if scenario in {"fall-release", "automatic-release", "recovery-ghost"}:
        for row in observations:
            if row["state"]["owned_corpse_count"] != 1:
                raise ScenarioFailure("ghost case did not persist exactly one owned corpse")
    if scenario in {"reclaim", "healer", "wisp", "offer"} and epoch == SCENARIOS[scenario]:
        final_state = next((item.get("state") for item in reversed(observations)
                            if item.get("actor") == ("Qadeathwisp" if scenario == "wisp" else
                                "Qadeathfall") and item.get("state")), None)
        if not final_state or int(final_state["health"]) <= 0:
            raise ScenarioFailure("resurrection case did not persist positive health")
    return {"scenario": scenario, "epoch": epoch, "scenario_observation": "passed",
            "observations": observations, "parity_assessed": False}


def _validate_mob_evidence(evidence: dict[str, Any], observation: dict[str, Any]) -> None:
    trace = evidence.get("trace", [])
    attack_start = min((int(item["sequence"]) for item in trace
                        if item.get("action") == "attack_swing" and item.get("kind") == "action_started"),
                       default=-1)
    if attack_start < 0:
        raise ScenarioFailure("mob plan did not retain its observed-target attack boundary")
    finish_start = min((int(item["sequence"]) for item in trace
                        if item.get("action") == "finish_logout"
                        and item.get("kind") == "action_started"), default=-1)
    if finish_start < 0:
        raise ScenarioFailure("mob evidence lacks its pre-shutdown observation boundary")
    attack_events = [event for event in evidence.get("events", [])
                     if event.get("opcode") == ATTACKER_STATE_UPDATE
                     and int(event.get("body_bytes", 0)) > 0]
    event_traces = {item.get("event_sequence"): item for item in trace
                    if item.get("event_sequence") is not None}
    root_entries = [item for item in trace if item.get("opcode") == MOVE_ROOT
                    and item.get("kind") in {"packet_observed", "packet_redacted"}
                    and item.get("connection") == "instance"
                    and 0 <= int(item.get("sequence", -1)) < finish_start]
    if not attack_events or not root_entries:
        raise ScenarioFailure("mob evidence lacks retained damage or movement-root packets")
    damage_phases = []
    for event in attack_events:
        packet_trace = event_traces.get(event.get("sequence"))
        if packet_trace is None:
            raise ScenarioFailure("mob damage packet has no retained trace phase")
        if int(packet_trace["sequence"]) >= finish_start:
            continue
        phase = ("login_observation" if int(packet_trace["sequence"]) < attack_start
                 else "post_attack_observation")
        damage_phases.append({"phase": phase, "event": event})
    if not damage_phases:
        raise ScenarioFailure("mob evidence lacks retained damage before finish began")
    root_phases = ["login_observation" if int(item.get("sequence", -1)) < attack_start
                   else "post_attack_observation" for item in root_entries]
    selections = [item for item in evidence.get("target_selections", [])
                  if item.get("action") == "attack_swing"
                  and item.get("selector", {}).get("entry") == 6
                  and item.get("selector", {}).get("map_id") == 0]
    if not selections:
        raise ScenarioFailure("mob attack did not retain its observed entry-6 creature GUID")
    selected_guid = selections[0].get("guid")
    if not isinstance(selected_guid, dict) or not selected_guid.get("low") or not selected_guid.get("high"):
        raise ScenarioFailure("mob observed creature selection did not retain a nonempty runtime GUID")
    observation["mob_evidence"] = {"damage_phases": damage_phases,
        "root_phases": root_phases, "selected_creature_guid": selected_guid,
        "selected_target_source_event_sha256": selections[0].get("source_event_sha256")}


def _failure_report(exc: BaseException, scenario: str | None, epoch: int | None) -> dict[str, Any]:
    # Exception text and tracebacks may contain command arguments or private config details.
    return {"schema": "rustycore-death-scenario-v1", "scenario": scenario,
            "epoch": epoch, "scenario_observation": "failed", "parity_assessed": False,
            "failure": {"type": type(exc).__name__, "reason": "scenario runner failed; inspect private logs"}}


def _failure_path_from_environment() -> Path | None:
    try:
        capture = Path(os.environ["RUSTYCORE_CAPTURE_DIR"])
        report = Path(os.environ["RUSTYCORE_CAPTURE_REPORT"])
        if (capture.is_absolute() and not capture.is_symlink() and capture.is_dir()
                and capture.resolve(strict=True) == capture
                and capture.stat().st_mode & 0o777 == 0o700
                and report == capture / "scenario-report.json"
                and not report.exists() and not report.is_symlink()):
            return report
    except (KeyError, OSError, ValueError):
        return None
    return None


def run() -> int:
    identity: dict[str, Any] | None = None
    try:
        identity = _runtime_identity()
        journal = death_fixture.read_journal(identity["journal_path"])
        if journal.get("state") != "prepared":
            raise ScenarioFailure("fixture journal is not in prepared state")
        bot_config_hash = _sha256(identity["bot_config"])
        death_fixture.validate_private_bot_config(identity["bot_config"])
        world_raw = death_fixture.parse_db_info(identity["world_config"])
        db = death_fixture.MariaDB(world_raw)
        death_fixture.require_same_target_database(db, journal)
        realm_text = os.environ.get("REALM_ID") or _read_world_setting(identity["world_config"], "RealmID") or "1"
        if not re.fullmatch(r"[0-9]+", realm_text) or not 1 <= int(realm_text) <= 0x1FFF:
            raise ScenarioFailure("realm ID is outside the player GUID range")
        durability_text = _read_world_setting(identity["world_config"], "DurabilityLoss.OnDeath") or "10.0"
        try:
            durability_value = float(durability_text)
        except ValueError:
            raise ScenarioFailure("death durability setting is invalid") from None
        if not math.isfinite(durability_value) or durability_value != 10.0:
            raise ScenarioFailure("death durability setting must be the reviewed 10 percent")
        _require_stuck_death_config(identity["scenario"], identity["world_config"])
        scenario_report = _run_case(identity, journal, int(realm_text), int(durability_value), db)
        if _sha256(identity["bot_config"]) != bot_config_hash:
            raise ScenarioFailure("private bot config changed during scenario execution")
        scenario_report.update({"schema": "rustycore-death-scenario-v1",
            "backend": os.environ["RUSTYCORE_CAPTURE_BACKEND"],
            "hashes": _hashes(identity), "bot_config_unchanged": True,
            "candidate_server_sha256": os.environ["RUSTYCORE_CAPTURE_CANDIDATE_SHA256"].lower(),
            "scenario_runner_sha256": _sha256(Path(__file__).resolve()),
            "target_database": journal["target_database"],
            "native_collection": "recorded by outer capture wrapper", "parity_assessed": False})
        _write_private(identity["report_path"], scenario_report)
        return 0
    except BaseException as exc:
        for pass_ in list(ACTIVE_BOTS):
            _stop_owned(pass_)
        report_path = identity["report_path"] if identity else _failure_path_from_environment()
        if report_path is not None:
            try:
                _write_private(report_path, _failure_report(
                    exc, identity.get("scenario") if identity else
                    os.environ.get("RUSTYCORE_DEATH_SCENARIO"),
                    identity.get("epoch") if identity else None))
            except Exception:
                pass
        return 70


if __name__ == "__main__":
    raise SystemExit(run())
