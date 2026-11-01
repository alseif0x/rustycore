#!/usr/bin/env python3
"""Guarded #45 solo instance reset scenario using the death-fixture wrapper."""

from __future__ import annotations

import math
import os
from pathlib import Path
import re
from typing import Any

import death_fixture
import death_scenario
import logout_scenario


SCENARIO = "instance-solo-reset"
ACTOR_NAME = "Qadeathcast"
SCENARIOS = {SCENARIO: ACTOR_NAME}
INSTANCE_MAP = 33
INSTANCE_POSITION = (-228.191, 2111.41, 76.8904)
CMSG_RESET_INSTANCES = 0x366A
SMSG_INSTANCE_RESET = 0x2686
LOGOUT_COMPLETE = 0x2684
POSITION_TOLERANCE = 0.02
ScenarioFailure = death_scenario.ScenarioFailure


def observer_plan(actor: dict[str, Any]) -> dict[str, Any]:
    return death_scenario.observer_plan(3_000, int(actor["guid"]))


def reset_plan(actor: dict[str, Any]) -> dict[str, Any]:
    return {
        "character_guid": int(actor["guid"]),
        "timeout_ms": 10_000,
        "actions": [
            {"action": "reset_instances"},
            death_scenario._wait("instance", SMSG_INSTANCE_RESET, 5_000),
        ],
        "expect": [
            death_scenario._packet(
                "instance", SMSG_INSTANCE_RESET, INSTANCE_MAP.to_bytes(4, "little")
            ),
            death_scenario._packet("realm", LOGOUT_COMPLETE),
        ],
        "finish_policy": "logout",
    }


def _require_offline(db: death_fixture.MariaDB, journal: dict[str, Any],
                     actor: dict[str, Any]) -> None:
    rows = db.character_identities(journal)
    matches = [row for row in rows if len(row) == 4 and row[0] == str(actor["guid"])
               and row[1] == str(actor["account_id"]) and row[2] == actor["name"]]
    if len(matches) != 1 or matches[0][3] != "0":
        raise ScenarioFailure("instance scenario actor is not offline with its exact account identity")


def _position_update(actor: dict[str, Any], map_id: int, instance_id: int,
                     position: tuple[float, float, float]) -> str:
    x, y, z = position
    return (
        f"UPDATE characters SET map={map_id},instance_id={instance_id},"
        f"position_x={x!r},position_y={y!r},position_z={z!r},orientation=0.0 "
        f"WHERE guid={int(actor['guid'])} AND account={int(actor['account_id'])} "
        f"AND name={death_fixture.sql_quote(actor['name'])} AND online=0"
    )


def _position_select(actor: dict[str, Any]) -> str:
    return (
        "SELECT guid,account,name,online,map,instance_id,position_x,position_y,position_z,orientation "
        f"FROM characters WHERE guid={int(actor['guid'])}"
    )


def _database_position(db: death_fixture.MariaDB, actor: dict[str, Any]) -> dict[str, Any]:
    rows = death_fixture.MariaDB._rows(db.execute(_position_select(actor)), 10)
    if len(rows) != 1:
        raise ScenarioFailure("instance scenario character position row is missing or ambiguous")
    row = rows[0]
    if row[:4] != [str(actor["guid"]), str(actor["account_id"]), actor["name"], "0"]:
        raise ScenarioFailure("instance scenario character identity or offline guard changed")
    try:
        return {
            "guid": int(row[0]), "account_id": int(row[1]), "name": row[2], "online": int(row[3]),
            "map": int(row[4]), "instance_id": int(row[5]),
            "position_x": float(row[6]), "position_y": float(row[7]),
            "position_z": float(row[8]), "orientation": float(row[9]),
        }
    except ValueError:
        raise ScenarioFailure("instance scenario character position contains invalid numbers") from None


def _require_database_position(db: death_fixture.MariaDB, actor: dict[str, Any],
                               map_id: int, instance_id: int,
                               position: tuple[float, float, float]) -> dict[str, Any]:
    state = _database_position(db, actor)
    if state["map"] != map_id or state["instance_id"] != instance_id or not _near(state, position):
        raise ScenarioFailure("instance scenario position update did not persist its exact target")
    if not math.isclose(state["orientation"], 0.0, abs_tol=1e-6):
        raise ScenarioFailure("instance scenario orientation was not persisted as zero")
    return state


def _near(state: dict[str, Any], position: tuple[float, float, float]) -> bool:
    return all(math.isclose(state[key], value, abs_tol=POSITION_TOLERANCE)
               for key, value in zip(("position_x", "position_y", "position_z"), position, strict=True))


def relocate_inside(db: death_fixture.MariaDB, journal: dict[str, Any],
                    actor: dict[str, Any]) -> dict[str, Any]:
    _require_offline(db, journal, actor)
    db.transaction([_position_update(actor, INSTANCE_MAP, 0, INSTANCE_POSITION)])
    return _require_database_position(db, actor, INSTANCE_MAP, 0, INSTANCE_POSITION)


def restore_outside(db: death_fixture.MariaDB, journal: dict[str, Any],
                    actor: dict[str, Any]) -> dict[str, Any]:
    _require_offline(db, journal, actor)
    home = tuple(float(value) for value in actor["home"])
    db.transaction([_position_update(actor, int(actor["map"]), 0, home)])
    return _require_database_position(db, actor, int(actor["map"]), 0, home)


def _snapshot_position(snapshot: dict[str, Any], actor: dict[str, Any]) -> dict[str, Any]:
    row = death_scenario._actor_snapshot(snapshot, actor)
    try:
        return {
            "guid": int(row["guid"]), "online": int(row["online"]),
            "logout_time": int(row.get("logout_time", 0)),
            "map": int(row["map"]), "instance_id": int(row["instance_id"]),
            "position_x": float(row["position_x"]), "position_y": float(row["position_y"]),
            "position_z": float(row["position_z"]), "orientation": float(row["orientation"]),
        }
    except (KeyError, ValueError):
        raise ScenarioFailure("instance snapshot lacks valid position or instance state") from None


def _save_position_snapshot(db: death_fixture.MariaDB, journal: dict[str, Any],
                            actor: dict[str, Any], output: Path,
                            label: str) -> tuple[dict[str, Any], dict[str, Any], Path]:
    death_scenario._poll_actor_offline(db, journal, actor)
    snapshot = db.snapshot(journal)
    path = death_scenario._snapshot_path(output, SCENARIO, 1, actor, label)
    death_scenario._write_private(path, snapshot)
    state = _snapshot_position(snapshot, actor)
    if state["online"] != 0:
        raise ScenarioFailure("instance scenario snapshot retained an online character")
    return snapshot, state, path


def validate_inside_state(state: dict[str, Any], expected_instance_id: int | None = None) -> int:
    if state.get("map") != INSTANCE_MAP or not _near(state, INSTANCE_POSITION):
        raise ScenarioFailure("instance entry did not persist map 33 at the native location")
    instance_id = state.get("instance_id")
    if type(instance_id) is not int or instance_id <= 0:
        raise ScenarioFailure("instance entry did not allocate a nonzero instance ID")
    if expected_instance_id is not None and instance_id != expected_instance_id:
        raise ScenarioFailure("relogin did not retain the exact instance incarnation")
    if not math.isclose(state["orientation"], 0.0, abs_tol=1e-6):
        raise ScenarioFailure("instance entry orientation was not zero")
    return instance_id


def validate_reconnect_state(initial: dict[str, Any], reconnect: dict[str, Any]) -> int:
    initial_instance = validate_inside_state(initial)
    return validate_inside_state(reconnect, initial_instance)


def validate_outside_state(actor: dict[str, Any], state: dict[str, Any]) -> None:
    home = tuple(float(value) for value in actor["home"])
    if state.get("map") != int(actor["map"]) or state.get("instance_id") != 0 or not _near(state, home):
        raise ScenarioFailure("final character position is not the outside journal home")
    if not math.isclose(state["orientation"], 0.0, abs_tol=1e-6):
        raise ScenarioFailure("final outside orientation was not zero")


def _sequence(item: dict[str, Any]) -> int:
    value = item.get("observation_sequence", -1)
    return value if type(value) is int else -1


def validate_reset_evidence(evidence: dict[str, Any]) -> dict[str, Any]:
    requests = [packet for packet in evidence.get("sent_packets", [])
                if isinstance(packet, dict) and packet.get("action") == "reset_instances"
                and packet.get("connection") == "instance"
                and packet.get("opcode") == CMSG_RESET_INSTANCES
                and packet.get("body_hex") == ""
                and packet.get("write_attempted") is True
                and packet.get("write_completed") is True
                and packet.get("write_error") is None
                and packet.get("action_index") is not None]
    if len(requests) != 1:
        raise ScenarioFailure("solo reset requires exactly one completed typed reset request")
    expected_body = INSTANCE_MAP.to_bytes(4, "little").hex()
    responses = [event for event in evidence.get("events", [])
                 if isinstance(event, dict) and event.get("opcode") == SMSG_INSTANCE_RESET
                 and event.get("connection") == "instance"
                 and event.get("body_hex") == expected_body]
    all_responses = [event for event in evidence.get("events", [])
                     if isinstance(event, dict) and event.get("opcode") == SMSG_INSTANCE_RESET]
    if len(responses) != 1 or len(all_responses) != 1:
        raise ScenarioFailure("solo reset requires exactly one map-33 instance-reset response")
    if _sequence(responses[0]) <= _sequence(requests[0]):
        raise ScenarioFailure("instance reset response was not observed after its typed request")
    return {
        "request_sequence": requests[0].get("sequence"),
        "response_sequence": responses[0].get("sequence"),
        "map_id": INSTANCE_MAP,
    }


def _run_case(identity: dict[str, Any], journal: dict[str, Any], realm_id: int,
              db: death_fixture.MariaDB) -> dict[str, Any]:
    del realm_id
    actor = next(item for item in journal["actors"] if item["name"] == ACTOR_NAME)
    relocated = False
    restored = False
    try:
        relocate_inside(db, journal, actor)
        relocated = True
        observations = []

        entry_pass = death_scenario.BotPass(actor, observer_plan(actor), "instance-entry-001")
        entry_evidence = logout_scenario._run_single(entry_pass, identity, observer=True)
        _, entry_state, entry_path = _save_position_snapshot(
            db, journal, actor, identity["capture"], entry_pass.label)
        instance_id = validate_inside_state(entry_state)
        observations.append(logout_scenario._observation(
            entry_pass, entry_evidence, entry_state, entry_path, {"instance_id": instance_id}))

        relogin_pass = death_scenario.BotPass(actor, observer_plan(actor), "instance-relogin-001")
        relogin_evidence = logout_scenario._run_single(relogin_pass, identity, observer=True)
        _, relogin_state, relogin_path = _save_position_snapshot(
            db, journal, actor, identity["capture"], relogin_pass.label)
        validate_reconnect_state(entry_state, relogin_state)
        observations.append(logout_scenario._observation(
            relogin_pass, relogin_evidence, relogin_state, relogin_path,
            {"relogin_same_instance": True, "instance_id": instance_id}))

        restore_outside(db, journal, actor)
        restored = True
        reset_pass = death_scenario.BotPass(actor, reset_plan(actor), "instance-reset-001")
        reset_evidence = logout_scenario._run_single(reset_pass, identity)
        reset_facts = validate_reset_evidence(reset_evidence)
        _, final_state, final_path = _save_position_snapshot(
            db, journal, actor, identity["capture"], reset_pass.label)
        validate_outside_state(actor, final_state)
        observations.append(logout_scenario._observation(
            reset_pass, reset_evidence, final_state, final_path, reset_facts))
        return {
            "scenario": SCENARIO,
            "epoch": 1,
            "scenario_observation": "passed",
            "observations": observations,
            "final_state": final_state,
            "position_restoration": {
                "outside_home": True,
                "scenario_owned_changes_restored": True,
                "group_or_lock_rows_mutated": False,
            },
            "parity_assessed": False,
        }
    except BaseException:
        for pass_ in list(death_scenario.ACTIVE_BOTS):
            death_scenario._stop_owned(pass_)
        if relocated and not restored:
            death_scenario._poll_actor_offline(db, journal, actor)
            restore_outside(db, journal, actor)
        raise


def _runtime_identity() -> dict[str, Any]:
    required = ("RUSTYCORE_DEATH_JOURNAL", "RUSTYCORE_DEATH_EPOCH",
                "RUSTYCORE_DEATH_EPOCH_INDEX", "RUSTYCORE_DEATH_BOT_CONFIG",
                "RUSTYCORE_DEATH_WORLD_CONFIG", "RUSTYCORE_DEATH_SCENARIO",
                "RUSTYCORE_CAPTURE_DIR", "RUSTYCORE_CAPTURE_REPORT",
                "RUSTYCORE_CAPTURE_BACKEND", "WOW_BOT_EXEC")
    if any(not os.environ.get(key) for key in required):
        raise ScenarioFailure("required guarded scenario environment is incomplete")
    scenario = os.environ["RUSTYCORE_DEATH_SCENARIO"]
    if scenario not in SCENARIOS or os.environ["RUSTYCORE_DEATH_EPOCH_INDEX"] != "1" \
            or os.environ["RUSTYCORE_DEATH_EPOCH"] != "epoch-001":
        raise ScenarioFailure("instance scenario or epoch is outside the bounded case set")
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
        if not path.is_absolute() or path.is_symlink() or not path.exists() \
                or path.resolve(strict=True) != path:
            raise ScenarioFailure("scenario input path is not canonical")
    bot_exec = Path(os.environ["WOW_BOT_EXEC"])
    if not bot_exec.is_file() or not os.access(bot_exec, os.X_OK):
        raise ScenarioFailure("candidate bot is not an executable file")
    candidate_sha = os.environ.get("RUSTYCORE_CAPTURE_CANDIDATE_SHA256", "")
    bot_sha = os.environ.get("WOW_BOT_EXEC_SHA256", "")
    if not re.fullmatch(r"[0-9a-fA-F]{64}", candidate_sha) or \
            not re.fullmatch(r"[0-9a-fA-F]{64}", bot_sha) or \
            death_scenario._sha256(bot_exec) != bot_sha.lower():
        raise ScenarioFailure("candidate executable hash pin is invalid")
    if os.environ["RUSTYCORE_CAPTURE_BACKEND"] not in {"rust", "cpp"}:
        raise ScenarioFailure("capture backend is invalid")
    return {"scenario": scenario, "epoch": 1, "capture": capture, "report_path": report,
            "journal_path": Path(os.environ["RUSTYCORE_DEATH_JOURNAL"]),
            "bot_config": Path(os.environ["RUSTYCORE_DEATH_BOT_CONFIG"]),
            "world_config": Path(os.environ["RUSTYCORE_DEATH_WORLD_CONFIG"]),
            "bot_exec": bot_exec, "backend": os.environ["RUSTYCORE_CAPTURE_BACKEND"]}


def _failure_report(exc: BaseException, scenario: str | None) -> dict[str, Any]:
    report = death_scenario._failure_report(exc, scenario, 1)
    report["schema"] = "rustycore-instance-scenario-v1"
    return report


def run() -> int:
    identity: dict[str, Any] | None = None
    try:
        identity = _runtime_identity()
        journal = death_fixture.read_journal(identity["journal_path"])
        if journal.get("state") != "prepared":
            raise ScenarioFailure("fixture journal is not in prepared state")
        bot_config_hash = death_scenario._sha256(identity["bot_config"])
        death_fixture.validate_private_bot_config(identity["bot_config"])
        db = death_fixture.MariaDB(death_fixture.parse_db_info(identity["world_config"]))
        death_fixture.require_same_target_database(db, journal)
        realm_text = os.environ.get("REALM_ID") or death_scenario._read_world_setting(
            identity["world_config"], "RealmID") or "1"
        if not re.fullmatch(r"[0-9]+", realm_text) or not 1 <= int(realm_text) <= 0x1FFF:
            raise ScenarioFailure("realm ID is outside the player GUID range")
        report = _run_case(identity, journal, int(realm_text), db)
        if death_scenario._sha256(identity["bot_config"]) != bot_config_hash:
            raise ScenarioFailure("private bot config changed during scenario execution")
        report.update({
            "schema": "rustycore-instance-scenario-v1",
            "backend": os.environ["RUSTYCORE_CAPTURE_BACKEND"],
            "hashes": death_scenario._hashes(identity),
            "bot_config_unchanged": True,
            "candidate_server_sha256": os.environ["RUSTYCORE_CAPTURE_CANDIDATE_SHA256"].lower(),
            "scenario_runner_sha256": death_scenario._sha256(Path(__file__).resolve()),
            "target_database": journal["target_database"],
            "native_collection": "recorded by outer capture wrapper",
            "parity_assessed": False,
        })
        death_scenario._write_private(identity["report_path"], report)
        return 0
    except BaseException as exc:
        for pass_ in list(death_scenario.ACTIVE_BOTS):
            death_scenario._stop_owned(pass_)
        report_path = identity["report_path"] if identity else death_scenario._failure_path_from_environment()
        if report_path is not None:
            try:
                death_scenario._write_private(report_path, _failure_report(
                    exc, identity.get("scenario") if identity else
                    os.environ.get("RUSTYCORE_DEATH_SCENARIO")))
            except Exception:
                pass
        return 70


if __name__ == "__main__":
    raise SystemExit(run())
