#!/usr/bin/env python3
"""Cut the reviewed battle-pet DTO items into wow-world-core for #1263 F4a P4a."""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))

import f4_test_fixtures_gate as item_tools


SOURCE = Path("crates/wow-world/src/session/battle_pet_adapter.rs")
CORE = Path("crates/wow-world-core/src/session/battle_pet_adapter.rs")
SESSION_MOD = Path("crates/wow-world/src/session/mod.rs")
SHELL_IMPORT_OLD = "use super::{AuraApplication, Instant, ObjectGuid, RepresentedAuraEffectLikeCpp, WorldSession};"
SHELL_IMPORT_OLD_MEMBERS = (
    "AuraApplication",
    "Instant",
    "ObjectGuid",
    "RepresentedAuraEffectLikeCpp",
    "WorldSession",
)
SHELL_IMPORT_NEW_MEMBERS = (
    "AuraApplication",
    "Instant",
    "ObjectGuid",
    "RepresentedAuraEffectLikeCpp",
    "RepresentedBattlePetCalculatedStatsLikeCpp",
    "RepresentedBattlePetDataLikeCpp",
    "WorldSession",
)
SHELL_CAGE_IMPORT = "RepresentedBattlePetCageItemLikeCpp"
SHELL_IMPORT_TARGETS = frozenset(
    (*SHELL_IMPORT_OLD_MEMBERS, *SHELL_IMPORT_NEW_MEMBERS, SHELL_CAGE_IMPORT)
)
SHELL_IMPORT_NEW = """#[cfg(test)]
use super::RepresentedBattlePetCageItemLikeCpp;
use super::{
    AuraApplication, Instant, ObjectGuid, RepresentedAuraEffectLikeCpp,
    RepresentedBattlePetCalculatedStatsLikeCpp, RepresentedBattlePetDataLikeCpp, WorldSession,
};"""

DTO_KINDS = {
    "RepresentedBattlePetSaveInfoLikeCpp": "enum",
    "RepresentedBattlePetCageItemLikeCpp": "struct",
    "RepresentedBattlePetCalculatedStatsLikeCpp": "struct",
    "RepresentedBattlePetLevelCriteriaLikeCpp": "struct",
    "RepresentedBattlePetDataLikeCpp": "struct",
    "RepresentedBattlePetQueryCompanionLikeCpp": "struct",
    "RepresentedBattlePetSlotLikeCpp": "struct",
}

DTO_FIELDS = {
    "RepresentedBattlePetCageItemLikeCpp": (
        "item_id",
        "species_id",
        "breed_data",
        "level",
        "display_id",
    ),
    "RepresentedBattlePetCalculatedStatsLikeCpp": ("max_health", "power", "speed"),
    "RepresentedBattlePetLevelCriteriaLikeCpp": ("species", "level"),
    "RepresentedBattlePetDataLikeCpp": (
        "species",
        "creature_id",
        "display_id",
        "breed",
        "level",
        "exp",
        "flags",
        "power",
        "health",
        "max_health",
        "speed",
        "quality",
        "owner_info",
        "name",
        "name_timestamp",
        "declined_names",
        "save_info",
    ),
    "RepresentedBattlePetQueryCompanionLikeCpp": (
        "creature_id",
        "name_timestamp",
        "is_summon",
        "owner_is_player",
        "battle_pet_companion_guid",
    ),
    "RepresentedBattlePetSlotLikeCpp": ("pet_guid", "collar_id", "index", "locked"),
}

DTO_IMPLS = (
    "RepresentedBattlePetSlotLikeCpp",
    "RepresentedBattlePetDataLikeCpp",
)

DTO_METHODS = (
    "locked_empty",
    "packet_slot_like_cpp",
    "minimal_like_cpp",
    "packet_info_like_cpp",
)

IDENTIFIER = r"[A-Za-z_][A-Za-z0-9_]*"
SUPER_USE_HEAD = re.compile(
    r"(?m)^[ \t]*(?:(?:pub(?:\([^)]*\))?)[ \t]+)?"
    r"use[ \t]+super[ \t]*::"
)
CFG_TEST_BEFORE_IMPORT = re.compile(
    r"(?m)^[ \t]*#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\][ \t]*(?:\r?\n[ \t]*)+$"
)
ROOT_FIXTURE_CFG_BEFORE_REEXPORT = re.compile(
    r'(?m)^[ \t]*#\s*\[\s*cfg\s*\(\s*any\s*\(\s*test\s*,\s*'
    r'feature\s*=\s*"test-fixtures"\s*\)\s*\)\s*\][ \t]*'
    r"(?:\r?\n[ \t]*)+$"
)
ROOT_FIXTURE_GATE = '#[cfg(any(test, feature = "test-fixtures"))]'

CORE_IMPORTS = """//! Session-owned battle-pet data transfer objects shared with the world shell.

use wow_core::ObjectGuid;
"""

SHELL_TYPES = tuple(DTO_KINDS)


class CodemodError(Exception):
    """The reviewed source shape or one of its exact item targets changed."""


def _item_support(root: Path):
    return item_tools.lib(root)


def _named_item_matches(source: str, lexer, name: str):
    code = lexer.blank_noncode(source)
    pattern = re.compile(item_tools.ITEM_HEAD + re.escape(name) + r"\b")
    return list(pattern.finditer(code))


def _statement_matches(source: str, lexer, statement: str):
    code = lexer.blank_noncode(source)
    pattern = re.compile(r"(?m)^[ \t]*" + re.escape(statement) + r"[ \t]*$")
    return list(pattern.finditer(code))


def _parse_super_import(statement: str) -> tuple[str, list[str]] | None:
    statement = statement.strip()
    grouped = re.fullmatch(
        r"use\s+super\s*::\s*\{(?P<members>[^{}]*)\}\s*;", statement, re.S
    )
    if grouped is not None:
        parts = grouped.group("members").split(",")
        if parts and not parts[-1].strip():
            parts.pop()
        members = [part.strip() for part in parts]
        if not members or any(re.fullmatch(IDENTIFIER, member) is None for member in members):
            return None
        return "group", members

    single = re.fullmatch(rf"use\s+super\s*::\s*({IDENTIFIER})\s*;", statement, re.S)
    if single is not None:
        return "single", [single.group(1)]
    return None


def _has_exact_attribute_before(source: str, code: str, lexer, item_start: int, pattern) -> bool:
    preceding = source[:item_start]
    match = pattern.search(preceding)
    if match is None or match.end() != len(preceding):
        return False
    return (
        code[match.start() : match.end()] == lexer.blank_noncode(match.group(0))
        and not _has_unexpected_preceding_attribute(code, match.start())
    )


def _has_unexpected_preceding_attribute(code: str, item_start: int) -> bool:
    return code[:item_start].rstrip().endswith("]")


def _is_module_level(code: str, item_start: int) -> bool:
    depth = 0
    for char in code[:item_start]:
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth < 0:
                return False
    return depth == 0


def _shell_import_layout(source: str, lexer) -> tuple[str, list[tuple[int, int, str, list[str]]]]:
    """Recognize only the exact old or new DTO-related `use super` shape."""
    code = lexer.blank_noncode(source)
    imports: list[tuple[int, int, str, list[str]]] = []
    for match in SUPER_USE_HEAD.finditer(code):
        semicolon = code.find(";", match.end())
        if semicolon < 0:
            raise CodemodError("unterminated battle-pet shell import")
        statement = code[match.start() : semicolon + 1]
        mentioned = set(re.findall(IDENTIFIER, statement)) & SHELL_IMPORT_TARGETS
        if not mentioned:
            continue
        if not _is_module_level(code, match.start()):
            raise CodemodError("battle-pet shell imports must be module-level")
        parsed = _parse_super_import(statement)
        if parsed is None:
            raise CodemodError("malformed battle-pet shell import")
        kind, members = parsed
        if kind == "group" and _has_unexpected_preceding_attribute(code, match.start()):
            raise CodemodError("battle-pet grouped shell import must be unconditional")
        imports.append((match.start(), semicolon + 1, kind, members))

    if (
        len(imports) == 1
        and imports[0][2] == "group"
        and len(imports[0][3]) == len(SHELL_IMPORT_OLD_MEMBERS)
        and set(imports[0][3]) == set(SHELL_IMPORT_OLD_MEMBERS)
    ):
        return "old", imports

    grouped = [item for item in imports if item[2] == "group"]
    cage = [
        item
        for item in imports
        if item[2] == "single" and item[3] == [SHELL_CAGE_IMPORT]
    ]
    if (
        len(imports) == 2
        and len(grouped) == 1
        and len(cage) == 1
        and len(grouped[0][3]) == len(SHELL_IMPORT_NEW_MEMBERS)
        and set(grouped[0][3]) == set(SHELL_IMPORT_NEW_MEMBERS)
        and _has_exact_attribute_before(
            source, code, lexer, cage[0][0], CFG_TEST_BEFORE_IMPORT
        )
    ):
        return "new", imports

    raise CodemodError("unexpected battle-pet shell import set or cfg")


def _root_reexport_statement(name: str, *, core: bool) -> str:
    owner = "wow_world_core::session::battle_pet_adapter" if core else "battle_pet_adapter"
    return f"pub(crate) use {owner}::{name};"


def _root_reexport_layout(session_mod: str, lexer) -> str:
    """Require all seven shell paths in one complete old or new layout."""
    code = lexer.blank_noncode(session_mod)
    item_matches: dict[str, tuple[str, re.Match[str]]] = {}
    states: set[str] = set()
    for name in SHELL_TYPES:
        old_matches = _statement_matches(
            session_mod, lexer, _root_reexport_statement(name, core=False)
        )
        new_matches = _statement_matches(
            session_mod, lexer, _root_reexport_statement(name, core=True)
        )
        if len(old_matches) == 1 and not new_matches:
            states.add("old")
            item_matches[name] = ("old", old_matches[0])
        elif len(new_matches) == 1 and not old_matches:
            states.add("new")
            item_matches[name] = ("new", new_matches[0])
        else:
            raise CodemodError(f"expected one unique old or Core shell reexport for {name}")
        if not _is_module_level(code, item_matches[name][1].start()):
            raise CodemodError(f"{name} shell reexport must be module-level")

    if len(states) != 1:
        raise CodemodError("mixed old and Core battle-pet shell reexport set")
    state = next(iter(states))

    for name, (item_state, match) in item_matches.items():
        requires_fixture_gate = name == "RepresentedBattlePetCageItemLikeCpp" or (
            name == "RepresentedBattlePetSaveInfoLikeCpp" and item_state == "new"
        )
        if requires_fixture_gate:
            if not _has_exact_attribute_before(
                session_mod,
                code,
                lexer,
                match.start(),
                ROOT_FIXTURE_CFG_BEFORE_REEXPORT,
            ):
                raise CodemodError(f"expected one exact fixture gate on {name} reexport")
        elif _has_unexpected_preceding_attribute(code, match.start()):
            raise CodemodError(f"unexpected attribute on {name} shell reexport")
    return state


def _selected_spans(source: str, lexer):
    code = lexer.blank_noncode(source)
    spans: list[tuple[int, int, str]] = []
    for name, kind in DTO_KINDS.items():
        matches = _named_item_matches(source, lexer, name)
        if len(matches) != 1:
            raise CodemodError(f"expected one DTO declaration for {name}, found {len(matches)}")
        span = item_tools.item_span(lexer, source, code, name)
        if span is None:
            raise CodemodError(f"missing DTO item: {name}")
        start, end = span
        header = re.search(
            rf"(?m)^\s*pub\(crate\)\s+{kind}\s+{re.escape(name)}\b",
            source[start:end],
        )
        if header is None:
            raise CodemodError(f"unexpected declaration for {name}")
        spans.append((start, end, name))

    impls = item_tools.impl_blocks(lexer, code)
    for name in DTO_IMPLS:
        matches = [(start, end) for start, end, self_type in impls if self_type == name]
        if len(matches) != 1:
            raise CodemodError(f"expected one inherent impl for {name}, found {len(matches)}")
        start, end = matches[0]
        spans.append((start, end, f"impl {name}"))
    return sorted(spans)


def extract_dto_items(source: str, lexer) -> tuple[str, str]:
    """Return source with only the seven DTOs/two impls removed and their exact text."""
    spans = _selected_spans(source, lexer)
    snippets = [source[start:end] for start, end, _ in spans]
    remaining = source
    for start, end, _ in reversed(spans):
        remaining = remaining[:start] + remaining[end:]
    return remaining, "\n\n".join(snippets)


def _widen_one(source: str, pattern: re.Pattern[str], label: str) -> tuple[str, bool]:
    matches = list(pattern.finditer(source))
    if len(matches) != 1:
        raise CodemodError(f"expected one declaration for {label}, found {len(matches)}")
    match = matches[0]
    if match.group("visibility") == "pub":
        return source, False
    start, end = match.span("visibility")
    return source[:start] + "pub" + source[end:], True


def widen_moved_items(source: str, lexer) -> tuple[str, list[str]]:
    """Widen only the seven DTOs, their externally used fields, and four methods."""
    changed: list[str] = []
    for name, kind in DTO_KINDS.items():
        pattern = re.compile(
            rf"(?m)^(?P<indent>[ \t]*)(?P<visibility>pub(?:\(crate\))?)"
            rf"(?=\s+{kind}\s+{re.escape(name)}\b)"
        )
        source, did_change = _widen_one(source, pattern, f"{kind} {name}")
        if did_change:
            changed.append(f"{kind} {name}")

    for type_name, fields in DTO_FIELDS.items():
        for field in fields:
            code = lexer.blank_noncode(source)
            span = item_tools.item_span(lexer, source, code, type_name)
            if span is None:
                raise CodemodError(f"missing DTO item while widening {type_name}.{field}")
            start, end = span
            item_text = source[start:end]
            pattern = re.compile(
                rf"(?m)^(?P<indent>[ \t]*)(?P<visibility>pub(?:\(crate\))?)"
                rf"(?=\s+{re.escape(field)}\s*:)"
            )
            updated, did_change = _widen_one(item_text, pattern, f"{type_name}.{field}")
            if did_change:
                source = source[:start] + updated + source[end:]
                changed.append(f"field {type_name}.{field}")

    for name in DTO_METHODS:
        pattern = re.compile(
            rf"(?m)^(?P<indent>[ \t]*)(?P<visibility>pub(?:\(crate\))?)"
            rf"(?=\s+(?:async\s+)?fn\s+{re.escape(name)}\s*\()"
        )
        source, did_change = _widen_one(source, pattern, f"method {name}")
        if did_change:
            changed.append(f"method {name}")
    return source, changed


def _build_core_module(original: str, lexer) -> tuple[str, str, list[str]]:
    remaining, items = extract_dto_items(original, lexer)
    widened, changed = widen_moved_items(CORE_IMPORTS + "\n" + items + "\n", lexer)
    return remaining, widened, changed


def _update_shell_import(source: str, lexer) -> tuple[str, bool]:
    layout, imports = _shell_import_layout(source, lexer)
    if layout == "new":
        return source, False
    start, end = imports[0][:2]
    return source[:start] + SHELL_IMPORT_NEW + source[end:], True


def _update_shell_reexports(source: str, lexer) -> tuple[str, list[str]]:
    if _root_reexport_layout(source, lexer) != "old":
        raise CodemodError("expected the complete old battle-pet shell reexport set")

    changed: list[str] = []
    for name in SHELL_TYPES:
        old = _root_reexport_statement(name, core=False)
        new = _root_reexport_statement(name, core=True)
        match = _statement_matches(source, lexer, old)[0]
        matched_text = source[match.start() : match.end()]
        indent = matched_text[: len(matched_text) - len(matched_text.lstrip(" \t"))]
        replacement = (
            f"{indent}{ROOT_FIXTURE_GATE}\n{indent}{new}"
            if name == "RepresentedBattlePetSaveInfoLikeCpp"
            else f"{indent}{new}"
        )
        source = source[: match.start()] + replacement + source[match.end() :]
        changed.append(name)
    if _root_reexport_layout(source, lexer) != "new":
        raise CodemodError("updated battle-pet shell reexports failed exact layout check")
    return source, changed


def _already_applied(source: str, core: str, session_mod: str, lexer) -> bool:
    no_source_dtos = all(not _named_item_matches(source, lexer, name) for name in DTO_KINDS)
    try:
        source_import_updated = _shell_import_layout(source, lexer)[0] == "new"
    except CodemodError:
        source_import_updated = False
    source_code = lexer.blank_noncode(source)
    no_source_impls = not any(
        self_type in DTO_IMPLS for _, _, self_type in item_tools.impl_blocks(lexer, source_code)
    )

    core_has_items = True
    for name, kind in DTO_KINDS.items():
        matches = _named_item_matches(core, lexer, name)
        if len(matches) != 1:
            core_has_items = False
            break
        declaration = matches[0].group(0)
        if not re.search(rf"\bpub\s+{kind}\s+{re.escape(name)}\b", declaration):
            core_has_items = False
            break

    core_code = lexer.blank_noncode(core)
    core_impls = item_tools.impl_blocks(lexer, core_code)
    core_has_impls = all(
        sum(self_type == name for _, _, self_type in core_impls) == 1 for name in DTO_IMPLS
    )
    try:
        shell_has_reexports = _root_reexport_layout(session_mod, lexer) == "new"
    except CodemodError:
        shell_has_reexports = False
    return (
        no_source_dtos
        and no_source_impls
        and source_import_updated
        and core_has_items
        and core_has_impls
        and shell_has_reexports
    )


def run(action: str, root: Path) -> None:
    source_path = root / SOURCE
    core_path = root / CORE
    session_mod_path = root / SESSION_MOD
    for path in (source_path, session_mod_path):
        if not path.is_file():
            raise CodemodError(f"missing source file: {path}")

    source = source_path.read_text(encoding="utf-8")
    session_mod = session_mod_path.read_text(encoding="utf-8")
    lexer = _item_support(root)
    if core_path.exists():
        core = core_path.read_text(encoding="utf-8")
        if not _already_applied(source, core, session_mod, lexer):
            raise CodemodError(f"partial or unexpected extraction state at {core_path}")
        print("battle-pet DTO cut: already applied")
        return

    new_source, new_core, changed = _build_core_module(source, lexer)
    new_source, import_changed = _update_shell_import(new_source, lexer)
    new_session_mod, reexports = _update_shell_reexports(session_mod, lexer)
    print("battle-pet DTO cut: move seven DTOs and two inherent impls")
    print(f"  public declarations: {len(changed)}")
    print(f"  shell reexports: {len(reexports)}")
    if action == "apply":
        core_path.parent.mkdir(parents=True, exist_ok=True)
        core_path.write_text(new_core, encoding="utf-8")
        source_path.write_text(new_source, encoding="utf-8")
        session_mod_path.write_text(new_session_mod, encoding="utf-8")
        print(f"  shell import updated: {import_changed}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("plan", "apply"))
    parser.add_argument("--root", type=Path, default=REPO)
    args = parser.parse_args()
    try:
        run(args.action, args.root)
    except (CodemodError, OSError) as error:
        print(f"f4_battle_pet_extract: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
