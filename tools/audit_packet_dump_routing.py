#!/usr/bin/env python3
"""Audit Rust packet dump connection routing against the C++ opcode table."""

from __future__ import annotations

import argparse
import collections
import dataclasses
import pathlib
import sys


DEFAULT_OPCODE_TSV = pathlib.Path("docs/migration/inventory/cpp-server-opcodes.tsv")

# C++ can override the opcode table default at packet construction time. Keep
# these narrow and backed by an explicit callsite, not by Rust behavior.
CALLSITE_CONNECTION_OVERRIDES = {
    # CharacterHandler.cpp:1051:
    # SendPacket(WorldPackets::Auth::ResumeComms(CONNECTION_TYPE_INSTANCE).Write());
    0x304B: "instance",
}


@dataclasses.dataclass(frozen=True)
class OpcodeInfo:
    cpp_name: str
    opcode: int
    expected_connection: str
    packet_class: str


@dataclasses.dataclass(frozen=True)
class MetaPacket:
    path: pathlib.Path
    direction: str
    addr: str
    seq: int
    counter: int
    opcode: int
    name: str
    length: int


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Compare s2c packet dump .meta routing against C++ "
            "CONNECTION_TYPE_REALM/INSTANCE expectations."
        )
    )
    parser.add_argument("dump_dir", type=pathlib.Path, help="Directory containing packet .meta files")
    parser.add_argument(
        "--opcodes",
        type=pathlib.Path,
        default=DEFAULT_OPCODE_TSV,
        help=f"C++ server opcode TSV, default: {DEFAULT_OPCODE_TSV}",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Exit with status 1 if routing mismatches are found",
    )
    return parser.parse_args()


def load_cpp_server_opcodes(path: pathlib.Path) -> dict[int, OpcodeInfo]:
    opcodes: dict[int, OpcodeInfo] = {}
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue

            parts = line.split("\t")
            if len(parts) < 6:
                continue

            _cpp_line, cpp_name, opcode_hex, _status, connection, packet_class = parts[:6]
            if opcode_hex == "0xBADD":
                continue
            if connection not in {"CONNECTION_TYPE_REALM", "CONNECTION_TYPE_INSTANCE"}:
                continue

            opcode = int(opcode_hex, 16)
            expected_connection = connection.removeprefix("CONNECTION_TYPE_").lower()
            expected_connection = CALLSITE_CONNECTION_OVERRIDES.get(opcode, expected_connection)
            opcodes[opcode] = OpcodeInfo(
                cpp_name=cpp_name,
                opcode=opcode,
                expected_connection=expected_connection,
                packet_class=packet_class,
            )
    return opcodes


def parse_meta(path: pathlib.Path) -> MetaPacket | None:
    values: dict[str, str] = {}
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            if "=" not in line:
                continue
            key, value = line.rstrip("\n").split("=", 1)
            values[key] = value

    try:
        return MetaPacket(
            path=path,
            direction=values["direction"],
            addr=values["addr"],
            seq=int(values["seq"]),
            counter=int(values["counter"]),
            opcode=int(values["opcode"], 16),
            name=values["name"],
            length=int(values["len"]),
        )
    except (KeyError, ValueError):
        return None


def load_meta_packets(dump_dir: pathlib.Path) -> list[MetaPacket]:
    packets: list[MetaPacket] = []
    for path in sorted(dump_dir.glob("*.meta")):
        packet = parse_meta(path)
        if packet is not None:
            packets.append(packet)
    return packets


def infer_connection_by_addr(
    packets: list[MetaPacket], cpp_opcodes: dict[int, OpcodeInfo]
) -> tuple[dict[str, str], dict[str, collections.Counter[str]]]:
    votes: dict[str, collections.Counter[str]] = collections.defaultdict(collections.Counter)
    for packet in packets:
        if packet.direction != "s2c":
            continue
        info = cpp_opcodes.get(packet.opcode)
        if info is None:
            continue
        votes[packet.addr][info.expected_connection] += 1

    mapping: dict[str, str] = {}
    for addr, counter in votes.items():
        realm = counter["realm"]
        instance = counter["instance"]
        if realm > instance:
            mapping[addr] = "realm"
        elif instance > realm:
            mapping[addr] = "instance"
        else:
            mapping[addr] = "ambiguous"

    return mapping, votes


def main() -> int:
    args = parse_args()
    cpp_opcodes = load_cpp_server_opcodes(args.opcodes)
    packets = load_meta_packets(args.dump_dir)
    s2c_packets = [packet for packet in packets if packet.direction == "s2c"]
    known_s2c_packets = [packet for packet in s2c_packets if packet.opcode in cpp_opcodes]
    addr_mapping, votes = infer_connection_by_addr(packets, cpp_opcodes)

    mismatches: list[tuple[MetaPacket, OpcodeInfo, str]] = []
    unknown_addr_packets: list[MetaPacket] = []

    for packet in known_s2c_packets:
        observed = addr_mapping.get(packet.addr)
        if observed is None or observed == "ambiguous":
            unknown_addr_packets.append(packet)
            continue

        info = cpp_opcodes[packet.opcode]
        if observed != info.expected_connection:
            mismatches.append((packet, info, observed))

    print(f"dump_dir={args.dump_dir}")
    print(f"meta_packets={len(packets)} s2c={len(s2c_packets)} known_s2c={len(known_s2c_packets)}")
    print()
    print("inferred_connections:")
    for addr in sorted(votes):
        counter = votes[addr]
        label = addr_mapping.get(addr, "unknown")
        print(
            f"  {addr}\tobserved={label}\t"
            f"realm_votes={counter['realm']}\tinstance_votes={counter['instance']}"
        )

    if unknown_addr_packets:
        print()
        print(f"unknown_or_ambiguous_addr_packets={len(unknown_addr_packets)}")

    grouped: dict[tuple[int, str, str, str], list[MetaPacket]] = collections.defaultdict(list)
    for packet, info, observed in mismatches:
        grouped[(packet.opcode, packet.name, info.expected_connection, observed)].append(packet)

    print()
    print(f"routing_mismatches={len(mismatches)} unique_opcodes={len(grouped)}")
    for (opcode, rust_name, expected, observed), group in sorted(grouped.items()):
        info = cpp_opcodes[opcode]
        examples = ", ".join(
            f"seq={packet.seq}/counter={packet.counter}/len={packet.length}" for packet in group[:5]
        )
        if len(group) > 5:
            examples += f", ... +{len(group) - 5}"
        print(
            f"  0x{opcode:04X}\t{rust_name}\tcpp={info.cpp_name}\t"
            f"expected={expected}\tobserved={observed}\tcount={len(group)}\t{examples}"
        )

    return 1 if args.strict and mismatches else 0


if __name__ == "__main__":
    sys.exit(main())
