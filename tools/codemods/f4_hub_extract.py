#!/usr/bin/env python3
"""Read-only lexical census for the #1263 P4b hub impl extraction plan."""
from __future__ import annotations

import argparse
import collections
import copy
import hashlib
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path


REPO = Path(__file__).resolve().parents[2]
SRC = Path("crates/wow-world/src")
TARGETS = Path("tools/codemods/f4_base_targets.json")
sys.path.insert(0, str(Path(__file__).resolve().parent))
import f4_test_fixtures_gate as item_tools


IMPL = re.compile(r"(?m)^[ \t]*impl\b")
ATTR = re.compile(r"(?m)^[ \t]*#(?!\!)[ \t]*\[")
FN = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\b")
UNRESOLVED = re.compile(r"\bWorldSession\b|\b[A-Z][A-Za-z0-9_]*Cx(?:Ref)?\b")
IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


class PlanError(RuntimeError):
    pass


def _line_start(text: str, offset: int) -> int:
    return text.rfind("\n", 0, offset) + 1


def _square_close(code: str, open_at: int) -> int:
    depth = 0
    for i in range(open_at, len(code)):
        if code[i] == "[":
            depth += 1
        elif code[i] == "]":
            depth -= 1
            if depth == 0:
                return i
    raise PlanError(f"unclosed attribute bracket at {open_at}")


def _outer_prefix_start(raw: str, line: int) -> int:
    start = item_tools.with_attrs(raw, line)
    # The shared helper also recognizes module-inner docs/attrs. They belong to
    # the source module, not to the following impl, so keep them on the shell.
    offset = start
    last_inner_end = None
    for text in raw[start:line].splitlines(keepends=True):
        if text.lstrip().startswith(("//!", "#![")):
            last_inner_end = offset + len(text)
        offset += len(text)
    return last_inner_end if last_inner_end is not None else start


def _prefix_start(raw: str, code: str, impl_at: int, _lexer) -> int:
    line = _line_start(code, impl_at)
    start = _outer_prefix_start(raw, line)
    # `with_attrs` reaches the nearest ordinary attrs/docs. Walk back from that
    # prefix to collect earlier multiline attrs, one at a time; each must end
    # before the current prefix with only lexically blank text in between.
    # This handles cfg + interleaved docs + allow, while code from a preceding
    # item prevents the walk from absorbing that item's attrs or documentation.
    cursor = start
    while cursor > 0:
        previous_attrs = []
        for match in ATTR.finditer(code, 0, cursor):
            if code.startswith("#!", match.start()):
                continue
            bracket = code.find("[", match.start(), cursor)
            if bracket < 0:
                continue
            close = _square_close(code, bracket)
            if close < cursor and not code[close + 1:cursor].strip():
                previous_attrs.append(_line_start(raw, match.start()))
        if not previous_attrs:
            break
        previous_start = _outer_prefix_start(raw, max(previous_attrs))
        if previous_start >= cursor:
            break
        start = min(start, previous_start)
        cursor = previous_start
    return start


def _top_keyword(text: str, keyword: str) -> int | None:
    angle = paren = bracket = 0
    i = 0
    while i < len(text):
        ch = text[i]
        if angle == paren == bracket == 0 and text.startswith(keyword, i):
            before = i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")
            end = i + len(keyword)
            after = end == len(text) or not (text[end].isalnum() or text[end] == "_")
            if before and after:
                return i
        if ch == "<":
            angle += 1
        elif ch == ">" and not (i and text[i - 1] == "-"):
            angle = max(0, angle - 1)
        elif ch == "(":
            paren += 1
        elif ch == ")":
            paren = max(0, paren - 1)
        elif ch == "[":
            bracket += 1
        elif ch == "]":
            bracket = max(0, bracket - 1)
        i += 1
    return None


def _parse_owner(header: str) -> tuple[str, str | None, str]:
    rest = re.sub(r"^\s*impl\b", "", header, count=1).strip()
    if rest.startswith("<"):
        depth = 0
        for i, ch in enumerate(rest):
            if ch == "<":
                depth += 1
            elif ch == ">" and not (i and rest[i - 1] == "-"):
                depth -= 1
                if depth == 0:
                    rest = rest[i + 1:].strip()
                    break
    where_at = _top_keyword(rest, "where")
    head = rest[:where_at].strip() if where_at is not None else rest
    for_at = _top_keyword(head, "for")
    trait = " ".join(head[:for_at].split()) if for_at is not None else None
    owner = " ".join(head[for_at + 3:].split()) if for_at is not None else " ".join(head.split())
    base = re.sub(r"<.*", "", owner).strip()
    owner_name = IDENT.findall(base)
    if not owner_name:
        raise PlanError(f"cannot identify impl owner from header: {header.strip()!r}")
    return owner, trait, owner_name[-1]


def _function_names(code: str, open_brace: int, close_brace: int) -> list[str]:
    names = []
    depth = 1
    i = open_brace + 1
    while i < close_brace:
        if depth == 1 and code.startswith("fn", i):
            before = i == 0 or not (code[i - 1].isalnum() or code[i - 1] == "_")
            match = FN.match(code, i) if before else None
            if match:
                names.append(match.group(1))
        if code[i] == "{":
            depth += 1
        elif code[i] == "}":
            depth -= 1
        i += 1
    return names


def _cfg_prefix(raw: str, code: str, start: int, impl_at: int) -> list[str]:
    found = []
    for match in ATTR.finditer(code, start, impl_at):
        bracket = code.find("[", match.start(), impl_at)
        close = _square_close(code, bracket)
        attr = raw[match.start():close + 1]
        if re.match(r"#\s*\[\s*cfg(?:_attr)?\b", attr):
            found.append(attr)
    return found


def _physical_module_hint(path: Path, source_root: Path) -> str:
    parts = list(path.relative_to(source_root).with_suffix("").parts)
    if parts and parts[-1] == "mod":
        parts.pop()
    return "crate" + ("::" + "::".join(parts) if parts else "")


def _destination_path(source_file: str) -> str:
    prefix = SRC.as_posix() + "/"
    if not source_file.startswith(prefix):
        raise PlanError(f"source file is outside {SRC.as_posix()}: {source_file}")
    relative = source_file[len(prefix):]
    if not relative or relative.startswith("../"):
        raise PlanError(f"invalid mirrored source path: {source_file}")
    return "crates/wow-world-core/src/" + relative


def _block_records(path: Path, raw: str, code: str, module: str, lexer, target_owners: set[str]):
    records = []
    source_file = path.as_posix()
    source_sha256 = hashlib.sha256(raw.encode("utf-8")).hexdigest()
    destination_path = _destination_path(source_file)
    for block_start, end, _ in item_tools.impl_blocks(lexer, code):
        impl_match = IMPL.search(code, block_start, end)
        if impl_match is None:
            continue
        impl_at = impl_match.start()
        brace = code.find("{", impl_match.end(), end)
        if brace < 0:
            continue
        try:
            owner, trait, name = _parse_owner(code[impl_at:brace])
        except PlanError as error:
            records.append({"source_file": path.as_posix(), "parse_error": str(error)})
            continue
        if name not in target_owners:
            continue
        prefix = _prefix_start(raw, code, impl_at, lexer)
        methods = _function_names(code, brace, end - 1)
        symbols = []
        for match in UNRESOLVED.finditer(code, impl_at, end):
            symbol = match.group(0)
            symbols.append({
                "name": symbol,
                "line": code.count("\n", 0, match.start()) + 1,
            })
        fragment = raw[prefix:end]
        records.append({
            "source_file": source_file,
            "destination_path": destination_path,
            "module_path_hint": module,
            "source_sha256": source_sha256,
            "owner": {"type": owner, "name": name, "trait": trait,
                      "kind": "trait" if trait else "inherent"},
            "cfg_prefix": _cfg_prefix(raw, code, prefix, impl_at),
            "function_names": methods,
            "unresolved": symbols,
            "span": {
                "start_line": raw.count("\n", 0, prefix) + 1,
                "impl_line": raw.count("\n", 0, impl_at) + 1,
                "end_line": raw.count("\n", 0, end - 1) + 1,
                "char_start": prefix,
                "char_end": end,
                "sha256": hashlib.sha256(fragment.encode("utf-8")).hexdigest(),
            },
        })
    return records


@dataclass(frozen=True)
class ExtractedBlock:
    """One validated source fragment and its unchanged planner metadata."""

    record: dict
    source: str


def extract_recorded_blocks(raw: str, records: list[dict], lexer) -> tuple[str, list[ExtractedBlock]]:
    """Return source with planned impl spans removed and their exact bytes.

    Records must all describe one unchanged source file from `build_plan`.
    The whole-file and per-span hashes, source coordinates, owner headers, and
    non-overlap checks make stale plans and repeated extraction fail closed.
    This function does not read or write files.
    """
    if not records:
        return raw, []
    if not all(isinstance(record, dict) for record in records):
        raise PlanError("records must be JSON objects")

    source_files = [record.get("source_file") for record in records]
    if any(not isinstance(path, str) for path in source_files) or len(set(source_files)) != 1:
        raise PlanError("records must describe exactly one source file")
    source_hashes = [record.get("source_sha256") for record in records]
    if any(not isinstance(value, str) for value in source_hashes) or len(set(source_hashes)) != 1:
        raise PlanError("records must carry one source_sha256")
    actual_source_hash = hashlib.sha256(raw.encode("utf-8")).hexdigest()
    expected_source_hash = source_hashes[0]
    if actual_source_hash != expected_source_hash:
        raise PlanError("source drift or reapply: source_sha256 does not match")

    spans = []
    for record in records:
        try:
            span = record["span"]
            start, end = span["char_start"], span["char_end"]
            digest = span["sha256"]
        except (KeyError, TypeError) as error:
            raise PlanError("record is missing a valid span") from error
        if (not isinstance(start, int) or isinstance(start, bool)
                or not isinstance(end, int) or isinstance(end, bool)
                or not isinstance(digest, str)):
            raise PlanError("record has invalid span coordinates or sha256")
        if start < 0 or end <= start or end > len(raw):
            raise PlanError(f"span is out of bounds: {start}:{end}")
        spans.append((start, end, record))

    spans.sort(key=lambda value: (value[0], value[1]))
    for previous, current in zip(spans, spans[1:]):
        if previous[:2] == current[:2]:
            raise PlanError(f"duplicate span: {current[0]}:{current[1]}")
        if current[0] < previous[1]:
            raise PlanError(
                f"overlapping spans: {previous[0]}:{previous[1]} and {current[0]}:{current[1]}"
            )

    code = lexer.blank_noncode(raw)
    impls = item_tools.impl_blocks(lexer, code)
    validated = []
    for start, end, record in spans:
        span = record["span"]
        fragment = raw[start:end]
        if hashlib.sha256(fragment.encode("utf-8")).hexdigest() != span["sha256"]:
            raise PlanError(f"stale block sha256 at {start}:{end}")
        if start != _line_start(raw, start):
            raise PlanError(f"span does not start at a source line boundary: {start}")
        expected_start_line = raw.count("\n", 0, start) + 1
        expected_end_line = raw.count("\n", 0, end - 1) + 1
        if span.get("start_line") != expected_start_line or span.get("end_line") != expected_end_line:
            raise PlanError(f"stale line coordinates at {start}:{end}")

        impl_match = IMPL.search(code, start, end)
        if impl_match is None:
            raise PlanError(f"span does not contain an impl header: {start}:{end}")
        impl_at = impl_match.start()
        if record.get("span", {}).get("impl_line") != raw.count("\n", 0, impl_at) + 1:
            raise PlanError(f"stale impl line at {start}:{end}")
        matching_impls = [item for item in impls if item[1] == end and item[0] <= impl_at]
        if len(matching_impls) != 1:
            raise PlanError(f"span does not identify exactly one impl end: {start}:{end}")
        brace = code.find("{", impl_match.end(), end)
        if brace < 0:
            raise PlanError(f"impl header has no body: {start}:{end}")
        owner, trait, name = _parse_owner(code[impl_at:brace])
        planned_owner = record.get("owner", {})
        if not isinstance(planned_owner, dict):
            raise PlanError(f"record has invalid owner metadata at {start}:{end}")
        if (owner != planned_owner.get("type") or trait != planned_owner.get("trait")
                or name != planned_owner.get("name")):
            raise PlanError(f"impl owner changed at {start}:{end}")
        if _prefix_start(raw, code, impl_at, lexer) != start:
            raise PlanError(f"impl prefix span changed at {start}:{end}")
        validated.append((start, end, copy.deepcopy(record), fragment))

    retained_parts = []
    cursor = 0
    moved = []
    for start, end, record, fragment in validated:
        retained_parts.append(raw[cursor:start])
        cursor = end
        moved.append(ExtractedBlock(record=record, source=fragment))
    retained_parts.append(raw[cursor:])
    return "".join(retained_parts), moved


def build_plan(root: Path) -> dict:
    root = Path(root).resolve()
    lexer = item_tools.lib(root)
    target_path = root / TARGETS
    target_data = json.loads(target_path.read_text(encoding="utf-8"))
    target_owners = set(target_data["impl_types"])
    source_root = root / SRC
    paths = sorted(source_root.rglob("*.rs"))
    blocks = []
    all_impls = world_session = cx_impls = 0
    for path in paths:
        raw = path.read_bytes().decode("utf-8")
        code = lexer.blank_noncode(raw)
        impls = item_tools.impl_blocks(lexer, code)
        all_impls += len(impls)
        for start, end, owner_hint in impls:
            if owner_hint == "WorldSession":
                world_session += 1
            if re.search(r"Cx(?:Ref)?$", owner_hint):
                cx_impls += 1
        rel = Path("crates/wow-world/src") / path.relative_to(source_root)
        module = _physical_module_hint(path, source_root)
        blocks.extend(_block_records(rel, raw, code, module, lexer, target_owners))

    counts = collections.Counter(
        block.get("owner", {}).get("name") for block in blocks if "owner" in block
    )
    unresolved_counts = collections.Counter(
        symbol["name"] for block in blocks for symbol in block.get("unresolved", [])
    )
    candidates = [block for block in blocks if "owner" in block]
    return {
        "scope": "read-only lexical impl census; no file is classified wholesale",
        "source_root": SRC.as_posix(),
        "source_rs_files": len(paths),
        "impl_blocks_seen": all_impls,
        "target_owner_names": sorted(target_owners),
        "target_impl_blocks": len(candidates),
        "target_owners_found": sum(value > 0 for value in counts.values()),
        "missing_target_owners": sorted(target_owners - set(counts)),
        "owner_counts": {name: counts.get(name, 0) for name in sorted(target_owners)},
        "handler_impl_blocks": sum(
            block["source_file"].startswith("crates/wow-world/src/handlers/")
            for block in candidates
        ),
        "trait_impl_blocks": sum(block["owner"]["trait"] is not None for block in candidates),
        "world_session_impl_blocks_retained": world_session,
        "cx_impl_blocks_retained": cx_impls,
        "unresolved_block_count": sum(bool(block.get("unresolved")) for block in candidates),
        "unresolved_symbol_counts": dict(sorted(unresolved_counts.items())),
        "blocks": candidates,
        "parse_errors": [block for block in blocks if "parse_error" in block],
    }


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    plan = sub.add_parser("plan", help="print the read-only impl census as JSON")
    plan.add_argument("--root", type=Path, default=REPO)
    args = parser.parse_args(argv)
    if args.command == "plan":
        print(json.dumps(build_plan(args.root), indent=2, sort_keys=True))
        return 0
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
