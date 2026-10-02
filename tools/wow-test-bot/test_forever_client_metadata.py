"""Unit tests for the bounded Forever client metadata parser; no client execution."""

import struct
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import forever_client_metadata as metadata


def _minimal_pe(section_size=0x4000):
    raw_pointer = 0x400
    data = bytearray(raw_pointer + section_size)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 0x3C, 0x80)
    data[0x80:0x84] = b"PE\0\0"
    struct.pack_into("<HHIIIHH", data, 0x84, 0x8664, 1, 0, 0, 0, 0xF0, 0x2022)
    optional = 0x98
    struct.pack_into("<H", data, optional, 0x20B)
    struct.pack_into("<Q", data, optional + 24, metadata.EXPECTED_IMAGE_BASE)
    struct.pack_into("<II", data, optional + 56, 0x5000, raw_pointer)
    section = optional + 0xF0
    data[section:section + 8] = b".rdata\0\0"
    struct.pack_into("<IIII", data, section + 8, section_size, 0x1000, section_size, raw_pointer)
    return data


def _put(data, rva, value):
    offset = 0x400 + (rva - 0x1000)
    data[offset:offset + len(value)] = value


def _put_u16(data, rva, value):
    _put(data, rva, struct.pack("<H", value))


def _put_u32(data, rva, value):
    _put(data, rva, struct.pack("<I", value))


def _put_ptr(data, rva, target):
    _put(data, rva, struct.pack("<Q", metadata.EXPECTED_IMAGE_BASE + target))


def _metadata_fixture():
    layout = metadata.MetadataLayout(0x1800, 0x1900, 0x1A00, (0x1B00, 0x1B10, 0x1B20))
    data = _minimal_pe()
    strings = {0x2800: "JSONSuperDistrictList", 0x2820: "JamJSONSuperDistrictEntry",
               0x2840: "superDistricts", 0x2860: "disallowLogin",
               0x2880: "superDistrictID", 0x28A0: "holdDownUntilTime"}
    for rva, value in strings.items():
        _put(data, rva, value.encode("ascii") + b"\0")
    _put_u32(data, layout.outer_descriptor_rva, 1)
    _put_ptr(data, layout.outer_descriptor_rva + 0x10, 0x1C00)
    _put_ptr(data, layout.outer_descriptor_rva + 0x18, 0x1D00)
    _put_ptr(data, layout.outer_descriptor_rva + 0x20, 0x1E00)
    _put_ptr(data, layout.outer_descriptor_rva + 0x50, 0x2800)
    _put_ptr(data, 0x1C00, 0x2840)
    _put_ptr(data, 0x1D00, layout.outer_type_rva)
    _put_ptr(data, 0x1D08, layout.entry_descriptor_rva)
    _put_u16(data, 0x1E00, 0)
    _put_u32(data, layout.entry_descriptor_rva, 3)
    _put_ptr(data, layout.entry_descriptor_rva + 0x10, 0x1F00)
    _put_ptr(data, layout.entry_descriptor_rva + 0x18, 0x2000)
    _put_ptr(data, layout.entry_descriptor_rva + 0x20, 0x2100)
    _put_ptr(data, layout.entry_descriptor_rva + 0x50, 0x2820)
    for index, string_rva in enumerate((0x2860, 0x2880, 0x28A0)):
        _put_ptr(data, 0x1F00 + index * 8, string_rva)
    for index, type_rva in enumerate(layout.entry_type_rvas):
        _put_ptr(data, 0x2000 + index * 8, type_rva)
    for index, offset in enumerate((8, 0, 4)):
        _put_u16(data, 0x2100 + index * 2, offset)
    for type_rva in (layout.outer_type_rva, *layout.entry_type_rvas):
        _put(data, type_rva, b"type")
    return bytes(data), layout


class PEParserTests(unittest.TestCase):
    def test_pe64_image_base_and_rva_translation(self):
        image = metadata.parse_pe(bytes(_minimal_pe()))
        self.assertEqual(image.image_base, metadata.EXPECTED_IMAGE_BASE)
        self.assertEqual(image.rva_to_offset(0x1000), 0x400)
        self.assertEqual(image.rva_to_offset(0x1000, 4), 0x400)
        with self.assertRaises(metadata.MetadataError):
            image.rva_to_offset(0x1000 + 0x4000)

    def test_invalid_pe_and_section_ranges_are_rejected(self):
        for bad in (b"", b"NO", b"MZ" + b"\0" * 100):
            with self.subTest(bad=bad), self.assertRaises(metadata.MetadataError):
                metadata.parse_pe(bad)
        data = _minimal_pe()
        struct.pack_into("<Q", data, 0x98 + 24, 0x150000000)
        with self.assertRaises(metadata.MetadataError):
            metadata.parse_pe(bytes(data))
        data = _minimal_pe()
        struct.pack_into("<I", data, 0x188 + 12, 0x4FFF)
        with self.assertRaises(metadata.MetadataError):
            metadata.parse_pe(bytes(data))


class SuperDistrictMetadataTests(unittest.TestCase):
    def test_descriptor_projection_is_sanitized_and_exact(self):
        data, layout = _metadata_fixture()
        report = metadata.extract_metadata(data, layout=layout)
        self.assertEqual(report["version"], [1, 60, 1])
        self.assertEqual(report["build"], 70170)
        self.assertNotIn("data", report)
        self.assertEqual(report["metadata"]["field_name"], "superDistricts")
        self.assertEqual([(field["name"], field["offset"], field["type"])
                          for field in report["metadata"]["fields"]],
                         [("disallowLogin", 8, "bool"),
                          ("superDistrictID", 0, "int32"),
                          ("holdDownUntilTime", 4, "uint32")])

    def test_metadata_descriptor_ranges_and_names_are_fail_closed(self):
        data, layout = _metadata_fixture()
        for offset in (0x50, 0x10, 0x18, 0x20):
            bad = bytearray(data)
            _put_ptr(bad, layout.entry_descriptor_rva + offset, 0x8000)
            with self.subTest(offset=offset), self.assertRaises(metadata.MetadataError):
                metadata.extract_metadata(bytes(bad), layout=layout)
        bad = bytearray(data)
        _put_u16(bad, 0x2100, 7)
        with self.assertRaises(metadata.MetadataError):
            metadata.extract_metadata(bytes(bad), layout=layout)

    def test_hash_is_checked_before_pe_interpretation(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "WowB.exe"
            path.write_bytes(b"not the approved client")
            with patch.object(metadata, "extract_metadata", side_effect=AssertionError("must not parse")):
                with self.assertRaises(metadata.MetadataError) as error:
                    metadata.read_verified_client(path)
            self.assertIn("SHA-256", str(error.exception))


if __name__ == "__main__":
    unittest.main()
