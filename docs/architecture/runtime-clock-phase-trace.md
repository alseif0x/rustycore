# Runtime clock and phase ownership trace

Issue #188 records what actually drives time in RustyCore today, **before** any convergence
changes scheduling. This is a description, not a target. The machine-readable inventory is
[`tools/architecture/runtime-clock-phase-trace.json`](../../tools/architecture/runtime-clock-phase-trace.json)
and `python3 tools/architecture/check_architecture.py check` proves it stays truthful.

## Why a trace before a cut

The convergence plan in [`adr-runtime-tick-ownership.md`](../migration/adr-runtime-tick-ownership.md)
removes one legacy writer at a time. Every such cut needs to know which clock currently owns the
transition and what stops a second owner resolving it. Prior notes overstated how much of C++'s
`World::Update → MapManager::Update → Map::Update` order the canonical skeleton preserves, and
disagreed with the code about whether the global creature loop is on. A recorded, checked trace
replaces both.

## The seven clocks

| Clock | Availability | Cadence | Diff source | Owns |
|---|---|---|---|---|
| `world_update` | production | configured world interval | measured elapsed | world-level timers |
| `canonical_map_update` | production | configured map interval | measured elapsed, zero-diff skipped | canonical grid/spawn/respawn, area triggers, game events |
| `legacy_creature_runtime` | production | configured map interval | measured elapsed, propagated once into creature logical time | creature lifecycle, movement, aggro, spell, melee, respawn |
| `group_ready_check` | production | own interval | loop interval | ready-check expiry |
| `realm_list_update` | production | own interval | loop interval | realm list refresh |
| `db_keepalive` | production | own interval | loop interval | connection keepalive |
| `session_creature_tick` | **diagnostic** | session task | session diff | nothing in production |

## What the guard enforces

For every clock the checker requires a source file and entry point that exist, a stated cadence,
diff source and availability, at least one named resolution guard, and regression anchors that
resolve to real tests. It rejects any clock that declares `delivers_under_map_guard: true`,
because "no packet or cross-session command delivery while a map lock is held" is a
non-negotiable campaign invariant. Three adversarial cases are exercised: a missing entry point,
a clock admitting delivery under a guard, and a regression anchor no test defines.

## Single resolution

Two production clocks can touch a legacy creature. They cannot both resolve it:

- `legacy_creature_runtime` only acts when `RuntimeTickOwner` is `GlobalLegacy`, and the session
  path reads the same shared owner and skips.
- `canonical_map_update` dispatches no AI or combat side effects at all, so its visit cannot
  double-resolve anything the legacy loop owns.

The existing anchors pin both facts:
`two_sessions_sharing_legacy_map_manager_see_same_creature_state` and
`canonical_map_update_visits_creature_with_no_real_ai_combat_effect_like_cpp`.

Within the selected legacy owner, #371 removed the former second time source
(`WorldCreature::clock_started_at.elapsed()`). The loop's measured `diff_ms` now advances one
logical elapsed value exactly once at the `Unit::Update` boundary. Spline state, MotionMaster,
melee, spell, assistance, corpse and respawn deadlines read that same value; scheduler time between
calls cannot advance any of them independently.

## Delivery boundary

No traced clock delivers under a map guard. The `run_legacy_creature_*_and_deliver_once_like_cpp`
bridges collect outcomes inside the lock and deliver once after releasing it; the canonical loop
does the same for its recipients. That is recorded per clock so a future change that moves a send
inside the guard has to edit this file and fail review, rather than passing silently.

## The tick-owner default

`MapManager::new` constructs `RuntimeTickOwner::Session`, but **production never keeps it**:
`RustyCore.LegacyCreatureGlobalRuntime` defaults to enabled when absent (`unwrap_or(true)`), and
startup then sets `GlobalLegacy`. A stock server runs the global legacy creature loop; setting the
key to `0` restores the session-owned diagnostic path. The two defaults differ deliberately. The
ADR previously claimed the production default was off, and that entry is corrected.

## What this issue did not do

No authority flip, no new loop, no scheduling change, no bridge removal, no packet-order change
and no gameplay fix — #188's scope forbids all of them. The legacy/canonical bridge inventory it
relies on lives in `runtime-ownership-ledger.json`, where #258 already removed every completed
owner so each bridge names an open retirement or decision issue.
