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

import argparse, collections, json, os, pathlib, re, subprocess, sys

REPO = pathlib.Path(__file__).resolve().parents[2]
IDENT = r"[A-Za-z_][A-Za-z0-9_]*"
HUB = {"core", "catalogs", "config", "fixtures"}
STATE_TYPE = {"core": "SessionCore", "catalogs": "SessionCatalogs", "config": "SessionWorldConfig",
              "lifecycle": "SessionLifecycleState", "loot": "LootState", "inventory": "InventoryState",
              "spell_state": "SessionSpellState", "social": "SessionSocialLimits", "instances": "InstanceState",
              "world_entities": "WorldEntitiesState", "visibility": "VisibilityState",
              "interaction": "InteractionState", "quest_state": "SessionQuestState", "view": "SessionWorldView",
              "phase": "SessionPhaseRail"}
HUB_TYPES = {"HubRef", "HubMut"}
SHELL = "shell"
# File-domain -> default group (F3-E design.md, section 1). Domains not listed are handler shells.
DOMAIN_MAP = {g: d.split() for g, d in {
    "inventory": "session/player_items session/money session/buyback_adapter session/void_storage_adapter "
                 "session/currency_adapter session/item_modifiers session/trade_adapter",
    "spell_state": "session/spell_state session/player_cast spell_acquisition session/effect_learning",
    "spell_effects": "session/spell_effects", "vehicles": "session/taxi",
    "world_entities": "session/world_entities session/gameobject_interaction",
    "movement": "session/movement session/movement_protocol",
    "quest_state": "session/quest session/quest_dialog session/quest_interaction handlers/quest quest",
    "pets": "session/pets session/pet_loading session/pet_dismissal session/battle_pet_adapter",
    "lifecycle": "session/lifecycle session/lifecycle_ops session/persistence session_persistence_capabilities "
                 "session/player_bootstrap",
    "combat": "session/combat session/player_vitals_adapter",
    "instances": "session/instances session/world_state session/map_admission",
    "progression": "session/progression session/rest_progression session/progression_adapters session/xp_grants "
                   "session/faction_reactions session/trait_configs profession",
    "social": "session/social session/social_requests session/chat",
    "battleground": "session/battleground_adapter",
    "visibility": "session/visibility session/deferred_visibility session/object_updates",
    "presentation": "session/player_presentation session/stand_state_adapter session/action_bar_adapter "
                    "session/cinematic_adapter session/raid_profile_values session/character_customization "
                    "session/appearance",
    "interaction": "session/npc_interaction session/support_features",
    "collections": "session/collection_adapter session/collections battle_pet_purchase",
    "loot": "session/loot handlers/loot", "identity": "player",
    "catalogs": "session/catalogs session/spell_pet_catalogs session/player_condition_values",
    "config": "session/runtime_policy_access",
    "core": "session/connection session/connection_identity session/admission session/time_synchronization "
            "session/player_binding session/player_registry_binding session/canonical_access session/publication "
            "session/mailbox session_commands",
}.items()}
DOMAIN = {d: g for g, ds in DOMAIN_MAP.items() for d in ds}

SELF_FIELD = re.compile(r"\bself\s*\.\s*(" + IDENT + r")\b(?!\s*(?:\(|::\s*<))")
SELF_CALL = re.compile(r"\bself\s*\.\s*(" + IDENT + r")(\s*(?:::\s*<[^;{}()]*>\s*)?)\(")
TYPE_PATH = re.compile(r"\b(?:Self|WorldSession)\s*::\s*(" + IDENT + r")")
BARE_SELF = re.compile(r"\bself\b(?!\s*(?:\.|::))")
ANY_CALL = re.compile(r"(?:\.|::)\s*(" + IDENT + r")\s*(?:::\s*<[^;{}()]*>\s*)?\(")
SESSION_CALL = re.compile(r"\b(" + IDENT + r")\s*\.\s*(" + IDENT + r")\s*(?:::\s*<[^;{}()]*>\s*)?\(")
IMPL_ANY = re.compile(r"(?m)^[ \t]*impl\s*(?:<[^{};]*?>)?\s*(?:" + IDENT + r"\s*::\s*)*(" + IDENT +
                      r")\b\s*(?:<[^{};]*>)?\s*\{")


PRECONDITIONS = ("unsupported parameter", "unexpected receiver", "whole-self", "`Self::`", "macro-generated", "returns a borrow",
                 "reads config")


class CodemodError(Exception):
    pass


def lib(root):
    """The coupling tool's lexer; a synthetic tree without tools/ uses this checkout's copy."""
    base = root / "tools/architecture"
    sys.path.insert(0, str(base if (base / "wow_world_coupling.py").exists() else REPO / "tools/architecture"))
    import wow_world_coupling as W
    return W


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
                if depth == 0 and ch == "}" and body[k] == "{":
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
    out, depth, seg, j = [], 0, open_i + 1, open_i + 1
    while j < close:
        ch = code[j]
        if ch == "{" and depth == 0:
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
        elif ch == ";" and depth == 0:
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


def scan(root):
    W = lib(root)
    src = root / "crates/wow-world/src"
    srcs = W.load_sources(src)
    raw = {r: t for r, t, _ in srcs}
    code = {r: c for r, _, c in srcs}
    groups = [f["name"] for f in W.parse_fields(raw["session/state.rs"])]
    fns, owned = [], collections.defaultdict(set)
    for rel, text, c in srcs:
        if W.is_test_path(rel):
            continue
        for h in IMPL_ANY.finditer(c):
            tname = h.group(1)
            if tname != "WorldSession" and tname not in STATE_TYPE.values() and tname not in HUB_TYPES:
                continue
            close = W.matching_close(c, h.end() - 1)
            pre = c[max(0, h.start() - 160):h.start()].rstrip()
            impl_attr = re.findall(r"#\s*\[[^\]]*\]\s*$", pre)
            for seg, m, bo, bc in item_segments(W, c, h.end() - 1, close):
                if tname != "WorldSession":
                    owned[m.group(1)].add((tname, rel))
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
                                whole_self=len(BARE_SELF.findall(body)), type_path=bool(TYPE_PATH.search(body)),
                                macro="$" in sig + body, sig_end=bo,
                                ret_borrow=bool(re.search(r"(&|'[a-z_])", ret_type(c, m.end(), bo))),
                                domain=W.domain_of(rel)))
    names = {f["name"] for f in fns}
    for f in fns:
        f["calls"] = [x for x in f["calls"] if x in names or x in owned]
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
    for p in (root / "crates/wow-world/unit_tests").rglob("*.rs"):
        for x in ANY_CALL.findall(W.blank_noncode(p.read_text(errors="replace"))):
            tests[x] += 1
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


def kind_of(f, classes):
    """(class, kind, target type) or a blocked reason string."""
    g = f["target"]
    if g not in STATE_TYPE:
        return "fixture/stateless group: context-owned kind not implemented"
    fields = set(f["acc"])
    if g == "core" and len(fields) == 1 and fields <= {"catalogs", "config"}:
        g = next(iter(fields))                               # a pure store accessor re-homed into core
        f["target"], f["store_rehomed"] = g, True
    own = fields <= {g}
    if own:
        cls, kind = "P", "state"
    elif fields <= HUB | {g} and g not in ("catalogs", "config"):
        cls = "C-hub"
        kind = ("hubref" if f["recv"] == "&self" else "hubmut") if g == "core" else "state-hub"
    else:
        return f"class C (non-hub fields {sorted(fields - HUB - {g})})"
    if cls not in classes:
        return f"class {cls} not requested"
    if f["recv"] not in ("&self", "&mut self"):
        return f"unexpected receiver {f['recv']}"
    if f["whole_self"]:
        return "whole-self use (`self` as a value)"
    if f["type_path"]:
        return "`Self::`/`WorldSession::` path in body"
    if f["macro"]:
        return "macro-generated fn"
    if kind in ("hubref", "hubmut") and f["ret_borrow"]:
        return "returns a borrow (would borrow a temporary hub view)"
    if kind in ("hubref", "hubmut", "state-hub") and "config" in fields and not f["file"].startswith("session/"):
        return "reads config outside crate::session"
    tname = {"hubref": "HubRef", "hubmut": "HubMut"}.get(kind, STATE_TYPE[g])
    return cls, kind, tname


def upgrade(f, kind, classes):
    """P -> C-hub when a callee needs the hub view (core: HubRef/HubMut, other groups: state-hub)."""
    if f.get("store_rehomed"):                               # it needs the hub after all: back to core
        f["target"] = "core"
    if kind != "state" or "C-hub" not in classes or f["target"] in ("catalogs", "config"):
        return None
    if f["target"] != "core":
        return "C-hub", "state-hub", STATE_TYPE[f["target"]]
    if f["ret_borrow"]:
        return "returns a borrow (would borrow a temporary hub view)"
    return ("C-hub", "hubref", "HubRef") if f["recv"] == "&self" else ("C-hub", "hubmut", "HubMut")


def owner_kind(tname):
    return {"HubRef": "hubref", "HubMut": "hubmut"}.get(tname, "state")


def call_prefix(caller, ctype, callee_type, callee_kind):
    """Receiver text replacing `self.` for a call from a moved fn, or None if impossible."""
    if caller == "state":
        if callee_type == ctype and callee_kind == "state":
            return "self."
        return None
    if caller == "state-hub":
        if callee_type == ctype:
            return "self." if callee_kind == "state" else ("self.", "hub")
        g = {v: k for k, v in STATE_TYPE.items()}.get(callee_type)
        if g in HUB:
            return f"hub.{g}."
        return "hub." if callee_type == "HubRef" else None
    g = {v: k for k, v in STATE_TYPE.items()}.get(callee_type)
    if g in HUB:
        return f"self.{g}."
    if callee_type == "HubRef":
        return "self." if caller == "hubref" else "self.shared()."
    if callee_type == "HubMut":
        return "self." if caller == "hubmut" else None
    return None


def plan(root, groups_wanted, classes):
    global HANDLERS, EXT
    W, src, raw, code, groups, fns, owned, HANDLERS, EXT, tests = scan(root)
    count, all_by = len(fns), {f["name"]: f for f in fns}
    thunks = {f["name"] for f in fns if f["name"] in owned}  # already-moved names on WorldSession are thunks
    live = [f for f in fns if f["name"] not in thunks]
    targets(live)
    by = {f["name"]: f for f in live}
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
            if ct is not None and x != n and \
                    call_prefix(kind, tname, ct, cand[x][1] if x in cand else owner_kind(ct)) is None:
                return True
        return False
    for final in (False, True):                               # monotone upgrades first, then blocking
        changed = True
        while changed:
            changed = False
            for n in list(cand):
                if not unreachable(n) and not (final and any(x not in cand and x not in loc for x in by[n]["calls"])):
                    continue
                up = upgrade(by[n], cand[n][1], classes) if unreachable(n) else None
                if isinstance(up, tuple):
                    cand[n], changed = up, True
                elif final:
                    missing = [x for x in by[n]["calls"] if x not in cand and x not in loc]
                    blocked[n] = f"callee {missing[0]} not moved" if missing else "callee on an unreachable owner"
                    del cand[n]
                    changed = True
    for n in list(cand):
        try:
            signature(by[n], code, raw, cand[n][1] == "state-hub")
        except CodemodError as e:
            blocked[n] = "unsupported parameter pattern: " + str(e).rsplit(": ", 1)[-1]
            del cand[n]
    # thunk rule: count callers outside moved code; moving uncalled code would only regroup lints
    fixed = []
    for rel, c in code.items():                              # bodies already on target impls are moved code
        for h in IMPL_ANY.finditer(c):
            if h.group(1) in STATE_TYPE.values() or h.group(1) in HUB_TYPES:
                fixed.append((rel, h.start(), W.matching_close(c, h.end() - 1)))
    fixed += [(f["file"], f["seg"], f["body_close"]) for f in fns if f["name"] in thunks]
    while True:
        moved_spans = collections.defaultdict(list)
        for rel, a, b in fixed + [(by[n]["file"], by[n]["seg"], by[n]["body_close"]) for n in cand]:
            moved_spans[rel].append((a, b))
        resident = collections.Counter()
        for rel, c in code.items():
            spans = moved_spans.get(rel, [])
            for m in ANY_CALL.finditer(c):
                if m.group(1) in cand and not any(a <= m.start() <= b for a, b in spans):
                    resident[m.group(1)] += 1
        callers = collections.Counter(x for n in cand for x in by[n]["calls"] if x != n)
        dead = [n for n in cand if not (resident[n] or callers[n] or tests[n] or EXT[n])]
        if not dead:
            break
        for n in dead:
            blocked[n] = "no callers anywhere (dead or unused pub API); left in place"
            del cand[n]
    moved_callers = collections.Counter(x for n in cand if not by[n]["cfg_test"] for x in by[n]["calls"] if x != n)
    rows = []
    for n, (cls, kind, tname) in sorted(cand.items(), key=lambda kv: (by[kv[0]]["file"], by[kv[0]]["line"])):
        f = by[n]
        keep = "H/ext" if EXT[n] else (f"{resident[n]} resident caller(s)" if resident[n] else
                                        "pub API, no production caller" if f["vis"] == "pub" and not f["cfg_test"]
                                        and not moved_callers[n] else "")
        shim = tests[n] if not keep and tests[n] else 0
        rows.append(dict(name=n, file=f["file"], line=f["line"], cls=cls, kind=kind, type=tname,
                         thunk=keep, shim=shim, recv=f["recv"], is_async=f["is_async"]))
    stale = [t for t in thunks if t not in HANDLERS and not resident_thunk(t, code, moved_spans, EXT)
             and not (all_by[t]["vis"] == "pub" and not all_by[t]["cfg_test"])]
    return dict(W=W, src=src, raw=raw, code=code, by=by, cand=cand, blocked=blocked, rows=rows, loc=loc,
                count=count, thunks=thunks, stale=stale, modpaths=module_paths(src, code), tests=tests,
                allfns={f["name"]: f for f in fns})


def resident_thunk(name, code, spans, ext):
    if ext[name]:
        return True
    return any(m.group(1) == name and not any(a <= m.start() <= b for a, b in spans.get(rel, []))
               for rel, c in code.items() for m in ANY_CALL.finditer(c))


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
        if not pm or pm.group(1) == "_" or (hub_kind and pm.group(1) in ("hub", "state")):
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
                (P["loc"][name], owner_kind(P["loc"][name]))
            pref = call_prefix(kind, tname, ct, ck)
            if isinstance(pref, tuple):
                paren = e + c[e:].index("(") + 1
                edits.append((s, e - len(name), pref[0]))
                edits.append((paren, paren, "hub, " if c[paren:paren + 1] != ")" else "hub"))
            else:
                edits.append((s, e - len(name), pref))
        elif kind in ("state", "state-hub") and name == g:
            dot = re.match(r"\s*\.", after)
            edits.append((s, e + (dot.end() if dot else 0), "self." if dot else "(*self)"))
        elif kind == "state-hub" and name in HUB:
            edits.append((s, s + len("self"), "hub"))
    text = r[a:b]
    for s, e, rep in sorted(edits, reverse=True):
        text = text[:s - a] + rep + text[e - a:]
    if kind == "state-hub":
        _, _, _, popen = signature(f, P["code"], P["raw"])
        k = popen - a
        recv_end = text.index(",", k) + 1 if "," in text[k:text.index(")", k)] else text.index(")", k)
        insert = " hub: crate::session::HubRef<'_>," if text[recv_end - 1] == "," else ", hub: crate::session::HubRef<'_>"
        text = text[:recv_end] + insert + text[recv_end:]
    return a, b, text


def line_start(raw, off):
    k = off
    while k < len(raw) and raw[k] in " \t\r\n":
        k += 1
    return raw.rfind("\n", 0, k) + 1


def thunk_text(P, f, kind, tname, indent="    ", vis=None):
    """WorldSession entry point delegating to the moved fn; `vis` overrides the visibility (shims)."""
    attrs, sig, names, _ = signature(f, P["code"], P["raw"])
    if vis is not None:
        sig = vis + " " + re.sub(r"^pub\s*(?:\([^)]*\))?\s*", "", sig)
    args, g, n = ", ".join(names), f["target"], f["name"]
    aw = ".await" if f["is_async"] else ""
    lines = {"state": [f"self.{g}.{n}({args}){aw}"],
             "hubref": [f"crate::session::hub_ref(self).{n}({args}){aw}"],
             "hubmut": [f"crate::session::hub_mut(self).{n}({args}){aw}"],
             "state-hub": [f"let (state, hub) = crate::session::split_{g}(self);",
                           f"state.{n}(hub{', ' if args else ''}{args}){aw}"]}[kind]
    head = "".join(indent + a.strip() + "\n" for a in attrs.splitlines() if a.strip())
    return head + indent + sig + " {\n" + "".join(indent + "    " + x + "\n" for x in lines) + indent + "}"


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
pub(crate) fn split_{g}(s: &mut WorldSession) -> (&mut {t}, HubRef<'_>) {{
    (&mut s.{g}, HubRef {{ core: &s.core, catalogs: &s.catalogs, config: &s.config, #[cfg(test)] fixtures: &s.fixtures }})
}}
'''


def ensure_prelude(src, kinds, groups_state_hub, outside_types):
    """Create hub.rs and the re-exports the moved code needs. Returns the paths written."""
    written = []
    state, mod = src / "session/state.rs", src / "session/mod.rs"
    st, md = state.read_text(), mod.read_text()
    hub = src / "session/state/hub.rs"
    if kinds & {"hubref", "hubmut", "state-hub"}:
        text = hub.read_text() if hub.exists() else HUB_RS
        for g in sorted(groups_state_hub):
            if f"fn split_{g}(" not in text:
                text += SPLIT_FN.format(g=g, t=STATE_TYPE[g])
        if not hub.exists() or hub.read_text() != text:
            hub.write_text(text)
            written.append(hub)
        line = "mod hub;\npub(crate) use hub::{HubMut, HubRef, hub_mut, hub_ref"
        if "mod hub;" not in st:
            st = st.replace("mod session_core;", line + "};\nmod session_core;", 1)
        exports = ["HubMut", "HubRef", "hub_mut", "hub_ref"] + [f"split_{g}" for g in sorted(groups_state_hub)]
        st = re.sub(r"pub\(crate\) use hub::\{[^}]*\};", "pub(crate) use hub::{" + ", ".join(exports) + "};", st)
        md_line = "pub(crate) use state::{" + ", ".join(exports) + "};"
        md = re.sub(r"pub\(crate\) use state::\{Hub[^}]*\};\n", "", md)
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


def abs_vis(vis, modpath):
    if vis.startswith("pub(in"):
        return "pub(in " + vis[len("pub(in"):].strip()
    if vis in ("pub", "pub(crate)"):
        return vis
    if vis == "pub(super)":
        return f"pub(in {modpath.rsplit('::', 1)[0]})"
    return f"pub(in {modpath})"


def apply_text(root, P):
    rows = P["rows"]
    if not rows and not P["stale"]:
        return []
    by, src = P["by"], P["src"]
    kinds = {r["kind"] for r in rows}
    sh_groups = {by[r["name"]]["target"] for r in rows if r["kind"] == "state-hub"}
    outside = {r["type"] for r in rows if r["kind"] in ("state", "state-hub") and not r["file"].startswith("session/")}
    for t in outside:
        if t not in ("SessionCore", "SessionCatalogs", "LootState", "WorldEntitiesState", "VisibilityState",
                     "InteractionState", "InstanceState", "SessionSpellState", "SessionQuestState", "SessionSocialLimits",
                     "SessionLifecycleState"):
            raise CodemodError(f"{t} is not pub(crate); a fn outside crate::session cannot name it")
    written = ensure_prelude(src, kinds, sh_groups, outside)
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
                if P["tests"][r["name"]]:
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
            head = "".join(x.strip() + "\n" for x in attrs) + f"impl {type_path(tname, rel)}{lt} {{\n"
            edits.append((close, close, "\n\n" + head + "\n".join(p.rstrip() + "\n" for p in parts) + "}"))
        for a, b, rep in sorted(edits, key=lambda e: (e[0], e[1]), reverse=True):
            raw = raw[:a] + rep + raw[b:]
        (src / rel).write_text(raw)
        written.append(src / rel)
    for rel, items in shims.items():
        stem = rel[:-3]
        shim = root / "crates/wow-world/unit_tests" / stem / "f3_shims.rs"
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


NO_METHOD = re.compile(r"no method named `(" + IDENT + r")` found for (?:mutable )?(?:reference|struct) `[^`]*WorldSession")
NO_FIELD = re.compile(r"(?:no field `(" + IDENT + r")`|attempted to take value of method `(" + IDENT + r")`) on type `[^`]*"
                      r"(SessionCore|SessionCatalogs|SessionWorldConfig|HubRef|HubMut)")


def compile_loop(root, group, manifest, max_rounds, log_dir):
    """Patch only what the text step caused: restore a removed thunk (E0599 on WorldSession) and drop a
    leftover hub segment (E0609/E0615 on a target type). Anything else stops the loop with a report."""
    rounds = []
    for i in range(max_rounds):
        rc, msgs = cargo_check(root, log_dir / f"f3-{group}-round-{i + 1}.jsonl")
        restore, drops, other = set(), [], []
        for m in msgs:
            code = (m.get("code") or {}).get("code")
            mm = NO_METHOD.match(m["message"])
            fm = NO_FIELD.match(m["message"])
            if code == "E0599" and mm and mm.group(1) in manifest["unthunked"]:
                restore.add(mm.group(1))
            elif code in ("E0609", "E0615") and fm and (fm.group(1) or fm.group(2)) in HUB:
                sp = next(s for s in m["spans"] if s["is_primary"])
                drops.append((sp["file_name"], sp["byte_start"], fm.group(1) or fm.group(2)))
            elif not m["message"].startswith("aborting"):
                other.append(f"{code}: {m['message']}")
        for name in restore:                                   # the tool removed this thunk: put it back
            manifest["unthunked"].remove(name)
            manifest["restored"].append(name)
        if restore:
            restore_thunks(root, manifest, restore)
        by_file = collections.defaultdict(list)
        for fn, start, name in drops:
            by_file[fn].append((start, name))
        for fn, items in by_file.items():
            p = root / fn
            data = p.read_bytes()
            for start, name in sorted(items, reverse=True):
                if data[start:start + len(name)] != name.encode():
                    raise CodemodError(f"{fn}:{start}: span is not `{name}`")
                dot = start - 1
                while dot > 0 and data[dot:dot + 1].isspace():
                    dot -= 1
                if data[dot:dot + 1] != b".":
                    raise CodemodError(f"{fn}:{start}: `{name}` is not a field segment")
                data = data[:dot] + data[start + len(name):]          # `x.core.y` -> `x.y`, `x.core` -> `x`
            p.write_bytes(data)
        rounds.append(len(restore) + len(drops))
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
        shim = root / "crates/wow-world/unit_tests" / rel[:-3] / "f3_shims.rs"
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
              "state-hub": f"self.{f['target']}. -> self.; self.<hub> -> hub.<hub>; +hub param"}[r["kind"]]
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
    try:
        P = plan(root, groups, classes)
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
            P = plan(root, groups, classes)
        mpath.write_text(json.dumps(manifest, indent=1))
        print(f"text step: {moved} fns moved, {len(set(written))} files written" if written
              else "text step: already applied (no-op)", file=sys.stderr)
        if not a.text_only and written:
            rounds = compile_loop(root, "-".join(sorted(groups)), manifest, a.max_rounds, log_dir)
            print(f"compiler loop: rounds {rounds}", file=sys.stderr)
    except CodemodError as e:
        print(f"f3 codemod: {e}", file=sys.stderr)
        return 1
    return 0


HANDLERS, EXT = set(), collections.Counter()
if __name__ == "__main__":
    sys.exit(main())
