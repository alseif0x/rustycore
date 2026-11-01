#!/usr/bin/env python3
"""#1241 F3-0: fold the 11 cfg(test)-only WorldSession groups into `fixtures`.

`WorldSession` gains one `#[cfg(test)] fixtures: SessionFixtures` member, declared
where the first of the 11 groups (`identity`) is today. `SessionFixtures`
(`session/state/fixtures.rs`) nests the 11 group structs unchanged under their
current names, so every access gains exactly one path segment
(`self.progression.x` -> `self.fixtures.progression.x`). The groups keep their
own struct types, visibilities, documentation and relative order; none of them
owns anything with an observable drop, so moving them together does not touch
the drop-order contract on `WorldSession`.

Step 1 (text) moves the 11 member declarations and construction-literal entries.
Step 2 is the F2 compiler-guided loop (E0609/E0615 naming `WorldSession`),
inserting `fixtures.` before the reported group ident; rustc's E0615 method
suggestion is never applied. A second `apply` is a no-op. Standard library only.
"""
from __future__ import annotations

import argparse
import importlib.util
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
_spec = importlib.util.spec_from_file_location("f2_session_substates", HERE / "f2_session_substates.py")
f2 = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(f2)

GROUPS = (
    "identity", "collections", "auras", "progression", "combat", "movement",
    "teleport", "vehicles", "pets", "battleground", "presentation",
)
# Narrowest visibility that compiles: handler test modules outside
# `crate::session` read `auras`, `collections`, `progression`, `combat`,
# `vehicles` and `pets` through it (see the F2 visibility table).
VISIBILITY = "pub(crate)"
FIXTURES_FILE = f2.STATE_DIR / "fixtures.rs"
MARKER = re.compile(r"^    pub(?:\([^)]*\))? fixtures: SessionFixtures,$", re.M)
DOC = (
    "Test-only fixture groups (#1241 F3-0): the 11 cfg(test) domain groups, nested "
    "unchanged so an F3 context borrows one member instead of eleven."
)


def step1() -> bool:
    text = f2.STATE.read_text(encoding="utf-8")
    if MARKER.search(text):
        return False
    code = f2.blank_noncode(text)
    open_index, close, fields = f2.struct_fields(
        text, code, re.compile(r"\bpub\s+struct\s+WorldSession\s*\{")
    )
    names = [f.name for f in fields]
    if any(g not in names for g in GROUPS):
        raise f2.CodemodError(f"WorldSession lacks a fixture group: {sorted(set(GROUPS) - set(names))}")
    moved = [f for f in fields if f.name in GROUPS]
    if [f.name for f in moved] != list(GROUPS) or not all(f.cfg_test for f in moved):
        raise f2.CodemodError("fixture groups are not the expected cfg(test) members in order")
    wrapper = (
        "\n" + f2.doc_lines(DOC, "    ") + "    #[cfg(test)]\n"
        + f"    {VISIBILITY} fixtures: SessionFixtures"
    )
    kept = []
    for field in fields:
        if field.name == GROUPS[0]:
            kept.append(wrapper)
        if field.name not in GROUPS:
            kept.append(field.raw)
    new_text = text[:open_index + 1] + f2.emit_fields(kept) + text[close:]
    decl = "#[cfg(test)]\nmod fixtures;\n#[cfg(test)]\npub(in crate::session) use fixtures::SessionFixtures;\n"
    anchor = new_text.index("mod session_core;\n")
    new_text = new_text[:anchor] + decl + new_text[anchor:]

    body = f2.emit_fields([f.raw for f in moved])
    fixtures = (
        f2.HEADER
        + "//! `WorldSession::fixtures` (#1241 F3-0): the cfg(test) groups, moved unchanged.\n\n"
        + "use super::*;\n\n"
        + f2.doc_lines(DOC, "") + f"{VISIBILITY} struct SessionFixtures {{{body}}}\n"
    )
    construction = rewrite_construction()
    FIXTURES_FILE.write_text(fixtures, encoding="utf-8")
    f2.STATE.write_text(new_text, encoding="utf-8")
    f2.CONSTRUCTION.write_text(construction, encoding="utf-8")
    return True


def rewrite_construction() -> str:
    text = f2.CONSTRUCTION.read_text(encoding="utf-8")
    code = f2.blank_noncode(text)
    m = re.search(r"\n        Self \{", code)
    if not m:
        raise f2.CodemodError("WorldSession::new literal not found")
    open_index = m.end() - 1
    close = f2.matching_close(code, open_index)
    entries = []
    for a, b in f2.split_segments(code, open_index + 1, close):
        if not code[a:b].strip():
            continue
        rest, _ = f2.strip_attrs(code[a:b])
        n = re.match(r"(" + f2.IDENT + r")\s*(?::|$)", rest.strip())
        if not n:
            raise f2.CodemodError(f"cannot parse literal entry {rest[:60]!r}")
        entries.append((n.group(1), text[a:b]))
    moved = [raw for name, raw in entries if name in GROUPS]
    if len(moved) != len(GROUPS):
        raise f2.CodemodError("construction literal does not initialise every fixture group")
    out = []
    for name, raw in entries:
        if name == GROUPS[0]:
            out.append(
                "\n            #[cfg(test)]\n            fixtures: SessionFixtures {"
                + f2.emit_fields(moved) + "            }"
            )
        if name not in GROUPS:
            out.append(raw)
    new_text = text[:open_index + 1] + f2.emit_fields(out) + "        " + text[close:]
    # Before the first `use crate::session::state::` item, including its attributes.
    anchor = re.search(r"^(?:#\[[^\n]*\]\n)*use crate::session::state::", new_text, re.M).start()
    return (
        new_text[:anchor] + "#[cfg(test)]\nuse crate::session::state::SessionFixtures;\n"
        + new_text[anchor:]
    )


def step2(log_dir: pathlib.Path, max_rounds: int) -> list[int]:
    """The F2 loop with a segment table that maps every fixture group to `fixtures`."""
    rounds = []
    for _ in range(max_rounds):
        status, messages = f2.cargo_check(log_dir / f"f3-0-round-{len(rounds) + 1}.jsonl", [])
        edits: dict[pathlib.Path, set[tuple[int, int, str]]] = {}
        unexpected = []
        for message in messages:
            if message.get("level") != "error":
                continue
            code = (message.get("code") or {}).get("code")
            m = f2.TARGET_MESSAGE.match(message["message"]) if code in f2.PATCH_CODES else None
            name = m and (m.group(1) or m.group(2))
            if name not in GROUPS:
                unexpected.append(f"{code}: {message['message']}")
                continue
            span = next(s for s in message["spans"] if s["is_primary"])
            edits.setdefault(f2.REPO / span["file_name"], set()).add(
                (span["byte_start"], span["byte_end"], name)
            )
        count = 0
        for path, spans in edits.items():
            data = path.read_bytes()
            for start, end, name in sorted(spans, reverse=True):
                if data[start:end].decode("utf-8") != name:
                    raise f2.CodemodError(f"{path}:{start}: span text is not {name!r}")
                data = data[:start] + b"fixtures." + data[start:]
                count += 1
            path.write_bytes(data)
        rounds.append(count)
        print(f"round {len(rounds)}: exit {status}, {count} spans patched", file=sys.stderr)
        if count == 0:
            if status != 0:
                for line in unexpected[:40]:
                    print("unhandled:", line, file=sys.stderr)
                raise f2.CodemodError(f"cargo check still fails with {len(unexpected)} unhandled errors")
            return rounds
    raise f2.CodemodError("no fixed point within the round budget")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    apply = sub.add_parser("apply", help="text step, then the compiler-guided loop")
    apply.add_argument("--text-only", action="store_true")
    apply.add_argument("--log-dir", default=str(f2.REPO / "target/f3-0-codemod"))
    apply.add_argument("--max-rounds", type=int, default=8)
    args = parser.parse_args()
    try:
        changed = step1()
        print(f"text step: {'applied' if changed else 'already applied (no-op)'}", file=sys.stderr)
        if not args.text_only:
            log_dir = pathlib.Path(args.log_dir)
            log_dir.mkdir(parents=True, exist_ok=True)
            rounds = step2(log_dir, args.max_rounds)
            print(f"compiler loop: {len(rounds)} round(s), spans per round {rounds}", file=sys.stderr)
    except f2.CodemodError as error:
        print(f"f3-0 codemod: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
