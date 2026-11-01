"""Bounded offer-target evidence from native stationary Corpse CREATE blocks."""

from __future__ import annotations

import hashlib
import math
import struct
from typing import Any


def _packed_guid(body: bytes, offset: int, end: int) -> tuple[dict[str, int], int]:
    if offset + 2 > end:
        raise ValueError("truncated corpse evidence packed GUID")
    masks = body[offset:offset + 2]
    offset += 2
    words = []
    for mask in masks:
        word = 0
        for index in range(8):
            if mask & (1 << index):
                if offset >= end:
                    raise ValueError("truncated corpse evidence packed GUID")
                word |= body[offset] << (index * 8)
                offset += 1
        words.append(word)
    return {"low": words[0], "high": words[1]}, offset


def select_offer_corpse(evidence: dict[str, Any], victim: dict[str, Any],
                        owner: dict[str, int]) -> dict[str, Any]:
    """Match only Corpse::BuildValuesCreate, not arbitrary Player/NPC updates.

    Like the bot's creature discovery, scan retained CREATE candidates instead
    of decoding unrelated object families. Validate each candidate's complete
    stationary header and bounded ObjectData/CorpseData values (Corpse.cpp:241,
    UpdateFields.cpp:4525); the journal supplies map, location and exact owner.
    """
    candidates: dict[tuple[int, int], dict[str, Any]] = {}
    for event in evidence.get("events", []):
        if (not isinstance(event, dict) or event.get("connection") != "instance"
                or event.get("opcode") != 0x27CB):
            continue
        try:
            body = bytes.fromhex(event["body_hex"])
        except (KeyError, TypeError, ValueError):
            raise ValueError("malformed corpse probe object update") from None
        digest = hashlib.sha256(body).hexdigest()
        if (event.get("body_sha256") != digest or event.get("error")
                or not isinstance(event.get("sequence"), int)):
            raise ValueError("invalid corpse probe source evidence")
        if len(body) < 11 or body[6] & 0x7F:
            raise ValueError("truncated or invalid corpse probe update header")
        count, map_id = struct.unpack_from("<IH", body)
        offset = 7
        if body[6] & 0x80:
            if offset + 6 > len(body):
                raise ValueError("truncated corpse probe removal header")
            destroyed, removed = struct.unpack_from("<HI", body, offset)
            offset += 6
            if destroyed > removed or removed > (len(body) - offset) // 2:
                raise ValueError("invalid corpse probe removal count")
            for _ in range(removed):
                _, offset = _packed_guid(body, offset, len(body))
        if offset + 4 > len(body):
            raise ValueError("truncated corpse probe block length")
        length = struct.unpack_from("<I", body, offset)[0]
        start = offset + 4
        if length != len(body) - start:
            raise ValueError("invalid corpse probe block span")
        if not count or map_id != int(victim["map"]):
            continue
        # No general movement decoder: this operation supports stationary corpses only.
        for block in range(start, len(body) - 2):
            if body[block] not in (1, 2):
                continue
            try:
                guid, pos = _packed_guid(body, block + 1, len(body))
            except ValueError:
                continue
            if not guid["low"] or guid["high"] >> 58 != 14 or body[pos:pos + 1] != b"\x0a":
                continue
            pos += 1
            if body[pos:pos + 7] != b"\x04\x00\x00\x00\x00\x00\x00":
                raise ValueError("corpse probe requires stationary movement and no pause times")
            pos += 7
            if pos + 20 > len(body):
                raise ValueError("truncated corpse probe position")
            x, y, z, orientation, size = struct.unpack_from("<ffffI", body, pos)
            pos += 20
            end = pos + size
            if end > len(body) or size < 118:
                raise ValueError("truncated corpse probe values")
            if not all(math.isfinite(value) for value in (x, y, z, orientation)):
                raise ValueError("nonfinite corpse probe position")
            if body[pos] != 0 or struct.unpack_from("<i", body, pos + 1)[0] != 0:
                raise ValueError("invalid corpse probe ObjectData prefix")
            scale = struct.unpack_from("<f", body, pos + 9)[0]
            if not math.isfinite(scale) or scale < 0:
                raise ValueError("invalid corpse probe ObjectData scale")
            # Visibility flags + ObjectData (12) + CorpseData.DynamicFlags (4).
            corpse_owner, cursor = _packed_guid(body, pos + 17, end)
            _, cursor = _packed_guid(body, cursor, end)  # PartyGUID
            _, cursor = _packed_guid(body, cursor, end)  # GuildGUID
            cursor += 4 + 19 * 4 + 3  # DisplayID, Items, RaceID/Sex/Class
            if cursor + 12 > end:
                raise ValueError("truncated corpse probe appearance")
            choices = struct.unpack_from("<I", body, cursor)[0]
            if cursor + 12 + choices * 8 != end:
                raise ValueError("invalid corpse probe customization span")
            if corpse_owner != owner or math.dist((x, y, z), victim["home"]) > 5.0:
                continue
            candidates[(guid["low"], guid["high"])] = {
                "guid": guid, "map_id": map_id, "owner": corpse_owner,
                "position": {"x": x, "y": y, "z": z, "orientation": orientation},
                "source_event_sequence": event["sequence"],
                "source_connection": event["connection"],
                "source_connection_sequence": event.get("connection_sequence"),
                "source_event_opcode": event["opcode"], "source_event_sha256": digest,
                "block_offset": block, "block_sha256": hashlib.sha256(body[block:end]).hexdigest(),
            }
    if len(candidates) != 1:
        raise ValueError("corpse probe did not identify one unique journal-owned nearby corpse")
    return next(iter(candidates.values()))
