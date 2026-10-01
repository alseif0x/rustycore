#!/usr/bin/env python3
"""#1241 F3-I: inline WorldSession thunks at their call sites, then retire the thunks (move-only).

A thunk is a WorldSession fn whose whole body is one of the f3_move_methods delegation forms. Its
kind decides the policy (F4-E: limit the readability cost):

- `state`  `self.<g>.m(a)` / `self.fixtures.<g>.m(a)`   same length: always inlined;
- `hubref`/`hubmut`  `hub_ref|hub_mut(self).m(a)`       one-liner: always inlined;
- `cx`     `cx_<g>[_ref](self).m(a)`                     one-liner: always inlined;
- `state-hub`/`state-hubmut`  `let (s, hub) = split_<g>..(self); s.m([&mut ]hub, a)`: a 2-3 line
  block at each site, so only thunks with at most SPLIT_MAX call sites are inlined (the rest: F5).

Call sites are `<recv>.m(` with `recv` = `self`, a `*session*` binding or a field path ending in one,
in production code and in unit tests (privacy permitting; the compiler loop decides). Builder forms
take the receiver by reference: `self` as is, a binding declared `mut` in its fn as `&mut x` (`&x`
for shared forms), any other binding as is. When a `&mut` call's arguments read the receiver again,
the builder would hold the borrow while they run, so those arguments (and every non-trivial one) are
hoisted, in order, into `let`s ahead of the builder: `{ let a0 = ..; cx_g(self).m(a0, ..) }`.
Plain places and literals stay in place, and the builders have no side effects, so evaluation
order is unchanged. A split form nested in an expression that also reads the receiver is skipped;
a site inside another rewritten call is rewritten within it.

`apply` rewrites, runs `cargo check -p wow-world --all-targets` and reverts every site a diagnostic
points into (borrow, privacy, type), repeating to a fixed point; any other error stops the loop.
Then f3_move_methods' stale pass deletes each thunk without remaining callers (or keeps it as a
unit_tests shim) and its compiler loop restores a thunk an unseen caller still needs (E0599).
Never touched: registered handlers, external-API thunks, permanent thunks (`player_guid`), `pub`
API thunks and source-text-pinned names. Standard library only.
"""
from __future__ import annotations

import argparse, collections, json, os, pathlib, re, sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import f3_move_methods as F  # noqa: E402

IDENT = F.IDENT
SPLIT_MAX = 2
SINGLE = re.compile(r"^\s*(?P<recv>self\s*\.\s*(?:fixtures\s*\.\s*)?" + IDENT + r"|crate::session::(?:hub_ref|hub_mut|cx_" +
                    IDENT + r")\(self\))\s*\.\s*(?P<m>" + IDENT + r")\((?P<args>[^;]*)\)\s*(?P<aw>\.\s*await)?\s*$",
                    re.S)
SPLIT = re.compile(r"^\s*let \((?P<st>" + IDENT + r"), (?:mut )?hub\) =\s*crate::session::(?P<split>split_" + IDENT +
                   r")\(self\);\s*(?P=st)\s*\.\s*(?P<m>" + IDENT + r")\(\s*(?P<hub>&mut hub|hub)(?:\s*,\s*)?"
                   r"(?P<args>[^;]*)\)\s*(?P<aw>\.\s*await)?\s*$", re.S)
RECV = re.compile(r"(?:\b" + IDENT + r"\s*\.\s*)*\b(?:self|\w*session\w*)\s*\.\s*$")


def thunk_form(f, code):
    """Parse a WorldSession fn body as a delegation thunk (with its kind); None if it is anything else."""
    body = code[f["file"]][f["body_open"] + 1:f["body_close"]]
    m = SINGLE.match(body) or SPLIT.match(body)
    try:
        params = F.signature(f, code, code)[2]
    except F.CodemodError:
        return None
    if not m or [x.strip() for x in m.group("args").split(",") if x.strip()] != params:
        return None                                        # arguments not forwarded verbatim, in order
    m = SINGLE.match(body)
    if m:
        recv = re.sub(r"\s+", "", m.group("recv"))
        kind = ("state" if recv.startswith("self.") else "hubref" if "hub_ref(" in recv else
                "hubmut" if "hub_mut(" in recv else "cx")
        shared = kind == "hubref" or recv.endswith("_ref(self)")
        return dict(kind=kind, shape="single", recv=recv, m=m.group("m"), aw=bool(m.group("aw")), shared=shared)
    m = SPLIT.match(body)
    if m:
        shared = not m.group("hub").startswith("&mut")
        return dict(kind="state-hub" if shared else "state-hubmut", shape="split", split=m.group("split"),
                    m=m.group("m"), hub=m.group("hub"), aw=bool(m.group("aw")), shared=shared)
    return None


def call_extent(code, open_i):
    """(end, awaited) of the call whose `(` is at open_i; end covers a trailing `.await`."""
    depth = 0
    for j in range(open_i, len(code)):
        depth += {"(": 1, ")": -1}.get(code[j], 0)
        if depth == 0:
            aw = re.match(r"\s*\.\s*await\b", code[j + 1:j + 12])
            return j + 1 + (aw.end() if aw else 0), bool(aw)
    raise ValueError("unbalanced call")


def split_args(code, open_i, close_i):
    """Top-level argument spans (start, end) between the call parens."""
    out, depth, a = [], 0, open_i + 1
    for j in range(open_i + 1, close_i):
        ch = code[j]
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth -= 1
        elif ch == "," and depth == 0:
            out.append((a, j))
            a = j + 1
    if code[a:close_i].strip():
        out.append((a, close_i))
    return out


def statement_span(code, start, end):
    """The enclosing statement: back to `;`/`{`/`}` and forward to `;`/`}` at the same nesting."""
    depth, a = 0, start
    while a > 0:
        ch = code[a - 1]
        if ch in ")]":
            depth += 1
        elif ch in "([":
            if depth == 0:
                break
            depth -= 1
        elif ch in ";{}" and depth == 0:
            break
        a -= 1
    depth, b = 0, end
    while b < len(code):
        ch = code[b]
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            if depth == 0:
                break
            depth -= 1
        elif ch == ";" and depth == 0:
            break
        b += 1
    return a, b


def builder_arg(code, pos, recv, shared):
    """How a builder form takes the call's receiver (see the module doc)."""
    if recv == "self":
        return "self"
    if "." in recv:
        return ("&" if shared else "&mut ") + recv
    fn_start = max(code.rfind("fn ", 0, pos), 0)
    is_mut = re.search(r"\bmut\s+" + re.escape(recv) + r"\b", code[fn_start:pos])
    return "&" + recv if shared else "&mut " + recv if is_mut else recv


TRIVIAL = re.compile(r"\s*(?:&\s*(?:mut\s+)?)?(?:" + IDENT + r"(?:\s*\.\s*" + IDENT + r")*|-?[0-9][0-9_.a-z]*|"
                     r"true|false|'(?:[^'\\]|\\.)+')\s*")


def rewrite(form, target, args_text, args, aw, hoist):
    """Replacement text for one call site: `target` is the receiver (state) or builder argument.
    Without hoisting the argument text is kept verbatim. With it, each argument that reads the
    receiver or is not a plain place/literal is bound, in order, to `a<i>` ahead of the builder;
    plain places/literals stay in place (reading them later cannot observe the hoisted calls)."""
    tail = ".await" if aw else ""
    pre = ""
    if hoist:
        names = []
        for i, (arg, bind) in enumerate(zip(args, hoist)):
            if bind:
                pre += f"let a{i} = {arg.strip()}; "
            names.append(f"a{i}" if bind else arg.strip())
        args_text = ", ".join(names)
    if form["shape"] == "single":
        call = f"{form['recv'].replace('self', target, 1)}.{form['m']}({args_text}){tail}"
        return f"{{ {pre}{call} }}" if hoist else call
    hub = "h" if form["shared"] else "&mut h"
    sep = ", " if args_text.strip() else ""
    return (f"{{ {pre}let (s, {'' if form['shared'] else 'mut '}h) = crate::session::{form['split']}({target}); "
            f"s.{form['m']}({hub}{sep}{args_text}){tail} }}")


def load(root, W):
    """{rel: (raw, blanked)} for src and unit_tests files, rel relative to crates/wow-world."""
    crate = root / "crates/wow-world"
    out = {}
    for base in ("src", "unit_tests"):
        for p in sorted((crate / base).rglob("*.rs")):
            if p.name != "f3_shims.rs":
                text = p.read_text()
                out[p.relative_to(crate).as_posix()] = (text, W.blank_noncode(text))
    return out


def find_sites(files, names, skip_spans):
    """name -> [(rel, receiver start, call paren offset)] for every `<self|..session..>.name(` call."""
    found = collections.defaultdict(list)
    if not names:
        return found
    pattern = re.compile(r"\.\s*(" + "|".join(sorted(map(re.escape, names), key=len, reverse=True)) + r")\s*\(")
    for rel, (_text, c) in files.items():
        for m in pattern.finditer(c):
            if any(a <= m.start() <= b for a, b in skip_spans.get(rel, ())):
                continue
            r = RECV.search(c, max(0, m.start() - 160), m.start() + 1)
            if r and r.end() == m.start() + 1 and c[r.start() - 1:r.start()] not in (".", ":"):
                found[m.group(1)].append((rel, r.start(), m.end() - 1))   # other receivers: not a session
    return found


def session_private_fields(raw):
    """WorldSession fields visible only inside crate::session (`pub(in crate::session)`)."""
    body = raw["session/state.rs"].split("pub struct WorldSession", 1)[1]
    return set(re.findall(r"pub\(in crate::session\)\s+(" + IDENT + r")\s*:", body[:body.index("\n}")]))


def plan_site(form, text, c, rstart, open_i, rel="src/session/x.rs", private=()):
    """(site dict, None) or (None, skip reason) for one call."""
    field = re.match(r"self\.(?:fixtures\.)?(" + IDENT + ")", form.get("recv", ""))
    if form["kind"] == "state" and field and field.group(1) in private and \
            not rel.startswith(("src/session/", "unit_tests/session/")):
        return None, f"`{field.group(1)}` is private to crate::session"
    recv = re.sub(r"\s+", "", c[rstart:c.rindex(".", rstart, open_i)])
    end, aw = call_extent(c, open_i)
    spans = split_args(c, open_i, c.rindex(")", open_i, end))
    root_ident = recv.split(".")[0]
    word = re.compile(r"\b" + re.escape(root_ident) + r"\b")
    hoist = None
    if form["kind"] != "state" and not form["shared"] and any(word.search(c[a:b]) for a, b in spans):
        if any(re.search(r"\ba[0-9]+\b", c[a:b]) for a, b in spans):
            return None, "argument names collide with the hoisted bindings"
        if any(re.match(r"\s*(?:move\s*)?\|", c[a:b]) for a, b in spans):
            return None, "closure argument cannot be hoisted"
        if c[spans[-1][1] if spans else open_i + 1:c.rindex(")", open_i, end)].strip(" \t\n,") or \
                any(text[a:b].strip() != c[a:b].strip() for a, b in spans):
            return None, "comment in hoisted arguments"
        hoist = [bool(word.search(c[a:b]) or not TRIVIAL.fullmatch(c[a:b])) for a, b in spans]
    a, b = statement_span(c, rstart, end)
    statement_level = re.fullmatch(r"\s*(?:let\s+(?:mut\s+)?\w+(?:\s*:\s*[^=;]+)?\s*=\s*|return\s+)?",
                                   c[a:rstart]) is not None and not c[end:b].strip(" \t\n?")
    if form["shape"] == "split" and not statement_level and word.search(c[a:rstart] + c[end:b]):
        return None, "split form nested beside another receiver use"
    target = recv if form["kind"] == "state" else builder_arg(c, rstart, recv, form["shared"])
    close = c.rindex(")", open_i, end)
    new = rewrite(form, target, text[open_i + 1:close], [text[x:y] for x, y in spans], aw, hoist)
    if new.startswith("{") and (not c[a:rstart].strip() and c[end:end + 1] in ".?[" or
                                re.match(r"\s*else\b", c[end:end + 80])):
        new = f"({new})"                     # a leading block parses as a statement; `let .. = {..} else` is invalid
    return dict(start=rstart, end=end, old=text[rstart:end], new=new, hoist=bool(hoist)), None


def fits(text, site, width=100):
    """A unit_tests rewrite that rustfmt keeps on its line: one line, still within the width."""
    if "\n" in site["old"] or "\n" in site["new"]:
        return False
    a = text.rfind("\n", 0, site["start"]) + 1
    b = text.find("\n", site["end"])
    return len(text[a:b]) + len(site["new"]) - len(site["old"]) <= width


def plan(root, kinds=None, test_kinds=None):
    W, src, raw, code, groups, fns, owned, handlers, ext, tests = F.scan(root)
    F.HANDLERS, F.EXT = handlers, ext
    files = load(root, W)
    thunk_fns = {f["name"]: f for f in fns if f["name"] in owned}
    skip_spans = collections.defaultdict(list)               # moved code and the thunks themselves
    for rel, c in code.items():
        for h in F.IMPL_ANY.finditer(c):
            if F.is_owner_type(h.group(1)):
                skip_spans["src/" + rel].append((h.start(), W.matching_close(c, h.end() - 1)))
    for f in thunk_fns.values():
        skip_spans["src/" + f["file"]].append((f["seg"], f["body_close"]))
    found = find_sites(files, thunk_fns, skip_spans)
    pinned = F.PINNED | F.PINNED_SELF | F.PINNED_FN
    private = session_private_fields(raw)
    rows, summary = [], collections.Counter()
    for name, f in sorted(thunk_fns.items()):
        form = thunk_form(f, code)
        n_sites = len(found.get(name, ()))
        reason = ("registered handler" if name in handlers else "external API" if ext[name] else
                  "permanent thunk" if name in F.PERMANENT_THUNKS else "source-text pinned" if name in pinned else
                  "pub API" if f["vis"] == "pub" and not f["cfg_test"] else
                  "body is not a delegation thunk" if not form else
                  f"split kind with {n_sites} call sites (F5)" if form["shape"] == "split" and n_sites > SPLIT_MAX
                  else None)
        if not reason and kinds is not None and form["kind"] not in kinds:
            reason = "kind not selected"
        plan_sites, skipped = [], []
        for rel, rstart, open_i in ([] if reason else found.get(name, ())):
            if rel.startswith("unit_tests/") and test_kinds is not None and form["kind"] not in test_kinds:
                continue                                       # left to the thunk's unit_tests shim
            site, why = plan_site(form, *files[rel], rstart, open_i, rel, private)
            if why:
                skipped.append((rel, rstart, why))
            else:
                plan_sites.append(dict(site, file=rel, test=rel.startswith("unit_tests/")))
        tests_here = [x for x in plan_sites if x["test"]]
        if tests_here and not all(fits(files[x["file"]][0], x) for x in tests_here):
            plan_sites = [x for x in plan_sites if not x["test"]]   # a rewrap costs more than the shim
        status = reason or ("inline all" if plan_sites and not skipped else "partial" if plan_sites else
                            "no rewritable site")
        summary[status.split(" with ")[0]] += 1
        rows.append(dict(name=name, file=f["file"], kind=form["kind"] if form else None, status=status,
                         sites=plan_sites, skipped=skipped, test_callers=tests[name]))
    nest_sites(rows)
    return dict(rows=rows, summary=summary, files=files)


def nest_sites(rows):
    """A site inside another site's call (`self.a(self.b(x))`) is rewritten inside the outer
    replacement and travels with it (one ledger entry, reverted together)."""
    by_file = collections.defaultdict(list)
    for row in rows:
        for site in row["sites"]:
            by_file[site["file"]].append((row, site))
    for items in by_file.values():
        items.sort(key=lambda rs: rs[1]["end"] - rs[1]["start"])
        for i, (row, site) in enumerate(items):
            outer = next((o for _r, o in items[i + 1:] if o["start"] <= site["start"] and site["end"] <= o["end"]
                          and o is not site), None)
            if outer is None:
                continue
            row["sites"].remove(site)
            if site["old"] in outer["new"]:
                outer["new"] = outer["new"].replace(site["old"], site["new"], 1)
                outer.setdefault("nested", []).append(row["name"])
            else:
                row["skipped"].append((site["file"], site["start"], "inside another rewritten call"))


def apply_sites(root, rows):
    """Rewrite every planned site (back to front per file); returns the site ledger with new offsets."""
    by_file = collections.defaultdict(list)
    for row in rows:
        for site in row["sites"]:
            by_file[site["file"]].append(dict(site, thunk=row["name"], kind=row["kind"]))
    ledger = []
    crate = root / "crates/wow-world"
    for rel, items in by_file.items():
        text = (crate / rel).read_text()
        for site in sorted(items, key=lambda s: s["start"], reverse=True):
            assert text[site["start"]:site["end"]] == site["old"], (rel, site["start"])
            text = text[:site["start"]] + site["new"] + text[site["end"]:]
        (crate / rel).write_text(text)
        shift = 0
        for site in sorted(items, key=lambda s: s["start"]):
            start = site["start"] + shift
            ledger.append(dict(site, start=start, end=start + len(site["new"])))
            shift += len(site["new"]) - len(site["old"])
    return ledger


def revert(root, ledger, rejected):
    """Put the original call text back at each rejected site; later offsets in its file shift."""
    crate = root / "crates/wow-world"
    for rel in {s["file"] for s in rejected}:
        text = (crate / rel).read_text()
        for site in sorted((s for s in rejected if s["file"] == rel), key=lambda s: s["start"], reverse=True):
            assert text[site["start"]:site["end"]] == site["new"]
            text = text[:site["start"]] + site["old"] + text[site["end"]:]
            delta = len(site["old"]) - len(site["new"])
            for other in ledger:
                if other["file"] == rel and other["start"] > site["start"]:
                    other["start"] += delta
                    other["end"] += delta
        (crate / rel).write_text(text)
    ids = {id(s) for s in rejected}
    ledger[:] = [s for s in ledger if id(s) not in ids]


def char_offset(root, rel, byte, cache):
    """rustc spans are UTF-8 byte offsets; the ledger uses str offsets."""
    if rel not in cache:
        cache[rel] = (root / "crates/wow-world" / rel).read_bytes()
    return len(cache[rel][:byte].decode("utf-8", errors="ignore"))


def compile_loop(root, ledger, max_rounds, log_dir):
    """Revert every site a diagnostic points into; an error at no rewritten site stops the loop."""
    reverted = collections.Counter()
    for i in range(max_rounds):
        rc, msgs = F.cargo_check(root, log_dir / f"f3i-round-{i + 1}.jsonl")
        rejected, other, cache = {}, [], {}
        by_file = collections.defaultdict(list)
        for s in ledger:
            by_file[s["file"]].append(s)
        for m in msgs:
            if m["level"] != "error":
                continue
            code = (m.get("code") or {}).get("code") or "other"
            hits = []
            for span in m["spans"]:
                rel = os.path.normpath(span["file_name"]).split("crates/wow-world/", 1)[-1]
                if rel in by_file:
                    a, b = (char_offset(root, rel, span[k], cache) for k in ("byte_start", "byte_end"))
                    hits += [s for s in by_file[rel] if s["start"] <= b and a <= s["end"]]
            for hit in hits:
                rejected[id(hit)] = hit
                hit.setdefault("codes", set()).add(code)
            if not hits and not m["message"].startswith("aborting"):
                other.append(f"{code}: {m['message']}")
        for s in rejected.values():
            reverted[",".join(sorted(s["codes"]))] += 1
        if rejected:
            revert(root, ledger, list(rejected.values()))
        print(f"round {i + 1}: exit {rc}, {len(rejected)} sites reverted", file=sys.stderr)
        if not rejected:
            if rc != 0:
                for line in other[:40]:
                    print("unhandled:", line, file=sys.stderr)
                raise F.CodemodError(f"cargo check fails with {len(other)} errors at no rewritten site")
            return ledger, reverted
    raise F.CodemodError("no fixed point within the round budget")


def remaining_test_calls(root, W, names, code):
    """unit_tests calls that may still reach a WorldSession thunk: `.m(` except on a WorldSession
    field (`x.<field>.m(`), a builder result (`cx_g(..).m(`) or a split state (`s.m(h`)."""
    state = (root / "crates/wow-world/src/session/state.rs").read_text()
    fields = {f["name"] for f in W.parse_fields(state)} | set(F.FIXTURE_TYPE) | {"hub"}
    out = collections.Counter()
    if not names:
        return out
    pattern = re.compile(r"\.\s*(" + "|".join(sorted(map(re.escape, names), key=len, reverse=True)) + r")\s*\(")
    for rel, (_text, c) in load(root, W).items():
        if not rel.startswith("unit_tests/"):
            continue
        for m in pattern.finditer(c):
            before = c[max(0, m.start() - 80):m.start()].rstrip()
            last = re.search(r"(" + IDENT + r")\s*$", before)
            if before.endswith(")") or last and last.group(1) in fields or \
                    last and last.group(1) == "s" and re.match(r"\s*(?:&mut\s+)?h\b", c[m.end():m.end() + 12]):
                continue
            out[m.group(1)] += 1
    return out


def retire(root, text_only, max_rounds, log_dir):
    """f3_move_methods' stale pass with no group: delete (or shim) thunks nobody calls any more."""
    P = F.plan(root, set(), {"P"})
    P["tests"] = remaining_test_calls(root, P["W"], P["stale"], P["code"])
    P["manifest"] = manifest = dict(groups=[], classes=["P"], restored=[], unthunked=[], thunk_text={})
    if F.apply_text(root, P) and not text_only:
        F.compile_loop(root, "f3i-retire", manifest, max_rounds, log_dir)
    (log_dir / "f3i-retire-manifest.json").write_text(json.dumps(manifest, indent=1))
    return manifest


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    for name in ("plan", "apply"):
        p = sub.add_parser(name)
        p.add_argument("--root", default=str(F.REPO))
        p.add_argument("--json", action="store_true")
        p.add_argument("--kinds", default="state,hubref,hubmut,cx,state-hub,state-hubmut",
                       help="thunk kinds to inline (a split by kind lands the same-length kinds first)")
        p.add_argument("--test-kinds", default="state,hubref,hubmut,cx,state-hub,state-hubmut",
                       help="kinds whose unit_tests sites may be rewritten; a thunk's test sites are rewritten "
                            "only when every one stays on its line (test growth counts against R1 net-move), "
                            "otherwise the thunk keeps a unit_tests shim")
        if name == "apply":
            p.add_argument("--text-only", action="store_true")
            p.add_argument("--max-rounds", type=int, default=12)
    a = ap.parse_args(argv)
    root = pathlib.Path(a.root).resolve()
    P = plan(root, set(a.kinds.split(",")), set(filter(None, a.test_kinds.split(","))))
    try:
        if a.cmd == "plan":
            if a.json:
                print(json.dumps(P["rows"], indent=1))
            else:
                sites = [s for r in P["rows"] for s in r["sites"]]
                print(f"thunks {len(P['rows'])}: " + ", ".join(f"{k} {v}" for k, v in P["summary"].most_common()))
                print(f"rewritable sites {len(sites)} ({sum(s['test'] for s in sites)} in unit tests, "
                      f"{sum(s['hoist'] for s in sites)} hoisted); "
                      f"skipped {sum(len(r['skipped']) for r in P['rows'])}")
            return 0
        log_dir = root / "target/f3-codemod"
        log_dir.mkdir(parents=True, exist_ok=True)
        ledger = apply_sites(root, [r for r in P["rows"] if r["sites"]])
        reverted = collections.Counter()
        if not a.text_only:
            ledger, reverted = compile_loop(root, ledger, a.max_rounds, log_dir)
        manifest = retire(root, a.text_only, a.max_rounds, log_dir)
        (log_dir / "f3i-ledger.json").write_text(json.dumps(
            dict(sites=[{k: v for k, v in s.items() if k != "codes"} for s in ledger], reverted=reverted,
                 retired=manifest["unthunked"], restored=manifest["restored"]), indent=1))
        print(f"inlined {len(ledger)} sites; reverted {sum(reverted.values())} {dict(reverted)}; "
              f"retired {len(manifest['unthunked'])} thunks, restored {len(manifest['restored'])}", file=sys.stderr)
    except F.CodemodError as e:
        print(f"f3 inline: {e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
