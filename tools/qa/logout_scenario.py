#!/usr/bin/env python3
"""One guarded #54 logout scenario pass using the death-fixture capture wrapper."""

from __future__ import annotations

from datetime import datetime
import hashlib
import math
import os
from pathlib import Path
import re
import struct
import time
from typing import Any

import death_fixture
import death_scenario
from death_corpse_target import _packed_guid


SCENARIOS = {
    "logout-combat-denial": "Qadeathmob",
    "logout-fall-denial": "Qadeathfall",
    "logout-freeze-denial": "Qadeathcast",
    "logout-cancel": "Qadeathfall",
    "logout-timed": "Qadeathfall",
    "logout-disconnect": "Qadeathfall",
    "logout-repeated": "Qadeathfall",
    "logout-rbac-grant": "Qadeathcast",
    "logout-rbac-revoke": "Qadeathreinc",
    "logout-resting": "Qadeathfall",
}
CMSG_LOGOUT_REQUEST = 0x34D6
CMSG_LOGOUT_CANCEL = 0x34D8
LOGOUT_RESPONSE = 0x2683
LOGOUT_COMPLETE = 0x2684
LOGOUT_CANCEL_ACK = 0x2685
ATTACKER_STATE_UPDATE = 0x2952
MOVE_ROOT = 0x2DF9
MOVE_UNROOT = 0x2DFA
STAND_STATE_UPDATE = 0x271C
UPDATE_OBJECT = 0x27CB
UNIT_FLAG_STUNNED = 0x0004_0000
PLAYER_FLAGS_RESTING = 0x20
FREEZE_SPELL = 9454
LOGOUT_DELAY_MS = 20_000
DENIAL_OBSERVE_MS = 20_500
DISCONNECT_ONLINE_CHECK_SECONDS = 2
DISCONNECT_OFFLINE_GRACE_SECONDS = 5
INSTANT_SCENARIOS = {"logout-rbac-grant", "logout-resting"}
VARIANT_SCENARIOS = {*INSTANT_SCENARIOS, "logout-rbac-revoke"}
RBAC_FIXTURES = {
    "logout-rbac-grant": (9, [(1, 1)]),
    "logout-rbac-revoke": (12, [(1, 0), (194, 0)]),
}
ScenarioFailure = death_scenario.ScenarioFailure


def _packet(connection: str, opcode: int, body: bytes = b"") -> dict[str, Any]:
    return death_scenario._packet(connection, opcode, body)


def _wait(connection: str, opcode: int, timeout_ms: int = 5_000) -> dict[str, Any]:
    return death_scenario._wait(connection, opcode, timeout_ms)


def _request() -> dict[str, Any]:
    return {"action": "logout_request", "idle_logout": False}


def _response(reason: int, instant: bool = False) -> dict[str, Any]:
    return _packet("instance", LOGOUT_RESPONSE, reason.to_bytes(4, "little", signed=True)
                   + (b"\x80" if instant else b"\x00"))


def _fall_start(actor: dict[str, Any]) -> dict[str, Any]:
    home = actor["home"]
    return {"action": "movement", "kind": "heartbeat", "time_ms": 1,
            "x": home[0], "y": home[1], "z": home[2] + 100.0, "orientation": 0.0,
            "fall": {"fall_time_ms": 1, "z_speed": 0.0, "sin_angle": 0.0,
                     "cos_angle": 1.0, "xy_speed": 0.0}}


def plan_for(scenario: str, actor: dict[str, Any]) -> dict[str, Any]:
    if scenario not in SCENARIOS or actor.get("name") != SCENARIOS[scenario]:
        raise ScenarioFailure("logout scenario actor is outside the bounded case set")
    actions: list[dict[str, Any]]
    expected: list[dict[str, Any]]
    if scenario == "logout-combat-denial":
        actions = [
            {"action": "attack_swing", "target": {"entry": 6, "map_id": 0,
             "near_xyz": [-8783.05, -161.565, 82.0352], "radius": 40.0}},
            _wait("instance", ATTACKER_STATE_UPDATE, 10_000),
            _request(), _wait("instance", LOGOUT_RESPONSE),
        ]
        expected = [_response(1)]
        timeout_ms = 15_000
    elif scenario == "logout-fall-denial":
        actions = [_fall_start(actor), _request(), _wait("instance", LOGOUT_RESPONSE),
                   {"action": "wait", "milliseconds": DENIAL_OBSERVE_MS}]
        expected = [_response(3)]
        timeout_ms = 30_000
    elif scenario == "logout-freeze-denial":
        actions = [_request(), _wait("instance", LOGOUT_RESPONSE),
                   {"action": "wait", "milliseconds": DENIAL_OBSERVE_MS}]
        expected = [_response(2)]
        timeout_ms = 30_000
    elif scenario in INSTANT_SCENARIOS:
        actions = [_request(), _wait("instance", LOGOUT_RESPONSE, 15_000),
                   _wait("realm", LOGOUT_COMPLETE)]
        expected = [_response(0, True), _packet("realm", LOGOUT_COMPLETE)]
        timeout_ms = 20_000
    elif scenario in {"logout-cancel", "logout-rbac-revoke"}:
        actions = [_request(), _wait("instance", LOGOUT_RESPONSE),
                   _wait("instance", MOVE_ROOT), {"action": "logout_cancel"},
                   _wait("instance", LOGOUT_CANCEL_ACK), _wait("instance", MOVE_UNROOT)]
        expected = [_response(0), _packet("instance", LOGOUT_CANCEL_ACK)]
        timeout_ms = 15_000
    elif scenario == "logout-timed":
        actions = [_request(), _wait("instance", LOGOUT_RESPONSE),
                   _wait("instance", MOVE_ROOT), _wait("realm", LOGOUT_COMPLETE, 30_000)]
        expected = [_response(0), _packet("realm", LOGOUT_COMPLETE)]
        timeout_ms = 35_000
    elif scenario == "logout-disconnect":
        actions = [_request(), _wait("instance", LOGOUT_RESPONSE),
                   _wait("instance", MOVE_ROOT)]
        expected = [_response(0)]
        timeout_ms = 10_000
    elif scenario == "logout-repeated":
        actions = [
            _request(), _wait("instance", LOGOUT_RESPONSE), _wait("instance", MOVE_ROOT),
            {"action": "wait", "milliseconds": 12_000},
            _request(), _wait("instance", LOGOUT_RESPONSE), _wait("instance", MOVE_ROOT),
            {"action": "wait", "milliseconds": 10_000},
            {"action": "logout_cancel"}, _wait("instance", LOGOUT_CANCEL_ACK),
            _wait("instance", MOVE_UNROOT),
        ]
        expected = [_response(0), _response(0), _packet("instance", LOGOUT_CANCEL_ACK)]
        timeout_ms = 35_000
    else:
        raise AssertionError("bounded logout scenario was not exhaustively mapped")
    return {"character_guid": int(actor["guid"]), "timeout_ms": timeout_ms,
            "actions": actions, "expect": expected, "finish_policy": "disconnect"}


def relogin_plan(actor: dict[str, Any]) -> dict[str, Any]:
    return {"character_guid": int(actor["guid"]), "observe_only": True,
            "timeout_ms": 3_000, "actions": [], "expect": [],
            "finish_policy": "disconnect"}


def freeze_statements(actor: dict[str, Any], realm_id: int) -> list[str]:
    guid = death_scenario.player_guid(actor, realm_id)
    raw_guid = struct.pack("<QQ", guid["low"], guid["high"]).hex()
    empty_guid = "00" * 16
    key = f"UNHEX('{raw_guid}'),UNHEX('{empty_guid}')"
    owner = int(actor["guid"])
    return [
        f"DELETE FROM character_aura_effect WHERE guid={owner} AND spell={FREEZE_SPELL}",
        f"DELETE FROM character_aura WHERE guid={owner} AND spell={FREEZE_SPELL}",
        "INSERT INTO character_aura (guid,casterGuid,itemGuid,spell,effectMask,"
        "recalculateMask,difficulty,stackCount,maxDuration,remainTime,remainCharges,"
        f"castItemId,castItemLevel) VALUES ({owner},{key},{FREEZE_SPELL},1,0,0,1,-1,-1,0,0,-1)",
        "INSERT INTO character_aura_effect (guid,casterGuid,itemGuid,spell,effectMask,"
        f"effectIndex,amount,baseAmount) VALUES ({owner},{key},{FREEZE_SPELL},1,0,0,0)",
    ]


def _seed_freeze(scenario: str, db: death_fixture.MariaDB,
                 actor: dict[str, Any], realm_id: int) -> None:
    if scenario != "logout-freeze-denial":
        raise ScenarioFailure("fixture mutation is restricted to the freeze denial case")
    db.transaction(freeze_statements(actor, realm_id))


def _rbac_identity(scenario: str, actor: dict[str, Any]) -> tuple[int, list[tuple[int, int]]]:
    expected_account, permissions = RBAC_FIXTURES[scenario]
    account = int(actor["account_id"])
    if account != expected_account:
        raise ScenarioFailure("RBAC logout fixture actor account is outside its bounded identity")
    return account, permissions


def _rbac_select(account: int, permission: int) -> str:
    return ("SELECT accountId,permissionId,granted,realmId FROM auth.rbac_account_permissions "
            f"WHERE accountId={account} AND permissionId={permission} AND realmId=-1")


def _access_select(account: int) -> str:
    return ("SELECT AccountID,SecurityLevel,RealmID FROM auth.account_access "
            f"WHERE AccountID={account} AND RealmID=-1")


def _resting_identity(db: death_fixture.MariaDB, actor: dict[str, Any]) -> int:
    guid = int(actor["guid"])
    rows = death_fixture.MariaDB._rows(db.execute(
        f"SELECT guid,account,playerFlags FROM characters WHERE guid={guid}"), 3)
    if len(rows) != 1 or rows[0][0] != str(guid) \
            or rows[0][1] != str(actor["account_id"]):
        raise ScenarioFailure("resting logout fixture character identity is missing or ambiguous")
    try:
        return int(rows[0][2])
    except ValueError:
        raise ScenarioFailure("resting logout fixture playerFlags is invalid") from None


def _require_absent(db: death_fixture.MariaDB, query: str) -> None:
    if db.execute(query):
        raise ScenarioFailure("logout variant fixture row already exists; refusing mutation")


def _access_security(db: death_fixture.MariaDB, account: int) -> int | None:
    rows = death_fixture.MariaDB._rows(db.execute(_access_select(account)), 3)
    if not rows:
        return None
    if len(rows) != 1 or rows[0][0] != str(account) or rows[0][2] != "-1":
        raise ScenarioFailure("revoke logout account access identity is missing or ambiguous")
    try:
        security = int(rows[0][1])
    except ValueError:
        raise ScenarioFailure("revoke logout account access security is invalid") from None
    if security != 0:
        raise ScenarioFailure("revoke logout account access security must be zero")
    return security


def _prepare_variant(scenario: str, db: death_fixture.MariaDB,
                     actor: dict[str, Any]) -> dict[str, Any]:
    if scenario in RBAC_FIXTURES:
        account, permissions = _rbac_identity(scenario, actor)
        for permission, _granted in permissions:
            _require_absent(db, _rbac_select(account, permission))
        original_access = _access_security(db, account) \
            if scenario == "logout-rbac-revoke" else None
        statements = [
            "INSERT INTO auth.rbac_account_permissions "
            "(accountId,permissionId,granted,realmId) "
            f"VALUES ({account},{permission},{granted},-1)"
            for permission, granted in permissions
        ]
        if scenario == "logout-rbac-revoke":
            if original_access is None:
                statements.insert(0, "INSERT INTO auth.account_access "
                                     "(AccountID,SecurityLevel,RealmID) "
                                     f"VALUES ({account},1,-1)")
            else:
                statements.insert(0, "UPDATE auth.account_access SET SecurityLevel=1 "
                                     f"WHERE AccountID={account} AND RealmID=-1")
        db.transaction(statements)
        state = {"account_id": account}
        if scenario == "logout-rbac-revoke":
            state["original_access_security"] = original_access
        return state
    if scenario == "logout-resting":
        original = _resting_identity(db, actor)
        db.transaction([f"UPDATE characters SET playerFlags=playerFlags|{PLAYER_FLAGS_RESTING} "
                        f"WHERE guid={int(actor['guid'])} AND account={int(actor['account_id'])}"])
        return {"original_player_flags": original}
    raise ScenarioFailure("fixture mutation is restricted to the bounded logout variants")


def _restore_variant(scenario: str, db: death_fixture.MariaDB,
                     actor: dict[str, Any], state: dict[str, Any]) -> None:
    if scenario in RBAC_FIXTURES:
        account, permissions = _rbac_identity(scenario, actor)
        statements = [
            "DELETE FROM auth.rbac_account_permissions "
            f"WHERE accountId={account} AND permissionId={permission} "
            f"AND granted={granted} AND realmId=-1"
            for permission, granted in permissions
        ]
        if scenario == "logout-rbac-revoke":
            original_access = state["original_access_security"]
            if original_access is None:
                statements.append("DELETE FROM auth.account_access "
                                  f"WHERE AccountID={account} AND RealmID=-1")
            else:
                statements.append(f"UPDATE auth.account_access SET SecurityLevel={original_access} "
                                  f"WHERE AccountID={account} AND RealmID=-1")
        db.transaction(statements)
        for permission, _granted in permissions:
            _require_absent(db, _rbac_select(account, permission))
        if scenario == "logout-rbac-revoke" \
                and _access_security(db, account) != state["original_access_security"]:
            raise ScenarioFailure("revoke logout account access restoration failed")
        return
    if scenario == "logout-resting":
        original = int(state["original_player_flags"])
        db.transaction([f"UPDATE characters SET playerFlags={original} "
                        f"WHERE guid={int(actor['guid'])} AND account={int(actor['account_id'])}"])
        if _resting_identity(db, actor) != original:
            raise ScenarioFailure("resting logout fixture playerFlags restoration failed")
        return
    raise ScenarioFailure("fixture restoration is restricted to the bounded logout variants")


def _events(evidence: dict[str, Any], opcode: int, connection: str | None = None,
            body: bytes | None = None) -> list[dict[str, Any]]:
    result = []
    for event in evidence.get("events", []):
        if not isinstance(event, dict) or event.get("opcode") != opcode:
            continue
        if connection is not None and event.get("connection") != connection:
            continue
        if body is not None and event.get("body_hex") != body.hex():
            continue
        result.append(event)
    return result


def _sequence(item: dict[str, Any]) -> int:
    value = item.get("observation_sequence", -1)
    return value if type(value) is int else -1


def _sends(evidence: dict[str, Any], action: str) -> list[dict[str, Any]]:
    return [packet for packet in evidence.get("sent_packets", [])
            if isinstance(packet, dict) and packet.get("action") == action
            and packet.get("write_attempted") is True
            and packet.get("write_completed") is True and packet.get("write_error") is None]


def _typed_requests(evidence: dict[str, Any]) -> list[dict[str, Any]]:
    packets = [packet for packet in _sends(evidence, "logout_request")
               if packet.get("connection") == "instance"
               and packet.get("opcode") == CMSG_LOGOUT_REQUEST
               and packet.get("body_hex") == "00" and packet.get("action_index") is not None]
    return sorted(packets, key=_sequence)


def _typed_cancels(evidence: dict[str, Any]) -> list[dict[str, Any]]:
    return sorted((packet for packet in _sends(evidence, "logout_cancel")
                   if packet.get("connection") == "instance"
                   and packet.get("opcode") == CMSG_LOGOUT_CANCEL
                   and packet.get("body_hex") == "" and packet.get("action_index") is not None),
                  key=_sequence)


def _stand_states(evidence: dict[str, Any], state: int) -> list[dict[str, Any]]:
    return sorted((event for event in _events(evidence, STAND_STATE_UPDATE, "realm")
                   if event.get("body_hex") == (b"\x00" * 4 + bytes((state,))).hex()),
                  key=_sequence)


_UNIT_ROOT_FIELDS = {0, 32, 64, 96, 116, 167, 174, 190, 212}
_UNIT_GUID_FIELDS = {*range(11, 21), 107, 108, 112}
_UNIT_UNSUPPORTED_FIELDS = {1, 2, 3, 4, 22, 168, 169, 170}
_UNIT_FIELD_WIDTHS = {field: 8 for field in (5, 6, 21)}
for _field in (7, 8, 9, 10, 23, 29, 30, 31, *range(33, 56), *range(60, 64),
               *range(65, 73), 75, 76, *range(81, 96), *range(97, 107),
               *range(109, 112), *range(113, 116), *range(117, 167),
               *range(171, 174), *range(175, 227)):
    _UNIT_FIELD_WIDTHS[_field] = 4
for _field in (73, 74):
    _UNIT_FIELD_WIDTHS[_field] = 2
for _field in (*range(24, 29), *range(56, 60), *range(77, 81)):
    _UNIT_FIELD_WIDTHS[_field] = 1


def _value_blocks(body: bytes) -> list[tuple[dict[str, int], bytes]]:
    if len(body) < 11:
        raise ValueError("truncated UpdateObject header")
    count, _map_id = struct.unpack_from("<IH", body)
    if body[6] & 0x7F:
        raise ValueError("invalid UpdateObject list flag")
    offset = 7
    if body[6] & 0x80:
        destroyed, removed = struct.unpack_from("<HI", body, offset)
        offset += 6
        if destroyed > removed:
            raise ValueError("invalid UpdateObject removal counts")
        for _ in range(removed):
            _, offset = _packed_guid(body, offset, len(body))
    size = struct.unpack_from("<I", body, offset)[0]
    offset += 4
    end = offset + size
    if end != len(body):
        raise ValueError("invalid UpdateObject data span")
    blocks = []
    while offset < end:
        update_type = body[offset]
        if update_type != 0:
            raise ValueError("non-VALUES block in bounded UnitData decode")
        guid, cursor = _packed_guid(body, offset + 1, end)
        value_size = struct.unpack_from("<I", body, cursor)[0]
        cursor += 4
        value_end = cursor + value_size
        if value_end > end:
            raise ValueError("truncated UpdateObject VALUES block")
        blocks.append((guid, body[cursor:value_end]))
        offset = value_end
    if len(blocks) != count:
        raise ValueError("UpdateObject block count does not match its header")
    return blocks


def _unit_data_flags(data: bytes) -> int | None:
    if len(data) < 5:
        raise ValueError("truncated UnitData values")
    type_mask = struct.unpack_from("<I", data)[0]
    if type_mask not in {0x20, 0x21}:
        return None
    offset = 4
    if type_mask & 1:
        object_groups = data[offset] >> 4
        offset += 1
        if object_groups & 1:
            for bit in (1, 2, 3):
                if object_groups & (1 << bit):
                    offset += 4
    if offset >= len(data):
        raise ValueError("truncated UnitData group mask")
    group_mask = data[offset]
    offset += 1
    words = []
    for group in range(8):
        if group_mask & (1 << group):
            if offset + 4 > len(data):
                raise ValueError("truncated UnitData field mask")
            words.append((group, int.from_bytes(data[offset:offset + 4], "big")))
            offset += 4
    fields = [group * 32 + bit for group, word in words
              for bit in range(32) if word & (1 << bit)]
    flags = None
    for field in fields:
        if field in _UNIT_ROOT_FIELDS:
            continue
        if field in _UNIT_UNSUPPORTED_FIELDS:
            raise ValueError("dynamic UnitData field in bounded logout decode")
        if field in _UNIT_GUID_FIELDS:
            _, offset = _packed_guid(data, offset, len(data))
            continue
        width = _UNIT_FIELD_WIDTHS.get(field)
        if width is None or offset + width > len(data):
            raise ValueError("unknown or truncated UnitData field")
        if field == 41:
            flags = int.from_bytes(data[offset:offset + 4], "little")
        offset += width
    if offset != len(data):
        raise ValueError("UnitData values have an invalid final span")
    return flags


def _stun_updates(evidence: dict[str, Any], owner: dict[str, int]) -> list[dict[str, Any]]:
    updates = []
    for event in _events(evidence, UPDATE_OBJECT, "instance"):
        try:
            body = bytes.fromhex(event["body_hex"])
            if event.get("body_sha256") != hashlib.sha256(body).hexdigest():
                continue
            for guid, data in _value_blocks(body):
                if guid != owner:
                    continue
                flags = _unit_data_flags(data)
                if flags is not None:
                    updates.append({"event_sequence": event.get("sequence"),
                                    "observation_sequence": _sequence(event), "flags": flags,
                                    "stunned": bool(flags & UNIT_FLAG_STUNNED)})
        except (KeyError, TypeError, ValueError, struct.error):
            continue
    return sorted(updates, key=lambda item: item["observation_sequence"])


def _require_common(scenario: str, evidence: dict[str, Any]) -> tuple[list[dict[str, Any]], int]:
    requests = _typed_requests(evidence)
    expected_count = 2 if scenario == "logout-repeated" else 1
    if len(requests) != expected_count or _sends(evidence, "finish_logout"):
        raise ScenarioFailure("typed logout request count or disconnect boundary is invalid")
    cancel_scenarios = {"logout-cancel", "logout-repeated", "logout-rbac-revoke"}
    if scenario in cancel_scenarios and len(_typed_cancels(evidence)) != 1:
        raise ScenarioFailure("cancel scenario requires exactly one typed cancel")
    reason = {"logout-combat-denial": 1, "logout-fall-denial": 3,
              "logout-freeze-denial": 2}.get(scenario, 0)
    instant = b"\x80" if scenario in INSTANT_SCENARIOS else b"\x00"
    responses = _events(evidence, LOGOUT_RESPONSE, "instance",
                        reason.to_bytes(4, "little", signed=True) + instant)
    if len(responses) != expected_count:
        raise ScenarioFailure("logout response reason, body, route, or count is incorrect")
    if len(_events(evidence, LOGOUT_RESPONSE)) != expected_count:
        raise ScenarioFailure("unexpected additional logout response was observed")
    if _sends(evidence, "logout_cancel") and scenario not in cancel_scenarios:
        raise ScenarioFailure("non-cancel scenario sent a logout cancel")
    return requests, reason


def _validate_denial(scenario: str, evidence: dict[str, Any],
                     requests: list[dict[str, Any]]) -> dict[str, Any]:
    boundary = _sequence(requests[0])
    if any(_sequence(event) > boundary for event in _events(evidence, MOVE_ROOT, "instance")):
        raise ScenarioFailure("denied logout emitted a movement root")
    if any(_sequence(event) > boundary for event in _stand_states(evidence, 1)):
        raise ScenarioFailure("denied logout emitted a sit transition")
    if _events(evidence, LOGOUT_COMPLETE) or evidence.get("logout_confirmed") is True:
        raise ScenarioFailure("denied logout finalized or emitted LogoutComplete")
    if scenario == "logout-combat-denial":
        attacks = [packet for packet in _sends(evidence, "attack_swing")
                   if 0 <= _sequence(packet) < boundary]
        if not attacks:
            raise ScenarioFailure("combat denial lacks a written attack swing before the request")
        combat = [event for event in _events(evidence, ATTACKER_STATE_UPDATE, "instance")
                  if event.get("body_bytes", 0) > 0 and 0 <= _sequence(event) < boundary]
        if not combat:
            raise ScenarioFailure("combat denial lacks a non-empty attack update before the request")
    if scenario == "logout-fall-denial" and not any(
            0 <= _sequence(packet) < boundary for packet in _sends(evidence, "movement_heartbeat")):
        raise ScenarioFailure("fall denial lacks a real falling movement action")
    return {"denial_observation": "no root, sit, countdown completion, or finalization"}


def _validate_instant(evidence: dict[str, Any], actor: dict[str, Any], realm_id: int,
                      requests: list[dict[str, Any]]) -> dict[str, Any]:
    request_boundary = _sequence(requests[0])
    responses = _events(evidence, LOGOUT_RESPONSE, "instance")
    complete = _events(evidence, LOGOUT_COMPLETE, "realm", b"")
    if _sequence(responses[0]) <= request_boundary:
        raise ScenarioFailure("instant logout response was not observed after its typed request")
    if len(complete) != 1 or len(_events(evidence, LOGOUT_COMPLETE)) != 1 \
            or _sequence(complete[0]) <= _sequence(responses[0]):
        raise ScenarioFailure("instant logout requires one empty realm Complete after its response")
    if any(_sequence(event) > request_boundary
           for event in _events(evidence, MOVE_ROOT, "instance")):
        raise ScenarioFailure("instant logout emitted a movement root")
    if any(_sequence(event) > request_boundary for event in _stand_states(evidence, 1)):
        raise ScenarioFailure("instant logout emitted a sit transition")
    owner = death_scenario.player_guid(actor, realm_id)
    if any(item["observation_sequence"] > request_boundary
           for item in _stun_updates(evidence, owner)):
        raise ScenarioFailure("instant logout emitted a stun transition")
    return {"complete_event_sequence": complete[0].get("sequence")}


def _validate_admitted(scenario: str, evidence: dict[str, Any], actor: dict[str, Any],
                       realm_id: int, requests: list[dict[str, Any]]) -> dict[str, Any]:
    responses = sorted(_events(evidence, LOGOUT_RESPONSE, "instance"), key=_sequence)
    request_sequences = [_sequence(packet) for packet in requests]
    if any(_sequence(response) <= request_sequences[index]
           for index, response in enumerate(responses)):
        raise ScenarioFailure("logout response was not observed after its typed request")
    if scenario == "logout-repeated" and not (
            _sequence(responses[0]) < request_sequences[1] < _sequence(responses[1])):
        raise ScenarioFailure("repeated request was not sent between the two accepted responses")
    cancel = _typed_cancels(evidence)
    cancel_boundary = _sequence(cancel[0]) if cancel else math.inf
    roots = sorted((event for event in _events(evidence, MOVE_ROOT, "instance")
                    if _sequence(event) > request_sequences[0]), key=_sequence)
    sits = [event for event in _stand_states(evidence, 1)
            if _sequence(event) > request_sequences[0] and _sequence(event) < cancel_boundary]
    expected_roots = 2 if scenario == "logout-repeated" else 1
    if len(roots) != expected_roots or len(sits) != 1:
        raise ScenarioFailure("timed logout did not produce the exact root and single sit transitions")
    for index, root in enumerate(roots):
        if _sequence(root) < _sequence(responses[index]) or _sequence(root) > cancel_boundary:
            raise ScenarioFailure("logout root was outside its accepted request phase")
    ack = _events(evidence, LOGOUT_CANCEL_ACK, "instance", b"") if cancel else []
    ack_boundary = _sequence(ack[0]) if len(ack) == 1 else math.inf
    owner = death_scenario.player_guid(actor, realm_id)
    stun = _stun_updates(evidence, owner)
    stunned = [item for item in stun if item["stunned"]
               and request_sequences[0] < item["observation_sequence"] < ack_boundary]
    if not stunned or stunned[0]["observation_sequence"] < _sequence(roots[0]):
        raise ScenarioFailure("timed logout lacks its UNIT_FLAG_STUNNED set evidence")
    first_set = stunned[0]
    if any(item["flags"] != first_set["flags"] for item in stunned[1:]):
        raise ScenarioFailure("duplicate logout stun-set updates have conflicting flags")
    facts: dict[str, Any] = {"roots": [event.get("sequence") for event in roots],
                             "sit_count": len(sits), "stun_set": first_set}
    if cancel:
        unroots = [event for event in _events(evidence, MOVE_UNROOT, "instance")
                   if _sequence(event) > cancel_boundary]
        stands = [event for event in _stand_states(evidence, 0)
                  if _sequence(event) > cancel_boundary]
        cleared = [item for item in stun if not item["stunned"]
                   and item["observation_sequence"] > cancel_boundary]
        if len(ack) != 1 or len(unroots) != 1 or len(stands) != 1 or not cleared:
            raise ScenarioFailure("cancel lacks one ACK, unroot, stand, or stun-clear transition")
        last_clear = cleared[-1]
        if any(item["flags"] != last_clear["flags"] for item in cleared[:-1]):
            raise ScenarioFailure("duplicate logout stun-clear updates have conflicting flags")
        if not (cancel_boundary < _sequence(ack[0]) < _sequence(unroots[0])
                < last_clear["observation_sequence"]):
            raise ScenarioFailure("cancel ACK, unroot, and stun-clear order is invalid")
        if first_set["flags"] ^ last_clear["flags"] != UNIT_FLAG_STUNNED:
            raise ScenarioFailure("cancel did not clear exactly the logout stun flag")
        facts.update({"ack_count": 1, "unroot": unroots[0].get("sequence"),
                      "stand": stands[0].get("sequence"), "stun_clear": last_clear})
    if scenario != "logout-timed" and (
            _events(evidence, LOGOUT_COMPLETE) or evidence.get("logout_confirmed") is True):
        raise ScenarioFailure("cancel, repeated, or disconnect case emitted LogoutComplete")
    return facts


def _validate_timed(plan: dict[str, Any], evidence: dict[str, Any]) -> dict[str, Any]:
    complete = _events(evidence, LOGOUT_COMPLETE, "realm", b"")
    if len(complete) != 1 or len(_events(evidence, LOGOUT_COMPLETE)) != 1:
        raise ScenarioFailure("timed logout requires exactly one empty realm LogoutComplete")
    waits = [index for index, action in enumerate(plan["actions"])
             if action.get("action") == "wait_for_packet"
             and action.get("connection") == "realm"
             and action.get("opcode") == LOGOUT_COMPLETE]
    started = [item for item in evidence.get("trace", [])
               if item.get("kind") == "action_started"
               and item.get("action") == "wait_for_packet"
               and item.get("action_index") in waits and type(item.get("sequence")) is int]
    requests = _typed_requests(evidence)
    roots = [event for event in _events(evidence, MOVE_ROOT, "instance")
             if _sequence(event) > _sequence(requests[0])]
    if len(waits) != 1 or len(started) != 1 or len(roots) != 1:
        raise ScenarioFailure("timed logout requires one Complete wait started after its root")
    if started[0]["sequence"] < _sequence(roots[0]) or \
            _sequence(complete[0]) <= started[0]["sequence"]:
        raise ScenarioFailure("request, response, root, Complete wait, and Complete order is invalid")
    return {"complete_event_sequence": complete[0].get("sequence"),
            "complete_wait_started_trace_sequence": started[0]["sequence"]}


def validate_evidence(scenario: str, plan: dict[str, Any], evidence: dict[str, Any],
                      actor: dict[str, Any], realm_id: int) -> dict[str, Any]:
    requests, _ = _require_common(scenario, evidence)
    if scenario.endswith("-denial"):
        return _validate_denial(scenario, evidence, requests)
    if scenario in INSTANT_SCENARIOS:
        return _validate_instant(evidence, actor, realm_id, requests)
    facts = _validate_admitted(scenario, evidence, actor, realm_id, requests)
    if scenario == "logout-timed":
        facts.update(_validate_timed(plan, evidence))
    return facts


def _capture_packet_path(capture: Path, direction: str, opcode: int) -> Path:
    matches = []
    for path in capture.glob("*.meta"):
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except (OSError, UnicodeError):
            raise ScenarioFailure("unable to read logout packet capture metadata") from None
        values = dict(line.split("=", 1) for line in lines if "=" in line)
        try:
            packet_opcode = int(values["opcode"], 16)
        except (KeyError, ValueError):
            continue
        if values.get("direction") == direction and packet_opcode == opcode:
            matches.append(path)
    if len(matches) != 1:
        raise ScenarioFailure("timed logout capture requires exactly one request and Complete packet")
    return matches[0]


def _measure_timed_logout(capture: Path) -> dict[str, Any]:
    request = _capture_packet_path(capture, "c2s", CMSG_LOGOUT_REQUEST)
    complete = _capture_packet_path(capture, "s2c", LOGOUT_COMPLETE)
    elapsed_ms = (complete.stat().st_mtime_ns - request.stat().st_mtime_ns) / 1_000_000
    if not 19_000 <= elapsed_ms <= 25_000:
        raise ScenarioFailure("timed logout completion is outside the 19-to-25-second window")
    return {"request_path": str(request), "complete_path": str(complete),
            "elapsed_ms": elapsed_ms}


def _measure_timed_logout_cpp(capture: Path) -> dict[str, Any]:
    bot_log = capture / "logout-timed-001.bot.log"
    try:
        lines = bot_log.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeError):
        raise ScenarioFailure("unable to read timed logout bot log") from None
    markers = {"CAST_LIFECYCLE_READY": [], "Report written": []}
    for line in lines:
        match = re.match(r"^(\S+).*\b(CAST_LIFECYCLE_READY|Report written)\b", line)
        if match:
            markers[match.group(2)].append(match.group(1))
    if any(len(timestamps) != 1 for timestamps in markers.values()):
        raise ScenarioFailure("timed logout bot log requires exactly one ready and report marker")
    try:
        ready = datetime.fromisoformat(markers["CAST_LIFECYCLE_READY"][0].replace("Z", "+00:00"))
        report = datetime.fromisoformat(markers["Report written"][0].replace("Z", "+00:00"))
        elapsed_ms = (report - ready).total_seconds() * 1_000
    except (TypeError, ValueError):
        raise ScenarioFailure("timed logout bot log has invalid ISO timestamps") from None
    if not 19_000 <= elapsed_ms <= 25_000:
        raise ScenarioFailure("timed logout completion is outside the 19-to-25-second window")
    return {"elapsed_ms": elapsed_ms, "timing_source": "bot-log",
            "bot_log_path": str(bot_log)}


def _actor_online(db: death_fixture.MariaDB, journal: dict[str, Any],
                  actor: dict[str, Any]) -> bool:
    rows = db.character_identities(journal)
    matches = [row for row in rows if len(row) == 4 and row[0] == str(actor["guid"])
               and row[1] == str(actor["account_id"]) and row[2] == actor["name"]]
    if len(matches) != 1 or matches[0][3] not in {"0", "1"}:
        raise ScenarioFailure("character online identity is missing or ambiguous")
    return matches[0][3] == "1"


def _sleep_until(deadline: float) -> None:
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            return
        time.sleep(min(remaining, 0.25))


def prove_disconnect_timing(db: death_fixture.MariaDB, journal: dict[str, Any],
                            actor: dict[str, Any], disconnected_at: float) -> dict[str, Any]:
    early_at = disconnected_at + DISCONNECT_ONLINE_CHECK_SECONDS
    _sleep_until(early_at)
    early_elapsed = time.monotonic() - disconnected_at
    if not _actor_online(db, journal, actor):
        raise ScenarioFailure("disconnected countdown character was not online at the two-second check")
    _sleep_until(disconnected_at + LOGOUT_DELAY_MS / 1000)
    deadline = time.monotonic() + DISCONNECT_OFFLINE_GRACE_SECONDS
    while time.monotonic() < deadline:
        if not _actor_online(db, journal, actor):
            return {"online_check_elapsed_seconds": early_elapsed,
                    "offline_check_elapsed_seconds": time.monotonic() - disconnected_at,
                    "online_at_two_seconds": True, "offline_after_twenty_seconds": True}
        time.sleep(0.25)
    raise ScenarioFailure("disconnected countdown character did not go offline after its deadline")


def _save_snapshot(db: death_fixture.MariaDB, journal: dict[str, Any], actor: dict[str, Any],
                   scenario: str, output: Path, label: str) -> tuple[dict[str, Any], dict[str, Any], Path]:
    death_scenario._poll_actor_offline(db, journal, actor)
    snapshot = db.snapshot(journal)
    path = death_scenario._snapshot_path(output, scenario, 1, actor, label)
    death_scenario._write_private(path, snapshot)
    row = death_scenario._actor_snapshot(snapshot, actor)
    try:
        online = int(row["online"])
        logout_time = int(row.get("logout_time", 0))
    except (KeyError, ValueError):
        raise ScenarioFailure("logout snapshot lacks valid online or logout-time state") from None
    if online != 0:
        raise ScenarioFailure("logout snapshot retained an online character")
    state = {"guid": actor["guid"], "online": online, "logout_time": logout_time,
             "position": {key: row.get(key) for key in
                          ("map", "position_x", "position_y", "position_z", "orientation")}}
    return snapshot, state, path


def _run_single(pass_: death_scenario.BotPass, identity: dict[str, Any], *,
                observer: bool = False) -> dict[str, Any]:
    death_scenario.start_bot(pass_, bot_exec=identity["bot_exec"],
                             bot_config=identity["bot_config"], output=identity["capture"])
    timeout = math.ceil(pass_.plan["timeout_ms"] / 1000) + 60
    return death_scenario.finish_bot(pass_, timeout_seconds=timeout,
                                     actor=pass_.actor, observer=observer)


def _observation(pass_: death_scenario.BotPass, evidence: dict[str, Any],
                 state: dict[str, Any], snapshot_path: Path,
                 facts: dict[str, Any]) -> dict[str, Any]:
    return {"pass": pass_.label, "actor": pass_.actor["name"],
            "report_sha256": pass_.report_hash, "state": state,
            "lifecycle": death_scenario._evidence_digest(evidence),
            "snapshot_sha256": death_scenario._sha256(snapshot_path), **facts}


def _run_case_body(identity: dict[str, Any], journal: dict[str, Any], realm_id: int,
                   db: death_fixture.MariaDB) -> dict[str, Any]:
    scenario = identity["scenario"]
    actor = next(item for item in journal["actors"] if item["name"] == SCENARIOS[scenario])
    plan = plan_for(scenario, actor)
    if scenario == "logout-freeze-denial":
        _seed_freeze(scenario, db, actor, realm_id)
    pass_ = death_scenario.BotPass(actor, plan, f"{scenario}-001")
    evidence = _run_single(pass_, identity)
    facts = validate_evidence(scenario, plan, evidence, actor, realm_id)
    if scenario == "logout-disconnect":
        disconnected_at = time.monotonic()
        facts.update(prove_disconnect_timing(db, journal, actor, disconnected_at))
    snapshot, state, snapshot_path = _save_snapshot(db, journal, actor, scenario,
                                                    identity["capture"], pass_.label)
    if scenario in {"logout-timed", *INSTANT_SCENARIOS} and state["logout_time"] <= 0:
        raise ScenarioFailure("completed logout did not persist a logout timestamp")
    if scenario == "logout-freeze-denial":
        auras = death_scenario._snapshot_facts(snapshot, actor)["auras"]
        if not any(row.get("spell") == FREEZE_SPELL for row in auras):
            raise ScenarioFailure("freeze denial did not retain its seeded 9454 aura")
        facts["freeze_aura_rows"] = [row for row in auras if row.get("spell") == FREEZE_SPELL]
    observations = [_observation(pass_, evidence, state, snapshot_path, facts)]
    if scenario == "logout-disconnect":
        relogin = death_scenario.BotPass(actor, relogin_plan(actor), "logout-disconnect-relogin")
        relogin_evidence = _run_single(relogin, identity, observer=True)
        _, relogin_state, relogin_path = _save_snapshot(
            db, journal, actor, scenario, identity["capture"], relogin.label)
        observations.append(_observation(relogin, relogin_evidence, relogin_state,
                                         relogin_path, {"relogin_observed": True}))
    return {"scenario": scenario, "epoch": 1, "scenario_observation": "passed",
            "observations": observations, "parity_assessed": False}


def _run_case(identity: dict[str, Any], journal: dict[str, Any], realm_id: int,
              db: death_fixture.MariaDB) -> dict[str, Any]:
    scenario = identity["scenario"]
    if scenario not in VARIANT_SCENARIOS:
        report = _run_case_body(identity, journal, realm_id, db)
        if scenario == "logout-timed":
            backend = identity.get("backend", "rust")
            if backend == "rust":
                timing = _measure_timed_logout(identity["capture"])
            elif backend == "cpp":
                timing = _measure_timed_logout_cpp(identity["capture"])
            else:
                raise ScenarioFailure("capture backend is invalid")
            report["observations"][0].update(timing)
        return report
    actor = next(item for item in journal["actors"] if item["name"] == SCENARIOS[scenario])
    state = _prepare_variant(scenario, db, actor)
    try:
        report = _run_case_body(identity, journal, realm_id, db)
    finally:
        _restore_variant(scenario, db, actor, state)
    report["variant_fixture_restored"] = True
    return report


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
        raise ScenarioFailure("logout scenario or epoch is outside the bounded case set")
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
    report["schema"] = "rustycore-logout-scenario-v1"
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
        report.update({"schema": "rustycore-logout-scenario-v1",
            "backend": os.environ["RUSTYCORE_CAPTURE_BACKEND"],
            "hashes": death_scenario._hashes(identity), "bot_config_unchanged": True,
            "candidate_server_sha256": os.environ["RUSTYCORE_CAPTURE_CANDIDATE_SHA256"].lower(),
            "scenario_runner_sha256": death_scenario._sha256(Path(__file__).resolve()),
            "target_database": journal["target_database"],
            "native_collection": "recorded by outer capture wrapper", "parity_assessed": False})
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
