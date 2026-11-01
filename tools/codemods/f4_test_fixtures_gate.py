#!/usr/bin/env python3
"""#1241 F4a-P2: gate the future wow-world-core code on `any(test, feature = "test-fixtures")`.

`plan` lists every cfg predicate the conversion rewrites; `apply` rewrites them in place, then runs
a compiler loop (`--text-only` skips it). A rewrite only adds the feature next to a bare `test` atom:
`cfg(test)` -> `cfg(any(test, feature = "test-fixtures"))`, `not(test)` ->
`not(any(test, feature = "test-fixtures"))`, `all(test, P)` -> `all(any(test, feature = ..), P)`,
`cfg_attr(test, X)` -> `cfg_attr(any(test, feature = ..), X)`, `cfg!(test)` likewise, and
`any(test, P)` -> `any(test, feature = "test-fixtures", P)`. With the feature off (every production
build) each rewritten predicate reduces to the original, so production compiles the same items.

Rewritten sites: whole target files, impl blocks of the target types anywhere, the listed items and
impls (from `f4_base_targets.json`), plus the sites that must agree with them: struct literals and
patterns of a target struct (their field-init attributes), cfg-gated fields of non-target structs
whose type names a target type, `mod` mounts of target files and the `use` lines re-exporting them.
Never rewritten: unit-test mounts (`#[path = ".../unit_tests/..."]`, `mod tests`, `mod f3_shims`)
and inline `mod tests { .. }` blocks. Line count never changes. Re-running is a no-op. Standard
library only.
"""
from __future__ import annotations

import argparse, collections, json, os, pathlib, re, subprocess, sys

REPO = pathlib.Path(__file__).resolve().parents[2]
FEATURE = 'feature = "test-fixtures"'
GATE = f"any(test, {FEATURE})"
SRC = "crates/wow-world/src"
IDENT = r"[A-Za-z_][A-Za-z0-9_]*"
DEFAULT_TARGETS = pathlib.Path(__file__).resolve().parent / "f4_base_targets.json"
NEVER_MOUNT = {"tests", "f3_shims"}


class CodemodError(Exception):
    pass


def lib(root):
    """The coupling tool's lexer (blank_noncode/matching_close); falls back to this checkout's copy."""
    for base in (os.environ.get("F4_ARCH_DIR"), root / "tools/architecture", REPO / "tools/architecture"):
        if base and (pathlib.Path(base) / "wow_world_coupling.py").exists():
            sys.path.insert(0, str(base))
            import wow_world_coupling as W
            return W
    raise CodemodError("tools/architecture/wow_world_coupling.py not found (set F4_ARCH_DIR)")


# ---- cfg predicate rewriting -------------------------------------------------------------------

TOKEN = re.compile(r'\s*(?:(' + IDENT + r')|("(?:[^"\\]|\\.)*")|([(),=]))')


def parse_pred(text):
    """Tiny cfg-predicate parser: atom | key = "str" | name(list). Returns (node, end)."""
    pos = 0

    def tok():
        nonlocal pos
        m = TOKEN.match(text, pos)
        if not m:
            raise CodemodError(f"cannot parse cfg predicate {text!r}")
        pos = m.end()
        return m.group(1) or m.group(2) or m.group(3)

    def peek():
        m = TOKEN.match(text, pos)
        return (m.group(1) or m.group(2) or m.group(3)) if m else None

    def node():
        name = tok()
        if peek() == "(":
            tok()
            kids = []
            while peek() != ")":
                kids.append(node())
                if peek() == ",":
                    tok()
            tok()
            return ("call", name, kids)
        if peek() == "=":
            tok()
            return ("kv", name, tok())
        return ("atom", name)

    n = node()
    if text[pos:].strip():
        raise CodemodError(f"trailing text in cfg predicate {text!r}")
    return n


def show(n):
    if n[0] == "atom":
        return n[1]
    if n[0] == "kv":
        return f"{n[1]} = {n[2]}"
    return f"{n[1]}(" + ", ".join(show(k) for k in n[2]) + ")"


FEAT_NODE = ("kv", "feature", '"test-fixtures"')


def gate(n):
    """Add the feature next to every bare `test` atom; already-gated `any(test, feature..)` stays."""
    if n == ("atom", "test"):
        return ("call", "any", [("atom", "test"), FEAT_NODE])
    if n[0] != "call":
        return n
    kids = n[2]
    if n[1] == "any" and ("atom", "test") in kids:
        if FEAT_NODE in kids:
            return ("call", "any", [k if k == ("atom", "test") or k == FEAT_NODE else gate(k) for k in kids])
        i = kids.index(("atom", "test"))
        rest = [gate(k) for k in kids[i + 1:]]
        return ("call", "any", [gate(k) for k in kids[:i]] + [("atom", "test"), FEAT_NODE] + rest)
    return ("call", n[1], [gate(k) for k in kids])


def has_bare_test(n, parent_any_feat=False):
    if n == ("atom", "test"):
        return not parent_any_feat
    if n[0] != "call":
        return False
    feat = n[1] == "any" and FEAT_NODE in n[2]
    return any(has_bare_test(k, feat) for k in n[2])


# A cfg site: `#[cfg(P)]`, `#![cfg(P)]`, `#[cfg_attr(P, ..)]`, `cfg!(P)`. Offsets on blanked code.
SITE = re.compile(r"#!?\s*\[\s*(cfg|cfg_attr)\s*\(|\bcfg\s*!\s*\(")


def cfg_sites(W, raw, code, a=0, b=None):
    """[(pred_start, pred_end, form, attr_start, attr_end)] for every cfg predicate in [a, b)."""
    b = len(code) if b is None else b
    out = []
    for m in SITE.finditer(code, a, b):
        open_i = m.end() - 1
        close = paren_close(code, open_i)
        if m.group(1) == "cfg_attr":
            depth, j = 0, open_i + 1                         # the predicate ends at the first top-level `,`
            while j < close:
                ch = code[j]
                if ch == "(":
                    depth += 1
                elif ch == ")":
                    depth -= 1
                elif ch == "," and depth == 0:
                    break
                j += 1
            ps, pe, form = open_i + 1, j, "cfg_attr"
        else:
            ps, pe, form = open_i + 1, close, "cfg!" if m.group(1) is None else "cfg"
        end = code.find("]", close) + 1 if m.group(0).startswith("#") else close + 1
        out.append((ps, pe, form, m.start(), end))
    return out


def paren_close(code, i):
    depth = 0
    for j in range(i, len(code)):
        if code[j] == "(":
            depth += 1
        elif code[j] == ")":
            depth -= 1
            if depth == 0:
                return j
    raise CodemodError(f"unbalanced parenthesis at offset {i}")


def form_of(node):
    """Mapping-form label for reporting."""
    s = show(node)
    if s == "test":
        return "cfg(test)"
    if s == "not(test)":
        return "not(test)"
    return "compound"


# ---- target spans ------------------------------------------------------------------------------

IMPL_HEAD = re.compile(r"(?m)^[ \t]*impl\b")
ITEM_HEAD = r"(?m)^[ \t]*(?:pub\s*(?:\([^)]*\))?\s*)?(?:(?:const|async|unsafe)\s+)*(?:struct|enum|union|fn|const|static|type|trait)\s+"


def line_start(text, i):
    return text.rfind("\n", 0, i) + 1


def with_attrs(raw, start):
    """Extend `start` (a line start) upward over attribute and doc-comment lines."""
    s = start
    while s > 0:
        prev = line_start(raw, s - 1)
        line = raw[prev:s].strip()
        if line.startswith(("#[", "#![", "///", "//!")):
            s = prev
        else:
            break
    return s


def item_end(W, code, i):
    """End offset (exclusive) of the item whose header starts at i: matching `}` or the `;`."""
    depth = 0
    j = i
    while j < len(code):
        ch = code[j]
        if ch in "([":
            depth += 1
        elif ch in ")]":
            depth -= 1
        elif ch == "{" and depth == 0:
            close = W.matching_close(code, j)
            k = close + 1
            m = re.match(r"\s*;", code[k:k + 20])               # `const X: T = T { .. };`
            return k + (m.end() if m and re.search(r"\b(?:const|static)\b", code[line_start(code, i):i + 40]) else 0)
        elif ch == ";" and depth == 0:
            return j + 1
        j += 1
    return len(code)


def impl_self(header):
    """Self type name of an `impl ... {` header (last path segment, generics stripped)."""
    h = re.sub(r"^\s*impl\s*", "", header)
    if h.startswith("<"):
        depth = 0
        for k, ch in enumerate(h):
            depth += ch == "<"
            depth -= ch == ">"
            if depth == 0:
                h = h[k + 1:]
                break
    h = re.split(r"\bwhere\b", h)[0]
    parts = re.split(r"\bfor\b", h)
    ty = parts[-1].strip().lstrip("&").strip()
    ty = re.sub(r"<.*", "", ty, flags=re.S).strip()
    return ty.split("::")[-1].strip()


def impl_blocks(W, code):
    """[(start_line_offset, end, self_type)] of top-level-or-nested impl blocks."""
    out = []
    for m in IMPL_HEAD.finditer(code):
        brace = code.find("{", m.end())
        semi = code.find(";", m.end())
        if brace < 0 or (0 <= semi < brace):
            continue
        out.append((line_start(code, m.start()), W.matching_close(code, brace) + 1, impl_self(code[m.start():brace])))
    return out


def item_span(W, raw, code, name, kinds=None):
    for m in re.finditer(ITEM_HEAD + re.escape(name) + r"\b", code):
        s = line_start(code, m.start())
        return with_attrs(raw, s), item_end(W, code, m.start())
    return None


def mount_target(rel, name, path_attr):
    """Repository-relative file a `mod name;` in `rel` resolves to."""
    d = os.path.dirname(rel)
    base = os.path.basename(rel)
    if path_attr:
        return os.path.normpath(os.path.join(d, path_attr))
    sub = d if base in ("mod.rs", "lib.rs", "main.rs") else os.path.join(d, base[:-3])
    return os.path.join(sub, name + ".rs")


MOUNT = re.compile(r"(?P<attrs>(?:#\s*\[[^\]]*\]\s*)+)(?:pub\s*(?:\([^)]*\))?\s+)?mod\s+(?P<name>" + IDENT + r")\s*(?P<end>[;{])")


def never_mounts(raw, code):
    """Attribute spans of unit-test mounts and inline `mod tests {}` blocks: never rewritten."""
    spans = []
    for m in MOUNT.finditer(raw):
        attrs = m.group("attrs")
        p = re.search(r'path\s*=\s*"([^"]+)"', attrs)
        if m.group("name") in NEVER_MOUNT or (p and "unit_tests/" in p.group(1)):
            end = m.end()
            if m.group("end") == "{":
                end = lib_close(code, m.end() - 1)
            spans.append((m.start(), end))
    return spans


def lib_close(code, i):
    depth = 0
    for j in range(i, len(code)):
        if code[j] == "{":
            depth += 1
        elif code[j] == "}":
            depth -= 1
            if depth == 0:
                return j + 1
    return len(code)


def load_targets(path):
    t = json.loads(pathlib.Path(path).read_text())
    t.setdefault("files", [])
    t.setdefault("impl_types", [])
    t.setdefault("items", [])
    t.setdefault("impls", [])
    t.setdefault("literal_types", [])
    t.setdefault("struct_fields", [])
    t.setdefault("use_names", [])
    return t


def plan(root, targets):
    W = lib(root)
    src = root / SRC
    tfiles = set(targets["files"])
    impl_types = set(targets["impl_types"])
    item_by_file = collections.defaultdict(set)
    for it in targets["items"]:
        item_by_file[it["file"]].add(it["name"])
    impl_by_file = collections.defaultdict(set)
    for it in targets["impls"]:
        impl_by_file[it["file"]].add(it["self"])
    struct_items = {it["name"] for it in targets["items"] if it.get("kind") in ("struct", "enum", "union")}
    lit_types = impl_types | struct_items | set(targets["literal_types"]) | set(targets["struct_fields"])
    type_names = lit_types | {it["name"] for it in targets["items"]}
    files = {}
    for p in sorted(src.rglob("*.rs")):
        rel = str(p.relative_to(root))
        raw = p.read_text()
        files[rel] = (raw, W.blank_noncode(raw))
    paired = collections.defaultdict(set)                 # struct -> fields whose declaration pairs
    for rel, (raw, code) in files.items():
        if rel not in tfiles:
            for *_span, struct, field in field_decl_regions(W, raw, code, type_names):
                paired[struct].add(field)
    sites, excluded = [], 0
    for rel, (raw, code) in files.items():
        regions = []                                          # (start, end, category)
        if rel in tfiles:
            regions.append((0, len(code), "target-file"))
        else:
            for s, e, ty in impl_blocks(W, code):
                if ty in impl_types or ty in impl_by_file.get(rel, ()):
                    regions.append((with_attrs(raw, s), e, "target-impl"))
            for name in impl_types:                               # the target types' own definitions
                sp = item_span(W, raw, code, name)
                if sp and re.search(r"\b(?:struct|enum|union)\s+" + re.escape(name) + r"\b", code[sp[0]:sp[1]]):
                    regions.append((sp[0], sp[1], "target-type"))
            for name in item_by_file.get(rel, ()):
                sp = item_span(W, raw, code, name)
                if sp:
                    regions.append((sp[0], sp[1], "target-item"))
            for name in targets["struct_fields"]:
                sp = item_span(W, raw, code, name)
                if sp and re.search(r"\b(?:struct|union)\s+" + re.escape(name) + r"\b", code[sp[0]:sp[1]]):
                    regions.append((sp[0], sp[1], "struct-fields"))
            regions += literal_regions(W, raw, code, lit_types)
            regions += [r[:3] for r in field_decl_regions(W, raw, code, type_names)]
            regions += paired_literal_regions(W, raw, code, paired)
            regions += mount_regions(raw, code, rel, tfiles, type_names, bool(regions))
            regions += use_name_regions(raw, set(targets["use_names"]))
        if not regions:
            continue
        never = never_mounts(raw, code)
        seen = set()
        for region in regions:
            a, b, cat = region[:3]
            allowed = region[3] if len(region) > 3 else None
            for ps, pe, form, at, ae in cfg_sites(W, raw, code, a, b):
                if (ps, pe) in seen or (allowed is not None and at not in allowed):
                    continue
                seen.add((ps, pe))
                if any(x <= at < y for x, y in never):
                    excluded += 1
                    continue
                pred = raw[ps:pe]
                node = parse_pred(pred)
                if not has_bare_test(node):
                    continue
                sites.append(dict(file=rel, line=raw.count("\n", 0, ps) + 1, start=ps, end=pe, old=pred.strip(),
                                  new=show(gate(node)), category=cat, form=form if form != "cfg" else form_of(node)))
    return dict(sites=sites, excluded=excluded, files=files)


def literal_regions(W, raw, code, lit_types):
    """Struct literal / pattern bodies of a target type: their depth-1 field attributes pair."""
    out = []
    if not lit_types:
        return out
    pat = re.compile(r"\b(" + "|".join(sorted(map(re.escape, lit_types), key=len, reverse=True)) + r")\s*\{")
    for m in pat.finditer(code):
        if is_literal(code, m.start()):
            out.append(literal_body(W, code, m.end() - 1, "literal-pairing"))
    for s, e, ty in impl_blocks(W, code):                     # `Self {` inside `impl T`
        if ty in lit_types:
            for m in re.finditer(r"\bSelf\s*\{", code[s:e]):
                if is_literal(code, s + m.start()):
                    out.append(literal_body(W, code, s + m.end() - 1, "literal-pairing"))
    return out


def is_literal(code, k):
    """Whether `Name {` at k opens a struct literal/pattern (not a definition, impl or return type)."""
    while True:                                               # skip a `a::b::` path prefix
        pm = re.search(r"(" + IDENT + r")\s*::\s*$", code[max(0, k - 80):k])
        if not pm:
            break
        k -= len(pm.group(0))
    prev = re.search(r"(\S+)\s*$", code[max(0, k - 40):k])
    return not (prev and re.search(r"(?:\bstruct|\benum|\bunion|\bimpl|\bfor|\btrait|->|\bdyn|\btype|\bmod|&|'_|>)$",
                                   prev.group(1)))


def literal_body(W, code, open_i, cat, fields=None):
    """(start, end, cat, allowed attribute offsets): depth-1 attributes, optionally only on `fields`."""
    close = W.matching_close(code, open_i)
    allowed, depth = set(), 0
    for j in range(open_i, close):
        ch = code[j]
        if ch in "{([":
            depth += 1
        elif ch in "})]":
            depth -= 1
        elif ch == "#" and depth == 1:
            if fields is None:
                allowed.add(j)
            else:
                fm = re.match(r"(?:#\s*\[[^\]]*\]\s*)+(" + IDENT + r")\s*[:,}]", code[j:j + 400])
                if fm and fm.group(1) in fields:
                    allowed.add(j)
    return (open_i + 1, close, cat, allowed)


def paired_literal_regions(W, raw, code, paired):
    """Literals (`T {` or `Self {` inside `impl T`) of structs with a paired field: that field's init."""
    out = []
    if not paired:
        return out
    pat = re.compile(r"\b(" + "|".join(sorted(map(re.escape, paired), key=len, reverse=True)) + r")\s*\{")
    for m in pat.finditer(code):
        if is_literal(code, m.start()):
            out.append(literal_body(W, code, m.end() - 1, "field-init-pairing", paired[m.group(1)]))
    for s, e, ty in impl_blocks(W, code):
        if ty in paired:
            for m in re.finditer(r"\bSelf\s*\{", code[s:e]):
                if is_literal(code, s + m.start()):
                    out.append(literal_body(W, code, s + m.end() - 1, "field-init-pairing", paired[ty]))
    return out


def field_decl_regions(W, raw, code, type_names):
    """cfg attributes on fields of non-target structs whose type names a target type."""
    out = []
    if not type_names:
        return out
    tn = re.compile(r"\b(?:" + "|".join(map(re.escape, sorted(type_names))) + r")\b")
    for m in re.finditer(r"\bstruct\s+(" + IDENT + r")[^;{]*\{", code):
        open_i = m.end() - 1
        close = W.matching_close(code, open_i)
        body = code[open_i + 1:close]
        depth, seg = 0, 0
        for k, ch in enumerate(body + ","):
            if ch in "([{<":
                depth += 1
            elif ch in ")]}>":
                depth -= 1
            elif ch == "," and depth == 0:
                field = body[seg:k]
                fm = re.search(r":(.*)$", field, flags=re.S)
                fname = re.search(r"(?<![:\w])(" + IDENT + r")\s*:(?!:)", field)
                if fm and fname and tn.search(fm.group(1)) and re.search(r"cfg\s*\(\s*test\s*\)", field):
                    out.append((open_i + 1 + seg, open_i + 1 + k, "field-pairing", m.group(1), fname.group(1)))
                seg = k + 1
    return out


def mount_regions(raw, code, rel, tfiles, type_names, has_regions=False):
    """`mod x;` mounts of target files, and `use` lines naming a converted mount or target type."""
    out, mounted = [], set()
    for m in MOUNT.finditer(raw):
        if m.group("end") != ";":
            continue
        p = re.search(r'path\s*=\s*"([^"]+)"', m.group("attrs"))
        tgt = mount_target(rel, m.group("name"), p.group(1) if p else None)
        alt = tgt[:-3] + "/mod.rs"
        if tgt in tfiles or alt in tfiles:
            out.append((m.start(), m.end(), "mount-pairing"))
            mounted.add(m.group("name"))
    names = mounted | set(type_names)
    if mounted or has_regions:
        for m in re.finditer(r"(?P<attrs>(?:#\s*\[[^\]]*\]\s*)+)(?:pub\s*(?:\([^)]*\))?\s+)?use\s+(?P<path>[^;]+);", raw):
            idents = set(re.findall(IDENT, m.group("path")))
            first = m.group("path").strip().split("::")[0].strip()
            if first in mounted or (idents & names and first in {"super", "self", "crate"}):
                leaves = {x for x in idents if x[0].isupper()}
                out.append((m.start(), m.end(), "use-pairing" if leaves <= names else "use-pairing-mixed"))
    return out


def use_leaves(path):
    """Imported names of a `use` path: last segments and `{..}` members, never module segments."""
    return set(re.findall(r"(" + IDENT + r")\s*(?=[,}]|$|\s+as\b)", path.strip()))


USE_STMT = re.compile(r"(?P<attrs>(?:#\s*\[[^\]]*\]\s*)+)(?:pub\s*(?:\([^)]*\))?\s+)?use\s+(?P<path>[^;]+);")


def use_name_regions(raw, use_names):
    """`use` lines importing a name the feature-only build could not resolve (compile-loop growth)."""
    if not use_names:
        return []
    return [(m.start(), m.end(), "use-loop") for m in USE_STMT.finditer(raw)
            if use_leaves(m.group("path")) & use_names]


def apply_text(root, P):
    by_file = collections.defaultdict(list)
    for s in P["sites"]:
        by_file[s["file"]].append(s)
    written = []
    for rel, ss in by_file.items():
        raw = P["files"][rel][0]
        out = raw
        for s in sorted(ss, key=lambda s: s["start"], reverse=True):
            lead = out[s["start"]:s["end"]]
            pad_l = lead[:len(lead) - len(lead.lstrip())]
            pad_r = lead[len(lead.rstrip()):]
            out = out[:s["start"]] + pad_l + s["new"] + pad_r + out[s["end"]:]
        assert_diff_shape(raw, out, rel)
        if out != raw:
            (root / rel).write_text(out)
            written.append(rel)
    return written


# ---- diff-shape assertion ----------------------------------------------------------------------

Q3 = f"any(test, {FEATURE}, "


def unmap_line(line):
    return line.replace(Q3, "any(test, ").replace(GATE, "test")


def assert_diff_shape(old, new, label="<text>", formatted=False):
    """Every changed line must be its old line plus inserted feature gates; line count unchanged.

    `formatted=True` (after `cargo fmt`, which rewraps a few long `cfg!(..)` expressions) compares
    whitespace-normalized token text instead: the same claim, independent of line breaks."""
    if formatted:
        x, y = (re.sub(r"\s+", " ", t).replace("( ", "(").replace(" )", ")") for t in (old, new))
        if unmap_line(x) != unmap_line(y) or (x != y and y.count(FEATURE) <= x.count(FEATURE)):
            raise CodemodError(f"{label}: tokens differ beyond feature-gate insertions")
        return True
    a, b = old.split("\n"), new.split("\n")
    if len(a) != len(b):
        raise CodemodError(f"{label}: line count changed {len(a)} -> {len(b)} (use --formatted after cargo fmt)")
    for i, (x, y) in enumerate(zip(a, b), 1):
        if x == y:
            continue
        if unmap_line(x) != unmap_line(y) or y.count(FEATURE) <= x.count(FEATURE) or "cfg" not in y:
            raise CodemodError(f"{label}:{i}: change is not a pure feature-gate insertion:\n- {x}\n+ {y}")
    return True


def diff_shape_dirs(before, after, formatted=False, exclude=()):
    """Check every .rs file under `after` against `before` (same relative paths), except `exclude`."""
    n = 0
    for p in sorted(pathlib.Path(after).rglob("*.rs")):
        q = pathlib.Path(before) / p.relative_to(after)
        if q.exists() and str(p.relative_to(after)) not in exclude:
            assert_diff_shape(q.read_text(), p.read_text(), str(p.relative_to(after)), formatted)
            n += 1
    return n


# ---- compiler loop (wired like f3_move_methods.compile_loop; needs a scheduled cargo slot) ------

CHECKS = (["check", "-p", "wow-world", "--lib", "--features", "test-fixtures"],
          ["check", "-p", "wow-world", "--all-targets"],
          ["check", "-p", "world-server", "--all-targets"])
CANNOT_FIND = re.compile(r"cannot find (?:type|value|function|struct, variant or union type|macro|trait) `(" + IDENT + ")`"
                         r"|unresolved import `(?:[\w:]*::)?(" + IDENT + ")`|no `(" + IDENT + ")` in `")
NO_FIELD = re.compile(r"no field `" + IDENT + r"` on type `[^`]*?(" + IDENT + r")(?:<[^`]*>)?`|struct `(?:[\w:]*::)?(" + IDENT + r")` has no field named")
MISSING = re.compile(r"missing fields? (.*) in initializer of `(?:[\w:]*::)?(" + IDENT + r")")
NO_METHOD = re.compile(r"no (?:method|function or associated item) named `(" + IDENT + r")`")


def cargo_check(root, args, log):
    env = dict(os.environ)
    env.setdefault("CARGO_BUILD_JOBS", "1")
    env.setdefault("CARGO_TARGET_DIR", str(root / "target"))
    with log.open("w") as h:
        rc = subprocess.run(["cargo", *args, "--message-format=json"], cwd=root, stdout=h,
                            stderr=subprocess.DEVNULL, env=env).returncode
    msgs = []
    for line in log.read_text().splitlines():
        try:
            rec = json.loads(line)
        except json.JSONDecodeError:
            continue
        if rec.get("reason") == "compiler-message" and rec["message"].get("level") == "error":
            msgs.append(rec["message"])
    return rc, msgs


def gated_definition(root, name):
    """(file, name) of a `#[cfg(test)]`-gated wow-world src item named `name`, or None."""
    W = lib(root)
    for p in sorted((root / SRC).rglob("*.rs")):
        raw = p.read_text()
        code = W.blank_noncode(raw)
        for m in re.finditer(ITEM_HEAD + re.escape(name) + r"\b", code):
            s = with_attrs(raw, line_start(code, m.start()))
            if re.search(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", raw[s:m.start()]):
                return str(p.relative_to(root))
    return None


def gated_use(root, name):
    """Whether some wow-world src `use` line importing `name` is still `#[cfg(test)]`-gated."""
    for p in sorted((root / SRC).rglob("*.rs")):
        raw = p.read_text()
        if name not in raw:
            continue
        for m in USE_STMT.finditer(raw):
            if name in use_leaves(m.group("path")) and re.search(r"cfg\s*\(\s*test\s*\)", m.group("attrs")):
                return True
    return False


def add_once(lst, value):
    if value in lst:
        return 0
    lst.append(value)
    return 1


def compile_loop(root, targets, tpath, max_rounds, log_dir):
    """Grow the target set from what the feature-only build cannot see; stop on anything else."""
    if os.environ.get("F4_ALLOW_CARGO") != "1":
        raise CodemodError("compile loop needs a scheduled cargo slot: set F4_ALLOW_CARGO=1 (see --text-only)")
    rounds = []
    for i in range(max_rounds):
        grown, other, rc_all = 0, [], 0
        for k, args in enumerate(CHECKS):
            rc, msgs = cargo_check(root, args, log_dir / f"f4-gate-round-{i + 1}-{k}.jsonl")
            rc_all |= rc
            for m in msgs:
                text = m["message"]
                mm = CANNOT_FIND.search(text) or NO_METHOD.search(text)
                nf, ms = NO_FIELD.search(text), MISSING.search(text)
                names = set(re.findall(r"`(?:[\w:]*::)?(" + IDENT + r")`", text)) if mm or text.startswith(
                    "unresolved import") else set()
                hit = False
                for name in sorted(names):
                    if (rel := gated_definition(root, name)):
                        grown += add_once(targets["items"], {"file": rel, "name": name, "kind": "auto"})
                        hit = True
                    if gated_use(root, name):
                        grown += add_once(targets["use_names"], name)
                        hit = True
                if hit:
                    continue
                elif nf:
                    grown += add_once(targets["struct_fields"], next(g for g in nf.groups() if g))
                elif ms:
                    grown += add_once(targets["literal_types"], ms.group(2))
                elif not text.startswith("aborting"):
                    other.append(f"{(m.get('code') or {}).get('code')}: {text}")
        if grown:
            pathlib.Path(tpath).write_text(json.dumps(targets, indent=1))
            apply_text(root, plan(root, targets))
        rounds.append(grown)
        print(f"round {i + 1}: exit {rc_all}, {grown} target-set additions", file=sys.stderr)
        if grown == 0:
            if rc_all != 0:
                for line in other[:40]:
                    print("unhandled:", line, file=sys.stderr)
                raise CodemodError(f"cargo fails with {len(other)} errors the gate conversion did not cause")
            return rounds
    raise CodemodError("no fixed point within the round budget")


# ---- report / main -----------------------------------------------------------------------------

def report(P):
    s = P["sites"]
    out = [f"{len(s)} cfg predicates to gate in {len({x['file'] for x in s})} files; "
           f"{P['excluded']} unit-test mount attributes left unchanged"]
    out.append("by category: " + ", ".join(f"{k} {v}" for k, v in collections.Counter(x["category"] for x in s).most_common()))
    out.append("by form: " + ", ".join(f"{k} {v}" for k, v in collections.Counter(x["form"] for x in s).most_common()))
    top = collections.Counter(x["file"] for x in s).most_common(15)
    out.append("top files: " + ", ".join(f"{f} {n}" for f, n in top))
    out.append("")
    for x in s:
        out.append(f"{x['file']}:{x['line']}\t{x['category']}\t{x['old']} -> {x['new']}")
    return "\n".join(out)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    for name in ("plan", "apply"):
        p = sub.add_parser(name)
        p.add_argument("--root", default=str(REPO))
        p.add_argument("--targets", default=str(DEFAULT_TARGETS))
        p.add_argument("--json", action="store_true")
        if name == "apply":
            p.add_argument("--text-only", action="store_true")
            p.add_argument("--max-rounds", type=int, default=6)
    d = sub.add_parser("diff-shape", help="assert before/after trees differ only by feature-gate insertions")
    d.add_argument("before")
    d.add_argument("after")
    d.add_argument("--formatted", action="store_true", help="token comparison (after cargo fmt rewraps)")
    d.add_argument("--exclude", action="append", default=[], help="relative path edited by hand (repeatable)")
    a = ap.parse_args(argv)
    try:
        if a.cmd == "diff-shape":
            print(f"diff-shape ok: {diff_shape_dirs(a.before, a.after, a.formatted, set(a.exclude))} files")
            return 0
        root = pathlib.Path(a.root).resolve()
        targets = load_targets(a.targets)
        P = plan(root, targets)
        if a.cmd == "plan":
            print(json.dumps([{k: v for k, v in x.items() if k not in ("start", "end")} for x in P["sites"]], indent=1)
                  if a.json else report(P))
            return 0
        written = apply_text(root, P)
        print(f"text step: {len(P['sites'])} predicates gated in {len(written)} files" if written
              else "text step: already applied (no-op)", file=sys.stderr)
        if not a.text_only:
            log_dir = root / "target/f4-codemod"
            log_dir.mkdir(parents=True, exist_ok=True)
            print(f"compiler loop: rounds {compile_loop(root, targets, a.targets, a.max_rounds, log_dir)}", file=sys.stderr)
    except CodemodError as e:
        print(f"f4 gate codemod: {e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
