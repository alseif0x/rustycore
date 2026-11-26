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
    snapshot = None
    if flag:
        # SpellCastLogData: health, AP, SP, armor, 9-bit count and power rows.
        require(data[21:23] == b"\x00\x80", "expected one primary power row")
        health, ap, sp, armor = struct.unpack_from("<qiii", data, 1)
        power_type, power, cost = struct.unpack_from("<iii", data, 23)
        require(cost == 0, "nonzero melee power cost")
        snapshot = dict(health=health, attack_power=ap, spell_power=sp, armor=armor,
                        power_type=power_type, power=power, cost=cost)
        offset = 35
    length, = struct.unpack_from("<I", data, offset)
    payload = data[offset + 4:]
    require(len(payload) == length, "invalid attack round size")
    return payload, snapshot


OPCODES = {
    "PhaseShiftChange": ("s2c", 1, 0x2578),
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


def verify(directory, actor, observer, entry, first, last, actor_mode, observer_mode):
    # LoginVerifyWorld names the map. PhaseShiftChange confirms the full GUID
    # of the recipient on that same INSTANCE stream.
    maps = {}
    sessions = {}
    combatants = None
    streams = defaultdict(list)
    for seq, name, socket, data in frames(directory):
        if last is not None and seq > last:
            break
        if name == "LoginVerifyWorld":
            for counter, (_, old_socket, _) in list(sessions.items()):
                if old_socket == socket:
                    require(combatants is None, "world entry during encounter")
                    del sessions[counter]
            maps[socket] = struct.unpack_from("<i", data)[0]
        elif name == "PhaseShiftChange" and socket in maps:
            identity, _ = guid(data)
            require(identity[1] >> 58 == 2, "phase message is missing a player GUID")
            if identity[0] in (actor, observer):
                session = (identity, socket, maps[socket])
                if combatants:
                    require(sessions.get(identity[0]) == session, "player reconnected during encounter")
                sessions[identity[0]] = session
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
            payload, snapshot = round_info(data) if name == "AttackerStateUpdate" else (data, None)
            source, pos = guid(payload, 4 if name == "AttackerStateUpdate" else 0)
            victim, pos = guid(payload, pos)
            if (source, victim) not in (combatants, combatants[::-1]):
                continue
            damage = struct.unpack_from("<i", payload, pos)[0] if name == "AttackerStateUpdate" else None
            if damage is not None:
                mode = actor_mode if socket == actor_socket else observer_mode
                require((snapshot is not None) == (mode == "full"), "unexpected combat logging mode")
            signature = (name, source[0], victim[0], damage, hashlib.sha256(payload).hexdigest())
            streams[actor if socket == actor_socket else observer].append((signature, seq, snapshot))
    require(combatants is not None, "no actor swing against the requested creature")
    actor_events, observer_events = streams[actor], streams[observer]
    require([e[0] for e in actor_events] == [e[0] for e in observer_events], "missing, extra or reordered combat event at the observer")
    for source, victim in (combatants, combatants[::-1]):
        pair = [e[0] for e in actor_events if e[0][1:3] == (source[0], victim[0])]
        require(pair and pair[0][0] == "AttackStart" and sum(e[0] == "AttackStart" for e in pair) == 1,
                "expected one attack start before damage")
        require(any(e[0] == "AttackerStateUpdate" and e[3] > 0 for e in pair), "no positive damage for one direction")
    return [dict(opcode=s[0], attacker=s[1], victim=s[2], damage=s[3], round_sha256=s[4],
                 actor_frame=a, observer_frame=o, actor_log=af, observer_log=of)
            for (s, a, af), (_, o, of) in zip(actor_events, observer_events)]


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    for option in ("actor", "observer", "entry"):
        parser.add_argument(f"--{option}", required=True, type=int)
    parser.add_argument("--first", default=0, type=int, help="first combat frame to consider")
    parser.add_argument("--last", type=int, help="last frame of the encounter, inclusive")
    parser.add_argument("--actor-mode", choices=("basic", "full"), required=True)
    parser.add_argument("--observer-mode", choices=("basic", "full"), required=True)
    args = parser.parse_args()
    try:
        print(json.dumps(verify(args.directory, args.actor, args.observer, args.entry, args.first,
                                args.last, args.actor_mode, args.observer_mode), indent=2))
    except (ValueError, IndexError, KeyError, struct.error) as error:
        parser.exit(1, f"Inconclusive or failed capture: {error}\n")
