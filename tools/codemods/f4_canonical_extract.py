#!/usr/bin/env python3
"""Move canonical-player accessors into wow-world-core with a narrow shell façade."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


CODEMODS = Path(__file__).resolve().parent
sys.path.insert(0, str(CODEMODS))

import f4_test_fixtures_gate as item_tools  # noqa: E402


ROOT = Path(__file__).resolve().parents[2]
SOURCE = Path("crates/wow-world/src/canonical_player_access.rs")
CORE = Path("crates/wow-world-core/src/canonical_player_access.rs")
INSTALLER = "install_canonical_player_owner_for_test"

PUBLIC_API = (
    ("type", "HonorStatsLikeCpp"),
    ("fn", "canonical_player_unit_flags2_like_cpp"),
    ("const", "PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP"),
    ("fn", "canonical_player_is_contested_pvp_like_cpp"),
    ("fn", "canonical_player_reputation_state_flags_like_cpp"),
    ("fn", "canonical_player_forced_reputation_faction_ids_like_cpp"),
    ("fn", "canonical_player_reputation_standings_like_cpp"),
)

PUBLIC_WORLD_FUNCTIONS = (
    "with_canonical_player_at_like_cpp",
    "with_canonical_player_at_mut_like_cpp",
    "canonical_player_presentation_like_cpp",
    "set_player_visible_item_values_like_cpp",
    "configure_canonical_player_vitals_for_test",
    "configure_canonical_player_party_flags_for_test",
)
FIXTURE_FUNCTIONS = (
    "configure_canonical_player_vitals_for_test",
    "configure_canonical_player_party_flags_for_test",
)

OLD_SESSION_LINK = "//! owning [`WorldSession`](crate::session::WorldSession)."
NEW_SESSION_TEXT = "//! owning WorldSession."
OLD_FIXTURE_CFG = "#[cfg(test)]"
NEW_FIXTURE_CFG = '#[cfg(any(test, feature = "test-fixtures"))]'

SHELL_PREFIX = """//! Compatibility exports for canonical player access retained in wow-world.

pub use wow_world_core::canonical_player_access::{
    HonorStatsLikeCpp, PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP,
    canonical_player_forced_reputation_faction_ids_like_cpp,
    canonical_player_is_contested_pvp_like_cpp, canonical_player_reputation_standings_like_cpp,
    canonical_player_reputation_state_flags_like_cpp, canonical_player_unit_flags2_like_cpp,
};

pub(crate) use wow_world_core::canonical_player_access::set_player_visible_item_values_like_cpp;

#[cfg(test)]
pub(crate) use wow_world_core::canonical_player_access::{
    canonical_player_presentation_like_cpp, configure_canonical_player_party_flags_for_test,
    configure_canonical_player_vitals_for_test, with_canonical_player_at_like_cpp,
    with_canonical_player_at_mut_like_cpp,
};

#[cfg(test)]
use crate::session::SharedCanonicalMapManager;
#[cfg(test)]
use wow_core::ObjectGuid;
#[cfg(test)]
use wow_entities::Player;

"""


class CodemodError(Exception):
    """The source shape differs from the reviewed exact extraction allowlist."""


def _lexer(root: Path):
    try:
        return item_tools.lib(root)
    except item_tools.CodemodError as error:
        raise CodemodError(str(error)) from error


def _item(source: str, lexer, name: str) -> tuple[int, int]:
    code = lexer.blank_noncode(source)
    pattern = re.compile(item_tools.ITEM_HEAD + re.escape(name) + r"\b")
    matches = list(pattern.finditer(code))
    if len(matches) != 1:
        raise CodemodError(f"expected one Rust item named {name}, found {len(matches)}")
    span = item_tools.item_span(lexer, source, code, name)
    if span is None:
        raise CodemodError(f"could not locate Rust item named {name}")
    return span


def _item_count(source: str, lexer, name: str) -> int:
    code = lexer.blank_noncode(source)
    pattern = re.compile(item_tools.ITEM_HEAD + re.escape(name) + r"\b")
    return len(list(pattern.finditer(code)))


def extract_installer(source: str, lexer) -> tuple[str, str]:
    """Return source without the unique WorldSession installer and its exact text."""
    start, end = _item(source, lexer, INSTALLER)
    moved = source[start:end]
    code = lexer.blank_noncode(moved)
    if not re.search(
        r"(?m)^\s*#\[cfg\(test\)\]\s*\n\s*pub\(crate\)\s+fn\s+"
        + re.escape(INSTALLER)
        + r"\s*\(",
        code,
    ):
        raise CodemodError("installer no longer has its reviewed cfg(test)/pub(crate) signature")
    return source[:start] + source[end:], moved


def _rewrite_exact(source: str, old: str, new: str, label: str) -> tuple[str, bool]:
    old_count = source.count(old)
    new_count = source.count(new)
    if old_count + new_count != 1:
        raise CodemodError(
            f"expected exactly one old or new candidate for {label}; "
            f"found old={old_count} new={new_count}"
        )
    if old_count:
        return source.replace(old, new, 1), True
    return source, False


def _rewrite_item_cfg(source: str, lexer, name: str) -> tuple[str, bool]:
    start, end = _item(source, lexer, name)
    item = source[start:end]
    old_count = len(re.findall(r"(?m)^[ \t]*" + re.escape(OLD_FIXTURE_CFG) + r"[ \t]*$", item))
    new_count = len(re.findall(r"(?m)^[ \t]*" + re.escape(NEW_FIXTURE_CFG) + r"[ \t]*$", item))
    if old_count + new_count != 1:
        raise CodemodError(
            f"expected exactly one old or new cfg for {name}; "
            f"found old={old_count} new={new_count}"
        )
    if new_count:
        return source, False
    updated = re.sub(
        r"(?m)^[ \t]*" + re.escape(OLD_FIXTURE_CFG) + r"[ \t]*$",
        NEW_FIXTURE_CFG,
        item,
        count=1,
    )
    return source[:start] + updated + source[end:], True


def _rewrite_visibility(source: str, lexer, kind: str, name: str) -> tuple[str, bool]:
    code = lexer.blank_noncode(source)
    pattern = re.compile(
        rf"(?m)^[ \t]*(?P<visibility>pub(?:\([^)]*\))?)[ \t]+"
        rf"{re.escape(kind)}[ \t]+{re.escape(name)}\b"
    )
    matches = list(pattern.finditer(code))
    if len(matches) != 1:
        raise CodemodError(
            f"expected one {kind} declaration for {name}, found {len(matches)}"
        )
    match = matches[0]
    visibility = match.group("visibility")
    if visibility == "pub":
        return source, False
    if visibility != "pub(crate)":
        raise CodemodError(f"unexpected visibility {visibility} for {kind} {name}")
    start, end = match.span("visibility")
    return source[:start] + "pub" + source[end:], True


def _require_public_items(source: str, lexer) -> None:
    for kind, name in PUBLIC_API:
        code = lexer.blank_noncode(source)
        pattern = re.compile(
            rf"(?m)^[ \t]*(?P<visibility>pub(?:\([^)]*\))?)[ \t]+"
            rf"{re.escape(kind)}[ \t]+{re.escape(name)}\b"
        )
        matches = list(pattern.finditer(code))
        if len(matches) != 1 or matches[0].group("visibility") != "pub":
            raise CodemodError(f"original public API item {name} is missing or changed visibility")


def transform_core(source: str, lexer) -> tuple[str, list[str]]:
    """Apply only the reviewed doc, test-gate, and visibility changes to Core text."""
    if _item_count(source, lexer, INSTALLER):
        raise CodemodError("WorldSession-typed installer must remain in the wow-world shell")

    changed: list[str] = []
    source, did_change = _rewrite_exact(
        source, OLD_SESSION_LINK, NEW_SESSION_TEXT, "Core WorldSession rustdoc"
    )
    if did_change:
        changed.append("WorldSession rustdoc link")

    for name in FIXTURE_FUNCTIONS:
        source, did_change = _rewrite_item_cfg(source, lexer, name)
        if did_change:
            changed.append(f"cfg {name}")

    for name in PUBLIC_WORLD_FUNCTIONS:
        source, did_change = _rewrite_visibility(source, lexer, "fn", name)
        if did_change:
            changed.append(f"fn {name}")
    source, did_change = _rewrite_visibility(
        source, lexer, "type", "CanonicalPlayerPresentationLikeCpp"
    )
    if did_change:
        changed.append("type CanonicalPlayerPresentationLikeCpp")

    _require_public_items(source, lexer)
    return source, changed


def render_shell(installer: str) -> str:
    return SHELL_PREFIX + installer + "\n"


def transform_source(source: str, lexer) -> tuple[str, str, list[str]]:
    without_installer, installer = extract_installer(source, lexer)
    core, changed = transform_core(without_installer, lexer)
    return core, render_shell(installer), changed


def _validate_applied(source: str, core_source: str, lexer) -> None:
    _, installer = extract_installer(source, lexer)
    if render_shell(installer) != source:
        raise CodemodError("source shell differs from the reviewed explicit reexport façade")
    normalized, changed = transform_core(core_source, lexer)
    if normalized != core_source or changed:
        raise CodemodError("Core file is not in the fully applied extraction shape")


def run(action: str, root: Path) -> None:
    source_path = root / SOURCE
    core_path = root / CORE
    if not source_path.is_file():
        raise CodemodError(f"missing canonical-player source: {source_path}")
    original = source_path.read_text(encoding="utf-8")
    lexer = _lexer(root)

    if original.startswith("//! Compatibility exports for canonical player access"):
        if not core_path.is_file():
            raise CodemodError("compatibility façade exists but the Core destination is missing")
        _validate_applied(original, core_path.read_text(encoding="utf-8"), lexer)
        print("already applied: Core implementation and explicit shell façade")
        return

    core_source, shell_source, changed = transform_source(original, lexer)
    if core_path.exists():
        existing_core = core_path.read_text(encoding="utf-8")
        if existing_core != core_source:
            raise CodemodError(f"Core destination already exists with different content: {core_path}")

    if action == "plan":
        print(f"would move {SOURCE} -> {CORE} (excluding {INSTALLER})")
        print(f"would replace {SOURCE} with explicit shell façade retaining {INSTALLER}")
        if changed:
            print("Core-only changes: " + ", ".join(changed))
        return

    if source_path.read_text(encoding="utf-8") != original:
        raise CodemodError(f"refusing concurrent change to {source_path}; rerun plan")
    if core_path.exists() and core_path.read_text(encoding="utf-8") != core_source:
        raise CodemodError(f"refusing concurrent change to {core_path}; rerun plan")
    core_path.parent.mkdir(parents=True, exist_ok=True)
    if not core_path.exists():
        core_path.write_text(core_source, encoding="utf-8")
    source_path.write_text(shell_source, encoding="utf-8")
    print(f"moved implementation to {CORE}; retained {INSTALLER} in {SOURCE}")
    if changed:
        print("Core-only changes: " + ", ".join(changed))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("plan", "apply"))
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    try:
        run(args.action, args.root)
    except (CodemodError, OSError) as error:
        print(f"f4_canonical_extract: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
