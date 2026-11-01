#!/usr/bin/env python3
"""R1 net-move check for the wow-world distribution (#1241, #1263).

"Move, never copy". The check answers two separate questions.

*Copy gate.* A normalized `fn` body that is present in
`crates/wow-world/src/` and also outside it is a copy whose origin was not
deleted. Bodies of at least MIN_DUPLICATE_CHARS normalized characters are
compared by digest, without a similarity heuristic, so a failure is always a
real identical body. Exceptions live in the reviewed allowlist of
`tools/architecture/net-move-policy.json`: an entry that no longer matches any
body fails as obsolete, so the list cannot rot. This is the rule's actual
subject and the only part that detects duplication.

*Budget.* `crates/wow-world/src/` shrink (S) pays for growth in the policy's
destination roots: the crates that receive wow-world code. Growth anywhere else
(tooling, documentation, test trees, other crates) is not a move destination,
is reported as accounted growth, and is not charged, exactly as the check's
purpose states: ordinary feature work outside a move is not a move. The
allowance is `S * (1 + ratio) + slack + reviewed new code`, where the reviewed
new-code budget is the recorded, per-phase review of the code that P4b/F4b/F5
added to the destinations; destinations may not grow past it, so unreviewed
growth still fails. Without the policy file the check keeps the original strict
form: every path outside the shrink root is charged and no new code is allowed.

Measured on `.rs` physical lines between the merge-base with `--base` and the
working tree, so staged, unstaged and untracked (nonignored) changes count.
Generated sources attributed in physical-file-policy.json are excluded, reusing
the physical inventory's policy rather than a second exclusion list.

Read-only: every git call runs with GIT_OPTIONAL_LOCKS=0 so no opportunistic
index refresh is written.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys
from typing import Any

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import physical_files as physical  # noqa: E402

SHRINK_ROOT = "crates/wow-world/src/"
SUFFIX = ".rs"
POLICY_PATH = "tools/architecture/physical-file-policy.json"
MOVE_POLICY_PATH = "tools/architecture/net-move-policy.json"
DEFAULT_RATIO = 0.05
DEFAULT_SLACK = 300
TOP = 10
MIN_DUPLICATE_CHARS = 120
SOURCE_ROOTS = ("crates/",)

assert SUFFIX in physical.SOURCE_SUFFIXES


class NetMoveError(RuntimeError):
    """A git or input failure; reported with exit status 2."""


def git(root: pathlib.Path, *args: str) -> bytes:
    environment = dict(os.environ, GIT_OPTIONAL_LOCKS="0")
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, env=environment)
    if result.returncode != 0:
        detail = result.stderr.decode("utf-8", "replace").strip()
        raise NetMoveError(f"git {' '.join(args)} failed ({result.returncode}): {detail}")
    return result.stdout


def repository_root(start: pathlib.Path) -> pathlib.Path:
    return pathlib.Path(git(start, "rev-parse", "--show-toplevel").decode("utf-8").strip())


def generated_paths(root: pathlib.Path) -> set[str]:
    policy = root / POLICY_PATH
    if not policy.is_file():
        return set()
    try:
        data = json.loads(policy.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise NetMoveError(f"cannot read {POLICY_PATH}: {exc}") from exc
    return {entry["path"] for entry in data.get("generated", []) if isinstance(entry, dict) and "path" in entry}


def line_deltas(root: pathlib.Path, base: str) -> tuple[str, dict[str, int]]:
    """Per-path net `.rs` line delta from merge-base(base, HEAD) to the working tree."""
    base_commit = git(root, "rev-parse", "--verify", "--quiet", f"{base}^{{commit}}").decode().strip()
    merge_base = git(root, "merge-base", base_commit, "HEAD").decode().strip()
    deltas: dict[str, int] = {}
    numstat = git(root, "diff", "--numstat", "-z", "--no-renames", "--no-ext-diff",
                  merge_base, "--", f"*{SUFFIX}")
    for record in numstat.decode("utf-8", "surrogateescape").split("\0"):
        if not record:
            continue
        added, deleted, path = record.split("\t", 2)
        if added == "-" or deleted == "-":
            continue  # binary content has no line count
        deltas[path] = deltas.get(path, 0) + int(added) - int(deleted)
    untracked = git(root, "ls-files", "-z", "--others", "--exclude-standard", "--", f"*{SUFFIX}")
    for path in untracked.decode("utf-8", "surrogateescape").split("\0"):
        if not path or not path.endswith(SUFFIX):
            continue
        source = root / path
        if source.is_symlink() or not source.is_file():
            continue
        with source.open("rb") as handle:
            deltas[path] = deltas.get(path, 0) + len(handle.read().splitlines())
    return merge_base, {path: delta for path, delta in deltas.items() if delta and path.endswith(SUFFIX)}


def load_move_policy(root: pathlib.Path) -> dict[str, Any]:
    """The reviewed R1 record; the strict original rule when it is absent."""
    path = root / MOVE_POLICY_PATH
    if not path.is_file():
        return {"reviewed": False, "shrink_root": SHRINK_ROOT, "destination_roots": (),
                "reviewed_new_code": 0, "duplicates": ()}
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise NetMoveError(f"cannot read {MOVE_POLICY_PATH}: {exc}") from exc
    roots = tuple(data.get("destination_roots") or ())
    if not roots:
        raise NetMoveError(f"{MOVE_POLICY_PATH}: destination_roots must not be empty")
    shrink_root = data.get("shrink_root")
    if shrink_root != SHRINK_ROOT:
        raise NetMoveError(
            f"{MOVE_POLICY_PATH}: shrink_root must be {SHRINK_ROOT!r}, found {shrink_root!r}")
    for entry in roots:
        if not isinstance(entry, str) or not entry.endswith("/"):
            raise NetMoveError(f"{MOVE_POLICY_PATH}: destination root {entry!r} must be a directory prefix")
    reviewed = data.get("reviewed_new_code")
    if not isinstance(reviewed, dict) or "budget" not in reviewed:
        raise NetMoveError(f"{MOVE_POLICY_PATH}: reviewed_new_code.budget is required")
    budget = int(reviewed["budget"])
    if budget < 0:
        raise NetMoveError(f"{MOVE_POLICY_PATH}: reviewed_new_code.budget must be nonnegative")
    duplicates = tuple(data.get("duplicates") or ())
    for entry in duplicates:
        if not isinstance(entry, dict) or not entry.get("name") or not entry.get("destination"):
            raise NetMoveError(f"{MOVE_POLICY_PATH}: duplicate entries need name and destination")
        if not entry.get("family") or not entry.get("reason"):
            raise NetMoveError(
                f"{MOVE_POLICY_PATH}: duplicate {entry['name']} needs a family and reason")
    return {
        "reviewed": True,
        "shrink_root": shrink_root,
        "destination_roots": roots,
        "reviewed_new_code": budget,
        "measured_requirement": int(reviewed.get("measured_requirement", budget)),
        "duplicates": duplicates,
        "path": MOVE_POLICY_PATH,
    }


def evaluate(deltas: dict[str, int], excluded: set[str], ratio: float = DEFAULT_RATIO,
             slack: int = DEFAULT_SLACK, policy: dict[str, Any] | None = None) -> dict[str, Any]:
    """Charge growth in move destinations against the shrink of the shrink root."""
    policy = policy or {"reviewed": False, "destination_roots": (), "reviewed_new_code": 0}
    destinations = tuple(policy.get("destination_roots") or ())

    def charged(path: str) -> bool:
        if destinations:
            return path.startswith(destinations)
        return not path.startswith(SHRINK_ROOT)

    shrink = 0
    growth = 0
    accounted = 0
    others: dict[str, int] = {}
    elsewhere: dict[str, int] = {}
    for path, delta in deltas.items():
        if path in excluded:
            continue
        if path.startswith(SHRINK_ROOT):
            shrink -= delta
        elif charged(path):
            growth += delta
            others[path] = delta
        else:
            accounted += delta
            elsewhere[path] = delta
    growing = sorted(((p, d) for p, d in others.items() if d > 0), key=lambda item: (-item[1], item[0]))
    accounted_rows = sorted(((p, d) for p, d in elsewhere.items() if d),
                            key=lambda item: (-item[1], item[0]))
    report: dict[str, Any] = {
        "shrink_root": SHRINK_ROOT,
        "destination_roots": list(destinations),
        "reviewed": bool(policy.get("reviewed")),
        "shrink": shrink,
        "growth": growth,
        "accounted_growth": accounted,
        "reviewed_new_code": int(policy.get("reviewed_new_code", 0)),
        "ratio": ratio,
        "slack": slack,
        "top_growth": [{"path": p, "delta": d} for p, d in growing[:TOP]],
        "top_accounted": [{"path": p, "delta": d} for p, d in accounted_rows[:TOP]],
    }
    if shrink <= 0:
        report.update(verdict="not-applicable", allowance=None,
                      message=f"not a move: {SHRINK_ROOT} did not shrink (shrink {shrink})")
        return report
    allowance = shrink * (1 + ratio) + slack + report["reviewed_new_code"]
    report["allowance"] = allowance
    if growth > allowance:
        if report["reviewed"]:
            message = (
                f"R1 net-move violation: growth in the reviewed move destinations is {growth} lines, "
                f"exceeding the allowance {allowance:.1f} (shrink {shrink} * (1 + {ratio}) + {slack} + "
                f"reviewed new code {report['reviewed_new_code']}) by {growth - allowance:.1f} lines; "
                f"delete the moved source in the same change or record the reviewed growth in "
                f"{MOVE_POLICY_PATH}")
        else:
            message = (
                f"R1 net-move violation: growth outside {SHRINK_ROOT} is {growth} lines, exceeding the "
                f"allowance {allowance:.1f} (shrink {shrink} * (1 + {ratio}) + {slack}) by "
                f"{growth - allowance:.1f} lines; delete the moved source in the same change")
        report.update(verdict="fail", message=message)
    else:
        report.update(verdict="pass", message=(
            f"net move within allowance: growth {growth} <= {allowance:.1f} "
            f"({accounted} accounted lines elsewhere are not charged)"))
    return report


def without_comments(source: str) -> str:
    """Drop `//`, `/* */` and string literals so copies compare on code."""
    out: list[str] = []
    index = 0
    length = len(source)
    while index < length:
        char = source[index]
        if char == "/" and index + 1 < length and source[index + 1] == "/":
            end = source.find("\n", index)
            index = length if end < 0 else end
            continue
        if char == "/" and index + 1 < length and source[index + 1] == "*":
            end = source.find("*/", index + 2)
            index = length if end < 0 else end + 2
            continue
        if char == '"':
            index += 1
            while index < length:
                if source[index] == "\\":
                    index += 2
                    continue
                if source[index] == '"':
                    index += 1
                    break
                index += 1
            continue
        out.append(char)
        index += 1
    return "".join(out)


def body_index(root: pathlib.Path, path: pathlib.Path) -> dict[tuple[str, str], list[tuple[str, int]]]:
    """`{(name, digest): [(path, lines)]}` for one file."""
    index: dict[tuple[str, str], list[tuple[str, int]]] = {}
    try:
        source = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return index
    stripped = without_comments(source)
    for match in re.finditer(r"\bfn\s+([A-Za-z0-9_]+)", stripped):
        opening = stripped.find("{", match.end())
        if opening < 0 or ";" in stripped[match.end():opening]:
            continue
        depth = 0
        index_end = opening
        while index_end < len(stripped):
            char = stripped[index_end]
            if char == "{":
                depth += 1
            elif char == "}":
                depth -= 1
                if depth == 0:
                    break
            index_end += 1
        body = re.sub(r"\s+", " ", stripped[opening:index_end + 1]).strip()
        if len(body) < MIN_DUPLICATE_CHARS:
            continue
        digest = hashlib.sha256(body.encode("utf-8")).hexdigest()
        relative = path.relative_to(root).as_posix()
        lines = stripped[opening:index_end + 1].count("\n") + 1
        index.setdefault((match.group(1), digest), []).append((relative, lines))
    return index


def is_test_tree(relative: str) -> bool:
    """Test trees are not move destinations of shipped code."""
    return "/unit_tests/" in relative or "/tests/" in relative


def duplicate_bodies(root: pathlib.Path, policy: dict[str, Any]) -> dict[str, Any]:
    """Identical `fn` bodies present in the shrink root and outside it."""
    origins: dict[tuple[str, str], list[tuple[str, int]]] = {}
    shrink = root / SHRINK_ROOT.rstrip("/")
    for path in sorted(shrink.rglob(f"*{SUFFIX}")):
        for key, value in body_index(root, path).items():
            origins.setdefault(key, []).extend(value)
    allowed = {(entry["name"], entry["destination"]): entry for entry in policy.get("duplicates") or ()}
    matched: set[tuple[str, str]] = set()
    violations: list[dict[str, Any]] = []
    seen: set[tuple[str, str, str]] = set()
    for base in SOURCE_ROOTS:
        area = root / base.rstrip("/")
        if not area.is_dir():
            continue
        for path in sorted(area.rglob(f"*{SUFFIX}")):
            relative = path.relative_to(root).as_posix()
            if relative.startswith(SHRINK_ROOT) or is_test_tree(relative):
                continue
            for key, value in body_index(root, path).items():
                if key not in origins:
                    continue
                entry = allowed.get((key[0], relative))
                if entry is None:
                    for origin_path, lines in origins[key]:
                        record = (key[0], origin_path, relative)
                        if record in seen:
                            continue
                        seen.add(record)
                        violations.append({"name": key[0], "origin": origin_path,
                                           "destination": relative, "lines": lines})
                    continue
                matched.add((key[0], relative))
    obsolete = [entry for key, entry in allowed.items() if key not in matched]
    return {
        "violations": violations,
        "allowlisted": [allowed[key] for key in sorted(matched)],
        "obsolete": obsolete,
    }


def render(report: dict[str, Any]) -> str:
    allowance = "n/a" if report["allowance"] is None else f"{report['allowance']:.1f}"
    lines = [
        f"net-move base: {report['base']} (merge-base {report['merge_base']})",
        f"S (shrink of {report['shrink_root']}): {report['shrink']}",
        f"G (net growth charged): {report['growth']}",
        f"accounted growth elsewhere (not charged): {report['accounted_growth']}",
        f"allowance: {allowance} (ratio {report['ratio']}, slack {report['slack']}, "
        f"reviewed new code {report['reviewed_new_code']})",
        "top growing paths:",
    ]
    lines.extend(f"  {row['delta']:+d} {row['path']}" for row in report["top_growth"])
    if not report["top_growth"]:
        lines.append("  (none)")
    lines.append("top accounted paths:")
    lines.extend(f"  {row['delta']:+d} {row['path']}" for row in report["top_accounted"])
    if not report["top_accounted"]:
        lines.append("  (none)")
    copies = report.get("copies") or {}
    lines.append(f"duplicate bodies: {len(copies.get('violations') or [])} violations, "
                 f"{len(copies.get('allowlisted') or [])} allowlisted, "
                 f"{len(copies.get('obsolete') or [])} obsolete")
    lines.append(f"{report['verdict'].upper()}: {report['message']}")
    return "\n".join(lines)


def check(args: argparse.Namespace) -> int:
    if args.ratio < 0 or args.slack < 0:
        print("net_move: --ratio and --slack must be nonnegative", file=sys.stderr)
        return 2
    root = repository_root(pathlib.Path.cwd())
    merge_base, deltas = line_deltas(root, args.base)
    policy = load_move_policy(root)
    report = evaluate(deltas, generated_paths(root), args.ratio, args.slack, policy)
    copies = duplicate_bodies(root, policy)
    report["copies"] = copies
    if copies["violations"] or copies["obsolete"]:
        details = []
        for row in copies["violations"]:
            details.append(f"copy of {row['name']} in {row['destination']} keeps its origin "
                           f"{row['origin']} ({row['lines']} lines)")
        for entry in copies["obsolete"]:
            details.append(f"obsolete duplicate allowlist entry {entry['name']} in "
                           f"{entry['destination']}")
        report.update(verdict="fail", message=(
            "R1 net-move violation: " + "; ".join(details) +
            f"; delete the origin or record the reviewed exception in {MOVE_POLICY_PATH}"))
    report.update(base=args.base, merge_base=merge_base)
    print(json.dumps(report, indent=2, sort_keys=True) if args.json else render(report))
    return 1 if report["verdict"] == "fail" else 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    commands = parser.add_subparsers(dest="command", required=True)
    checker = commands.add_parser("check", help="compare wow-world shrink with growth elsewhere")
    checker.add_argument("--base", required=True, help="base revision; its merge-base with HEAD is used")
    checker.add_argument("--ratio", type=float, default=DEFAULT_RATIO)
    checker.add_argument("--slack", type=int, default=DEFAULT_SLACK)
    checker.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    try:
        return check(args)
    except NetMoveError as exc:
        print(f"net_move: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
