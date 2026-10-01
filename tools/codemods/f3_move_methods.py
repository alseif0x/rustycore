#!/usr/bin/env python3
"""#1241 F3: move `impl WorldSession` fns onto sub-states and hub views (move-only).

`plan --group g[,g]` prints what moves; `apply` moves it in place, then runs a compiler loop.
Kinds: P of a real-state group -> `impl <State>` (`self.<g>.` -> `self.`, bare `self.<g>` ->
`(*self)`); C-hub of core -> `impl HubRef<'_>` (`&self`) / `impl HubMut<'_>` (`&mut self`),
fields unchanged; C-hub of another real-state group -> `impl <State>` plus a `hub: HubRef<'_>`
parameter (`self.<hub>` -> `hub.<hub>`). Fixture-only groups are planned, not applied.
Moved code calls moved fns through their new owner, never a WorldSession thunk. A thunk stays
only for H/external API, while an unmoved caller remains, or for a `pub` fn with no production
caller. unit_tests callers get shims in `unit_tests/<src path>/f3_shims.rs`, mounted as a child
module of the source file. Each fn is cut byte-for-byte into a new impl block right after its
original block. Re-running is a no-op; stale thunks are removed. Standard library only.
"""
from __future__ import annotations

import argparse, bisect, collections, json, os, pathlib, re, subprocess, sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from f3_codemod_lib import (HUB_RS, SPLIT_FN, SPLIT_MUT_FN, SPLIT_REF_FN, abs_vis, cargo_check, item_segments,  # noqa: E402
                            git_ignored, line_start, module_paths, param_span, relocate_ignored_shims, ret_type,
                            shim_path, split_params, strip_cfg_test)
from f3_codemod_model import *  # noqa: E402,F401,F403  (groups, owner types, patterns, kind rules)

REPO = pathlib.Path(__file__).resolve().parents[2]
def lib(root):
    """The coupling tool's lexer; a synthetic tree without tools/ uses this checkout's copy."""
    base = root / "tools/architecture"
    sys.path.insert(0, str(base if (base / "wow_world_coupling.py").exists() else REPO / "tools/architecture"))
    import wow_world_coupling as W
    return W


def scan(root):
    OWNED_KIND.clear()
    W = lib(root)
    src = root / "crates/wow-world/src"
    srcs = W.load_sources(src)
    raw = {r: t for r, t, _ in srcs}
    code = {r: c for r, _, c in srcs}
    groups = [f["name"] for f in W.parse_fields(raw["session/state.rs"])]
    fns, owned, test_path_fns = [], collections.defaultdict(set), set()
    for rel, text, c in srcs:
        if W.is_test_path(rel):                              # test-path fns never move, but are callees
            test_path_fns.update(W.impl_methods(c))
            continue
        for h in IMPL_ANY.finditer(c):
            tname = h.group(1)
            if tname != "WorldSession" and tname not in OWNER_TYPES:
                continue
            close = W.matching_close(c, h.end() - 1)
            pre = c[max(0, h.start() - 160):h.start()].rstrip()
            impl_attr = re.findall(r"#\s*\[[^\]]*\]\s*$", pre)
            for seg, m, bo, bc in item_segments(W, c, h.end() - 1, close):
                if tname != "WorldSession":
                    owned[m.group(1)].add((tname, rel))
                    head = c[m.end():bo]                     # an already-moved group fn's hub parameter
                    if tname not in HUB_TYPES and re.search(r"\bhub\s*:\s*&\s*mut\b", head):
                        OWNED_KIND[m.group(1)] = "state-hubmut"
                    elif tname not in HUB_TYPES and re.search(r"\bhub\s*:", head):
                        OWNED_KIND[m.group(1)] = "state-hub"
                    continue
                sig, body = c[seg:bo], c[bo + 1:bc]
                attrs = c[seg:m.start()]
                vm = re.search(r"\bpub\s*(\([^)]*\))?\s*(?:const\s+|async\s+|unsafe\s+)*$", attrs)
                params = c[m.end():bo]
                recv = ("&mut self" if re.search(r"&\s*(?:'\w+\s+)?mut\s+self\b", params) else
                        "&self" if re.search(r"&\s*(?:'\w+\s+)?self\b", params) else
                        "self" if re.search(r"\(\s*(?:mut\s+)?self\b", params) else "none")
                pbody = strip_cfg_test(body)
                acc = collections.Counter(x for x in SELF_FIELD.findall(body) if x in groups)
                pacc = collections.Counter(x for x in SELF_FIELD.findall(pbody) if x in groups)
                cfg_test = bool(impl_attr and re.search(r"cfg\s*\(\s*test\s*\)", impl_attr[-1])) or bool(
                    re.search(r"cfg\s*\(\s*test\s*\)", attrs))
                fns.append(dict(name=m.group(1), file=rel, line=c.count("\n", 0, m.start()) + 1,
                                impl_start=h.start(), impl_attr=impl_attr, seg=seg, fn_at=m.start(), body_open=bo,
                                body_close=bc, vis=("pub" + (vm.group(1) or "").replace(" ", "")) if vm else "priv",
                                recv=recv, is_async=bool(re.search(r"\basync\s+(?:unsafe\s+)?$", attrs)),
                                cfg_test=cfg_test, acc=dict(acc), pacc=dict(pacc),
                                calls=sorted({x for x, _ in SELF_CALL.findall(body)}),
                                pcalls=sorted({x for x, _ in SELF_CALL.findall(pbody)}),
                                fx=sorted(set(FIXTURE_FIELD.findall(body))), guard_call=guard_call(body), whole_self=len(BARE_SELF.findall(body)), type_path=bool(TYPE_PATH.search(body)),
                                macro="$" in sig + body, sig_end=bo, writes_store=bool(STORE_WRITE.search(body)),
                                ret_borrow=bool(re.search(r"(&|'[a-z_])", ret_type(c, m.end(), bo))),
                                domain=W.domain_of(rel)))
    names = {f["name"] for f in fns}
    for f in fns:
        f["calls"] = [x for x in f["calls"] if x in names or x in owned or x in test_path_fns]
        f["pcalls"] = [x for x in f["pcalls"] if x in f["calls"]]
    handlers = set()
    for rel, c in code.items():
        for sm in re.finditer(r"inventory\s*::\s*submit\s*!\s*\{", c):
            blk = c[sm.end():W.matching_close(c, sm.end() - 1)]
            hm = re.search(r"handler\s*:", blk)
            if hm:
                handlers |= {x for _, x in SESSION_CALL.findall(blk[hm.end():]) if x in names}
    ext = collections.Counter()
    for p in (root / "crates").rglob("*.rs"):
        rp = p.relative_to(root).as_posix()
        if rp.startswith("crates/wow-world/") or "test" in rp:
            continue
        for recv_name, x in SESSION_CALL.findall(W.blank_noncode(p.read_text(errors="replace"))):
            if x in names and "session" in recv_name:
                ext[x] += 1
    tests = collections.Counter()
    PINNED.clear()
    PINNED_SELF.clear()
    for p in (root / "crates/wow-world/unit_tests").rglob("*.rs"):
        if p.name != "f3_shims.rs":                          # a shim delegating is not a test caller
            text = p.read_text(errors="replace")
            for x in ANY_CALL.findall(W.blank_noncode(text)):
                tests[x] += 1
            if "include_str!" in text:                       # source-text tests pin `name(args` literally
                for lit in re.findall(r'"((?:[^"\\\n]|\\.)*)"', text):
                    PINNED.update(re.findall(r"\b(" + IDENT + r")\(", lit))
                    PINNED_SELF.update(re.findall(r"\bself\.(" + IDENT + r")\(", lit))
    return W, src, raw, code, groups, fns, owned, handlers, ext, tests


def targets(fns):
    """Default target per fn plus the F3-E re-homing rules (a)-(c)."""
    by = {f["name"]: f for f in fns}
    for f in fns:
        f["H"] = f["name"] in HANDLERS or (EXT[f["name"]] > 0 and f["vis"] == "pub")
        f["target"] = DOMAIN.get(f["domain"], SHELL)
    def prod(f):
        return set(f["pacc"]) - {"fixtures"}
    hubc = {f["name"]: (f["cfg_test"] or prod(f) <= HUB) and not f["whole_self"] for f in fns}
    changed = True
    while changed:
        changed = False
        for f in fns:
            if hubc[f["name"]] and any(c in by and not hubc[c] for c in f["calls"]):
                hubc[f["name"]], changed = False, True
    def affinity(f):
        c = collections.Counter({g: v for g, v in f["pacc"].items() if g not in HUB})
        for x in f["calls"]:
            if x in by and by[x]["target"] not in HUB | {SHELL}:
                c[by[x]["target"]] += 1
        return c.most_common(1)[0][0] if c else None
    for _ in range(3):
        for f in fns:
            if f["target"] == SHELL and not f["H"]:
                f["target"] = affinity(f) or SHELL
    callers = collections.defaultdict(set)
    for f in fns:
        for x in f["calls"]:
            callers[x].add(f["target"])
    def pure(f):
        return set(f["acc"]) <= {f["target"]} and all(x not in by or by[x]["target"] == f["target"] for x in f["calls"])
    seeds = []
    for f in fns:
        if f["H"]:
            continue
        if hubc[f["name"]] and not f["cfg_test"] and f["target"] not in HUB and \
                len(callers[f["name"]] - {f["target"], SHELL}) >= 2:
            seeds.append(f["name"])
        if f["target"] in ("catalogs", "config") and not pure(f):
            if hubc[f["name"]]:
                seeds.append(f["name"])
            else:
                f["target"] = affinity(f) or "core"
        if f["target"] == "core" and not hubc[f["name"]]:
            f["target"] = affinity(f) or "core"
    seen = set()
    while seeds:
        n = seeds.pop()
        if n in seen or n not in by:
            continue
        seen.add(n)
        f = by[n]
        if f["H"] or (f["target"] in ("catalogs", "config") and pure(f)):
            continue
        f["target"] = "core"
        seeds.extend(f["calls"])


def plan(root, groups_wanted, classes, rehome=None):
    """`rehome` maps fn name -> group: an explicit, reviewed override of the derived target."""
    global HANDLERS, EXT
    W, src, raw, code, groups, fns, owned, HANDLERS, EXT, tests = scan(root)
    count, all_by = len(fns), {f["name"]: f for f in fns}
    thunks = {f["name"] for f in fns if f["name"] in owned}  # already-moved names on WorldSession are thunks
    live = [f for f in fns if f["name"] not in thunks]
    targets(live)
    by = {f["name"]: f for f in live}
    for name, group in (rehome or {}).items():
        if name in by:
            by[name]["target"] = group
        elif name not in owned:                              # already moved by an earlier pass: fine
            raise CodemodError(f"--rehome: {name} is not a WorldSession fn")
    loc = {n: sorted(o)[0][0] for n, o in owned.items()}
    cand, blocked = {}, {}
    for f in live:
        if f["target"] not in groups_wanted or f["H"]:
            continue
        k = kind_of(f, classes)
        (cand if isinstance(k, tuple) else blocked)[f["name"]] = k
    def unreachable(n):
        kind, tname = cand[n][1], cand[n][2]
        for x in by[n]["calls"]:
            ct = cand[x][2] if x in cand else loc.get(x)
            if ct is None or x == n:
                continue
            pref = call_prefix(kind, tname, ct, cand[x][1] if x in cand else OWNED_KIND.get(x, owner_kind(ct)),
                               by[x]["recv"] if x in by else "&self")
            if pref is None or ("config." in str(pref) and not by[n]["file"].startswith("session/")):
                return True                                   # `HubRef::config` is crate::session-only
        return False
    def settle():
        for final in (False, True):                           # monotone upgrades first, then blocking
            changed = True
            while changed:
                changed = False
                for n in list(cand):
                    missing = [x for x in by[n]["calls"] if x not in cand and x not in loc]
                    if not unreachable(n) and not (final and missing):
                        continue
                    up = upgrade(by[n], cand[n][1], classes) if unreachable(n) else None
                    if isinstance(up, tuple):
                        cand[n], changed = up, True
                    elif final:
                        blocked[n] = up or (f"callee {missing[0]} not moved" if missing
                                            else "callee on an unreachable owner")
                        del cand[n]
                        changed = True
    test_spans = collections.defaultdict(list)                # cfg(test) code: its callers get shims
    for f in fns:
        if f["cfg_test"]:
            test_spans[f["file"]].append((f["seg"], f["body_close"]))
    masks = {rel: strip_cfg_test(c) for rel, c in code.items() if "cfg" in c}

    def in_test(rel, pos):
        mask = masks.get(rel)
        return bool(mask and mask[pos] == " " and code[rel][pos] != " ") or \
            any(a <= pos <= b for a, b in test_spans.get(rel, []))

    dead_prod = prod_dead_names(W, code, fns, in_test)
    group_of = {f["name"]: (f["file"], f["impl_start"]) for f in fns}
    owner_spans = Spans((f["file"], f["seg"], f["body_close"], f["name"]) for f in fns)
    owner_impls = Spans((rel, h.start(), W.matching_close(c, h.end() - 1)) for rel, c in code.items()
                        for h in IMPL_ANY.finditer(c) if h.group(1) in OWNER_TYPES)
    sites = call_sites(code)
    prod_sites = collections.defaultdict(list)               # production WorldSession call sites -> caller fn
    any_sites = collections.Counter()                        # any context, excluding recursion
    for name, where in sites.items():
        for rel, pos in where:
            if owner_impls.find(rel, pos):
                continue
            hit = owner_spans.find(rel, pos)
            owner = hit[2] if hit else None
            if owner != name:
                any_sites[name] += 1
            if not in_test(rel, pos):
                prod_sites[name].append(owner)

    def lint_groups():
        """Dead-in-production fns move as whole lint groups; returns the names it demoted."""
        drop = set()
        uncalled = [n for n in cand if not (any_sites[n] or tests[n] or EXT[n] or n in PERMANENT_THUNKS
                                            or any(n in by[c]["calls"] for c in cand if c != n))]
        for n in uncalled:                                    # decided first: it stays, so its group does too
            blocked[n] = "no callers anywhere (dead or unused pub API); left in place"
            cand.pop(n)
        for n in cand:
            if n in dead_prod and by[n]["vis"] != "pub":
                peers = {f["name"] for f in fns if group_of[f["name"]] == group_of[n] and f["name"] in dead_prod
                         and f["vis"] != "pub"}
                if any(x not in cand or cand[x][2] != cand[n][2] for x in peers):
                    drop |= {x for x in peers if x in cand}
        for n in cand:                                        # its thunk would serve dead callers only
            sites = [o for o in prod_sites.get(n, []) if o != n and o not in thunks
                     and (o is None or o not in cand or o in dead_prod)]
            if sites and all(o in dead_prod for o in sites) and not EXT[n] and n not in PERMANENT_THUNKS:
                drop.add(n)
        for n in list(drop):                                  # a staying dead caller keeps its dead callees
            drop |= {x for x in by[n]["calls"] if x in cand and x in dead_prod and by[x]["vis"] != "pub"}
        for n in sorted(drop):                                # deterministic report order
            blocked[n] = ("dead in non-test builds; moving would split its dead-code lint group" if n in dead_prod
                          else "only dead-in-production callers remain; its thunk would be dead")
            cand.pop(n, None)
        return drop | set(uncalled)

    while True:                                               # a signature failure re-blocks its callers
        settle()
        bad = []
        for n in cand:                                        # a hub used only under cfg(test) is unused
            f = by[n]
            if cand[n][1] in SH and not f["cfg_test"] and not set(f["pacc"]) & HUB and not any(
                    x != n and (x not in cand or cand[x][2] != cand[n][2] or cand[x][1] != "state")
                    for x in f["pcalls"]):
                bad.append((n, "uses its hub only under cfg(test) (an unused parameter in non-test builds)"))
        for n in cand:                                        # a pinned call would gain a `hub` argument
            def changes(x):
                pref = call_prefix(cand[n][1], cand[n][2], *(cand[x][2:0:-1] if x in cand else (loc.get(x), OWNED_KIND.get(x, "state"))),
                                   by[x]["recv"] if x in by else "&self")
                return isinstance(pref, tuple) and x in PINNED or pref not in (None, "self.") and x in PINNED_SELF
            pinned = [x for x in by[n]["calls"] if changes(x)]
            if pinned:
                bad.append((n, f"source-text test pins the call `{pinned[0]}(`"))
                continue
            try:
                signature(by[n], code, raw, cand[n][1] in SH)
            except CodemodError as e:
                bad.append((n, "unsupported parameter pattern: " + str(e).rsplit(": ", 1)[-1]))
        for n, why in bad:
            blocked.setdefault(n, why)
            cand.pop(n, None)
        if not bad and not lint_groups():
            break
    # thunk rule: count callers outside moved code; moving uncalled code would only regroup lints
    fixed = []
    for rel, c in code.items():                              # bodies already on target impls are moved code
        for h in IMPL_ANY.finditer(c):
            if h.group(1) in OWNER_TYPES:
                fixed.append((rel, h.start(), W.matching_close(c, h.end() - 1)))
    fixed += [(f["file"], f["seg"], f["body_close"]) for f in fns if f["name"] in thunks]
    while True:
        moved = Spans(fixed + [(by[n]["file"], by[n]["seg"], by[n]["body_close"]) for n in cand])
        resident, tcallers = collections.Counter(), collections.Counter()
        for name in cand:
            for rel, pos in sites.get(name, ()):
                if not moved.find(rel, pos):
                    (tcallers if in_test(rel, pos) else resident)[name] += 1
        callers = collections.Counter(x for n in cand for x in by[n]["calls"] if x != n)
        dead = [n for n in cand if not (resident[n] or callers[n] or tests[n] or tcallers[n] or EXT[n])]
        if not dead:
            break
        for n in dead:
            blocked[n] = "no callers anywhere (dead or unused pub API); left in place"
            del cand[n]
    moved_callers = collections.Counter(x for n in cand if not by[n]["cfg_test"] for x in by[n]["calls"] if x != n)
    rows = []
    for n, (cls, kind, tname) in sorted(cand.items(), key=lambda kv: (by[kv[0]]["file"], by[kv[0]]["line"])):
        f = by[n]
        keep = "permanent inline thunk" if n in PERMANENT_THUNKS else "H/ext" if EXT[n] else (f"{resident[n]} resident caller(s)" if resident[n] else
                                        "pub API, no production caller" if f["vis"] == "pub" and not f["cfg_test"]
                                        and not moved_callers[n] else "")
        shim = 0 if keep else tests[n] + tcallers[n]
        rows.append(dict(name=n, file=f["file"], line=f["line"], cls=cls, kind=kind, type=tname,
                         thunk=keep, shim=shim, recv=f["recv"], is_async=f["is_async"]))
    stale = [t for t in thunks if t not in HANDLERS | PERMANENT_THUNKS
             and not resident_thunk(t, sites, moved, EXT, in_test)
             and not (all_by[t]["vis"] == "pub" and not all_by[t]["cfg_test"])]
    src_calls = collections.Counter({name: sum(1 for rel, pos in where if not moved.find(rel, pos))
                                     for name, where in sites.items()})
    stale_shims = {}                                          # shims whose last test caller moved
    for p in sorted((root / "crates/wow-world/unit_tests").rglob("f3_shims.rs")):
        for name in W.impl_methods(W.blank_noncode(p.read_text())):
            if not tests[name] and not src_calls[name] and name not in all_by:
                stale_shims[name] = p
    return dict(W=W, src=src, raw=raw, code=code, by=by, cand=cand, blocked=blocked, rows=rows, loc=loc,
                count=count, thunks=thunks, stale=stale, stale_shims=stale_shims, test_called={
                    t for t in stale if any(in_test(rel, pos) for rel, pos in sites.get(t, ()))}, modpaths=module_paths(src, code), tests=tests,
                allfns={f["name"]: f for f in fns})


def prod_dead_names(W, code, fns, in_test):
    """WorldSession fn names no live production code reaches (rustc's dead-code view, lexically)."""
    spans = collections.defaultdict(list)
    for f in fns:
        spans[f["file"]].append((f["seg"], f["body_close"], f["name"]))
    for rel, c in code.items():                              # moved fns carry their thunk's name
        for h in IMPL_ANY.finditer(c):
            if h.group(1) in OWNER_TYPES:
                for seg, m, _bo, bc in item_segments(W, c, h.end() - 1, W.matching_close(c, h.end() - 1)):
                    spans[rel].append((seg, bc, m.group(1)))
    names = {f["name"] for f in fns}
    live = {f["name"] for f in fns if f["name"] in HANDLERS or EXT[f["name"]] or f["name"] in PERMANENT_THUNKS
            or (f["vis"] == "pub" and not f["cfg_test"])}
    edges = collections.defaultdict(set)
    for rel, c in code.items():
        for m in WS_CALL.finditer(c):
            x = m.group(1)
            if x not in names or in_test(rel, m.start()):
                continue
            owner = next((n for a, b, n in spans.get(rel, []) if a <= m.start() <= b), None)
            if owner is None:
                live.add(x)
            elif owner != x:
                edges[owner].add(x)
    work = list(live)
    while work:
        for x in edges.get(work.pop(), ()):
            if x not in live:
                live.add(x)
                work.append(x)
    return {f["name"] for f in fns if not f["cfg_test"] and f["name"] not in live}


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


def call_sites(code):
    """name -> [(file, offset)] of every WorldSession-reaching call (WS_CALL), built once per plan."""
    sites = collections.defaultdict(list)
    for rel, c in code.items():
        for m in WS_CALL.finditer(c):
            sites[m.group(1)].append((rel, m.start()))
    return sites


def resident_thunk(name, sites, moved, ext, in_test):
    if ext[name]:
        return True
    return any(not moved.find(rel, pos) and not in_test(rel, pos) for rel, pos in sites.get(name, ()))


def signature(f, code, raw, hub_kind=False):
    """(attrs_raw, sig_raw, param_names, param_open) of a WorldSession fn."""
    c = code[f["file"]]
    m = re.compile(r"\bfn\s+" + re.escape(f["name"]) + r"\b").search(c, f["seg"])
    vis = re.search(r"(?:\bpub\s*(?:\([^)]*\))?\s*)?(?:const\s+|async\s+|unsafe\s+)*$", c[f["seg"]:m.start()])
    sig_start = f["seg"] + vis.start()
    k, close = param_span(c, m.end(), f["body_open"])
    params = split_params(c[k + 1:close])[1:]
    names = []
    for p in params:
        pm = re.match(r"(?:mut\s+)?(" + IDENT + r")\s*:", p)
        if not pm or pm.group(1) == "_" or (hub_kind and pm.group(1) == "hub"):
            raise CodemodError(f"{f['file']}:{f['line']} {f['name']}: unsupported parameter pattern {p!r}")
        names.append(pm.group(1))
    attrs = "".join(a + "\n" for a in re.findall(r"#\s*\[[^\]]*\]", raw[f["file"]][f["seg"]:sig_start])
                    if not a.lstrip("# [").startswith("doc"))
    return attrs, raw[f["file"]][sig_start:f["body_open"]].rstrip(), names, k


def type_path(tname, rel):
    if tname in HUB_TYPES:
        return f"crate::session::{tname}"
    return f"crate::session::state::{tname}" if rel.startswith("session/") else f"crate::session::{tname}"


def rewrite_body(P, f, kind, tname):
    """Rewritten raw text of [seg, body_close] for the moved fn."""
    rel, c, r = f["file"], P["code"][f["file"]], P["raw"][f["file"]]
    a, b = line_start(r, f["seg"]), f["body_close"] + 1
    g = f["target"]
    edits = []
    for m in re.finditer(r"\bself\s*\.\s*(" + IDENT + r")\b", c[a:b]):
        s, e, name = a + m.start(), a + m.end(), m.group(1)
        if s < f["body_open"]:
            continue
        after = c[e:e + 40]
        is_call = re.match(r"\s*(?:::\s*<[^;{}()]*>\s*)?\(", after)
        if is_call:
            if name not in P["cand"] and name not in P["loc"]:
                continue
            ct, ck = (P["cand"][name][2], P["cand"][name][1]) if name in P["cand"] else \
                (P["loc"][name], OWNED_KIND.get(name, owner_kind(P["loc"][name])))
            pref = call_prefix(kind, tname, ct, ck, P["by"][name]["recv"] if name in P["by"] else "&self")
            if isinstance(pref, tuple):
                paren = e + c[e:].index("(") + 1
                edits.append((s, e - len(name), pref[0]))
                edits.append((paren, paren, pref[1] + (", " if c[paren:paren + 1] != ")" else "")))
            else:
                edits.append((s, e - len(name), pref))
        elif kind == "state" and g in FIXTURE_TYPE and name == "fixtures":
            seg = re.match(r"\s*\.\s*" + g + r"\b(\s*\.)?", after)
            if seg:
                edits.append((s, e + seg.end(), "self." if seg.group(1) else "(*self)"))
        elif kind in ("state", *SH) and name == g:
            dot = re.match(r"\s*\.", after)
            edits.append((s, e + (dot.end() if dot else 0), "self." if dot else "(*self)"))
        elif kind in SH and name in HUB:
            edits.append((s, s + len("self"), "hub"))
    text = r[a:b]
    for s, e, rep in sorted(edits, reverse=True):
        text = text[:s - a] + rep + text[e - a:]
    if kind in SH:
        _, _, _, popen = signature(f, P["code"], P["raw"])
        k = popen - a
        recv_end = text.index(",", k) + 1 if "," in text[k:text.index(")", k)] else text.index(")", k)
        ty = "crate::session::HubRef<'_>" if kind == "state-hub" else "&mut crate::session::HubMut<'_>"
        insert = f" hub: {ty}," if text[recv_end - 1] == "," else f", hub: {ty}"
        text = text[:recv_end] + insert + text[recv_end:]
    return a, b, text


def thunk_text(P, f, kind, tname, indent="    ", vis=None):
    """WorldSession entry point delegating to the moved fn; `vis` overrides the visibility (shims)."""
    attrs, sig, names, _ = signature(f, P["code"], P["raw"])
    if vis is not None:
        sig = vis + " " + re.sub(r"^pub\s*(?:\([^)]*\))?\s*", "", sig)
    for name in names:                                       # the thunk only forwards: no `mut` binding
        sig = re.sub(r"\bmut\s+(" + name + r"\s*:)", r"\1", sig)
    args, g, n = ", ".join(names), f["target"], f["name"]
    aw = ".await" if f["is_async"] else ""
    st = next(b for b in ("state", "owner", "group_state", "f3_state") if b not in names)  # never shadow a param
    lines = {"state": [f"self.{'fixtures.' if g in FIXTURE_TYPE else ''}{g}.{n}({args}){aw}"],
             "hubref": [f"crate::session::hub_ref(self).{n}({args}){aw}"],
             "hubmut": [f"crate::session::hub_mut(self).{n}({args}){aw}"],
             "state-hub": [f"let ({st}, hub) = crate::session::split_{g}_ref(self);",
                           f"{st}.{n}(hub{', ' if args else ''}{args}){aw}"],
             "state-hubmut": [f"let ({st}, mut hub) = crate::session::split_{g}_mut(self);",
                              f"{st}.{n}(&mut hub{', ' if args else ''}{args}){aw}"]}[kind]
    head = "".join(indent + a.strip() + "\n" for a in attrs.splitlines() if a.strip())
    if n in PERMANENT_THUNKS and vis is None:
        head += indent + "#[inline]\n"
    return head + indent + sig + " {\n" + "".join(indent + "    " + x + "\n" for x in lines) + indent + "}"


def ensure_prelude(src, kinds, groups_state_hub, outside_types):
    """Create hub.rs and the re-exports the moved code needs. Returns the paths written."""
    written = []
    state, mod = src / "session/state.rs", src / "session/mod.rs"
    st, md = state.read_text(), mod.read_text()
    hub = src / "session/state/hub.rs"
    if kinds & {"hubref", "hubmut", *SH}:
        text = hub.read_text() if hub.exists() else HUB_RS
        for g, variant in sorted(groups_state_hub):
            if f"fn split_{g}{variant}(" not in text:
                text += {"_ref": SPLIT_REF_FN, "_mut": SPLIT_MUT_FN, "": SPLIT_FN}[variant].format(g=g, t=STATE_TYPE[g])
        if not hub.exists() or hub.read_text() != text:
            hub.write_text(text)
            written.append(hub)
        line = "mod hub;\npub(crate) use hub::{HubMut, HubRef, hub_mut, hub_ref"
        if "mod hub;" not in st:
            st = st.replace("mod session_core;", line + "};\nmod session_core;", 1)
        exports = ["HubMut", "HubRef"] + sorted(re.findall(r"(?m)^pub\(crate\) fn (" + IDENT + r")\(", text))
        st = re.sub(r"pub\(crate\) use hub::\{[^}]*\};", "pub(crate) use hub::{" + ", ".join(exports) + "};", st)
        md_line = "pub(crate) use state::{" + ", ".join(exports) + "};"
        md = re.sub(r"pub\(crate\) use state::\{\s*Hub[^}]*\};\n", "", md)
        md = md.replace("pub use state::WorldSession;", "pub use state::WorldSession;\n" + md_line, 1)
    for t in sorted(outside_types):
        st = re.sub(r"pub\(in crate::session\) use (\w+)::" + t + ";", r"pub(crate) use \1::" + t + ";", st)
        if f"pub(crate) use state::{t};" not in md:
            md = md.replace("pub use state::WorldSession;", f"pub use state::WorldSession;\npub(crate) use state::{t};", 1)
    for p, new in ((state, st), (mod, md)):
        if p.read_text() != new:
            p.write_text(new)
            written.append(p)
    return written


def apply_text(root, P):
    rows = P["rows"]
    relocated = relocate_ignored_shims(root, P["src"])
    if not rows and not P["stale"] and not P["stale_shims"]:
        return relocated
    written = relocated
    for name, shim in P["stale_shims"].items():               # remove shims nobody calls any more
        remove_shim_fn(shim, name)
        P["manifest"].setdefault("retired_shims", []).append(name)
        written.append(shim)
    by, src = P["by"], P["src"]
    kinds = {r["kind"] for r in rows}
    sh_groups = {(by[r["name"]]["target"], "_ref" if r["kind"] == "state-hub" else "_mut")
                 for r in rows if r["kind"] in SH}
    outside = {r["type"] for r in rows if r["kind"] in ("state", *SH) and not r["file"].startswith("session/")}
    for t in sorted(outside):                                # the owner type must be nameable crate-wide
        for decl in sorted((src / "session/state").glob("*.rs")):
            text = decl.read_text()
            if f"pub(in crate::session) struct {t} " in text:
                decl.write_text(text.replace(f"pub(in crate::session) struct {t} ", f"pub(crate) struct {t} ", 1))
                written.append(decl)
    written += ensure_prelude(src, kinds, sh_groups, outside)
    per_file = collections.defaultdict(list)
    for r in rows:
        per_file[r["file"]].append(r)
    for name in P["stale"]:
        per_file[P["allfns"][name]["file"]].append(dict(name=name, stale=True))
    shims = collections.defaultdict(list)
    for rel, rs in per_file.items():
        raw = P["raw"][rel]
        edits, blocks = [], collections.defaultdict(list)
        for r in rs:
            if r.get("stale"):                                # last unmoved caller gone: drop the thunk
                f = P["allfns"][r["name"]]
                a, b = line_start(raw, f["seg"]), f["body_close"] + 1
                edits.append((a, b + (raw[b:b + 1] == "\n"), ""))
                P["manifest"]["unthunked"].append(r["name"])
                P["manifest"]["thunk_text"][r["name"]] = (rel, raw[a:b], f["impl_attr"])
                if P["tests"][r["name"]] or r["name"] in P["test_called"]:
                    shims[rel].append((f, None, raw[a:b]))
                continue
            f = by[r["name"]]
            a, b, moved = rewrite_body(P, f, r["kind"], r["type"])
            thunk = thunk_text(P, f, r["kind"], r["type"]) if r["thunk"] else ""
            edits.append((a, b + (0 if thunk or raw[b:b + 1] != "\n" else 1), thunk))
            blocks[(f["impl_start"], r["type"])].append(moved)
            P["manifest"]["thunk_text"].setdefault(r["name"], (rel, thunk_text(P, f, r["kind"], r["type"]),
                                                                f["impl_attr"]))
            if not r["thunk"]:
                P["manifest"]["unthunked"].append(r["name"])
            if r["shim"]:
                shims[rel].append((f, r, None))
        for (impl_start, tname), parts in blocks.items():
            close = P["W"].matching_close(P["code"][rel], P["code"][rel].index("{", impl_start)) + 1
            attrs = next(f for f in by.values() if f["file"] == rel and f["impl_start"] == impl_start)["impl_attr"]
            lt = "<'_>" if tname in HUB_TYPES else ""
            attrs = list(attrs) + (["#[cfg(test)]"] if tname in FIXTURE_OF and not any(
                re.search(r"cfg\s*\(\s*test\s*\)", x) for x in attrs) else [])
            head = "".join(x.strip() + "\n" for x in attrs) + f"impl {type_path(tname, rel)}{lt} {{\n"
            edits.append((close, close, "\n\n" + head + "\n".join(p.rstrip() + "\n" for p in parts) + "}"))
        for a, b, rep in sorted(edits, key=lambda e: (e[0], e[1]), reverse=True):
            raw = raw[:a] + rep + raw[b:]
        (src / rel).write_text(raw)
        written.append(src / rel)
    for rel, items in shims.items():
        shim = shim_path(root, rel)
        text = shim.read_text() if shim.exists() else (
            "// Copyright (c) 2026 alseif0x\n// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html\n\n"
            "//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.\n\n"
            "#[allow(unused_imports)]\nuse super::*;\n\nimpl crate::session::WorldSession {\n}\n")
        add = ""
        for f, r, verbatim in items:
            if re.search(r"\bfn\s+" + f["name"] + r"\b", text):
                continue
            vis = abs_vis(f["vis"], P["modpaths"][rel])
            if verbatim:                                     # a stale thunk becomes the shim as-is
                add += re.sub(r"(?m)^(\s*)(?:pub\s*(?:\([^)]*\))?\s+)?((?:async\s+)?fn\s+" + f["name"] + r")",
                              r"\1" + vis + r" \2", verbatim, count=1).rstrip() + "\n"
            else:
                add += thunk_text(P, f, r["kind"], r["type"], vis=vis) + "\n"
        text = text[:text.rindex("}")] + add + "}\n"
        shim.parent.mkdir(parents=True, exist_ok=True)
        shim.write_text(text)
        srcfile = src / rel
        mount = os.path.relpath(shim, srcfile.parent)
        s = srcfile.read_text()
        if "mod f3_shims;" not in s:
            srcfile.write_text(s.rstrip("\n") + f'\n\n#[cfg(test)]\n#[path = "{mount}"]\nmod f3_shims;\n')
        written += [shim, srcfile]
    return written


NO_METHOD = re.compile(r"no method named `(" + IDENT + r")` found for (?:mutable )?(?:reference|struct) `[^`]*WorldSession")


def remove_shim_fn(shim, name):
    W = lib(REPO)
    t = shim.read_text()
    code = W.blank_noncode(t)
    m = re.search(r"(?m)^[ \t]*(?:#\[[^\]]*\][ \t]*\n[ \t]*)*(?:pub[^\n]*?)?fn\s+" + name + r"\b", code)
    if m:
        close = W.matching_close(code, code.index("{", m.end())) + 1
        shim.write_text(t[:m.start()] + t[close:].lstrip("\n"))


def compile_loop(root, group, manifest, max_rounds, log_dir):
    """Patch only what the text step caused: restore a thunk it removed (E0599 on WorldSession).
    Anything else stops the loop with a report; the text step itself rewrites every hub segment."""
    rounds = []
    for i in range(max_rounds):
        rc, msgs = cargo_check(root, log_dir / f"f3-{group}-round-{i + 1}.jsonl")
        restore, other, dead = set(), [], {}
        for m in msgs:
            code = (m.get("code") or {}).get("code")
            mm = NO_METHOD.match(m["message"])
            span = next((s for s in m["spans"] if s["is_primary"]), None)
            if m["level"] == "warning":                       # a shim whose last caller just moved
                if span and span["file_name"].endswith("f3_shims.rs") and "never used" in m["message"]:
                    dead.update({n: root / span["file_name"] for n in re.findall(r"`(" + IDENT + r")`", m["message"])})
            elif code == "E0599" and mm and mm.group(1) in manifest["unthunked"]:
                restore.add(mm.group(1))
            elif not m["message"].startswith("aborting"):
                other.append(f"{code}: {m['message']}")
        for name, shim in dead.items():
            remove_shim_fn(shim.resolve(), name)
            manifest.setdefault("retired_shims", []).append(name)
        for name in restore:                                   # the tool removed this thunk: put it back
            manifest["unthunked"].remove(name)
            manifest["restored"].append(name)
        if restore:
            restore_thunks(root, manifest, restore)
        rounds.append(len(restore) + len(dead))
        print(f"round {i + 1}: exit {rc}, {rounds[-1]} tool-caused spans patched", file=sys.stderr)
        if rounds[-1] == 0:
            if rc != 0:
                for line in other[:40]:
                    print("unhandled:", line, file=sys.stderr)
                raise CodemodError(f"cargo check fails with {len(other)} errors the tool did not cause")
            return rounds
    raise CodemodError("no fixed point within the round budget")


def restore_thunks(root, manifest, names):
    """Re-add WorldSession thunks for `names` from the manifest's recorded thunk texts."""
    src = root / "crates/wow-world/src"
    for name in names:
        rel, text, impl_attr = manifest["thunk_text"][name]
        p = src / rel
        attrs = "".join(x.strip() + "\n" for x in impl_attr)
        p.write_text(p.read_text().rstrip("\n") + f"\n\n{attrs}impl WorldSession {{\n{text.rstrip()}\n}}\n")
        shim = shim_path(root, rel)
        if shim.exists():
            W = lib(root)
            t = shim.read_text()
            m = re.search(r"\n[^\n]*\bfn\s+" + name + r"\b", W.blank_noncode(t))
            if m:
                close = W.matching_close(W.blank_noncode(t), t.index("{", m.end())) + 1
                shim.write_text(t[:m.start()] + t[close:])


def report(P, groups):
    rows, by = P["rows"], P["by"]
    kept = sum(1 for r in rows if r["thunk"])
    out = [f"groups {sorted(groups)}: {len(rows)} fns move, {kept} thunks kept, "
           f"{sum(1 for r in rows if r['shim'])} test shims, {len(P['blocked'])} blocked"]
    out.append(f"impl WorldSession fns: {P['count']} now -> {P['count'] - len(rows) + kept} after "
               f"(minus {len(P['stale'])} stale thunks removable)")
    by_kind = collections.Counter((r["cls"], r["type"]) for r in rows)
    out.append("by class/target: " + ", ".join(f"{c}->{t}: {n}" for (c, t), n in sorted(by_kind.items())))
    reasons = collections.Counter(re.sub(r"\b(callee) \S+", r"\1 <x>", v) for v in P["blocked"].values())
    out.append("blocked: " + "; ".join(f"{k}: {n}" for k, n in reasons.most_common()))
    out.append("")
    for r in rows:
        f = by[r["name"]]
        rw = {"state": f"self.{f['target']}. -> self.", "hubref": "fields unchanged; moved-callee receivers",
              "hubmut": "fields unchanged; HubRef callees via self.shared()",
              "state-hub": f"self.{f['target']}. -> self.; self.<hub> -> hub.<hub>; +HubRef param",
              "state-hubmut": f"self.{f['target']}. -> self.; self.<hub> -> hub.<hub>; +&mut HubMut param"}[r["kind"]]
        out.append(f"{r['file']}:{r['line']}\t{r['name']}\t{r['cls']}\timpl {r['type']}\t{rw}\t"
                   f"thunk={'yes (' + r['thunk'] + ')' if r['thunk'] else 'no'}\tshim={'yes' if r['shim'] else 'no'}")
    return "\n".join(out)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    for name in ("plan", "apply"):
        p = sub.add_parser(name)
        p.add_argument("--group", required=True)
        p.add_argument("--classes", default="P,C-hub")
        p.add_argument("--rehome", default="", help="name=group[,name=group]: reviewed target overrides")
        p.add_argument("--root", default=str(REPO))
        p.add_argument("--json", action="store_true")
        if name == "apply":
            p.add_argument("--text-only", action="store_true")
            p.add_argument("--max-rounds", type=int, default=6)
            p.add_argument("--demote-blocked", action="store_true",
                           help="move only unblocked fns (blocked ones stay; their callers re-check)")
    a = ap.parse_args(argv)
    root = pathlib.Path(a.root).resolve()
    groups, classes = set(a.group.split(",")), set(a.classes.split(","))
    rehome = dict(item.split("=", 1) for item in a.rehome.split(",") if item)
    try:
        P = plan(root, groups, classes, rehome)
        if a.cmd == "plan":
            print(json.dumps({"rows": P["rows"], "blocked": P["blocked"], "count": P["count"]}, indent=1)
                  if a.json else report(P, groups))
            return 0
        explicit = {n: v for n, v in P["blocked"].items() if v.startswith(PRECONDITIONS)}
        if explicit and not a.demote_blocked:
            for n, v in sorted(explicit.items())[:40]:
                print(f"precondition: {n}: {v}", file=sys.stderr)
            raise CodemodError(f"{len(explicit)} fns fail a precondition; nothing written (see --demote-blocked)")
        log_dir = root / "target/f3-codemod"
        log_dir.mkdir(parents=True, exist_ok=True)
        mpath = log_dir / f"{'-'.join(sorted(groups))}-manifest.json"
        manifest = json.loads(mpath.read_text()) if mpath.exists() else dict(
            groups=sorted(groups), classes=sorted(classes), restored=[], unthunked=[], thunk_text={})
        written, moved = [], 0
        for _ in range(4):                                   # a move can unblock re-homed callers: repeat
            P["manifest"] = manifest
            step = apply_text(root, P)
            if not step:
                break
            written, moved = written + step, moved + len(P["rows"])
            P = plan(root, groups, classes, rehome)
        mpath.write_text(json.dumps(manifest, indent=1))
        print(f"text step: {moved} fns moved, {len(set(written))} files written" if written
              else "text step: already applied (no-op)", file=sys.stderr)
        if not a.text_only and written:
            rounds = compile_loop(root, "-".join(sorted(groups)), manifest, a.max_rounds, log_dir)
            mpath.write_text(json.dumps(manifest, indent=1))
            print(f"compiler loop: rounds {rounds}", file=sys.stderr)
    except CodemodError as e:
        print(f"f3 codemod: {e}", file=sys.stderr)
        return 1
    return 0


HANDLERS, EXT, PINNED, PINNED_SELF, OWNED_KIND = set(), collections.Counter(), set(), set(), {}
if __name__ == "__main__":
    sys.exit(main())
