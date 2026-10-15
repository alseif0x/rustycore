#!/usr/bin/env python3
"""Index of the production inherent `WorldSession` methods (#1363 done criterion).

Usage:
  python3 tools/architecture/session_method_index.py index
      [--repo PATH] [--methods] [--json PATH]
  python3 tools/architecture/session_method_index.py check
      [--repo PATH] [--policy PATH] [--done]
  python3 tools/architecture/session_method_index.py self-test

What it counts (the slice 12b1 index, made reproducible; #1413 review
follow-ups F1-F4):

  * every inherent `WorldSession` impl in the production modules of
    `crates/wow-world/src`, also path-qualified (`impl super::WorldSession`,
    `impl crate::session::WorldSession`) or generic (`impl<'a> WorldSession`);
    trait impls (`impl Trait for WorldSession`) are not inherent methods;
  * production is decided by cfg, never by file name: the walk starts at
    `lib.rs` and follows `mod x;` (with `#[path]` and inline-module nesting);
    a module, impl, method or nested item whose `cfg(...)` is off in a build
    without `test` and without the dev-only `test-fixtures` feature is
    excluded (an inner `#![cfg(..)]` gates its whole file or block); any other
    cfg counts as possibly on;
  * comments and literals are blanked before parsing (hotspot_metrics);
  * a method's lines run from its first doc or attribute line through its
    closing brace;
  * a **forward** is a method whose body has at most one `;`, none of the
    keywords `let`, `if`, `match`, `for`, `while`, `loop`, `return`, no
    closure, no `async`/`unsafe` block and at most one `?`; every other method
    is **non-forward** (it holds logic).

`index` prints the totals and one row per module (`session/<dir>` or
`handlers/<dir>`, else the file) with non-forward and forward counts and lines.

`check` reads `session-method-index.json` and fails (exit 1) when, inside the
done scope (`crates/wow-world/src/session/**`):

  * the non-forward methods in the allow-listed runtime-shell and
    construction-API files (the "staying" set) exceed `staying_bound`;
  * the non-forward methods outside the allow-list exceed `outside_ceiling`
    (a ratchet: tighten it when a slice moves logic out);
  * the non-forward methods of the whole crate exceed `crate_ceiling`, so logic
    cannot leave `session/**` for `handlers/` or another module unseen;
  * an allow-list entry matches no non-forward method, or a source under
    `src` is not reached from `lib.rs`;
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

from hotspot_metrics import mask_rust_noncode  # noqa: E402

REPO = Path(__file__).resolve().parents[2]
SRC = Path("crates/wow-world/src")
POLICY = Path("tools/architecture/session-method-index.json")
DONE_SCOPE = "session/"

# An inherent impl of `WorldSession`, also path-qualified (`impl super::WorldSession`,
# `impl crate::session::WorldSession`) and generic (`impl<'a> WorldSession<'a>`);
# never `impl Trait for WorldSession` (the trait name is not followed by `::`).
IMPL_RE = re.compile(
    rb"(?m)^[ \t]*impl(?:[ \t]*<[^>{;]*>)?\s+(?:\w+\s*::\s*)*WorldSession\b"
    rb"(?:\s*<[^>{;]*>)?\s*(?:where\b[^{;]*)?\{"
)
FN_RE = re.compile(
    rb"(?m)^[ \t]+(?:pub(?:\s*\([^)]*\))?\s+)?(?:default\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+(\w+)"
)
LOGIC_RE = re.compile(rb"\b(let|if|match|for|while|loop|return)\b")
BLOCK_EXPR_RE = re.compile(rb"\b(?:async(?:\s+move)?|unsafe)\s*\{")
CFG_ATTR_RE = re.compile(rb"#\s*(!)?\s*\[\s*cfg\s*\(")
MOD_RE = re.compile(rb"(?m)^[ \t]*(?:pub(?:\s*\([^)]*\))?\s+)?mod\s+(?:r#)?(\w+)\s*([;{])")
PATH_ATTR_RE = re.compile(r'#\s*\[\s*path\s*=\s*"([^"]+)"\s*\]')
# A closure's opening `|` follows an argument/assignment position or `move`;
# a boolean `||` follows an operand.
CLOSURE_LEAD = b"(,={:"

# The production configuration the done criterion measures: no `test`, and no
# `test-fixtures` (enabled only through dev-dependencies). Any other cfg is
# unknown and treated as possibly on, so it never hides production code.
OFF_FEATURES = {"test-fixtures"}


def _cfg_tokens(text: str) -> list[str]:
    return re.findall(r'"(?:[^"\\]|\\.)*"|\w+|[(),=]', text)


def _cfg_parse(tokens: list[str], index: int) -> tuple[bool | None, int]:
    name = tokens[index]
    index += 1
    if index < len(tokens) and tokens[index] == "=":
        value = tokens[index + 1].strip('"')
        index += 2
        if name == "feature" and value in OFF_FEATURES:
            return False, index
        return None, index
    if index < len(tokens) and tokens[index] == "(":
        index += 1
        values = []
        while tokens[index] != ")":
            value, index = _cfg_parse(tokens, index)
            values.append(value)
            if tokens[index] == ",":
                index += 1
        index += 1
        if name == "not":
            return (None if values[0] is None else not values[0]), index
        if name == "any":
            if True in values:
                return True, index
            return (False if all(v is False for v in values) else None), index
        if name == "all":
            if False in values:
                return False, index
            return (True if all(v is True for v in values) else None), index
        return None, index
    if name == "test":
        return False, index
    return None, index


def cfg_is_production(predicate: str) -> bool:
    """Whether a `cfg(...)` predicate can hold in a production build (Kleene logic)."""
    try:
        value, _ = _cfg_parse(_cfg_tokens(predicate), 0)
    except (IndexError, ValueError):
        return True
    return value is not False


def _matching(masked: bytes, open_index: int) -> int:
    pairs = {ord("{"): ord("}"), ord("("): ord(")"), ord("["): ord("]")}
    opening = masked[open_index]
    closing = pairs[opening]
    depth = 0
    for index in range(open_index, len(masked)):
        if masked[index] == opening:
            depth += 1
        elif masked[index] == closing:
            depth -= 1
            if depth == 0:
                return index
    raise ValueError("unbalanced delimiters")


def _item_end(masked: bytes, cursor: int) -> int:
    """End of the item or element an outer attribute ending before `cursor` is on."""
    while True:
        while cursor < len(masked) and chr(masked[cursor]).isspace():
            cursor += 1
        if masked[cursor:cursor + 1] == b"#":
            bracket = cursor + 1
            while chr(masked[bracket]).isspace():
                bracket += 1
            if masked[bracket:bracket + 1] == b"[":
                cursor = _matching(masked, bracket) + 1
                continue
        break
    depth = 0
    while cursor < len(masked):
        char = masked[cursor]
        if char in b"([":
            depth += 1
        elif char in b")]}":
            if depth == 0:
                return cursor
            depth -= 1
        elif char == ord("{"):
            if depth == 0:
                return _matching(masked, cursor) + 1
            depth += 1
        elif char in b";," and depth == 0:
            return cursor + 1
        cursor += 1
    return cursor


def _enclosing_block(masked: bytes, index: int) -> tuple[int, int]:
    depth = 0
    for cursor in range(index - 1, -1, -1):
        char = masked[cursor]
        if char == ord("}"):
            depth += 1
        elif char == ord("{"):
            if depth == 0:
                return cursor, _matching(masked, cursor) + 1
            depth -= 1
    return 0, len(masked)


def nonproduction_spans(source: str) -> list[tuple[int, int]]:
    """Byte spans of items gated by a cfg that is off in production, at any depth.

    An outer `#[cfg(..)]` covers its item (or field, arm, statement); an inner
    `#![cfg(..)]` covers its enclosing block, or the whole file at top level.
    """
    original = source.encode("utf-8")
    masked = mask_rust_noncode(source)
    spans = []
    for attr in CFG_ATTR_RE.finditer(masked):
        bracket = masked.index(b"[", attr.start())
        close = _matching(masked, bracket)
        predicate = original[bracket + 1:close].decode("utf-8").strip()
        predicate = predicate[predicate.index("(") + 1:predicate.rindex(")")]
        if cfg_is_production(predicate):
            continue
        if attr.group(1):
            spans.append(_enclosing_block(masked, attr.start()))
        else:
            spans.append((attr.start(), _item_end(masked, close + 1)))
    return spans


def _inside(spans: list[tuple[int, int]], index: int) -> bool:
    return any(start <= index < end for start, end in spans)


def _attribute_start(lines: list[str], line_index: int) -> int:
    """First line of the doc comments and attributes directly above `line_index`."""
    start = line_index
    while start > 0 and lines[start - 1].strip().startswith(("///", "#[", "//")):
        start -= 1
    return start


def closure_count(body: bytes) -> int:
    count = 0
    for index, char in enumerate(body):
        if char != ord("|") or (index and body[index - 1] == ord("|")):
            continue
        back = index - 1
        while back >= 0 and chr(body[back]).isspace():
            back -= 1
        if back < 0 or body[back] in CLOSURE_LEAD or body[max(0, back - 3):back + 1] == b"move":
            count += 1
    return count


def is_forward(body: bytes) -> bool:
    """At most one statement, no control flow, no closure, block expression or `?` chain."""
    return (
        not LOGIC_RE.search(body)
        and body.count(b";") <= 1
        and not BLOCK_EXPR_RE.search(body)
        and closure_count(body) == 0
        and body.count(b"?") <= 1
    )


def file_methods(source: str, rel: str) -> list[dict]:
    """Production inherent `WorldSession` methods of one file (file-level cfg aside)."""
    masked = mask_rust_noncode(source)
    lines = source.splitlines()
    gated = nonproduction_spans(source)
    methods = []
    for impl in IMPL_RE.finditer(masked):
        open_index = impl.end() - 1
        close_index = _matching(masked, open_index)
        block = masked[open_index + 1:close_index]
        depth_at = _depth_map(block)
        for fn in FN_RE.finditer(block):
            if depth_at[fn.start()] != 0:
                continue
            fn_abs = open_index + 1 + fn.start()
            # A gated impl, module or method covers the method's `fn` token.
            if _inside(gated, fn_abs + len(fn.group(0)) - len(fn.group(0).lstrip())):
                continue
            fn_line = masked.count(b"\n", 0, fn_abs)
            start_line = _attribute_start(lines, fn_line)
            body_open = _first_body_brace(masked, fn_abs)
            body_close = _matching(masked, body_open)
            end_line = masked.count(b"\n", 0, body_close)
            methods.append(
                {
                    "name": fn.group(1).decode(),
                    "file": rel,
                    "start": start_line + 1,
                    "end": end_line + 1,
                    "lines": end_line - start_line + 1,
                    "forward": is_forward(masked[body_open + 1:body_close]),
                }
            )
    return methods


def _depth_map(block: bytes) -> list[int]:
    depth = 0
    out = []
    for char in block:
        if char == ord("}"):
            depth -= 1
        out.append(depth)
        if char == ord("{"):
            depth += 1
    return out


def _first_body_brace(masked: bytes, fn_index: int) -> int:
    """The `{` that opens a fn body: the first one outside the signature's parens/brackets."""
    depth = 0
    for index in range(fn_index, len(masked)):
        char = masked[index]
        if char in b"([":
            depth += 1
        elif char in b")]":
            depth -= 1
        elif char == ord("{") and depth == 0:
            return index
    raise ValueError("fn without body")


def module_files(root: Path) -> dict[Path, bool]:
    """Every file the crate mounts from `lib.rs`, and whether any mount chain is production.

    Follows `mod x;` (with `#[path]` and inline `mod m { mod x; }` nesting) and
    resolves each declaration's cfg with `nonproduction_spans`, so a module is
    excluded because it is gated off in production, never because of its name.
    """
    seen: dict[Path, bool] = {}
    pending = [(root / "lib.rs", True, True)]
    while pending:
        path, production, mod_rs = pending.pop()
        if path in seen and (seen[path] or not production):
            continue
        seen[path] = seen.get(path, False) or production
        source = path.read_text(encoding="utf-8")
        masked = mask_rust_noncode(source)
        lines = source.splitlines()
        gated = nonproduction_spans(source)
        inline = [
            (m.group(1).decode(), m.end() - 1, _matching(masked, m.end() - 1))
            for m in MOD_RE.finditer(masked)
            if m.group(2) == b"{"
        ]
        base = path.parent if mod_rs else path.parent / path.stem
        for decl in MOD_RE.finditer(masked):
            if decl.group(2) != b";":
                continue
            name = decl.group(1).decode()
            nesting = [n for n, start, end in sorted(inline, key=lambda r: r[1]) if start < decl.start() < end]
            decl_line = masked.count(b"\n", 0, decl.start() + len(decl.group(0)) - len(decl.group(0).lstrip()))
            attrs = "\n".join(lines[_attribute_start(lines, decl_line):decl_line])
            path_attr = PATH_ATTR_RE.search(attrs)
            if path_attr:
                parent = (path.parent if not nesting or mod_rs else path.parent / path.stem).joinpath(*nesting)
                child, child_mod_rs = (parent / path_attr.group(1)).resolve(), True
            else:
                parent = base.joinpath(*nesting)
                child, child_mod_rs = parent / f"{name}.rs", False
                if not child.exists():
                    child, child_mod_rs = parent / name / "mod.rs", True
            if not child.exists() or not child.resolve().is_relative_to(root.resolve()):
                continue
            child_production = production and not _inside(gated, decl.start() + len(decl.group(0)) - len(decl.group(0).lstrip()))
            pending.append((child.resolve(), child_production, child_mod_rs))
    return seen


def repo_methods(repo: Path, unreached: list[str] | None = None) -> list[dict]:
    root = (repo / SRC).resolve()
    mounted = module_files(root)
    methods = []
    for path in sorted(root.rglob("*.rs")):
        rel = path.relative_to(root).as_posix()
        if path.resolve() not in mounted:
            if unreached is not None:
                unreached.append(rel)
            continue
        if not mounted[path.resolve()]:
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


def stale_entries(methods: list[dict], allow: dict) -> list[str]:
    """Allow-list entries that match no non-forward method (they must be removed)."""
    used = set()
    for method in methods:
        if method["forward"]:
            continue
        for entries in allow.values():
            for entry in entries:
                if allowed(method["file"], method["name"], {"g": [entry]}):
                    used.add(entry)
    return [entry for entries in allow.values() for entry in entries if entry not in used]


def evaluate(methods: list[dict], policy: dict, done: bool) -> tuple[list[str], dict]:
    staying = {"methods": 0, "lines": 0}
    outside = {"methods": 0, "lines": 0}
    crate = {"methods": 0, "lines": 0}
    outside_rows = []
    for method in methods:
        if method["forward"]:
            continue
        crate["methods"] += 1
        crate["lines"] += method["lines"]
        if not method["file"].startswith(DONE_SCOPE):
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
        ("crate", crate, policy["crate_ceiling"]),
    ):
        for key in ("methods", "lines"):
            if measured[key] > bound[key]:
                errors.append(f"{label} non-forward {key} {measured[key]} > bound {bound[key]}")
    errors.extend(
        f"allow-list entry matches no non-forward method: {entry}"
        for entry in stale_entries(methods, policy["allow"])
    )
    if done and outside["methods"]:
        errors.append(f"done criterion: {outside['methods']} non-forward methods outside the allow-list")
        errors.extend(f"  {m['file']}:{m['start']} {m['name']} ({m['lines']})" for m in outside_rows)
    return errors, {"staying": staying, "outside": outside, "crate": crate}


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


SELF_TEST_SOURCE = """
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

    #[cfg(any(test, feature = "test-fixtures"))]
    fn fixture_only(&self) { let _ = 1; }

    #[cfg(feature = "test-fixtures")]
    fn feature_only(&self) { let _ = 1; }

    #[cfg(not(test))]
    fn production_only(&self) { let _ = 1; }

    fn with_closure(&self) -> u32 {
        self.items().map(|item| { item.count() }).sum()
    }

    fn closure_chain(&self) -> Option<u32> {
        self.a().and_then(|a| a.b()).map(move |b| b + 1)
    }

    fn async_block(&self) -> impl Future<Output = ()> {
        async move { self.a().await }
    }

    fn try_chain(&self) -> Option<u32> {
        Some(self.a()?.b()?)
    }

    fn boolean_or(&self) -> bool {
        self.a() || self.b()
    }

    fn string_has_let(&self) -> &str {
        "let if match;; |x| async {"
    }
}

impl super::WorldSession {
    fn qualified_super(&self) { let _ = 1; }
}

impl super::super::WorldSession {
    fn qualified_super_super(&self) { let _ = 1; }
}

impl crate::session::WorldSession {
    fn qualified_crate(&self) { let _ = 1; }
}

impl<'a> WorldSession
{
    fn generic(&self) { let _ = 1; }
}

#[cfg(test)]
impl WorldSession {
    fn cfg_impl(&self) { let _ = 1; }
}

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::WorldSession {
    fn fixture_impl(&self) { let _ = 1; }
}

impl Other for WorldSession {
    fn trait_method(&self) { let _ = 1; }
}

impl crate::Other for WorldSession {
    fn qualified_trait_method(&self) { let _ = 1; }
}

impl WorldSessionExt {
    fn other_type(&self) { let _ = 1; }
}

#[cfg(test)]
mod tests {
    impl WorldSession {
        fn nested(&self) { let _ = 1; }
    }
}

mod outer {
    #[cfg(test)]
    mod inner {
        impl super::super::WorldSession {
            fn deep_nested(&self) { let _ = 1; }
        }
    }
}
"""

SELF_TEST_EXPECTED = [
    ("forward_one", True, 4),
    ("logic", False, 4),
    ("production_only", False, 2),
    ("with_closure", False, 3),
    ("closure_chain", False, 3),
    ("async_block", False, 3),
    ("try_chain", False, 3),
    ("boolean_or", True, 3),
    ("string_has_let", True, 3),
    ("qualified_super", False, 1),
    ("qualified_super_super", False, 1),
    ("qualified_crate", False, 1),
    ("generic", False, 1),
]


def _self_test_tree(repo: Path) -> None:
    """A crate whose module gates decide production, whatever the file names."""
    src = repo / SRC
    method = "impl super::WorldSession {\n    fn {name}(&self) {{ let _ = 1; }}\n}\n"
    files = {
        "lib.rs": (
            "mod session;\n"
            "#[cfg(any(test, feature = \"test-fixtures\"))]\nmod fixtures;\n"
            "#[cfg(test)]\nmod gated_test_shims;\n"
            "mod plain_shims;\n"
            "#[cfg(test)]\nmod tests {\n    mod nested_in_test;\n}\n"
            "mod inline {\n    mod nested_inline;\n}\n"
        ),
        "session/mod.rs": (
            "// mod commented_out;\n"
            "mod a;\n"
            "#[path = \"elsewhere/b.rs\"]\nmod b;\n"
            "#[cfg(not(test))]\nmod c;\n"
        ),
        "session/a.rs": "#![cfg(test)]\n" + method.replace("{name}", "inner_attr_gated"),
        "session/elsewhere/b.rs": method.replace("{name}", "path_mounted"),
        "session/c.rs": method.replace("{name}", "not_test_gated"),
        "session/commented_out.rs": method.replace("{name}", "unmounted"),
        "fixtures.rs": method.replace("{name}", "fixture_module"),
        "gated_test_shims.rs": method.replace("{name}", "gated_shim"),
        "plain_shims.rs": method.replace("{name}", "ungated_shim"),
        "tests/nested_in_test.rs": method.replace("{name}", "nested_in_test"),
        "inline/nested_inline.rs": method.replace("{name}", "nested_inline"),
    }
    for rel, text in files.items():
        (src / rel).parent.mkdir(parents=True, exist_ok=True)
        (src / rel).write_text(text)


def self_test() -> int:
    methods = file_methods(SELF_TEST_SOURCE, "session/x/a.rs")
    got = [(m["name"], m["forward"], m["lines"]) for m in methods]
    assert got == SELF_TEST_EXPECTED, got
    assert file_methods("#![cfg(test)]\n" + SELF_TEST_SOURCE, "session/x/a.rs") == []
    for predicate, production in (
        ("test", False),
        ('any(test, feature = "test-fixtures")', False),
        ('feature = "test-fixtures"', False),
        ("all(test, unix)", False),
        ("not(test)", True),
        ("unix", True),
        ('any(test, feature = "other")', True),
        ("not(unix)", True),
    ):
        assert cfg_is_production(predicate) is production, predicate
    policy = {
        "allow": {"runtime_shell": ["session/driver/"], "construction_api": ["session/z.rs::set_store"]},
        "staying_bound": {"methods": 2, "lines": 10},
        "outside_ceiling": {"methods": 1, "lines": 4},
        "crate_ceiling": {"methods": 4, "lines": 64},
    }
    rows = [
        {"name": "run", "file": "session/driver/a.rs", "lines": 5, "forward": False, "start": 1},
        {"name": "set_store", "file": "session/z.rs", "lines": 5, "forward": False, "start": 1},
        {"name": "other", "file": "session/z.rs", "lines": 4, "forward": False, "start": 9},
        {"name": "fwd", "file": "session/y.rs", "lines": 50, "forward": True, "start": 1},
        {"name": "outside_scope", "file": "handlers/y.rs", "lines": 50, "forward": False, "start": 1},
    ]
    errors, counts = evaluate(rows, policy, done=False)
    assert errors == [] and counts == {
        "staying": {"methods": 2, "lines": 10},
        "outside": {"methods": 1, "lines": 4},
        "crate": {"methods": 4, "lines": 64},
    }, (errors, counts)
    errors, _ = evaluate(rows, policy, done=True)
    assert errors and errors[0].startswith("done criterion: 1"), errors
    errors, _ = evaluate(rows + [dict(rows[0], name="run2")], policy, done=False)
    assert errors == [
        "staying non-forward methods 3 > bound 2",
        "staying non-forward lines 15 > bound 10",
        "crate non-forward methods 5 > bound 4",
        "crate non-forward lines 69 > bound 64",
    ], errors
    errors, _ = evaluate(rows + [dict(rows[2], name="o2")], policy, done=False)
    assert errors == [
        "outside non-forward methods 2 > bound 1",
        "outside non-forward lines 8 > bound 4",
        "crate non-forward methods 5 > bound 4",
        "crate non-forward lines 68 > bound 64",
    ], errors
    # Logic escaping session/** into handlers/ is caught by the crate ceiling.
    errors, _ = evaluate(rows + [dict(rows[4], name="escaped", lines=1)], policy, done=True)
    assert errors == [
        "crate non-forward methods 5 > bound 4",
        "crate non-forward lines 65 > bound 64",
        "done criterion: 1 non-forward methods outside the allow-list",
        "  session/z.rs:9 other (4)",
    ], errors
    stale = dict(policy, allow={"runtime_shell": ["session/driver/", "session/gone.rs::old"],
                                "construction_api": ["session/z.rs::set_store"]})
    errors, _ = evaluate(rows, stale, done=False)
    assert errors == ["allow-list entry matches no non-forward method: session/gone.rs::old"], errors
    assert allowed("session/driverx/a.rs", "run", policy["allow"]) is None
    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp)
        _self_test_tree(repo)
        unreached: list[str] = []
        names = sorted(m["name"] for m in repo_methods(repo, unreached))
        assert names == ["nested_inline", "not_test_gated", "path_mounted", "ungated_shim"], names
        assert unreached == ["session/commented_out.rs"], unreached
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
    unreached: list[str] = []
    methods = repo_methods(args.repo, unreached)
    if args.command == "index":
        print_index(methods, args.methods)
        if args.json:
            args.json.write_text(json.dumps(methods, indent=1) + "\n", encoding="utf-8")
        return 0
    policy = json.loads((args.policy or args.repo / POLICY).read_text(encoding="utf-8"))
    errors, counts = evaluate(methods, policy, args.done)
    # A source the module walk cannot reach is either dead or mounted in a way
    # the walk does not follow; both must be resolved, not silently skipped.
    errors.extend(f"source not reached from lib.rs: {rel}" for rel in unreached)
    print(
        "session_method_index: "
        + ", ".join(
            f"{label} {counts[label]['methods']} / {counts[label]['lines']} "
            f"(bound {policy[key]['methods']} / {policy[key]['lines']})"
            for label, key in (
                ("staying", "staying_bound"),
                ("outside", "outside_ceiling"),
                ("crate", "crate_ceiling"),
            )
        )
    )
    for label, key in (("outside", "outside_ceiling"), ("crate", "crate_ceiling")):
        for unit in ("methods", "lines"):
            if counts[label][unit] < policy[key][unit]:
                print(f"note: {key}.{unit} can be tightened to {counts[label][unit]}")
    for error in errors:
        print(error, file=sys.stderr)
    print("session_method_index check:", "FAIL" if errors else "PASS")
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
