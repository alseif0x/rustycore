"""Rust source scanning for the #1442 wow-world test census (no Cargo, no rustc).

Comments and literals are blanked with offsets kept (`mask`), braces are paired
once per file, and a file is parsed into its functions (with their attributes,
`impl` owner, `cfg(test)` gating and body identifiers), test-side types, `use`
imports and `mod` mounts. `wow_world_test_census.py` owns the classification.
"""
from __future__ import annotations

import re
import sys
from dataclasses import dataclass, field
from functools import lru_cache
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from session_method_index import cfg_is_production  # noqa: E402


# Too generic to be evidence of a session call or a domain owner.
IGNORED_NAMES = frozenset({"new", "default", "clone", "from", "into", "fmt", "drop", "eq", "hash"})

KEYWORDS = frozenset(
    "as async await break const continue crate dyn else enum extern false fn for if impl in let loop "
    "match mod move mut pub ref return self Self static struct super trait true type unsafe use where "
    "while union macro_rules".split()
)
PREVIOUS_SEGMENT_RE = re.compile(r"([A-Za-z_]\w*)\s*::\s*\Z")
IDENT_RE = re.compile(r"\b[A-Za-z_][A-Za-z0-9_]*\b")
FN_RE = re.compile(r"\bfn\s+(?:r#)?([A-Za-z_]\w*)")
DEF_RE = re.compile(r"\b(fn|struct|enum|trait|type|const|static|union)\s+(?:r#)?([A-Za-z_]\w*)")
CRATE_PATH_RE = re.compile(r"\b(wow_world(?:_[a-z_]+)?)\s*::")
ROOT_PATH_RE = re.compile(r"(?<![\w:])([a-z][a-z0-9_]*)\s*::")
# A source-text contract test (`include_str!("../wow-world-x/src/...")`) is owned by the crate it reads.
INCLUDE_RE = re.compile(r'include_str!\s*\(\s*"[^"]*?\b(wow-world(?:-[a-z-]+?)?)/src/')
TYPE_RE = re.compile(r"\b(?:struct|enum|union)\s+(?:r#)?([A-Za-z_]\w*)")
# An enum variant name (`Commit {`, `Read(`, `Unknown,`), which names nothing outside the enum.
VARIANT_RE = re.compile(r"(?:\{|,)\s*(?:#\s*\[[^\]]*\]\s*)*([A-Z]\w*)\s*(?=[{(,}=])")
IMPL_RE = re.compile(r"(?m)^[ \t]*(?:unsafe\s+)?impl\b")
MOD_RE = re.compile(r"(?m)^[ \t]*(?:pub(?:\s*\([^)]*\))?\s+)?mod\s+(?:r#)?(\w+)\s*([;{])")
USE_RE = re.compile(r"(?m)^[ \t]*(?:pub(?:\s*\([^)]*\))?\s+)?use\s+([^;]*);")
ATTR_RE = re.compile(r"#\s*!?\s*\[")
# A test attribute is any path ending in `test`: `#[test]`, `#[tokio::test]`,
# `#[::tokio::test]`, `#[core::prelude::v1::test]` (not `#[cfg(test)]` or `#[test_case]`).
TEST_ATTR_PATTERN = r"#\s*\[\s*(?:::\s*)?(?:[A-Za-z_]\w*\s*::\s*)*test\b"
TEST_ATTR_SCAN_RE = re.compile(TEST_ATTR_PATTERN)
TEST_ATTR_RE = re.compile("^" + TEST_ATTR_PATTERN)
IGNORE_ATTR_RE = re.compile(r"^#\s*\[\s*ignore\b")
CFG_ATTR_RE = re.compile(r"^#\s*(!)?\s*\[\s*cfg\s*\(")
PATH_ATTR_RE = re.compile(r'^#\s*\[\s*path\s*=\s*"([^"]+)"\s*\]$')
QUALIFIERS_RE = re.compile(
    r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?(?:default\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?"
    r"(?:extern\s+)?\s*$"
)
LITERAL_RE = re.compile(
    r"(?=[/\"'bcr])(?:(?P<line>//)|(?P<block>/\*)|(?P<raw>\b[bc]?r(?P<hashes>#*)\")|(?P<str>\b[bc]?\"|\")"
    r"|(?P<char>\bb?'(?:[^'\\\n]|\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\}|.))'|'(?:[^'\\\n]|\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\}|.))'))"
)


class CensusError(ValueError):
    """A census, ledger or ceiling violation."""


# --------------------------------------------------------------------------- lexing

@lru_cache(maxsize=None)
def mask(source: str) -> str:
    """Blank comments, string/char literals and non-ASCII text; offsets and newlines are kept."""
    out: list[str] = []
    pos, length = 0, len(source)
    while True:
        match = LITERAL_RE.search(source, pos)
        if not match:
            out.append(source[pos:])
            break
        start = match.start()
        out.append(source[pos:start])
        if match.group("line"):
            end = source.find("\n", start)
            end = length if end < 0 else end
        elif match.group("block"):
            depth, end = 1, start + 2
            while end < length and depth:
                if source.startswith("/*", end):
                    depth, end = depth + 1, end + 2
                elif source.startswith("*/", end):
                    depth, end = depth - 1, end + 2
                else:
                    end += 1
        elif match.group("raw"):
            closing = '"' + match.group("hashes")
            end = source.find(closing, match.end())
            end = length if end < 0 else end + len(closing)
        elif match.group("str"):
            end = match.end()
            while end < length and source[end] != '"':
                end += 2 if source[end] == "\\" else 1
            end = min(end + 1, length)
        else:
            end = match.end()
        out.append(re.sub(r"[^\n]", " ", source[start:end]))
        pos = end
    return re.sub(r"[^\x00-\x7f]", " ", "".join(out))


def brace_pairs(masked: str) -> dict[int, int]:
    pairs: dict[int, int] = {}
    stack: list[int] = []
    for match in re.finditer(r"[{}]", masked):
        if match.group(0) == "{":
            stack.append(match.start())
        elif stack:
            pairs[stack.pop()] = match.start()
    return pairs


def bracket_end(masked: str, start: int) -> int:
    """Index of the `]` closing the `[` at `start`."""
    depth = 0
    for match in re.finditer(r"[\[\]]", masked[start:]):
        depth += 1 if match.group(0) == "[" else -1
        if depth == 0:
            return start + match.start()
    raise CensusError("unbalanced attribute")


def signature_end(masked: str, start: int) -> tuple[int, str]:
    """First `{` or `;` outside parentheses/brackets after `start`."""
    depth = 0
    for match in re.finditer(r"[()\[\]{;]", masked[start:]):
        char = match.group(0)
        if char in "([":
            depth += 1
        elif char in ")]":
            depth -= 1
        elif depth == 0:
            return start + match.start(), char
    return len(masked), ""


def symbol_names(masked: str) -> set[str]:
    """Identifiers in call, path or type position (never plain local variables).

    A member path (`Step::Commit`, `Foo::assoc()`) contributes `Foo`, not the
    member, except `WorldSession::method(..)` calls.
    """
    names = set()
    for match in IDENT_RE.finditer(masked):
        name = match.group(0)
        if name in KEYWORDS or name in IGNORED_NAMES:
            continue
        after = masked[match.end():match.end() + 3].lstrip()[:2]
        before = masked[max(0, match.start() - 3):match.start()].rstrip()[-2:]
        if before == "::":
            # `Type::Variant`/`Type::assoc` names a member of `Type`, not an item of its own.
            segment = PREVIOUS_SEGMENT_RE.search(masked, max(0, match.start() - 80), match.start())
            if segment and segment.group(1)[0].isupper() and segment.group(1) not in ("WorldSession", "Self"):
                continue
        if name[0].isupper() or after[:1] in ("(", "!") or after == "::" or before == "::":
            names.add(name)
    return names


def specific(name: str) -> bool:
    """A type/const or a multi-word function name; one-word lowercase names are std/trait methods."""
    return name[0].isupper() or "_" in name.strip("_")


def crate_name(ident: str) -> str:
    return ident.replace("_", "-")


# --------------------------------------------------------------------------- parsing

@dataclass
class Fn:
    name: str
    file: str
    start: int
    end: int
    line: int
    lines: int
    body: str
    impl_self: str | None
    impl_trait: str | None
    gated: bool
    test: bool
    ignored: bool
    module: tuple[str, ...]
    names: set[str] = field(default_factory=set)
    paths: set[str] = field(default_factory=set)


@dataclass
class Parsed:
    rel: str
    fns: list[Fn]
    uses: dict[str, str]
    mounts: list[tuple[str, str | None, tuple[str, ...], bool]]
    types: list[Fn] = field(default_factory=list)
    gated: list[tuple[int, int]] = field(default_factory=list)


def attribute_runs(source: str, masked: str) -> dict[int, list[tuple[int, str]]]:
    """Map an item start to its outer attributes (start offset, original text)."""
    runs: dict[int, list[tuple[int, str]]] = {}
    pending: list[tuple[int, str]] = []
    cursor = 0
    for match in ATTR_RE.finditer(masked):
        if match.start() < cursor:
            continue
        bracket = masked.index("[", match.start())
        end = bracket_end(masked, bracket) + 1
        text = source[match.start():end]
        gap = masked[cursor:match.start()]
        if pending and gap.strip():
            pending = []
        if "!" not in masked[match.start():bracket]:
            pending.append((match.start(), text))
        cursor = end
        following = len(masked[end:]) - len(masked[end:].lstrip())
        item = end + following
        if not masked.startswith("#", item):
            runs[item] = pending
            pending = []
    return runs


def attrs_for(runs: dict[int, list[tuple[int, str]]], masked: str, fn_start: int) -> list[tuple[int, str]]:
    line_start = masked.rfind("\n", 0, fn_start) + 1
    for item in sorted((s for s in runs if line_start - 400 <= s <= fn_start), reverse=True):
        if QUALIFIERS_RE.match(masked[item:fn_start]):
            return runs[item]
    return []


def cfg_off_spans(source: str, masked: str, runs, pairs: dict[int, int]) -> list[tuple[int, int]]:
    """Spans whose `cfg` is off without `test` and `test-fixtures`."""
    spans = []
    for item, attrs in runs.items():
        for _, text in attrs:
            if CFG_ATTR_RE.match(text) and not cfg_is_production(text[text.index("(") + 1:text.rindex(")")]):
                spans.append((item, item_end(masked, item, pairs)))
    for match in re.finditer(r"#\s*!\s*\[\s*cfg\s*\(", masked):
        text = source[match.start():bracket_end(masked, masked.index("[", match.start())) + 1]
        if cfg_is_production(text[text.index("(") + 1:text.rindex(")")]):
            continue
        opener = max((o for o, c in pairs.items() if o < match.start() < c), default=None)
        spans.append((0, len(masked)) if opener is None else (opener, pairs[opener]))
    return spans


def item_end(masked: str, start: int, pairs: dict[int, int]) -> int:
    """End of the item, statement, field or argument that starts at `start`."""
    depth = 0
    for match in re.finditer(r"[()\[\]{};,]", masked[start:]):
        char, index = match.group(0), start + match.start()
        if char in "([":
            depth += 1
        elif char in ")]}":
            if depth == 0:
                return index
            depth -= 1
        elif char == "{" and depth == 0:
            return pairs.get(index, len(masked) - 1) + 1
        elif char == "{":
            depth += 1
        elif depth == 0:
            return index + 1
    return len(masked)


def inside(spans, index: int) -> bool:
    return any(start <= index < end for start, end in spans)


def parse_uses(masked: str, prefix_crate) -> dict[str, str]:
    """`use` imports from a crate root: local name -> crate root identifier."""
    imports: dict[str, str] = {}

    def walk(tree: str, root: str | None) -> None:
        tree = tree.strip()
        if not tree:
            return
        if tree.startswith("{"):
            depth, part, parts = 0, "", []
            for char in tree[1:tree.rfind("}")]:
                if char == "{":
                    depth += 1
                elif char == "}":
                    depth -= 1
                if char == "," and depth == 0:
                    parts.append(part)
                    part = ""
                else:
                    part += char
            parts.append(part)
            for part in parts:
                walk(part, root)
            return
        head, sep, rest = tree.partition("::")
        head = head.strip()
        if root is None:
            root = head.lstrip(":")
        if sep:
            walk(rest, root)
            return
        name = head.split(" as ")[-1].strip()
        if name and name not in ("*", "self") and root:
            imports[name] = root

    for match in USE_RE.finditer(masked):
        walk(match.group(1), None)
    return {name: root for name, root in imports.items() if prefix_crate(root)}


def parse_file(source: str, rel: str, test_side: bool) -> Parsed:
    masked = mask(source)
    pairs = brace_pairs(masked)
    runs = attribute_runs(source, masked)
    gated_spans = cfg_off_spans(source, masked, runs, pairs)
    impls = []
    for match in IMPL_RE.finditer(masked):
        open_index, char = signature_end(masked, match.end())
        if char != "{":
            continue
        header = re.sub(r"\s+", " ", masked[match.end():open_index]).strip()
        header = re.sub(r"\bwhere\b.*$", "", header)
        while header.startswith("<"):
            depth = 0
            for index, ch in enumerate(header):
                depth += ch == "<"
                depth -= ch == ">"
                if depth == 0:
                    header = header[index + 1:].strip()
                    break
            else:
                break
        trait, _, self_type = header.rpartition(" for ")
        last = lambda path: re.sub(r"<.*$", "", path).strip().split("::")[-1].strip() if path else None
        impls.append((open_index, pairs.get(open_index, len(masked)), last(self_type), trait.strip() or None))
    traits = [(o, pairs.get(o, len(masked))) for o in (
        signature_end(masked, m.end())[0] for m in re.finditer(r"\btrait\s+\w+", masked))]
    inline_mods = [(m.group(1), m.end() - 1, pairs.get(m.end() - 1, len(masked)))
                   for m in MOD_RE.finditer(masked) if m.group(2) == "{"]
    fns = []
    for match in FN_RE.finditer(masked):
        if any(o < match.start() < c for o, c in traits):
            continue
        open_index, char = signature_end(masked, match.end())
        if char != "{":
            continue
        close = pairs.get(open_index, len(masked) - 1)
        owner = max((imp for imp in impls if imp[0] < match.start() < imp[1]), default=None, key=lambda i: i[0])
        enclosing_fn = any(f.start < match.start() < f.end for f in fns)
        if enclosing_fn:
            continue
        attrs = attrs_for(runs, masked, match.start())
        texts = [text for _, text in attrs]
        start = attrs[0][0] if attrs else match.start()
        line = masked.count("\n", 0, start) + 1
        body = masked[open_index:close + 1]
        module = tuple(name for name, o, c in sorted(inline_mods, key=lambda r: r[1]) if o < match.start() < c)
        fn = Fn(
            name=match.group(1), file=rel, start=match.start(), end=close, line=line,
            lines=masked.count("\n", start, close) + 1, body=body,
            impl_self=owner[2] if owner else None, impl_trait=owner[3] if owner else None,
            gated=test_side or inside(gated_spans, match.start()),
            test=any(TEST_ATTR_RE.match(t) for t in texts),
            ignored=any(IGNORE_ATTR_RE.match(t) for t in texts), module=module,
        )
        fn.names = symbol_names(body) | ({fn.impl_trait.split("::")[-1].split("<")[0].strip()} if fn.impl_trait else set())
        fn.paths = {m.group(1) for m in ROOT_PATH_RE.finditer(body)} | {
            m.group(1).replace("-", "_") for m in INCLUDE_RE.finditer(source[open_index:close + 1])}
        fns.append(fn)
    # Test-side types (`struct Fixture { session: WorldSession }`) are helpers too:
    # naming one pulls its definition and its impl methods into a test's closure.
    types = []
    for match in TYPE_RE.finditer(masked):
        if not (test_side or inside(gated_spans, match.start())):
            continue
        end = item_end(masked, match.start(), pairs)
        types.append(Fn(
            name=match.group(1), file=rel, start=match.start(), end=end,
            line=masked.count("\n", 0, match.start()) + 1, lines=masked.count("\n", match.start(), end) + 1,
            body=masked[match.start():end], impl_self=None, impl_trait=None, gated=True, test=False,
            ignored=False, module=(), names=symbol_names(masked[match.end():end]) - (
                set(VARIANT_RE.findall(masked[match.end():end])) if match.group(0).startswith("enum") else set()),
            paths={m.group(1) for m in ROOT_PATH_RE.finditer(masked[match.start():end])},
        ))
    mounts = []
    for decl in MOD_RE.finditer(masked):
        if decl.group(2) != ";":
            continue
        head = decl.start() + len(decl.group(0)) - len(decl.group(0).lstrip())
        attrs = runs.get(head, [])
        path_attr = next((PATH_ATTR_RE.match(t).group(1) for _, t in attrs if PATH_ATTR_RE.match(t)), None)
        nesting = tuple(n for n, o, c in sorted(inline_mods, key=lambda r: r[1]) if o < decl.start() < c)
        mounts.append((decl.group(1), path_attr, nesting, test_side or inside(gated_spans, head)))
    uses = parse_uses(masked, lambda root: root.startswith("wow_") or root in {"crate", "super"})
    return Parsed(rel, fns, uses, mounts, types, gated_spans)
