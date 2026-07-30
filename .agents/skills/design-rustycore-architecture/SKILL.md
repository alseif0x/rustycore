---
name: design-rustycore-architecture
description: Design and audit RustyCore architecture using evidence from the current Rust workspace and the legacy C++ server. Use when deciding crate or module boundaries, assigning canonical state ownership, untangling dependencies, decomposing project-wide hotspots such as session, handlers, map, player, world-server, data, packets, or QA tooling, planning runtime convergence, or creating an incremental architecture/refactor campaign. Do not use for a small behavior fix that has no structural decision; use refactor-rustycore-safely when implementing an already approved behavior-preserving restructuring.
---

# Design RustyCore Architecture

Produce an evidence-backed target architecture and an incremental migration path. Optimize for
one canonical owner per mutable state, C++-faithful behavior, narrow public APIs, directional
dependencies, and PR-sized delivery.

## Load the required context

1. Read the repository `AGENTS.md` completely and follow its session kickoff.
2. Read `docs/migration/STATE.md` and the relevant `docs/migration/PORT_PLAN.md` sections. For
   `docs/migration/adr-runtime-tick-ownership.md`, read its context, decision, risks, and the
   current slice sections relevant to the task; do not load its entire historical progress log
   for a bounded decision.
3. Read [references/decision-rules.md](references/decision-rules.md) for every architecture
   decision.
4. Read [references/current-architecture.md](references/current-architecture.md) for workspace-wide
   audits or changes involving session, network, runtime, map, player, data, handlers, binaries, or
   large-file decomposition.
5. Locate the exact C++ owner and call path under `/home/server/woltk-trinity-legacy` before
   assigning gameplay responsibility. Treat C++ as the behavioral oracle, not as a required Rust
   file layout.

Do not trust old migration percentages, crate names, comments, or previous summaries as proof of
current ownership.

## Architecture workflow

### 1. Establish the current baseline

Inspect the current branch, latest commits, workspace manifest, target crate manifests, module
trees, public re-exports, largest production files, and internal dependency edges. Recalculate
metrics on HEAD; never copy stale counts into a decision.

Use `rg`, `rg --files`, `cargo metadata --format-version 1 --no-deps`, `cargo tree`, and focused
source inspection. Distinguish production logic from inline tests and generated or declarative
code.

### 2. Map ownership before proposing folders

For every important mutable state, record:

- semantic owner in C++;
- current Rust owner or owners;
- all writers and readers;
- mirrors, clones, caches, registries, and sync directions;
- lock/task/tick owner;
- persistence owner;
- client-visible publication path;
- intended canonical owner;
- exact retirement condition for every temporary mirror.

Do not propose concurrency, actors, worker pools, or new abstractions until the state owner is
unambiguous.

### 3. Diagnose the architectural defect

Classify each finding as one or more of:

- mixed responsibilities inside one owner;
- duplicated authority or transitional mirror;
- dependency inversion;
- infrastructure leaking into domain/application logic;
- application orchestration leaking into network or binaries;
- overly broad public API;
- registration or dispatch with multiple sources of truth;
- file-size or test-layout symptom without an ownership defect;
- intentional aggregate that is large but cohesive.

Prioritize duplicated authority and incorrect dependency direction over cosmetic file splitting.

### 4. Select modules, crates, ports, and tasks deliberately

Apply the decision rules from the reference:

- start with a private module when ownership or API is still changing;
- promote to a crate only with a stable owner, narrow contract, downward dependencies, and useful
  independent tests/build;
- introduce a trait only at a real adapter boundary or when a deterministic fake provides value;
- introduce a task/channel only when one owner must serialize state or I/O and backpressure is
  explicit;
- keep Tokio, sockets, SQL, DB2 loading, filesystem, and packet presentation at the edges;
- keep simulation and rules synchronous and deterministic where practical.

### 5. Define the target and dependency direction

Describe:

- the canonical owner of each state;
- the allowed dependency direction;
- the small public commands, queries, outcomes, and events at each boundary;
- where persistence and packet conversion occur;
- how C++ phase order and observable behavior remain preserved;
- which legacy bridge remains temporarily and how it will disappear.

Prefer a modular monolith. Do not propose microservices merely to obtain code organization.

### 6. Produce an incremental migration campaign

Split the work so every PR has one dominant change class:

- mechanical relocation with unchanged ownership and behavior;
- dependency-boundary change;
- ownership migration with one source of truth;
- intentional behavior change or proven legacy defect repair.

Never combine all four. Preserve compatibility paths with temporary re-exports where useful, but
attach a deletion condition. Require focused positive/negative tests and capture-diff for
client-visible behavior.

## Hard invariants

- Preserve the accepted single tick-owner invariant and C++ `Map::Update` phase order.
- Never add a second canonical `Player`, `Creature`, `Map`, combat relation, loot authority, or
  persistence owner.
- Never retain a lock across `.await` or send packets while holding a map lock.
- Build plans or outcomes under the owner; persist and publish in the documented order outside
  incompatible locks.
- Keep packet handlers thin: decode, session gate, construct command, invoke use case, present
  result.
- Keep gameplay rules out of `wow-network`, SQL adapters, packet DTOs, and composition roots.
- Keep public visibility narrow. Do not make internals `pub` merely to move files or test them.
- Do not populate an empty domain crate by moving unstable code solely to satisfy the workspace
  diagram.
- Follow `docs/migration/STATE.md` fidelity policy for proven C++ defects. Separate a deliberate
  correction from a behavior-preserving refactor.

## Required output

Lead with a verdict. Then provide:

1. evidence with exact Rust and C++ anchors;
2. current owners, mirrors, and dependency defects;
3. target ownership and dependency direction;
4. module-versus-crate decisions with reasons;
5. a risk-ranked sequence of PR-sized slices;
6. invariants and verification for each slice;
7. explicit deferrals and bridge-retirement conditions.

If implementation is requested after approval, hand the selected slice to
`$refactor-rustycore-safely`.
