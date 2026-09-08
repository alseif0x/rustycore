#!/usr/bin/env python3
"""Create a private, synthetic #587 data overlay; never modify stock assets.

SpellMisc 336029 / SpellID 30798 loses CAST_WHEN_LEARNED. The default target
remains 674; explicit --target-spell 6197 also changes only SpellEffect 705389's
TriggerSpell bits. This preserves the talent-owned source while allowing a
client-triggered EffectLearnSpell after login. It is controlled conformance
evidence, not stock-spell acceptance.
The caller must separately verify effective SQL hotfixes and served preflight.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct


KNOWN = {
    "23a649b44401e99d123375dc7a0d7be1eee76287af02e7bd91c2a45636a089c2": (
        4304338, 1032996, 3742638, 4003498,
        "28236be2aada1fd1376f8fa15095a0b9b25c062c8fa0db518baadaed98097edc",
    ),
    "d6c60d9e3dda17e12d5dc1133457a96506b01ac12958fa5bcf2d90eb50c00d34": (
        4303970, 1033036, 3742246, 4003082,
        "2d9775585059f96e99b1650b4fb3e7315d8d6ad19a0da595103cb69b9545e64a",
    ),
}

KNOWN_EFFECT = {
    "c786e3f82d69f6a2675cdcc7a0a6c48a2684044fb23344bd488425d982b525f8": (
        2785510, 10, 74732, 69430, 554090, 2020360, 2369108,
        "c5e3eb1cc6378674e29e0696fa0fb85df84ac4ec7ba9e8d98992cf55b6913661",
    ),
    "773d1248f5ebef2acd08f1ab554935920797edec5745cad33d77de1815b1d41b": (
        2785355, 11, 74760, 69424, 554118, 2020226, 2368950,
        "50c34e2634ae7b8126d667fdec182fe8395e5456b7c9206fedde749d1de6d0ba",
    ),
}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(condition):
    if not condition:
        raise ValueError("fixture integrity check failed")


def patched_spell_misc(source):
    original = source.read_bytes()
    digest = sha(original)
    if digest not in KNOWN:
        raise ValueError(f"unreviewed SpellMisc input: {source} ({digest})")
    size, offset, id_offset, relation_offset, expected = KNOWN[digest]
    header = struct.unpack_from("<10I2H7I", original)
    require(len(original) == size and original[:4] == b"WDC4")
    require(header[2:4] == (13, 72))
    require(header[5:7] == (0xC603EE28, 0x316AB86A))
    field_info = 72 + 40 * header[18] + 4 * header[2]
    require(struct.unpack_from("<HH5I", original, field_info) == (0, 480, 0, 0, 0, 0, 0))
    require(struct.unpack_from("<I", original, id_offset)[0] == 336029)
    require(struct.unpack_from("<II", original, relation_offset) == (30798, 14183))
    require(struct.unpack_from("<I", original, offset)[0] == 0x80000000)
    patched = bytearray(original)
    struct.pack_into("<I", patched, offset, 0)
    require([i for i, (a, b) in enumerate(zip(original, patched)) if a != b] == [offset + 3])
    require(sha(patched) == expected)
    return bytes(patched), {
        "source_sha256": digest,
        "overlay_sha256": expected,
        "changed_byte": offset + 3,
        "changed_bits": [offset * 8 + 31],
        "bit_numbering": "absolute file bit offsets, least-significant bit first per byte",
        "record_id": 336029,
        "spell_id": 30798,
        "attributes_index": 1,
        "sql_column": "Attributes2",
        "before": 0x80000000,
        "after": 0,
    }


def patched_spell_effect(source):
    """Pinned fixed-record WDC4; no generic DB2 mutation or SQL override."""
    original = source.read_bytes()
    digest = sha(original)
    if digest not in KNOWN_EFFECT:
        raise ValueError(f"unreviewed SpellEffect input: {source} ({digest})")
    size, sections, start, count, offset, id_offset, relation_offset, expected = KNOWN_EFFECT[digest]
    header = struct.unpack_from("<10I2H7I", original)
    require(len(original) == size and original[:4] == b"WDC4")
    require(header[2:4] == (28, 27))
    require(header[5:7] == (4030871717, 0x6B64DD7A))
    require(header[10] == 4 and header[12] == 28 and header[15] == 672)
    require(header[16] == 0 and header[18] == sections)
    section = struct.unpack_from("<Q8I", original, 72)
    require(section == (0, start, count, 2, 0, count * 4, count * 8 + 12, 0, 0))
    row = 17754
    require(offset == start + row * 27)
    # WDC4 fixed records end at start + count * size, then strings and IDs.
    # offset_records_end is zero here; it belongs to the variable-record case.
    ids_start = start + count * 27 + section[3]
    require(id_offset == ids_start + row * 4)
    require(struct.unpack_from("<I", original, id_offset)[0] == 705389)
    relations_start = ids_start + section[5] + section[8] * 8
    require(struct.unpack_from("<I", original, relations_start)[0] == count)
    require(relation_offset == relations_start + 12 + row * 8)
    require(struct.unpack_from("<II", original, relation_offset) == (30798, row))
    field_info = 72 + sections * 40 + header[2] * 4
    trigger_field = struct.unpack_from("<HH5I", original, field_info + 17 * 24)
    require(trigger_field == (145, 20, 0, 5, 145, 20, 0))
    record = int.from_bytes(original[offset:offset + 27], "little")
    palette_offset = field_info + header[15]
    # Independently verify difficulty 0, effect index 0 and LEARN_SPELL 36.
    for field, expected_field, expected_value in [
        (0, (0, 4, 40, 3, 0, 4, 0), 0),
        (1, (4, 4, 44, 3, 4, 4, 0), 0),
        (2, (8, 8, 656, 3, 8, 8, 0), 36),
    ]:
        info = struct.unpack_from("<HH5I", original, field_info + field * 24)
        require(info == expected_field)
        index = (record >> info[0]) & ((1 << info[1]) - 1)
        require(index * 4 < info[2])
        require(struct.unpack_from("<I", original, palette_offset + index * 4)[0] == expected_value)
        palette_offset += info[2]
    bit_offset, width = trigger_field[:2]
    value_mask = (1 << width) - 1
    require((record >> bit_offset) & value_mask == 674)
    replacement = (record & ~(value_mask << bit_offset)) | (6197 << bit_offset)
    require((replacement ^ record) & ~(value_mask << bit_offset) == 0)
    patched = bytearray(original)
    patched[offset:offset + 27] = replacement.to_bytes(27, "little")
    require((int.from_bytes(patched[offset:offset + 27], "little") >> bit_offset) & value_mask == 6197)
    changes = [
        {"offset": i, "before": before, "after": after}
        for i, (before, after) in enumerate(zip(original, patched)) if before != after
    ]
    require(changes == [
        {"offset": offset + 18, "before": 0x44, "after": 0x6A},
        {"offset": offset + 19, "before": 0x05, "after": 0x30},
    ])
    require(sha(patched) == expected)
    return bytes(patched), {
        "source_sha256": digest,
        "overlay_sha256": expected,
        "wdc4_table_hash": header[5],
        "wdc4_layout_hash": header[6],
        "wdc4_section_count": sections,
        "record_id": 705389,
        "spell_id": 30798,
        "difficulty_id": 0,
        "effect_index": 0,
        "effect": 36,
        "sql_column": "EffectTriggerSpell",
        "before": 674,
        "after": 6197,
        "section_index": 0,
        "section_record_index": row,
        "record_offset": offset,
        "record_size": 27,
        "id_offset": id_offset,
        "parent_relation_offset": relation_offset,
        "storage_field": 17,
        "compression": 5,
        "record_bit_offset": bit_offset,
        "bit_width": width,
        "changed_bytes": changes,
        "changed_bits": [offset * 8 + bit for bit in range(27 * 8) if (record ^ replacement) & (1 << bit)],
        "bit_numbering": "absolute file bit offsets, least-significant bit first per byte",
    }


def prepare(source, destination, target_spell=674):
    require(target_spell in (674, 6197))
    source = source.resolve(strict=True)
    destination = destination.absolute()
    resolved_destination = destination.resolve()
    if resolved_destination.is_relative_to(source) or source.is_relative_to(resolved_destination):
        raise ValueError("overlay must be outside the source data tree")
    if destination.exists() or destination.is_symlink():
        raise FileExistsError("overlay destination must be new")
    inputs = sorted((source / "dbc").glob("*/SpellMisc.db2"))
    if not inputs:
        raise ValueError("no locale SpellMisc files under source/dbc")
    # Validate every locale before creating output. Unknown builds fail closed.
    prepared = {path.relative_to(source): patched_spell_misc(path) for path in inputs}
    if target_spell == 6197:
        require({"enUS", "esES", "ruRU"}.issubset({path.parent.name for path in inputs}))
        for path in inputs:
            effect = path.with_name("SpellEffect.db2")
            prepared[effect.relative_to(source)] = patched_spell_effect(effect)
    destination.mkdir(mode=0o700)
    for child in source.iterdir():
        if child.name != "dbc":
            (destination / child.name).symlink_to(child, target_is_directory=child.is_dir())
    (destination / "dbc").mkdir(mode=0o700)
    for locale in (source / "dbc").iterdir():
        out_locale = destination / "dbc" / locale.name
        if not locale.is_dir():
            out_locale.symlink_to(locale)
            continue
        out_locale.mkdir(mode=0o700)
        for entry in locale.iterdir():
            relative = entry.relative_to(source)
            output = destination / relative
            if relative not in prepared:
                output.symlink_to(entry, target_is_directory=entry.is_dir())
                continue
            content, evidence = prepared[relative]
            with output.open("xb") as stream:
                require(stream.write(content) == len(content))
            require(output.read_bytes() == content)
            require(sha(entry.read_bytes()) == evidence["source_sha256"])
    manifest = {
        "contract": "synthetic-paired-effect-learn-spell-587",
        "fixture_version": 1 if target_spell == 674 else 2,
        "source_spell": 30798,
        "target_spell": target_spell,
        "stock_spell_acceptance": False,
        "source_data": str(source),
        "files": {str(path): evidence for path, (_, evidence) in prepared.items()},
        "required_served_preflight": f"30798 known and active; {target_spell} absent after login",
        "sql_hotfix_verification": "required separately before either runtime",
    }
    (destination / "fixture.json").write_text(json.dumps(manifest, indent=2) + "\n")
    return manifest


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-data", type=Path, required=True)
    parser.add_argument("--output-data", type=Path, required=True)
    parser.add_argument("--target-spell", type=int, choices=(674, 6197), default=674)
    args = parser.parse_args()
    result = prepare(args.source_data, args.output_data, args.target_spell)
    print(json.dumps(result, indent=2))
