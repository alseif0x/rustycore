import unittest

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


if __name__ == "__main__": unittest.main()
