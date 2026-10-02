#!/usr/bin/env python3
"""Read-only definitions and source-file guide for the #1263 P4b hub cut."""
from __future__ import annotations

import argparse
import collections
import hashlib
import json
import re
import sys
from pathlib import Path


REPO = Path(__file__).resolve().parents[2]
WORLD_SRC = Path("crates/wow-world/src")
CORE_SRC = Path("crates/wow-world-core/src")
TARGETS = Path("tools/codemods/f4_base_targets.json")
EXTRA_FILES = (
    "crates/wow-world/src/session/state/hub_support.rs",
    "crates/wow-world/src/session/state/driver_phase.rs",
)
IDENT = r"[A-Za-z_][A-Za-z0-9_]*"
DECL = re.compile(
    r"(?m)^[ \t]*(?:(?:pub(?:\s*\([^)]*\))?|async|const|unsafe|extern\s+\"[^\"]+\")\s+)*"
    r"(struct|enum|union|fn|const|static|type|trait)\s+(" + IDENT + r")\b"
)
CORE_USE = re.compile(
    r"(?m)^[ \t]*(?P<visibility>pub(?:\s*\([^)]*\))?\s+)?use\s+(?P<path>[^;]+);"
)
DIRECT_COUPLING = re.compile(r"\bWorldSession\b|\b[A-Z][A-Za-z0-9_]*Cx(?:Ref)?\b")

sys.path.insert(0, str(Path(__file__).resolve().parent))
import f4_test_fixtures_gate as item_tools


class PlanError(RuntimeError):
    pass


def _sha(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def _destination(source: str) -> str:
    prefix = WORLD_SRC.as_posix() + "/"
    if not source.startswith(prefix):
        raise PlanError(f"hint source is outside {WORLD_SRC}: {source}")
    return (CORE_SRC / source[len(prefix):]).as_posix()


def _module_path(path: Path, base: Path) -> tuple[str, ...]:
    rel = path.relative_to(base).with_suffix("")
    parts = list(rel.parts)
    if parts and parts[-1] == "mod":
        parts.pop()
    return tuple(parts)


def _declarations(code: str, name: str | None = None) -> list[re.Match[str]]:
    matches = list(DECL.finditer(code))
    return [m for m in matches if name is None or m.group(2) == name]


def _location(raw: str, code: str, match: re.Match[str], lexer) -> dict | None:
    name = match.group(2)
    header = code.rfind("\n", 0, match.start()) + 1
    span = item_tools.item_span(lexer, raw, code, name, kinds={match.group(1)})
    if span is None or not (span[0] <= header < span[1]):
        return None
    start, end = span
    metadata = raw[start:header]
    metadata_line = raw.count("\n", 0, start) + 1
    cfg_lines = [
        metadata_line + metadata.count("\n", 0, m.start())
        for m in re.finditer(r"(?m)^[ \t]*#\s*\[\s*cfg(?:_attr)?\b", metadata)
    ]
    doc_lines = [
        metadata_line + metadata.count("\n", 0, m.start())
        for m in re.finditer(r"(?m)^[ \t]*///|^[ \t]*//!", metadata)
    ]
    return {
        "kind": match.group(1),
        "name": name,
        "span": {
            "start_line": raw.count("\n", 0, start) + 1,
            "header_line": raw.count("\n", 0, header) + 1,
            "end_line": raw.count("\n", 0, max(start, end - 1)) + 1,
            "sha256": _sha(raw[start:end]),
        },
        "cfg_docs": {
            "start_line": metadata_line,
            "end_line": raw.count("\n", 0, max(start, header - 1)) + 1,
            "sha256": _sha(metadata),
            "cfg_lines": cfg_lines,
            "doc_lines": doc_lines,
        },
        "symbols": _direct_symbols(code, start, end),
        "_start": start,
        "_end": end,
    }


def _direct_symbols(code: str, start: int, end: int) -> list[dict]:
    found: dict[str, set[int]] = collections.defaultdict(set)
    for match in DIRECT_COUPLING.finditer(code, start, end):
        found[match.group(0)].add(code.count("\n", 0, match.start()) + 1)
    return [
        {"name": name, "lines": sorted(lines)}
        for name, lines in sorted(found.items())
    ]


def _load_source(root: Path, rel: str, cache: dict, lexer):
    if rel not in cache:
        path = root / rel
        if not path.is_file():
            cache[rel] = None
        else:
            raw = path.read_text()
            cache[rel] = (raw, lexer.blank_noncode(raw))
    return cache[rel]


def _reexport_index(root: Path, lexer) -> dict[str, list[dict]]:
    """Index public Core imports; comments, literals and nested imports are excluded."""
    out: dict[str, list[dict]] = collections.defaultdict(list)
    for path in sorted((root / WORLD_SRC).rglob("*.rs")):
        raw = path.read_text()
        code = lexer.blank_noncode(raw)
        for match in CORE_USE.finditer(code):
            if match.group("visibility") is None:
                continue
            prefix = code[:match.start()]
            if prefix.count("{") != prefix.count("}"):
                continue
            use_path = match.group("path").strip()
            if not use_path.startswith("wow_world_core::"):
                continue
            rel = path.relative_to(root).as_posix()
            after_crate = use_path[len("wow_world_core::"):]
            grouped = "::{" in after_crate
            module_prefix = after_crate.split("::{", 1)[0].removesuffix("::*").rstrip(":")
            out[rel].append({
                "path": use_path,
                "after_crate": after_crate,
                "leaves": sorted(item_tools.use_leaves(use_path)),
                "module_prefix": tuple(p for p in module_prefix.split("::") if p),
                "grouped": grouped,
                "wildcard": "*" in after_crate,
            })
    return out


def _routes_for(source: str, destination: str, name: str, reexports: dict) -> list[dict]:
    dest_module = _module_path(Path(destination), CORE_SRC)
    routes = []
    for world_file, statements in reexports.items():
        for statement in statements:
            after = statement["after_crate"]
            leaves = statement["leaves"]
            exact_named_item = False
            if statement["grouped"]:
                module = statement["module_prefix"]
                prefix_matches = bool(module) and dest_module[:len(module)] == module
                named = name in leaves and prefix_matches
                group_body = after.split("::{", 1)[1].rsplit("}", 1)[0]
                simple_group = all(
                    re.fullmatch(IDENT, leaf.strip())
                    for leaf in group_body.split(",") if leaf.strip()
                )
                exact_named_item = (
                    world_file == source
                    and module == dest_module
                    and simple_group
                    and name in {leaf.strip() for leaf in group_body.split(",") if leaf.strip()}
                )
                next_module = dest_module[len(module)] if prefix_matches and len(dest_module) > len(module) else None
                module_route = bool(next_module and next_module in leaves)
                module_route = module_route or (statement["wildcard"] and prefix_matches)
            elif statement["wildcard"]:
                module = statement["module_prefix"]
                module_route = bool(module) and dest_module[:len(module)] == module
                named = False
            else:
                parts = tuple(p for p in after.split("::") if p)
                is_named_item = bool(parts and parts[-1] == name)
                module = parts[:-1] if is_named_item else parts
                prefix_matches = bool(module) and dest_module[:len(module)] == module
                named = is_named_item and prefix_matches
                exact_named_item = world_file == source and parts == (*dest_module, name)
                module_route = not is_named_item and prefix_matches
            if named or module_route:
                routes.append({
                    "world_file": world_file,
                    "path": statement["path"],
                    "route_kind": "named_item" if named else "module_prefix",
                    "class": (
                        "already_in_core_verified"
                        if exact_named_item
                        else "core_candidate_unresolved_route"
                    ),
                })
    return routes


def _definition_index(root: Path, base: Path, cache: dict, lexer) -> dict:
    by_name: dict[str, list[dict]] = collections.defaultdict(list)
    for path in sorted((root / base).rglob("*.rs")):
        rel = path.relative_to(root).as_posix()
        loaded = _load_source(root, rel, cache, lexer)
        if loaded is None:
            continue
        raw, code = loaded
        for match in _declarations(code):
            item = _location(raw, code, match, lexer)
            if item is None:
                continue
            item.pop("_start", None)
            item.pop("_end", None)
            by_name[match.group(2)].append({"file": rel, **item})
    return by_name


def _impl_index(root: Path, base: Path, cache: dict, lexer) -> dict:
    by_owner: dict[str, list[dict]] = collections.defaultdict(list)
    for path in sorted((root / base).rglob("*.rs")):
        rel = path.relative_to(root).as_posix()
        loaded = _load_source(root, rel, cache, lexer)
        if loaded is None:
            continue
        raw, code = loaded
        for start, end, owner in item_tools.impl_blocks(lexer, code):
            by_owner[owner].append({
                "file": rel,
                "start_line": code.count("\n", 0, start) + 1,
                "end_line": code.count("\n", 0, max(start, end - 1)) + 1,
                "sha256": _sha(raw[start:end]),
            })
    return by_owner


def _item_plan(hint: dict, world_defs: dict, core_defs: dict, reexports: dict) -> dict:
    hint_source = hint["file"]
    hint_destination = _destination(hint_source)
    name = hint["name"]
    expected = hint.get("kind", "auto")
    origin_all = world_defs.get(name, [])
    target_all = core_defs.get(name, [])
    origin = [row for row in origin_all if expected == "auto" or row["kind"] == expected]
    target_candidates = [row for row in target_all if expected == "auto" or row["kind"] == expected]
    origin_hint = [row for row in origin if row["file"] == hint_source]
    candidate_routes = [
        (row, _routes_for(hint_source, row["file"], name, reexports))
        for row in target_candidates
    ]
    routed_targets = [
        (row, routes) for row, routes in candidate_routes
        if any(route["class"] == "already_in_core_verified" for route in routes)
    ]
    if len(target_candidates) > 1 and len(routed_targets) == 1:
        target, routes = [routed_targets[0][0]], routed_targets[0][1]
    else:
        target = target_candidates
        routes = [route for _, candidate in candidate_routes for route in candidate]
    if len(origin_hint) == 1 and not target:
        classification, reason = "remaining_in_world", "unique hinted definition remains in World"
    elif not origin and len(target) == 1 and any(route["class"] == "already_in_core_verified" for route in routes):
        classification, reason = "already_in_core_verified", "unique Core definition and exact public World named-item reexport"
    elif not origin and len(target) == 1 and any(route["class"] == "core_candidate_unresolved_route" for route in routes):
        classification, reason = "core_candidate_unresolved_route", "unique Core definition has only a route candidate; exact reexport resolution is unverified"
    elif not origin_hint and len(origin) == 1 and not target:
        classification, reason = "remaining_in_world", "unique definition remains in World at a path differing from the hint"
    else:
        classification = "missing_or_ambiguous"
        if len(origin_hint) > 1 or len(origin) > 1 or len(target) > 1:
            reason = "duplicate same-kind declarations in a crate"
        elif (not origin and not target and (origin_all or target_all)):
            reason = "declaration kind differs from the hint"
        elif origin and target:
            reason = "definition exists in both crates or destination is ambiguous"
        elif not origin and len(target) == 1 and not routes:
            reason = "Core definition exists but no public World reexport route was found"
        else:
            reason = "definition or reexport evidence is missing/ambiguous"
    origin_item = origin[0] if len(origin) == 1 else None
    target_item = target[0] if len(target) == 1 else None
    kind = (origin_item or target_item or {}).get("kind", expected)
    source = origin_item["file"] if origin_item else hint_source
    destination = target_item["file"] if target_item else _destination(source)
    return {
        "class": classification,
        "reason": reason,
        "source": source,
        "destination": destination,
        "hint_source": hint_source,
        "hint_destination": hint_destination,
        "kind": kind,
        "hint_kind": expected,
        "name": name,
        "origin_definition_count": len(origin),
        "destination_definition_count": len(target),
        "origin_hint_definition_count": len(origin_hint),
        "other_world_definition_files": sorted({row["file"] for row in origin if row["file"] != hint_source}),
        "world_candidate_files": sorted({row["file"] for row in origin}),
        "core_candidate_files": sorted({row["file"] for row in target}),
        "origin": origin_item,
        "destination_definition": target_item,
        "world_reexport_evidence": routes,
    }


def _impl_plan(hint: dict, world_impls: dict, core_impls: dict, core_defs: dict, reexports: dict) -> dict:
    hint_source = hint["file"]
    hint_destination = _destination(hint_source)
    owner = hint["self"]
    world_blocks = world_impls.get(owner, [])
    core_blocks = core_impls.get(owner, [])
    owner_defs = core_defs.get(owner, [])
    routes = _routes_for(hint_source, owner_defs[0]["file"], owner, reexports) if len(owner_defs) == 1 else []
    verified_route = any(route["class"] == "already_in_core_verified" for route in routes)
    candidate_route = any(route["class"] == "core_candidate_unresolved_route" for route in routes)
    if world_blocks and not core_blocks:
        cls = "remaining_in_world"
    elif not world_blocks and core_blocks and verified_route:
        cls = "already_in_core_verified"
    elif not world_blocks and core_blocks and candidate_route:
        cls = "core_candidate_unresolved_route"
    else:
        cls = "missing_or_ambiguous"
    source_files = sorted({row["file"] for row in world_blocks})
    destination_files = sorted({row["file"] for row in core_blocks})
    return {
        "class": cls,
        "source": source_files[0] if len(source_files) == 1 else hint_source,
        "destination": destination_files[0] if len(destination_files) == 1 else hint_destination,
        "hint_source": hint_source,
        "hint_destination": hint_destination,
        "kind": "inherent_or_trait_impl",
        "name": owner,
        "world_impl_blocks": len(world_blocks),
        "core_impl_blocks": len(core_blocks),
        "world_impl_locations": world_blocks,
        "core_impl_locations": core_blocks,
        "world_reexport_evidence": routes,
    }


def _file_guide(root: Path, source: str, item_rows: list[dict], cache: dict, lexer, reexports: dict) -> dict:
    destination = _destination(source)
    world = _load_source(root, source, cache, lexer)
    core = _load_source(root, destination, cache, lexer)
    if world is None:
        role = "world_source_absent"
        world_names, hinted = [], []
    else:
        raw, code = world
        world_names = [m.group(2) for m in _declarations(code)]
        hinted = [
            row["name"] for row in item_rows
            if row["hint_source"] == source and row["origin_definition_count"] == 1
        ]
        own_reexports = reexports.get(source, [])
        if hinted:
            role = "world_contains_target_definitions"
        elif world_names:
            role = "world_contains_definitions"
        elif own_reexports:
            role = "world_facade"
        else:
            role = "world_has_no_target_definitions"
        del raw
    if core is None:
        core_names = []
    else:
        core_names = [m.group(2) for m in _declarations(core[1])]
    return {
        "source": source,
        "destination": destination,
        "world_role": role,
        "world_exists": world is not None,
        "core_exists": core is not None,
        "world_hint_definitions": sorted(set(hinted)),
        "world_declaration_count": len(world_names),
        "world_declaration_names": sorted(set(world_names))[:24],
        "core_declaration_count": len(core_names),
        "core_declaration_names": sorted(set(core_names))[:24],
        "world_core_reexport_count": len(reexports.get(source, [])),
    }


def build_plan(root: Path, targets_path: Path) -> dict:
    root = root.resolve()
    targets = json.loads(targets_path.read_text())
    lexer = item_tools.lib(root)
    cache: dict = {}
    reexports = _reexport_index(root, lexer)
    world_defs = _definition_index(root, WORLD_SRC, cache, lexer)
    core_defs = _definition_index(root, CORE_SRC, cache, lexer)
    world_impls = _impl_index(root, WORLD_SRC, cache, lexer)
    core_impls = _impl_index(root, CORE_SRC, cache, lexer)
    items = [_item_plan(row, world_defs, core_defs, reexports) for row in targets.get("items", [])]
    files = list(dict.fromkeys([*targets.get("files", []), *EXTRA_FILES]))
    guides = [_file_guide(root, rel, items, cache, lexer, reexports) for rel in files]
    impls = [_impl_plan(row, world_impls, core_impls, core_defs, reexports) for row in targets.get("impls", [])]
    item_counts = collections.Counter(row["class"] for row in items)
    file_counts = collections.Counter(row["world_role"] for row in guides)
    impl_counts = collections.Counter(row["class"] for row in impls)
    unresolved = [
        {
            "domain": "item", "source": row["source"], "destination": row["destination"],
            "kind": row["kind"], "name": row["name"], "reason": row["reason"],
            "world_candidates": row["world_candidate_files"], "core_candidates": row["core_candidate_files"],
        }
        for row in items if row["class"] in {"missing_or_ambiguous", "core_candidate_unresolved_route"}
    ] + [
        {
            "domain": "impl", "source": row["source"], "destination": row["destination"],
            "kind": row["kind"], "name": row["name"],
            "reason": (
                "Core owner has only an unresolved route candidate"
                if row["class"] == "core_candidate_unresolved_route"
                else "impl/source/core state is missing or ambiguous"
            ),
        }
        for row in impls if row["class"] in {"missing_or_ambiguous", "core_candidate_unresolved_route"}
    ]
    return {
        "schema": "f4-hub-definitions-plan-v1",
        "root": str(root),
        "hint_counts": {
            "items": len(targets.get("items", [])),
            "files": len(targets.get("files", [])),
            "impls": len(targets.get("impls", [])),
            "impl_types": len(targets.get("impl_types", [])),
        },
        "counts": {
            "items_by_class": dict(sorted(item_counts.items())),
            "files_by_world_role": dict(sorted(file_counts.items())),
            "impls_by_class": dict(sorted(impl_counts.items())),
            "unresolved": len(unresolved),
        },
        "items": items,
        "files": guides,
        "impls": impls,
        "unresolved": unresolved,
    }


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    plan = sub.add_parser("plan", help="emit read-only source/destination definition plan as JSON")
    plan.add_argument("--root", type=Path, default=REPO)
    plan.add_argument("--targets", type=Path, default=REPO / TARGETS)
    plan.add_argument("--summary", action="store_true", help="emit counts and unresolved rows only")
    args = parser.parse_args(argv)
    try:
        result = build_plan(args.root, args.targets)
    except (OSError, json.JSONDecodeError, PlanError, item_tools.CodemodError) as error:
        parser.error(str(error))
    if args.summary:
        result = {
            "schema": result["schema"],
            "root": result["root"],
            "hint_counts": result["hint_counts"],
            "counts": result["counts"],
            "unresolved": result["unresolved"],
            "special_files": [
                row for row in result["files"] if row["source"] in EXTRA_FILES
            ],
        }
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
