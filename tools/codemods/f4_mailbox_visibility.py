#!/usr/bin/env python3
"""Apply the narrow cross-crate visibility changes required by F4 mailbox moves."""

from __future__ import annotations

import argparse
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
TARGETS = {
    Path("crates/wow-world-core/src/session/mailbox/session_phase_rail.rs"): (
        (
            "PendingWorldPhaseFinalizationLikeCpp.permit",
            "pub struct PendingWorldPhaseFinalizationLikeCpp {\n"
            "    pub(crate) permit: Arc<SessionPhasePermitLikeCpp>,",
            "pub struct PendingWorldPhaseFinalizationLikeCpp {\n"
            "    pub permit: Arc<SessionPhasePermitLikeCpp>,",
        ),
        (
            "PendingWorldPhaseFinalizationLikeCpp.response_tx",
            "pub struct PendingWorldPhaseFinalizationLikeCpp {\n"
            "    pub permit: Arc<SessionPhasePermitLikeCpp>,\n"
            "    pub(crate) response_tx: flume::Sender<RunWorldPhasePassResultLikeCpp>,",
            "pub struct PendingWorldPhaseFinalizationLikeCpp {\n"
            "    pub permit: Arc<SessionPhasePermitLikeCpp>,\n"
            "    pub response_tx: flume::Sender<RunWorldPhasePassResultLikeCpp>,",
        ),
        (
            "PendingWorldPhaseFinalizationLikeCpp.result",
            "pub struct PendingWorldPhaseFinalizationLikeCpp {\n"
            "    pub permit: Arc<SessionPhasePermitLikeCpp>,\n"
            "    pub response_tx: flume::Sender<RunWorldPhasePassResultLikeCpp>,\n"
            "    pub(crate) result: RunWorldPhasePassResultLikeCpp,",
            "pub struct PendingWorldPhaseFinalizationLikeCpp {\n"
            "    pub permit: Arc<SessionPhasePermitLikeCpp>,\n"
            "    pub response_tx: flume::Sender<RunWorldPhasePassResultLikeCpp>,\n"
            "    pub result: RunWorldPhasePassResultLikeCpp,",
        ),
    ),
    Path("crates/wow-world-core/src/session/mailbox/protocol.rs"): (
        (
            "SessionCommand.is_visibility_gated_like_cpp",
            "    pub(super) fn is_visibility_gated_like_cpp(",
            "    pub fn is_visibility_gated_like_cpp(",
        ),
    ),
}


def rewrite_one(
    text: str, rewrite: tuple[str, str, str]
) -> tuple[str, bool]:
    name, old, new = rewrite
    old_count = text.count(old)
    new_count = text.count(new)
    if old_count + new_count != 1:
        raise ValueError(
            f"expected exactly one old or new candidate for {name}; "
            f"found old={old_count} new={new_count}"
        )
    if old_count == 1:
        return text.replace(old, new, 1), True
    return text, False


def rewrite(text: str, rewrites: tuple[tuple[str, str, str], ...]) -> tuple[str, list[str]]:
    changed: list[str] = []
    for item in rewrites:
        text, did_change = rewrite_one(text, item)
        if did_change:
            changed.append(item[0])
    return text, changed


def selftest() -> None:
    fixture = "\n".join(
        (
            "pub struct PendingWorldPhaseFinalizationLikeCpp {",
            "    pub(crate) permit: Arc<SessionPhasePermitLikeCpp>,",
            "    pub(crate) response_tx: flume::Sender<RunWorldPhasePassResultLikeCpp>,",
            "    pub(crate) result: RunWorldPhasePassResultLikeCpp,",
            "}",
            "impl DurableCreatureRuntimeCommandsLikeCpp {",
            "    pub(crate) fn retain_deferred_visibility_like_cpp(",
            "}",
            "impl SessionCommand {",
            "    pub(super) fn is_visibility_gated_like_cpp(",
            "    pub(crate) fn unrelated_declaration(",
            "}",
        )
    )
    expected = "\n".join(
        (
            "pub struct PendingWorldPhaseFinalizationLikeCpp {",
            "    pub permit: Arc<SessionPhasePermitLikeCpp>,",
            "    pub response_tx: flume::Sender<RunWorldPhasePassResultLikeCpp>,",
            "    pub result: RunWorldPhasePassResultLikeCpp,",
            "}",
            "impl DurableCreatureRuntimeCommandsLikeCpp {",
            "    pub(crate) fn retain_deferred_visibility_like_cpp(",
            "}",
            "impl SessionCommand {",
            "    pub fn is_visibility_gated_like_cpp(",
            "    pub(crate) fn unrelated_declaration(",
            "}",
        )
    )
    combined = fixture
    names: list[str] = []
    for rewrites in TARGETS.values():
        combined, changed = rewrite(combined, rewrites)
        names.extend(changed)
    if combined != expected or len(names) != sum(len(items) for items in TARGETS.values()):
        raise SystemExit("selftest failed: rewrite escaped its declaration allowlist")
    repeated, changed = combined, []
    for rewrites in TARGETS.values():
        repeated, item_changes = rewrite(repeated, rewrites)
        changed.extend(item_changes)
    if repeated != combined or changed:
        raise SystemExit("selftest failed: exact reapplication was not idempotent")

    probe = TARGETS[Path("crates/wow-world-core/src/session/mailbox/session_phase_rail.rs")][0]
    for label, text in (
        ("missing", "struct OtherType { field: u8 }"),
        ("old-plus-new", probe[1] + "\n" + probe[2]),
        ("duplicate-new", probe[2] + "\n" + probe[2]),
    ):
        try:
            rewrite_one(text, probe)
        except ValueError:
            continue
        raise SystemExit(f"selftest failed: {label} candidate was accepted")
    print("selftest passed: only the four allowlisted declarations change")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=("plan", "apply", "selftest"))
    args = parser.parse_args()
    if args.action == "selftest":
        selftest()
        return

    planned: dict[Path, tuple[str, str, list[str]]] = {}
    for relative, rewrites in TARGETS.items():
        path = ROOT / relative
        original = path.read_text(encoding="utf-8")
        updated, changed = rewrite(original, rewrites)
        planned[relative] = (original, updated, changed)
    changes = [(path, changed) for path, (_, _, changed) in planned.items() if changed]
    if args.action == "plan":
        if changes:
            for path, names in changes:
                print(f"would widen {path}: " + ", ".join(names))
        else:
            print("already applied: all four allowlisted declarations")
        return
    for relative, (original, _, _) in planned.items():
        if (ROOT / relative).read_text(encoding="utf-8") != original:
            raise SystemExit(f"refusing concurrent change to {relative}; rerun plan")
    for relative, (original, updated, _) in planned.items():
        path = ROOT / relative
        if updated != original:
            path.write_text(updated, encoding="utf-8")
    if changes:
        for path, names in changes:
            print(f"widened {path}: " + ", ".join(names))
    else:
        print("already applied: all four allowlisted declarations")


if __name__ == "__main__":
    main()
