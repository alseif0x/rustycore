# Workspace structure programme

The phase identifiers in the first table column are inputs to
`cargo run -p xtask -- context <phase>`; the second column supplies the objective printed by that
command. Keep both machine-readable positions intact. Current completion belongs to the active
branch checkpoint and owner issue, not to historical notes in this plan.

**Authorities:** [structure-and-conventions.md](structure-and-conventions.md) owns structural
rules; [wow-world-distribution-plan.md](wow-world-distribution-plan.md) owns the `wow-world`
shape and F0–F13; this programme owns workspace-wide order and dependencies;
[refactor-completion-plan.md](refactor-completion-plan.md) owns the overall port plan.

## 1. Purpose and baseline

This programme orders structural work by real dependencies: remove workspace noise, reduce
oversized units, change contracts, then perform acceptance. Its starting measurements and the
detailed record of the 2026-09-24 campaign are preserved in the
[immutable original programme](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/architecture/workspace-structure-programme.md). Those historical counts are not current status.

Structural decisions are independent of C++ layout. Behavior still follows the versioned 3.4.3
C++ reference and its cited evidence.

## 2. Master sequence

A phase starts only after its declared dependencies are complete. Goals are concise; acceptance
evidence is defined in the owning issue and phase checkpoint.

### Wave A0 — establish the standard before crate changes

| id | objective | closure evidence | depends on |
|---|---|---|---|
| A0 | Establish the structural standard, build/tooling guardrails, and supporting decisions before crate changes. | A0.1–A0.7 are complete and their required checks are in place. | — |
| A0.1 | Decide optional subsystem features and ensure the base build does not require them. | Base build succeeds without optional features; decision recorded in the standard. | — |
| A0.2 | Add workspace `xtask` structure tools for crate audits, layer-edge checks, and reviewed policy updates. | `cargo xtask structure-audit` and `cargo xtask check-layers` pass. | — |
| A0.3 | Enable `[workspace.lints]` and per-crate `unsafe`/public-doc policies. | Lints are active and affected crates gain no warnings. | — |
| A0.4 | Audit dependency hygiene with `cargo-machete`, `cargo-deny`, and `cargo tree -d`. | Initial report recorded; work assigned to A1/C/D. | — |
| A0.5 | Record structural alternatives in ADRs and mark vendored code as exempt from size/lint budgets. | ADRs link from the standard; vendored crates are marked. | — |
| A0.6 | Set a documentation budget: keep standards concise and move history into ADRs. | Architecture documents stay at or below 300 lines. | — |
| A0.7 | Decide whether to adopt optional `cargo-nextest`, `cargo-public-api`, and (only if measured build duplication warrants it) `cargo-hakari`. | Decision recorded; adopted commands are integrated into `xtask`. | A0.2 |

Do not copy a hundreds-of-crates plugin architecture from Bevy or Zed: this workspace has no such
plugin ecosystem, and roughly 12–14 domains plus modules is the intended scale. Do not feature-gate
everything or adopt `cargo-hakari`/workspace-hack without measurements.

### Wave A — low-cost cleanup without contract or behavior changes

| id | objective | closure evidence | depends on |
|---|---|---|---|
| A1 | Fold `rustycore-db` into `wow-database`; retire `wow-pvp` and `wow-achievement`; consolidate `wow-scripts` into `wow-script`; rename the `wow-collections` utility; decide the `wow-session` and `wow-chat` boundaries. | Workspace compiles; crate count and moved-test/opcode-registration invariants are recorded. | A0 |
| A2 | Correct the `world-modules` composition-boundary audit and enforce the module-crate dependency rule. | Composition dependencies match ADR-002; module crates depend only on `wow-module-api` among server crates. | A1 |
| A3 | Classify tooling outside game layers and identify vendored navmesh code correctly. | Dependency graph and classifications match the standard. | A1 |

### Wave B — `wow-world` (details belong to its distribution plan)

| id | objective | depends on |
|---|---|---|
| B1 | F5: split `misc` into 13 domain modules. | — |
| B2 | F6: review candidate cohesive domains (`reputation`, `planner`, `profession`/`trainer`, `entity_update_bridge`, `battle_pet_*`) and move only code whose dependencies fit the owning layer. | B1 |
| B3 | F10a: move `loot_tests`, `quest_tests`, `character_tests`, and `group_tests` to their domain crates. | B2 |
| B4 | F7: design owning substates within `WorldSession`. | B2 |
| B5 | F8: design adapters of at most 600 lines for large handlers. | B4 |
| B6 | F9: move `map_manager` and its tests to `wow-map` behind a contract. | B4 |
| B7 | F10b: move remaining tests; complete F13 warnings and reviewed ownership-policy regeneration. | B5, B6 |

### Wave C — layer inversions, one small move at a time

| id | objective | depends on |
|---|---|---|
| C1 | Move `wow-data` record types to their owner and remove `data→entities` and `data→movement` edges. | A1 |
| C2 | Move loot types needed by entities/map/packet to their owner and remove `entities→loot`, `map→loot`, and `packet→loot`. | C1 |
| C3 | Correct `packet→movement` and `database→persistence` dependency directions. | C2 |
| C4 | Resolve L3 lateral edges `ai→instances`, `conditions→loot`, and `scripts→script`; document each domain boundary. | C3 |

### Wave D — remaining large units

| id | objective | priority | depends on |
|---|---|---|---|
| D1 | Split `wow-entities`' 4,726-line largest file to at most 1,000 lines, separating canonical model from gameplay state. | High | C2 |
| D2 | Split `world-server`'s 5,675-line `app.rs` into phase modules and keep composition thin. | High | B7 |
| D3 | Split `wow-map`'s 5,133-line files to at most 1,000 lines; separate grid runtime from its facade. | Medium | B6 |
| D4 | Apply file ceilings and catalog/table organization to `wow-data` and `wow-database`. | Medium | C1 |
| D5 | Apply file ceilings to `wow-packet` and `wow-social`; keep packet code wire-focused. | Medium | C3 |
| D6 | Build the module ecosystem from ADR-002: versioned `wow-module-api`, native and Wasm adapters, opt-in `wasm-runtime`, host-function allowlist, execution limits, and fault isolation. | Medium | A0.1 |

### Wave E — workspace closure

| id | objective | depends on |
|---|---|---|
| E1 | Audit the workspace against the standard: zero stubs, unused crates, prohibited layer inversions, and files over 1,000 lines. | A–D |
| E2 | Reconcile physical ledgers and policy with the tree using reviewed deltas. | E1 |
| E3 | Run one final `./tools/validation-v2 final --architecture` campaign and publish its evidence. | E2 |
| E4 | Measure build and tool latency before/after for #1231 and publish the results, including negative results. | E1 |

## 3. Current phase status

Use the active branch checkpoint and owner issue to determine which phase is current and what
evidence it has reached. Do not copy historical completion markers into a different branch or
treat a phase identifier as proof of completion.

## 4. Evidence levels

1. **Compile:** `cargo check -p <crate>` and affected consumers. This is a work loop, not delivery
   acceptance.
2. **Crate suite:** run the affected crate's tests and preserve moved-test counts:
   `#[test]`/`#[tokio::test]`, `inventory::submit!`, and scenario identities.
3. **Composition:** check `world-server --tests` and affected integration targets.
4. **Acceptance:** the planned final campaign. Only this level establishes acceptance.

A phase cannot be called complete from compile evidence alone. Record the level actually reached.

## 5. Delivery rules

1. Keep each phase coherent and record what changed and what was verified.
2. Separate code movement from behavior changes.
3. Never raise a ceiling, regenerate a policy blindly, or remove tests to pass a phase.
4. Add newly discovered structural work here before starting it.
5. At wave boundaries, rerun the applicable audit and update the current checkpoint.
6. Push, PR, and merge require their own authorization.

## 6. Architectural decisions (ADR-001–009)

These decisions were approved on 2026-09-24 unless noted. Implement each in its owning phase.

**ADR-001 — Delivery by waves.** Each wave is self-contained, green, and revertible as a merge.
For mainline work, target `origin/3.4.3` and check for overlapping paths; branch-specific work
follows its explicitly assigned base. Run `final --architecture` once per wave, not per phase;
use crate checks inside the wave. Where hosted checks skip first-party author PRs, local
acceptance evidence is the actual gate.

**ADR-002 — Native modules by default; Wasm optional.** First-party functionality builds natively;
optional subsystems are disabled by default. Native and Wasm modules share a versioned data
contract in `wow-module-api`, with owned payloads and `MODULE_API_VERSION`. Native modules use
a thin in-process adapter; Wasm serializes at the host boundary. The selected Wasm direction is
Wasmtime Component Model/WIT. Sandbox modules with an allowlist of host functions, no database
handles, packet writers, or internal-storage access, execution limits (fuel/epoch), and isolated
failure handling. Reject incompatible API versions at installation. Module crates may depend only
on `wow-module-api` among server crates; `tools/modules/compose.py` enforces this (#1445).
Generated `world-modules` and `world-server` are both composition, so the former may call
`world_server::run_with_modules`. The Rust/Wasm/C product is required; operator activation is
optional.

**ADR-003 — Functional core, imperative shell.** L0–L3 domain crates are synchronous and pure:
no `tokio`, `parking_lot`, `sqlx`, or `rand` dependencies. The application owns awaits,
locks, and persistence fences, with lock order documented per crate. Domain and application code
forbid unsafe; intents are `#[must_use]`.

**ADR-004 — Inject data, time, and randomness.** Pass catalogs by borrow (for example,
`&ItemStore`), not by pool or `Arc<dyn Store>`. Pass time as values such as `GameTime` and
`diff_ms`; domain rules do not call wall-clock APIs. Inject deterministic, seeded randomness so
tests can reproduce and compare outcomes with C++.

**ADR-005 — Errors.** Libraries expose a per-domain `#[non_exhaustive]` error enum using
`thiserror`; `anyhow` is for binaries, never public library APIs. Domain errors contain no
application or infrastructure types. The application owns each error-family effect and logging.

**ADR-006 — Generated code.** Check generated source into the repository with the
`// @generated by xtask codegen — do not edit` header. Generated files are exempt from size
ceilings, not syntax checks. `cargo xtask check-generated` regenerates and compares; do not
generate source under `src/` from `build.rs`.

**ADR-007 — Agent context and scope.** `cargo xtask context <phase>` prints at most 150 lines
from the active programme slice. `cargo xtask check-scope <phase>` fails when changed paths lie
outside that phase's declared scope.

**ADR-008 — Public API surface.** Every `pub` needs an identified consumer. Use
`#[non_exhaustive]` and sealed traits at extension points, `#[doc(hidden)]` for bridges, and
reviewed `cargo-public-api` snapshots for `wow-entities`, `wow-map`, and `wow-packet` at wave
closure. `adapter.rs` bridges are temporary and removed with their migration.

**ADR-009 — Parity naming (user decision, 2026-09-24).** `_like_cpp` and `...LikeCpp` are legacy
and must not appear in new code or refactors. Name identifiers for their domain; keep C++
provenance in comments/anchors, commits, and checkpoints. Do not mass-rename legacy names here;
rename opportunistically during real rewrites or extractions, retaining a temporary alias or
adapter only when compatibility requires it.

## 7. A0 implementation decisions

- **Features:** `modules`/`wasm-runtime`, `scripts`, and `anticheat` are optional, off by
  default; the base build must not require them. Reserved domain crate names are not features.
- **Cargo resolver:** retain resolver 2. It addresses the relevant feature-unification risk
  between build and target-specific dependencies; resolver 3's MSRV-aware resolution adds no
  current benefit with a pinned toolchain.
- **Dependency tools:** A0.4 still requires a GPL-compatible-license audit with `cargo-deny`
  and `cargo-machete` when available. Do not claim either passed based on `xtask` ratchets.
- **Documentation:** keep architecture documents at or below 300 lines; move history into ADRs or
  the immutable record.
- **Optional tools:** `cargo-nextest` is not required; public-API snapshots remain required by
  ADR-008 even if the tool is not installed; do not adopt `cargo-hakari` without measured
  feature-build duplication.

## 8. Lessons retained from Wave A

Dependency categories in `dependency-policy.json` can be stricter than the programme's layer
diagram. Check both crates' categories before moving code. When an audit script and the policy
checker disagree, follow the policy and correct the checker. Keep vendored code classified even
when exempt from size and naming rules.

## 9. Programme maintenance

Update the active checkpoint when phase evidence changes. Update this programme when order,
dependencies, goals, or budgets change; update the structural standard when a rule changes.
Keep completed campaign diagnostics and detailed exceptions in the immutable record rather than
repeating them here.

## 10. Historical record and related plans

The [full original programme and permanent 2026-09-24 campaign record](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/architecture/workspace-structure-programme.md) preserves
the initial measured baseline, phase outcomes, exceptions, and detailed diagnostics. The
[refactor completion plan](refactor-completion-plan.md) tracks core, product, and independent-audit
obligations; the [wow-world distribution plan](wow-world-distribution-plan.md) owns its F0–F13
delivery details.
