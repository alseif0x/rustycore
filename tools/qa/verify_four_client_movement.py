#!/usr/bin/env python3
"""Check four authenticated 54261 clients' movement against all twelve peer streams."""

import argparse
from collections import defaultdict
import json
import math
from pathlib import Path
import struct

from verify_melee_observer import guid, require


PACKETS = {
    "PhaseShiftChange": ("s2c", 0x2578), "MoveHeartbeat": ("c2s", 0x3A10),
    "MoveUpdate": ("s2c", 0x2DE0),
}


def frames(directory, first, last, name, direction, opcode):
    for path in directory.glob(f"rust-{direction}-*-{name}-*.bin"):
        if "-unencrypted-" in path.name:
            continue
        seq = int(path.name.split("-")[2])
        if not first <= seq <= last:
            continue
        meta = dict(line.split("=", 1) for line in path.with_suffix(".meta").read_text().splitlines() if "=" in line)
        require(meta["connection_id"] == "1" and int(meta["opcode"], 16) == opcode, f"invalid {name} route")
        data = path.read_bytes()
        require(len(data) == int(meta["len"]) and int.from_bytes(data[:2], "little") == opcode,
                f"invalid {name} frame")
        yield seq, meta["addr"], data


def capture(directory, first, last):
    rows = []
    for name, (direction, opcode) in PACKETS.items():
        for seq, socket, data in frames(directory, first, last, name, direction, opcode):
            owner, offset = guid(data, 2)
            if owner[1] >> 58 != 2:
                continue
            position = None if name == "PhaseShiftChange" else struct.unpack_from("<4f", data, offset + 16)
            rows.append((seq, name, socket, owner, position))
    return sorted(rows)


def verify_relogin_positions(directory, first, last, expected):
    """Match each login snapshot to its own socket before comparing the previous logout position."""
    logins = {}
    for seq, socket, data in frames(directory, first, last, "LoginVerifyWorld", "s2c", 0x2597):
        require(socket not in logins and len(data) == 26, "duplicate or invalid login position")
        map_id, x, y, z = struct.unpack_from("<i3f", data, 2)
        require(map_id == 0, "unexpected relogin map")
        logins[socket] = (seq, (x, y, z))
    positions = {}
    sockets = {}
    for seq, socket, data in frames(directory, first, last, "PhaseShiftChange", *PACKETS["PhaseShiftChange"]):
        owner, _ = guid(data, 2)
        actor = owner[0]
        require(owner[1] >> 58 == 2 and actor in expected, "unexpected relogin identity")
        require(socket in logins and logins[socket][0] < seq, "missing preceding login position")
        require(actor not in sockets or sockets[actor] == socket, "ambiguous relogin identity")
        sockets[actor] = socket
        position = logins[socket][1]
        require(all(math.isclose(a, b, rel_tol=0, abs_tol=0.001)
                    for a, b in zip(position, expected[actor], strict=True)),
                f"character {actor} reloaded a different position")
        positions[actor] = position
    require(len(logins) == len(expected) == len(set(sockets.values())) and positions.keys() == expected.keys(),
            "incomplete relogin positions")
    return positions


def verify(directory, characters, first, last):
    require(len(set(characters)) == 4, "four different character counters are required")
    sockets, requests, deliveries = {}, defaultdict(list), defaultdict(list)
    for seq, name, socket, owner, position in capture(directory, first, last):
        actor = owner[0]
        if actor not in characters:
            continue
        if name == "PhaseShiftChange":
            require(socket not in sockets or sockets[socket] == owner, "socket changed character")
            require(owner not in sockets.values() or sockets.get(socket) == owner, "character reconnected during cycle")
            sockets[socket] = owner
        elif len(sockets) == 4:
            require(socket in sockets, "movement outside the four current sessions")
            if name == "MoveHeartbeat":
                require(owner == sockets[socket], "unexpected mover in character heartbeat")
                requests[actor].append((seq, position))
            else:
                deliveries[(actor, sockets[socket][0])].append((seq, position))

    require(len(sockets) == 4, "missing in-world phase identity")
    counts = {}
    for actor in characters:
        sent = requests[actor]
        require(len({position for _, position in sent}) >= 2, f"character {actor} did not change position")
        for peer in set(characters) - {actor}:
            received = iter(deliveries[(actor, peer)])
            for seq, position in sent:
                require(any(n > seq and pos == position for n, pos in received),
                        f"peer {peer} missed or reordered a position from {actor} (frame {seq})")
            counts[f"{actor}->{peer}"] = len(sent)
    return {"clients": len(sockets), "heartbeats": sum(map(len, requests.values())), "peer_deliveries": counts}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("characters", type=int, nargs=4)
    parser.add_argument("--first", type=int, default=0)
    parser.add_argument("--last", type=int, default=2**64 - 1)
    args = parser.parse_args()
    print(json.dumps(verify(args.directory, args.characters, args.first, args.last), sort_keys=True))
