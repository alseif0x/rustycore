#!/usr/bin/env python3
"""Compare one real melee encounter across two authenticated 54261 INSTANCE streams."""

import argparse
from collections import defaultdict
import hashlib
import json
from pathlib import Path
import struct


def require(condition, message):
    if not condition:
        raise ValueError(message)


def guid(data, offset=0):
    low, high = struct.unpack_from("BB", data, offset)
    offset += 2
    parts = []
    for mask in (low, high):
        value = 0
        for index in range(8):
            if mask & (1 << index):
                value |= data[offset] << (8 * index)
                offset += 1
        parts.append(value)
    return tuple(parts), offset


def round_info(data):
    flag = data[0]
    require(flag in (0, 0x80), "invalid log-data flag")
    offset = 1
    if flag:
        # SpellCastLogData: health, AP, SP, armor, 9-bit count and power rows.
        count = int.from_bytes(data[21:23], "big") >> 7
        offset = 23 + 12 * count
    length, = struct.unpack_from("<I", data, offset)
    payload = data[offset + 4:]
    require(len(payload) == length, "invalid attack round size")
    return payload, bool(flag)


OPCODES = {
    "PlayerLogin": ("c2s", 0, 0x35EB),
    "LoginVerifyWorld": ("s2c", 1, 0x2597),
    "AttackSwing": ("c2s", 1, 0x3255),
    "AttackStart": ("s2c", 1, 0x293D),
    "AttackerStateUpdate": ("s2c", 1, 0x2952),
}


def frames(directory):
    files = []
    for name in OPCODES:
        for path in directory.glob(f"*-{name}-*.bin"):
            if "-unencrypted-" in path.name:
                continue
            meta = dict(line.split("=", 1) for line in path.with_suffix(".meta").read_text().splitlines() if "=" in line)
            route = (meta["direction"], int(meta["connection_id"]), int(meta["opcode"], 16))
            require(route == OPCODES[name], f"wrong opcode or route for {name}")
            data = path.read_bytes()
            require(len(data) == int(meta["len"]) and int.from_bytes(data[:2], "little") == route[2], f"invalid {name} frame")
            files.append((int(meta["seq"]), name, meta["addr"], data[2:]))
    return sorted(files)


def verify(directory, actor, observer, entry, first):
    pending = None
    sessions = {}
    combatants = None
    streams = defaultdict(list)
    for seq, name, socket, data in frames(directory):
        if name == "PlayerLogin":
            require(pending is None, "interleaved logins cannot identify the receiving sockets")
            pending = guid(data)[0]
        elif name == "LoginVerifyWorld":
            require(pending is not None, "world entry without a matching login")
            sessions[pending[0]] = (pending, socket, struct.unpack_from("<i", data)[0])
            pending = None
        elif seq >= first and actor in sessions and observer in sessions:
            player, actor_socket, actor_map = sessions[actor]
            _, observer_socket, observer_map = sessions[observer]
            require(actor_map == observer_map and actor_socket != observer_socket, "expected distinct players on one map")
            if name == "AttackSwing" and socket == actor_socket:
                target, _ = guid(data)
                if ((target[1] >> 6) & 0x7FFFFF) == entry:
                    require(combatants in (None, (player, target)), "multiple targets in this window")
                    combatants = (player, target)
                continue
            if not combatants or socket not in (actor_socket, observer_socket) or name == "AttackSwing":
                continue
            payload, advanced = round_info(data) if name == "AttackerStateUpdate" else (data, False)
            source, pos = guid(payload, 4 if name == "AttackerStateUpdate" else 0)
            victim, pos = guid(payload, pos)
            if (source, victim) not in (combatants, combatants[::-1]):
                continue
            damage = struct.unpack_from("<i", payload, pos)[0] if name == "AttackerStateUpdate" else None
            signature = (name, source[0], victim[0], damage, hashlib.sha256(payload).hexdigest())
            streams[actor if socket == actor_socket else observer].append((signature, seq, advanced))
    require(combatants is not None, "no actor swing against the requested creature")
    actor_events, observer_events = streams[actor], streams[observer]
    require([e[0] for e in actor_events] == [e[0] for e in observer_events], "missing, extra or reordered combat event at the observer")
    for source, victim in (combatants, combatants[::-1]):
        pair = [e[0] for e in actor_events if e[0][1:3] == (source[0], victim[0])]
        require(pair and pair[0][0] == "AttackStart", "missing attack start before damage")
        require(any(e[0] == "AttackerStateUpdate" and e[3] > 0 for e in pair), "no positive damage for one direction")
    return [dict(opcode=s[0], attacker=s[1], victim=s[2], damage=s[3], round_sha256=s[4],
                 actor_frame=a, observer_frame=o, actor_full=af, observer_full=of)
            for (s, a, af), (_, o, of) in zip(actor_events, observer_events)]


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    for option in ("actor", "observer", "entry"):
        parser.add_argument(f"--{option}", required=True, type=int)
    parser.add_argument("--first", default=0, type=int, help="first combat frame to consider")
    args = parser.parse_args()
    try:
        print(json.dumps(verify(args.directory, args.actor, args.observer, args.entry, args.first), indent=2))
    except (ValueError, IndexError, KeyError, struct.error) as error:
        parser.exit(1, f"Inconclusive or failed capture: {error}\n")
