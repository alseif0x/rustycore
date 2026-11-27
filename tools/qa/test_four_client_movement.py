import tempfile
from pathlib import Path
import struct
import unittest

from verify_four_client_movement import PACKETS, verify, verify_relogin_positions


class FourClientMovementTest(unittest.TestCase):
    def test_relogin_matches_the_previous_position_on_each_players_own_connection(self):
        with tempfile.TemporaryDirectory() as root:
            directory = Path(root)
            expected = {actor: (float(actor), 1., 2.) for actor in (15, 16, 17, 18)}
            for actor, position in expected.items():
                frames = [
                    ("LoginVerifyWorld", 0x2597, struct.pack("<i4fI", 0, *position, 0., 0)),
                    ("PhaseShiftChange", 0x2578, bytes([1, 0xA0, actor, 4, 8])),
                ]
                for offset, (name, opcode, payload) in enumerate(frames):
                    data = struct.pack("<H", opcode) + payload
                    sequence = 2 * actor + offset
                    path = directory / f"rust-s2c-{sequence:08}-counter0-0x{opcode:04X}-{name}-len{len(data)}.bin"
                    path.write_bytes(data)
                    path.with_suffix(".meta").write_text(
                        f"addr=peer-{actor}\nconnection_id=1\nopcode=0x{opcode:04X}\nlen={len(data)}\n")
            self.assertEqual(verify_relogin_positions(directory, 30, 37, expected), expected)
            expected[15], expected[16] = expected[16], expected[15]
            with self.assertRaisesRegex(ValueError, "reloaded a different position"):
                verify_relogin_positions(directory, 30, 37, expected)

    def test_one_missing_observer_fails_even_when_every_sender_moves(self):
        players = (15, 16, 17, 18)
        with tempfile.TemporaryDirectory() as root:
            directory = Path(root)
            sequence = 0

            def emit(name, actor, recipient, x=0.0):
                nonlocal sequence
                sequence += 1
                direction, opcode = PACKETS[name]
                # A player GUID with realm 1 and the supplied low counter.
                guid = bytes([1, 0xA0, actor, 4, 8])
                payload = struct.pack("<H", opcode) + guid
                if name != "PhaseShiftChange":
                    payload += struct.pack("<IIII4f", 0, 0, 0, sequence, x, 0., 0., 0.)
                path = directory / f"rust-{direction}-{sequence:08}-counter0-0x{opcode:04X}-{name}-len{len(payload)}.bin"
                path.write_bytes(payload)
                path.with_suffix(".meta").write_text(f"addr=peer-{recipient}\nconnection_id=1\nopcode=0x{opcode:04X}\nlen={len(payload)}\n")
                return path

            for actor in players:
                emit("PhaseShiftChange", actor, actor)
            for x in (1.0, 2.0):
                for actor in players:
                    emit("MoveHeartbeat", actor, actor, x)
                    for peer in players:
                        if actor != peer:
                            last = emit("MoveUpdate", actor, peer, x)
            result = verify(directory, players, 0, sequence)
            self.assertEqual(sum(result["peer_deliveries"].values()), 24)
            last.unlink()
            with self.assertRaisesRegex(ValueError, "missed or reordered"):
                verify(directory, players, 0, sequence)


if __name__ == "__main__":
    unittest.main()
