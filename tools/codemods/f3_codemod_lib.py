#!/usr/bin/env python3
"""Text helpers for f3_move_methods.py (#1241 F3): lexing, signatures, paths and templates.

Standard library only; split out so the codemod stays a reviewable size.
"""
from __future__ import annotations

import json
import os
import pathlib
import re
import subprocess

IDENT = r"[A-Za-z_][A-Za-z0-9_]*"


def strip_cfg_test(body):
    """Blank `#[cfg(test)]` statements/blocks so production accesses remain."""
    out = list(body)
    for m in re.finditer(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", body):
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
        if rec.get("reason") == "compiler-message" and rec["message"].get("level") == "error":
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
    #[cfg(test)]
    pub(crate) fixtures: &'a SessionFixtures,
}

/// Mutable hub view for moved fns that take `&mut self` (core and fixtures writable).
pub(crate) struct HubMut<'a> {
    pub(crate) core: &'a mut SessionCore,
    pub(crate) catalogs: &'a SessionCatalogs,
    pub(in crate::session) config: &'a SessionWorldConfig,
    #[cfg(test)]
    pub(crate) fixtures: &'a mut SessionFixtures,
}

impl HubMut<'_> {
    pub(crate) fn shared(&self) -> HubRef<'_> {
        HubRef { core: &*self.core, catalogs: self.catalogs, config: self.config,
                 #[cfg(test)] fixtures: &*self.fixtures }
    }
}

/// Builds the shared view from disjoint WorldSession fields (free fn: not an `impl WorldSession` item).
pub(crate) fn hub_ref(s: &WorldSession) -> HubRef<'_> {
    HubRef { core: &s.core, catalogs: &s.catalogs, config: &s.config, #[cfg(test)] fixtures: &s.fixtures }
}

pub(crate) fn hub_mut(s: &mut WorldSession) -> HubMut<'_> {
    HubMut { core: &mut s.core, catalogs: &s.catalogs, config: &s.config, #[cfg(test)] fixtures: &mut s.fixtures }
}
'''


SPLIT_FN = '''
/// `&mut` group state plus the shared hub, borrowed from disjoint WorldSession fields.
pub(crate) fn split_{g}(s: &mut WorldSession) -> (&mut {t}, HubRef<'_>) {{
    (&mut s.{g}, HubRef {{ core: &s.core, catalogs: &s.catalogs, config: &s.config, #[cfg(test)] fixtures: &s.fixtures }})
}}
'''
SPLIT_REF_FN = '''
/// Shared group state plus the shared hub for `&self` methods.
pub(crate) fn split_{g}_ref(s: &WorldSession) -> (&{t}, HubRef<'_>) {{
    (&s.{g}, hub_ref(s))
}}
'''
