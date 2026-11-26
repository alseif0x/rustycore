#!/usr/bin/env python3
"""Verify creature CREATE → normal OUT_OF_RANGE → CREATE for a still observer.

Input is a RustyCore native packet dump. A candidate is accepted only when both
CREATE packets are single-block Unit updates, an exact full GUID is listed in
the non-destroy removal segment, and that same GUID has a moving OnMonsterMove
spline on the observer's INSTANCE socket before leaving range.
"""

from __future__ import annotations

import argparse
from collections import defaultdict
from dataclasses import dataclass
import hashlib
import json
import math
from pathlib import Path
import re
import struct
from typing import Any

from verify_melee_observer import guid as unpack_guid, require


INSTANCE = 1
UPDATE_OBJECT = ("s2c", INSTANCE, 0x27CB)
ON_MONSTER_MOVE = ("s2c", INSTANCE, 0x2DD4)
PHASE_SHIFT_CHANGE = ("s2c", INSTANCE, 0x2578)
LOGIN_VERIFY_WORLD = ("s2c", INSTANCE, 0x2597)
NEW_WORLD = ("s2c", INSTANCE, 0x2594)
MOVE_TELEPORT = ("s2c", INSTANCE, 0x2E04)
MOVE_TELEPORT_ACK = ("c2s", INSTANCE, 0x39FA)
MOVE_INIT_ACTIVE_MOVER_COMPLETE = ("c2s", INSTANCE, 0x3A46)
MOVE_SEAMLESS_TRANSFER_COMPLETE = ("c2s", INSTANCE, 0x3A44)
UNIT_TYPE_ID = 5
CREATE_TYPES = {1: "CreateObject", 2: "CreateObject2"}
TIMESTAMP_RE = re.compile(
    r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})$"
)
TIMESTAMP_KEYS = ("timestamp_utc", "timestamp", "captured_at", "time_utc")
TRANSLATIONAL_STARTS = {
    "MoveStartForward", "MoveStartBackward", "MoveStartStrafeLeft",
    "MoveStartStrafeRight", "MoveJump", "MoveStartAscend",
    "MoveStartDescend", "MoveStartSwim",
}
TRANSLATIONAL_STOPS = {
    "MoveStop", "MoveStopStrafe", "MoveStopAscend", "MoveStopSwim", "MoveFallLand",
}
NON_POSITION_MOVE = {
    "MoveGuildBankItem", "MoveTimeSkipped", "MoveInitActiveMoverComplete",
    "MoveSeamlessTransferComplete", "MoveTeleportAck",
}


@dataclass(frozen=True)
class Frame:
    seq: int
    direction: str
    connection_id: int
    socket_key: str
    counter: int
    opcode: int
    name: str
    raw: bytes
    body: bytes
    sha256: str
    timestamp: str | None


class MalformedRemovalHeader(ValueError):
    pass


def read_frames(directory: Path) -> list[Frame]:
    require(directory.is_dir(), "native packet directory is missing")
    frames: list[Frame] = []
    seen: set[int] = set()
    for meta_path in directory.glob("*.meta"):
        try:
            meta = dict(line.split("=", 1) for line in
                        meta_path.read_text(encoding="utf-8").splitlines() if "=" in line)
            direction = meta["direction"]
            conn = int(meta["connection_id"])
            socket_key = meta["addr"]  # internal correlation only; never reported
            seq = int(meta["seq"])
            counter = int(meta["counter"])
            opcode = int(meta["opcode"], 16)
            name = meta["name"]
            raw = meta_path.with_suffix(".bin").read_bytes()
            expected_len = int(meta["len"])
        except (OSError, KeyError, ValueError):
            raise ValueError("native packet metadata is incomplete or invalid") from None
        require(seq not in seen, "native packet sequence is duplicated")
        seen.add(seq)
        require(direction in {"s2c", "c2s", "s2c-unencrypted", "c2s-unencrypted"},
                "native packet direction is invalid")
        require(conn in (0, 1), "native packet connection type is invalid")
        require(len(raw) == expected_len and len(raw) >= 2,
                "native packet length does not match metadata")
        require(int.from_bytes(raw[:2], "little") == opcode,
                "native packet opcode does not match metadata")
        timestamp = next((meta[key] for key in TIMESTAMP_KEYS
                          if meta.get(key) and TIMESTAMP_RE.fullmatch(meta[key])), None)
        frames.append(Frame(seq, direction.removesuffix("-unencrypted"), conn,
                            socket_key, counter, opcode, name, raw, raw[2:],
                            hashlib.sha256(raw).hexdigest(), timestamp))
    require(frames, "native packet directory contains no packet metadata")
    return sorted(frames, key=lambda item: item.seq)


def read_guid(data: bytes, offset: int, end: int) -> tuple[tuple[int, int], int]:
    require(offset <= end, "packed GUID starts beyond packet boundary")
    try:
        value, next_offset = unpack_guid(data, offset)
    except (IndexError, struct.error):
        raise ValueError("truncated packed GUID") from None
    require(next_offset <= end, "packed GUID crosses packet boundary")
    return value, next_offset


def read_u32(data: bytes, offset: int, end: int) -> tuple[int, int]:
    require(offset + 4 <= end, "truncated packet integer")
    return struct.unpack_from("<I", data, offset)[0], offset + 4


def read_floats(data: bytes, offset: int, count: int, end: int) -> tuple[tuple[float, ...], int]:
    require(offset + count * 4 <= end, "truncated packet coordinates")
    values = struct.unpack_from("<" + "f" * count, data, offset)
    require(all(math.isfinite(value) for value in values), "packet has non-finite coordinates")
    return values, offset + count * 4


def read_bits(data: bytes, offset: int, count: int) -> tuple[int, int]:
    value = 0
    for index in range(count):
        pos = offset + index
        require(pos // 8 < len(data), "truncated packet bit field")
        value = (value << 1) | int(bool(data[pos // 8] & (1 << (7 - pos % 8))))
    return value, offset + count


def guid_entry(guid: tuple[int, int]) -> int:
    return (guid[1] >> 6) & 0x7FFFFF


def parse_update_object(frame: Frame) -> dict[str, Any]:
    """Decode list framing for every packet; accept CREATE only as one block."""
    body = frame.body
    end = len(body)
    if end < 7:
        raise MalformedRemovalHeader("truncated UpdateObject header")
    block_count, map_id = struct.unpack_from("<IH", body)
    if body[6] & 0x7F:
        raise MalformedRemovalHeader("invalid UpdateObject removal-list flag")
    offset = 7
    destroyed: list[tuple[int, int]] = []
    out_of_range: list[tuple[int, int]] = []
    if body[6] & 0x80:
        try:
            require(offset + 6 <= end, "truncated UpdateObject removal counts")
            destroy_count, removed_count = struct.unpack_from("<HI", body, offset)
            offset += 6
            require(destroy_count <= removed_count and removed_count <= 1_000_000,
                    "invalid UpdateObject removal counts")
            removed = []
            for _ in range(removed_count):
                guid, offset = read_guid(body, offset, end)
                removed.append(guid)
            destroyed, out_of_range = removed[:destroy_count], removed[destroy_count:]
        except (ValueError, IndexError, struct.error):
            raise MalformedRemovalHeader("malformed UpdateObject removal list") from None
    create = None
    try:
        data_size, data_offset = read_u32(body, offset, end)
        data_end = data_offset + data_size
        if data_end == end and block_count == 1 and data_offset < data_end \
                and body[data_offset] in CREATE_TYPES:
            update_type = body[data_offset]
            guid, cursor = read_guid(body, data_offset + 1, data_end)
            require(cursor + 4 <= data_end, "truncated CREATE type and movement flags")
            type_id = body[cursor]
            flags = body[cursor + 1 : cursor + 4]
            cursor += 4
            # MovementUpdate is bit 3 in the CreateObjectBits. Its mover GUID
            # must match the object GUID before this counts as a verified Unit.
            if type_id == UNIT_TYPE_ID and flags[0] & 0x10:
                mover_guid, cursor = read_guid(body, cursor, data_end)
                if mover_guid == guid and cursor + 40 <= data_end:
                    create = {"guid": guid, "entry": guid_entry(guid), "type_id": type_id,
                              "update_type": CREATE_TYPES[update_type], "block_index": 0}
    except (ValueError, IndexError, struct.error):
        # The removal header is already decoded. An unsupported or malformed
        # block body cannot manufacture a CREATE and does not poison removals.
        pass
    return {"map_id": map_id, "block_count": block_count,
            "create": create, "destroyed": destroyed, "out_of_range": out_of_range}


def parse_monster_move(frame: Frame) -> dict[str, Any]:
    """Read the nested MovementSpline path; outer Destination is often zero."""
    body = frame.body
    end = len(body)
    guid, offset = read_guid(body, 0, end)
    start, offset = read_floats(body, offset, 3, end)
    require(offset + 4 <= end, "truncated OnMonsterMove spline ID")
    spline_id, = struct.unpack_from("<I", body, offset)
    offset += 4
    _, offset = read_floats(body, offset, 3, end)  # MovementMonsterSpline::Destination
    require(offset + 1 + 17 + 1 <= end, "truncated OnMonsterMove spline")
    offset += 1  # CRZ teleport and stop-distance bits
    spline_flags, elapsed, duration, _fade = struct.unpack_from("<IiII", body, offset)
    offset += 16
    offset += 1  # MovementSpline mode
    _, offset = read_guid(body, offset, end)  # transport GUID
    require(offset < end, "truncated OnMonsterMove vehicle seat")
    offset += 1
    require(offset + 5 <= end, "truncated OnMonsterMove path flags")
    bits = body[offset : offset + 5]
    face, bit = read_bits(bits, 0, 2)
    point_count, bit = read_bits(bits, bit, 16)
    _vehicle_exit, bit = read_bits(bits, bit, 1)
    _interpolate, bit = read_bits(bits, bit, 1)
    packed_delta_count, bit = read_bits(bits, bit, 16)
    has_filter, bit = read_bits(bits, bit, 1)
    has_spell, bit = read_bits(bits, bit, 1)
    has_jump, bit = read_bits(bits, bit, 1)
    has_anim, _ = read_bits(bits, bit, 1)
    offset += 5
    require(point_count <= 4096 and packed_delta_count <= 4096,
            "OnMonsterMove path exceeds the bounded decoder")
    if has_filter:
        raise ValueError("OnMonsterMove spline filter is unsupported")
    if face == 1:
        _, offset = read_floats(body, offset, 3, end)
    elif face == 2:
        _, offset = read_floats(body, offset, 1, end)
        _, offset = read_guid(body, offset, end)
    elif face == 3:
        _, offset = read_floats(body, offset, 1, end)
    points = []
    for _ in range(point_count):
        point, offset = read_floats(body, offset, 3, end)
        points.append(point)
    require(offset + packed_delta_count * 4 <= end, "truncated OnMonsterMove packed path")
    offset += packed_delta_count * 4
    if has_spell:
        _, offset = read_guid(body, offset, end)
        require(offset + 16 <= end, "truncated OnMonsterMove spell data")
        offset += 16
    if has_jump:
        require(offset + 12 <= end, "truncated OnMonsterMove jump data")
        offset += 12
    if has_anim:
        require(offset + 13 <= end, "truncated OnMonsterMove animation data")
        offset += 13
    require(offset == end, "OnMonsterMove has an invalid final span")
    endpoint_displacement = math.dist(start, points[-1]) if points else 0.0
    return {"guid": guid, "spline_id": spline_id, "start": start,
            "points": points, "packed_delta_count": packed_delta_count,
            "duration_ms": duration, "elapsed_ms": elapsed,
            "spline_flags": spline_flags, "endpoint_displacement": endpoint_displacement}


def parse_movement_position(frame: Frame) -> tuple[tuple[int, int], tuple[float, float, float]]:
    guid, offset = read_guid(frame.body, 0, len(frame.body))
    require(offset + 28 <= len(frame.body), "truncated observer movement report")
    offset += 16  # flags, flags2, flags3, move time
    xyz, _ = read_floats(frame.body, offset, 3, len(frame.body))
    return guid, xyz  # type: ignore[return-value]


def event(frame: Frame, **details: Any) -> dict[str, Any]:
    return {"frame": frame.seq, "packet_counter": frame.counter,
            "timestamp": frame.timestamp, "sha256": frame.sha256,
            "digest_scope": "complete .bin frame including opcode", **details}


def motion_evidence(frames: list[Frame], socket_key: str,
                    observer_guid: tuple[int, int], start: int, end: int) -> dict[str, Any] | None:
    movement_frames = [frame for frame in frames if frame.socket_key == socket_key
                       and frame.connection_id == INSTANCE and frame.direction == "c2s"
                       and frame.name.startswith("Move")]
    active = False
    for frame in movement_frames:
        if frame.seq >= start:
            break
        if frame.name in TRANSLATIONAL_STARTS:
            active = True
        elif frame.name in TRANSLATIONAL_STOPS:
            active = False
    if active:
        return None
    samples = []
    for frame in movement_frames:
        if not start < frame.seq < end or frame.name in NON_POSITION_MOVE:
            continue
        if frame.name in TRANSLATIONAL_STARTS:
            return None
        try:
            mover, position = parse_movement_position(frame)
        except (ValueError, IndexError, struct.error):
            return None
        if mover != observer_guid:
            return None
        samples.append((frame, position))
    if not samples:
        return {"basis": "no observer translational CMSG movement inputs in window",
                "position_reports": [], "translation_active_at_start": False}
    before = []
    for frame in movement_frames:
        if frame.seq >= start or frame.name in NON_POSITION_MOVE:
            continue
        try:
            mover, position = parse_movement_position(frame)
        except (ValueError, IndexError, struct.error):
            return None
        if mover == observer_guid:
            before.append(position)
    if not before:
        return None
    anchor = before[-1]
    if any(math.dist(anchor, position) > 0.05 for _, position in samples):
        return None
    return {"basis": "observer XYZ reports remain at the pre-window anchor",
            "anchor_xyz": [round(value, 5) for value in anchor],
            "position_reports": [event(frame, xyz=[round(v, 5) for v in position])
                                 for frame, position in samples],
            "translation_active_at_start": False}


def verify(directory: Path, player_low: int, entry_id: int,
           min_seq: int | None, counter: int | None) -> dict[str, Any]:
    frames = read_frames(directory)
    phase_by_socket: dict[str, list[tuple[Frame, tuple[int, int]]]] = defaultdict(list)
    login_by_socket: dict[str, list[tuple[Frame, int]]] = defaultdict(list)
    updates_by_socket: dict[str, list[tuple[Frame, dict[str, Any]]]] = defaultdict(list)
    malformed_removals_by_socket: dict[str, list[Frame]] = defaultdict(list)
    moves_by_socket: dict[str, list[tuple[Frame, dict[str, Any]]]] = defaultdict(list)
    transfers = []
    for frame in frames:
        route = (frame.direction, frame.connection_id, frame.opcode)
        if route == PHASE_SHIFT_CHANGE:
            guid, _ = read_guid(frame.body, 0, len(frame.body))
            if guid[0] == player_low and guid[1] >> 58 == 2:
                phase_by_socket[frame.socket_key].append((frame, guid))
        elif route == LOGIN_VERIFY_WORLD:
            require(len(frame.body) >= 4, "truncated LoginVerifyWorld")
            map_id, = struct.unpack_from("<i", frame.body)
            login_by_socket[frame.socket_key].append((frame, map_id))
        elif route == UPDATE_OBJECT:
            try:
                updates_by_socket[frame.socket_key].append((frame, parse_update_object(frame)))
            except MalformedRemovalHeader:
                malformed_removals_by_socket[frame.socket_key].append(frame)
            except (ValueError, IndexError, struct.error):
                continue
        elif route == ON_MONSTER_MOVE:
            try:
                moves_by_socket[frame.socket_key].append((frame, parse_monster_move(frame)))
            except (ValueError, IndexError, struct.error):
                continue
        elif route in {NEW_WORLD, MOVE_TELEPORT, MOVE_TELEPORT_ACK,
                       MOVE_INIT_ACTIVE_MOVER_COMPLETE, MOVE_SEAMLESS_TRANSFER_COMPLETE}:
            transfers.append(frame)

    creates: dict[str, list[tuple[Frame, dict[str, Any], int]]] = defaultdict(list)
    removals: dict[str, list[tuple[Frame, tuple[int, int], int, str]]] = defaultdict(list)
    for socket_key, packets in updates_by_socket.items():
        for frame, parsed in packets:
            create = parsed["create"]
            if (create and create["entry"] == entry_id
                    and (counter is None or create["guid"][0] == counter)):
                creates[socket_key].append((frame, create, parsed["map_id"]))
            for guid in parsed["destroyed"]:
                if guid_entry(guid) == entry_id and (counter is None or guid[0] == counter):
                    removals[socket_key].append((frame, guid, parsed["map_id"], "DESTROY"))
            for guid in parsed["out_of_range"]:
                if guid_entry(guid) == entry_id and (counter is None or guid[0] == counter):
                    removals[socket_key].append((frame, guid, parsed["map_id"], "OUT_OF_RANGE"))

    candidates: dict[tuple[int, int], list[dict[str, Any]]] = defaultdict(list)
    for socket_key, player_phases in phase_by_socket.items():
        for first_frame, first_create, map_id in creates.get(socket_key, []):
            if min_seq is not None and first_frame.seq < min_seq:
                continue
            prior_logins = [(frame, selected_map) for frame, selected_map in
                            login_by_socket.get(socket_key, []) if frame.seq < first_frame.seq]
            if not prior_logins:
                continue
            login_frame, observer_map = prior_logins[-1]
            phase_in_entry = [(frame, guid) for frame, guid in player_phases
                              if login_frame.seq < frame.seq < first_frame.seq]
            if not phase_in_entry or observer_map != map_id:
                continue
            observer_guid = phase_in_entry[-1][1]
            guid = first_create["guid"]
            all_same_guid_removals = sorted(
                (item for item in removals.get(socket_key, [])
                 if item[1] == guid),
                key=lambda item: item[0].seq,
            )
            same_guid_creates = sorted(
                (item for item in creates.get(socket_key, [])
                 if item[1]["guid"] == guid), key=lambda item: item[0].seq)
            previous_create = max((item[0].seq for item in same_guid_creates
                                   if item[0].seq < first_frame.seq), default=-1)
            previous_removal = max((item[0].seq for item in all_same_guid_removals
                                    if item[0].seq < first_frame.seq), default=-1)
            if previous_create > previous_removal:
                continue
            same_guid_removals = [item for item in all_same_guid_removals
                                  if item[0].seq > first_frame.seq]
            matching_removals = [item for item in same_guid_removals
                                 if item[0].seq > first_frame.seq and item[2] == map_id]
            for out_frame, out_guid, out_map, removal_kind in matching_removals:
                if removal_kind != "OUT_OF_RANGE":
                    break
                if min_seq is not None and out_frame.seq < min_seq:
                    continue
                if any(first_frame.seq < item[0].seq < out_frame.seq
                       for item in same_guid_removals):
                    continue
                if any(first_frame.seq < item[0].seq < out_frame.seq
                       for item in same_guid_creates):
                    continue
                later_creates = [item for item in same_guid_creates
                                 if item[0].seq > out_frame.seq]
                if not later_creates:
                    continue
                again_frame, again_create, again_map = later_creates[0]
                if again_create is None or again_map != map_id:
                    continue
                if min_seq is not None and again_frame.seq < min_seq:
                    continue
                if any(out_frame.seq < item[0].seq < again_frame.seq
                       for item in same_guid_removals):
                    continue
                if any(first_frame.seq <= frame.seq <= again_frame.seq
                       for frame in malformed_removals_by_socket.get(socket_key, [])):
                    continue
                if sum(first_frame.seq <= frame.seq <= again_frame.seq for frame in frames) \
                        != again_frame.seq - first_frame.seq + 1:
                    continue
                moving = sorted(
                    ((frame, detail) for frame, detail in moves_by_socket.get(socket_key, [])
                     if first_frame.seq < frame.seq < out_frame.seq and detail["guid"] == guid
                     and detail["duration_ms"] > 0 and detail["endpoint_displacement"] > 0.05),
                    key=lambda item: item[0].seq,
                )
                if not moving:
                    continue
                last_seq = again_frame.seq
                if any(frame.socket_key == socket_key and first_frame.seq < frame.seq < last_seq
                       for frame in transfers):
                    continue
                if any(first_frame.seq < frame.seq < last_seq
                       for frame, _ in login_by_socket.get(socket_key, [])):
                    continue
                if any(first_frame.seq < phase_frame.seq < last_seq
                       and other_socket != socket_key
                       for other_socket, rows in phase_by_socket.items()
                       for phase_frame, _ in rows):
                    continue
                still = motion_evidence(frames, socket_key, observer_guid,
                                        first_frame.seq, last_seq)
                if still is None:
                    continue
                candidates[guid].append({
                    "socket_key": socket_key,
                    "map_id": map_id,
                    "initial": (first_frame, first_create),
                    "out": (out_frame, out_guid, out_map),
                    "recreate": (again_frame, again_create),
                    "movement": moving[0],
                    "stationary": still,
                })
                break

    require(candidates, "no complete stationary-observer relocation sequence was verified")
    require(len(candidates) == 1, "multiple creature GUIDs satisfy the relocation evidence")
    guid, rows = next(iter(candidates.items()))
    require(len({row["socket_key"] for row in rows}) == 1,
            "the same creature sequence appears on multiple observer sockets")
    result = min(rows, key=lambda row: row["initial"][0].seq)
    first_frame, first = result["initial"]
    out_frame, out_guid, out_map = result["out"]
    again_frame, again = result["recreate"]
    movement_frame, movement = result["movement"]
    require(first["guid"] == out_guid == again["guid"] == guid,
            "CREATE and removal events do not share the same full GUID")
    require(first["entry"] == again["entry"] == entry_id
            and first["type_id"] == again["type_id"] == UNIT_TYPE_ID,
            "CREATE type or entry does not match the requested creature")
    require(out_map == result["map_id"] and out_frame.seq < again_frame.seq,
            "removal map or event order is invalid")
    return {
        "result": "verified",
        "observer": {"connection_type": "INSTANCE", "map_id": result["map_id"],
                     "stationary_through_frame": again_frame.seq},
        "creature": {"entry": entry_id, "type_id": UNIT_TYPE_ID,
                     "guid_low": guid[0], "guid_high_hex": f"0x{guid[1]:016x}"},
        "window": {"first_frame": first_frame.seq, "last_frame": again_frame.seq,
                   "timestamp_start": first_frame.timestamp,
                   "timestamp_end": again_frame.timestamp,
                   "timestamp_note": timestamp_note(frames)},
        "events": {
            "initial_create": event(first_frame, update_type=first["update_type"], block_count=1),
            "normal_out_of_range": event(out_frame, removal_kind="OUT_OF_RANGE", map_id=out_map),
            "recreate": event(again_frame, update_type=again["update_type"], block_count=1),
            "same_guid_movement": event(
                movement_frame, opcode="OnMonsterMove", spline_id=movement["spline_id"],
                duration_ms=movement["duration_ms"], elapsed_ms=movement["elapsed_ms"],
                spline_flags=movement["spline_flags"],
                start_xyz=[round(value, 5) for value in movement["start"]],
                path_point_count=len(movement["points"]),
                path_final_xyz=([round(value, 5) for value in movement["points"][-1]]
                                if movement["points"] else None),
                packed_delta_count=movement["packed_delta_count"],
                endpoint_displacement=round(movement["endpoint_displacement"], 5)),
        },
        "observer_motion_evidence": result["stationary"],
        "checks": {"same_instance_socket": True, "same_full_creature_guid": True,
                   "normal_out_of_range_not_destroy": True,
                   "same_guid_spline_movement_before_out_of_range": True,
                   "observer_translation_in_window": False,
                   "observer_teleport_or_relogin_in_window": False},
    }


def timestamp_note(frames: list[Frame]) -> str:
    if any(frame.timestamp for frame in frames):
        return "timestamp copied from native metadata when it is a valid UTC timestamp"
    return "native packet metadata has no timestamp; frame sequence is the ordering evidence"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path, help="native packet .bin/.meta directory")
    parser.add_argument("--player-low-guid", "--observer", dest="player_low", required=True, type=int)
    parser.add_argument("--creature-entry", "--entry", dest="entry", required=True, type=int)
    parser.add_argument("--counter", type=int, help="optional creature low GUID/runtime counter")
    parser.add_argument("--min-seq", type=int, help="ignore lifecycle evidence before this frame")
    args = parser.parse_args()
    try:
        require(args.player_low >= 0, "player low GUID must be non-negative")
        require(0 < args.entry <= 0x7FFFFF, "creature entry is outside the GUID entry range")
        if args.counter is not None:
            require(0 <= args.counter <= 0xFFFFFFFFFFFFFFFF, "counter is outside low GUID range")
        if args.min_seq is not None:
            require(args.min_seq >= 0, "minimum frame sequence must be non-negative")
        print(json.dumps(verify(args.directory, args.player_low, args.entry,
                                args.min_seq, args.counter), indent=2, sort_keys=True))
    except (ValueError, IndexError, KeyError, struct.error) as error:
        parser.exit(1, f"Inconclusive or failed capture: {error}\n")


if __name__ == "__main__":
    main()
