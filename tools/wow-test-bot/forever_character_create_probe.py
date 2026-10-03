#!/usr/bin/env python3
"""Read-only, metadata-only inspection of one private build-70170 Create request.

Source: 02245dcd CharacterPackets.cpp:487-516 (annotated 70009 Classic layout).
Matching a capture is wire-layout evidence, not appearance/admission/save parity.
"""
import argparse
import hashlib
import json
import os
import stat
import struct
from pathlib import Path

BUILD, OPCODE, MAX_PAYLOAD = 70170, 0x440070, 64 * 1024
ROOT = Path(__file__).resolve().parents[2] / "target" / "forever-login"


def inspect(data):
    if len(data) < 16 or data[:4] != b"FCR1":
        raise ValueError("invalid capture header")
    build, opcode, length = struct.unpack_from("<III", data, 4)
    if build != BUILD or opcode != OPCODE or length > MAX_PAYLOAD or len(data) != 16 + length:
        raise ValueError("wrong build/opcode/length")
    payload = memoryview(data)[16:]
    if len(payload) < 18:
        raise ValueError("truncated create header")
    name_bytes = payload[0] >> 2
    has_template = bool(payload[0] & 2)
    surname_bytes = ((payload[1] & 15) << 2) | (payload[2] >> 6)
    count, unknown, season = struct.unpack_from("<IiI", payload, 6)
    expected = 18 + name_bytes + surname_bytes + (4 if has_template else 0) + 8 * count
    if len(payload) != expected:
        raise ValueError("create layout length mismatch")
    # Validate bounded string framing without returning or printing either name.
    try:
        bytes(payload[18:18 + name_bytes]).decode("utf-8")
        bytes(payload[18 + name_bytes:18 + name_bytes + surname_bytes]).decode("utf-8")
    except UnicodeDecodeError as error:
        raise ValueError("capture names are not UTF-8") from error
    offset = 18 + name_bytes + surname_bytes + (4 if has_template else 0)
    options = [struct.unpack_from("<II", payload, offset + index * 8)[0]
               for index in range(count)]
    return {
        "build": build, "opcode": opcode, "payload_bytes": length,
        "source_layout": "02245dcd Classic 70009; not a creation-success claim",
        "race": payload[3], "class": payload[4], "sex": payload[5],
        "name_bytes": name_bytes, "surname_bytes": surname_bytes,
        "template_present": has_template, "trial_boost": bool(payload[0] & 1),
        "use_npe": bool(payload[1] & 0x80), "hardcore_self_found": bool(payload[1] & 0x40),
        "unknown_flag_bits": (payload[1] >> 4) & 3, "unknown_int32": unknown,
        "timerunning_season": season, "customization_count": count,
        "options_sorted": options == sorted(options),
        "duplicate_options": count - len(set(options)),
        "capture_sha256": hashlib.sha256(data).hexdigest(),
        "names_and_choices_rendered": False,
    }


def read_private(path, root=ROOT):
    path, root = Path(path), Path(root).resolve(strict=True)
    if path.is_symlink() or not path.resolve(strict=True).is_relative_to(root):
        raise ValueError("capture must be a private regular file in the isolated runtime")
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(descriptor, "rb") as stream:
        info = os.fstat(stream.fileno())
        if not stat.S_ISREG(info.st_mode) or stat.S_IMODE(info.st_mode) & 0o077:
            raise ValueError("capture is not private/regular")
        if info.st_size > 16 + MAX_PAYLOAD:
            raise ValueError("capture exceeds bound")
        return stream.read(17 + MAX_PAYLOAD)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--private-capture", required=True, type=Path)
    args = parser.parse_args()
    try:
        result = inspect(read_private(args.private_capture))
    except (OSError, ValueError, struct.error):
        parser.exit(1, "Private create capture rejected; no contents rendered.\n")
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
