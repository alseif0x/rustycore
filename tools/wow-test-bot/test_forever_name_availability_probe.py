import json
import struct
import unittest
from forever_name_availability_probe import BUILD, OPCODE, inspect


def capture(name=b"gear", surname=b"fd", flags=0):
    bits = (len(name) << 10) | (flags << 7) | (len(surname) << 1)
    payload = struct.pack("<I", 42) + bits.to_bytes(2, "big") + name + surname
    return b"FNR1" + struct.pack("<III", BUILD, OPCODE, len(payload)) + payload


class NameCaptureTests(unittest.TestCase):
    def test_source_literal_and_no_private_names_or_sequence(self):
        self.assertEqual(capture()[20:22], bytes.fromhex("1004"))
        for flags in range(8):
            result = inspect(capture(flags=flags))
            self.assertEqual((result["name_bytes"], result["surname_bytes"]), (4, 2))
            self.assertEqual(result["unknown_bits"], flags)
            self.assertFalse(result["availability_proven"])
            self.assertFalse(result["names_rendered"])
            self.assertNotIn("gear", json.dumps(result))
            self.assertNotIn("sequence", result)

    def test_utf8_empty_and_six_bit_maximum(self):
        for name, surname in ((b"", b""), ("Málaga".encode(), "Łódź".encode()),
                              (b"N" * 63, b"S" * 63)):
            result = inspect(capture(name, surname))
            self.assertEqual(result["name_bytes"], len(name))
            self.assertEqual(result["surname_bytes"], len(surname))

    def test_rejects_truncation_trailing_wrong_metadata_and_utf8(self):
        valid = capture()
        for end in range(len(valid)):
            with self.assertRaises(ValueError): inspect(valid[:end])
        with self.assertRaises(ValueError): inspect(valid + b"extra")
        for offset, value in ((4, 70009), (8, 0x440070), (12, 133)):
            changed = bytearray(valid)
            struct.pack_into("<I", changed, offset, value)
            with self.assertRaises(ValueError): inspect(changed)
        with self.assertRaises(ValueError): inspect(capture(b"\xff"))


if __name__ == "__main__":
    unittest.main()
