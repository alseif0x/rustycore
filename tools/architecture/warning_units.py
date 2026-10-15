#!/usr/bin/env python3
"""Per-unit warning counter for the canonical warning recipe (#1363, #1404 review).

Reads the JSON stream of one recipe command (`cargo check ... --jobs 1
--message-format json`) and attributes every warning to the unit that emitted
it. Cargo prints a unit's compiler messages before that unit's
`compiler-artifact` line (cached units replay theirs), so a message belongs to
the next artifact of the same package and target. A unit is the target name
plus its kinds, with `+test` for the `cfg(test)` build of a library.

    warning_units.py count lib.json [--detail lib.tsv]
    warning_units.py compare base.tsv head.tsv [--map OLD=NEW ...]
    warning_units.py self-test

`count` prints the errors, the warnings per unit and the `wow_world*` library
total, and the per-lint split of the `wow_world` units. `--detail` writes one
tab-separated row per warning (unit, kind, lint, file, line, message) for every
unit. `compare` matches two detail files by unit kind, lint, file and message (line
numbers are ignored; `--map` renames a moved path prefix in the base rows) and
prints what only one side has. The crate name is not part of the key, so a
warning that moves with its body to another crate matches through `--map`
(the file path already names the crate). It exits 1 when the head has a warning the base
does not have, so a new warning cannot pass unnoticed.
"""
from __future__ import annotations

import argparse
import collections
import json
import sys
import tempfile


def attribute(lines):
    """Return (errors, [(unit, [warning...])]) from cargo JSON lines."""
    pending = collections.defaultdict(list)
    units = []
    errors = 0
    for line in lines:
        if not line.startswith("{"):
            continue
        msg = json.loads(line)
        key = (msg.get("package_id"), msg.get("target", {}).get("name"))
        if msg.get("reason") == "compiler-message":
            body = msg["message"]
            if body["level"] == "error":
                errors += 1
            if body["level"] != "warning" or not body["spans"]:
                continue
            span = [s for s in body["spans"] if s["is_primary"]][0]
            code = (body.get("code") or {}).get("code") or "none"
            pending[key].append((code, span["file_name"], span["line_start"], body["message"]))
        elif msg.get("reason") == "compiler-artifact":
            target = msg["target"]
            kind = ",".join(target["kind"]) + ("+test" if msg["profile"]["test"] else "")
            units.append(((target["name"], kind), pending.pop(key, [])))
    unattributed = sum(len(v) for v in pending.values())
    return errors, unattributed, units


def count(path, detail=None, out=sys.stdout):
    with open(path, encoding="utf-8") as handle:
        errors, unattributed, units = attribute(handle)
    print(f"errors {errors} unattributed {unattributed}", file=out)
    total = 0
    lints = collections.Counter()
    rows = []
    for (name, kind), warnings in units:
        if not warnings:
            continue
        if name.startswith("wow_world") and kind.startswith("lib") and not kind.endswith("+test"):
            total += len(warnings)
        print(f"{len(warnings)}\t{name}\t{kind}", file=out)
        for code, file_name, line, message in warnings:
            if name == "wow_world":
                lints[(kind, code)] += 1
            rows.append("\t".join(map(str, (name, kind, code, file_name, line, message))))
    print(f"wow_world* lib units total {total}", file=out)
    for (kind, code), n in sorted(lints.items()):
        print(f"  wow_world {kind} {code} {n}", file=out)
    if detail:
        with open(detail, "w", encoding="utf-8") as handle:
            handle.write("\n".join(sorted(rows)) + ("\n" if rows else ""))
    return errors


def _keys(path, mapping=()):
    keys = collections.Counter()
    with open(path, encoding="utf-8") as handle:
        for row in handle:
            if not row.strip():
                continue
            name, kind, code, file_name, _line, message = row.rstrip("\n").split("\t", 5)
            for old, new in mapping:
                if file_name.startswith(old):
                    file_name = new + file_name[len(old):]
            keys[(kind, code, file_name, message)] += 1
    return keys


def compare(base, head, mapping=(), out=sys.stdout):
    base_keys = _keys(base, mapping)
    head_keys = _keys(head)
    only_base = base_keys - head_keys
    only_head = head_keys - base_keys
    print(f"base {sum(base_keys.values())} head {sum(head_keys.values())}", file=out)
    for label, keys in (("only in base", only_base), ("only in head", only_head)):
        print(f"{label}: {sum(keys.values())}", file=out)
        for key, n in sorted(keys.items()):
            print(f"  {n} x {' | '.join(map(str, key))}", file=out)
    return 1 if only_head else 0


def self_test():
    def artifact(name, test=False):
        return json.dumps({"reason": "compiler-artifact", "package_id": name,
                           "target": {"name": name, "kind": ["lib"]}, "profile": {"test": test}})

    def warning(name, code, file_name, message):
        return json.dumps({"reason": "compiler-message", "package_id": name, "target": {"name": name},
                           "message": {"level": "warning", "code": {"code": code}, "message": message,
                                       "spans": [{"is_primary": True, "file_name": file_name,
                                                  "line_start": 3}]}})

    base_stream = [warning("wow_world", "unused_imports", "src/a.rs", "unused import: `X`"),
                   artifact("wow_world"),
                   warning("wow_world_quest", "unused_mut", "src/q.rs", "variable does not need to be mutable"),
                   artifact("wow_world_quest")]
    head_stream = [warning("wow_world", "unused_mut", "src/moved/q.rs", "variable does not need to be mutable"),
                   artifact("wow_world"),
                   artifact("wow_world_quest")]
    errors, unattributed, units = attribute(base_stream)
    assert errors == 0 and unattributed == 0
    assert [len(w) for _, w in units] == [1, 1], units
    with tempfile.TemporaryDirectory() as tmp:
        paths = {}
        for label, stream in (("base", base_stream), ("head", head_stream)):
            src = f"{tmp}/{label}.json"
            with open(src, "w", encoding="utf-8") as handle:
                handle.write("\n".join(stream) + "\n")
            paths[label] = f"{tmp}/{label}.tsv"
            count(src, paths[label], out=open("/dev/null", "w"))
        sink = open("/dev/null", "w")
        assert compare(paths["base"], paths["head"], [("src/q.rs", "src/moved/q.rs")], out=sink) == 0
        assert compare(paths["head"], paths["base"], out=sink) == 1, "a new warning must fail"
    print("warning_units self-test: PASS")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    p_count = sub.add_parser("count")
    p_count.add_argument("json")
    p_count.add_argument("--detail")
    p_compare = sub.add_parser("compare")
    p_compare.add_argument("base")
    p_compare.add_argument("head")
    p_compare.add_argument("--map", action="append", default=[], help="OLD=NEW path prefix in the base")
    sub.add_parser("self-test")
    args = parser.parse_args()
    if args.command == "count":
        count(args.json, args.detail)
        return 0
    if args.command == "compare":
        return compare(args.base, args.head, [tuple(m.split("=", 1)) for m in args.map])
    return self_test()


if __name__ == "__main__":
    sys.exit(main())
