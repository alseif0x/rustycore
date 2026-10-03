import hashlib
import json
import struct
import unittest
from forever_name_availability_probe import (
    BUILD,
    OPCODE,
    RESPONSE_OPCODE,
    inspect,
    inspect_response,
)


def capture(name=b"gear", surname=b"fd", flags=0):
    bits = (len(name) << 10) | (flags << 7) | (len(surname) << 1)
    payload = struct.pack("<I", 42) + bits.to_bytes(2, "big") + name + surname
    return b"FNR1" + struct.pack("<III", BUILD, OPCODE, len(payload)) + payload


def response(request, raw_result=0, sequence=None, build=BUILD,
             opcode=RESPONSE_OPCODE, payload_length=8, magic=b"FNS1"):
    if sequence is None:
        sequence = struct.unpack_from("<I", request, 16)[0]
    payload = struct.pack("<II", sequence, raw_result)
    return magic + struct.pack("<III", build, opcode, payload_length) + payload


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

    def test_response_metadata_accepts_source_result_codes_without_claims(self):
        request = capture()
        for raw_result in (0, 27, *range(98, 108)):
            result = inspect_response(request, response(request, raw_result))
            self.assertEqual(result["response_result"], raw_result)
            self.assertEqual(result["response_opcode"], RESPONSE_OPCODE)
            self.assertEqual(result["response_payload_bytes"], 8)
            self.assertEqual(result["response_sha256"],
                             hashlib.sha256(response(request, raw_result)).hexdigest())
            self.assertTrue(result["response_layout_matches"])
            self.assertTrue(result["sequence_matches"])
            self.assertFalse(result["names_rendered"])
            self.assertFalse(result["client_acceptance_proven"])
            self.assertFalse(result["creation_proven"])
            self.assertNotIn("response_sequence", result)
            self.assertNotIn("request_sequence", result)
            self.assertNotIn("gear", json.dumps(result))

    def test_response_rejects_truncation_trailing_wrong_metadata_and_sequence(self):
        request = capture()
        valid = response(request)
        for end in range(len(valid)):
            with self.assertRaises(ValueError): inspect_response(request, valid[:end])
        with self.assertRaises(ValueError): inspect_response(request, valid + b"extra")
        for offset, value in ((4, 70009), (8, 0x46001A), (12, 7)):
            changed = bytearray(valid)
            struct.pack_into("<I", changed, offset, value)
            with self.assertRaises(ValueError): inspect_response(request, changed)
        with self.assertRaises(ValueError): inspect_response(request, b"BAD!" + valid[4:])
        with self.assertRaises(ValueError): inspect_response(
            request, response(request, sequence=43))

    def test_response_requires_a_valid_request(self):
        valid = response(capture())
        with self.assertRaises(ValueError): inspect_response(b"invalid", valid)


if __name__ == "__main__":
    unittest.main()
