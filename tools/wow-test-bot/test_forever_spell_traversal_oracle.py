#!/usr/bin/env python3
"""Parent-exclusive native source-container QA. No DB, assets, wire or network.

Compile/run only at the completed-delivery acceptance, not per internal edit.
The production build pins headers independently; this driver also checks its
full extracted tree, and obtains reference headers from immutable Git objects.
"""
from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
import re
import subprocess
import sys
import tempfile

SOURCE = "02245dcd245e7433e524577656177723d3e4992e"
TREE = "6442dc47d86b5c718d698d65a33ab098c13074a56ff2d7ca695689ffac41b711"
ROOT = Path(__file__).resolve().parents[2]


def verify_headers(root: Path) -> None:
    if root.is_symlink() or not root.is_dir():
        raise ValueError("unsupported Boost root")
    if {path.name for path in root.iterdir()} != {"boost", "LICENSE_1_0.txt"}:
        raise ValueError("unexpected Boost root entries")
    paths: list[Path] = []
    for path in root.rglob("*"):
        if path.is_symlink():
            raise ValueError("Boost source symlink rejected")
        if path.is_file():
            paths.append(path.relative_to(root))
        elif not path.is_dir():
            raise ValueError("unsupported Boost source entry")
    manifest = hashlib.sha256()
    for relative in sorted(paths, key=lambda path: path.as_posix()):
        name = relative.as_posix()
        if not name.isascii() or any(char in name for char in "\n\r\\"):
            raise ValueError("ambiguous Boost source path")
        digest = hashlib.sha256((root / relative).read_bytes()).hexdigest()
        manifest.update(f"{digest}  {name}\n".encode("ascii"))
    if manifest.hexdigest() != TREE:
        raise ValueError("Boost header tree differs from the pinned official archive")


def source_file(reference: Path, path: str) -> bytes:
    result = subprocess.run(
        ["git", "-C", str(reference), "show", f"{SOURCE}:{path}"],
        capture_output=True, check=True,
    )
    return result.stdout


def unique_source(data: bytes, pattern: bytes, responsibility: str) -> bytes:
    matches = re.findall(pattern, data, re.DOTALL)
    if len(matches) != 1:
        raise ValueError(f"missing or ambiguous source {responsibility}")
    return matches[0]


def source_function(data: bytes, signature: bytes) -> bytes:
    # Top-level function closing braces have no indentation in this pin.
    # Matching a complete unique body is fail-closed, not a line-number slice.
    return unique_source(data, re.escape(signature) + rb"\n\{.*?\n\}", "cast function")


def run(args: argparse.Namespace) -> int:
    verify_headers(args.boost_headers_root)
    # The reference checkout may have placeholders. Only this immutable object
    # identity and object contents matter, never its physical header files.
    identity = subprocess.run(
        ["git", "-C", str(args.reference_root), "rev-parse", "HEAD"],
        capture_output=True, check=True, text=True,
    ).stdout.strip()
    if identity != SOURCE:
        raise ValueError("unexpected target source identity")
    header = source_file(args.reference_root, "src/common/Utilities/Hash.h")
    enums = source_file(args.reference_root, "src/server/game/DataStores/DBCEnums.h")
    matches = re.findall(rb"enum Difficulty : int16\s*\{[^}]+\};", enums)
    if len(matches) != 1:
        raise ValueError("missing or ambiguous source Difficulty declaration")
    stores = source_file(args.reference_root, "src/server/game/DataStores/DB2Stores.cpp")
    skill_alias = re.findall(
        rb"typedef std::unordered_multimap<uint32, SkillRaceClassInfoEntry const\*> SkillRaceClassInfoContainer;",
        stores,
    )
    if len(skill_alias) != 1:
        raise ValueError("missing or ambiguous source skill RC container declaration")
    player = source_file(args.reference_root, "src/server/game/Entities/Player/Player.h")
    book_declarations = []
    for pattern in (
        rb"enum PlayerSpellState : uint8\s*\{[^}]+\};",
        rb"struct PlayerSpellTrait\s*\{.*?\n\};",
        rb"struct PlayerSpell\s*\{.*?\n\};",
        rb"typedef std::unordered_map<uint32, PlayerSpell> PlayerSpellMap;",
    ):
        declarations = re.findall(pattern, player, re.DOTALL)
        if len(declarations) != 1:
            raise ValueError("missing or ambiguous source Player spell declaration")
        book_declarations.append(declarations[0])
    directory = ROOT / "crates/world-server/src/forever/spell_traversal"
    overrides = re.findall(
        rb"std::unordered_map<uint32 /\*overridenSpellId\*/, std::unordered_set<uint32> /\*newSpellId\*/> m_overrideSpells;",
        player,
    )
    if len(overrides) != 1:
        raise ValueError("missing or ambiguous source Player override declaration")
    unit_header = source_file(args.reference_root, "src/server/game/Entities/Unit/Unit.h")
    context = unique_source(
        unit_header, rb"struct GetCastSpellInfoContext\n        \{.*?\n        \};", "cast context",
    )
    definitions = source_file(args.reference_root, "src/server/game/Spells/SpellDefines.h")
    triggered = unique_source(
        definitions, rb"enum TriggerCastFlags : uint32\n\{.*?\n\};", "trigger flags",
    )
    shared = source_file(args.reference_root, "src/server/game/Miscellaneous/SharedDefines.h")
    aura_types = source_file(args.reference_root, "src/server/game/Spells/Auras/SpellAuraDefines.h")
    enumerators = [unique_source(data, re.escape(name) + rb"[^\n]+", "cast enum member")
                   for data, name in [
                       (shared, b"SPELL_ATTR8_IGNORE_SPELLCAST_OVERRIDE_COST "),
                       (shared, b"SPELL_ATTR11_IGNORE_SPELLCAST_OVERRIDE_SHAPESHIFT_REQUIREMENTS "),
                       (aura_types, b"SPELL_AURA_OVERRIDE_ACTIONBAR_SPELLS "),
                       (aura_types, b"SPELL_AURA_OVERRIDE_ACTIONBAR_SPELLS_TRIGGERED "),
                   ]]
    functions = [source_function(source_file(args.reference_root, path), signature)
                 for path, signature in [
                     ("src/server/game/Entities/Player/Player.cpp",
                      b"SpellInfo const* Player::GetCastSpellInfo(SpellInfo const* spellInfo, TriggerCastFlags& triggerFlag, GetCastSpellInfoContext* context) const"),
                     ("src/server/game/Entities/Unit/Unit.cpp",
                      b"bool Unit::GetCastSpellInfoContext::AddSpell(uint32 spellId)"),
                     ("src/server/game/Entities/Unit/Unit.cpp",
                      b"SpellInfo const* Unit::GetCastSpellInfo(SpellInfo const* spellInfo, TriggerCastFlags& triggerFlag, GetCastSpellInfoContext* context) const"),
                     ("src/server/game/Spells/Auras/SpellAuraEffects.cpp",
                      b"bool AuraEffect::IsAffectingSpell(SpellInfo const* spell) const"),
                     ("src/server/game/Spells/SpellInfo.cpp",
                      b"bool SpellInfo::IsAffected(uint32 familyName, flag128 const& familyFlags) const"),
                 ]]
    with tempfile.TemporaryDirectory(prefix="forever-spell-traversal-") as staging:
        staging_root = Path(staging)
        (staging_root / "ForeverReferenceHash.hpp").write_bytes(header)
        (staging_root / "SourceDifficulty.hpp").write_bytes(
            b"#include <cstdint>\nusing int16 = std::int16_t;\n" + matches[0] + b"\n"
        )
        (staging_root / "SourceSkillRaceClassContainer.hpp").write_bytes(
            b"#include <cstdint>\n#include <unordered_map>\nusing uint32 = std::uint32_t;\n"
            b"struct SkillRaceClassInfoEntry { uint32 ID; std::uint16_t SkillID; };\n"
            + skill_alias[0] + b"\n"
        )
        (staging_root / "SourcePlayerSpellMap.hpp").write_bytes(
            b"#include <cstdint>\n#include <optional>\n#include <unordered_map>\n"
            b"using uint8 = std::uint8_t; using uint32 = std::uint32_t;\n"
            b"using int32 = std::int32_t;\n"
            b"template<class T> using Optional = std::optional<T>;\n"
            + b"\n".join(book_declarations) + b"\n"
        )
        (staging_root / "SourcePlayerOverrides.hpp").write_bytes(
            b"#include <cstdint>\n#include <unordered_map>\n#include <unordered_set>\n"
            b"using uint32 = std::uint32_t;\nstruct ReferenceOverrideOwner {\n"
            + overrides[0] + b"\n};\n"
        )
        (staging_root / "SourceCastContext.hpp").write_bytes(context + b"\n")
        (staging_root / "SourceCastFunctions.hpp").write_bytes(b"\n".join(functions) + b"\n")
        (staging_root / "SourceCastEnums.hpp").write_bytes(
            b"#include <cstdint>\nusing uint32 = std::uint32_t; using int32 = std::int32_t;\n"
            b"using int16 = std::int16_t;\n" + triggered + b"\n"
            b"inline TriggerCastFlags operator~(TriggerCastFlags f) { return TriggerCastFlags(~uint32(f)); }\n"
            b"inline TriggerCastFlags& operator|=(TriggerCastFlags& a, TriggerCastFlags b) { a = TriggerCastFlags(uint32(a) | uint32(b)); return a; }\n"
            b"inline TriggerCastFlags& operator&=(TriggerCastFlags& a, TriggerCastFlags b) { a = TriggerCastFlags(uint32(a) & uint32(b)); return a; }\n"
            b"enum SpellAttr8 : uint32 { " + enumerators[0] + b"\n};\n"
            b"enum SpellAttr11 : uint32 { " + enumerators[1] + b"\n};\n"
            b"enum AuraType : uint32 { " + enumerators[2] + b"\n" + enumerators[3] + b"\n};\n"
        )
        executable = staging_root / "source-container-oracle"
        result = subprocess.run([
            args.compiler, "-std=c++20", "-O2",
            f"-I{args.boost_headers_root.resolve()}", f"-I{staging_root}",
            str(directory / "bridge.cpp"), str(directory / "oracle.cpp"),
            str(directory / "birth_lookup.cpp"), str(directory / "birth_lookup_oracle.cpp"),
            str(directory / "book_order.cpp"), str(directory / "book_order_oracle.cpp"),
            str(directory / "override_order.cpp"), str(directory / "override_order_oracle.cpp"),
            str(directory / "cast_oracle.cpp"),
            "-o", str(executable),
        ], capture_output=True, text=True, timeout=120)
        if result.returncode != 0:
            # These are public C++ source diagnostics, not private assets/logs.
            sys.stderr.write(result.stderr)
            return result.returncode
        result = subprocess.run([str(executable)], capture_output=True, text=True, timeout=30)
        if result.returncode == 0 and result.stdout.startswith("PASS "):
            print(result.stdout.strip())
            return 0
        print(f"FAIL source-container-oracle exit={result.returncode}", file=sys.stderr)
        return result.returncode or 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--boost-headers-root", type=Path,
                        default=ROOT / "target/forever-login/boost_1_83_0")
    parser.add_argument("--reference-root", type=Path,
                        default=ROOT / "target/forever-cpp-reference")
    parser.add_argument("--compiler", default="g++")
    args = parser.parse_args()
    try:
        return run(args)
    except (OSError, ValueError, subprocess.SubprocessError):
        print("FAIL source-container QA prerequisites or execution", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
