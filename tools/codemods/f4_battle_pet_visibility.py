#!/usr/bin/env python3
"""Widen only reviewed cross-crate battle-pet account items for #1263 F4a P4a."""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))

import f4_test_fixtures_gate as item_tools


ACCOUNT_ROOT = Path("crates/wow-world-core/src/battle_pet_account.rs")
ACCOUNT_ADD = Path("crates/wow-world-core/src/battle_pet_account/add.rs")
ACCOUNT_LIFECYCLE = Path("crates/wow-world-core/src/battle_pet_account/lifecycle.rs")
ACCOUNT_MUTATE = Path("crates/wow-world-core/src/battle_pet_account/mutate.rs")
ACCOUNT_QUERIES = Path("crates/wow-world-core/src/battle_pet_account/queries.rs")
FILES = (ACCOUNT_ROOT, ACCOUNT_ADD, ACCOUNT_LIFECYCLE, ACCOUNT_MUTATE, ACCOUNT_QUERIES)

TYPE_ITEMS = (
    (ACCOUNT_ROOT, "struct", "BattlePetLeaseIdLikeCpp"),
    (ACCOUNT_ROOT, "struct", "BattlePetAddRequestLikeCpp"),
    (ACCOUNT_ROOT, "enum", "BattlePetAddFailureLikeCpp"),
    (ACCOUNT_ROOT, "enum", "BattlePetAddOutcomeLikeCpp"),
    (ACCOUNT_ROOT, "enum", "BattlePetMutationFailureLikeCpp"),
    (ACCOUNT_ROOT, "struct", "BattlePetAccountOwnerLikeCpp"),
    (ACCOUNT_ROOT, "enum", "BattlePetFencedReceiptProbeLikeCpp"),
)

METHOD_ITEMS = {
    ACCOUNT_ROOT: (
        ("BattlePetAccountAttachmentLikeCpp", "try_acquire_lease_like_cpp"),
        ("BattlePetAccountAttachmentLikeCpp", "has_lease_like_cpp"),
        ("BattlePetAccountAttachmentLikeCpp", "owner_like_cpp"),
        ("BattlePetAccountAttachmentLikeCpp", "lease_id_like_cpp"),
        ("BattlePetAccountRegistryLikeCpp", "new_with_persistence_like_cpp"),
    ),
    ACCOUNT_ADD: (
        ("BattlePetAccountOwnerLikeCpp", "receipt_probe_for_account_fenced_like_cpp"),
        ("BattlePetAccountOwnerLikeCpp", "add_request_committed_like_cpp"),
        ("BattlePetAccountOwnerLikeCpp", "try_add_pet_like_cpp"),
    ),
    ACCOUNT_MUTATE: (
        ("BattlePetAccountOwnerLikeCpp", "try_mutate_pet_like_cpp"),
        ("BattlePetAccountOwnerLikeCpp", "try_mutate_pet_without_lease_like_cpp"),
        ("BattlePetAccountOwnerLikeCpp", "try_remove_pet_like_cpp"),
        ("BattlePetAccountOwnerLikeCpp", "try_set_slot_like_cpp"),
    ),
    ACCOUNT_QUERIES: (
        ("BattlePetAccountOwnerLikeCpp", "species_entry_like_cpp"),
        ("BattlePetAccountOwnerLikeCpp", "species_has_flag_like_cpp"),
        ("BattlePetAccountOwnerLikeCpp", "xp_per_level_like_cpp"),
        ("BattlePetAccountOwnerLikeCpp", "calculate_stats_like_cpp"),
        ("BattlePetAccountOwnerLikeCpp", "journal_like_cpp"),
        ("BattlePetAccountOwnerLikeCpp", "pet_snapshot_like_cpp"),
        ("BattlePetAccountOwnerLikeCpp", "max_pet_level_like_cpp"),
        ("BattlePetAccountOwnerLikeCpp", "has_max_pet_count_like_cpp"),
        ("BattlePetAccountOwnerLikeCpp", "pet_count_like_cpp"),
        ("BattlePetAccountOwnerLikeCpp", "unique_species_count_like_cpp"),
    ),
}

ALIAS_BLOCKS = (
    (
        "battle-pet persistence aliases",
        """pub(crate) use wow_persistence::{
    BattlePetAccountPersistencePortLikeCpp as BattlePetPersistenceLikeCpp,
    BattlePetAddRequestKeyLikeCpp, BattlePetPersistenceErrorLikeCpp, BattlePetProcessLeaseLikeCpp,
    DurableBattlePetAddLikeCpp, DurableBattlePetRowLikeCpp, DurableBattlePetSlotLikeCpp,
    LoadedBattlePetAccountLikeCpp, PersistBattlePetAddOutcomeLikeCpp,
};""",
        """pub use wow_persistence::{
    BattlePetAccountPersistencePortLikeCpp as BattlePetPersistenceLikeCpp,
    BattlePetAddRequestKeyLikeCpp, BattlePetPersistenceErrorLikeCpp, BattlePetProcessLeaseLikeCpp,
    DurableBattlePetAddLikeCpp, DurableBattlePetRowLikeCpp, DurableBattlePetSlotLikeCpp,
    LoadedBattlePetAccountLikeCpp, PersistBattlePetAddOutcomeLikeCpp,
};""",
    ),
    (
        "test-fixture receipt alias",
        """#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_persistence::{
    DurableBattlePetAddReceiptLikeCpp, PersistenceFutureLikeCpp as PersistenceFuture,
};""",
        """#[cfg(any(test, feature = "test-fixtures"))]
pub use wow_persistence::DurableBattlePetAddReceiptLikeCpp;
#[cfg(test)]
pub(crate) use wow_persistence::PersistenceFutureLikeCpp as PersistenceFuture;""",
    ),
)


class CodemodError(Exception):
    """The moved account files do not match the reviewed visibility cut."""


def _lexer(root: Path):
    return item_tools.lib(root)


def _promote_match(source: str, match: re.Match[str], label: str) -> tuple[str, bool]:
    visibility = match.group("visibility")
    if visibility == "pub":
        return source, False
    if visibility != "pub(crate)":
        raise CodemodError(f"unexpected visibility for {label}: {visibility!r}")
    start, end = match.span("visibility")
    return source[:start] + "pub" + source[end:], True


def promote_type(
    source: str,
    lexer,
    kind: str,
    name: str,
) -> tuple[str, bool]:
    """Promote one exact struct/enum declaration; fields remain untouched."""
    code = lexer.blank_noncode(source)
    pattern = re.compile(
        rf"(?m)^[ \t]*(?:(?P<visibility>pub(?:\([^)]*\))?)[ \t]+)?"
        rf"{re.escape(kind)}[ \t]+{re.escape(name)}\b"
    )
    matches = list(pattern.finditer(code))
    if len(matches) != 1:
        raise CodemodError(f"expected one {kind} declaration for {name}, found {len(matches)}")
    return _promote_match(source, matches[0], f"{kind} {name}")


def promote_method(
    source: str,
    lexer,
    owner: str,
    name: str,
) -> tuple[str, bool]:
    """Promote one named async/sync method in its reviewed inherent impl."""
    code = lexer.blank_noncode(source)
    pattern = re.compile(
        rf"(?m)^[ \t]*(?:(?P<visibility>pub(?:\([^)]*\))?)[ \t]+)?"
        rf"(?:async[ \t]+)?fn[ \t]+{re.escape(name)}\b"
    )
    candidates = [
        match
        for start, end, self_type in item_tools.impl_blocks(lexer, code)
        if self_type == owner
        for match in pattern.finditer(code, start, end)
    ]
    if len(candidates) != 1:
        raise CodemodError(
            f"expected one method {owner}::{name}, found {len(candidates)}"
        )
    return _promote_match(source, candidates[0], f"method {owner}::{name}")


def _replace_exact_block(source: str, label: str, old: str, new: str) -> tuple[str, bool]:
    old_count = source.count(old)
    new_count = source.count(new)
    if old_count == 1 and new_count == 0:
        return source.replace(old, new, 1), True
    if old_count == 0 and new_count == 1:
        return source, False
    raise CodemodError(f"expected one original or widened {label} block")


def transform_file(path: Path, source: str, lexer) -> tuple[str, list[str]]:
    changed: list[str] = []
    for item_path, kind, name in TYPE_ITEMS:
        if item_path != path:
            continue
        source, did_change = promote_type(source, lexer, kind, name)
        if did_change:
            changed.append(f"{kind} {name}")

    for owner, name in METHOD_ITEMS.get(path, ()):
        source, did_change = promote_method(source, lexer, owner, name)
        if did_change:
            changed.append(f"{owner}::{name}")

    if path == ACCOUNT_ROOT:
        for label, old, new in ALIAS_BLOCKS:
            source, did_change = _replace_exact_block(source, label, old, new)
            if did_change:
                changed.append(label)
    return source, changed


def run(action: str, root: Path) -> None:
    paths = {relative: root / relative for relative in FILES}
    missing = [path for path in paths.values() if not path.is_file()]
    if missing:
        raise CodemodError(f"missing moved account source: {missing[0]}")

    lexer = _lexer(root)
    results: dict[Path, str] = {}
    report: list[tuple[Path, str]] = []
    for relative, path in paths.items():
        original = path.read_text(encoding="utf-8")
        updated, names = transform_file(relative, original, lexer)
        results[path] = updated
        report.extend((relative, name) for name in names)

    print(f"battle-pet account visibility: {len(report)} exact promotions")
    for path, name in report:
        print(f"  {path}: {name}")
    if action == "apply":
        for path, updated in results.items():
            original = path.read_text(encoding="utf-8")
            if updated != original:
                path.write_text(updated, encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("plan", "apply"))
    parser.add_argument("--root", type=Path, default=REPO)
    args = parser.parse_args()
    try:
        run(args.action, args.root)
    except (CodemodError, OSError) as error:
        print(f"f4_battle_pet_visibility: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
