#!/usr/bin/env python3
"""One-shot, loopback-only world-connection probe, NOT a world server.

Accepts the native client's V2 connection preamble after a BNet join, then
closes before AuthChallenge. No authentication bypass, account access, character
data or packet capture. Never advertise the fixture online outside this probe.
Reference: advocaite/TrinityCore 02245dcd, WorldSocket connection initialization.
"""
import argparse
import json
import socket

SERVER_HELLO = b"WORLD OF WARCRAFT CONNECTION - SERVER TO CLIENT - V2\n"
CLIENT_HELLO = b"WORLD OF WARCRAFT CONNECTION - CLIENT TO SERVER - V2\n"


def exchange(connection):
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
    return {"world_connection_preamble": True, "world_authentication_tested": False,
            "character_selection_tested": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ack-isolated-probe", action="store_true")
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
                result = exchange(connection)
            print(json.dumps(result, sort_keys=True))
        return 0
    except Exception as error:
        print(f"forever_world_probe: FAIL ({type(error).__name__})")
        return 1


if __name__ == "__main__": raise SystemExit(main())
