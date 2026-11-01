#!/usr/bin/env python3
"""Widen only the directory getter used by the retained world shell."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path


CODEMODS = Path(__file__).resolve().parent
sys.path.insert(0, str(CODEMODS))

from f4_extract_core import CodemodError, widen_declaration  # noqa: E402


ROOT = Path(__file__).resolve().parents[2]
TARGET = Path("crates/wow-world-core/src/session/directory.rs")
METHOD = "canonical_map_manager_like_cpp"


def transform(source: str) -> tuple[str, bool]:
    return widen_declaration(source, "method", METHOD)


def selftest() -> None:
    fixture = "\n".join(
        (
            "impl PlayerRegistry {",
            "    pub(crate) fn canonical_map_manager_like_cpp(&self) -> Option<Map> {",
            "        None",
            "    }",
            "    pub(crate) fn fixture_only_helper(&self) {}",
            "}",
        )
    )
    expected = fixture.replace(
        "pub(crate) fn canonical_map_manager_like_cpp",
        "pub fn canonical_map_manager_like_cpp",
        1,
    )
    updated, changed = transform(fixture)
    if updated != expected or not changed:
        raise SystemExit("selftest failed: getter did not match its exact allowlist")
    repeated, changed_again = transform(updated)
    if repeated != updated or changed_again:
        raise SystemExit("selftest failed: reapplying the visibility change was not idempotent")
    if "pub(crate) fn fixture_only_helper" not in updated:
        raise SystemExit("selftest failed: unrelated crate visibility changed")

    for label, source in (
        ("missing", "impl PlayerRegistry { fn unrelated(&self) {} }"),
        (
            "ambiguous",
            "\n".join(
                (
                    "impl PlayerRegistry {",
                    "    pub(crate) fn canonical_map_manager_like_cpp(&self) {}",
                    "    pub(crate) fn canonical_map_manager_like_cpp(&self) {}",
                    "}",
                )
            ),
        ),
    ):
        try:
            transform(source)
        except CodemodError:
            continue
        raise SystemExit(f"selftest failed: {label} declaration was accepted")

    print("selftest passed: only the allowlisted directory getter changes")


def run(action: str, root: Path) -> None:
    path = root / TARGET
    if not path.is_file():
        raise CodemodError(f"missing moved directory source: {path}")
    original = path.read_text(encoding="utf-8")
    updated, changed = transform(original)
    if not changed:
        print(f"{path}: already applied")
        return
    if action == "plan":
        print(f"{path}: would widen method {METHOD}")
        return
    if path.read_text(encoding="utf-8") != original:
        raise CodemodError(f"refusing concurrent change to {path}; rerun plan")
    path.write_text(updated, encoding="utf-8")
    print(f"{path}: widened method {METHOD}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("plan", "apply", "selftest"))
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    if args.action == "selftest":
        selftest()
        return 0
    try:
        run(args.action, args.root)
    except (CodemodError, OSError) as error:
        print(f"f4_directory_visibility: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
