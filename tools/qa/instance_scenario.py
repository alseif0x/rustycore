#!/usr/bin/env python3
"""Guarded #45 solo instance reset scenario using the death-fixture wrapper."""

from __future__ import annotations

import hashlib
import json
import math
import os
from pathlib import Path
import re
import struct
import time
from typing import Any

import death_fixture
import death_scenario
import instance_party_update
import instance_soak
import logout_scenario


SCENARIO = "instance-solo-reset"
GROUP_SCENARIO = "instance-group-reset"
DIFFICULTY_SCENARIO = "instance-difficulty-restart"
LOCK_SCENARIO = "instance-lock-reset"
ACTOR_NAME = "Qadeathcast"
MEMBER_NAME = "Qadeathfall"
SCENARIOS = {
    SCENARIO: ACTOR_NAME,
    GROUP_SCENARIO: ACTOR_NAME,
    DIFFICULTY_SCENARIO: ACTOR_NAME,
    LOCK_SCENARIO: ACTOR_NAME,
    instance_soak.SCENARIO: ACTOR_NAME,
}
INSTANCE_MAP = 33
INSTANCE_POSITION = (-228.191, 2111.41, 76.8904)
CMSG_RESET_INSTANCES = 0x366A
CMSG_LEAVE_GROUP = 0x364C
CMSG_CAST_SPELL = 0x329C
SMSG_PARTY_COMMAND_RESULT = 0x2796
HEARTHSTONE_SPELL_ID = 8690
SMSG_NEW_WORLD = 0x2594
SMSG_INSTANCE_RESET = 0x2686
SMSG_INSTANCE_RESET_FAILED = 0x2687
SMSG_RESET_FAILED_NOTIFY = 0x26B7
SMSG_PENDING_RAID_LOCK = 0x26F8
SMSG_RAID_INSTANCE_MESSAGE = 0x2BB4
SMSG_INSTANCE_SAVE_CREATED = 0x2780
SMSG_CALENDAR_RAID_LOCKOUT_ADDED = 0x2699
CMSG_INSTANCE_LOCK_RESPONSE = 0x350B
CMSG_REQUEST_RAID_INFO = 0x36D2
SMSG_INSTANCE_INFO = 0x2634
SMSG_PARTY_INVITE = 0x25BD
SMSG_PARTY_UPDATE = 0x25F4
SMSG_SET_DUNGEON_DIFFICULTY = 0x26A4
LOCK_MAP = 631
LOCK_ID = 12
LOCK_DIFFICULTY = 4
LOCK_POSITION = (73.0174, 2211.49, 30.1157)
ICC_SHARED_SAVE_DATA = death_fixture.ICC_SHARED_SAVE_DATA
ICC_LEADER_LOCK_SEED_DATA = death_fixture.ICC_LEADER_LOCK_SEED_DATA
LOGOUT_COMPLETE = 0x2684
POSITION_TOLERANCE = 0.02
ScenarioFailure = death_scenario.ScenarioFailure


def observer_plan(actor: dict[str, Any]) -> dict[str, Any]:
    return death_scenario.observer_plan(3_000, int(actor["guid"]))


def disconnect_observer_plan(actor: dict[str, Any]) -> dict[str, Any]:
    return {
        "character_guid": int(actor["guid"]), "observe_only": True,
        "timeout_ms": 3_000, "actions": [], "expect": [],
        "finish_policy": "disconnect",
    }


def reset_plan(actor: dict[str, Any], realm_id: int) -> dict[str, Any]:
    return {
        "character_guid": int(actor["guid"]),
        "timeout_ms": 45_000,
        "actions": [
            {"action": "cast", "spell_id": HEARTHSTONE_SPELL_ID,
             "cast_id": {"low": 8_690_001, "high": 0},
             "target": death_scenario.player_guid(actor, realm_id)},
            death_scenario._wait("realm", SMSG_NEW_WORLD, 30_000),
            {"action": "wait", "milliseconds": 2_000},
            {"action": "reset_instances"},
            death_scenario._wait("realm", SMSG_INSTANCE_RESET, 5_000),
        ],
        "expect": [
            death_scenario._packet(
                "realm", SMSG_INSTANCE_RESET, INSTANCE_MAP.to_bytes(4, "little")
            ),
            death_scenario._packet("realm", LOGOUT_COMPLETE),
        ],
        "finish_policy": "logout",
    }


def group_reset_plan(actor: dict[str, Any]) -> dict[str, Any]:
    return {
        "character_guid": int(actor["guid"]),
        "timeout_ms": 10_000,
        "actions": [
            {"action": "reset_instances"},
            death_scenario._wait("realm", SMSG_INSTANCE_RESET, 5_000),
        ],
        "expect": [
            death_scenario._packet(
                "realm", SMSG_INSTANCE_RESET, INSTANCE_MAP.to_bytes(4, "little")
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
                            label: str, *, scenario_name: str = SCENARIO,
                            require_offline: bool = True,
                            epoch: int = 1) -> tuple[dict[str, Any], dict[str, Any], Path]:
    if require_offline:
        death_scenario._poll_actor_offline(db, journal, actor)
    snapshot = db.snapshot(journal)
    path = death_scenario._snapshot_path(output, scenario_name, epoch, actor, label)
    death_scenario._write_private(path, snapshot)
    state = _snapshot_position(snapshot, actor)
    if require_offline and state["online"] != 0:
        raise ScenarioFailure("instance scenario snapshot retained an online character")
    return snapshot, state, path


def validate_inside_state(state: dict[str, Any], expected_instance_id: int | None = None,
                          allow_unsaved_online: bool = False) -> int:
    if state.get("map") != INSTANCE_MAP or not _near(state, INSTANCE_POSITION):
        raise ScenarioFailure("instance entry did not persist map 33 at the native location")
    instance_id = state.get("instance_id")
    if instance_id == 0 and allow_unsaved_online and state.get("online") == 1:
        if not math.isclose(state["orientation"], 0.0, abs_tol=1e-6):
            raise ScenarioFailure("instance entry orientation was not zero")
        return expected_instance_id if expected_instance_id is not None else 0
    if type(instance_id) is not int or instance_id <= 0:
        raise ScenarioFailure("instance entry did not allocate a nonzero instance ID")
    if expected_instance_id is not None and instance_id != expected_instance_id:
        raise ScenarioFailure("relogin did not retain the exact instance incarnation")
    if not math.isclose(state["orientation"], 0.0, abs_tol=1e-6):
        raise ScenarioFailure("instance entry orientation was not zero")
    return instance_id


def validate_reconnect_state(initial: dict[str, Any], reconnect: dict[str, Any]) -> int:
    validate_inside_state(initial)
    return validate_inside_state(reconnect)


def validate_full_relogin_state(initial: dict[str, Any], relogin: dict[str, Any]) -> int:
    validate_inside_state(initial)
    return validate_inside_state(relogin)


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
                 and event.get("connection") == "realm"
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


def validate_solo_reset_evidence(evidence: dict[str, Any]) -> dict[str, Any]:
    facts = validate_reset_evidence(evidence)
    casts = [packet for packet in evidence.get("sent_packets", [])
             if isinstance(packet, dict) and packet.get("action") == "cast"
             and packet.get("connection") == "instance" and packet.get("opcode") == CMSG_CAST_SPELL
             and packet.get("action_index") is not None
             and packet.get("write_attempted") is True
             and packet.get("write_completed") is True and packet.get("write_error") is None]
    if len(casts) != 1:
        raise ScenarioFailure("solo reset requires one native Hearthstone cast before reset")
    teleports = [event for event in evidence.get("events", [])
                 if isinstance(event, dict) and event.get("connection") == "realm"
                 and event.get("opcode") == SMSG_NEW_WORLD]
    if len(teleports) != 1:
        raise ScenarioFailure("solo reset requires one native Hearthstone worldport")
    reset_request = next(packet for packet in evidence["sent_packets"]
                         if isinstance(packet, dict) and packet.get("action") == "reset_instances")
    if not (_sequence(casts[0]) < _sequence(teleports[0]) < _sequence(reset_request)):
        raise ScenarioFailure("Hearthstone worldport must precede the solo reset request")
    facts.update({
        "hearthstone_spell_id": HEARTHSTONE_SPELL_ID,
        "cast_sequence": casts[0].get("sequence"),
        "teleport_sequence": teleports[0].get("sequence"),
        "native_worldport": True,
    })
    return facts


def _run_case(identity: dict[str, Any], journal: dict[str, Any], realm_id: int,
              db: death_fixture.MariaDB) -> dict[str, Any]:
    if identity.get("scenario") == instance_soak.SCENARIO:
        return instance_soak.run_workload(identity, journal, realm_id, db, _run_case)
    if identity.get("scenario", SCENARIO) == GROUP_SCENARIO:
        return _run_group_case(identity, journal, realm_id, db)
    if identity.get("scenario") == DIFFICULTY_SCENARIO:
        return _run_difficulty_case(identity, journal, realm_id, db)
    if identity.get("scenario") == LOCK_SCENARIO:
        return _run_lock_case(identity, journal, realm_id, db)
    actor = next(item for item in journal["actors"] if item["name"] == ACTOR_NAME)
    relocated = False
    restored = False
    try:
        relocate_inside(db, journal, actor)
        relocated = True
        observations = []

        reset_pass = death_scenario.BotPass(actor, reset_plan(actor, realm_id), "instance-reset-001")
        reset_evidence = logout_scenario._run_single(reset_pass, identity)
        reset_facts = validate_solo_reset_evidence(reset_evidence)
        _, reset_state, reset_path = _save_position_snapshot(
            db, journal, actor, identity["capture"], reset_pass.label)
        validate_outside_state(actor, reset_state)
        observations.append(logout_scenario._observation(
            reset_pass, reset_evidence, reset_state, reset_path, reset_facts))

        relocate_inside(db, journal, actor)
        entry_pass = death_scenario.BotPass(actor, observer_plan(actor), "instance-entry-001")
        entry_evidence = logout_scenario._run_single(entry_pass, identity, observer=True)
        _, entry_state, entry_path = _save_position_snapshot(
            db, journal, actor, identity["capture"], entry_pass.label)
        instance_id = validate_inside_state(entry_state)
        observations.append(logout_scenario._observation(
            entry_pass, entry_evidence, entry_state, entry_path, {"instance_id": instance_id}))

        disconnect_pass = death_scenario.BotPass(
            actor, disconnect_observer_plan(actor), "instance-socket-reconnect-001")
        disconnect_evidence = logout_scenario._run_single(disconnect_pass, identity, observer=True)
        observations.append({
            "pass": disconnect_pass.label,
            "actor": actor["name"],
            "report_sha256": disconnect_pass.report_hash,
            "lifecycle": death_scenario._evidence_digest(disconnect_evidence),
            "disconnect_reconnect": True,
        })

        reconnect_pass = death_scenario.BotPass(actor, observer_plan(actor), "instance-reconnect-001")
        reconnect_evidence = logout_scenario._run_single(reconnect_pass, identity, observer=True)
        _, reconnect_state, reconnect_path = _save_position_snapshot(
            db, journal, actor, identity["capture"], reconnect_pass.label)
        reconnect_instance = validate_reconnect_state(entry_state, reconnect_state)
        observations.append(logout_scenario._observation(
            reconnect_pass, reconnect_evidence, reconnect_state, reconnect_path,
            {"reconnect_instance_id": reconnect_instance,
             "instance_id": reconnect_instance}))

        relogin_pass = death_scenario.BotPass(actor, observer_plan(actor), "instance-relogin-001")
        relogin_evidence = logout_scenario._run_single(relogin_pass, identity, observer=True)
        _, relogin_state, relogin_path = _save_position_snapshot(
            db, journal, actor, identity["capture"], relogin_pass.label)
        full_relogin_instance = validate_full_relogin_state(entry_state, relogin_state)
        observations.append(logout_scenario._observation(
            relogin_pass, relogin_evidence, relogin_state, relogin_path,
            {"full_relogin_instance_id": full_relogin_instance,
             "instance_id": full_relogin_instance}))

        restore_outside(db, journal, actor)
        restored = True
        _, final_state, _ = _save_position_snapshot(
            db, journal, actor, identity["capture"], "instance-final-outside-001")
        validate_outside_state(actor, final_state)
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


def member_invite_plan(member: dict[str, Any]) -> dict[str, Any]:
    return {
        "character_guid": int(member["guid"]), "timeout_ms": 45_000,
        "actions": [
            death_scenario._wait("realm", SMSG_PARTY_INVITE, 30_000),
            {"action": "party_invite_response", "accept": True},
            death_scenario._wait("realm", SMSG_PARTY_UPDATE, 30_000),
        ],
        "expect": [
            death_scenario._packet("realm", LOGOUT_COMPLETE),
        ],
        "finish_policy": "logout",
    }


def leader_invite_plan(leader: dict[str, Any], member: dict[str, Any],
                       realm_id: int) -> dict[str, Any]:
    return {
        "character_guid": int(leader["guid"]), "timeout_ms": 45_000,
        "actions": [
            {"action": "party_invite", "name": member["name"],
             "target": death_scenario.player_guid(member, realm_id)},
            death_scenario._wait("realm", SMSG_PARTY_UPDATE, 30_000),
        ],
        "expect": [
            death_scenario._packet("realm", LOGOUT_COMPLETE),
        ],
        "finish_policy": "logout",
    }


def member_occupied_plan(member: dict[str, Any]) -> dict[str, Any]:
    return {
        "character_guid": int(member["guid"]), "timeout_ms": 95_000,
        "actions": [{"action": "wait", "milliseconds": 90_000}],
        "expect": [death_scenario._packet("realm", SMSG_RESET_FAILED_NOTIFY)],
        "finish_policy": "disconnect",
    }


def reset_failed_plan(actor: dict[str, Any]) -> dict[str, Any]:
    body = INSTANCE_MAP.to_bytes(4, "little") + b"\x00"
    return {
        "character_guid": int(actor["guid"]), "timeout_ms": 10_000,
        "actions": [
            {"action": "reset_instances"},
            death_scenario._wait("realm", SMSG_INSTANCE_RESET_FAILED, 5_000),
        ],
        "expect": [
            death_scenario._packet("realm", SMSG_INSTANCE_RESET_FAILED, body),
            death_scenario._packet("realm", LOGOUT_COMPLETE),
        ],
        "finish_policy": "logout",
    }


def leave_group_plan(actor: dict[str, Any]) -> dict[str, Any]:
    return {
        "character_guid": int(actor["guid"]), "timeout_ms": 10_000,
        "actions": [
            {"action": "leave_group"},
            death_scenario._wait("realm", SMSG_PARTY_COMMAND_RESULT, 5_000),
        ],
        "expect": [death_scenario._packet("realm", LOGOUT_COMPLETE)],
        "finish_policy": "logout",
    }


def _typed_packets(evidence: dict[str, Any], action: str, connection: str,
                   opcode: int, body_hex: str) -> list[dict[str, Any]]:
    return [packet for packet in evidence.get("sent_packets", [])
            if isinstance(packet, dict) and packet.get("action") == action
            and packet.get("connection") == connection and packet.get("opcode") == opcode
            and packet.get("body_hex") == body_hex and packet.get("action_index") is not None
            and packet.get("write_attempted") is True and packet.get("write_completed") is True
            and packet.get("write_error") is None]


def _opcode_events(evidence: dict[str, Any], connection: str, opcode: int,
                   body_hex: str) -> list[dict[str, Any]]:
    return [event for event in evidence.get("events", [])
            if isinstance(event, dict) and event.get("connection") == connection
            and event.get("opcode") == opcode and event.get("body_hex") == body_hex]


def validate_group_formation_evidence(leader: dict[str, Any], member: dict[str, Any]) -> None:
    invites = [packet for packet in leader.get("sent_packets", [])
               if isinstance(packet, dict) and packet.get("action") == "party_invite"
               and packet.get("connection") == "realm" and packet.get("opcode") == 0x3604
               and packet.get("action_index") is not None
               and packet.get("write_attempted") is True
               and packet.get("write_completed") is True and packet.get("write_error") is None]
    if len(invites) != 1:
        raise ScenarioFailure("leader did not send exactly one reviewed party invite")
    if len(_typed_packets(member, "party_invite_response", "realm", 0x3606, "40")) != 1:
        raise ScenarioFailure("member did not accept exactly one reviewed party invite")
    for evidence, client in ((leader, "leader"), (member, "member")):
        updates = [event for event in evidence.get("events", [])
                   if isinstance(event, dict) and event.get("connection") == "realm"
                   and event.get("opcode") == SMSG_PARTY_UPDATE]
        if not updates:
            raise ScenarioFailure(f"{client} did not observe a party update")


def validate_reset_failed_evidence(evidence: dict[str, Any]) -> dict[str, Any]:
    requests = _typed_packets(evidence, "reset_instances", "instance", CMSG_RESET_INSTANCES, "")
    if len(requests) != 1:
        raise ScenarioFailure("occupied group reset requires exactly one typed reset request")
    body = (INSTANCE_MAP.to_bytes(4, "little") + b"\x00").hex()
    responses = _opcode_events(evidence, "realm", SMSG_INSTANCE_RESET_FAILED, body)
    all_responses = [event for event in evidence.get("events", [])
                     if isinstance(event, dict) and event.get("opcode") == SMSG_INSTANCE_RESET_FAILED]
    if len(responses) != 1 or len(all_responses) != 1 \
            or _sequence(responses[0]) <= _sequence(requests[0]):
        raise ScenarioFailure("occupied group reset requires one map-33 reason-zero failure")
    return {"request_sequence": requests[0].get("sequence"),
            "response_sequence": responses[0].get("sequence"), "reason": 0}


def validate_reset_failed_notify_evidence(evidence: dict[str, Any]) -> dict[str, Any]:
    notifications = _opcode_events(evidence, "realm", SMSG_RESET_FAILED_NOTIFY, "")
    all_notifications = [event for event in evidence.get("events", [])
                         if isinstance(event, dict) and event.get("opcode") == SMSG_RESET_FAILED_NOTIFY]
    if len(notifications) != 1 or len(all_notifications) != 1:
        raise ScenarioFailure("occupied member requires exactly one empty ResetFailedNotify")
    return {"notify_sequence": notifications[0].get("sequence"), "body_bytes": 0}


def validate_leave_group_evidence(evidence: dict[str, Any], expected_leader_name: str) -> dict[str, Any]:
    requests = [packet for packet in evidence.get("sent_packets", [])
                if isinstance(packet, dict) and packet.get("action") == "leave_group"]
    completed = _typed_packets(evidence, "leave_group", "realm", CMSG_LEAVE_GROUP, "00")
    if len(requests) != 1 or len(completed) != 1:
        raise ScenarioFailure("leader must send exactly one realm LeaveGroup with the party index absent")

    name = expected_leader_name.encode("utf-8")
    if not name or len(name) >= (1 << 9):
        raise ScenarioFailure("leader name cannot be represented by PartyCommandResult")
    # PartyCommandResult::Write emits Name(9), Command(4), Result(6), ResultData,
    # packed empty ResultGUID, then Name. PARTY_OP_LEAVE=2 and ERR_PARTY_RESULT_OK=0.
    header = ((len(name) << 15) | (2 << 11)).to_bytes(3, "big")
    expected_body = (header + bytes(6) + name).hex()
    results = [event for event in evidence.get("events", [])
               if isinstance(event, dict) and event.get("opcode") == SMSG_PARTY_COMMAND_RESULT]
    matching = _opcode_events(evidence, "realm", SMSG_PARTY_COMMAND_RESULT, expected_body)
    request_sequence = _sequence(completed[0])
    response_sequence = _sequence(matching[0]) if matching else -1
    if len(results) != 1 or len(matching) != 1 \
            or request_sequence < 0 or response_sequence <= request_sequence:
        raise ScenarioFailure(
            "leader leave requires one ordered successful PARTY_OP_LEAVE PartyCommandResult")
    return {"request_sequence": request_sequence, "response_sequence": response_sequence,
            "command": 2, "result": 0, "name": expected_leader_name}


def _group_rows_absent(db: death_fixture.MariaDB,
                       actors: tuple[dict[str, Any], dict[str, Any]]) -> bool:
    ids = ",".join(str(actor["guid"]) for actor in actors)
    leaders = db.execute(f"SELECT leaderGuid FROM groups WHERE leaderGuid IN ({ids})")
    members = db.execute(f"SELECT memberGuid FROM group_member WHERE memberGuid IN ({ids})")
    return not leaders and not members


def _wait_group_rows_absent(db: death_fixture.MariaDB,
                            actors: tuple[dict[str, Any], dict[str, Any]]) -> None:
    # Group::Disband queues its transaction asynchronously after the leave response.
    deadline = time.monotonic() + 5.0
    while True:
        if _group_rows_absent(db, actors):
            return
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise ScenarioFailure("accepted leader leave did not remove the scenario group rows")
        time.sleep(min(0.1, remaining))


def preflight_group(db: death_fixture.MariaDB, journal: dict[str, Any],
                    actors: tuple[dict[str, Any], dict[str, Any]]) -> dict[str, Any]:
    for actor in actors:
        _require_offline(db, journal, actor)
    if not _group_rows_absent(db, actors):
        raise ScenarioFailure("instance group scenario requires no preexisting reviewed group rows")
    return {"cleanup_allowed": True}


def cleanup_group(db: death_fixture.MariaDB,
                  actors: tuple[dict[str, Any], dict[str, Any]],
                  preflight: dict[str, Any]) -> None:
    if preflight.get("cleanup_allowed") is not True:
        raise ScenarioFailure("group cleanup requires a proven empty preflight")
    ids = ",".join(str(actor["guid"]) for actor in actors)
    db.transaction([
        f"DELETE FROM group_member WHERE memberGuid IN ({ids})",
        f"DELETE FROM groups WHERE leaderGuid IN ({ids})",
    ])
    if not _group_rows_absent(db, actors):
        raise ScenarioFailure("scenario-owned group rows were not removed")


def _group_snapshot(db: death_fixture.MariaDB, journal: dict[str, Any],
                    actor: dict[str, Any], capture: Path, label: str,
                    require_offline: bool, *, scenario_name: str = GROUP_SCENARIO,
                    epoch: int = 1) -> tuple[dict[str, Any], dict[str, Any], Path]:
    return _save_position_snapshot(
        db, journal, actor, capture, label,
        scenario_name=scenario_name, require_offline=require_offline, epoch=epoch)


def _run_group_case(identity: dict[str, Any], journal: dict[str, Any], realm_id: int,
                    db: death_fixture.MariaDB) -> dict[str, Any]:
    leader = next(item for item in journal["actors"] if item["name"] == ACTOR_NAME)
    member = next(item for item in journal["actors"] if item["name"] == MEMBER_NAME)
    actors = (leader, member)
    preflight = preflight_group(db, journal, actors)
    relocated = False
    observations = []
    try:
        member_invite = death_scenario.BotPass(
            member, member_invite_plan(member), "instance-group-member-invite-001")
        death_scenario.start_bot(
            member_invite, bot_exec=identity["bot_exec"],
            bot_config=identity["bot_config"], output=identity["capture"])
        leader_invite = death_scenario.BotPass(
            leader, leader_invite_plan(leader, member, realm_id),
            "instance-group-leader-invite-001")
        leader_evidence = logout_scenario._run_single(leader_invite, identity)
        member_evidence = death_scenario.finish_bot(
            member_invite, timeout_seconds=75, actor=member)
        validate_group_formation_evidence(leader_evidence, member_evidence)
        _, leader_form_state, leader_form_path = _group_snapshot(
            db, journal, leader, identity["capture"], leader_invite.label, True)
        _, member_form_state, member_form_path = _group_snapshot(
            db, journal, member, identity["capture"], member_invite.label, True)
        observations.extend([
            logout_scenario._observation(
                leader_invite, leader_evidence, leader_form_state, leader_form_path,
                {"group_formed": True}),
            logout_scenario._observation(
                member_invite, member_evidence, member_form_state, member_form_path,
                {"group_formed": True}),
        ])
        for actor in actors:
            relocate_inside(db, journal, actor)
        relocated = True

        member_pass = death_scenario.BotPass(
            member, member_occupied_plan(member), "instance-group-member-occupied-001")
        death_scenario.start_bot(
            member_pass, bot_exec=identity["bot_exec"],
            bot_config=identity["bot_config"], output=identity["capture"])
        if not member_pass.ready.wait(180):
            raise ScenarioFailure("occupied member did not become ready inside the group instance")
        leader_entry = death_scenario.BotPass(
            leader, observer_plan(leader), "instance-group-leader-entry-001")
        leader_entry_evidence = logout_scenario._run_single(leader_entry, identity, observer=True)
        _, leader_entry_state, leader_entry_path = _group_snapshot(
            db, journal, leader, identity["capture"], leader_entry.label, True)
        instance_id = validate_inside_state(leader_entry_state)
        _, member_inside_state, member_inside_path = _group_snapshot(
            db, journal, member, identity["capture"], "instance-group-member-inside-001", False)
        validate_inside_state(member_inside_state, instance_id, allow_unsaved_online=True)
        observations.append(logout_scenario._observation(
            leader_entry, leader_entry_evidence, leader_entry_state, leader_entry_path,
            {"instance_id": instance_id, "member_instance_id": instance_id,
             "member_snapshot_sha256": death_scenario._sha256(member_inside_path)}))

        restore_outside(db, journal, leader)
        failed_pass = death_scenario.BotPass(
            leader, reset_failed_plan(leader), "instance-group-reset-occupied-001")
        failed_evidence = logout_scenario._run_single(failed_pass, identity)
        failed_facts = validate_reset_failed_evidence(failed_evidence)
        _, failed_state, failed_path = _group_snapshot(
            db, journal, leader, identity["capture"], failed_pass.label, True)
        _, member_occupied_state, member_occupied_path = _group_snapshot(
            db, journal, member, identity["capture"], "instance-group-member-still-inside-001", False)
        validate_inside_state(member_occupied_state, instance_id, allow_unsaved_online=True)
        observations.append(logout_scenario._observation(
            failed_pass, failed_evidence, failed_state, failed_path,
            {**failed_facts, "member_still_inside": True,
             "member_snapshot_sha256": death_scenario._sha256(member_occupied_path)}))

        member_evidence = death_scenario.finish_bot(
            member_pass, timeout_seconds=150, actor=member)
        notify_facts = validate_reset_failed_notify_evidence(member_evidence)
        _, member_offline_state, member_offline_path = _group_snapshot(
            db, journal, member, identity["capture"], member_pass.label, True)
        validate_inside_state(member_offline_state, instance_id)
        observations.append(logout_scenario._observation(
            member_pass, member_evidence, member_offline_state, member_offline_path,
            {**notify_facts, "not_expelled": True, "instance_id": instance_id}))

        success_pass = death_scenario.BotPass(
            leader, group_reset_plan(leader), "instance-group-reset-empty-001")
        success_evidence = logout_scenario._run_single(success_pass, identity)
        success_facts = validate_reset_evidence(success_evidence)
        _, success_state, success_path = _group_snapshot(
            db, journal, leader, identity["capture"], success_pass.label, True)
        validate_outside_state(leader, success_state)
        observations.append(logout_scenario._observation(
            success_pass, success_evidence, success_state, success_path,
            {**success_facts, "empty_after_member_offline": True}))

        restore_outside(db, journal, member)
        leave_pass = death_scenario.BotPass(
            leader, leave_group_plan(leader), "instance-group-leave-001")
        leave_evidence = logout_scenario._run_single(leave_pass, identity)
        leave_facts = validate_leave_group_evidence(leave_evidence, leader["name"])
        _, leave_state, leave_path = _group_snapshot(
            db, journal, leader, identity["capture"], leave_pass.label, True)
        validate_outside_state(leader, leave_state)
        observations.append(logout_scenario._observation(
            leave_pass, leave_evidence, leave_state, leave_path,
            {**leave_facts, "leave_group_accepted": True}))
        _wait_group_rows_absent(db, actors)
        _, final_member_state, final_member_path = _group_snapshot(
            db, journal, member, identity["capture"], "instance-group-member-outside-001", True)
        validate_outside_state(member, final_member_state)
        return {
            "scenario": GROUP_SCENARIO, "epoch": 1, "scenario_observation": "passed",
            "observations": observations,
            "final_state": {"leader": leave_state, "member": final_member_state},
            "group_cleanup": {
                "preflight_absence_proven": True,
                "server_leave_rows_absent": True,
                "scenario_group_rows_removed": True,
                "other_groups_untouched": True,
                "lock_rows_mutated": False,
                "member_snapshot_sha256": death_scenario._sha256(final_member_path),
            },
            "position_restoration": {"leader_outside": True, "member_outside": True},
            "parity_assessed": False,
        }
    except BaseException:
        for pass_ in list(death_scenario.ACTIVE_BOTS):
            death_scenario._stop_owned(pass_)
        if relocated:
            for actor in actors:
                try:
                    death_scenario._poll_actor_offline(db, journal, actor)
                    restore_outside(db, journal, actor)
                except Exception:
                    pass
        try:
            cleanup_group(db, actors, preflight)
        except Exception:
            pass
        raise


def difficulty_plan(actor: dict[str, Any]) -> dict[str, Any]:
    return {
        "character_guid": int(actor["guid"]), "timeout_ms": 10_000,
        "actions": [
            {"action": "set_dungeon_difficulty", "difficulty_id": 2},
            death_scenario._wait("realm", SMSG_SET_DUNGEON_DIFFICULTY, 5_000),
        ],
        "expect": [death_scenario._packet("realm", LOGOUT_COMPLETE)],
        "finish_policy": "logout",
    }


def member_difficulty_plan(member: dict[str, Any]) -> dict[str, Any]:
    return {
        "character_guid": int(member["guid"]), "timeout_ms": 95_000,
        "actions": [{"action": "wait", "milliseconds": 90_000}],
        "expect": [death_scenario._packet("realm", LOGOUT_COMPLETE)],
        "finish_policy": "logout",
    }


def validate_difficulty_evidence(evidence: dict[str, Any]) -> dict[str, Any]:
    body = (2).to_bytes(4, "little", signed=True).hex()
    requests = _typed_packets(evidence, "set_dungeon_difficulty", "instance", 0x3684, body)
    if len(requests) != 1:
        raise ScenarioFailure("difficulty change requires exactly one typed SetDungeonDifficulty")
    responses = _opcode_events(evidence, "realm", SMSG_SET_DUNGEON_DIFFICULTY, body)
    if not responses or _sequence(responses[0]) <= _sequence(requests[0]):
        raise ScenarioFailure("difficulty change requires a body-2 response after its request")
    return {"request_sequence": requests[0].get("sequence"),
            "response_sequence": responses[0].get("sequence"), "difficulty_id": 2}


def validate_no_reset_events(*evidences: dict[str, Any]) -> None:
    reset_opcodes = {SMSG_INSTANCE_RESET, SMSG_INSTANCE_RESET_FAILED, SMSG_RESET_FAILED_NOTIFY}
    for evidence in evidences:
        if any(isinstance(event, dict) and event.get("opcode") in reset_opcodes
               for event in evidence.get("events", [])):
            raise ScenarioFailure("occupied difficulty change emitted a reset packet")


def validate_login_difficulty(evidence: dict[str, Any], receiver: dict[str, Any],
                              leader: dict[str, Any], peer: dict[str, Any],
                              realm_id: int) -> dict[str, Any]:
    body = (2).to_bytes(4, "little", signed=True).hex()
    responses = [event for event in evidence.get("events", [])
                 if isinstance(event, dict) and event.get("opcode") == SMSG_SET_DUNGEON_DIFFICULTY
                 and event.get("connection") == "realm"
                 and event.get("body_hex") == body]
    if not responses:
        raise ScenarioFailure("fresh login requires a persisted body-2 dungeon difficulty")
    return instance_party_update.validate_login_party_update(
        evidence, receiver, leader, peer, realm_id)


def require_group_difficulty(db: death_fixture.MariaDB,
                             actors: tuple[dict[str, Any], dict[str, Any]]) -> dict[str, Any]:
    leader, member = actors
    rows = death_fixture.MariaDB._rows(db.execute(
        "SELECT guid,leaderGuid,difficulty FROM groups "
        f"WHERE leaderGuid={int(leader['guid'])}"), 3)
    if len(rows) != 1 or rows[0][1] != str(leader["guid"]) or rows[0][2] != "2":
        raise ScenarioFailure("scenario group did not persist difficulty 2 under its exact leader")
    group_guid = rows[0][0]
    member_rows = death_fixture.MariaDB._rows(db.execute(
        f"SELECT memberGuid FROM group_member WHERE guid={group_guid} ORDER BY memberGuid"), 1)
    expected = sorted([str(leader["guid"]), str(member["guid"])])
    if [row[0] for row in member_rows] != expected:
        raise ScenarioFailure("scenario group member rows are not exactly leader and member")
    return {"group_guid": int(group_guid), "leader_guid": leader["guid"],
            "member_guid": member["guid"], "difficulty": 2,
            "members": [int(value) for value in expected]}


def _difficulty_snapshot(db: death_fixture.MariaDB, journal: dict[str, Any],
                         actor: dict[str, Any], capture: Path, label: str,
                         require_offline: bool, epoch: int) -> tuple[dict[str, Any], dict[str, Any], Path]:
    return _group_snapshot(
        db, journal, actor, capture, label, require_offline,
        scenario_name=DIFFICULTY_SCENARIO, epoch=epoch)


def _run_difficulty_case(identity: dict[str, Any], journal: dict[str, Any], realm_id: int,
                         db: death_fixture.MariaDB) -> dict[str, Any]:
    epoch = identity["epoch"]
    leader = next(item for item in journal["actors"] if item["name"] == ACTOR_NAME)
    member = next(item for item in journal["actors"] if item["name"] == MEMBER_NAME)
    actors = (leader, member)
    preflight = preflight_group(db, journal, actors) if epoch == 1 else {"cleanup_allowed": True}
    relocated = False
    observations = []
    group_state = None
    try:
        if epoch == 2:
            group_state = require_group_difficulty(db, actors)
            for actor in actors:
                _require_offline(db, journal, actor)
        if epoch == 1:
            member_invite = death_scenario.BotPass(
                member, member_invite_plan(member), "instance-difficulty-member-invite-001")
            death_scenario.start_bot(
                member_invite, bot_exec=identity["bot_exec"],
                bot_config=identity["bot_config"], output=identity["capture"])
            leader_invite = death_scenario.BotPass(
                leader, leader_invite_plan(leader, member, realm_id),
                "instance-difficulty-leader-invite-001")
            leader_evidence = logout_scenario._run_single(leader_invite, identity)
            member_evidence = death_scenario.finish_bot(
                member_invite, timeout_seconds=75, actor=member)
            validate_group_formation_evidence(leader_evidence, member_evidence)
            _, leader_form_state, leader_form_path = _difficulty_snapshot(
                db, journal, leader, identity["capture"], leader_invite.label, True, epoch)
            _, member_form_state, member_form_path = _difficulty_snapshot(
                db, journal, member, identity["capture"], member_invite.label, True, epoch)
            observations.extend([
                logout_scenario._observation(
                    leader_invite, leader_evidence, leader_form_state, leader_form_path,
                    {"group_formed": True}),
                logout_scenario._observation(
                    member_invite, member_evidence, member_form_state, member_form_path,
                    {"group_formed": True}),
            ])
            for actor in actors:
                relocate_inside(db, journal, actor)
            relocated = True
            member_pass = death_scenario.BotPass(
                member, member_difficulty_plan(member), "instance-difficulty-member-occupied-001")
        else:
            member_pass = None

        if epoch == 1:
            death_scenario.start_bot(
                member_pass, bot_exec=identity["bot_exec"],
                bot_config=identity["bot_config"], output=identity["capture"])
            if not member_pass.ready.wait(180):
                raise ScenarioFailure("difficulty member did not become ready inside the instance")
            leader_entry = death_scenario.BotPass(
                leader, observer_plan(leader), "instance-difficulty-leader-entry-001")
            leader_entry_evidence = logout_scenario._run_single(leader_entry, identity, observer=True)
            _, leader_entry_state, leader_entry_path = _difficulty_snapshot(
                db, journal, leader, identity["capture"], leader_entry.label, True, epoch)
            instance_id = validate_inside_state(leader_entry_state)
            _, member_inside_state, member_inside_path = _difficulty_snapshot(
                db, journal, member, identity["capture"], "instance-difficulty-member-inside-001",
                False, epoch)
            validate_inside_state(member_inside_state, instance_id, allow_unsaved_online=True)
            observations.append(logout_scenario._observation(
                leader_entry, leader_entry_evidence, leader_entry_state, leader_entry_path,
                {"instance_id": instance_id, "member_instance_id": instance_id,
                 "member_snapshot_sha256": death_scenario._sha256(member_inside_path)}))

            restore_outside(db, journal, leader)
            difficulty_pass = death_scenario.BotPass(
                leader, difficulty_plan(leader), "instance-difficulty-change-001")
            difficulty_evidence = logout_scenario._run_single(difficulty_pass, identity)
            difficulty_facts = validate_difficulty_evidence(difficulty_evidence)
            _, difficulty_state, difficulty_path = _difficulty_snapshot(
                db, journal, leader, identity["capture"], difficulty_pass.label, True, epoch)
            _, member_occupied_state, member_occupied_path = _difficulty_snapshot(
                db, journal, member, identity["capture"], "instance-difficulty-member-still-inside-001",
                False, epoch)
            validate_inside_state(member_occupied_state, instance_id, allow_unsaved_online=True)
            observations.append(logout_scenario._observation(
                difficulty_pass, difficulty_evidence, difficulty_state, difficulty_path,
                {**difficulty_facts, "member_still_inside": True,
                 "member_snapshot_sha256": death_scenario._sha256(member_occupied_path)}))
            member_evidence = death_scenario.finish_bot(
                member_pass, timeout_seconds=150, actor=member)
            validate_no_reset_events(difficulty_evidence, member_evidence)
            _, member_offline_state, member_offline_path = _difficulty_snapshot(
                db, journal, member, identity["capture"], member_pass.label, True, epoch)
            validate_inside_state(member_offline_state, instance_id)
            observations.append(logout_scenario._observation(
                member_pass, member_evidence, member_offline_state, member_offline_path,
                {"not_expelled": True, "instance_id": instance_id}))
            group_state = require_group_difficulty(db, actors)
            return {
                "scenario": DIFFICULTY_SCENARIO, "epoch": 1, "scenario_observation": "passed",
                "observations": observations,
                "final_state": {
                    "leader": difficulty_state, "member": member_offline_state,
                    "group": group_state,
                },
                "restart_persistence": {
                    "positions_retained_for_epoch_2": True,
                    "group_retained_for_epoch_2": True,
                    "difficulty_id": 2,
                },
                "parity_assessed": False,
            }

        member_login = death_scenario.BotPass(
            member, observer_plan(member), "instance-difficulty-member-restart-001")
        member_login_evidence = logout_scenario._run_single(member_login, identity, observer=True)
        member_login_facts = validate_login_difficulty(
            member_login_evidence, member, leader, leader, realm_id)
        _, member_login_state, member_login_path = _difficulty_snapshot(
            db, journal, member, identity["capture"], member_login.label, True, epoch)
        current_instance = validate_inside_state(member_login_state)
        observations.append(logout_scenario._observation(
            member_login, member_login_evidence, member_login_state, member_login_path,
            {"restart_login": True, "instance_id": current_instance,
             **member_login_facts}))

        relocate_inside(db, journal, leader)
        leader_login = death_scenario.BotPass(
            leader, observer_plan(leader), "instance-difficulty-leader-restart-001")
        leader_login_evidence = logout_scenario._run_single(leader_login, identity, observer=True)
        leader_login_facts = validate_login_difficulty(
            leader_login_evidence, leader, leader, member, realm_id)
        instance_party_update.validate_same_relogin_group(
            member_login_facts, leader_login_facts)
        _, leader_login_state, leader_login_path = _difficulty_snapshot(
            db, journal, leader, identity["capture"], leader_login.label, True, epoch)
        validate_inside_state(leader_login_state, current_instance)
        observations.append(logout_scenario._observation(
            leader_login, leader_login_evidence, leader_login_state, leader_login_path,
            {"restart_login": True, "current_shared_instance": True,
             "instance_id": current_instance, **leader_login_facts}))

        for actor in actors:
            restore_outside(db, journal, actor)
        leave_pass = death_scenario.BotPass(
            leader, leave_group_plan(leader), "instance-difficulty-leave-001")
        leave_evidence = logout_scenario._run_single(leave_pass, identity)
        leave_facts = validate_leave_group_evidence(leave_evidence, leader["name"])
        _wait_group_rows_absent(db, actors)
        _, leave_state, leave_path = _difficulty_snapshot(
            db, journal, leader, identity["capture"], leave_pass.label, True, epoch)
        validate_outside_state(leader, leave_state)
        observations.append(logout_scenario._observation(
            leave_pass, leave_evidence, leave_state, leave_path,
            {**leave_facts, "leave_group_accepted": True,
             "group_rows_absent_after_leave": True}))
        _, final_member_state, final_member_path = _difficulty_snapshot(
            db, journal, member, identity["capture"], "instance-difficulty-member-outside-001",
            True, epoch)
        validate_outside_state(member, final_member_state)
        return {
            "scenario": DIFFICULTY_SCENARIO, "epoch": 2, "scenario_observation": "passed",
            "observations": observations,
            "final_state": {"leader": leave_state, "member": final_member_state},
            "restart_persistence": {
                "difficulty_preserved_across_restart": True,
                "current_shared_instance": current_instance,
                "old_instance_id_not_required": True,
            },
            "group_cleanup": {
                "scenario_group_rows_removed": True,
                "server_leave_rows_absent": True,
                "other_groups_untouched": True,
                "lock_rows_mutated": False,
                "member_snapshot_sha256": death_scenario._sha256(final_member_path),
            },
            "position_restoration": {"leader_outside": True, "member_outside": True},
            "parity_assessed": False,
        }
    except BaseException:
        for pass_ in list(death_scenario.ACTIVE_BOTS):
            death_scenario._stop_owned(pass_)
        if relocated or epoch == 2:
            for actor in actors:
                try:
                    death_scenario._poll_actor_offline(db, journal, actor)
                    restore_outside(db, journal, actor)
                except Exception:
                    pass
        try:
            cleanup_group(db, actors, preflight)
        except Exception:
            pass
        raise


def member_lock_plan(member: dict[str, Any], accept: bool) -> dict[str, Any]:
    actions: list[dict[str, Any]] = [
        death_scenario._wait("realm", SMSG_PENDING_RAID_LOCK, 30_000),
        death_scenario._wait("realm", SMSG_PENDING_RAID_LOCK, 5_000),
        {"action": "instance_lock_response", "accept": accept},
    ]
    if accept:
        actions.append(death_scenario._wait("realm", SMSG_INSTANCE_SAVE_CREATED, 10_000))
        actions.append(death_scenario._wait("realm", SMSG_CALENDAR_RAID_LOCKOUT_ADDED, 10_000))
        expectations = [death_scenario._packet("realm", SMSG_INSTANCE_SAVE_CREATED, b"\x00")]
    else:
        actions.append({"action": "wait", "milliseconds": 1_000})
        expectations = [death_scenario._packet("realm", LOGOUT_COMPLETE)]
    return {
        "character_guid": int(member["guid"]), "timeout_ms": 70_000,
        "actions": actions, "expect": expectations,
        "finish_policy": "logout",
    }


def leader_auto_lock_plan(leader: dict[str, Any]) -> dict[str, Any]:
    return {
        "character_guid": int(leader["guid"]), "timeout_ms": 130_000,
        "actions": [
            death_scenario._wait("realm", SMSG_PENDING_RAID_LOCK, 30_000),
            death_scenario._wait("realm", SMSG_CALENDAR_RAID_LOCKOUT_ADDED, 90_000),
        ],
        "expect": [
            death_scenario._packet("realm", SMSG_INSTANCE_SAVE_CREATED, b"\x00"),
        ],
        "finish_policy": "logout",
    }


def pending_lock_body(*, extending: bool) -> bytes:
    return ((60_000).to_bytes(4, "little", signed=True)
            + (1).to_bytes(4, "little")
            + (b"\x80" if extending else b"\x00"))


def raid_info_plan(actor: dict[str, Any]) -> dict[str, Any]:
    return {
        "character_guid": int(actor["guid"]), "timeout_ms": 15_000,
        "actions": [
            {"action": "request_raid_info"},
            death_scenario._wait("realm", SMSG_INSTANCE_INFO, 5_000),
        ],
        "expect": [death_scenario._packet("realm", LOGOUT_COMPLETE)],
        "finish_policy": "logout",
    }


def lock_reset_plan(actor: dict[str, Any]) -> dict[str, Any]:
    return {
        "character_guid": int(actor["guid"]), "timeout_ms": 10_000,
        "actions": [
            {"action": "reset_instances"},
            {"action": "wait", "milliseconds": 2_000},
        ],
        "expect": [death_scenario._packet("realm", LOGOUT_COMPLETE)],
        "finish_policy": "logout",
    }


def lock_occupant_plan(actor: dict[str, Any]) -> dict[str, Any]:
    return {
        "character_guid": int(actor["guid"]), "timeout_ms": 25_000,
        "actions": [{"action": "wait", "milliseconds": 18_000}],
        "expect": [death_scenario._packet("realm", LOGOUT_COMPLETE)],
        "finish_policy": "logout",
    }


def validate_calendar_lockout_added(evidence: dict[str, Any], save_event: dict[str, Any],
                                    expected_instance_id: int | None) -> dict[str, Any]:
    events = [event for event in evidence.get("events", [])
              if isinstance(event, dict) and event.get("opcode") == SMSG_CALENDAR_RAID_LOCKOUT_ADDED]
    if len(events) != 1 or events[0].get("connection") != "realm" \
            or _sequence(events[0]) <= _sequence(save_event):
        raise ScenarioFailure("calendar raid-lockout update must follow save-created on realm")
    try:
        body = bytes.fromhex(events[0].get("body_hex", ""))
        instance_id, packed_time, map_id, difficulty_id, remaining = struct.unpack("<QIiIi", body)
    except (ValueError, struct.error):
        raise ScenarioFailure("calendar raid-lockout update has an invalid wire body") from None
    year = (packed_time >> 24) & 0x1F
    month = (packed_time >> 20) & 0x0F
    month_day = (packed_time >> 14) & 0x3F
    week_day = (packed_time >> 11) & 0x07
    hour = (packed_time >> 6) & 0x1F
    minute = packed_time & 0x3F
    flags = (packed_time >> 29) & 0x03
    if (map_id != LOCK_MAP or difficulty_id != LOCK_DIFFICULTY or remaining <= 0
            or (expected_instance_id is not None and instance_id != expected_instance_id)
            or packed_time & (1 << 31) or flags != 3
            or not (year < 31 and month < 12 and month_day < 31 and week_day < 7
                    and hour < 24 and minute < 60)):
        raise ScenarioFailure("calendar raid-lockout identity, remaining time, or packed time is invalid")
    return {"connection": "realm", "sequence": _sequence(events[0]),
            "instance_id": instance_id, "map_id": map_id,
            "difficulty_id": difficulty_id, "time_remaining_seconds": remaining,
            "server_time": {"year_since_2000": year, "month_zero_based": month,
                            "day_zero_based": month_day, "weekday": week_day,
                            "hour": hour, "minute": minute, "flags": flags}}


def entry_lock_prompts(evidence: dict[str, Any], extending: bool) -> list[int]:
    prompts = [event for event in evidence.get("events", [])
               if isinstance(event, dict) and event.get("opcode") == SMSG_PENDING_RAID_LOCK]
    if (not prompts or prompts[0].get("connection") != "realm"
            or prompts[0].get("body_hex") != pending_lock_body(extending=extending).hex()
            or _sequence(prompts[0]) < 0):
        raise ScenarioFailure("ICC entry did not publish the expected 60-second raid lock prompt")
    # This fixture starts with the attached lock already expired and extended.
    expiry_body = struct.pack("<BIIB", 5, LOCK_MAP, LOCK_DIFFICULTY, 0).hex()
    expirations = [event for event in evidence.get("events", [])
                   if isinstance(event, dict) and event.get("opcode") == SMSG_RAID_INSTANCE_MESSAGE
                   and (event.get("body_hex") or "").startswith("05")]
    if (len(prompts) != 2 or len(expirations) != 1
            or expirations[0].get("connection") != "realm"
            or expirations[0].get("body_hex") != expiry_body
            or prompts[1].get("connection") != "realm"
            or prompts[1].get("body_hex") != pending_lock_body(extending=True).hex()
            or not _sequence(prompts[0]) < _sequence(expirations[0]) < _sequence(prompts[1])):
        raise ScenarioFailure("expired ICC lock requires an expiry announcement then an extending prompt")
    return [_sequence(prompt) for prompt in prompts]


def validate_lock_response_evidence(evidence: dict[str, Any], *, accept: bool,
                                    extending: bool = False,
                                    expected_instance_id: int | None = None) -> dict[str, Any]:
    prompts = entry_lock_prompts(evidence, extending)
    response = _typed_packets(evidence, "instance_lock_response", "realm",
                              CMSG_INSTANCE_LOCK_RESPONSE, "80" if accept else "00")
    if len(response) != 1 or _sequence(response[0]) <= prompts[-1]:
        raise ScenarioFailure("ICC lock response must follow the extending prompt on realm")
    saves = _opcode_events(evidence, "realm", SMSG_INSTANCE_SAVE_CREATED, "00")
    all_saves = [event for event in evidence.get("events", [])
                 if isinstance(event, dict) and event.get("opcode") == SMSG_INSTANCE_SAVE_CREATED]
    if accept:
        if len(saves) != 1 or len(all_saves) != 1 \
                or _sequence(saves[0]) <= _sequence(response[0]):
            raise ScenarioFailure("accepted ICC bind did not publish one ordered save-created opcode")
        calendar_facts = validate_calendar_lockout_added(
            evidence, saves[0], expected_instance_id=expected_instance_id)
    else:
        calendars = [event for event in evidence.get("events", [])
                     if isinstance(event, dict)
                     and event.get("opcode") == SMSG_CALENDAR_RAID_LOCKOUT_ADDED]
        if all_saves or calendars:
            raise ScenarioFailure("rejected ICC bind unexpectedly published lockout state")
    return {"prompt_sequence": prompts[0], "prompt_sequences": prompts,
            "response_sequence": _sequence(response[0]), "accepted": accept,
            "save_created_sequence": _sequence(saves[0]) if saves else None,
            "calendar_lockout": calendar_facts if accept else None}


def validate_leader_auto_lock_evidence(evidence: dict[str, Any],
                                       expected_instance_id: int | None = None) -> dict[str, Any]:
    prompts = entry_lock_prompts(evidence, True)
    saves = _opcode_events(evidence, "realm", SMSG_INSTANCE_SAVE_CREATED, "00")
    all_saves = [event for event in evidence.get("events", [])
                 if isinstance(event, dict) and event.get("opcode") == SMSG_INSTANCE_SAVE_CREATED]
    if len(saves) != 1 or len(all_saves) != 1 or _sequence(saves[0]) <= prompts[-1]:
        raise ScenarioFailure("expired extended leader lock did not auto-confirm after its prompt")
    if any(packet.get("action") == "instance_lock_response"
           for packet in evidence.get("sent_packets", []) if isinstance(packet, dict)):
        raise ScenarioFailure("leader automatic confirmation unexpectedly used a client response")
    calendar_facts = validate_calendar_lockout_added(
        evidence, saves[0], expected_instance_id=expected_instance_id)
    return {"prompt_sequence": prompts[0], "prompt_sequences": prompts,
            "save_created_sequence": _sequence(saves[0]),
            "client_response_sent": False, "automatic_confirmation": True,
            "calendar_lockout": calendar_facts}


def validate_raid_info_evidence(evidence: dict[str, Any], actor: dict[str, Any],
                                instance_id: int, expected_mask: int) -> dict[str, Any]:
    requests = _typed_packets(evidence, "request_raid_info", "realm",
                              CMSG_REQUEST_RAID_INFO, "")
    all_events = [event for event in evidence.get("events", [])
                  if isinstance(event, dict) and event.get("opcode") == SMSG_INSTANCE_INFO]
    if len(requests) != 1:
        raise ScenarioFailure("post-restart bind retrieval requires one typed RaidInfo request and response")
    responses = [event for event in all_events
                 if _sequence(event) > _sequence(requests[0])]
    if len(responses) != 1:
        raise ScenarioFailure("RaidInfo response was not observed once after the requested action")
    event = responses[0]
    if event.get("connection") != "realm":
        raise ScenarioFailure("RaidInfo response arrived on the wrong connection")
    try:
        body = bytes.fromhex(event.get("body_hex", ""))
    except ValueError:
        raise ScenarioFailure("RaidInfo response body is not hexadecimal") from None
    if len(body) < 4:
        raise ScenarioFailure("RaidInfo response is shorter than its lock count")
    count = int.from_bytes(body[:4], "little")
    row_size = 25
    if len(body) != 4 + count * row_size:
        raise ScenarioFailure("RaidInfo response count does not match its wire rows")
    locks = []
    for index in range(count):
        offset = 4 + index * row_size
        map_id, difficulty_id, row_instance_id, remaining, completed_mask = struct.unpack_from(
            "<IIQII", body, offset)
        flags = body[offset + 24]
        locks.append({"map_id": map_id, "difficulty_id": difficulty_id,
                      "instance_id": row_instance_id, "time_remaining_seconds": remaining,
                      "completed_encounters_mask": completed_mask, "flags": flags})
    matches = [lock for lock in locks
               if lock["map_id"] == LOCK_MAP and lock["difficulty_id"] == LOCK_DIFFICULTY
               and lock["instance_id"] == instance_id]
    if (len(matches) != 1 or matches[0]["time_remaining_seconds"] <= 0
            or matches[0]["completed_encounters_mask"] != expected_mask
            or matches[0]["flags"] != 0x80):
        raise ScenarioFailure("RaidInfo did not reload the active same-ID ICC lock and saved mask")
    return {"actor_guid": int(actor["guid"]), "request_sequence": _sequence(requests[0]),
            "response_sequence": _sequence(event), "entry": matches[0],
            "server_reported_lock_count": count}


def validate_lock_inside_state(state: dict[str, Any], instance_id: int,
                               allow_unsaved_online: bool = False) -> None:
    if state.get("map") != LOCK_MAP or not _near(state, LOCK_POSITION) or not math.isclose(
            state["orientation"], 0.0, abs_tol=1e-6):
        raise ScenarioFailure("ICC entry did not retain map 631, the chosen instance, and native position")
    if state.get("instance_id") == 0:
        if allow_unsaved_online and state.get("online") == 1:
            return
        raise ScenarioFailure("offline ICC snapshot has no allocated instance ID")
    if state["instance_id"] != instance_id:
        raise ScenarioFailure("ICC snapshot changed the chosen instance ID")


def validate_lock_reset_evidence(leader: dict[str, Any], member: dict[str, Any]) -> None:
    requests = _typed_packets(leader, "reset_instances", "instance", CMSG_RESET_INSTANCES, "")
    if len(requests) != 1:
        raise ScenarioFailure("non-new ICC lock requires exactly one completed reset request")
    validate_no_reset_events(leader, member)


def _icc_save_payload(data: str) -> dict[str, Any]:
    try:
        payload = json.loads(data)
        states = payload["BossStates"]
    except (ValueError, KeyError, TypeError):
        raise ScenarioFailure("ICC lock save data is malformed") from None
    if (not isinstance(payload, dict) or payload.get("Header") != "ICCv1"
            or not isinstance(states, list) or len(states) != 13
            or any(type(state) is not int or state not in range(4) for state in states)):
        raise ScenarioFailure("ICC lock save data does not contain the reviewed ICCv1 13-boss state")
    return payload


def _sha256_text(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def _lock_rows(db: death_fixture.MariaDB,
               actors: tuple[dict[str, Any], dict[str, Any]]) -> list[list[str]]:
    guids = ",".join(str(int(actor["guid"])) for actor in actors)
    rows = death_fixture.MariaDB._rows(db.execute(
        "SELECT guid,mapId,lockId,instanceId,difficulty,data,completedEncountersMask,"
        "entranceWorldSafeLocId,expiryTime,extended FROM character_instance_lock "
        f"WHERE guid IN ({guids}) AND mapId={LOCK_MAP} AND lockId={LOCK_ID} ORDER BY guid"), 10)
    return rows


def _instance_row(db: death_fixture.MariaDB, instance_id: int) -> list[str]:
    rows = death_fixture.MariaDB._rows(db.execute(
        "SELECT instanceId,data,completedEncountersMask,entranceWorldSafeLocId FROM instance "
        f"WHERE instanceId={instance_id}"), 4)
    if len(rows) != 1:
        raise ScenarioFailure("reserved ICC instance row is missing or ambiguous")
    return rows[0]


def relocate_lock_inside(db: death_fixture.MariaDB, journal: dict[str, Any],
                         actor: dict[str, Any], instance_id: int) -> dict[str, Any]:
    _require_offline(db, journal, actor)
    db.transaction([_position_update(actor, LOCK_MAP, instance_id, LOCK_POSITION)])
    return _require_database_position(db, actor, LOCK_MAP, instance_id, LOCK_POSITION)


def prepare_lock_entry(db: death_fixture.MariaDB, journal: dict[str, Any],
                       actors: tuple[dict[str, Any], dict[str, Any]], instance_id: int,
                       expiry_time: int) -> dict[str, Any]:
    leader, member = actors
    for actor in actors:
        _require_offline(db, journal, actor)
    reservation = journal.get("instance_lock_reservation")
    expected_reservation = {
        "map_id": LOCK_MAP, "lock_id": LOCK_ID, "instance_id": instance_id,
        "difficulty_id": LOCK_DIFFICULTY,
        "actor_guids": [int(actor["guid"]) for actor in actors],
        "seed_guid": int(leader["guid"]),
    }
    if (not isinstance(reservation, dict)
            or any(reservation.get(key) != value for key, value in expected_reservation.items())
            or reservation.get("ownership_state") != "owned"
            or reservation.get("seed_expiry_time") != expiry_time):
        raise ScenarioFailure("ICC setup has no matching owned stopped-world lock reservation")
    db.transaction([
        f"UPDATE groups SET groupType=2,raidDifficulty={LOCK_DIFFICULTY},legacyRaidDifficulty=3 "
        f"WHERE leaderGuid={int(leader['guid'])}",
        _position_update(leader, LOCK_MAP, instance_id, LOCK_POSITION),
        _position_update(member, LOCK_MAP, instance_id, LOCK_POSITION),
    ])
    for actor in actors:
        _require_database_position(db, actor, LOCK_MAP, instance_id, LOCK_POSITION)
    expected_lock = [str(leader["guid"]), str(LOCK_MAP), str(LOCK_ID), str(instance_id),
                     str(LOCK_DIFFICULTY), ICC_LEADER_LOCK_SEED_DATA, "1", "0",
                     str(expiry_time), "1"]
    if _lock_rows(db, actors) != [expected_lock]:
        raise ScenarioFailure("seeded expired extended leader lock did not persist exactly")
    if _instance_row(db, instance_id) != [str(instance_id), ICC_SHARED_SAVE_DATA, "1", "0"]:
        raise ScenarioFailure("seeded ICC instance save row did not persist exactly")
    return require_lock_fixture(db, actors, instance_id, expiry_time)


def require_lock_group(db: death_fixture.MariaDB,
                       actors: tuple[dict[str, Any], dict[str, Any]]) -> dict[str, Any]:
    leader, member = actors
    rows = death_fixture.MariaDB._rows(db.execute(
        "SELECT guid,groupType,raidDifficulty,legacyRaidDifficulty FROM groups "
        f"WHERE leaderGuid={int(leader['guid'])}"), 4)
    if len(rows) != 1 or rows[0][1:] != ["2", "4", "3"]:
        raise ScenarioFailure("ICC scenario group is not raid difficulty 4 with legacy difficulty 3")
    group_guid = rows[0][0]
    member_rows = death_fixture.MariaDB._rows(db.execute(
        f"SELECT memberGuid FROM group_member WHERE guid={group_guid} ORDER BY memberGuid"), 1)
    expected = sorted([str(leader["guid"]), str(member["guid"])])
    if [row[0] for row in member_rows] != expected:
        raise ScenarioFailure("ICC scenario group members are not exactly leader and member")
    return {"group_guid": int(group_guid), "group_type": 2,
            "raid_difficulty": 4, "legacy_raid_difficulty": 3,
            "members": [int(value) for value in expected]}


def require_lock_fixture(db: death_fixture.MariaDB,
                         actors: tuple[dict[str, Any], dict[str, Any]], instance_id: int,
                         expiry_time: int) -> dict[str, Any]:
    leader, member = actors
    expected = [str(leader["guid"]), str(LOCK_MAP), str(LOCK_ID), str(instance_id),
                str(LOCK_DIFFICULTY), ICC_LEADER_LOCK_SEED_DATA, "1", "0",
                str(expiry_time), "1"]
    if _lock_rows(db, actors) != [expected]:
        raise ScenarioFailure("expired extended ICC leader lock changed before native confirmation")
    if _lock_rows(db, (member,)):
        raise ScenarioFailure("member already has an ICC lock before its fresh bind")
    if _instance_row(db, instance_id) != [str(instance_id), ICC_SHARED_SAVE_DATA, "1", "0"]:
        raise ScenarioFailure("ICC shared instance save row changed before binding")
    if expiry_time >= int(time.time()):
        raise ScenarioFailure("leader ICC lock is not expired before automatic confirmation")
    return {
        "instance_id": instance_id,
        "map_id": LOCK_MAP,
        "lock_id": LOCK_ID,
        "difficulty_id": LOCK_DIFFICULTY,
        "expiry_time": expiry_time,
        "completed_encounters_mask": 1,
        "extended": True,
        "data": ICC_SHARED_SAVE_DATA,
        "group": require_lock_group(db, actors),
    }


def require_bound_lock_rows(db: death_fixture.MariaDB,
                            actors: tuple[dict[str, Any], dict[str, Any]],
                            instance_id: int, *, leader_renewed: bool) -> dict[str, Any]:
    leader, member = actors
    rows = _lock_rows(db, actors)
    expected_guids = sorted([str(leader["guid"]), str(member["guid"])])
    if len(rows) != 2 or [row[0] for row in rows] != expected_guids:
        raise ScenarioFailure("both actors must have exactly one committed ICC character lock")
    by_guid = {row[0]: row for row in rows}
    for row in rows:
        if (row[1:5] != [str(LOCK_MAP), str(LOCK_ID), str(instance_id), str(LOCK_DIFFICULTY)]
                or not row[8].isdecimal()):
            raise ScenarioFailure("committed ICC lock keys or encounter state changed")
        _icc_save_payload(row[5])
    leader_row = by_guid[str(leader["guid"])]
    member_row = by_guid[str(member["guid"])]
    if leader_renewed:
        if (leader_row[6:8] != ["1", "0"] or leader_row[5] != ICC_SHARED_SAVE_DATA
                or leader_row[9] != "0" or int(leader_row[8]) <= int(time.time())):
            raise ScenarioFailure("expired extended leader lock was not renewed")
    elif (leader_row[6:8] != ["1", "0"] or leader_row[5] != ICC_LEADER_LOCK_SEED_DATA
          or leader_row[9] != "1" or int(leader_row[8]) > int(time.time())):
        raise ScenarioFailure("leader lock changed before its 60-second confirmation")
    if (member_row[6:8] != ["1", "0"] or member_row[5] != ICC_SHARED_SAVE_DATA
            or member_row[9] != "0" or int(member_row[8]) <= int(time.time())):
        raise ScenarioFailure("accepted member lock was not active and unextended")
    if leader_renewed and leader_row[5] != member_row[5]:
        raise ScenarioFailure("shared ICC character locks have different save payloads")
    instance_row = _instance_row(db, instance_id)
    _icc_save_payload(instance_row[1])
    if (instance_row[1] != ICC_SHARED_SAVE_DATA or instance_row[2:] != ["1", "0"]
            or instance_row[1] != member_row[5]):
        raise ScenarioFailure("ICC instance save data differs from committed character locks")
    return {"instance_id": instance_id, "leader_lock": leader_row,
            "member_lock": member_row, "instance_row": instance_row,
            "data": leader_row[5] if leader_renewed else member_row[5],
            "expiry_time": int(leader_row[8]),
            "completed_encounters_mask": int(leader_row[6]),
            "entrance_world_safe_loc_id": int(leader_row[7]),
            "extended": leader_row[9] == "1"}


def save_post_bind_snapshot(identity: dict[str, Any], journal: dict[str, Any],
                            snapshot: dict[str, Any]) -> None:
    journal["icc_post_bind_snapshot"] = {
        "instance_id": snapshot["instance_id"],
        "lock_rows": [snapshot["leader_lock"], snapshot["member_lock"]],
        "instance_row": snapshot["instance_row"],
    }
    death_fixture.validate_journal(journal, identity["journal_path"])
    death_fixture.write_journal(identity["journal_path"], journal, create=False)


def _lock_snapshot(db: death_fixture.MariaDB, journal: dict[str, Any],
                   actor: dict[str, Any], capture: Path, label: str,
                   require_offline: bool, epoch: int) -> tuple[dict[str, Any], dict[str, Any], Path]:
    return _group_snapshot(
        db, journal, actor, capture, label, require_offline,
        scenario_name=LOCK_SCENARIO, epoch=epoch)


def _run_lock_case(identity: dict[str, Any], journal: dict[str, Any], realm_id: int,
                   db: death_fixture.MariaDB) -> dict[str, Any]:
    epoch = identity["epoch"]
    leader = next(item for item in journal["actors"] if item["name"] == ACTOR_NAME)
    member = next(item for item in journal["actors"] if item["name"] == MEMBER_NAME)
    actors = (leader, member)
    preflight = preflight_group(db, journal, actors) if epoch == 1 else {"cleanup_allowed": True}
    reservation = journal.get("instance_lock_reservation")
    if not isinstance(reservation, dict) or reservation.get("ownership_state") != "owned":
        raise ScenarioFailure("ICC scenario has no owned stopped-world instance reservation")
    instance_id = int(reservation["instance_id"])
    expiry_time = int(reservation["seed_expiry_time"])
    relocated = False
    group_owned = epoch != 1
    observations = []
    try:
        if epoch == 1:
            seed_lock_rows = _lock_rows(db, actors)
            expected_seed = [str(leader["guid"]), str(LOCK_MAP), str(LOCK_ID),
                             str(instance_id), str(LOCK_DIFFICULTY),
                             ICC_LEADER_LOCK_SEED_DATA, "1", "0", str(expiry_time), "1"]
            if seed_lock_rows != [expected_seed] \
                    or _instance_row(db, instance_id) != [
                        str(instance_id), ICC_SHARED_SAVE_DATA, "1", "0"]:
                raise ScenarioFailure("stopped-world ICC seed rows changed before epoch 1")
            member_invite = death_scenario.BotPass(
                member, member_invite_plan(member), "instance-lock-member-invite-001")
            death_scenario.start_bot(
                member_invite, bot_exec=identity["bot_exec"],
                bot_config=identity["bot_config"], output=identity["capture"])
            leader_invite = death_scenario.BotPass(
                leader, leader_invite_plan(leader, member, realm_id),
                "instance-lock-leader-invite-001")
            leader_evidence = logout_scenario._run_single(leader_invite, identity)
            member_evidence = death_scenario.finish_bot(
                member_invite, timeout_seconds=75, actor=member)
            validate_group_formation_evidence(leader_evidence, member_evidence)
            group_owned = True
            _, leader_form_state, leader_form_path = _lock_snapshot(
                db, journal, leader, identity["capture"], leader_invite.label, True, epoch)
            _, member_form_state, member_form_path = _lock_snapshot(
                db, journal, member, identity["capture"], member_invite.label, True, epoch)
            observations.extend([
                logout_scenario._observation(
                    leader_invite, leader_evidence, leader_form_state, leader_form_path,
                    {"group_formed": True}),
                logout_scenario._observation(
                    member_invite, member_evidence, member_form_state, member_form_path,
                    {"group_formed": True}),
            ])
            seed_facts = prepare_lock_entry(db, journal, actors, instance_id, expiry_time)
            relocated = True
            _, leader_seed_state, leader_seed_path = _lock_snapshot(
                db, journal, leader, identity["capture"], "instance-lock-leader-seed-001",
                True, epoch)
            _, member_seed_state, member_seed_path = _lock_snapshot(
                db, journal, member, identity["capture"], "instance-lock-member-seed-001",
                True, epoch)
            validate_lock_inside_state(leader_seed_state, instance_id)
            validate_lock_inside_state(member_seed_state, instance_id)
            observations.extend([
                {"pass": "instance-lock-leader-seed", "actor": leader["name"],
                 "state": leader_seed_state,
                 "snapshot_sha256": death_scenario._sha256(leader_seed_path)},
                {"pass": "instance-lock-member-seed", "actor": member["name"],
                 "state": member_seed_state,
                 "snapshot_sha256": death_scenario._sha256(member_seed_path)},
            ])
            return {
                "scenario": LOCK_SCENARIO, "epoch": 1, "scenario_observation": "passed",
                "observations": observations, "seed_facts": seed_facts,
                "final_state": {"leader": leader_seed_state, "member": member_seed_state},
                "restart_persistence": {
                    "positions_retained_for_epoch_2": True,
                    "group_retained_for_epoch_2": True,
                    "owned_expired_extended_leader_lock_retained": True,
                },
                "parity_assessed": False,
            }

        if epoch == 2:
            require_lock_fixture(db, actors, instance_id, expiry_time)
            require_lock_group(db, actors)
            for actor in actors:
                _require_offline(db, journal, actor)
            member_reject = death_scenario.BotPass(
                member, member_lock_plan(member, False), "instance-lock-member-reject-001")
            reject_evidence = logout_scenario._run_single(member_reject, identity)
            reject_facts = validate_lock_response_evidence(
                reject_evidence, accept=False, expected_instance_id=instance_id)
            _, rejected_state, rejected_path = _lock_snapshot(
                db, journal, member, identity["capture"], member_reject.label, True, epoch)
            if _lock_rows(db, (member,)):
                raise ScenarioFailure("rejected ICC bind unexpectedly committed a member lock")
            observations.append(logout_scenario._observation(
                member_reject, reject_evidence, rejected_state, rejected_path,
                {**reject_facts, "member_lock_absent": True,
                 "member_bind_rejected_without_lockout_publication": True}))

            restore_outside(db, journal, member)
            relocate_lock_inside(db, journal, member, instance_id)
            member_accept = death_scenario.BotPass(
                member, member_lock_plan(member, True), "instance-lock-member-accept-001")
            accept_evidence = logout_scenario._run_single(member_accept, identity)
            accept_facts = validate_lock_response_evidence(
                accept_evidence, accept=True, expected_instance_id=instance_id)
            _, member_bound_state, member_bound_path = _lock_snapshot(
                db, journal, member, identity["capture"], member_accept.label, True, epoch)
            validate_lock_inside_state(member_bound_state, instance_id)
            pre_auto_snapshot = require_bound_lock_rows(
                db, actors, instance_id, leader_renewed=False)
            observations.append(logout_scenario._observation(
                member_accept, accept_evidence, member_bound_state, member_bound_path,
                {**accept_facts, "fresh_member_lock_committed": True,
                 "instance_id": instance_id,
                 "member_data_sha256": _sha256_text(pre_auto_snapshot["member_lock"][5])}))

            leader_auto = death_scenario.BotPass(
                leader, leader_auto_lock_plan(leader), "instance-lock-leader-auto-confirm-001")
            leader_auto_evidence = logout_scenario._run_single(leader_auto, identity)
            auto_facts = validate_leader_auto_lock_evidence(
                leader_auto_evidence, expected_instance_id=instance_id)
            _, leader_auto_state, leader_auto_path = _lock_snapshot(
                db, journal, leader, identity["capture"], leader_auto.label, True, epoch)
            validate_lock_inside_state(leader_auto_state, instance_id)
            bound_snapshot = require_bound_lock_rows(db, actors, instance_id, leader_renewed=True)
            if bound_snapshot["expiry_time"] <= expiry_time:
                raise ScenarioFailure("leader automatic confirmation did not renew the expired lock")
            save_post_bind_snapshot(identity, journal, bound_snapshot)
            observations.append(logout_scenario._observation(
                leader_auto, leader_auto_evidence, leader_auto_state, leader_auto_path,
                {**auto_facts, "leader_lock_renewed": True, "instance_id": instance_id,
                 "leader_data_sha256": _sha256_text(bound_snapshot["leader_lock"][5])}))
            return {
                "scenario": LOCK_SCENARIO, "epoch": 2, "scenario_observation": "passed",
                "observations": observations,
                "post_bind_snapshot": journal["icc_post_bind_snapshot"],
                "bind_state": {"member_rejection_observed": True,
                               "member_acceptance_observed": True,
                               "leader_automatic_confirmation_observed": True,
                               "expired_leader_lock_renewed": True,
                               "both_character_locks_committed": True,
                               "instance_id": instance_id},
                "restart_persistence": {"bind_created_before_epoch_3_restart": True},
                "parity_assessed": False,
            }

        expected_snapshot = journal.get("icc_post_bind_snapshot")
        if not isinstance(expected_snapshot, dict):
            raise ScenarioFailure("third ICC epoch has no durable post-bind snapshot")
        if expected_snapshot.get("instance_id") != instance_id:
            raise ScenarioFailure("ICC instance identity changed in the durable post-bind snapshot")
        persisted = require_bound_lock_rows(db, actors, instance_id, leader_renewed=True)
        if (persisted["leader_lock"] != expected_snapshot["lock_rows"][0]
                or persisted["member_lock"] != expected_snapshot["lock_rows"][1]
                or persisted["instance_row"] != expected_snapshot["instance_row"]):
            raise ScenarioFailure("post-restart ICC lock identity or serialized content changed")
        require_lock_group(db, actors)
        for actor, expected_row in zip(actors, expected_snapshot["lock_rows"], strict=True):
            pass_ = death_scenario.BotPass(
                actor, raid_info_plan(actor), f"instance-lock-raid-info-{actor['name']}-001")
            evidence = logout_scenario._run_single(pass_, identity)
            info_facts = validate_raid_info_evidence(
                evidence, actor, instance_id, int(expected_row[6]))
            if any(event.get("opcode") == SMSG_PENDING_RAID_LOCK
                   for event in evidence.get("events", []) if isinstance(event, dict)):
                raise ScenarioFailure("reloaded ICC lock unexpectedly prompted for another bind")
            _, state, path = _lock_snapshot(
                db, journal, actor, identity["capture"], pass_.label, True, epoch)
            validate_lock_inside_state(state, instance_id)
            observations.append(logout_scenario._observation(
                pass_, evidence, state, path,
                {**info_facts, "loaded_lock_after_restart": True,
                 "same_instance_id": True}))

        restore_outside(db, journal, leader)
        occupant_pass = death_scenario.BotPass(
            member, lock_occupant_plan(member), "instance-lock-reset-member-occupant-001")
        death_scenario.start_bot(
            occupant_pass, bot_exec=identity["bot_exec"],
            bot_config=identity["bot_config"], output=identity["capture"])
        if not occupant_pass.ready.wait(180):
            raise ScenarioFailure("member did not become ready in the persisted ICC lock")
        reset_pass = death_scenario.BotPass(
            leader, lock_reset_plan(leader), "instance-lock-reset-after-bind-001")
        reset_evidence = logout_scenario._run_single(reset_pass, identity)
        occupant_evidence = death_scenario.finish_bot(
            occupant_pass, timeout_seconds=60, actor=member)
        validate_lock_reset_evidence(reset_evidence, occupant_evidence)
        _, reset_state, reset_path = _lock_snapshot(
            db, journal, leader, identity["capture"], reset_pass.label, True, epoch)
        validate_outside_state(leader, reset_state)
        _, occupied_state, occupied_path = _lock_snapshot(
            db, journal, member, identity["capture"], occupant_pass.label, True, epoch)
        validate_lock_inside_state(occupied_state, instance_id)
        after_reset = require_bound_lock_rows(db, actors, instance_id, leader_renewed=True)
        if (after_reset["leader_lock"] != expected_snapshot["lock_rows"][0]
                or after_reset["member_lock"] != expected_snapshot["lock_rows"][1]
                or after_reset["instance_row"] != expected_snapshot["instance_row"]):
            raise ScenarioFailure("manual reset changed the post-bind ICC lock snapshot")
        observations.append(logout_scenario._observation(
            reset_pass, reset_evidence, reset_state, reset_path,
            {"cannot_reset": True, "member_inside_during_request": True,
             "both_character_locks_preserved": True,
             "member_snapshot_sha256": death_scenario._sha256(occupied_path),
             "instance_id": instance_id}))
        observations.append(logout_scenario._observation(
            occupant_pass, occupant_evidence, occupied_state, occupied_path,
            {"remained_inside_same_instance": True, "instance_id": instance_id}))

        for actor in actors:
            restore_outside(db, journal, actor)
        leave_pass = death_scenario.BotPass(
            leader, leave_group_plan(leader), "instance-lock-leave-group-after-bind-001")
        leave_evidence = logout_scenario._run_single(leave_pass, identity)
        leave_facts = validate_leave_group_evidence(leave_evidence, leader["name"])
        _, leave_state, leave_path = _lock_snapshot(
            db, journal, leader, identity["capture"], leave_pass.label, True, epoch)
        validate_outside_state(leader, leave_state)
        _wait_group_rows_absent(db, actors)
        observations.append(logout_scenario._observation(
            leave_pass, leave_evidence, leave_state, leave_path,
            {**leave_facts, "native_group_leave_completed": True,
             "scenario_group_rows_removed": True}))
        _, final_member_state, final_member_path = _lock_snapshot(
            db, journal, member, identity["capture"], "instance-lock-member-outside-after-bind-001",
            True, epoch)
        validate_outside_state(member, final_member_state)
        final_snapshot = require_bound_lock_rows(db, actors, instance_id, leader_renewed=True)
        if (final_snapshot["leader_lock"] != expected_snapshot["lock_rows"][0]
                or final_snapshot["member_lock"] != expected_snapshot["lock_rows"][1]
                or final_snapshot["instance_row"] != expected_snapshot["instance_row"]):
            raise ScenarioFailure("group cleanup changed the durable ICC lock snapshot")
        return {
            "scenario": LOCK_SCENARIO, "epoch": 3, "scenario_observation": "passed",
            "observations": observations,
            "restart_persistence": {
                "same_instance_id": instance_id,
                "leader_lock_same_content": True,
                "member_lock_same_content": True,
                "instance_save_same_content": True,
                "raid_info_loaded_both_actor_locks": True,
                "reset_preserved_post_bind_snapshot": True,
            },
            "group_cleanup": {"native_leave_group": True,
                              "scenario_group_rows_removed": True},
            "position_restoration": {"leader_outside": True, "member_outside": True,
                                     "member_snapshot_sha256": death_scenario._sha256(final_member_path)},
            "final_state": {"leader": leave_state, "member": final_member_state},
            "parity_assessed": False,
        }
    except BaseException:
        for pass_ in list(death_scenario.ACTIVE_BOTS):
            death_scenario._stop_owned(pass_)
        if relocated or epoch in {2, 3}:
            for actor in actors:
                try:
                    death_scenario._poll_actor_offline(db, journal, actor)
                    restore_outside(db, journal, actor)
                except Exception:
                    pass
        if group_owned:
            try:
                cleanup_group(db, actors, preflight)
            except Exception:
                pass
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
    try:
        epoch = int(os.environ["RUSTYCORE_DEATH_EPOCH_INDEX"])
    except ValueError:
        raise ScenarioFailure("instance scenario epoch index is invalid") from None
    allowed_epochs = ({1, 2, 3} if scenario == LOCK_SCENARIO else
                      {1, 2} if scenario == DIFFICULTY_SCENARIO else {1})
    if scenario not in SCENARIOS or epoch not in allowed_epochs \
            or os.environ["RUSTYCORE_DEATH_EPOCH"] != f"epoch-{epoch:03d}":
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
    return {"scenario": scenario, "epoch": epoch, "capture": capture, "report_path": report,
            "journal_path": Path(os.environ["RUSTYCORE_DEATH_JOURNAL"]),
            "bot_config": Path(os.environ["RUSTYCORE_DEATH_BOT_CONFIG"]),
            "world_config": Path(os.environ["RUSTYCORE_DEATH_WORLD_CONFIG"]),
            "bot_exec": bot_exec, "backend": os.environ["RUSTYCORE_CAPTURE_BACKEND"]}


def _failure_report(exc: BaseException, scenario: str | None,
                    epoch: int = 1) -> dict[str, Any]:
    report = death_scenario._failure_report(exc, scenario, epoch)
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
            "scenario_dependency_sha256": {
                "tools/qa/instance_party_update.py": death_scenario._sha256(
                    Path(__file__).with_name("instance_party_update.py").resolve()),
            },
            "target_database": journal["target_database"],
            "native_collection": "recorded by outer capture wrapper",
            "parity_assessed": False,
        })
        if identity["scenario"] == instance_soak.SCENARIO:
            report["scenario_dependency_sha256"]["tools/qa/instance_soak.py"] = (
                death_scenario._sha256(Path(instance_soak.__file__)))
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
                    os.environ.get("RUSTYCORE_DEATH_SCENARIO"),
                    identity.get("epoch", 1) if identity else 1))
            except Exception:
                pass
        return 70


if __name__ == "__main__":
    raise SystemExit(run())
