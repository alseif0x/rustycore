#!/usr/bin/env python3
"""Index of the production inherent `WorldSession` methods (#1363 done criterion).

Usage:
  python3 tools/architecture/session_method_index.py index
      [--repo PATH] [--methods] [--json PATH]
  python3 tools/architecture/session_method_index.py check
      [--repo PATH] [--policy PATH] [--done]
  python3 tools/architecture/session_method_index.py self-test

What it counts (the slice 12b1 index, made reproducible):

  * every `impl WorldSession { ... }` block in `crates/wow-world/src/**.rs`;
  * skipped: files whose name contains `shim`, files that are entirely
    `#![cfg(test)]`, impl blocks inside a top-level `cfg(test)` item or carrying
    `#[cfg(test)]` themselves, and methods whose own attributes include
    `#[cfg(test)]`;
  * comments and literals are blanked before parsing (hotspot_metrics);
  * a method's lines run from its first doc or attribute line through its
    closing brace;
  * a **forward** is a method whose body has at most one `;` and none of the
    keywords `let`, `if`, `match`, `for`, `while`, `loop`, `return`; every
    other method is **non-forward** (it holds logic).

`index` prints the totals and one row per module (`session/<dir>` or
`handlers/<dir>`, else the file) with non-forward and forward counts and lines.

`check` reads `session-method-index.json` and fails (exit 1) when, inside the
done scope (`crates/wow-world/src/session/**`):

  * the non-forward methods in the allow-listed runtime-shell and
    construction-API files (the "staying" set) exceed `staying_bound`;
  * the non-forward methods outside the allow-list exceed `outside_ceiling`
    (a ratchet: tighten it when a slice moves logic out);
  * with `--done` (the #1363 done criterion): any non-forward method remains
    outside the allow-list.
"""
from __future__ import annotations

import argparse
import collections
import json
import re
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from hotspot_metrics import (  # noqa: E402
    file_is_entirely_cfg_test,
    mask_rust_noncode,
    top_level_cfg_test_line_indexes,
)

REPO = Path(__file__).resolve().parents[2]
SRC = Path("crates/wow-world/src")
POLICY = Path("tools/architecture/session-method-index.json")
DONE_SCOPE = "session/"

IMPL_RE = re.compile(r"(?m)^[ \t]*impl[ \t]+WorldSession[ \t]*\{")
FN_RE = re.compile(r"(?m)^[ \t]+(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+(\w+)")
LOGIC_RE = re.compile(r"\b(let|if|match|for|while|loop|return)\b")


def _match_brace(masked: str, open_index: int) -> int:
    depth = 0
    for index in range(open_index, len(masked)):
        char = masked[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return index
    raise ValueError("unbalanced braces")


def _attribute_start(lines: list[str], line_index: int) -> int:
    """First line of the doc comments and attributes directly above `line_index`."""
    start = line_index
    while start > 0 and lines[start - 1].strip().startswith(("///", "#[", "//")):
        start -= 1
    return start


def file_methods(source: str, rel: str) -> list[dict]:
    """Production inherent `WorldSession` methods of one file."""
    if file_is_entirely_cfg_test(source):
        return []
    masked = mask_rust_noncode(source).decode()
    lines = source.splitlines()
    test_lines = top_level_cfg_test_line_indexes(source)
    methods = []
    for impl in IMPL_RE.finditer(masked):
        impl_line = masked.count("\n", 0, impl.start())
        if impl_line in test_lines:
            continue
        impl_attrs = lines[_attribute_start(lines, impl_line):impl_line]
        if any(line.strip() == "#[cfg(test)]" for line in impl_attrs):
            continue
        open_index = masked.index("{", impl.start())
        close_index = _match_brace(masked, open_index)
        block = masked[open_index + 1:close_index]
        depth_at = _depth_map(block)
        for fn in FN_RE.finditer(block):
            if depth_at[fn.start()] != 0:
                continue
            fn_abs = open_index + 1 + fn.start()
            fn_line = masked.count("\n", 0, fn_abs)
            start_line = _attribute_start(lines, fn_line)
            attrs = lines[start_line:fn_line]
            if any(line.strip() == "#[cfg(test)]" for line in attrs):
                continue
            body_open = _first_body_brace(masked, fn_abs)
            body_close = _match_brace(masked, body_open)
            end_line = masked.count("\n", 0, body_close)
            body = masked[body_open + 1:body_close]
            forward = not LOGIC_RE.search(body) and body.count(";") <= 1
            methods.append(
                {
                    "name": fn.group(1),
                    "file": rel,
                    "start": start_line + 1,
                    "end": end_line + 1,
                    "lines": end_line - start_line + 1,
                    "forward": forward,
                }
            )
    return methods


def _depth_map(block: str) -> list[int]:
    depth = 0
    out = []
    for char in block:
        if char == "}":
            depth -= 1
        out.append(depth)
        if char == "{":
            depth += 1
    return out


def _first_body_brace(masked: str, fn_index: int) -> int:
    """The `{` that opens a fn body: the first one outside the signature's parens/brackets."""
    depth = 0
    for index in range(fn_index, len(masked)):
        char = masked[index]
        if char in "([":
            depth += 1
        elif char in ")]":
            depth -= 1
        elif char == "{" and depth == 0:
            return index
    raise ValueError("fn without body")


def repo_methods(repo: Path) -> list[dict]:
    root = repo / SRC
    methods = []
    for path in sorted(root.rglob("*.rs")):
        rel = path.relative_to(root).as_posix()
        if "shim" in path.name:
            continue
        methods.extend(file_methods(path.read_text(encoding="utf-8"), rel))
    return methods


def module_key(rel: str) -> str:
    parts = rel.split("/")
    if parts[0] in ("session", "handlers") and len(parts) > 2:
        return "/".join(parts[:2])
    return rel.removesuffix(".rs")


def summarize(methods: list[dict]) -> dict:
    total = {"non_forward": 0, "non_forward_lines": 0, "forward": 0, "forward_lines": 0}
    modules: dict[str, dict] = collections.defaultdict(lambda: dict.fromkeys(total, 0))
    for method in methods:
        kind = "forward" if method["forward"] else "non_forward"
        for bucket in (total, modules[module_key(method["file"])]):
            bucket[kind] += 1
            bucket[kind + "_lines"] += method["lines"]
    return {"total": total, "modules": dict(modules)}


def allowed(rel: str, name: str, allow: dict) -> str | None:
    for group, entries in allow.items():
        for entry in entries:
            if "::" in entry:
                path, method = entry.split("::", 1)
                if rel == path and name == method:
                    return group
            elif rel == entry or (entry.endswith("/") and rel.startswith(entry)):
                return group
    return None


def evaluate(methods: list[dict], policy: dict, done: bool) -> tuple[list[str], dict]:
    staying = {"methods": 0, "lines": 0}
    outside = {"methods": 0, "lines": 0}
    outside_rows = []
    for method in methods:
        if method["forward"] or not method["file"].startswith(DONE_SCOPE):
            continue
        if allowed(method["file"], method["name"], policy["allow"]):
            staying["methods"] += 1
            staying["lines"] += method["lines"]
        else:
            outside["methods"] += 1
            outside["lines"] += method["lines"]
            outside_rows.append(method)
    errors = []
    for label, measured, bound in (
        ("staying", staying, policy["staying_bound"]),
        ("outside", outside, policy["outside_ceiling"]),
    ):
        for key in ("methods", "lines"):
            if measured[key] > bound[key]:
                errors.append(f"{label} non-forward {key} {measured[key]} > bound {bound[key]}")
    if done and outside["methods"]:
        errors.append(f"done criterion: {outside['methods']} non-forward methods outside the allow-list")
        errors.extend(f"  {m['file']}:{m['start']} {m['name']} ({m['lines']})" for m in outside_rows)
    return errors, {"staying": staying, "outside": outside}


def print_index(methods: list[dict], list_methods: bool) -> None:
    summary = summarize(methods)
    total = summary["total"]
    print(
        f"total non-forward {total['non_forward']} ({total['non_forward_lines']} lines), "
        f"forward {total['forward']} ({total['forward_lines']} lines)"
    )
    for key, row in sorted(summary["modules"].items(), key=lambda kv: (-kv[1]["non_forward_lines"], kv[0])):
        print(
            f"{key:48} non-forward {row['non_forward']:4} {row['non_forward_lines']:6}"
            f"   forward {row['forward']:4} {row['forward_lines']:5}"
        )
    if list_methods:
        for method in methods:
            kind = "F" if method["forward"] else "N"
            print(f"{kind} {method['file']}:{method['start']} {method['name']} {method['lines']}")


def self_test() -> int:
    source = '''
impl WorldSession {
    /// doc
    pub fn forward_one(&self) -> u32 {
        self.core.value()
    }

    pub(crate) async fn logic(&mut self) {
        let x = self.a();
        self.b(x);
    }

    #[cfg(test)]
    fn test_only(&self) { let _ = 1; }

    fn with_closure(&self) -> u32 {
        self.items().map(|item| { item.count() }).sum()
    }

    fn string_has_let(&self) -> &str {
        "let if match;;"
    }
}

#[cfg(test)]
impl WorldSession {
    fn cfg_impl(&self) { let _ = 1; }
}

impl Other for WorldSession {
    fn trait_method(&self) { let _ = 1; }
}

#[cfg(test)]
mod tests {
    impl WorldSession {
        fn nested(&self) { let _ = 1; }
    }
}
'''
    methods = file_methods(source, "session/x/a.rs")
    got = [(m["name"], m["forward"], m["lines"]) for m in methods]
    expected = [
        ("forward_one", True, 4),
        ("logic", False, 4),
        ("with_closure", True, 3),
        ("string_has_let", True, 3),
    ]
    assert got == expected, got
    assert file_methods("#![cfg(test)]\n" + source, "session/x/a.rs") == []
    policy = {
        "allow": {"runtime_shell": ["session/driver/"], "construction_api": ["session/z.rs::set_store"]},
        "staying_bound": {"methods": 2, "lines": 10},
        "outside_ceiling": {"methods": 1, "lines": 4},
    }
    rows = [
        {"name": "run", "file": "session/driver/a.rs", "lines": 5, "forward": False, "start": 1},
        {"name": "set_store", "file": "session/z.rs", "lines": 5, "forward": False, "start": 1},
        {"name": "other", "file": "session/z.rs", "lines": 4, "forward": False, "start": 9},
        {"name": "fwd", "file": "session/y.rs", "lines": 50, "forward": True, "start": 1},
        {"name": "outside_scope", "file": "handlers/y.rs", "lines": 50, "forward": False, "start": 1},
    ]
    errors, counts = evaluate(rows, policy, done=False)
    assert errors == [] and counts == {"staying": {"methods": 2, "lines": 10}, "outside": {"methods": 1, "lines": 4}}, (errors, counts)
    errors, _ = evaluate(rows, policy, done=True)
    assert errors and errors[0].startswith("done criterion: 1"), errors
    errors, _ = evaluate(rows + [dict(rows[0], name="run2")], policy, done=False)
    assert errors == ["staying non-forward methods 3 > bound 2", "staying non-forward lines 15 > bound 10"], errors
    errors, _ = evaluate(rows + [dict(rows[2], name="o2")], policy, done=False)
    assert errors == ["outside non-forward methods 2 > bound 1", "outside non-forward lines 8 > bound 4"], errors
    assert allowed("session/driverx/a.rs", "run", policy["allow"]) is None
    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp)
        (repo / SRC / "session").mkdir(parents=True)
        (repo / SRC / "session" / "a.rs").write_text(source)
        (repo / SRC / "session" / "a_test_shims.rs").write_text(source)
        assert len(repo_methods(repo)) == 4
    print("session_method_index self-test: PASS")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    index = sub.add_parser("index")
    index.add_argument("--repo", type=Path, default=REPO)
    index.add_argument("--methods", action="store_true")
    index.add_argument("--json", type=Path)
    check = sub.add_parser("check")
    check.add_argument("--repo", type=Path, default=REPO)
    check.add_argument("--policy", type=Path)
    check.add_argument("--done", action="store_true")
    sub.add_parser("self-test")
    args = parser.parse_args()
    if args.command == "self-test":
        return self_test()
    methods = repo_methods(args.repo)
    if args.command == "index":
        print_index(methods, args.methods)
        if args.json:
            args.json.write_text(json.dumps(methods, indent=1) + "\n", encoding="utf-8")
        return 0
    policy = json.loads((args.policy or args.repo / POLICY).read_text(encoding="utf-8"))
    errors, counts = evaluate(methods, policy, args.done)
    print(
        f"session_method_index: staying {counts['staying']['methods']} / {counts['staying']['lines']} "
        f"(bound {policy['staying_bound']['methods']} / {policy['staying_bound']['lines']}), "
        f"outside {counts['outside']['methods']} / {counts['outside']['lines']} "
        f"(ceiling {policy['outside_ceiling']['methods']} / {policy['outside_ceiling']['lines']})"
    )
    for key in ("methods", "lines"):
        if counts["outside"][key] < policy["outside_ceiling"][key]:
            print(f"note: outside_ceiling.{key} can be tightened to {counts['outside'][key]}")
    for error in errors:
        print(error, file=sys.stderr)
    print("session_method_index check:", "FAIL" if errors else "PASS")
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
