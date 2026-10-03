#!/usr/bin/env python3
"""Read-only metadata inspection of a private 70170 name-availability request.

Source 02245dcd CharacterPackets.cpp:464-484 (70009 annotation). Matching the
layout is not name-policy, availability, reservation or creation-success proof.
An optional FNS1 response check validates only the captured wire envelope and
correlates its sequence; it does not establish client acceptance.
"""
import argparse
import hashlib
import json
import struct
from pathlib import Path
from forever_character_create_probe import read_private

BUILD, OPCODE, MAX_PAYLOAD = 70170, 0x440071, 132
RESPONSE_OPCODE = 0x46001B
RESPONSE_BYTES = 24
RESPONSE_PAYLOAD_BYTES = 8


def inspect(data):
    if len(data) < 22 or data[:4] != b"FNR1":
        raise ValueError("invalid name capture header")
    build, opcode, length = struct.unpack_from("<III", data, 4)
    if build != BUILD or opcode != OPCODE or length > MAX_PAYLOAD or len(data) != 16 + length:
        raise ValueError("wrong build/opcode/length")
    payload = memoryview(data)[16:]
    # MSB-first 6-bit name, 3 unvalidated bits, 6-bit surname, 1 alignment bit.
    name_bytes = payload[4] >> 2
    unknown_bits = ((payload[4] & 3) << 1) | (payload[5] >> 7)
    surname_bytes = (payload[5] >> 1) & 63
    if length != 6 + name_bytes + surname_bytes:
        raise ValueError("name layout length mismatch")
    try:
        bytes(payload[6:6 + name_bytes]).decode("utf-8")
        bytes(payload[6 + name_bytes:]).decode("utf-8")
    except UnicodeDecodeError as error:
        raise ValueError("capture names are not UTF-8") from error
    return {
        "build": build, "opcode": opcode, "payload_bytes": length,
        "name_bytes": name_bytes, "surname_bytes": surname_bytes,
        "unknown_bits": unknown_bits,
        "capture_sha256": hashlib.sha256(data).hexdigest(),
        "names_rendered": False, "availability_proven": False,
    }


def inspect_response(request_bytes, response_bytes):
    """Validate one FNR1 request and its exact FNS1 response metadata-only."""
    request = inspect(request_bytes)
    if len(response_bytes) != RESPONSE_BYTES or response_bytes[:4] != b"FNS1":
        raise ValueError("invalid name response envelope")
    build, opcode, length = struct.unpack_from("<III", response_bytes, 4)
    if build != BUILD or opcode != RESPONSE_OPCODE or length != RESPONSE_PAYLOAD_BYTES:
        raise ValueError("wrong name response build/opcode/length")
    response_sequence, raw_result = struct.unpack_from("<II", response_bytes, 16)
    request_sequence = struct.unpack_from("<I", request_bytes, 16)[0]
    if response_sequence != request_sequence:
        raise ValueError("name response sequence mismatch")
    request.update({
        "response_build": build,
        "response_opcode": opcode,
        "response_payload_bytes": length,
        "response_result": raw_result,
        "response_sha256": hashlib.sha256(response_bytes).hexdigest(),
        "response_layout_matches": True,
        "sequence_matches": True,
        "client_acceptance_proven": False,
        "creation_proven": False,
    })
    return request


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--private-capture", required=True, type=Path)
    parser.add_argument("--private-response", type=Path)
    args = parser.parse_args()
    try:
        request = read_private(args.private_capture)
        if args.private_response is None:
            result = inspect(request)
        else:
            result = inspect_response(request, read_private(args.private_response))
    except (OSError, ValueError, struct.error):
        message = (
            "Private name capture/response rejected; no contents rendered.\n"
            if args.private_response is not None
            else "Private name capture rejected; no contents rendered.\n"
        )
        parser.exit(1, message)
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
