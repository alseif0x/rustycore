#!/usr/bin/env python3
"""Verify the Northshire Phase Test fixture on two 54261 instance streams."""

import argparse
from pathlib import Path
import struct

from verify_melee_observer import guid, require


def capture(directory):
    frames = []
    for path in directory.glob("*.meta"):
        meta = dict(line.split("=", 1) for line in path.read_text().splitlines() if "=" in line)
        if meta["connection_id"] != "1" or "-unencrypted-" in path.name:
            continue
        payload = path.with_suffix(".bin").read_bytes()
        require(len(payload) == int(meta["len"]), "incomplete capture")
        require(int.from_bytes(payload[:2], "little") == int(meta["opcode"], 16), "wrong opcode")
        frames.append((int(meta["seq"]), meta["name"], meta["addr"], payload[2:]))
    return sorted(frames)


def phases(data):
    player, offset = guid(data)
    count, = struct.unpack_from("<I", data, offset + 4)
    _, offset = guid(data, offset + 8)
    return player, [struct.unpack_from("<H", data, offset + 4*i + 2)[0] for i in range(count)]


def removed(data, entry):
    require(data[6] & 128, "missing removal list")
    destroyed, count = struct.unpack_from("<HI", data, 7)
    offset = 13
    found = {}
    for index in range(count):
        start = offset
        (_, high), offset = guid(data, start)
        kind = high >> 58
        if (high >> 6) & 0x7fffff == entry and kind in (8, 11):
            require(index >= destroyed and kind not in found, "unexpected fixture removal")
            found[kind] = data[start:offset]
    require(set(found) == {8, 11}, "missing creature or game object")
    return found


def creates_any(packets, objects):
    for kind, packed in objects.items():
        object_type = 5 if kind == 8 else 8
        headers = [bytes([block]) + packed + bytes([object_type]) for block in (1, 2)]
        require(any(header in packet for packet in packets for header in headers),
                "missing create header for the original GUID")


def verify(directory):
    frames = capture(directory)
    shifts = [(n, socket, *phases(data)) for n, name, socket, data in frames if name == "PhaseShiftChange"]
    sockets = {}
    for player in (15, 16):
        peers = {socket for _, socket, identity, _ in shifts if identity[0] == player and identity[1] >> 58 == 2}
        require(len(peers) == 1, f"expected one instance stream for player {player}")
        sockets[player] = peers.pop()
    control, mage = sockets[15], sockets[16]
    require(control != mage, "players must have distinct instance sockets")

    def events(name, peer):
        return [(n, data) for n, op, socket, data in frames if op == name and socket == peer]

    (control_login, pos), = events("LoginVerifyWorld", control)
    (mage_login, mage_pos), = events("LoginVerifyWorld", mage)
    require(control_login < mage_login and pos == mage_pos, "players did not enter together at the fixture")
    (accept, request), = events("QuestGiverAcceptQuest", mage)
    _, offset = guid(request)
    require(struct.unpack_from("<I", request, offset)[0] == 3510000, "wrong quest")
    (abandon, _), = events("QuestLogRemoveQuest", mage)
    changes = [(n, ids) for n, peer, _, ids in shifts if peer == mage and n >= accept]
    require([ids for _, ids in changes] == [[198], []], "expected phase grant and clear")
    grant, clear = (n for n, _ in changes)
    updates = events("UpdateObject", mage)
    created, grant_data = next(pair for pair in updates if pair[0] > grant)
    restored, clear_data = next(pair for pair in updates if pair[0] > clear)
    (wave, _), = events("SendTextEmote", mage)
    require(mage_login < accept < grant < created < abandon < clear < restored < wave, "wrong action order")
    restricted = removed(clear_data, 3510001)
    controls = removed(grant_data, 3510002)
    creates_any([grant_data], restricted)
    creates_any([clear_data], controls)
    require(all(not ids for _, peer, _, ids in shifts if peer == control), "control changed phase")
    control_updates = events("UpdateObject", control)
    creates_any([data for n, data in control_updates if n < accept], controls)
    require(all(packed not in data for n, data in control_updates if n <= wave
                for packed in restricted.values()), "control received a restricted GUID")
    echo = next(data for n, data in events("TextEmote", mage) if n > wave)
    require(any(n > wave and data == echo for n, data in events("TextEmote", control)),
            "missing shared emote after visibility was restored")
    print(f"Verified paired phase grant {grant}, create {created}, clear {clear}, removal {restored}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    verify(parser.parse_args().directory)
