---
name: develop-rustycore
description: "Day-to-day placement and conventions for any RustyCore code change (feature, fix or refactor) in wow-world, world-server, world-modules or a domain crate: where new state, logic and tests go in the split WorldSession, the thunk rule, visibility, drop order, locks, and the per-PR acceptance checklist. Use before editing those crates."
---

# Develop in RustyCore's structure

This skill turns the current structure into actions, so nobody relies on memory. It does not
own the rules. The rules come from:

- [AGENTS.md](../../../AGENTS.md): scope, authority, validation and Git;
- [wow-world-distribution-plan.md](../../../docs/architecture/wow-world-distribution-plan.md):
  the #1241 programme, with rules R1–R5 (§3) and module compatibility M1–M4 (§4);
- [structure-and-conventions.md](../../../docs/architecture/structure-and-conventions.md):
  layers, naming, visibility, test placement and budgets.

If this skill and one of those documents disagree, the document wins. Fix the skill.

## When to use, and the other skills

Use this skill for every feature, fix or refactor that touches `crates/wow-world`, `world-server`,
`world-modules` or a `wow-world-<domain>` crate. It tells you where code goes and what a PR must
prove.

- [refactor-rustycore-safely](../refactor-rustycore-safely/SKILL.md): structural moves and
  ownership migration.
- [design-rustycore-architecture](../design-rustycore-architecture/SKILL.md): new boundaries,
  new crates, new top-level state, or an unresolved owner.
- [orchestrate-rustycore](../orchestrate-rustycore/SKILL.md): parent/worker routing.

Behavior still needs its C++ anchors and tests (AGENTS.md "Fidelity"). Placement is no evidence
of parity.

## Current layout, and how to verify it

Re-measure before you rely on any count:
`python3 tools/architecture/wow_world_coupling.py report [--json]`. It prints fields per domain,
hub fields, cross-domain edges, DAG violations and the R5 metric. Do not copy counts from here
or from older PRs.

- `WorldSession` (crates/wow-world/src/session/state.rs) is `core: SessionCore` plus one member
  per domain (`lifecycle`, `loot`, `inventory`, `spell_state`, `instances`, ...). The sub-state
  structs are in `crates/wow-world/src/session/state/*.rs`.
- `HubRef`/`HubMut` (core, catalogs, config, plus fixtures under cfg(test)) and the builders
  `hub_ref`, `hub_mut` and `split_<group>[_ref|_mut]` are in `session/state/hub.rs`. A split builder
  returns the group's state plus a hub view, borrowed from disjoint fields.
- Test-only state is behind `#[cfg(test)] fixtures: SessionFixtures` (`session/state/fixtures.rs`).
- Unit tests are in `crates/wow-world/unit_tests/`, at paths that mirror `src/`. They are mounted
  from the source file with `#[cfg(test)] #[path = "../../unit_tests/<mirrored path>.rs"] mod x;`.
- Integration tests (public API, production composition) are in `crates/wow-world/tests/`.

## Where new code goes

**State**
- Do: put it in the owning `<Domain>State`. Hub data (identity, connection, map and registry
  handles) belongs in `SessionCore`.
- Don't: add a new top-level `WorldSession` field without a design decision
  (design-rustycore-architecture). Don't add a second mutable mirror of existing state.

**Logic**
- Do: write `impl <Domain>State`, or `impl HubRef<'_>`/`impl HubMut<'_>` when the logic only
  touches the hub. Take the narrowest context: your own state plus `hub: HubRef<'_>` or
  `&mut HubMut<'_>`, built by a `split_*` builder from disjoint fields.
- Don't: use a universal context, pass `&mut WorldSession` into domain logic, or add a trait,
  lock, clone or `pub` field only to get past the borrow checker.
- Add no new `impl WorldSession` fn. The only exceptions are a packet handler registered through
  `PacketHandlerEntry` (session/registry.rs), a permanent API thunk such as `player_guid()`, or a
  temporary thunk under the thunk rule.

**The thunk rule.** Moved code never calls a `WorldSession` thunk; it calls the new owner. A thunk
exists only while an unmoved caller remains. The PR that moves the last caller deletes it.

**Test-only code**
- Test-only state goes in `fixtures.<group>`.
- A test-only entry point that unit tests still call on `WorldSession` goes in a
  `#[cfg(test)] impl WorldSession` shim in the unit_tests `f3_shims` file next to the mirrored
  path (the f3 codemod creates and retires these).
- New tests go at the mirrored unit_tests path.
- #[path] nesting: a nested `mod x;` inside a #[path]-loaded file resolves next to that file,
  not under a directory named after it.
- Never use a path segment that `.gitignore` matches. PR #1249 shipped a missing file because a
  `skills/` segment was ignored. Use a flat name such as `skills_f3_shims.rs`.
  `python3 tools/architecture/ignored_sources.py check` catches this.

**Visibility.** Use the narrowest visibility that compiles; structure-and-conventions §5 gives the
ladder. Don't add `pub` fields or wrappers to relocate code. A moved member keeps its visibility.

**Drop order.** `WorldSession`'s declaration order is its drop order. Read the comment above
`pub struct WorldSession` in session/state.rs before adding or reordering members. Never reorder
the side-effecting members it lists.

**Locks and await.** Never hold a std, map or entity guard across a self call, an `.await`, I/O or
packet delivery.

**Persistence and COMMIT fences.** Keep each body's sequence (validate, transaction, commit
classification, application, publication). A borrow scope may narrow but must never widen. A
fence fn moves byte for byte.

**Source-text tests.** Some tests use `include_str!` on a source file and pin call text. Before you
edit a pinned call, find them with `rg -lU 'include_str!\(\s*"[^"]*src/' crates/wow-world/unit_tests`.

**Modules (#583, plan M1–M4).** `wow-module-api` and `wow-script` stay foundation crates: domain
crates may depend on them, never the reverse. The module surface never exposes `WorldSession` or
a `<Domain>State`. Hook call points move unchanged (same point, order and context). New hooks are
#583 work. `world-server`/`world-modules` stay the only composition points.

## Programme workflow for structural moves

Read [refactor-rustycore-safely](../refactor-rustycore-safely/SKILL.md) first. For #1241:

- Open one PR per phase or domain and merge it continuously into `3.4.3`. Don't keep a
  long-lived WIP branch.
- Move, never copy (R1). The same commit deletes the source. Check it with
  `python3 tools/architecture/net_move.py check --base origin/3.4.3`.
- A move is not a redesign (R2): no semantic change, no rename, no new canonical path. Behavior
  changes go to F6 with parity evidence.
- Do mechanical moves with the compiler-guided codemods in `tools/codemods/` (R4): `f3_move_methods.py`,
  its library `f3_codemod_lib.py` and its self-test `test_f3_move_methods.py`. The loop is:
  1. `plan --group <g>` and review it;
  2. `apply --group <g>`, then let the compiler loop finish;
  3. `cargo fmt --all`;
  4. run a second `apply`, which must be a no-op.
  Fix the codemod rather than hand-editing its output. Run its self-test after a change.
- Review every regenerated baseline (field/item baselines, physical rows, hotspot ratchet, handler
  contract). Never regenerate one blindly. Raise a ceiling only with a one-sentence reason, and
  tighten the rows that shrank.

## Per-PR acceptance checklist

Run these in order, one at a time, on the committed candidate, with this environment:

```bash
export PROTOC=/home/ubuntu/.local/protoc/bin/protoc CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=<checkout>/target   # per worktree, absolute
```

1. `python3 tools/architecture/ignored_sources.py check`. Also confirm that
   `git status --short --ignored -- crates` shows no untracked or ignored source the build needs.
2. Check matrix, all exit 0: `cargo check -p wow-world --all-targets`,
   `cargo check -p wow-world --all-targets --features test-fixtures`,
   `cargo check -p world-server --all-targets` and `cargo check -p world-modules --all-targets`.
   Add a moved domain crate if there is one. Check for new warnings: there must be none.
3. Save `cargo test -p wow-world -- --list` under `$CARGO_TARGET_DIR`, at the base and at the
   candidate, and diff the two. They must match, unless the PR adds tests on purpose. Then list
   the added tests.
4. `cargo test -p wow-world` (the full suite). Report passed, failed and ignored.
5. `python3 tools/architecture/check_architecture.py check --self-test` and session ownership
   `--syntax-only` (the command is in AGENTS.md). Both also run inside step 8; on their own they
   are only a quick pre-check.
6. `python3 tools/architecture/net_move.py check --base origin/3.4.3` for a move.
7. `git diff --check` and `cargo fmt --all --check`.
8. `./tools/validation-v2 final --base origin/3.4.3 --architecture --timings --logs`, then
   `./tools/validation-v2 verify --manifest <path> --require-profile final`.

Keep each command's real exit status. Save long output to a log; never let a pipe mask the exit.
A zero-test filter proves nothing.

The PR body records:
- the candidate SHA and whether the tree was clean;
- the campaign start/end and duration. The budget is 600 s; report an overrun, don't hide it;
- R5 before → after (from the coupling report);
- the baseline deltas you reviewed.

## Git hygiene

- Stage exact paths only, and leave unrelated dirty work alone.
- Squash-merge each phase/domain PR into `3.4.3`. Merging and pushing still need their own
  authorization (AGENTS.md).
- Owner's rule: commit messages and PR bodies carry no AI co-author trailer and no
  "Generated with" attribution line.

## What to report when a unit is finished

Report back to the parent with:
- the model and effort that actually ran;
- the base SHA and the diff identity (`git diff --stat` or the SHA);
- the changed paths and symbols;
- each check with its real exit, and the test counts (confirm the filter ran tests);
- R5 before → after when `impl WorldSession` counts changed;
- what was not run, any blocked fns or thunks left, and concrete blockers or uncertainty.

Keep implementation evidence separate from accepted evidence.
