#!/usr/bin/env python3
"""One-shot, loopback-only world-connection probe, NOT a world server.

Accepts the native client's V2 connection preamble after a BNet join, then
closes before AuthChallenge, or validates only AuthSession shape when explicitly
requested. No digest verification, authentication bypass, account access, character
data or packet capture. Never advertise the fixture online outside this probe.
Reference: advocaite/TrinityCore 02245dcd, WorldSocket connection initialization.
"""
import argparse
import json
import socket
import secrets
import struct

from forever_bnet_join import VARIANT

SERVER_HELLO = b"WORLD OF WARCRAFT CONNECTION - SERVER TO CLIENT - V2\n"
CLIENT_HELLO = b"WORLD OF WARCRAFT CONNECTION - CLIENT TO SERVER - V2\n"


def read_exact(connection, count):
    received = bytearray()
    while len(received) < count:
        part = connection.recv(count - len(received))
        if not part:
            raise ValueError("client closed before bounded payload")
        received.extend(part)
    return bytes(received)


def auth_session_metadata(frame):
    if len(frame) < 4 + 81 or struct.unpack_from("<I", frame)[0] != 0x450001:
        raise ValueError("unexpected modern AuthSession frame")
    payload = frame[4:]
    region, district, realm = struct.unpack_from("<III", payload, 8)
    ticket_size = struct.unpack_from("<I", payload, 77)[0]
    if payload[76] & 0x7F or not 1 <= ticket_size <= 1024 or len(payload) != 81 + ticket_size:
        raise ValueError("invalid modern AuthSession ticket length")
    ticket = json.loads(payload[81:])
    expected = {"gameAccount": "1#1", "platform": VARIANT["platformType"],
                "clientArch": VARIANT["clientArch"], "type": VARIANT["type"]}
    if ticket != expected or (region, district, realm) != (2, 1, 1):
        raise ValueError("AuthSession differs from the isolated fixture")
    return {"auth_session_received": True, "opcode": 0x450001,
            "local_challenge_length": 32, "digest_length": 24,
            "ticket_variant": True, "realm_admission": True}


def exchange(connection, auth_challenge=False):
    connection.settimeout(10)
    connection.sendall(SERVER_HELLO)
    received = bytearray()
    while len(received) < len(CLIENT_HELLO):
        part = connection.recv(len(CLIENT_HELLO) - len(received))
        if not part:
            raise ValueError("client closed before connection preamble")
        received.extend(part)
    if received != CLIENT_HELLO:
        raise ValueError("unexpected client connection preamble")
    result = {"world_connection_preamble": True, "world_authentication_tested": False,
              "character_selection_tested": False}
    if auth_challenge:
        challenge = struct.pack("<I", 0x4D0000) + secrets.token_bytes(64) + b"\x01"
        connection.sendall(struct.pack("<I", len(challenge)) + bytes(12) + challenge)
        header = read_exact(connection, 16)
        size = struct.unpack_from("<I", header)[0]
        if not 4 <= size < 0x10000 or header[4:] != bytes(12):
            raise ValueError("invalid plain world frame")
        result.update(auth_session_metadata(read_exact(connection, size)))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ack-isolated-probe", action="store_true")
    parser.add_argument("--auth-challenge", action="store_true",
                        help="send the 70170 challenge, check AuthSession shape, then close")
    args = parser.parse_args()
    if not args.ack_isolated_probe:
        parser.error("--ack-isolated-probe is required")
    try:
        with socket.socket() as listener:
            # No reuse-port: an existing realm listener must make this fail.
            listener.bind(("127.0.0.1", 18085))
            listener.listen(1)
            listener.settimeout(120)
            print("Isolated world preamble probe ready on 127.0.0.1:18085", flush=True)
            connection, address = listener.accept()
            with connection:
                if address[0] != "127.0.0.1":
                    raise ValueError("non-loopback peer")
                result = exchange(connection, args.auth_challenge)
            print(json.dumps(result, sort_keys=True))
        return 0
    except Exception as error:
        print(f"forever_world_probe: FAIL ({type(error).__name__})")
        return 1


if __name__ == "__main__": raise SystemExit(main())
