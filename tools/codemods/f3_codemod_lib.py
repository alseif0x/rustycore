#!/usr/bin/env python3
"""Text helpers for f3_move_methods.py (#1241 F3): lexing, signatures, paths and templates.

Standard library only; split out so the codemod stays a reviewable size.
"""
from __future__ import annotations

import bisect
import collections
import json
import os
import pathlib
import re
import subprocess

IDENT = r"[A-Za-z_][A-Za-z0-9_]*"
# #1241 F4a-P2: code that moves to wow-world-core is gated on the `test-fixtures` feature as well as
# `test`, so a dependent crate's tests still see it. TEST_PRED matches either predicate, on raw text
# or on text whose string contents are blanked (`"test-fixtures"` -> 13 spaces between the quotes).
FIXTURES_GATE = 'any(test, feature = "test-fixtures")'
FIXTURES_ATTR = f"#[cfg({FIXTURES_GATE})]"
TEST_PRED = r'(?:test|any\s*\(\s*test\s*,\s*feature\s*=\s*"(?:test-fixtures| {13})"\s*\))'
TEST_ATTR = re.compile(r"#\s*\[\s*cfg\s*\(\s*" + TEST_PRED + r"\s*\)\s*\]")
TEST_CFG = re.compile(r"cfg\s*\(\s*" + TEST_PRED + r"\s*\)")


def strip_cfg_test(body):
    """Blank `#[cfg(test)]` and `#[cfg(any(test, feature = "test-fixtures"))]` statements/blocks so
    production accesses remain."""
    out = list(body)
    for m in TEST_ATTR.finditer(body):
        k = m.end()
        while k < len(body) and body[k].isspace():
            k += 1
        depth, e = 0, k
        while e < len(body):
            ch = body[e]
            if ch in "([{":
                depth += 1
            elif ch in ")]}":
                depth -= 1
                if depth < 0:
                    break
                if depth == 0 and ch == "}":                    # a block, item or impl ends here
                    e += 1
                    break
            elif ch in ";," and depth == 0:
                e += 1
                break
            e += 1
        for q in range(m.start(), min(e, len(body))):
            if out[q] != "\n":
                out[q] = " "
    return "".join(out)


def item_segments(W, code, open_i, close):
    """(segment_start, fn_name_match_start, body_open, body_close) for fns directly in an impl body."""
    out, depth, nest, seg, j = [], 0, 0, open_i + 1, open_i + 1
    while j < close:
        ch = code[j]
        if ch in "([" and depth == 0:                         # `-> [u8; 3]`: a `;` inside brackets
            nest += 1
        elif ch in ")]" and depth == 0:
            nest -= 1
        elif ch == "{" and depth == 0:
            m = W.FN_ITEM.search(code, seg, j)
            if m:
                bclose = W.matching_close(code, j)
                out.append((seg, m, j, bclose))
                seg = j = bclose + 1
                continue
            depth += 1
        elif ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                seg = j + 1
        elif ch == ";" and depth == 0 and nest == 0:
            seg = j + 1
        j += 1
    return out


def module_paths(src, codes):
    """Module path per file, honouring `#[path]` on `mod x;` declarations."""
    paths = {"lib.rs": "crate"}
    todo = ["lib.rs"]
    decl = re.compile(r"((?:#\s*\[[^\]]*\]\s*)*)(?:pub(?:\s*\([^)]*\))?\s+)?mod\s+(" + IDENT + r")\s*;")
    while todo:
        rel = todo.pop()
        p = pathlib.PurePosixPath(rel)
        base = p.parent if p.name in ("mod.rs", "lib.rs") else p.parent / p.stem
        raw = (src / rel).read_text(errors="replace")
        for m in decl.finditer(codes.get(rel, "")):
            pm = re.search(r'#\s*\[\s*path\s*=\s*"([^"]+)"', raw[m.start():m.end()])
            cands = ([str(pathlib.PurePosixPath(os.path.normpath(str(p.parent / pm.group(1)))))] if pm else
                     [str(base / f"{m.group(2)}.rs"), str(base / m.group(2) / "mod.rs")])
            for c in cands:
                if c in codes and c not in paths:
                    paths[c] = paths[rel] + "::" + m.group(2)
                    todo.append(c)
    return paths


def param_span(c, start, bo):
    """(open, close) of the parameter list after a fn name (generic `<..>` skipped)."""
    depth, k = 0, start
    while k < bo and not (c[k] == "(" and depth == 0):
        depth += {"<": 1, ">": -1}.get(c[k], 0) if c[k - 1] != "-" else 0
        k += 1
    d, close = 0, k
    for close in range(k, bo):
        d += {"(": 1, ")": -1}.get(c[close], 0)
        if d == 0:
            break
    return k, close


def ret_type(c, start, bo):
    tail = re.split(r"\bwhere\b", c[param_span(c, start, bo)[1] + 1:bo])[0]
    return tail.split("->", 1)[1] if "->" in tail else ""


def split_params(text):
    out, depth, start = [], 0, 0
    for j, ch in enumerate(text):
        if ch in "([{<":
            depth += 1
        elif ch in ")]}>" and not (ch == ">" and j and text[j - 1] == "-"):
            depth -= 1
        elif ch == "," and depth == 0:
            out.append(text[start:j])
            start = j + 1
    out.append(text[start:])
    return [p.strip() for p in out if p.strip()]


def line_start(raw, off):
    k = off
    while k < len(raw) and raw[k] in " \t\r\n":
        k += 1
    return raw.rfind("\n", 0, k) + 1


def abs_vis(vis, modpath):
    if vis.startswith("pub(in"):
        return "pub(in " + vis[len("pub(in"):].strip()
    if vis in ("pub", "pub(crate)"):
        return vis
    if vis == "pub(super)":
        return f"pub(in {modpath.rsplit('::', 1)[0]})"
    return f"pub(in {modpath})"


def cargo_check(root, log):
    cmd = ["cargo", "check", "-p", "wow-world", "--all-targets", "--message-format=json"]
    env = dict(os.environ)
    env.setdefault("CARGO_BUILD_JOBS", "1")
    env.setdefault("CARGO_TARGET_DIR", str(root / "target"))
    with log.open("w") as h:
        rc = subprocess.run(cmd, cwd=root, stdout=h, stderr=subprocess.DEVNULL, env=env).returncode
    msgs = []
    for line in log.read_text().splitlines():
        try:
            rec = json.loads(line)
        except json.JSONDecodeError:
            continue
        if rec.get("reason") == "compiler-message" and rec["message"].get("level") in ("error", "warning"):
            msgs.append(rec["message"])
    return rc, msgs


HUB_RS = '''// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Hub views (#1241 F3): split borrows of the hub members, built from disjoint WorldSession fields.

use super::*;

/// Shared hub view: core, catalogs, config and the cfg(test) fixtures. Copy; it holds only
/// shared references and never a lock guard, so it is Send wherever the session is Sync.
#[derive(Clone, Copy)]
pub(crate) struct HubRef<'a> {
    pub(crate) core: &'a SessionCore,
    pub(crate) catalogs: &'a SessionCatalogs,
    pub(in crate::session) config: &'a SessionWorldConfig,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fixtures: &'a SessionFixtures,
}

/// Mutable hub view for moved fns that take `&mut self` (core and fixtures writable).
pub(crate) struct HubMut<'a> {
    pub(crate) core: &'a mut SessionCore,
    pub(crate) catalogs: &'a SessionCatalogs,
    pub(in crate::session) config: &'a SessionWorldConfig,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fixtures: &'a mut SessionFixtures,
}

impl HubMut<'_> {
    pub(crate) fn shared(&self) -> HubRef<'_> {
        HubRef { core: &*self.core, catalogs: self.catalogs, config: self.config,
                 #[cfg(any(test, feature = "test-fixtures"))] fixtures: &*self.fixtures }
    }
}

/// Builds the shared view from disjoint WorldSession fields (free fn: not an `impl WorldSession` item).
pub(crate) fn hub_ref(s: &WorldSession) -> HubRef<'_> {
    HubRef { core: &s.core, catalogs: &s.catalogs, config: &s.config, #[cfg(any(test, feature = "test-fixtures"))] fixtures: &s.fixtures }
}

pub(crate) fn hub_mut(s: &mut WorldSession) -> HubMut<'_> {
    HubMut { core: &mut s.core, catalogs: &s.catalogs, config: &s.config, #[cfg(any(test, feature = "test-fixtures"))] fixtures: &mut s.fixtures }
}
'''


SPLIT_FN = '''
/// `&mut` group state plus the shared hub, borrowed from disjoint WorldSession fields.
pub(crate) fn split_{g}(s: &mut WorldSession) -> (&mut {t}, HubRef<'_>) {{
    (&mut s.{g}, HubRef {{ core: &s.core, catalogs: &s.catalogs, config: &s.config, #[cfg(any(test, feature = "test-fixtures"))] fixtures: &s.fixtures }})
}}
'''
SPLIT_MUT_FN = '''
/// `&mut` group state plus the mutable hub (core and fixtures), borrowed from disjoint fields.
pub(crate) fn split_{g}_mut(s: &mut WorldSession) -> (&mut {t}, HubMut<'_>) {{
    (&mut s.{g}, HubMut {{ core: &mut s.core, catalogs: &s.catalogs, config: &s.config, #[cfg(any(test, feature = "test-fixtures"))] fixtures: &mut s.fixtures }})
}}
'''
SPLIT_REF_FN = '''
/// Shared group state plus the shared hub for `&self` methods.
pub(crate) fn split_{g}_ref(s: &WorldSession) -> (&{t}, HubRef<'_>) {{
    (&s.{g}, hub_ref(s))
}}
'''


def git_ignored(root, path):
    return subprocess.run(["git", "-C", str(root), "check-ignore", "-q", str(path)],
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0


MOUNT = re.compile(r'#\[path = "([^"]+f3_shims\.rs)"\]\nmod f3_shims;')


def default_shim_path(root, rel):
    """`unit_tests/<src path>/f3_shims.rs`, or `<src path>_f3_shims.rs` where .gitignore hides the dir."""
    base = root / "crates/wow-world/unit_tests" / rel[:-3]
    return base / "f3_shims.rs" if not git_ignored(root, base / "f3_shims.rs") else \
        base.with_name(base.name + "_f3_shims.rs")


def shim_path(root, rel):
    """The shim file the source already mounts, else the default path for a new one."""
    source = root / "crates/wow-world/src" / rel
    m = MOUNT.search(source.read_text()) if source.exists() else None
    return pathlib.Path(os.path.normpath(source.parent / m.group(1))) if m else default_shim_path(root, rel)


def relocate_ignored_shims(root, src):
    """Move only shim files a .gitignore rule hides (e.g. a `skills/` dir) and repoint their mounts.
    A mounted, visible shim stays where it is, whatever the default path would be today."""
    moved = []
    for p in sorted(src.rglob("*.rs")):
        text = p.read_text()
        m = MOUNT.search(text)
        if not m:
            continue
        old = pathlib.Path(os.path.normpath(p.parent / m.group(1)))
        if not old.exists() or not git_ignored(root, old):
            continue
        new = default_shim_path(root, p.relative_to(src).as_posix())
        if new != old:
            new.write_text(old.read_text())
            old.unlink()
            p.write_text(text.replace(m.group(1), os.path.relpath(new, p.parent)))
            moved += [p, new]
    return moved


def add_cx_items(text, g, siblings, variant, shared, state_type, own_state=True):
    """Append the capped Cx struct + builder for one variant (and `shared()`) unless present.
    A fixture domain has no production state of its own (`own_state=False`): it reaches
    `fixtures.<g>` through the hub, so its Cx holds the siblings and the hub only."""
    stem = "".join(part.capitalize() for part in g.split("_"))
    members = (g, *siblings) if own_state else tuple(siblings)
    t_mut, t_ref = f"{stem}Cx", f"{stem}CxRef"
    if variant == "cx" and f"pub(crate) struct {t_mut}<" not in text:
        fields = "".join(f"    pub(crate) {m}: &'a mut {state_type[m]},\n" for m in members)
        init = " ".join(f"{m}: &mut s.{m}," for m in members)
        text += (f"\n/// Capped group context (#1241 F3): `{g}` state, its sibling states and the hub, borrowed\n"
                 f"/// from disjoint WorldSession fields.\npub(crate) struct {t_mut}<'a> {{\n{fields}"
                 f"    pub(crate) hub: HubMut<'a>,\n}}\n\npub(crate) fn cx_{g}(s: &mut WorldSession) -> {t_mut}<'_> {{\n"
                 f"    {t_mut} {{ {init} hub: HubMut {{ core: &mut s.core, catalogs: &s.catalogs, config: &s.config, "
                 f"{FIXTURES_ATTR} fixtures: &mut s.fixtures }} }}\n}}\n")
    if (variant == "cx-ref" or shared) and f"pub(crate) struct {t_ref}<" not in text:
        fields = "".join(f"    pub(crate) {m}: &'a {state_type[m]},\n" for m in members)
        init = " ".join(f"{m}: &s.{m}," for m in members)
        text += (f"\n/// Shared counterpart of `{t_mut}` for `&self` methods.\npub(crate) struct {t_ref}<'a> {{\n{fields}"
                 f"    pub(crate) hub: HubRef<'a>,\n}}\n")
        if variant == "cx-ref":
            text += (f"\npub(crate) fn cx_{g}_ref(s: &WorldSession) -> {t_ref}<'_> {{\n"
                     f"    {t_ref} {{ {init} hub: hub_ref(s) }}\n}}\n")
    if variant == "cx-ref" and f"fn cx_{g}_ref(" not in text:
        init = " ".join(f"{m}: &s.{m}," for m in members)
        text += f"\npub(crate) fn cx_{g}_ref(s: &WorldSession) -> {t_ref}<'_> {{\n    {t_ref} {{ {init} hub: hub_ref(s) }}\n}}\n"
    if shared and f"impl {t_mut}<'_> {{" not in text:
        init = " ".join(f"{m}: &*self.{m}," for m in members)
        text += (f"\nimpl {t_mut}<'_> {{\n    pub(crate) fn shared(&self) -> {t_ref}<'_> {{\n"
                 f"        {t_ref} {{ {init} hub: self.hub.shared() }}\n    }}\n}}\n")
    return text


class Spans:
    """Non-overlapping (start, end[, tag]) ranges per file with a bisect lookup."""

    def __init__(self, items):
        self.by_file = collections.defaultdict(list)
        for rel, a, b, *tag in items:
            self.by_file[rel].append((a, b, tag[0] if tag else None))
        for spans in self.by_file.values():
            spans.sort(key=lambda span: span[:2])
        self.starts = {rel: [a for a, _b, _t in spans] for rel, spans in self.by_file.items()}

    def find(self, rel, pos):
        """(start, end, tag) of the range holding pos, or None."""
        spans = self.by_file.get(rel)
        if not spans:
            return None
        i = bisect.bisect_right(self.starts[rel], pos) - 1
        return spans[i] if i >= 0 and spans[i][0] <= pos <= spans[i][1] else None


CX_IMPL = re.compile(r"(" + TEST_ATTR.pattern + r"\s*)?\bimpl\s+crate::session::(" + IDENT + r"Cx(?:Ref)?)<'_>\s*\{")
CX_BUILDER = re.compile(r"\bcx_(" + IDENT + r"?)(_ref)?\s*\(")


def _drop_item(text, head):
    """Remove one generated hub.rs item (and its `///`/attribute lines) whose first line matches `head`."""
    m = re.search(r"(?m)^(?:(?:///[^\n]*|#\[[^\n]*\])\n)*" + head, text)
    if not m:
        return text
    depth, k = 0, text.index("{", m.end() - 1)
    for k in range(k, len(text)):
        depth += {"{": 1, "}": -1}.get(text[k], 0)
        if depth == 0:
            break
    head_text, tail = text[:m.start()].rstrip("\n"), text[k + 1:].lstrip("\n")
    return head_text + ("\n\n" + tail if tail else "\n")


def regen_cx_items(text, src, W, groups, state_type):
    """Rebuild every `<G>Cx`/`<G>CxRef` item in hub.rs from what moved code really reads (#1241 F3):
    members (own state, siblings, `hub`) only when a Cx impl body reads them, builders/`shared()` only
    when called; test-only uses get `#[cfg(test)]` (a test-only `hub` member gets the `test-fixtures`
    gate, matching what F4a-P2 gave hub-typed fields). Unread members would be dead fields."""
    uses = collections.defaultdict(lambda: collections.defaultdict(bool))   # type -> member -> prod?
    calls = collections.defaultdict(bool)                                   # builder -> prod?
    files = [(p, False) for p in sorted(src.rglob("*.rs")) if p.name != "hub.rs"]
    files += [(p, True) for p in sorted((src.parent / "unit_tests").rglob("*.rs"))]
    for p, test_only in files:
        code = W.blank_noncode(p.read_text())
        prod = strip_cfg_test(code)
        for m in CX_IMPL.finditer(code):
            close = W.matching_close(code, m.end() - 1)
            live = not test_only and not m.group(1)
            for b in re.finditer(r"\bself\s*\.\s*(" + IDENT + r")", code[m.end():close]):
                pos = m.end() + b.start()
                uses[m.group(2)][b.group(1)] |= live and prod[pos:pos + 4] == "self"
        for m in CX_BUILDER.finditer(code):
            calls["cx_" + m.group(1) + (m.group(2) or "")] |= not test_only and prod[m.start():m.start() + 3] == "cx_"
    stems = {"".join(x.capitalize() for x in g.split("_")): g for g in groups}
    for t in sorted({t for t in re.findall(r"(?m)^pub\(crate\) struct (\w+)Cx(?:Ref)?<", text)} | {
            t.removesuffix("Ref").removesuffix("Cx") for t in uses}):
        g = stems.get(t)
        if g is None:
            continue
        for head in (rf"pub\(crate\) struct {t}Cx<", rf"pub\(crate\) fn cx_{g}\(", rf"impl {t}Cx<'_> \{{",
                     rf"pub\(crate\) struct {t}CxRef<", rf"pub\(crate\) fn cx_{g}_ref\("):
            text = _drop_item(text, head)
        mut, ref = uses.get(f"{t}Cx", {}), uses.get(f"{t}CxRef", {})
        shared = "shared" in mut
        members = {k: v for k, v in ref.items()} if shared else {}
        for k, v in mut.items():
            members[k] = members.get(k, False) or v
        order = lambda ms: [x for x in dict.fromkeys((g, *sorted(state_type))) if x in ms] + \
            (["hub"] if "hub" in ms else [])
        text = text.rstrip("\n") + "\n"
        for variant, ms, built in (("cx", members, f"{t}Cx" in uses), ("cx-ref", ref, f"{t}CxRef" in uses)):
            if not built:
                continue
            name, amp = (f"{t}Cx", "&'a mut ") if variant == "cx" else (f"{t}CxRef", "&'a ")
            cfg = lambda k: "" if ms[k] else (FIXTURES_ATTR + " " if k == "hub" else "#[cfg(test)] ")
            hub_ty = "HubMut<'a>" if variant == "cx" else "HubRef<'a>"
            fields = "".join(f"    {cfg(k)}pub(crate) {k}: {hub_ty if k == 'hub' else amp + state_type[k]},\n"
                             for k in order(ms))
            doc = (f"/// Capped group context (#1241 F3): the `{g}` state, sibling states and hub members its\n"
                   f"/// moved fns read, borrowed from disjoint WorldSession fields.\n") if variant == "cx" else \
                f"/// Shared counterpart of `{t}Cx` for `&self` methods.\n"
            text += f"\n{doc}pub(crate) struct {name}<'a> {{\n{fields}}}\n"
            bname = f"cx_{g}" + ("" if variant == "cx" else "_ref")
            if bname in calls:
                hub = ("HubMut { core: &mut s.core, catalogs: &s.catalogs, config: &s.config, "
                       f"{FIXTURES_ATTR} fixtures: &mut s.fixtures }}") if variant == "cx" else "hub_ref(s)"
                init = " ".join(f"{cfg(k)}{k}: {hub if k == 'hub' else ('&mut s.' if variant == 'cx' else '&s.') + k},"
                                for k in order(ms))
                arg = "&mut WorldSession" if variant == "cx" else "&WorldSession"
                text += (f"\n{'' if calls[bname] else '#[cfg(test)]' + chr(10)}pub(crate) fn {bname}(s: {arg}) -> "
                         f"{name}<'_> {{\n    {name} {{ {init} }}\n}}\n")
            if variant == "cx" and shared:
                init = " ".join(f"{'' if ref[k] else (FIXTURES_ATTR + ' ' if k == 'hub' else '#[cfg(test)] ')}{k}: "
                                f"{'self.hub.shared()' if k == 'hub' else '&*self.' + k}," for k in order(ref))
                text += (f"\n{'' if mut['shared'] else '#[cfg(test)]' + chr(10)}impl {name}<'_> {{\n"
                         f"    pub(crate) fn shared(&self) -> {t}CxRef<'_> {{\n        {t}CxRef {{ {init} }}\n    }}\n}}\n")
    return text


def remove_shim_fn(W, shim, name):
    """Delete one fn from a unit_tests shim file."""
    t = shim.read_text()
    code = W.blank_noncode(t)
    m = re.search(r"(?m)^[ \t]*(?:#\[[^\]]*\][ \t]*\n[ \t]*)*(?:pub[^\n]*?)?fn\s+" + name + r"\b", code)
    if m:
        close = W.matching_close(code, code.index("{", m.end())) + 1
        shim.write_text(t[:m.start()] + t[close:].lstrip("\n"))
