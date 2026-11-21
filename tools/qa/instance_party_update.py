"""Strict decoder for the persisted-group PartyUpdate used by #45 restart QA."""

from __future__ import annotations

import re
from typing import Any

import death_scenario


SMSG_PARTY_UPDATE = 0x25F4


class _Cursor:
    def __init__(self, payload: bytes) -> None:
        self.payload = payload
        self.offset = 0

    def take(self, length: int, field: str) -> bytes:
        end = self.offset + length
        if end > len(self.payload):
            raise death_scenario.ScenarioFailure(
                f"truncated PartyUpdate {field}: need {length} byte(s) at offset "
                f"{self.offset}, payload has {len(self.payload)}")
        result = self.payload[self.offset:end]
        self.offset = end
        return result

    def u8(self, field: str) -> int:
        return self.take(1, field)[0]

    def u16(self, field: str) -> int:
        return int.from_bytes(self.take(2, field), "little")

    def u32(self, field: str) -> int:
        return int.from_bytes(self.take(4, field), "little")

    def i32(self, field: str) -> int:
        return int.from_bytes(self.take(4, field), "little", signed=True)

    def packed_guid(self, field: str) -> tuple[int, int]:
        low_mask = self.u8(f"{field} low mask")
        high_mask = self.u8(f"{field} high mask")
        low = 0
        high = 0
        for index in range(8):
            if low_mask & (1 << index):
                byte = self.u8(f"{field} low byte {index}")
                if byte == 0:
                    raise death_scenario.ScenarioFailure(
                        f"noncanonical PartyUpdate {field} packed GUID")
                low |= byte << (index * 8)
        for index in range(8):
            if high_mask & (1 << index):
                byte = self.u8(f"{field} high byte {index}")
                if byte == 0:
                    raise death_scenario.ScenarioFailure(
                        f"noncanonical PartyUpdate {field} packed GUID")
                high |= byte << (index * 8)
        return low, high


def _player_guid(actor: dict[str, Any], realm_id: int) -> tuple[int, int]:
    value = death_scenario.player_guid(actor, realm_id)
    return value["low"], value["high"]


def _decode(payload: bytes, receiver: dict[str, Any], leader: dict[str, Any],
            peer: dict[str, Any], realm_id: int) -> dict[str, Any]:
    cursor = _Cursor(payload)
    party_flags = cursor.u16("PartyFlags")
    party_index = cursor.u8("PartyIndex")
    party_type = cursor.u8("PartyType")
    my_index = cursor.i32("MyIndex")
    party_guid = cursor.packed_guid("PartyGUID")
    sequence_num = cursor.u32("SequenceNum")
    leader_guid = cursor.packed_guid("LeaderGUID")
    _leader_faction_group = cursor.u8("LeaderFactionGroup")
    member_count = cursor.u32("PlayerList.size")
    optional_bits = cursor.u8("optional-value bits")

    if party_flags != 0 or party_index != 0 or party_type != 1:
        raise death_scenario.ScenarioFailure(
            "initial PartyUpdate was not a normal HOME party "
            f"(flags={party_flags:#06x}, index={party_index}, type={party_type})")
    if party_guid == (0, 0):
        raise death_scenario.ScenarioFailure("initial PartyUpdate carried an empty group GUID")
    # Player::_LoadGroup resets a newly loaded player's group sequence to 1;
    # Group::SendUpdateToPlayer consumes that value for this initial update.
    if sequence_num != 1:
        raise death_scenario.ScenarioFailure(
            f"initial PartyUpdate sequence number was {sequence_num}, expected 1")
    if leader_guid != _player_guid(leader, realm_id):
        raise death_scenario.ScenarioFailure("initial PartyUpdate leader did not match the fixture")
    if member_count != 2:
        raise death_scenario.ScenarioFailure(
            f"initial PartyUpdate carried {member_count} members, expected exactly two")
    if optional_bits != 0x60:
        raise death_scenario.ScenarioFailure(
            "initial PartyUpdate did not carry only loot and difficulty settings "
            f"(optional bits={optional_bits:#04x})")

    roster: list[tuple[tuple[int, int], bool]] = []
    for index in range(member_count):
        info_bits = int.from_bytes(cursor.take(2, f"member {index} bit fields"), "big")
        if info_bits & 1:
            raise death_scenario.ScenarioFailure(
                f"initial PartyUpdate member {index} had nonzero aligned bit padding")
        name_length = (info_bits >> 10) & 0x3F
        voice_length_plus_one = (info_bits >> 4) & 0x3F
        connected = bool(info_bits & 0x08)
        if name_length == 0 or voice_length_plus_one == 0:
            raise death_scenario.ScenarioFailure(
                f"initial PartyUpdate member {index} had invalid name or voice length")

        guid = cursor.packed_guid(f"member {index} GUID")
        subgroup = cursor.u8(f"member {index} subgroup")
        _flags = cursor.u8(f"member {index} flags")
        _roles = cursor.u8(f"member {index} assigned roles")
        _class = cursor.u8(f"member {index} class")
        _faction_group = cursor.u8(f"member {index} faction group")
        cursor.take(name_length, f"member {index} name")
        cursor.take(voice_length_plus_one - 1, f"member {index} voice state")
        if subgroup != 0:
            raise death_scenario.ScenarioFailure(
                f"initial PartyUpdate member {index} was outside HOME subgroup zero")
        roster.append((guid, connected))

    receiver_guid = _player_guid(receiver, realm_id)
    peer_guid = _player_guid(peer, realm_id)
    if sorted(guid for guid, _ in roster) != sorted((receiver_guid, peer_guid)):
        raise death_scenario.ScenarioFailure(
            "initial PartyUpdate did not contain exactly the receiver and offline peer")
    if not 0 <= my_index < len(roster) or roster[my_index][0] != receiver_guid:
        raise death_scenario.ScenarioFailure(
            "initial PartyUpdate MyIndex did not identify its recipient")
    connected_by_guid = {guid: connected for guid, connected in roster}
    if connected_by_guid != {receiver_guid: True, peer_guid: False}:
        raise death_scenario.ScenarioFailure(
            "initial PartyUpdate did not show the recipient online and peer offline")

    loot_method = cursor.u8("loot method")
    loot_master = cursor.packed_guid("loot master")
    loot_threshold = cursor.u8("loot threshold")
    dungeon_difficulty = cursor.u32("dungeon difficulty")
    raid_difficulty = cursor.u32("raid difficulty")
    legacy_raid_difficulty = cursor.u32("legacy raid difficulty")
    if (loot_method, loot_master, loot_threshold) != (5, (0, 0), 2):
        raise death_scenario.ScenarioFailure(
            "initial PartyUpdate did not carry the fixture's default personal-loot settings")
    if dungeon_difficulty != 2:
        raise death_scenario.ScenarioFailure(
            f"initial PartyUpdate dungeon difficulty was {dungeon_difficulty}, expected 2")
    if cursor.offset != len(payload):
        raise death_scenario.ScenarioFailure(
            f"initial PartyUpdate left {len(payload) - cursor.offset} trailing byte(s)")

    return {
        "party_update_category": party_index,
        "party_update_sequence": None,
        "party_sequence_num": sequence_num,
        "recipient_index": my_index,
        "member_count": member_count,
        "dungeon_difficulty": dungeon_difficulty,
        "raid_difficulty": raid_difficulty,
        "legacy_raid_difficulty": legacy_raid_difficulty,
        "offline_peer_present": connected_by_guid[peer_guid] is False,
        "group_guid_nonzero": True,
        "group_guid": [party_guid[0], party_guid[1]],
    }


def validate_login_party_update(evidence: dict[str, Any], receiver: dict[str, Any],
                                leader: dict[str, Any], peer: dict[str, Any],
                                realm_id: int) -> dict[str, Any]:
    events = evidence.get("events", [])
    updates = [event for event in events
               if isinstance(event, dict) and event.get("opcode") == SMSG_PARTY_UPDATE]
    if not updates:
        raise death_scenario.ScenarioFailure("fresh login did not receive a PartyUpdate")
    sequences = [event.get("observation_sequence") for event in updates]
    if any(type(sequence) is not int or sequence < 0 for sequence in sequences):
        raise death_scenario.ScenarioFailure(
            "PartyUpdate events require nonnegative observation sequence numbers")
    if len(set(sequences)) != len(sequences):
        raise death_scenario.ScenarioFailure("PartyUpdate observation sequence was ambiguous")

    initial = min(updates, key=lambda event: event["observation_sequence"])
    if initial.get("connection") != "realm":
        raise death_scenario.ScenarioFailure(
            "initial PartyUpdate was not received on the realm connection")
    trace = evidence.get("trace", [])
    logout_starts = [item for item in trace if isinstance(item, dict)
                     and item.get("kind") == "action_started"
                     and item.get("action") == "finish_logout"]
    if len(logout_starts) != 1:
        raise death_scenario.ScenarioFailure(
            "fresh login evidence requires one finish_logout action boundary")
    boundary = logout_starts[0].get("sequence")
    if type(boundary) is not int or initial["observation_sequence"] >= boundary:
        raise death_scenario.ScenarioFailure(
            "initial PartyUpdate arrived after the bot began logout")

    body_hex = initial.get("body_hex")
    if not isinstance(body_hex, str) or not re.fullmatch(r"(?:[0-9a-fA-F]{2})+", body_hex):
        raise death_scenario.ScenarioFailure("initial PartyUpdate body was not complete hexadecimal")
    facts = _decode(bytes.fromhex(body_hex), receiver, leader, peer, realm_id)
    facts["party_update_sequence"] = initial["observation_sequence"]
    return facts


def validate_same_relogin_group(first: dict[str, Any], second: dict[str, Any]) -> None:
    if first.get("group_guid") != second.get("group_guid"):
        raise death_scenario.ScenarioFailure(
            "the restarted party changed its group GUID between relogs")
