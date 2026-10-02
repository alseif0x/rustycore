#!/usr/bin/env python3
"""Read-only schema inspection for the original WoW Forever 1.60.1 client.

The command verifies the supplied executable before parsing it and emits only
sanitized discovery descriptor metadata.  It never executes the client and
does not print binary payloads.
"""

import argparse
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import struct
import sys


EXPECTED_SHA256 = "369ce842043f6177850947274287fc5a6cec3ee033a891faa0400a1d0a475d8e"
EXPECTED_IMAGE_BASE = 0x140000000
VERSION = (1, 60, 1)
BUILD = 70170

SUPER_DISTRICT_LIST_RVA = 0x49BD060
SUPER_DISTRICT_ENTRY_RVA = 0x49BCF50
SUPER_DISTRICT_LIST_TYPE_RVA = 0x4DFC2E0
SUPER_DISTRICT_TYPE_RVAS = (0x48E2ED0, 0x48E3420, 0x48E3530)
REALM_ENTRY_RVA = 0x49BDFD0
UTILITY_INFO_RVA = 0x49BD6A0
KNOWN_TYPE_NAMES = {
    0x48E2ED0: "bool",
    0x48E3420: "int32",
    0x48E3530: "uint32",
}
SUPER_DISTRICT_FIELDS = (
    ("disallowLogin", 8, "bool", 0x48E2ED0),
    ("superDistrictID", 0, "int32", 0x48E3420),
    ("holdDownUntilTime", 4, "uint32", 0x48E3530),
)
REALM_ENTRY_FIELDS = (
    ("wowRealmAddress", 0),
    ("useBleepChance", 320),
    ("cfgTimezonesID", 280),
    ("populationState", 312),
    ("cfgCategoriesID", 268),
    ("version", 296),
    ("cfgRealmsID", 4),
    ("gameServiceRegionId", 292),
    ("flags", 316),
    ("name", 8),
    ("cfgConfigsID", 276),
    ("cfgContentSetID", 284),
    ("cfgLanguagesID", 272),
    ("superDistrictID", 288),
)
UTILITY_INFO_FIELDS = (("realmPermissions", 0), ("loginLicenses", 8))


class MetadataError(ValueError):
    """A bounded, non-secret parse or validation failure."""


@dataclass(frozen=True)
class Section:
    virtual_address: int
    virtual_size: int
    raw_pointer: int
    raw_size: int


@dataclass(frozen=True)
class PEImage:
    data: bytes
    image_base: int
    size_of_image: int
    size_of_headers: int
    sections: tuple[Section, ...]

    def rva_to_offset(self, rva: int, size: int = 1) -> int:
        if not isinstance(rva, int) or not isinstance(size, int) or rva < 0 or size < 0:
            raise MetadataError("invalid RVA range")
        end = rva + size
        if end < rva:
            raise MetadataError("RVA range overflow")
        if rva < self.size_of_headers and end <= self.size_of_headers:
            if end <= len(self.data):
                return rva
            raise MetadataError("RVA exceeds PE headers")
        for section in self.sections:
            section_end = section.virtual_address + section.raw_size
            if section.virtual_address <= rva and end <= section_end:
                offset = section.raw_pointer + (rva - section.virtual_address)
                if offset + size <= section.raw_pointer + section.raw_size <= len(self.data):
                    return offset
        raise MetadataError("RVA is outside file-backed PE sections")

    def bytes_at(self, rva: int, size: int) -> bytes:
        offset = self.rva_to_offset(rva, size)
        return self.data[offset:offset + size]

    def u16(self, rva: int) -> int:
        return struct.unpack("<H", self.bytes_at(rva, 2))[0]

    def u32(self, rva: int) -> int:
        return struct.unpack("<I", self.bytes_at(rva, 4))[0]

    def u64(self, rva: int) -> int:
        return struct.unpack("<Q", self.bytes_at(rva, 8))[0]

    def pointer_rva(self, rva: int) -> int:
        value = self.u64(rva)
        if value < self.image_base or value - self.image_base > 0xFFFFFFFF:
            raise MetadataError("invalid PE pointer")
        return value - self.image_base

    def c_string(self, rva: int, limit: int = 256) -> str:
        if limit <= 0:
            raise MetadataError("invalid string limit")
        raw = bytearray()
        for index in range(limit):
            value = self.bytes_at(rva + index, 1)[0]
            if value == 0:
                try:
                    return raw.decode("ascii")
                except UnicodeDecodeError as error:
                    raise MetadataError("metadata name is not ASCII") from error
            raw.append(value)
        raise MetadataError("metadata name is unterminated")


@dataclass(frozen=True)
class MetadataLayout:
    """Descriptor addresses; the test fixture uses the same parser generically."""

    outer_descriptor_rva: int = SUPER_DISTRICT_LIST_RVA
    entry_descriptor_rva: int = SUPER_DISTRICT_ENTRY_RVA
    outer_type_rva: int = SUPER_DISTRICT_LIST_TYPE_RVA
    entry_type_rvas: tuple[int, int, int] = SUPER_DISTRICT_TYPE_RVAS
    realm_entry_descriptor_rva: int | None = None
    utility_info_descriptor_rva: int | None = None


FOREVER_LAYOUT = MetadataLayout(
    realm_entry_descriptor_rva=REALM_ENTRY_RVA,
    utility_info_descriptor_rva=UTILITY_INFO_RVA,
)


def parse_pe(data: bytes) -> PEImage:
    if len(data) < 0x40 or data[:2] != b"MZ":
        raise MetadataError("invalid DOS header")
    e_lfanew = struct.unpack_from("<I", data, 0x3C)[0]
    if e_lfanew < 0x40 or e_lfanew + 24 > len(data) or data[e_lfanew:e_lfanew + 4] != b"PE\0\0":
        raise MetadataError("invalid PE header")
    coff = e_lfanew + 4
    machine, section_count, _, _, _, optional_size, _ = struct.unpack_from("<HHIIIHH", data, coff)
    if machine != 0x8664 or not 1 <= section_count <= 96:
        raise MetadataError("PE is not bounded x64")
    optional = coff + 20
    if optional + optional_size > len(data) or optional_size < 112:
        raise MetadataError("truncated PE optional header")
    if struct.unpack_from("<H", data, optional)[0] != 0x20B:
        raise MetadataError("PE is not PE32+")
    image_base = struct.unpack_from("<Q", data, optional + 24)[0]
    if image_base != EXPECTED_IMAGE_BASE:
        raise MetadataError("unexpected PE image base")
    size_of_image, size_of_headers = struct.unpack_from("<II", data, optional + 56)
    if not size_of_image or not size_of_headers or size_of_headers > len(data):
        raise MetadataError("invalid PE image bounds")
    section_table = optional + optional_size
    table_end = section_table + section_count * 40
    if table_end > len(data) or table_end > size_of_headers:
        raise MetadataError("truncated PE section table")
    sections = []
    for index in range(section_count):
        header = section_table + index * 40
        virtual_size, virtual_address, raw_size, raw_pointer = struct.unpack_from("<IIII", data, header + 8)
        if virtual_address + max(virtual_size, raw_size) > size_of_image:
            raise MetadataError("PE section exceeds image")
        if raw_size:
            if raw_pointer < size_of_headers or raw_pointer + raw_size > len(data):
                raise MetadataError("PE section exceeds file")
            if virtual_address < size_of_headers:
                raise MetadataError("PE section overlaps headers")
        sections.append(Section(virtual_address, virtual_size, raw_pointer, raw_size))
    sections.sort(key=lambda section: section.virtual_address)
    for previous, current in zip(sections, sections[1:]):
        if previous.virtual_address + max(previous.virtual_size, previous.raw_size) > current.virtual_address:
            raise MetadataError("overlapping PE sections")
    return PEImage(bytes(data), image_base, size_of_image, size_of_headers, tuple(sections))


def _descriptor(pe: PEImage, rva: int, count: int) -> dict:
    if pe.u32(rva) != count:
        raise MetadataError("unexpected metadata field count")
    name = pe.c_string(pe.pointer_rva(rva + 0x50))
    names_rva = pe.pointer_rva(rva + 0x10)
    types_rva = pe.pointer_rva(rva + 0x18)
    offsets_rva = pe.pointer_rva(rva + 0x20)
    names = [pe.c_string(pe.pointer_rva(names_rva + index * 8)) for index in range(count)]
    types = [pe.pointer_rva(types_rva + index * 8) for index in range(count)]
    offsets = [pe.u16(offsets_rva + index * 2) for index in range(count)]
    return {"descriptor_rva": rva, "name": name, "names_rva": names_rva,
            "types_rva": types_rva, "offsets_rva": offsets_rva,
            "names": names, "types": types, "offsets": offsets}


def _schema_report(pe: PEImage, rva: int, count: int, name: str,
                   fields: tuple[tuple[str, int], ...]) -> dict:
    descriptor = _descriptor(pe, rva, count)
    expected = list(fields)
    actual = list(zip(descriptor["names"], descriptor["offsets"]))
    if descriptor["name"] != name or actual != expected:
        raise MetadataError(f"unexpected {name} descriptor")
    return {
        "name": descriptor["name"],
        "descriptor_rva": f"0x{descriptor['descriptor_rva']:x}",
        "names_array_rva": f"0x{descriptor['names_rva']:x}",
        "types_array_rva": f"0x{descriptor['types_rva']:x}",
        "offsets_array_rva": f"0x{descriptor['offsets_rva']:x}",
        "fields": [
            {
                "name": field_name,
                "offset": offset,
                "type": KNOWN_TYPE_NAMES.get(type_rva),
                "type_rva": f"0x{type_rva:x}",
            }
            for (field_name, offset), type_rva in zip(
                actual, descriptor["types"]
            )
        ],
    }


def _additional_metadata(pe: PEImage, layout: MetadataLayout) -> dict:
    report = {}
    if layout.realm_entry_descriptor_rva is not None:
        report["realm_entry"] = _schema_report(
            pe, layout.realm_entry_descriptor_rva, len(REALM_ENTRY_FIELDS),
            "JamJSONRealmEntry", REALM_ENTRY_FIELDS,
        )
    if layout.utility_info_descriptor_rva is not None:
        report["utility_info"] = _schema_report(
            pe, layout.utility_info_descriptor_rva, len(UTILITY_INFO_FIELDS),
            "JSONUtilityInfo", UTILITY_INFO_FIELDS,
        )
    return report


def extract_realm_metadata(data: bytes, layout: MetadataLayout = FOREVER_LAYOUT) -> dict:
    """Return the bounded RealmEntry/UtilityInfo projections without hashing."""
    return _additional_metadata(parse_pe(data), layout)


def extract_metadata(data: bytes, digest: str | None = None, layout: MetadataLayout = FOREVER_LAYOUT) -> dict:
    pe = parse_pe(data)
    outer = _descriptor(pe, layout.outer_descriptor_rva, 1)
    entry = _descriptor(pe, layout.entry_descriptor_rva, 3)
    if outer["name"] != "JSONSuperDistrictList" or entry["name"] != "JamJSONSuperDistrictEntry":
        raise MetadataError("unexpected SuperDistrict descriptor name")
    if outer["names"] != ["superDistricts"] or outer["offsets"] != [0]:
        raise MetadataError("unexpected SuperDistrict outer field")
    outer_types = [pe.pointer_rva(outer["types_rva"] + index * 8) for index in range(2)]
    if outer_types != [layout.outer_type_rva, layout.entry_descriptor_rva]:
        raise MetadataError("unexpected SuperDistrict type chain")
    expected = tuple((name, offset, type_name, type_rva) for name, offset, type_name, type_rva in (
        ("disallowLogin", 8, "bool", layout.entry_type_rvas[0]),
        ("superDistrictID", 0, "int32", layout.entry_type_rvas[1]),
        ("holdDownUntilTime", 4, "uint32", layout.entry_type_rvas[2]),
    ))
    actual = tuple(zip(entry["names"], entry["offsets"], ("bool", "int32", "uint32"), entry["types"]))
    if actual != expected:
        raise MetadataError("unexpected SuperDistrict entry fields")
    report = {
        "version": list(VERSION), "build": BUILD,
        "image_base": f"0x{pe.image_base:x}",
        "metadata": {
            "name": outer["name"], "descriptor_rva": f"0x{outer['descriptor_rva']:x}",
            "names_array_rva": f"0x{outer['names_rva']:x}",
            "types_array_rva": f"0x{outer['types_rva']:x}",
            "offsets_array_rva": f"0x{outer['offsets_rva']:x}",
            "field_name": outer["names"][0], "field_offset": outer["offsets"][0],
            "field_type_rva": f"0x{outer_types[0]:x}",
            "entry_descriptor_pointer_storage_rva": f"0x{outer['types_rva'] + 8:x}",
            "entry_descriptor_rva": f"0x{entry['descriptor_rva']:x}",
            "entry_name": entry["name"],
            "fields": [{"name": name, "offset": offset, "type": type_name,
                        "type_rva": f"0x{type_rva:x}"}
                       for name, offset, type_name, type_rva in actual],
        },
    }
    if digest is not None:
        report["sha256"] = digest
    report.update(_additional_metadata(pe, layout))
    return report


def read_verified_client(path: Path) -> dict:
    try:
        data = path.read_bytes()
    except OSError as error:
        raise MetadataError("unable to read client executable") from error
    digest = hashlib.sha256(data).hexdigest()
    if digest != EXPECTED_SHA256:
        raise MetadataError("client SHA-256 does not match the approved original")
    return extract_metadata(data, digest)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--client", required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        print(json.dumps(read_verified_client(args.client), indent=2, sort_keys=True))
        return 0
    except MetadataError as error:
        print(f"forever_client_metadata: FAIL ({error})", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
