#!/usr/bin/env python3
"""Create a private, synthetic #587 data overlay; never modify stock assets.

Only SpellMisc 336029 / SpellID 30798 loses CAST_WHEN_LEARNED. This preserves
the talent-owned dependency while allowing a client-triggered EffectLearnSpell
after login. It is controlled conformance evidence, not stock-spell acceptance.
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
        "record_id": 336029,
        "spell_id": 30798,
        "attributes_index": 1,
        "sql_column": "Attributes2",
        "before": 0x80000000,
        "after": 0,
    }


def prepare(source, destination):
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
        "stock_spell_acceptance": False,
        "source_data": str(source),
        "files": {str(path): evidence for path, (_, evidence) in prepared.items()},
        "required_served_preflight": "30798 known and active; 674 absent after login",
        "sql_hotfix_verification": "required separately before either runtime",
    }
    (destination / "fixture.json").write_text(json.dumps(manifest, indent=2) + "\n")
    return manifest


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-data", type=Path, required=True)
    parser.add_argument("--output-data", type=Path, required=True)
    args = parser.parse_args()
    result = prepare(args.source_data, args.output_data)
    print(json.dumps(result, indent=2))
