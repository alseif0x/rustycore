#!/usr/bin/env python3
"""Transitive-closure check over the wow-world* crates (#1363 slice A).

Usage:
  python3 tools/architecture/wow_world_closure.py check
      [--repo PATH] [--source lock|metadata] [--add A->B ...]

--source lock     (default) edges from <repo>/Cargo.lock, every dependency
                  kind; a package's dependency on itself (a self
                  dev-dependency that enables features) is dropped. Needs no
                  Cargo invocation.
--source metadata edges from `cargo metadata --no-deps --offline`, normal and
                  build dependencies only (dev-dependencies excluded).
--add A->B        simulate a planned edge before moving anything, e.g.
                  --add wow-world-inventory->wow-world-lifecycle.

Rules (exit 1 on any violation):
  * no wow-world* package reaches itself (cycle);
  * no domain crate reaches wow-world-application or wow-world;
  * inventory, spell, entities and loot never reach social or instances
    (social depends on inventory and spell, instances on social);
  * wow-world-character sits high: only wow-world-application and wow-world
    may reach it (no domain crate below it, including wow-world-core);
  * wow-world-quest sits above the domain crates: only wow-world-application,
    wow-world and wow-world-loot-ops may reach it;
  * wow-world-loot-ops (loot operations above quest) sits high: only
    wow-world-application and wow-world may reach it.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tomllib
from pathlib import Path

PREFIX = "wow-world"
TOP = frozenset({"wow-world", "wow-world-application"})
LOW = ("wow-world-inventory", "wow-world-spell", "wow-world-entities", "wow-world-loot")
HIGH = frozenset({"wow-world-social", "wow-world-instances"})
CHARACTER = "wow-world-character"
QUEST = "wow-world-quest"
LOOT_OPS = "wow-world-loot-ops"
QUEST_USERS = TOP | {LOOT_OPS}

Edges = dict[str, set[str]]


def lock_edges(lock_text: str) -> Edges:
    edges: Edges = {}
    for package in tomllib.loads(lock_text).get("package", []):
        name = package["name"]
        if not name.startswith(PREFIX):
            continue
        deps = {dep.split(" ")[0] for dep in package.get("dependencies", [])}
        edges[name] = {dep for dep in deps if dep.startswith(PREFIX) and dep != name}
    return edges


def metadata_edges(metadata: dict) -> Edges:
    edges: Edges = {}
    for package in metadata["packages"]:
        name = package["name"]
        if not name.startswith(PREFIX):
            continue
        edges[name] = {
            dep["name"]
            for dep in package["dependencies"]
            if dep["name"].startswith(PREFIX)
            and dep["kind"] in (None, "build")
            and dep["name"] != name
        }
    return edges


def closure(edges: Edges, start: str) -> set[str]:
    seen: set[str] = set()
    stack = list(edges.get(start, ()))
    while stack:
        node = stack.pop()
        if node not in seen:
            seen.add(node)
            stack.extend(edges.get(node, ()))
    return seen


def violations(edges: Edges) -> list[str]:
    found = []
    for name in sorted(edges):
        reach = closure(edges, name)
        if name in reach:
            found.append(f"cycle: {name} reaches itself")
        if name not in TOP and reach & TOP:
            found.append(f"{name} reaches {sorted(reach & TOP)}")
        if name in LOW and reach & HIGH:
            found.append(f"{name} reaches {sorted(reach & HIGH)}")
        if name not in TOP and name != CHARACTER and CHARACTER in reach:
            found.append(f"{name} reaches ['{CHARACTER}']")
        if name not in QUEST_USERS and name != QUEST and QUEST in reach:
            found.append(f"{name} reaches ['{QUEST}']")
        if name not in TOP and name != LOOT_OPS and LOOT_OPS in reach:
            found.append(f"{name} reaches ['{LOOT_OPS}']")
    return found


def add_edges(edges: Edges, planned: list[str]) -> None:
    for item in planned:
        source, separator, target = item.partition("->")
        if not separator or not source or not target:
            raise ValueError(f"--add expects A->B, got {item!r}")
        edges.setdefault(source, set()).add(target)
        edges.setdefault(target, set())


def report(edges: Edges) -> str:
    short = lambda names: ", ".join(sorted(n.removeprefix(PREFIX + "-") for n in names)) or "-"
    return "\n".join(
        f"{name}\n  direct: {short(edges[name])}\n  closure: {short(closure(edges, name))}"
        for name in sorted(edges)
    )


def check(repo: Path, source: str, planned: list[str]) -> int:
    if source == "lock":
        edges = lock_edges((repo / "Cargo.lock").read_text(encoding="utf-8"))
    else:
        out = subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--no-deps", "--offline"],
            cwd=repo, check=True, capture_output=True, text=True,
        ).stdout
        edges = metadata_edges(json.loads(out))
    add_edges(edges, planned)
    for item in planned:
        print(f"simulated edge: {item}")
    print(report(edges))
    found = violations(edges)
    print(f"wow-world closure ({source}): {len(edges)} packages, "
          f"{'violations: ' + '; '.join(found) if found else 'PASS'}")
    return 1 if found else 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("check")
    run.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[2])
    run.add_argument("--source", choices=("lock", "metadata"), default="lock")
    run.add_argument("--add", action="append", default=[], metavar="A->B")
    args = parser.parse_args(argv)
    try:
        return check(args.repo, args.source, args.add)
    except ValueError as error:
        parser.error(str(error))


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
