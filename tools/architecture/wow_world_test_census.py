#!/usr/bin/env python3
"""Per-test census of the `wow-world` unit tests and their size ratchet (#1442 T0).

Usage:
  python3 tools/architecture/wow_world_test_census.py census
      [--repo PATH] [--write] [--tests]
  python3 tools/architecture/wow_world_test_census.py check [--repo PATH]
  python3 tools/architecture/wow_world_test_census.py self-test

No Cargo call. The census walks `crates/wow-world/src/lib.rs` through every
`mod` declaration (`#[path]` and inline nesting included, test-only mounts
too) and records every `#[test]`/`#[tokio::test]` function it reaches, in
`src/` and in `unit_tests/`.

For each test it computes the closure of the test-side helpers the test calls
(free functions in `unit_tests/` or in `cfg(test)` code of `src/`), resolved by
name: same file first, then the ancestor modules (`use super::*`), then the
nearest directory, then every definition with that name (a superset, never a
miss). Over that closure it records:

  * `session`: the closure names `WorldSession`/`SessionFixtures` or calls a
    session constructor helper (one whose body builds a `WorldSession`);
  * `shims`: it calls a `WorldSession` method defined in test-only code (the
    `f3_shims`/`test_only`/`*_test_shims` files or a `cfg(test)` impl);
  * `owner`: the primary `wow-world-*` crate its symbols resolve to. Symbols
    are the definitions (`fn`, `struct`, `enum`, `trait`, `type`, `const`,
    `static`) of `crates/wow-world*/src`; an identifier in call, path or type
    position (one-word lowercase names are std/trait methods and do not vote)
    votes for the crates defining it, and an explicit `use wow_world_x::...`,
    `wow_world_x::` path or `include_str!("../wow-world-x/src/..")` decides. A
    name defined both in `wow-world` and a domain crate is the domain's (the
    `wow-world` one forwards). The session methods it calls add the votes of
    their bodies (one level). The owner is the most-voted domain crate; core
    only when no other domain crate is referenced, `wow-world` when none is.
    A test-side type it names (`struct Fixture`) is a helper too: its
    definition and impl methods join the closure;
  * `class`:
      P  session-free: no session at all. Destination is the owner when it
         reaches (Cargo.lock) every referenced `wow-world*` crate, else the
         lowest crate that does; it stays in `wow-world` when it uses
         `wow-world`-owned production items or no crate reaches them all;
      H  domain-movable: builds a session, but every session method it calls
         is a host-trait method of, or a one-statement forward into, a single
         domain crate (core allowed) that reaches every referenced crate, and
         it uses no other `wow-world` item; movable only with a test host of
         that crate (the T2 pilot decides whether that is worth it);
      W  composition: anything else (World logic, several domains, World
         fields only); stays in `wow-world`.

`census --write` regenerates `wow-world-test-census.json` (the ledger; one test
per line); `census --write --raise|--tighten --issue N --reason "..."` also
appends a reviewed ceiling entry at exactly the live metrics, in one step.

`check` fails (exit 1) when the ledger differs from the tree, when a
`unit_tests/*.rs` file is not mounted, when a test-visible source is mounted
from outside `src/` and `unit_tests/`, when the test attributes of every
`crates/wow-world/**/*.rs` outside `tests/` differ from the mounted test
functions, or when the live suite exceeds the newest reviewed entry of
`wow-world-test-ceiling.json`: physical lines of
`crates/wow-world/unit_tests/**/*.rs`, tests there, and tests in `src/`. Only a
`raise` entry may lift a ceiling, and while the newest entry is a raise the
live metrics must equal it (no silent headroom). The integration tests in
`crates/wow-world/tests/` are outside the census and the ratchet.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import sys
import tomllib
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from session_method_index import is_forward  # noqa: E402
from wow_world_test_scan import (  # noqa: E402
    CensusError, Fn, Parsed, CRATE_PATH_RE, DEF_RE, FN_RE, TEST_ATTR_SCAN_RE,
    attribute_runs, brace_pairs, cfg_off_spans, crate_name, inside, mask, parse_file, specific,
)

REPO = Path(__file__).resolve().parents[2]
CRATE = Path("crates/wow-world")
LEDGER = Path("tools/architecture/wow-world-test-census.json")
CEILING = Path("tools/architecture/wow-world-test-ceiling.json")
CORE = "wow-world-core"
WORLD = "wow-world"
TEST_SUPPORT = "wow-world-test-support"
CLASSES = {"P": "session-free", "H": "domain-movable", "W": "composition"}
REASONS = {
    "P-domain": "session-free; its symbols resolve to the destination crate, which reaches every referenced crate",
    "P-lowest-covering": "session-free; the primary owner does not reach the crates in blockers, the destination "
                         "is the lowest crate that reaches them all",
    "P-library": "session-free; no wow-world-* symbol: tests library code through wow-world (stays until a "
                 "library destination is chosen)",
    "P-world-items": "session-free but uses the wow-world-owned production items in blockers (stays)",
    "P-unreached": "session-free but no wow-world-* crate reaches every referenced crate (stays)",
    "H-host": "builds a session, but every session call is a host-trait method of, or a one-statement forward "
              "into, the destination; movable only with a destination test host (T2 go/no-go)",
    "W-world-logic": "calls the WorldSession methods in blockers, which hold World logic or World-owned forwards",
    "W-multi-domain": "its session calls route to the several domains in blockers",
    "W-core-only": "its session calls route only to wow-world-core, the hub that stays with composition",
    "W-world-items": "its session calls are domain forwards, but it uses the wow-world-owned items in blockers",
    "W-unreached": "its session calls route to one domain that does not reach the crates in blockers",
    "W-fields": "builds a WorldSession and only reads or writes its fields",
}
SESSION_MARKERS = frozenset({"WorldSession", "SessionFixtures"})
MAX_DEFINERS = 3
# Integer vote weight, divisible by every definer count up to MAX_DEFINERS, so the
# tally (and the owner tie-break) never depends on set iteration order.
VOTE = 6
BLOCKERS_SHOWN = 4

# --------------------------------------------------------------------------- repository model

def module_tree(repo: Path) -> tuple[dict[Path, tuple[Path | None, bool]], dict[Path, Parsed]]:
    """Every file mounted from `src/lib.rs`: file -> (parent file, test-only mount), and its parse."""
    root = (repo / CRATE / "src" / "lib.rs").resolve()
    crate_dir = (repo / CRATE).resolve()
    tree: dict[Path, tuple[Path | None, bool]] = {root: (None, False)}
    pending = [(root, True)]
    parsed_cache: dict[Path, Parsed] = {}
    while pending:
        path, mod_rs = pending.pop()
        test_only = tree[path][1]
        source = path.read_text(encoding="utf-8")
        parsed = parse_file(source, path.relative_to(crate_dir).as_posix(), test_only)
        parsed_cache[path] = parsed
        for name, path_attr, nesting, gated in parsed.mounts:
            if path_attr:
                parent = (path.parent if not nesting or mod_rs else path.parent / path.stem).joinpath(*nesting)
                child, child_mod_rs = (parent / path_attr).resolve(), True
            else:
                parent = (path.parent if mod_rs else path.parent / path.stem).joinpath(*nesting)
                child, child_mod_rs = parent / f"{name}.rs", False
                if not child.exists():
                    child, child_mod_rs = parent / name / "mod.rs", True
            child = child.resolve()
            if not child.exists() or not child.is_relative_to(crate_dir) or child in tree:
                continue
            tree[child] = (path, test_only or gated or child.is_relative_to(crate_dir / "unit_tests"))
            pending.append((child, child_mod_rs))
    return tree, parsed_cache


def lock_dependencies(repo: Path) -> dict[str, set[str]]:
    lock = tomllib.loads((repo / "Cargo.lock").read_text(encoding="utf-8"))
    return {
        package["name"]: {dep.split(" ")[0] for dep in package.get("dependencies", [])} - {package["name"]}
        for package in lock.get("package", [])
    }


def reach(deps: dict[str, set[str]], start: str) -> set[str]:
    seen, stack = set(), list(deps.get(start, ()))
    while stack:
        node = stack.pop()
        if node not in seen:
            seen.add(node)
            stack.extend(deps.get(node, ()))
    return seen


def domain_crates(repo: Path) -> list[str]:
    """The `wow-world-*` crates that can own a test (the session-free test-support crate cannot)."""
    return sorted(p.name for p in (repo / "crates").iterdir()
                  if p.name.startswith("wow-world-") and p.name != TEST_SUPPORT and (p / "src").is_dir())


def symbol_table(repo: Path, world_defs: dict[str, set[str]]) -> dict[str, set[str]]:
    """Defining crates of every item name in `crates/wow-world-*/src`, plus `wow-world`'s production items."""
    defs: dict[str, set[str]] = defaultdict(set)
    for crate in domain_crates(repo):
        for path in sorted((repo / "crates" / crate / "src").rglob("*.rs")):
            masked = mask(path.read_text(encoding="utf-8"))
            for match in DEF_RE.finditer(masked):
                defs[match.group(2)].add(crate)
    for name, crates in world_defs.items():
        defs[name] |= crates
    return defs


def world_definitions(parsed_files: list[Parsed], sources: dict[str, str],
                      test_only: dict[str, bool]) -> dict[str, set[str]]:
    """Production item names of each `wow-world` file (outside test-only mounts and `cfg(test)`)."""
    defs: dict[str, set[str]] = {}
    for parsed in parsed_files:
        defs[parsed.rel] = set() if test_only[parsed.rel] else {
            match.group(2) for match in DEF_RE.finditer(mask(sources[parsed.rel]))
            if not inside(parsed.gated, match.start())
        }
    return defs


@dataclass
class Model:
    tests: list[Fn]
    helpers: dict[str, list[Fn]]
    methods: dict[str, list[Fn]]
    defs: dict[str, set[str]]
    uses: dict[str, dict[str, str]]
    parents: dict[str, str | None]
    deps: dict[str, set[str]]
    crates: list[str]
    unmounted: list[str]
    files: dict[str, str]
    local: dict[str, set[str]]
    resolved: dict = field(default_factory=dict)


def build_model(repo: Path) -> Model:
    crate_dir = (repo / CRATE).resolve()
    tree, parsed_by_path = module_tree(repo)
    rel = lambda path: path.relative_to(crate_dir).as_posix()
    sources = {rel(p): p.read_text(encoding="utf-8") for p in tree}
    parsed_files = [parsed_by_path[p] for p in sorted(tree)]
    local = world_definitions(parsed_files, sources, {rel(p): tree[p][1] for p in tree})
    world_defs: dict[str, set[str]] = defaultdict(set)
    for names in local.values():
        for name in names:
            world_defs[name].add(WORLD)
    defs = symbol_table(repo, world_defs)
    tests, helpers, methods = [], defaultdict(list), defaultdict(list)
    test_types = {kind.name for parsed in parsed_files for kind in parsed.types}
    for parsed in parsed_files:
        for kind in parsed.types:
            helpers[kind.name].append(kind)
        for fn in parsed.fns:
            if fn.test:
                tests.append(fn)
            elif fn.impl_self == "WorldSession":
                methods[fn.name].append(fn)
            elif fn.impl_self is None and fn.gated:
                helpers[fn.name].append(fn)
            elif fn.impl_self in test_types and fn.gated:
                helpers[fn.impl_self].append(fn)
    unit_dir = crate_dir / "unit_tests"
    unmounted = sorted(rel(p) for p in unit_dir.rglob("*.rs") if p.resolve() not in tree)
    deps = lock_dependencies(repo)
    return Model(
        tests=sorted(tests, key=lambda f: (f.file, f.line)),
        helpers=helpers, methods=methods, defs=defs,
        uses={p.rel: p.uses for p in parsed_files},
        parents={rel(p): (rel(tree[p][0]) if tree[p][0] else None) for p in tree},
        deps=deps, crates=domain_crates(repo), unmounted=unmounted, files=sources, local=local,
    )


# --------------------------------------------------------------------------- classification

def ancestors(model: Model, file: str) -> list[str]:
    chain = []
    current = model.parents.get(file)
    while current:
        chain.append(current)
        current = model.parents.get(current)
    return chain


def resolve_helper(model: Model, name: str, file: str) -> list[Fn]:
    key = (name, file)
    if key not in model.resolved:
        model.resolved[key] = _resolve_helper(model, name, file)
    return model.resolved[key]


def _resolve_helper(model: Model, name: str, file: str) -> list[Fn]:
    candidates = model.helpers.get(name, [])
    if len(candidates) <= 1:
        return candidates
    for scope in [file, *ancestors(model, file)]:
        found = [fn for fn in candidates if fn.file == scope]
        if found:
            return found
    def shared(fn: Fn) -> int:
        a, b = Path(fn.file).parent.parts, Path(file).parent.parts
        return next((i for i, (x, y) in enumerate(zip(a, b)) if x != y), min(len(a), len(b)))
    best = max(shared(fn) for fn in candidates)
    return [fn for fn in candidates if shared(fn) == best]


def imports_for(model: Model, file: str) -> dict[str, str]:
    merged: dict[str, str] = {}
    for scope in reversed([file, *ancestors(model, file)]):
        merged.update(model.uses.get(scope, {}))
    return merged


def local_items(model: Model, file: str) -> set[str]:
    """`wow-world` production items visible by name in `file`: its own and its ancestors' (`use super::*`)."""
    key = ("local", file)
    if key not in model.resolved:
        model.resolved[key] = set().union(*(model.local.get(scope, set()) for scope in [file, *ancestors(model, file)]))
    return model.resolved[key]


def definers(model: Model, name: str, imports: dict[str, str], local: set[str] = frozenset()) -> set[str]:
    root = imports.get(name)
    if root and root.startswith("wow_world"):
        return {crate_name(root)} - {TEST_SUPPORT}
    if root and root.startswith("wow_"):
        return set()
    if name in local:
        return {WORLD}
    crates = set(model.defs.get(name, ()))
    if WORLD in crates and len(crates) > 1:
        crates.discard(WORLD)
    return crates if len(crates) <= MAX_DEFINERS else set()


def is_constructor(fn: Fn) -> bool:
    return bool(re.search(r"\bWorldSession\s*(?:::\s*new\s*\(|\{)", fn.body))


def method_route(model: Model, fn: Fn) -> str:
    if fn.impl_trait:
        trait = re.sub(r"<.*$", "", fn.impl_trait).strip()
        explicit = CRATE_PATH_RE.match(trait.replace(" ", ""))
        crates = {crate_name(explicit.group(1))} if explicit else definers(
            model, trait.split("::")[-1], imports_for(model, fn.file))
        return next(iter(crates)) if len(crates) == 1 else WORLD
    if not is_forward(fn.body[1:-1].encode()):
        return WORLD
    imports = imports_for(model, fn.file)
    refs = set()
    for name in fn.names - {fn.name}:
        refs |= definers(model, name, imports) if len(definers(model, name, imports)) == 1 else set()
    refs |= {crate_name(m.group(1)) for m in CRATE_PATH_RE.finditer(fn.body)}
    if fn.name in model.defs and model.defs[fn.name] - {WORLD}:
        refs |= {next(iter(model.defs[fn.name] - {WORLD}))} if len(model.defs[fn.name] - {WORLD}) == 1 else set()
    domains = refs - {CORE}
    if WORLD in domains or len(domains) > 1:
        return WORLD
    return next(iter(domains)) if domains else CORE


def classify(model: Model, test: Fn, routes: dict[str, str]) -> dict:
    closure, constructors = {id(test): test}, {}
    pending = [test]
    while pending:
        fn = pending.pop()
        for name in fn.names:
            for helper in resolve_helper(model, name, fn.file):
                if helper is test or id(helper) in closure or id(helper) in constructors:
                    continue
                if is_constructor(helper):
                    constructors[id(helper)] = helper
                else:
                    closure[id(helper)] = helper
                    pending.append(helper)
    members = list(closure.values())
    helper_names = {fn.name for fn in members} | {fn.name for fn in constructors.values()}
    all_names = set().union(*(fn.names for fn in members))
    session = bool(constructors) or bool(all_names & SESSION_MARKERS)
    votes: Counter[str] = Counter()
    used_crates: set[str] = set()
    world_items: set[str] = set()
    calls: set[str] = set()
    for fn in members:
        imports = imports_for(model, fn.file)
        local = local_items(model, fn.file)
        for root in fn.paths:
            if root.startswith("wow_"):
                used_crates.add(crate_name(root))
        for name in fn.names - helper_names:
            root = imports.get(name)
            if root and root.startswith("wow_"):
                used_crates.add(crate_name(root))
            is_call = session and name in model.methods and not (root and root.startswith("wow_world_"))
            if is_call:
                calls.add(name)
            if not specific(name):
                continue
            crates = definers(model, name, imports, local)
            if crates == {WORLD} and name not in SESSION_MARKERS and not is_call:
                world_items.add(name)
            for crate in crates - ({WORLD} if is_call else set()):
                votes[crate] += VOTE // len(crates)
        for root in fn.paths:
            if root.startswith("wow_world") and crate_name(root) != TEST_SUPPORT:
                votes[crate_name(root)] += VOTE
    for name in calls:
        for method in model.methods[name]:
            imports = imports_for(model, method.file)
            for inner in method.names - {name}:
                if not specific(inner):
                    continue
                crates = definers(model, inner, imports)
                for crate in crates:
                    votes[crate] += VOTE // len(crates)
    shims = any(m.gated for name in calls for m in model.methods[name])
    used_crates |= {c for c in votes if c != WORLD}
    referenced = {c for c in votes if c.startswith("wow-world-")}
    domains = sorted(referenced - {CORE})
    owner = (max(domains, key=lambda c: (votes[c], c)) if domains
             else CORE if CORE in referenced else WORLD)

    def unreached_from(crate: str) -> list[str]:
        return sorted(referenced - {crate} - reach(model.deps, crate))

    blockers: list[str] = []
    if not session:
        klass = "P"
        covering = sorted((c for c in referenced if not unreached_from(c)),
                          key=lambda c: (len(reach(model.deps, c)), -votes[c], c))
        if world_items:
            dest, reason, blockers = WORLD, "P-world-items", sorted(world_items)
        elif owner == WORLD:
            dest, reason = WORLD, "P-library"
        elif not unreached_from(owner):
            dest, reason = owner, "P-domain"
        elif covering:
            dest, reason, blockers = covering[0], "P-lowest-covering", unreached_from(owner)
        else:
            dest, reason, blockers = WORLD, "P-unreached", unreached_from(owner)
    else:
        call_routes = {name: routes[name] for name in calls}
        world_calls = sorted(n for n, r in call_routes.items() if r == WORLD)
        domain_routes = sorted({r for r in call_routes.values()} - {WORLD, CORE})
        klass, dest = "W", WORLD
        if not calls:
            reason = "W-fields"
        elif world_calls:
            reason, blockers = "W-world-logic", world_calls
        elif len(domain_routes) > 1:
            reason, blockers = "W-multi-domain", domain_routes
        elif not domain_routes:
            reason = "W-core-only"
        elif world_items:
            reason, blockers = "W-world-items", sorted(world_items)
        elif unreached_from(domain_routes[0]):
            reason, blockers = "W-unreached", unreached_from(domain_routes[0])
        else:
            klass, dest, reason = "H", domain_routes[0], "H-host"
    row = {
        "session": session, "shims": shims, "owner": owner, "class": klass, "destination": dest,
        "reason": reason, "blockers": blockers[:BLOCKERS_SHOWN],
    }
    missing = sorted(c for c in used_crates if dest != WORLD and c != dest and c not in model.deps.get(dest, set())
                     and c in model.deps)
    if missing:
        row["dev_deps"] = missing
    return row


def test_id(fn: Fn) -> str:
    return "::".join([fn.file, *fn.module, fn.name])


BRANCH_RE = re.compile(
    r"\b(?P<consumer>world_test_consumer_like_cpp)\b"
    r"|\btest_fixtures_ref_like_cpp\s*\(\s*\)\s*\.\s*is_(?:some|none)\s*\("
    r"|\blet\s+Some\s*\([^()=;]*\)\s*=\s*(?:[\w.()]*\.)?test_fixtures_ref_like_cpp\s*\(\s*\)"
)


def world_test_branches(repo: Path) -> list[dict]:
    """Production functions that branch on whether a World test fixture is present.

    One row per function and kind: `definition` of `world_test_consumer_like_cpp`,
    a `consumer` call of it, a direct `test_fixtures_ref_like_cpp().is_some()` /
    `.is_none()` `fixture-check`, or a `let Some(..) = ..test_fixtures_ref_like_cpp()`
    `fixture-binding`; `test_fixtures_gated` when the site only exists under
    `cfg(any(test, feature = "test-fixtures"))`.
    """
    rows: dict[tuple, int] = {}
    for crate in [WORLD, *domain_crates(repo)]:
        for path in sorted((repo / "crates" / crate / "src").rglob("*.rs")):
            source = path.read_text(encoding="utf-8")
            if "world_test_consumer_like_cpp" not in source and "test_fixtures_ref_like_cpp" not in source:
                continue
            masked = mask(source)
            pairs = brace_pairs(masked)
            runs = attribute_runs(source, masked)
            gated = cfg_off_spans(source, masked, runs, pairs)
            fns = [(m.start(), m.group(1)) for m in FN_RE.finditer(masked)]
            for match in BRANCH_RE.finditer(masked):
                enclosing = max((f for f in fns if f[0] < match.start()), default=(0, "<item>"))
                if masked[max(0, match.start() - 8):match.start()].rstrip().endswith("fn"):
                    kind = "definition"
                elif match.group("consumer"):
                    kind = "consumer"
                else:
                    kind = "fixture-binding" if match.group(0).startswith("let") else "fixture-check"
                key = (crate, path.relative_to(repo).as_posix(), enclosing[1], kind, inside(gated, match.start()))
                rows[key] = rows.get(key, 0) + 1
    return [
        {"crate": crate, "file": file, "fn": fn, "kind": kind, "test_fixtures_gated": gated, "sites": sites}
        for (crate, file, fn, kind, gated), sites in rows.items()
    ]


def raw_test_attributes(repo: Path) -> int:
    """Test attributes in every `.rs` under `crates/wow-world`, mounted or not, except `tests/`."""
    crate_dir = repo / CRATE
    return sum(
        len(TEST_ATTR_SCAN_RE.findall(mask(path.read_text(encoding="utf-8"))))
        for path in sorted(crate_dir.rglob("*.rs"))
        if path.relative_to(crate_dir).parts[0] != "tests"
    )


def census(repo: Path) -> tuple[dict, dict, dict[str, int]]:
    """The ledger, the live ratchet metrics and each test's line count (kept out of the ledger)."""
    model = build_model(repo)
    routes = {name: (WORLD if len({method_route(model, m) for m in defs}) != 1
                     else method_route(model, defs[0])) for name, defs in model.methods.items()}
    rows = []
    line_counts: dict[str, int] = {}
    seen: Counter[str] = Counter()
    for fn in model.tests:
        identity = test_id(fn)
        seen[identity] += 1
        if seen[identity] > 1:
            identity += f"#{seen[identity]}"
        row = {"id": identity}
        if fn.ignored:
            row["ignored"] = True
        row.update(classify(model, fn, routes))
        if not row["blockers"]:
            del row["blockers"]
        rows.append(row)
        line_counts[identity] = fn.lines
    host_surface: Counter[str] = Counter()
    for defs in model.methods.values():
        for method in defs:
            if method.impl_trait:
                route = method_route(model, method)
                if route not in (WORLD,):
                    host_surface[route] += 1
    attributes = sum(len(TEST_ATTR_SCAN_RE.findall(mask(source))) for source in model.files.values())
    if attributes != len(rows):
        raise CensusError(f"{attributes} test attributes in mounted sources but {len(rows)} test functions found")
    # Fail closed: the metrics count `unit_tests/` and `src/` only, so a test
    # mounted from anywhere else (`#[path = "../elsewhere/x.rs"]`) would escape them.
    outside = sorted(rel for rel in model.files if not rel.startswith(("src/", "unit_tests/")))
    if outside:
        raise CensusError("test-visible source mounted outside src/ and unit_tests/: " + ", ".join(outside))
    unit_files = sorted((repo / CRATE / "unit_tests").rglob("*.rs"))
    metrics = {
        "unit_tests_files": len(unit_files),
        "unit_tests_lines": sum(len(p.read_text(encoding="utf-8").splitlines()) for p in unit_files),
        "unit_tests_tests": sum(r["id"].startswith("unit_tests/") for r in rows),
        "src_tests": sum(r["id"].startswith("src/") for r in rows),
        "raw_test_attributes": raw_test_attributes(repo),
    }
    return {
        "schema_version": 1,
        "issue": 1442,
        "generator": "python3 tools/architecture/wow_world_test_census.py census --write",
        "classes": CLASSES,
        "reasons": REASONS,
        "summary": summarize(rows),
        "host_trait_methods": dict(sorted(host_surface.items())),
        "unmounted_unit_tests": model.unmounted,
        "world_test_branches": world_test_branches(repo),
        "tests": rows,
    }, metrics, line_counts


def summarize(rows: list[dict], lines: dict[str, int] | None = None) -> dict:
    """Tests per class, owner and destination (with `lines`, also their line totals)."""
    def table(key: str) -> dict:
        out: dict[str, dict] = {}
        for row in rows:
            entry = out.setdefault(row[key], {c: 0 for c in CLASSES})
            entry[row["class"]] += 1 if lines is None else lines[row["id"]]
        return {k: out[k] for k in sorted(out)}
    by_class = Counter(row["class"] for row in rows)
    return {
        "tests": len(rows),
        "ignored": sum(row.get("ignored", False) for row in rows),
        "session": sum(row["session"] for row in rows),
        "shims": sum(row["shims"] for row in rows),
        "by_class": {c: by_class.get(c, 0) for c in CLASSES},
        "by_reason": dict(sorted(Counter(row["reason"] for row in rows).items())),
        "by_owner": table("owner"),
        "by_destination": table("destination"),
    }


# --------------------------------------------------------------------------- ledger and ratchet

TEST_COLUMNS = ("id", "class", "destination", "owner", "reason", "session", "shims", "ignored", "blockers",
                "dev_deps")


def encode_row(row: dict) -> list:
    defaults = {"ignored": False, "blockers": [], "dev_deps": []}
    return [row.get(column, defaults.get(column)) for column in TEST_COLUMNS]


def decode_row(values: list) -> dict:
    if not isinstance(values, list) or len(values) != len(TEST_COLUMNS):
        raise CensusError(f"ledger test row must have the {len(TEST_COLUMNS)} columns {TEST_COLUMNS}")
    row = dict(zip(TEST_COLUMNS, values))
    for column, default in (("ignored", False), ("blockers", []), ("dev_deps", [])):
        if row[column] == default:
            del row[column]
    return row


def dump(ledger: dict) -> str:
    """Stable JSON with one test row (a `test_columns` list) per line, so a diff names its tests."""
    head = {k: v for k, v in ledger.items() if k not in ("tests", "world_test_branches")}
    head["test_columns"] = list(TEST_COLUMNS)
    text = json.dumps(head, indent=1)[:-2]
    for key, rows in (("world_test_branches", ledger["world_test_branches"]),
                      ("tests", [encode_row(row) for row in ledger["tests"]])):
        body = ",\n".join("  " + json.dumps(row) for row in rows)
        text += f',\n "{key}": [\n{body}\n ]'
    return text + "\n}\n"


def load(text: str) -> dict:
    ledger = json.loads(text)
    if ledger.get("test_columns") != list(TEST_COLUMNS):
        raise CensusError(f"ledger test_columns must be {list(TEST_COLUMNS)}")
    ledger["tests"] = [decode_row(row) for row in ledger["tests"]]
    del ledger["test_columns"]
    return ledger


CEILING_KEYS = ("unit_tests_lines", "unit_tests_tests", "src_tests")
ENTRY_KEYS = {"id", "kind", "issue", "reviewed_at", "reason", *CEILING_KEYS}


def effective_ceiling(policy: dict) -> dict:
    if not isinstance(policy, dict) or set(policy) != {"schema_version", "scope", "entries"}:
        raise CensusError("ceiling policy requires exactly schema_version, scope, entries")
    if policy["schema_version"] != 1:
        raise CensusError("unsupported ceiling schema")
    entries = policy["entries"]
    if not isinstance(entries, list) or not entries:
        raise CensusError("ceiling policy needs at least the baseline entry")
    previous, ids, last_date = None, set(), None
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict) or set(entry) != ENTRY_KEYS:
            raise CensusError(f"ceiling entry {index} requires exactly {sorted(ENTRY_KEYS)}")
        if entry["id"] in ids or not isinstance(entry["id"], str) or not entry["id"].strip():
            raise CensusError(f"ceiling entry {index} needs a unique id")
        ids.add(entry["id"])
        for key in CEILING_KEYS:
            if type(entry[key]) is not int or entry[key] < 0:
                raise CensusError(f"ceiling entry {entry['id']}: {key} must be a non-negative integer")
        if type(entry["issue"]) is not int or entry["issue"] <= 0:
            raise CensusError(f"ceiling entry {entry['id']}: issue must be a positive integer")
        if not isinstance(entry["reason"], str) or not entry["reason"].strip():
            raise CensusError(f"ceiling entry {entry['id']}: reason must be nonempty")
        try:
            reviewed = dt.date.fromisoformat(entry["reviewed_at"])
        except (TypeError, ValueError) as exc:
            raise CensusError(f"ceiling entry {entry['id']}: reviewed_at must be an ISO date") from exc
        if last_date and reviewed < last_date:
            raise CensusError(f"ceiling entry {entry['id']}: entries must be in review order")
        last_date = reviewed
        expected = "baseline" if index == 0 else None
        if expected and entry["kind"] != expected:
            raise CensusError("the first ceiling entry must be the baseline")
        if index and entry["kind"] not in ("tighten", "raise"):
            raise CensusError(f"ceiling entry {entry['id']}: kind must be tighten or raise")
        if previous is not None:
            grows = [k for k in CEILING_KEYS if entry[k] > previous[k]]
            if grows and entry["kind"] != "raise":
                raise CensusError(f"ceiling entry {entry['id']} raises {', '.join(grows)} without kind raise")
            if not grows and entry["kind"] == "raise":
                raise CensusError(f"ceiling entry {entry['id']} is a raise that raises nothing")
        previous = entry
    return previous


RAISE_COMMAND = "census --write --raise --issue N --reason '...'"


def ceiling_errors(metrics: dict, ceiling: dict) -> list[str]:
    errors = [
        f"{key} {metrics[key]} exceeds the reviewed ceiling {ceiling[key]} "
        f"(entry {ceiling['id']}); move or delete tests, or add a reviewed raise ({RAISE_COMMAND})"
        for key in CEILING_KEYS if metrics[key] > ceiling[key]
    ]
    if ceiling["kind"] == "raise":
        # A raise records the reviewed suite exactly; it never leaves silent headroom.
        errors += [
            f"{key} {metrics[key]} differs from the newest raise entry {ceiling['id']} ({ceiling[key]}); "
            "a raise must equal the live metrics: regenerate it, or record a reduction with "
            "census --write --tighten --issue N --reason '...'"
            for key in CEILING_KEYS if metrics[key] < ceiling[key]
        ]
    return errors


def ledger_diff(expected: dict, actual: dict) -> list[str]:
    if expected == actual:
        return []
    problems = []
    old = {row["id"]: row for row in expected.get("tests", [])}
    new = {row["id"]: row for row in actual.get("tests", [])}
    for key in sorted(new.keys() - old.keys())[:5]:
        problems.append(f"test not in ledger: {key}")
    for key in sorted(old.keys() - new.keys())[:5]:
        problems.append(f"ledger test no longer in tree: {key}")
    for key in sorted(k for k in old.keys() & new.keys() if old[k] != new[k])[:5]:
        changed = sorted(f for f in new[key] if old[key].get(f) != new[key][f])
        problems.append(f"ledger row changed: {key} ({', '.join(changed)})")
    for key in sorted(k for k in actual if k != "tests" and expected.get(k) != actual[k]):
        problems.append(f"ledger section changed: {key}")
    return problems or ["ledger differs from the tree"]


def check_errors(repo: Path) -> tuple[list[str], dict, dict, dict]:
    """Ledger drift, unmounted test sources and ceiling breaches, with the live metrics."""
    actual, metrics, _ = census(repo)
    errors = []
    ledger_path = repo / LEDGER
    if not ledger_path.is_file():
        errors.append(f"missing ledger {LEDGER}")
    else:
        drift = ledger_diff(load(ledger_path.read_text(encoding="utf-8")), load(dump(actual)))
        if drift:
            errors += drift + [f"regenerate with: {actual['generator']} and review the class/destination delta"]
    for path in actual["unmounted_unit_tests"]:
        errors.append(f"unit_tests source not mounted from src/lib.rs: {path}")
    mounted = metrics["unit_tests_tests"] + metrics["src_tests"]
    if metrics["raw_test_attributes"] != mounted:
        errors.append(f"{metrics['raw_test_attributes']} test attributes in crates/wow-world/** (outside tests/) "
                      f"but {mounted} mounted test functions: a test source is not mounted from src/lib.rs")
    ceiling = effective_ceiling(json.loads((repo / CEILING).read_text(encoding="utf-8")))
    errors += ceiling_errors(metrics, ceiling)
    return errors, metrics, ceiling, actual


def ceiling_entry(policy: dict, metrics: dict, kind: str, issue: int, reason: str,
                  today: dt.date | None = None) -> dict:
    """A reviewed `raise`/`tighten` entry at exactly the live metrics, validated against the policy."""
    if type(issue) is not int or issue <= 0:
        raise CensusError("--issue must be a positive issue number")
    if not reason or not reason.strip():
        raise CensusError("--reason must say why the ceiling changes")
    entries = policy["entries"]
    number = sum(entry["kind"] == kind for entry in entries) + 1
    entry = {
        "id": f"{kind}-{issue}-{number}", "kind": kind, "issue": issue,
        "reviewed_at": (today or dt.datetime.now(dt.timezone.utc).date()).isoformat(),
        "reason": reason.strip(), **{key: metrics[key] for key in CEILING_KEYS},
    }
    effective_ceiling({**policy, "entries": [*entries, entry]})
    return entry


def write_census(repo: Path, kind: str | None, issue: int | None, reason: str | None) -> tuple[dict, dict, dict]:
    """Rewrite the ledger and, with `kind`, append a ceiling entry at the live metrics (one step)."""
    ledger, metrics, lines = census(repo)
    entry = None
    if kind:
        policy = json.loads((repo / CEILING).read_text(encoding="utf-8"))
        entry = ceiling_entry(policy, metrics, kind, issue, reason)
        policy["entries"].append(entry)
        (repo / CEILING).write_text(json.dumps(policy, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    (repo / LEDGER).write_text(dump(ledger), encoding="utf-8")
    return ledger, metrics, lines


def check(repo: Path) -> int:
    errors, metrics, ceiling, actual = check_errors(repo)
    print("wow_world_test_census: " + ", ".join(
        f"{key} {metrics[key]} (ceiling {ceiling[key]})" for key in CEILING_KEYS)
        + f"; classes {actual['summary']['by_class']}")
    for key in CEILING_KEYS:
        if metrics[key] < ceiling[key]:
            print(f"note: {key} can be tightened to {metrics[key]} with a `tighten` entry")
    for error in errors:
        print(error, file=sys.stderr)
    print("wow_world_test_census check:", "FAIL" if errors else "PASS")
    return 1 if errors else 0


def print_census(ledger: dict, metrics: dict, lines: dict[str, int], list_tests: bool) -> None:
    summary = ledger["summary"]
    line_summary = summarize(ledger["tests"], lines)
    print(f"tests {summary['tests']} (ignored {summary['ignored']}), session {summary['session']}, "
          f"shims {summary['shims']}; classes {summary['by_class']}; metrics {metrics}")
    print(f"reasons {summary['by_reason']}")
    for title, key in (("owner", "by_owner"), ("destination", "by_destination")):
        print(f"\nby {title}: crate  " + "  ".join(f"{c} tests/lines" for c in CLASSES))
        for crate, row in summary[key].items():
            print(f"  {crate:28} " + "  ".join(
                f"{row[c]:5}/{line_summary[key][crate][c]:6}" for c in CLASSES))
    print(f"\nhost-trait methods World implements, by crate: {ledger['host_trait_methods']}")
    print(f"world_test branch functions: {len({(r['file'], r['fn']) for r in ledger['world_test_branches']})} "
          f"({sum(r['sites'] for r in ledger['world_test_branches'])} sites); "
          f"unmounted unit_tests files: {len(ledger['unmounted_unit_tests'])}")
    if list_tests:
        for row in ledger["tests"]:
            print(f"{row['class']} {row['destination']:28} {row['reason']:18} {row['id']} "
                  f"{' '.join(row.get('blockers', []))}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("census")
    run.add_argument("--repo", type=Path, default=REPO)
    run.add_argument("--write", action="store_true", help=f"rewrite {LEDGER}")
    run.add_argument("--tests", action="store_true", help="list every test row")
    change = run.add_mutually_exclusive_group()
    change.add_argument("--raise", dest="ceiling_kind", action="store_const", const="raise",
                        help=f"with --write, also append a reviewed raise entry at the live metrics to {CEILING}")
    change.add_argument("--tighten", dest="ceiling_kind", action="store_const", const="tighten",
                        help="with --write, also append a tighten entry at the live metrics")
    run.add_argument("--issue", type=int, help="issue that reviews the ceiling change")
    run.add_argument("--reason", help="why the ceiling changes")
    gate = sub.add_parser("check")
    gate.add_argument("--repo", type=Path, default=REPO)
    sub.add_parser("self-test")
    args = parser.parse_args(argv)
    if args.command == "self-test":
        import unittest
        suite = unittest.defaultTestLoader.discover(str(Path(__file__).resolve().parent),
                                                    pattern="test_wow_world_test_census.py")
        return 0 if unittest.TextTestRunner(verbosity=1).run(suite).wasSuccessful() else 1
    try:
        if args.command == "check":
            return check(args.repo)
        if args.ceiling_kind and not args.write:
            parser.error("--raise/--tighten require --write")
        if args.write:
            ledger, metrics, lines = write_census(args.repo, args.ceiling_kind, args.issue, args.reason)
        else:
            ledger, metrics, lines = census(args.repo)
        print_census(ledger, metrics, lines, args.tests)
        if args.write:
            print(f"wrote {LEDGER}" + (f" and a {args.ceiling_kind} entry in {CEILING}" if args.ceiling_kind else ""))
        return 0
    except (CensusError, OSError, ValueError) as exc:
        print(f"wow_world_test_census failed: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
