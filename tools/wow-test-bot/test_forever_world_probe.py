import unittest
import json
import struct

import forever_world_probe as probe


class Peer:
    def __init__(self, chunks):
        self.chunks = iter(chunks)
        self.sent = b""

    def settimeout(self, timeout):
        self.timeout = timeout

    def sendall(self, data):
        self.sent += data

    def recv(self, size):
        result = next(self.chunks, b"")
        assert len(result) <= size
        return result


class PreambleTests(unittest.TestCase):
    def test_fragmented_preamble_does_not_claim_authentication(self):
        peer = Peer([probe.CLIENT_HELLO[:5], probe.CLIENT_HELLO[5:]])
        result = probe.exchange(peer)
        self.assertEqual(peer.sent, probe.SERVER_HELLO)
        self.assertTrue(result["world_connection_preamble"])
        self.assertFalse(result["world_authentication_tested"])
        self.assertFalse(result["character_selection_tested"])

    def test_wrong_and_truncated_preambles_fail(self):
        for chunks in ([b"x" * len(probe.CLIENT_HELLO)], [probe.CLIENT_HELLO[:-1]], []):
            with self.assertRaises(ValueError): probe.exchange(Peer(chunks))


def auth_frame():
    ticket = json.dumps({"gameAccount": "1#1", "platform": probe.VARIANT["platformType"],
                         "clientArch": probe.VARIANT["clientArch"], "type": probe.VARIANT["type"]}).encode()
    return (struct.pack("<IQIII", 0x450001, 0, 2, 1, 1) + bytes(56) + b"\0" +
            struct.pack("<I", len(ticket)) + ticket)


class AuthShapeTests(unittest.TestCase):
    def test_native_shape_does_not_claim_digest_verification(self):
        frame = auth_frame()
        header = struct.pack("<I", len(frame)) + bytes(12)
        peer = Peer([probe.CLIENT_HELLO, header[:2], header[2:], frame[:5], frame[5:]])
        result = probe.exchange(peer, auth_challenge=True)
        self.assertTrue(result["auth_session_received"])
        self.assertFalse(result["world_authentication_tested"])
        self.assertEqual(len(peer.sent), len(probe.SERVER_HELLO) + 16 + 4 + 65)

    def test_truncation_trailing_overflow_and_padding_are_rejected(self):
        frame = auth_frame()
        for size in range(len(frame)):
            with self.assertRaises((ValueError, UnicodeError)): probe.auth_session_metadata(frame[:size])
        bad_padding = bytearray(frame); bad_padding[4 + 76] = 1
        overflow = bytearray(frame); struct.pack_into("<I", overflow, 4 + 77, 0xFFFFFFFF)
        for candidate in (frame + b"x", bad_padding, overflow):
            with self.assertRaises(ValueError): probe.auth_session_metadata(candidate)

    def test_wrong_opcode_realm_and_variant_are_rejected(self):
        frame = auth_frame()
        for offset in (0, 12, 16, 20, len(frame) - 4):
            candidate = bytearray(frame); candidate[offset] ^= 1
            with self.assertRaises(ValueError): probe.auth_session_metadata(candidate)

    def test_nonzero_plain_tag_is_rejected(self):
        peer = Peer([probe.CLIENT_HELLO, struct.pack("<I", 4) + bytes([1]) * 12])
        with self.assertRaises(ValueError): probe.exchange(peer, auth_challenge=True)


if __name__ == "__main__": unittest.main()
