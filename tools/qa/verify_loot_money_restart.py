#!/usr/bin/env python3
"""Check the 54261 loot-money threshold across two distinct world process captures."""

import argparse
import json
from pathlib import Path
import struct

from verify_melee_observer import guid, require


OPCODES = {
    "AllAchievementData": 0x2570, "CoinRemoved": 0x2617, "CriteriaUpdate": 0x26E1,
    "AchievementEarned": 0x2643, "LootMoneyNotify": 0x261C,
}


def frames(directory):
    result = []
    for name, opcode in OPCODES.items():
        for path in directory.glob(f"rust-s2c-*-{name}-*.bin"):
            if "-unencrypted-" in path.name:
                continue
            meta = dict(line.split("=", 1) for line in path.with_suffix(".meta").read_text().splitlines() if "=" in line)
            data = path.read_bytes()
            require(meta["connection_id"] == "1" and meta["direction"] == "s2c", f"wrong route for {name}")
            require(int(meta["opcode"], 16) == opcode == int.from_bytes(data[:2], "little"), f"wrong {name} opcode")
            require(len(data) == int(meta["len"]), f"truncated {name}")
            result.append((int(meta["seq"]), name, meta["addr"], data[2:]))
    result.sort()
    claimants = {row[2] for row in result if row[1] == "LootMoneyNotify"}
    require(len(claimants) == 1, "expected one claimant per capture")
    return [row for row in result if row[2] in claimants]


def progress(data, offset):
    criterion, counter = struct.unpack_from("<IQ", data, offset)
    owner, offset = guid(data, offset + 12)
    unused, flags, date, elapsed, created, raf = struct.unpack_from("<IIIqqB", data, offset)
    require((unused, flags, elapsed, created, raf) == (0, 0, 0, 0, 0), "unexpected untimed criterion fields")
    return (criterion, counter, owner, date), offset + 29


def snapshot(data):
    count, progress_count = struct.unpack_from("<II", data)
    awards, criteria, offset = {}, {}, 8
    for _ in range(count):
        achievement, date = struct.unpack_from("<II", data, offset)
        owner, offset = guid(data, offset + 8)
        realms = struct.unpack_from("<II", data, offset)
        require(achievement not in awards, "duplicate achievement in snapshot")
        awards[achievement] = (owner, date, realms)
        offset += 8
    for _ in range(progress_count):
        row, offset = progress(data, offset)
        require(row[0] not in criteria, "duplicate criterion in snapshot")
        criteria[row[0]] = row
    require(offset == len(data), "extra login achievement data")
    return awards, criteria


def verify(before, after, character):
    first, second = frames(before), frames(after)
    expected = ["AllAchievementData", "CoinRemoved", "CriteriaUpdate", "AchievementEarned", "LootMoneyNotify"]
    require([row[1] for row in first] == expected, "missing, duplicated or out-of-order first claim packets")
    require([row[1] for row in second] == ["AllAchievementData", "CoinRemoved", "LootMoneyNotify"],
            "reopening the spent chest emitted an extra achievement or payout packet")
    initial_awards, initial = snapshot(first[0][3])
    final_awards, final = snapshot(second[0][3])
    require(4093 in initial and initial[4093][1] == 999_995 and 1176 not in initial_awards, "incorrect seeded login")
    row, end = progress(first[2][3], 0)
    require(end == len(first[2][3]) and final.get(4093) == row, "login did not restore the live criterion")
    owner = initial[4093][2]
    require(owner[0] == character and owner[1] != 0, "wrong character GUID")
    require(row[:3] == (4093, 1_000_005, owner), "wrong live criterion, character or quantity")
    data = first[3][3]
    sender, offset = guid(data)
    earner, offset = guid(data, offset)
    achievement, date, native, virtual, flag = struct.unpack_from("<IIIIB", data, offset)
    require(offset + 17 == len(data) and sender == earner == owner, "invalid award owner or length")
    require((achievement, date, flag) == (1176, row[3], 0) and native == virtual != 0, "invalid award")
    require(final_awards.get(1176) == (owner, date, (native, virtual)), "award changed across restart")
    for stream, amount in ((first, 10), (second, 0)):
        require(stream[-1][3] == struct.pack("<QQB", amount, 0, 0x80), "wrong solo payout fields")
    return {"character": character, "criterion": 4093, "before": 999_995, "after_relogin": row[1],
            "earned": achievement, "restored_packed_date": date, "second_payout": 0}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before", type=Path)
    parser.add_argument("after", type=Path)
    parser.add_argument("--character", type=int, default=15)
    args = parser.parse_args()
    print(json.dumps(verify(args.before, args.after, args.character), sort_keys=True))
