#!/usr/bin/env python3
"""Move the reviewed phasing packet error and party payload builder to Core."""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))

import f4_test_fixtures_gate as item_tools


SOURCE = Path("crates/wow-world/src/phasing.rs")
CORE = Path("crates/wow-world-core/src/phasing.rs")

ERROR_TYPE = "PhaseShiftPacketBuildError"
PARTY_FUNCTION = "party_member_phase_states_like_cpp"

STD_IMPORT_OLD = "use std::{collections::HashSet, error::Error, fmt};"
STD_IMPORT_NEW = "use std::collections::HashSet;"
PARTY_IMPORT_OLD = "use wow_packet::packets::party::{PartyMemberPhase, PartyMemberPhaseStates};"
PARTY_IMPORT_NEW = (
    "#[cfg(test)]\n"
    "use wow_packet::packets::party::PartyMemberPhase;"
)
PARTY_REEXPORT = """pub use wow_world_core::phasing::{
    PhaseShiftPacketBuildError, party_member_phase_states_like_cpp,
};"""

CORE_IMPORTS = """use std::{error::Error, fmt};
use wow_entities::PhaseShift;
use wow_packet::packets::party::{PartyMemberPhase, PartyMemberPhaseStates};"""

# Exact later state reviewed after the P4a cut. Keep CORE_IMPORTS as the P4a
# generator header; this separate residual is accepted only by the applied-state
# recognizer and deliberately includes every P4b import, constant, and helper body.
P4B_CORE_RESIDUAL = """use std::{error::Error, fmt};
use wow_constants::{PhaseFlags, PhaseShiftFlags};
use wow_data::{PhaseGroupStore, PhaseStore, TerrainSwapStore};
use wow_entities::PhaseShift;
use wow_packet::packets::party::{PartyMemberPhase, PartyMemberPhaseStates};

pub const PHASE_USE_FLAGS_ALWAYS_VISIBLE: u8 = 0x01;
pub const PHASE_USE_FLAGS_INVERSE: u8 = 0x02;
const DEFAULT_PHASE: u32 = 169;

/// C++ local `PhasingHandler.cpp::GetPhaseFlags`.
pub fn phase_flags_for_id_like_cpp(phase_store: &PhaseStore, phase_id: u32) -> PhaseFlags {
    if phase_store.is_cosmetic_phase(phase_id) {
        return PhaseFlags::COSMETIC;
    }

    if phase_store.is_personal_phase(phase_id) {
        return PhaseFlags::PERSONAL;
    }

    PhaseFlags::NONE
}

/// C++ `PhasingHandler::InitDbPhaseShift`.
pub fn init_db_phase_shift_like_cpp(
    phase_shift: &mut PhaseShift,
    phase_store: &PhaseStore,
    phase_group_store: &PhaseGroupStore,
    phase_use_flags: u8,
    phase_id: u16,
    phase_group_id: u32,
) {
    phase_shift.clear_phases_like_cpp();
    phase_shift.set_db_phase_shift_like_cpp(true);

    let mut flags = PhaseShiftFlags::NONE;
    if phase_use_flags & PHASE_USE_FLAGS_ALWAYS_VISIBLE != 0 {
        flags |= PhaseShiftFlags::ALWAYS_VISIBLE | PhaseShiftFlags::UNPHASED;
    }
    if phase_use_flags & PHASE_USE_FLAGS_INVERSE != 0 {
        flags |= PhaseShiftFlags::INVERSE;
    }

    if phase_id != 0 {
        let phase_id = u32::from(phase_id);
        phase_shift.add_phase_like_cpp(
            phase_id,
            phase_flags_for_id_like_cpp(phase_store, phase_id),
            1,
        );
    } else if phase_group_id != 0
        && let Some(phases_in_group) = phase_group_store.phases_for_group(phase_group_id)
    {
        for phase_in_group in phases_in_group {
            phase_shift.add_phase_like_cpp(
                *phase_in_group,
                phase_flags_for_id_like_cpp(phase_store, *phase_in_group),
                1,
            );
        }
    }

    if phase_shift.phase_count_like_cpp() == 0 || phase_shift.has_phase_like_cpp(DEFAULT_PHASE) {
        if flags.contains(PhaseShiftFlags::INVERSE) {
            flags |= PhaseShiftFlags::INVERSE_UNPHASED;
        } else {
            flags |= PhaseShiftFlags::UNPHASED;
        }
    }

    phase_shift.set_flags_like_cpp(flags);
}

/// C++ `PhasingHandler::InitDbVisibleMapId`.
pub fn init_db_visible_map_id_like_cpp(
    phase_shift: &mut PhaseShift,
    terrain_swap_store: &TerrainSwapStore,
    visible_map_id: i32,
) {
    phase_shift.clear_visible_map_ids_like_cpp();
    if let Ok(visible_map_id) = u32::try_from(visible_map_id)
        && terrain_swap_store
            .terrain_swap_info(visible_map_id)
            .is_some()
    {
        phase_shift.add_visible_map_id_like_cpp(visible_map_id, 1);
    }
}"""

USE_ITEM = re.compile(
    r"(?m)^[ \t]*(?P<attrs>(?:\#[ \t]*\[[^\]\n]*\][ \t]*(?:\r?\n[ \t]*)?)*)"
    r"(?P<visibility>pub(?:[ \t]*\([^)]*\))?[ \t]+)?use[ \t]+(?P<path>[^;]+);"
)
PARTY_MODULE = "wow_packet::packets::party"
CORE_PHASING_MODULE = "wow_world_core::phasing"
CORE_REEXPORTS = {ERROR_TYPE, PARTY_FUNCTION}
P4B_CORE_REEXPORTS = {
    ERROR_TYPE,
    PARTY_FUNCTION,
    "PHASE_USE_FLAGS_ALWAYS_VISIBLE",
    "PHASE_USE_FLAGS_INVERSE",
    "init_db_phase_shift_like_cpp",
    "init_db_visible_map_id_like_cpp",
    "phase_flags_for_id_like_cpp",
}


class CodemodError(Exception):
    """The reviewed source shape or an exact extraction target changed."""


def _item_support(root: Path):
    return item_tools.lib(root)


def _named_item_matches(source: str, lexer, name: str):
    code = lexer.blank_noncode(source)
    return list(re.finditer(item_tools.ITEM_HEAD + re.escape(name) + r"\b", code))


def _impl_matches(source: str, lexer, trait: str):
    code = lexer.blank_noncode(source)
    expected = f"impl {trait} for {ERROR_TYPE}"
    matches = []
    for start, end, self_type in item_tools.impl_blocks(lexer, code):
        if self_type != ERROR_TYPE:
            continue
        brace = code.find("{", start, end)
        if brace < 0:
            continue
        header = re.sub(r"\s+", " ", code[start:brace]).strip()
        if header == expected:
            matches.append((start, end))
    return matches


def _selected_spans(source: str, lexer):
    code = lexer.blank_noncode(source)
    spans: list[tuple[int, int, str]] = []
    for name, kind in ((ERROR_TYPE, "enum"), (PARTY_FUNCTION, "fn")):
        matches = _named_item_matches(source, lexer, name)
        if len(matches) != 1:
            raise CodemodError(
                f"expected exactly one {kind} declaration for {name}, found {len(matches)}"
            )
        span = item_tools.item_span(lexer, source, code, name)
        if span is None:
            raise CodemodError(f"missing {kind} declaration for {name}")
        start, end = span
        if not re.search(rf"(?m)^\s*pub\s+{kind}\s+{re.escape(name)}\b", source[start:end]):
            raise CodemodError(f"unexpected visibility or declaration for {name}")
        spans.append((start, end, name))

    for trait in ("fmt::Display", "Error"):
        matches = _impl_matches(source, lexer, trait)
        if len(matches) != 1:
            raise CodemodError(
                f"expected exactly one `{trait}` impl for {ERROR_TYPE}, found {len(matches)}"
            )
        start, end = matches[0]
        spans.append((start, end, f"impl {trait} for {ERROR_TYPE}"))

    spans.sort()
    for left, right in zip(spans, spans[1:]):
        if left[1] > right[0]:
            raise CodemodError(f"overlapping extraction spans: {left[2]} and {right[2]}")
    return spans


def _erase_spans(source: str, spans) -> str:
    result = source
    for start, end, _ in reversed(spans):
        cursor = end
        while cursor < len(result):
            if result.startswith("\r\n", cursor):
                after_break = cursor + 2
            elif result[cursor] == "\n":
                after_break = cursor + 1
            else:
                break
            cursor = after_break
            while cursor < len(result) and result[cursor] in " \t":
                cursor += 1
            if cursor < len(result) and result.startswith("\r\n", cursor):
                continue
            if cursor < len(result) and result[cursor] == "\n":
                continue
            cursor = after_break
            break
        result = result[:start] + result[cursor:]
    return result


def _build_core_module(source: str, lexer) -> tuple[str, str]:
    spans = _selected_spans(source, lexer)
    moved = "\n\n".join(source[start:end].rstrip() for start, end, _ in spans)
    remaining = _erase_spans(source, spans)
    core = f"{CORE_IMPORTS}\n\n{moved}\n"
    return remaining, core


def _replace_once(source: str, old: str, new: str, label: str) -> str:
    count = source.count(old)
    if count != 1:
        raise CodemodError(f"expected one original {label}, found {count}")
    return source.replace(old, new, 1)


def _top_level_use_items(source: str, lexer):
    code = lexer.blank_noncode(source)
    items = []
    for match in USE_ITEM.finditer(code):
        prefix = code[:match.start()]
        if prefix.count("{") != prefix.count("}"):
            continue
        items.append(
            {
                "attrs": re.sub(r"\s+", "", match.group("attrs")),
                "visibility": re.sub(r"\s+", "", match.group("visibility") or ""),
                "path": re.sub(r"\s+", "", match.group("path")),
            }
        )
    return items


def _module_imports(items, module: str):
    return [
        item for item in items
        if item["path"] == module or item["path"].startswith(module + "::")
    ]


def _has_exact_party_import(source: str, lexer) -> bool:
    party_imports = _module_imports(_top_level_use_items(source, lexer), PARTY_MODULE)
    return (
        len(party_imports) == 1
        and party_imports[0]["attrs"] == "#[cfg(test)]"
        and party_imports[0]["visibility"] == ""
        and party_imports[0]["path"] == PARTY_MODULE + "::PartyMemberPhase"
    )


def _core_reexport_layout(source: str, lexer) -> str | None:
    reexports = _module_imports(_top_level_use_items(source, lexer), CORE_PHASING_MODULE)
    if len(reexports) != 1:
        return None
    item = reexports[0]
    path = item["path"]
    prefix = CORE_PHASING_MODULE + "::"
    if item["attrs"] or item["visibility"] != "pub" or not path.startswith(prefix):
        return None
    tree = path[len(prefix):]
    if not (tree.startswith("{") and tree.endswith("}")):
        return None
    names = [name for name in tree[1:-1].split(",") if name]
    if len(names) != len(set(names)):
        return None
    name_set = set(names)
    if name_set == CORE_REEXPORTS:
        return "p4a"
    if name_set == P4B_CORE_REEXPORTS:
        return "p4b"
    return None


def _has_exact_core_reexport(source: str, lexer) -> bool:
    return _core_reexport_layout(source, lexer) is not None


def _core_residual_layout(residual: str) -> str | None:
    if residual.strip() == CORE_IMPORTS.strip():
        return "p4a"
    if residual.strip() == P4B_CORE_RESIDUAL.strip():
        return "p4b"
    return None


def _update_world_imports(source: str, lexer) -> str:
    code = lexer.blank_noncode(source)
    party_imports = _module_imports(_top_level_use_items(source, lexer), PARTY_MODULE)
    old_party_path = re.sub(
        r"\s+", "", PARTY_IMPORT_OLD.removeprefix("use ").removesuffix(";")
    )
    if (
        code.count(STD_IMPORT_NEW)
        or len(party_imports) != 1
        or party_imports[0]["attrs"]
        or party_imports[0]["visibility"]
        or party_imports[0]["path"] != old_party_path
    ):
        raise CodemodError("partial or unexpected phasing import state")
    source = _replace_once(source, STD_IMPORT_OLD, STD_IMPORT_NEW, "standard-library import")
    source = _replace_once(source, PARTY_IMPORT_OLD, PARTY_IMPORT_NEW, "party-payload import")
    if _module_imports(_top_level_use_items(source, lexer), CORE_PHASING_MODULE):
        raise CodemodError("phasing Core reexport already exists without its destination")
    source = _replace_once(
        source,
        PARTY_IMPORT_NEW,
        PARTY_IMPORT_NEW + "\n\n" + PARTY_REEXPORT,
        "test-only party-payload import anchor",
    )
    return source


def _source_has_no_moved_items(source: str, lexer) -> bool:
    if _named_item_matches(source, lexer, ERROR_TYPE):
        return False
    if _named_item_matches(source, lexer, PARTY_FUNCTION):
        return False
    return not any(_impl_matches(source, lexer, trait) for trait in ("fmt::Display", "Error"))


def _already_applied(source: str, core: str, lexer) -> bool:
    try:
        core_spans = _selected_spans(core, lexer)
    except CodemodError:
        return False
    residual = _erase_spans(core, core_spans)
    residual_layout = _core_residual_layout(residual)
    reexport_layout = _core_reexport_layout(source, lexer)
    return (
        _source_has_no_moved_items(source, lexer)
        and residual_layout is not None
        and lexer.blank_noncode(source).count(STD_IMPORT_NEW) == 1
        and lexer.blank_noncode(source).count(STD_IMPORT_OLD) == 0
        and _has_exact_party_import(source, lexer)
        and reexport_layout == residual_layout
    )


def run(action: str, root: Path) -> None:
    source_path = root / SOURCE
    core_path = root / CORE
    if not source_path.is_file():
        raise CodemodError(f"missing source file: {source_path}")

    source = source_path.read_text(encoding="utf-8")
    lexer = _item_support(root)
    if core_path.exists():
        core = core_path.read_text(encoding="utf-8")
        if not _already_applied(source, core, lexer):
            raise CodemodError(f"partial or unexpected extraction state at {core_path}")
        print("phasing packet/party cut: already applied")
        return

    new_source, core = _build_core_module(source, lexer)
    new_source = _update_world_imports(new_source, lexer)
    print("phasing packet/party cut: move enum, two trait impls, and party payload builder")
    if action == "apply":
        core_path.parent.mkdir(parents=True, exist_ok=True)
        core_path.write_text(core, encoding="utf-8")
        source_path.write_text(new_source, encoding="utf-8")
        print(f"  created {core_path}")
        print(f"  updated {source_path}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("plan", "apply"))
    parser.add_argument("--root", type=Path, default=REPO)
    args = parser.parse_args()
    try:
        run(args.action, args.root)
    except (CodemodError, OSError) as error:
        print(f"f4_phasing_extract: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
