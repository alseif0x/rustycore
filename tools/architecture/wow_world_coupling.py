#!/usr/bin/env python3
"""WorldSession coupling map and R5 metric for crates/wow-world (#1241 F0).

A lexical scan, not a compiler query: comments and literals are blanked before any
brace matching or pattern search, but macro-generated items and calls through other
receivers stay invisible. The report prints that boundary as a footnote.
"""
from __future__ import annotations

import argparse
import collections
import json
import pathlib
import re
import sys
from typing import Any

DEFAULT_ROOT = pathlib.Path(__file__).resolve().parents[2]
CRATE_SRC = pathlib.Path("crates/wow-world/src")
STATE_FILE = pathlib.Path("session/state.rs")
DEFAULT_HUB_THRESHOLD = 8
DEFAULT_TOP = 30

TEST_PATH = re.compile(r"(_tests?(/|\.rs)|/tests?(/|\.rs)|test_support|test_fixture|fixtures)")
IDENT = r"[A-Za-z_][A-Za-z0-9_]*"
STRUCT_HEAD = re.compile(r"\bpub(?:\s*\([^)]*\))?\s+struct\s+WorldSession\b[^{;]*\{")
# Inherent impls only: optional generics, an optional path, then the type itself.
IMPL_HEAD = re.compile(
    r"(?m)^[ \t]*impl\s*(?:<[^{};]*?>)?\s*(?:" + IDENT + r"\s*::\s*)*WorldSession\b"
    r"\s*(?:<[^{};]*>)?\s*(?:where\b[^{;]*)?\{"
)
FN_ITEM = re.compile(r"\bfn\s+(" + IDENT + r")")
FIELD_ACCESS = re.compile(r"\b(?:self|session|s)\s*\.\s*(" + IDENT + r")\b(?!\s*(?:\(|::\s*<))")
# A second member after a sub-state group: `self.<group>.<leaf>` (#1241 F2).
LEAF_ACCESS = re.compile(r"\s*\.\s*(" + IDENT + r")\b(?!\s*(?:\(|::\s*<))")
METHOD_CALL = re.compile(
    r"\b(?:self|session)\s*\.\s*(" + IDENT + r")\s*(?:::\s*<[^;{}()]*>\s*)?\("
)
ATTRIBUTE = re.compile(r"#\s*!?\s*\[")
VISIBILITY = re.compile(r"^pub(?:\s*\([^)]*\))?\s+")
FIELD_NAME = re.compile(r"^(" + IDENT + r")\s*:(?!:)")

FOOTNOTE = (
    "Lexical scan limits: macro-generated fields/methods/impls are invisible; only "
    "`self.`/`session.`/`s.` field receivers and `self.`/`session.` call receivers are "
    "seen, so calls through other bindings, UFCS (`WorldSession::f(..)`) and trait "
    "dispatch are not counted, and same-named fields of other structs accessed through "
    "`self` may be over-counted; methods whose name is defined in several domains are "
    "left out of call edges; `#[cfg(test)]` code inside production-classified paths "
    "counts as production; trait impls `impl T for WorldSession` are excluded."
)


def blank_noncode(text: str) -> str:
    """Blank comments and string/char literal contents, keeping offsets and newlines."""
    out = list(text)
    n = len(text)
    i = 0

    def blank(a: int, b: int) -> None:
        for k in range(a, min(b, n)):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        c = text[i]
        nxt = text[i + 1] if i + 1 < n else ""
        if c == "/" and nxt == "/":
            end = text.find("\n", i)
            end = n if end < 0 else end
            blank(i, end)
            i = end
        elif c == "/" and nxt == "*":
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif text.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            blank(i, j)
            i = j
        elif c in "rb" and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
            m = re.match(r"(?:br|r)(#*)\"", text[i:i + 300])
            if m:
                close = '"' + m.group(1)
                end = text.find(close, i + m.end())
                end = n if end < 0 else end + len(close)
                blank(i + m.end(), end - len(close))
                i = end
            elif c == "b" and nxt in "\"'":
                i += 1  # byte string/char: handled as a plain literal next round
            else:
                i += 1
        elif c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            blank(i + 1, j)
            i = j + 1
        elif c == "'":
            # Char literal vs lifetime/label: 'x', '\n', '\u{..}' are literals.
            if nxt == "\\":
                end = text.find("'", i + 2 if text[i + 2:i + 3] != "'" else i + 3)
                end = n if end < 0 else end
                blank(i + 1, end)
                i = end + 1
            elif i + 2 < n and text[i + 2] == "'":
                blank(i + 1, i + 2)
                i += 3
            else:
                i += 1
        else:
            i += 1
    return "".join(out)


def matching_close(code: str, open_index: int) -> int:
    """Index of the brace closing the `{` at open_index (code must be blanked)."""
    depth = 0
    for j in range(open_index, len(code)):
        ch = code[j]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return j
    raise ValueError(f"unbalanced brace at offset {open_index}")


def split_top_level(body: str) -> list[str]:
    """Split a struct body at commas outside (), [], {} and <> nesting."""
    parts, depth, start = [], 0, 0
    for j, ch in enumerate(body):
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth -= 1
        elif ch == "<":
            depth += 1
        elif ch == ">" and not (j and body[j - 1] in "-="):
            depth -= 1
        elif ch == "," and depth == 0:
            parts.append(body[start:j])
            start = j + 1
    parts.append(body[start:])
    return parts


def strip_attributes(segment: str) -> tuple[str, list[str]]:
    """Remove leading `#[...]` attributes, returning the rest and the attribute texts."""
    attrs = []
    rest = segment.lstrip()
    while True:
        m = ATTRIBUTE.match(rest)
        if not m:
            return rest, attrs
        depth, j = 0, m.end() - 1
        for j in range(m.end() - 1, len(rest)):
            if rest[j] == "[":
                depth += 1
            elif rest[j] == "]":
                depth -= 1
                if depth == 0:
                    break
        attrs.append(" ".join(rest[:j + 1].split()))
        rest = rest[j + 1:].lstrip()


def struct_fields(code: str, head: re.Match[str]) -> list[dict[str, Any]]:
    """Named fields of the struct whose `{` ends `head` (code must be blanked)."""
    body = code[head.end():matching_close(code, head.end() - 1)]
    fields = []
    for segment in split_top_level(body):
        rest, attrs = strip_attributes(segment)
        rest = VISIBILITY.sub("", rest)
        m = FIELD_NAME.match(rest)
        if m:
            cfgs = [a for a in attrs if re.match(r"#\s*\[\s*cfg\s*\(", a)]
            field_type = " ".join(rest[m.end():].split())
            fields.append({"name": m.group(1), "cfg": cfgs, "type": field_type})
    return sorted(fields, key=lambda f: f["name"])


def parse_fields(state_text: str) -> list[dict[str, Any]]:
    """Named fields of `pub struct WorldSession`, with any cfg attributes and type."""
    code = blank_noncode(state_text)
    head = STRUCT_HEAD.search(code)
    if not head:
        raise ValueError("pub struct WorldSession not found")
    return struct_fields(code, head)


def type_ident(field_type: str) -> str:
    """Last path segment of a field type without generics (`a::B<C>` -> `B`)."""
    return field_type.split("<", 1)[0].strip().split("::")[-1].strip()


# Test-only container members whose own members are sub-state groups (#1241 F3-0):
# `self.fixtures.<group>.<leaf>` attributes to `fixtures.<group>`'s leaf.
CONTAINER_FIELDS = frozenset({"fixtures"})


def substate_leaves(
    fields: list[dict[str, Any]], sources: list[tuple[str, str, str]]
) -> dict[str, dict[str, Any]]:
    """Leaf fields per WorldSession sub-state group (#1241 F2 sub-states).

    A top-level field whose type is a struct defined in the crate is a group:
    its named fields are leaves (`self.<group>.<leaf>`). A container field
    (`fixtures`) nests groups one level deeper, so its groups are
    `fixtures.<group>` and their fields the leaves. Any other top-level field
    is its own leaf. A leaf inherits the cfg of the fields above it.
    """
    structs: dict[str, list[dict[str, Any]]] = {}
    for _rel, _text, code in sources:
        for m in re.finditer(r"\bstruct\s+(" + IDENT + r")\b[^{;()]*\{", code):
            if m.group(1) not in structs:
                structs[m.group(1)] = struct_fields(code, m)
    groups: list[tuple[str, list[str], list[dict[str, Any]] | None]] = []
    for field in fields:
        members = structs.get(type_ident(field["type"]))
        if members and field["name"] in CONTAINER_FIELDS:
            for group in members:
                groups.append((f"{field['name']}.{group['name']}", field["cfg"] + group["cfg"],
                               structs.get(type_ident(group["type"]))))
        else:
            groups.append((field["name"], field["cfg"], members))
    leaves: dict[str, dict[str, Any]] = {}
    for path, cfg, members in groups:
        if not members:
            leaves[path] = {"group": None, "cfg": cfg}
            continue
        for member in members:
            name = member["name"]
            if name in leaves:
                name = f"{path}.{name}"
            leaves[name] = {"group": path, "cfg": cfg + member["cfg"], "leaf": member["name"]}
    return leaves


def impl_methods(code: str, head_pattern: re.Pattern[str] = IMPL_HEAD) -> list[str]:
    """Names of `fn` items directly inside inherent `impl WorldSession` blocks (or `head_pattern`'s)."""
    names = []
    for head in head_pattern.finditer(code):
        open_index = head.end() - 1
        close = matching_close(code, open_index)
        depth = 0
        segment_start = open_index + 1
        for j in range(open_index + 1, close):
            ch = code[j]
            if ch == "{":
                if depth == 0:
                    names.extend(FN_ITEM.findall(code, segment_start, j))
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0:
                    segment_start = j + 1
            elif ch == ";" and depth == 0:
                segment_start = j + 1
    return names


def impl_head(types: set[str]) -> re.Pattern[str]:
    """Inherent impl heads of any of `types` (#1241 F3 sub-state and hub-view owners)."""
    names = "|".join(sorted(map(re.escape, types))) or "(?!)"
    return re.compile(
        r"(?m)^[ \t]*impl\s*(?:<[^{};]*?>)?\s*(?:" + IDENT + r"\s*::\s*)*(?:" + names + r")\b"
        r"\s*(?:<[^{};]*>)?\s*(?:where\b[^{;]*)?\{"
    )


def is_test_path(rel: str) -> bool:
    return bool(TEST_PATH.search("/" + rel))


def domain_of(rel: str) -> str:
    parts = rel.split("/")
    if parts[0] in ("handlers", "session") and len(parts) > 1:
        return parts[0] + "/" + re.sub(r"\.rs$", "", parts[1])
    return re.sub(r"\.rs$", "", parts[0])


def strongly_connected(nodes: list[str], edges: dict[str, list[str]]) -> list[list[str]]:
    """Iterative Tarjan SCC; returns components of size > 1, sorted."""
    index: dict[str, int] = {}
    low: dict[str, int] = {}
    stack: list[str] = []
    on_stack: set[str] = set()
    result: list[list[str]] = []
    counter = 0
    for root in nodes:
        if root in index:
            continue
        work = [(root, 0)]
        while work:
            node, child = work.pop()
            if child == 0:
                index[node] = low[node] = counter
                counter += 1
                stack.append(node)
                on_stack.add(node)
            successors = edges.get(node, [])
            if child < len(successors):
                work.append((node, child + 1))
                succ = successors[child]
                if succ not in index:
                    work.append((succ, 0))
                elif succ in on_stack:
                    low[node] = min(low[node], index[succ])
                continue
            if low[node] == index[node]:
                component = []
                while True:
                    member = stack.pop()
                    on_stack.discard(member)
                    component.append(member)
                    if member == node:
                        break
                if len(component) > 1:
                    result.append(sorted(component))
            if work:
                parent = work[-1][0]
                low[parent] = min(low[parent], low[node])
    return sorted(result)


def load_sources(src: pathlib.Path) -> list[tuple[str, str, str]]:
    """(relative path, raw text, blanked code) for every .rs file under src."""
    out = []
    for path in sorted(src.rglob("*.rs")):
        text = path.read_text(encoding="utf-8", errors="replace")
        out.append((path.relative_to(src).as_posix(), text, blank_noncode(text)))
    return out


def analyze(root: pathlib.Path, hub_threshold: int = DEFAULT_HUB_THRESHOLD,
            top: int = DEFAULT_TOP) -> dict[str, Any]:
    src = root / CRATE_SRC
    fields = parse_fields((src / STATE_FILE).read_text(encoding="utf-8"))
    sources = load_sources(src)
    leaves = substate_leaves(fields, sources)
    field_names = set(leaves)
    plain = {name for name, leaf in leaves.items() if leaf["group"] is None}
    group_leaves: dict[str, dict[str, str]] = collections.defaultdict(dict)
    for name, leaf in leaves.items():
        if leaf["group"] is not None:
            group_leaves[leaf["group"]][leaf["leaf"]] = name

    # #1241 F3: fns moved onto sub-state / hub-view impls, the WorldSession thunks that still
    # delegate to them (same name on both), and unit_tests-only WorldSession shims.
    owner_head = impl_head({type_ident(f["type"]) for f in fields} | {"HubRef", "HubMut"})
    owned_names: set[str] = set()
    owned_total = 0
    for rel, _text, code in sources:
        if not is_test_path(rel):
            moved = impl_methods(code, owner_head)
            owned_total += len(moved)
            owned_names.update(moved)
    shim_total = 0
    for shim in sorted((root / CRATE_SRC).parent.glob("unit_tests/**/f3_shims.rs")):
        shim_total += len(impl_methods(blank_noncode(shim.read_text(encoding="utf-8"))))

    lines = {"production": 0, "test": 0}
    method_totals = {"production": 0, "test": 0}
    impl_files = {"production": 0, "test": 0}
    dom_lines: collections.Counter[str] = collections.Counter()
    access: dict[str, collections.Counter[str]] = collections.defaultdict(collections.Counter)
    defs: dict[str, set[str]] = collections.defaultdict(set)
    production = []
    for rel, text, code in sources:
        kind = "test" if is_test_path(rel) else "production"
        lines[kind] += text.count("\n")
        names = impl_methods(code)
        method_totals[kind] += len(names)
        impl_files[kind] += bool(IMPL_HEAD.search(code))
        if kind == "test":
            continue
        dom = domain_of(rel)
        production.append((dom, code))
        dom_lines[dom] += text.count("\n")
        for name in names:
            defs[name].add(dom)
        for m in FIELD_ACCESS.finditer(code):
            name, end = m.group(1), m.end()
            if name in CONTAINER_FIELDS:
                inner = LEAF_ACCESS.match(code, end)
                if not inner:
                    continue
                name, end = f"{name}.{inner.group(1)}", inner.end()
            if name in group_leaves:
                leaf = LEAF_ACCESS.match(code, end)
                if leaf and leaf.group(1) in group_leaves[name]:
                    access[group_leaves[name][leaf.group(1)]][dom] += 1
            elif name in plain:
                access[name][dom] += 1

    owner = {name: min(c.items(), key=lambda kv: (-kv[1], kv[0]))[0]
             for name, c in access.items()}
    unique = {name: next(iter(ds)) for name, ds in defs.items() if len(ds) == 1}
    edges: collections.Counter[tuple[str, str]] = collections.Counter()
    intra: collections.Counter[str] = collections.Counter()
    for dom, code in production:
        for m in METHOD_CALL.finditer(code):
            target = unique.get(m.group(1))
            if target is None:
                continue
            if target == dom:
                intra[dom] += 1
            else:
                edges[(dom, target)] += 1

    stats: dict[str, dict[str, Any]] = {
        d: {"domain": d, "lines": n, "fields_owned": 0, "own_accesses": 0,
            "foreign_accesses": 0, "methods": 0, "intra_calls": intra[d],
            "calls_out": 0, "calls_in": 0, "outbound_domains": 0}
        for d, n in dom_lines.items()
    }
    for name, counter in access.items():
        stats[owner[name]]["fields_owned"] += 1
        for dom, n in counter.items():
            key = "own_accesses" if dom == owner[name] else "foreign_accesses"
            stats[dom][key] += n
    for dom in unique.values():
        stats[dom]["methods"] += 1
    outbound: dict[str, set[str]] = collections.defaultdict(set)
    for (a, b), n in edges.items():
        stats[a]["calls_out"] += n
        stats[b]["calls_in"] += n
        outbound[a].add(b)
    for dom, targets in outbound.items():
        stats[dom]["outbound_domains"] = len(targets)
    domains = sorted(stats.values(), key=lambda s: (-s["lines"], s["domain"]))

    hubs = sorted(
        ({"field": name, "domains": len(c), "accesses": sum(c.values()), "owner": owner[name]}
         for name, c in access.items() if len(c) >= hub_threshold),
        key=lambda h: (-h["domains"], -h["accesses"], h["field"]),
    )
    ranked_edges = sorted(edges.items(), key=lambda kv: (-kv[1], kv[0]))
    top_edges = [{"from": a, "to": b, "calls": n} for (a, b), n in ranked_edges[:top]]
    graph: dict[str, list[str]] = collections.defaultdict(list)
    for e in top_edges:
        graph[e["from"]].append(e["to"])
    for succ in graph.values():
        succ.sort()
    nodes = sorted({e["from"] for e in top_edges} | {e["to"] for e in top_edges})
    violations = []
    for component in strongly_connected(nodes, graph):
        members = set(component)
        violations.append({
            "domains": component,
            "edges": [e for e in top_edges if e["from"] in members and e["to"] in members],
        })
    order = sorted(
        ({"domain": s["domain"], "score": s["calls_in"] - s["calls_out"],
          "calls_in": s["calls_in"], "calls_out": s["calls_out"]}
         for s in domains if s["calls_in"] or s["calls_out"] or s["methods"]),
        key=lambda o: (-o["score"], o["domain"]),
    )
    return {
        "schema_version": 1,
        "crate_src": CRATE_SRC.as_posix(),
        "r5": {
            "production_lines": lines["production"],
            "test_lines": lines["test"],
            "worldsession_fields": len(fields),
            "worldsession_cfg_fields": sum(1 for f in fields if f["cfg"]),
            "substate_leaf_fields": len(leaves),
            "substate_leaf_cfg_fields": sum(1 for leaf in leaves.values() if leaf["cfg"]),
            "impl_methods_production": method_totals["production"],
            "impl_methods_test": method_totals["test"],
            "impl_method_names_production": len(defs),
            "impl_files_production": impl_files["production"],
            "impl_files_test": impl_files["test"],
            "substate_impl_methods_production": owned_total,
            "worldsession_thunks": len(owned_names & set(defs)),
            "f3_test_shims": shim_total,
        },
        "fields": {
            "total": len(fields),
            "unused_in_production": sorted(field_names - set(access)),
            "owner": {name: owner.get(name) for name in sorted(field_names)},
        },
        "methods": {"unique_domain": len(unique), "multi_domain": len(defs) - len(unique)},
        "hub_threshold": hub_threshold,
        "hub_fields": hubs,
        "domains": domains,
        "top_edges": top_edges,
        "dag_violations": violations,
        "extraction_order": order,
        "footnote": FOOTNOTE,
    }


def render(report: dict[str, Any], top: int) -> str:
    r5 = report["r5"]
    out = [
        f"wow-world coupling report ({report['crate_src']})",
        "R5 metric:",
        f"  production lines           {r5['production_lines']:>8}",
        f"  test lines (path-classed)  {r5['test_lines']:>8}",
        f"  WorldSession fields        {r5['worldsession_fields']:>8}"
        f"  ({r5['worldsession_cfg_fields']} cfg-gated)",
        f"  sub-state leaf fields      {r5['substate_leaf_fields']:>8}"
        f"  ({r5['substate_leaf_cfg_fields']} cfg-gated)",
        f"  impl WorldSession fns      {r5['impl_methods_production']:>8}"
        f"  production ({r5['impl_method_names_production']} distinct names),"
        f" {r5['impl_methods_test']} test",
        f"  files with impl blocks     {r5['impl_files_production']:>8}"
        f"  production, {r5['impl_files_test']} test",
        f"  moved onto sub-states      {r5['substate_impl_methods_production']:>8}"
        f"  fns ({r5['worldsession_thunks']} WorldSession thunks keep their name),"
        f" {r5['f3_test_shims']} unit_tests shims",
        f"Fields unused in production: {len(report['fields']['unused_in_production'])};"
        f" methods attributed to one domain: {report['methods']['unique_domain']},"
        f" ambiguous names: {report['methods']['multi_domain']}",
        "",
        f"Hub fields (>= {report['hub_threshold']} domains): {len(report['hub_fields'])}",
    ]
    for h in report["hub_fields"][:top]:
        out.append(f"  {h['field']:44}{h['domains']:>4} domains{h['accesses']:>7} accesses"
                   f"  owner {h['owner']}")
    out += ["", f"{'domain':32}{'lines':>7}{'owned':>6}{'own':>6}{'foreign':>8}{'methods':>8}"
            f"{'intra':>6}{'out':>6}{'in':>6}{'out_dom':>8}"]
    for s in report["domains"][:top]:
        out.append(f"{s['domain']:32}{s['lines']:>7}{s['fields_owned']:>6}{s['own_accesses']:>6}"
                   f"{s['foreign_accesses']:>8}{s['methods']:>8}{s['intra_calls']:>6}"
                   f"{s['calls_out']:>6}{s['calls_in']:>6}{s['outbound_domains']:>8}")
    out += ["", f"Top cross-domain call edges ({len(report['top_edges'])}):"]
    out += [f"  {e['from']} -> {e['to']}: {e['calls']}" for e in report["top_edges"]]
    out += ["", f"DAG violations among top edges: {len(report['dag_violations'])}"]
    for v in report["dag_violations"]:
        out.append("  cycle: " + ", ".join(v["domains"]))
        out += [f"    {e['from']} -> {e['to']}: {e['calls']}" for e in v["edges"]]
    out += ["", "Proposed bottom-up extraction order (calls_in - calls_out):"]
    for pos, o in enumerate(report["extraction_order"][:top], 1):
        out.append(f"  {pos:>3}. {o['domain']:32}{o['score']:>7}"
                   f"  (in {o['calls_in']}, out {o['calls_out']})")
    out += ["", "Note: " + report["footnote"]]
    return "\n".join(out) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    rep = sub.add_parser("report", help="print the coupling map and R5 metric")
    rep.add_argument("--root", type=pathlib.Path, default=DEFAULT_ROOT)
    rep.add_argument("--json", action="store_true", help="emit one JSON object")
    rep.add_argument("--top", type=int, default=DEFAULT_TOP,
                     help="rows per table and edges considered for DAG cycles")
    rep.add_argument("--hub-threshold", type=int, default=DEFAULT_HUB_THRESHOLD)
    args = parser.parse_args(argv)
    if args.top < 1 or args.hub_threshold < 1:
        parser.error("--top and --hub-threshold must be positive")
    try:
        report = analyze(args.root.resolve(), args.hub_threshold, args.top)
    except (OSError, ValueError) as error:
        print(f"wow_world_coupling: {error}", file=sys.stderr)
        return 2
    if args.json:
        json.dump(report, sys.stdout, indent=2, sort_keys=True)
        sys.stdout.write("\n")
    else:
        sys.stdout.write(render(report, args.top))
    return 0


if __name__ == "__main__":
    sys.exit(main())
