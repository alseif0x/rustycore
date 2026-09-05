# Runtime clock and phase ownership trace

Issue #188 introduced this trace. **Bounded source review: 2026-09-05, `7eaf8ddc`**
(production code unchanged from `93e4002a`). It covers the world-server map/creature loops,
three supporting periodic tasks and the diagnostic Session creature path; it is not an
exhaustive inventory of every timer in every process. The machine-readable inventory is
[`tools/architecture/runtime-clock-phase-trace.json`](../../tools/architecture/runtime-clock-phase-trace.json)
and `python3 tools/architecture/check_architecture.py check` checks its declarations and anchors.
Current spawn/call paths and affected tests must establish execution; a green metadata check
does not prove reachability, exact phase order or lock safety.

## Why a trace before a cut

The convergence plan in [`adr-runtime-tick-ownership.md`](../migration/adr-runtime-tick-ownership.md)
removes one legacy writer at a time. Every such cut needs to know which clock currently owns the
transition and what stops a second owner resolving it. Prior notes overstated how much of C++'s
`World::Update → MapManager::Update → Map::Update` order the canonical skeleton preserves, and
disagreed with the code about whether the global creature loop is on. This reviewed trace
corrects those descriptions without claiming that declaration checks prove runtime behavior.

## The six traced paths

| Clock | Availability | Cadence | Diff source | Owns |
|---|---|---|---|---|
| `canonical_map_update` | production | configured map interval | measured elapsed, zero-diff skipped | canonical grid/spawn/respawn, area triggers, game events |
| `legacy_creature_runtime` | production | configured map interval | measured elapsed, propagated once into creature logical time | creature lifecycle, movement, aggro, spell, melee, respawn; selected-owner player melee from #28 |
| `group_ready_check` | production | own interval | loop interval | ready-check expiry |
| `realm_list_update` | production | own interval | loop interval | realm list refresh |
| `db_keepalive` | production | own interval | loop interval | connection keepalive |
| `session_creature_tick` | **diagnostic** | session task | session diff | nothing in production |

`app.rs` starts the canonical and legacy loops and the ready-check/keepalive tasks during
world startup; it starts the realm-list task earlier. The diagnostic row concerns creature
updates only: Session still has other timers and gameplay work tracked by #578.

The former seventh row, `world_update`, was incorrect. `world_update_loop_step_like_cpp`
(`crates/world-server/src/lib.rs`) is a timing helper called by `main_tests.rs`, including
`world_update_loop_step_matches_cpp_timing_contract`; startup does not spawn or invoke it.
Its test does not establish a third production world/map simulation loop. Preserve that test
as bounded helper evidence, not as a runtime producer or a pending instruction to launch one.

## What the guard enforces

For every clock the checker requires a source file and entry point that exist, a stated cadence,
diff source and availability, at least one named resolution guard, and regression anchors that
resolve to real tests. It rejects any clock that declares `delivers_under_map_guard: true`,
because "no packet or cross-session command delivery while a map lock is held" is a
non-negotiable campaign invariant. Three adversarial cases are exercised: a missing entry point,
a clock admitting delivery under a guard, and a regression anchor no test defines.
An empty regression-anchor list is currently accepted; a descriptive non-identifier entry is
not resolved to a function. These checks neither analyze bodies/call graphs nor execute tests.

## Single resolution

The canonical and legacy paths can touch related creature state. Their recorded AI/combat
split prevents the canonical visitor from independently executing the selected legacy transition:

- `legacy_creature_runtime` only acts when `RuntimeTickOwner` is `GlobalLegacy`, and the session
  path reads the same shared owner and skips.
- `canonical_map_update` dispatches no AI or combat side effects at all, so its visit cannot
  independently resolve that AI/combat work. Spawn/respawn and synchronization bridges still
  require their explicit ownership/phase contracts; this is not proof that every transition is unified.

The existing anchors pin both facts:
`two_sessions_sharing_legacy_map_manager_see_same_creature_state` and
`canonical_map_update_visits_creature_with_no_real_ai_combat_effect_like_cpp`.

Within the selected legacy owner, #371 removed the former second time source
(`WorldCreature::clock_started_at.elapsed()`). The loop's measured `diff_ms` now advances one
logical elapsed value exactly once at the `Unit::Update` boundary. Spline state, MotionMaster,
melee, spell, assistance, corpse and respawn deadlines read that same value; scheduler time between
calls cannot advance any of them independently.

## Delivery boundary

The required invariant is no packet/command delivery under a map guard. The reviewed
`run_legacy_creature_*_and_deliver_once_like_cpp` bridges collect outcomes inside the lock and
deliver after releasing it; the canonical loop has an analogous boundary for its recipients.
The JSON records that declaration but cannot detect a new send hidden inside a function body.
Affected source review, production-linked tests and backpressure cases must verify the boundary
at the [#578 checkpoints](modularity-and-ecs-plan.md#reanalysis-checkpoints--evidence-before-replication).

## The tick-owner default

`MapManager::new` constructs `RuntimeTickOwner::Session`, but **stock startup selects the global owner**:
`RustyCore.LegacyCreatureGlobalRuntime` defaults to enabled when absent (`unwrap_or(true)`), and
startup then sets `GlobalLegacy`. A stock server runs the global legacy creature loop; setting the
key to `0` restores the session-owned diagnostic path. The two defaults differ deliberately. The
ADR previously claimed the production default was off, and that entry is corrected.

## What this issue did not do

No authority flip, no new loop, no scheduling change, no bridge removal, no packet-order change
and no gameplay fix — #188's scope forbids all of them. The legacy/canonical bridge inventory it
relies on lives in `runtime-ownership-ledger.json`, where #258 already removed every completed
owner so each bridge names an open retirement or decision issue.
The 2026-09-05 correction likewise changes documentation/metadata only, not scheduling or test
coverage; the broader C0/C3 production execution proof remains #578 work.
