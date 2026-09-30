#!/usr/bin/env python3
"""R1 net-move check for the wow-world distribution (#1241).

"Move, never copy": when crates/wow-world/src shrinks, the rest of the
repository may grow by at most that shrink plus a small allowance. A copy that
leaves the source in place (or deletes only part of it) grows the destination
far beyond the shrink and fails. Ordinary feature work that grows wow-world is
not a move and is not checked.

Measured on `.rs` physical lines between the merge-base with `--base` and the
working tree, so staged, unstaged and untracked (nonignored) changes count.
Generated sources attributed in physical-file-policy.json are excluded, reusing
the physical inventory's policy rather than a second exclusion list.

Read-only: every git call runs with GIT_OPTIONAL_LOCKS=0 so no opportunistic
index refresh is written.
"""
from __future__ import annotations

import argparse
import json
import os
import pathlib
import subprocess
import sys
from typing import Any

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import physical_files as physical  # noqa: E402

SHRINK_ROOT = "crates/wow-world/src/"
SUFFIX = ".rs"
POLICY_PATH = "tools/architecture/physical-file-policy.json"
DEFAULT_RATIO = 0.05
DEFAULT_SLACK = 300
TOP = 10

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


def evaluate(deltas: dict[str, int], excluded: set[str], ratio: float, slack: int) -> dict[str, Any]:
    shrink = 0
    growth = 0
    others: dict[str, int] = {}
    for path, delta in deltas.items():
        if path in excluded:
            continue
        if path.startswith(SHRINK_ROOT):
            shrink -= delta
        else:
            growth += delta
            others[path] = delta
    growing = sorted(((p, d) for p, d in others.items() if d > 0), key=lambda item: (-item[1], item[0]))
    report: dict[str, Any] = {
        "shrink_root": SHRINK_ROOT,
        "shrink": shrink,
        "growth": growth,
        "ratio": ratio,
        "slack": slack,
        "top_growth": [{"path": p, "delta": d} for p, d in growing[:TOP]],
    }
    if shrink <= 0:
        report.update(verdict="not-applicable", allowance=None,
                      message=f"not a move: {SHRINK_ROOT} did not shrink (shrink {shrink})")
        return report
    allowance = shrink * (1 + ratio) + slack
    report["allowance"] = allowance
    if growth > allowance:
        report.update(verdict="fail", message=(
            f"R1 net-move violation: growth outside {SHRINK_ROOT} is {growth} lines, exceeding the "
            f"allowance {allowance:.1f} (shrink {shrink} * (1 + {ratio}) + {slack}) by "
            f"{growth - allowance:.1f} lines; delete the moved source in the same change"))
    else:
        report.update(verdict="pass", message=(
            f"net move within allowance: growth {growth} <= {allowance:.1f}"))
    return report


def render(report: dict[str, Any]) -> str:
    allowance = "n/a" if report["allowance"] is None else f"{report['allowance']:.1f}"
    lines = [
        f"net-move base: {report['base']} (merge-base {report['merge_base']})",
        f"S (shrink of {report['shrink_root']}): {report['shrink']}",
        f"G (net growth elsewhere): {report['growth']}",
        f"allowance: {allowance} (ratio {report['ratio']}, slack {report['slack']})",
        "top growing paths:",
    ]
    lines.extend(f"  {row['delta']:+d} {row['path']}" for row in report["top_growth"])
    if not report["top_growth"]:
        lines.append("  (none)")
    lines.append(f"{report['verdict'].upper()}: {report['message']}")
    return "\n".join(lines)


def check(args: argparse.Namespace) -> int:
    if args.ratio < 0 or args.slack < 0:
        print("net_move: --ratio and --slack must be nonnegative", file=sys.stderr)
        return 2
    root = repository_root(pathlib.Path.cwd())
    merge_base, deltas = line_deltas(root, args.base)
    report = evaluate(deltas, generated_paths(root), args.ratio, args.slack)
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
