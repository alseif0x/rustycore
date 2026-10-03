import json
import struct
import tempfile
import unittest
from pathlib import Path
from forever_character_create_probe import BUILD, OPCODE, MAX_PAYLOAD, inspect, read_private


def capture(template=False, pairs=((20, 30), (10, 40))):
    # Pure synthetic two-byte names, all four flags and the unknown bits set.
    payload = bytes((8 | (2 if template else 0) | 1, 0xF0, 0x80, 1, 1, 0))
    payload += struct.pack("<IiI", len(pairs), -1, 0) + b"AbCd"
    if template:
        payload += struct.pack("<I", 19)
    payload += b"".join(struct.pack("<II", *pair) for pair in pairs)
    return b"FCR1" + struct.pack("<III", BUILD, OPCODE, len(payload)) + payload


class CreateCaptureTests(unittest.TestCase):
    def test_flags_surname_template_customizations_and_no_names(self):
        for template in (False, True):
            result = inspect(capture(template))
            self.assertEqual(result["name_bytes"], 2)
            self.assertEqual(result["surname_bytes"], 2)
            self.assertEqual(result["template_present"], template)
            self.assertTrue(result["trial_boost"])
            self.assertTrue(result["use_npe"])
            self.assertTrue(result["hardcore_self_found"])
            self.assertEqual(result["unknown_flag_bits"], 3)
            self.assertEqual(result["unknown_int32"], -1)
            self.assertEqual(result["customization_count"], 2)
            self.assertFalse(result["options_sorted"])
            self.assertEqual(result["duplicate_options"], 0)
            self.assertNotIn("Ab", json.dumps(result))
            self.assertNotIn("Cd", json.dumps(result))
        self.assertEqual(inspect(capture(pairs=((20, 30), (20, 40))))["duplicate_options"], 1)

    def test_truncation_trailing_wrong_build_opcode_count_and_utf8(self):
        valid = capture()
        for end in range(len(valid)):
            with self.assertRaises(ValueError): inspect(valid[:end])
        with self.assertRaises(ValueError): inspect(valid + b"extra")
        for offset, value in ((4, 70009), (8, 0x440010), (12, MAX_PAYLOAD + 1), (22, 0xFFFFFFFF)):
            changed = bytearray(valid)
            struct.pack_into("<I", changed, offset, value)
            with self.assertRaises(ValueError): inspect(changed)
        changed = bytearray(valid)
        changed[34] = 0xFF
        with self.assertRaises(ValueError): inspect(changed)

    def test_private_scope_permissions_symlink_and_size(self):
        with tempfile.TemporaryDirectory(prefix="forever-create-inspect-") as directory:
            root = Path(directory)
            path = root / "capture.bin"
            path.write_bytes(capture())
            path.chmod(0o600)
            self.assertEqual(read_private(path, root), capture())
            path.chmod(0o644)
            with self.assertRaises(ValueError): read_private(path, root)
            path.chmod(0o600)
            link = root / "link.bin"
            link.symlink_to(path)
            with self.assertRaises(ValueError): read_private(link, root)
            scope = root / "other-scope"
            scope.mkdir()
            with self.assertRaises(ValueError): read_private(path, scope)
            path.write_bytes(b"X" * (MAX_PAYLOAD + 17))
            with self.assertRaises(ValueError): read_private(path, root)


if __name__ == "__main__":
    unittest.main()
