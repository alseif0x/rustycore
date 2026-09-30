#!/usr/bin/env python3
"""#1241 F1: relocate test-only wow-world modules out of `src/` (mechanism B).

Each test-only file moves to `crates/wow-world/unit_tests/<same src-relative path>`
and stays a `#[cfg(test)]` module of the library: every `mod` declaration keeps
its parent, so module paths, visibility (`pub(super)`, `pub(in ..)`) and test
names do not change. Only the boundary mounts (a parent that stays in `src/`
declaring a child that moves) get a new or rewritten `#[path]`; mounts between
moved files keep their relative meaning because the layout is mirrored. Every
`include_str!`/`include_bytes!` in a moved file is re-pointed at the same target.

`plan` prints the edit set; `apply` performs it. All invariants are checked
before anything changes, and a second `apply` is a no-op. Standard library only.
"""
from __future__ import annotations

import argparse
import os
import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]
CRATE = REPO / "crates/wow-world"
SRC = CRATE / "src"
DEST = CRATE / "unit_tests"
TEST_PATH = re.compile(r"(_tests?(/|\.rs)|/tests?(/|\.rs)|test_support|test_fixture|fixtures)")
# Path-classed files that stay: compiled in production or under `test-fixtures`,
# or plain-mounted session fixtures whose move would reclassify hotspot lines.
KEEP = frozenset({
    "session/test_support/mod.rs",
    "session/test_support/operations.rs",
    "session/test_support/test_fixtures.rs",
    "player/directory_test_fixtures.rs",
    "session/directory/test_fixtures.rs",
    "session/instances/test_fixtures.rs",
    "session/persistence/test_fixtures.rs",
    "session/pets/test_fixtures.rs",
    "session/player_items/test_fixtures.rs",
    "session/quest/test_fixtures.rs",
    "session/social/test_fixtures.rs",
    "session/support_features/test_fixtures.rs",
    "session/visibility/test_fixtures.rs",
})
IDENT = r"[A-Za-z_][A-Za-z0-9_]*"
MOD_DECL = re.compile(
    r"((?:#\s*\[[^\]]*\]\s*)*)((?:pub(?:\s*\([^)]*\))?\s+)?mod\s+(" + IDENT + r")\s*;)"
)
PATH_ATTR = re.compile(r'#\s*\[\s*path\s*=\s*"([^"]*)"\s*\]')
CFG_ATTR = re.compile(r"#\s*\[\s*(cfg\s*\(.*?\))\s*\]", re.S)
INCLUDE = re.compile(r"\binclude_(?:str|bytes)\s*!\s*\(")
LITERAL = re.compile(r'\(\s*"([^"\\]*)"')
CHAR_LIT = re.compile(r"'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]+\}|.)|[^\\'\n])'")


class CodemodError(RuntimeError):
    pass


def blank(text: str) -> str:
    """Blank comments and literal contents, keeping every offset and newline."""
    out = list(text)
    i, n = 0, len(text)

    def wipe(a: int, b: int) -> None:
        for k in range(a, min(b, n)):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        c = text[i]
        if text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            wipe(i, j)
            i = j
        elif text.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif text.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            wipe(i, j)
            i = j
        elif (m := re.compile(r'b?r(#*)"').match(text, i)) and (i == 0 or not text[i - 1].isalnum()):
            end = text.find('"' + m.group(1), m.end())
            end = n if end < 0 else end
            wipe(m.end(), end)
            i = end + 1 + len(m.group(1))
        elif c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            wipe(i + 1, j)
            i = j + 1
        elif c == "'" and (m := CHAR_LIT.match(text, i)):
            wipe(i + 1, m.end() - 1)
            i = m.end()
        else:
            i += 1
    return "".join(out)


def logical(path: pathlib.Path) -> str:
    """Src-relative identity of a file, wherever it currently lives."""
    for root in (SRC, DEST):
        try:
            return path.relative_to(root).as_posix()
        except ValueError:
            continue
    raise CodemodError(f"module file outside src/ and unit_tests/: {path}")


def moves(rel: str) -> bool:
    return bool(TEST_PATH.search(rel)) and rel not in KEEP


def final_path(rel: str) -> pathlib.Path:
    return (DEST if moves(rel) else SRC) / rel


def relpath(target: pathlib.Path, start: pathlib.Path) -> str:
    return pathlib.Path(os.path.relpath(target, start)).as_posix()


def module_tree() -> dict[pathlib.Path, dict]:
    """Walk lib.rs like rustc: each file with its mount and declared children."""
    files: dict[pathlib.Path, dict] = {}
    pending = [(SRC / "lib.rs", True, None)]
    while pending:
        path, owns_dir, mount = pending.pop()
        path = path.resolve()
        if path in files:
            raise CodemodError(f"file mounted twice: {path}")
        text = path.read_text(encoding="utf-8")
        code = blank(text)
        node = {"text": text, "code": code, "owns_dir": owns_dir, "mount": mount, "children": []}
        files[path] = node
        depth_at = brace_depths(code)
        for m in MOD_DECL.finditer(code):
            if depth_at[m.start(2)]:
                raise CodemodError(f"{path}: `mod {m.group(3)};` inside a block is unsupported")
            attrs = text[m.start(1):m.end(1)]
            pm = PATH_ATTR.search(attrs)
            if pm:
                child, child_owns = (path.parent / pm.group(1)), True
            else:
                base = path.parent if owns_dir else path.parent / path.stem
                flat, nested = base / f"{m.group(3)}.rs", base / m.group(3) / "mod.rs"
                if flat.exists() == nested.exists():
                    raise CodemodError(f"{path}: cannot resolve `mod {m.group(3)};`")
                child, child_owns = (flat, False) if flat.exists() else (nested, True)
            decl = {
                "parent": path, "name": m.group(3), "attrs": (m.start(1), m.end(1)),
                "item": m.start(2), "path_attr": pm and (m.start(1) + pm.start(1), m.start(1) + pm.end(1)),
                "cfg": [re.sub(r"\s+", "", c) for c in CFG_ATTR.findall(attrs)],
                "line": text.count("\n", 0, m.start(2)) + 1, "child": child.resolve(),
            }
            node["children"].append(decl)
            pending.append((child, child_owns, decl))
    return files


def brace_depths(code: str) -> list[int]:
    depths, depth = [0] * (len(code) + 1), 0
    for i, c in enumerate(code):
        depths[i] = depth
        depth += (c == "{") - (c == "}")
    depths[len(code)] = depth
    return depths


def plan() -> dict:
    files = module_tree()
    errors, mount_edits, include_edits, renames = [], [], [], []
    for path, node in files.items():
        rel = logical(path)
        if moves(rel) and path != (DEST / rel).resolve():
            renames.append((path, DEST / rel))
        for decl in node["children"]:
            crel = logical(decl["child"])
            where = f"{rel}:{decl['line']} mod {decl['name']}"
            if moves(rel) != moves(crel) and moves(rel):
                errors.append(f"{where}: moved parent mounts kept file {crel}")
            if moves(rel) and moves(crel):
                # Internal mount: must keep its meaning under the mirrored layout.
                if decl["path_attr"]:
                    want = relpath(final_path(crel), final_path(rel).parent)
                    have = node["text"][slice(*decl["path_attr"])]
                    if want != have:
                        errors.append(f"{where}: internal #[path] {have!r} would not stay valid")
                continue
            if not moves(crel):
                continue
            # Boundary: the parent stays in src/, the child moves.
            if decl["cfg"] != ["cfg(test)"]:
                errors.append(f"{where}: boundary mount of {crel} is not exactly #[cfg(test)]")
            child_node = files[decl["child"]]
            if not decl["path_attr"] and not child_node["owns_dir"]:
                plain_kids = [k for k in child_node["children"] if not k["path_attr"]]
                if plain_kids:
                    errors.append(f"{where}: plain non-mod.rs child {crel} has plain children; "
                                  "an explicit #[path] would relocate them")
            want = relpath(DEST / crel, path.parent)
            if decl["path_attr"]:
                have = node["text"][slice(*decl["path_attr"])]
                if have != want:
                    mount_edits.append((path, "replace", decl["path_attr"], want, where, have))
            else:
                mount_edits.append((path, "insert", decl["item"], want, where, None))
        if moves(rel):
            for m in INCLUDE.finditer(node["code"]):
                lit = LITERAL.match(node["text"], m.end() - 1)
                if not lit:
                    errors.append(f"{rel}:{node['text'].count(chr(10), 0, m.start()) + 1}: "
                                  "include macro without a plain literal")
                    continue
                target = (path.parent / lit.group(1)).resolve()
                if not target.exists():
                    errors.append(f"{rel}: include target {lit.group(1)!r} does not exist")
                    continue
                try:
                    trel = logical(target)
                    target = final_path(trel) if trel.endswith(".rs") else target
                except CodemodError:
                    pass
                want = relpath(target, (DEST / rel).parent)
                if want != lit.group(1):
                    span = (m.end() - 1 + lit.start(1) - lit.start(), m.end() - 1 + lit.end(1) - lit.start())
                    include_edits.append((path, span, want, rel, lit.group(1)))
    # Include targets under src/ that are not .rs (none today) must not be moved files.
    return {"files": files, "errors": errors, "renames": sorted(renames),
            "mounts": mount_edits, "includes": include_edits}


def render(result: dict) -> str:
    moved = [p for p in result["files"] if moves(logical(p))]
    lines = [f"module files: {len(result['files'])}; test-only files relocated to unit_tests/: {len(moved)}",
             f"pending renames: {len(result['renames'])}",
             f"pending boundary mount edits: {len(result['mounts'])}",
             f"pending include_str!/include_bytes! edits: {len(result['includes'])}"]
    for src, dst in result["renames"]:
        lines.append(f"  git mv {src.relative_to(REPO)} {dst.relative_to(REPO)}")
    for _, kind, _, want, where, have in result["mounts"]:
        lines.append(f"  mount {kind} {where}: {have!r} -> {want!r}" if have else
                     f"  mount {kind} {where}: #[path = {want!r}]")
    for _, _, want, rel, have in result["includes"]:
        lines.append(f"  include {rel}: {have!r} -> {want!r}")
    return "\n".join(lines)


def apply(result: dict) -> None:
    edits: dict[pathlib.Path, list] = {}
    for path, kind, where, want, _, _ in result["mounts"]:
        edits.setdefault(path, []).append((kind, where, want))
    for path, span, want, _, _ in result["includes"]:
        edits.setdefault(path, []).append(("replace", span, want))
    new_text = {}
    for path, items in edits.items():
        text = result["files"][path]["text"]
        for kind, where, want in sorted(items, key=lambda e: e[1] if e[0] == "insert" else e[1][0], reverse=True):
            if kind == "replace":
                text = text[:where[0]] + want + text[where[1]:]
            else:
                line_start = text.rfind("\n", 0, where) + 1
                indent = text[line_start:where]
                if indent.strip():
                    raise CodemodError(f"{path}: mod item does not start its line")
                text = text[:line_start] + f'{indent}#[path = "{want}"]\n' + text[line_start:]
        new_text[path] = text
    for src, dst in result["renames"]:
        dst.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(["git", "-C", str(REPO), "mv", str(src), str(dst)], check=True)
    for src, _ in result["renames"]:
        # Git does not track directories; drop the ones the move emptied.
        parent = src.parent
        while parent != SRC and parent.is_dir() and not any(parent.iterdir()):
            parent.rmdir()
            parent = parent.parent
    renamed = dict(result["renames"])
    for path, text in new_text.items():
        renamed.get(path, path).write_text(text, encoding="utf-8")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("mode", choices=("plan", "apply"))
    args = parser.parse_args(argv)
    try:
        result = plan()
        if result["errors"]:
            print("f1_relocate_unit_tests: invariant violations, nothing changed:", file=sys.stderr)
            for error in result["errors"]:
                print(f"  {error}", file=sys.stderr)
            return 1
        print(render(result))
        if args.mode == "apply":
            apply(result)
            after = plan()
            if after["errors"] or after["renames"] or after["mounts"] or after["includes"]:
                print("f1_relocate_unit_tests: apply did not converge", file=sys.stderr)
                print(render(after), file=sys.stderr)
                return 1
            print("applied; re-plan is empty")
    except CodemodError as exc:
        print(f"f1_relocate_unit_tests: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
