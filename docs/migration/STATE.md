# RustyCore — Honest Current State (single source of truth)

**Historical capability-audit base:** 2026-08-09 · `3.4.3` @ `42977e9a`, including issue #26's
bounded creature-spell P1 wire/lifecycle acceptance and login faction hydration.
**Architecture/plan review:** 2026-09-05 · local #578 branch @ `93e4002a`.
The latter is not a new whole-port parity audit or a deployment claim. Undated subsystem tables,
counts and source locations below belong to the historical audit unless a later bounded note
explicitly updates them; recheck them against current code before selecting implementation work.

The guarded C++ and Rust `15691` evidence was recaptured from clean harness HEAD
`42977e9accb24fc3921af075f4122e1f0180f4a2`; strict diff is CLEAN at 2/2 packets and
`verify-required creature-spell-casting` is CLEAN.

This document replaces the drifting status snapshots in `_INDEX.md` (2026-05-01, "5–15%"),
the `MIGRATION_ROADMAP.md` §3 inherited table (which tells you not to trust it), and the
old append-log, now referenced through `current-session-handoff.md`'s Git-history pointer.
Its historical capability matrix is **grounded in
the named code audit down to subsystem/subdependency level**, not in what prior docs or the
inventory TSV claim. Architecture decisions: [adr-runtime-tick-ownership.md](adr-runtime-tick-ownership.md).
Forward plan: [PORT_PLAN.md](PORT_PLAN.md). Bugs found in already-shipped code:
[EXISTING-CODE-DEFECTS.md](EXISTING-CODE-DEFECTS.md). Source-verification issues #50–#64
and index #65 retain traceability under plan ledger L26, not authority for their original
diagnoses or finding counts. Recheck selected residuals against current Rust and exact C++
sources; preserve only independently supported behavior and evidence in the
[C++ findings](../audits/cpp-parity-findings.md).

Repository refactors are governed by
[`docs/architecture/ownership-and-boundaries.md`](../architecture/ownership-and-boundaries.md):
one mutable owner per concept, private modules before crates, explicit mirror retirement, and
executable Cargo/handler-contract guardrails.

The approved [module design guidelines](../architecture/module-design-guidelines.md) now require
both semantic boundaries and physical source/test navigability. Remaining monolith decomposition
remains #578 C2/C4 implementation work; #583 applies the same policy to its own SDK/modules.
The physical ratchet implemented above `8f5caedc` now covers repository source/tests/tooling,
with 103 initial legacy non-growth ceilings and an independent terminal mode. The first Rust
split above `d3f5c20c` reduces the persistence facade from 4,513 to 544 lines, preserving root
public contracts in private operation modules and retiring its legacy ceiling (102 remain).
The following adapter split above `1e6b7c40` reduces its 4,957-line root to 1,608 lines,
with private statement/row modules and responsibility-specific tests (101 legacy ceilings
remain). Its reviewed exhaustive inventory and matching policy are reconciled in the owning
checkpoint, including stale pre-catalog records and preservation of currency caller provenance.
These are physical decompositions, not closure of broad lifecycle capability cohesion. The existing
logical-owner guards remain. Migration PASS is not terminal acceptance: the legacy files
still need their stated splits or concrete bounded exceptions before #578 closes.

The following local C4 guard repair above published `9cd1da41` preserves scoped callable
reexport/import-chain provenance without exporting private aliases across packages. Unresolved
generic outputs conservatively retain known argument provenance instead of silently losing pools;
329 checker tests pass. Its reviewed exhaustive snapshot contains 10,084 rows: 53 explained
false aliases removed, 21 references added (20 deliberately conservative), with no direct SQL
operation removed and a byte-identical policy. Exact deltas and limitations live in the
owning checkpoint. The parser root's ceiling shrinks from 21,248 to 21,030 lines, with small
private provenance/test modules; this is not terminal physical or semantic closure.

The C0 cut above `36d0ccbf` adds the exact C++ world/map packet-filter contract to
`wow-handler`, tested across all processing classes and Player residences in dev/release.
It corrects the misleading Inplace/socket-thread description, without changing dispatch.
The current independent Session and map loops still lack the required phase coordination;
the integration cut and queue/incarnation/barrier obligations are recorded in the owning
checkpoint. A passing pure filter test is not production scheduling or C0 acceptance.

The C0/C1 residence cut above `590b93f0` checks canonical index, generation, backing
Player identity/container and world/map binding together. Existing residence queries
now fail closed on inconsistent state; the checked API distinguishes those errors
from missing and replaced owners. Public production-library lifecycle and Session
login/save regressions exercise the change. This is invalid-state admission hardening,
not a new scheduler, storage migration or complete lifecycle/durability acceptance;
exact tests and remaining boundaries live in the Session checkpoint.

## Current architecture and execution checkpoint — 2026-09-05

The approved implementation unit remains **#578 with draft PR #579**, under #133. Internal
commits/checkpoints do not create micro-issues, micro-PRs or a new approval gate. The current
contract-led plan, exact inventories, acceptance evidence and remaining boundaries live in
[`session-578-checkpoint.md`](../architecture/session-578-checkpoint.md). #153 verifies the
complete result; it is not an implementation owner for already-known cuts.

The [explicit reanalysis cadence](../architecture/modularity-and-ecs-plan.md#reanalysis-checkpoints--evidence-before-replication)
is conformance before production storage migration, then review of the first real C1/C2 vertical
with C0 execution evidence before replicating its design. C4 checks the complete #578 balance
before #583 production integration; #153 audits both merged macros. After architecture, review
each selected gameplay macro just in time and perform the fresh whole-port planning pass at
#47/M6.2. No checkpoint introduces another routine approval, issue or PR.

At the reviewed local HEAD, canonical `wow_entities::Player` owns the migrated gameplay families
and `wow_map::MapManager` coordinates its generation-checked active/detached lifetime. The former
whole-Player Session write-back and ObjectAccessor Player-copy paths are retired. This is real
ownership progress, not proof that Session is already a thin shell: gameplay orchestration,
catalog/service retention, broad mutable Map access and runtime bridges remain #578 work.
SQLx isolation under #169 is closed; terminal capability cohesion and the complete Session/runtime
boundary still need evidence. Closing #252/#297/#378 proved their stated directory, transport and
classification cuts, not completion of #133.

The next cuts are selected by complete operation contracts and their deletion conditions, not
field counts. Distinguish implementation, production-path integration and parity evidence. Use
focused checks during a cut, bounded integration/failure tests at the affected owner boundary,
and the exhaustive/final stack at terminal acceptance. Retain required live/capture evidence and
explicit publication/deployment approvals; a green fixture suite does not replace them.

After the architecture deliverable, re-audit the next port macro against current Rust and exact
C++ anchors before implementing its residual work. For example, #26 closed a bounded wire/lifecycle
slice, not general creature spell execution; #30's original claim of no power deduction is stale
(`handlers/spell.rs` already checks and deducts canonical Player power and tests rejection without
deduction). Existing #30–#35 and the full-parity ledgers retain their broader contracts. Do not
restart completed work from an old issue diagnosis or silently narrow a milestone to one capture.

The latest [modularity and ECS plan](../architecture/modularity-and-ecs-plan.md), reviewed above
laboratory HEAD `ee9a0128`, **selects private selective `hecs` now**, retaining cohesive domain
aggregates. This is a design choice, not an installed backend or proof it beats every alternative.
The finite independent-state/third-module checkpoint precedes production migration,
not another open-ended backend selection. Its two-module pre-freeze stage passes
at `118171c1`; the post-freeze independent third module also passes all four producer/lifecycle
tests at `c67acbfd`, without host/ABI/oracle edits. The 320-sample aarch64 campaign also passes
all preregistered gates: [result, costs and retained evidence](../architecture/modularity-conformance-results.md).
This completes finite pre-migration conformance, not production acceptance or a 10 ms frame
budget; the next checkpoint is the first real C1/C2 vertical with C0 admission/phase evidence.
The first production C1 cut now captures one coherent full-save projection and acknowledges
only its saved incarnation/row values, retaining changes made during pending I/O. The old
group-wide ACK is test-only. Production-linked controlled-persistence tests cover late change,
replacement, rollback, Unknown and cancellation; they do not establish real DB/relogin or
scheduler parity. All C0–C4 acceptance remains open to the extent recorded in the
[checkpoint](../architecture/session-578-checkpoint.md), including far-transfer save semantics.
An authorized live run on 2026-09-05 now adds bounded **real normal-save/relogin** evidence
for runtime `68fb338b` with QA tooling `04d54074`: two confirmed MariaDB save transactions,
two fresh logins, 13 skill/207 reputation rows retained, and identical 42-known-spell packets.
Persisted spell/favorite/equipment/transmog tables were empty, so their nonempty mutation
branches are not claimed live-proven. The original world executable was restored and verified
serving; BNet was not restarted. No crash, injected failure, scheduler/transfer parity,
publication or macro completion is implied. Exact hashes, scope and reports are in the checkpoint.
The C1 lifetime cut rejects occupied-map destruction/bulk unload and preserves the old
incarnation when replacement cannot allocate a generation. Controlled production-linked map
tests reproduce the old failure; automatic evacuation and complete shutdown QA remain open.
Map occupancy now comes solely from canonical Players: the manual count field/setter/fallback
is retired, and instance-full/GM fixtures use real occupants with unchanged packet assertions.

Native Rust is the default for first-party/custom extensions; Wasmtime/Core Wasm is the selected
operator-optional executor of shared hooks/state/lifecycle contracts. **Scope expansion:** #583
now delivers that bounded adapter and Rust/C guest evidence as well as external stateful modules,
composition and durable operator lifecycle after #231/#578. #153 audits both macros before #133
closes, including Wasm acceptance. The bounded delivery no longer waits for M6; the wider #99
ecosystem retains a fresh planning gate. Current login-message modules do not prove this product.

The completed [V1 laboratory](../architecture/modularity-lab-results.md) supplies 34 contract
checks and 120 corrected-campaign samples on aarch64, all within its pre-registered budgets.
It demonstrates the modeled contracts/costs, not arbitrary module state, a non-Rust guest, real
save durability or production integration. Its first campaign is retained as superseded after
three test/adapter defects were corrected. The separate V2 result above adds independent-module
evidence; neither experiment installs a production ECS/Wasm dependency, deploys code, advances
gameplay completion or establishes a whole-port capability-audit base.

Database migration boundary (issue #256): the daemon-owned permissive `DbUpdater` has been
retired. The `rustycore-db` composition binary is the sole schema migration authority, using a
source-controlled immutable SHA-256 manifest, per-database/component chains, advisory locks and a
durable incomplete marker that makes no false MariaDB DDL rollback claim. `world-server` validates
auth/characters/world/hotfixes and `bnet-server` validates auth through bounded read-only queries
before runtime writes or listeners; neither scans SQL paths, creates schemas, invokes a SQL client
or downloads artifacts. Exact legacy hashes or explicit schema fingerprints provide the
TDB343.24081 transition without reapplying already-materialized RustyCore DDL. Baseline artifact
acquisition remains #255 and the terminal persistence audit remains #153.

Trainer architecture note (issues #157/#158/#159, later dispatched by #142): list and the buy
adapter share one immutable offer decision. Normal trainer teaching revalidates that decision
under the exclusive money owner, commits effective money plus the exact #164 spell/skill result in
one Character DB transaction, attributes unknown COMMIT outcomes with a durable 128-bit operation
token, installs runtime state, and then publishes money, visual kits 179/362 and acquisition actions
in C++ success order. Non-packet acquisition effects install immediately after commit so a later
cross-socket fence failure cannot discard them; a valid cast fully suppressed by immunity or
rejected by the dynamic spell-disable gate still pays and emits both trainer visuals like C++,
while channeled wrappers remain outside the reduced projection. A process-wide pre-ConnectTo
character claim rejects a second live session for the same GUID and is released on a failed
instance handoff or late login packet-ordering fence, preserving C++'s single `Player*` save
authority; normal logout retains that claim through the account-wide offline write and old Player
identity teardown. Effective equipped-item and target-restriction duplicates follow C++'s deterministic
highest-record-ID assignment; ordinary pending spell/skill
changes are saved before trainer preparation instead of making the trainer unavailable until the
next autosave. Trainer failures and visuals use the Realm connection; creature visual fanout
retains the already validated canonical-or-legacy source position. Castable wrappers
require both a startup audit of effective/world-table blockers and a fresh player effect-mask proof;
the startup audit intentionally omits shapeshift metadata because C++ trainer wrappers use
`TRIGGERED_FULL_MASK`, including `TRIGGERED_IGNORE_SHAPESHIFT`;
the active proof rejects unsupported pet-aura hooks before mutation, replays definite self-target
and aura-spell cast failures after the C++-ordered fee/visuals, and resolves retained immunity auras
from their creation difficulty instead of the player's current map difficulty;
active auras now match covered `EffectAura`/`EffectAttributes` and negative aura-link immunity to the exact wrapper
effect/spell while startup excludes unsupported mechanic/state shapes; full C++ immunity-map parity
remains deferred until canonical Unit ownership.
Aura restrictions, equipped-item restrictions and craft reagents compose DB2,
official/custom hotfix overlays and final removals. Craft startup authority rejects a craft when
its created item or any positive reagent
effective sparse item template is absent, matching
`SpellMgr::IsSpellValid`.
Deterministic player `EffectLearnSpell` retains its distinct immediate-runtime/deferred-save timing.
Issue #142 later activated the `TrainerBuySpell` dispatcher arm and reconciled the
PartyUninvite/Vehicle registrations to exact equality with zero drift exceptions.

Battle-pet trainer purchase note (issue #161): a confirmed battle-pet species is now a purchasable
offer product (`Trainer.cpp:127-146` resolves `IsCastable()` before the `AddPet` branch, so only
direct-learn trainer spells reach it) and the list renders it available because C++ `GetSpellState`
has no cap gate. The purchase itself closes the legacy crash window — C++ charged in memory and
committed Character DB first and Login DB second at the next `Player::SaveToDB`
(`Player.cpp:19336-19344`), and `BattlePetMgr::SaveToDB` cleared `SaveInfo` at statement-append
time (`BattlePetMgr.cpp:377`), so a crash between commits kept the charge and silently lost the
pet — with a durable saga keyed by a 128-bit request key shared with the #160 Login DB receipt:
guarded charge + pending command in one Character DB transaction, exactly one pet through the #160
account owner (fence, journal lease and per-species capacity rechecked inside it), success packets
queued only after pet durability and recorded afterward by a durable `published` marker,
exactly-once refund for terminal failures with absolute durable-money reconciliation, and bounded login recovery that
converges interrupted commands without background tasks. Publication keeps the C++ battle-pet
order (money update, `SMSG_BATTLE_PET_UPDATES` petAdded, dependent runtime learn +
`SMSG_LEARNED_SPELLS`, trainer visual kits suppressed, silent cap). Pet, charge and refund are
exactly-once; packet enqueue attempts are recoverable and may repeat because enqueue has no client
ACK and cannot be atomic with the marker, while actual network delivery remains best-effort. A
crash may cause a recovery re-send without consuming the sole durable recovery signal first;
admission-time capacity/journal-lock failures return a structured result
while the wire stays silent like C++. Full design, transition table and fault matrix:
[battlepets.md](battlepets.md) (2026-08-03, #161). #142 activated the dispatcher arm.

### Fidelity policy for proven legacy defects

The legacy C++ server is the behavioral baseline, not an instruction to reproduce undefined
behavior or a demonstrated logic bug. An intentional Rust deviation is acceptable only when the
C++ behavior and defect are both pinned to exact source, the replacement is the smallest bounded
repair, focused tests distinguish it from both the legacy failure and a speculative rewrite, and
the deviation is recorded in the owning migration item. Client-visible changes additionally need
the corresponding C++/Rust capture decision recorded and, when deliberately different, a
re-pinned golden approved as a compatibility change. Suspicious literals, cleanup opportunities
and merely plausible optimizations do not meet that bar.

---

## 0. Historical capability audit: the "represented" pattern

The historical audit found many **`represented_*_like_cpp` paths** where a handler decoded
and validated a request but recorded intent without the required live mutation. This explained
why represented breadth was not playable parity. It is not a current rule that every function
with that suffix is inert: later paths directly mutate canonical owners. Trace the selected
operation from admission through mutation, persistence and publication before diagnosing it.

- Where the mutation path *was* wired, the feature genuinely **WORKS** (melee combat,
  bounded creature aggro/threat and spell-wire publication, inventory move/equip/destroy,
  loot, quest accept/turn-in, vendor, trainer, groups — durable state paths persist to DB).
- Where only the represented layer exists, the feature **looks handled but does nothing**
  observable (mail, auction, trade, taxi, resurrection, hearthstone bind, GO-use/portals,
  most spell effects and creature AI families beyond the bounded aggro/threat/melee/template-
  spell slices).

This is why the old "98% represented" metric and "bags don't open" coexist without
contradiction: ~98% of logic is *represented*, a much smaller fraction is *live*. The plan's
job is to convert represented→live for the playable path, then for everything.

Initial bridge convention: [represented-live-bridge.md](represented-live-bridge.md) documents
the handler → represented intent → live application boundary. The first converted example is
client stand-state change because it has one canonical owner and a deterministic, capturable
realm response. Applied outcomes emit bounded telemetry only after the canonical mutation
succeeds; client-controlled intent history is retained only in tests.

Live evidence (2026-07-12): the stand-state bot passed a Sit request with distinct realm and
instance sockets, and the bounded C++/Rust capture matched four packets exactly:
`CMSG_STAND_STATE_CHANGE 0x318C` on connection 1, `SMSG_STAND_STATE_UPDATE 0x271C` on
connection 0, `SMSG_UPDATE_OBJECT 0x27CB` on connection 1, and the `CMSG_PING 0x3768`
fence on connection 1. Strict capture-diff reported CLEAN after symmetrically excluding only
ambient `s2c:0x2DD4` creature movement. The slice remains represented-partial for the
documented full `Spell::cancel`, original cast-difficulty metadata, cross-Unit aura-application
lifecycle, and canonical `Map::SendObjectUpdates` ownership gaps. Missing masks now resolve from
an effective table composed in C++ load order from `SpellInterrupts.db2`, official/custom SQL
overlays by DB2 record ID, world `serverside_spell` masks, and the interrupt-mask subset of
`LoadSpellInfoCorrections`; this does not claim full server-side `SpellInfo` or correction parity.

Creature-movement evidence (2026-07-24, issues #21–#24): M2.1's production implementation had already
landed after the issue was opened. The global legacy tick launches random/waypoint `MoveSpline`s,
serializes `SMSG_ON_MONSTER_MOVE`, and fans them to nearby sessions through the final
`HaveAtClient` gate; PR #77 installed/restarted that runtime and manually verified visible
creature movement in the client. The issue closeout pins a real 117-byte C++ compressed-waypoint
packet from the accredited capture artifact
`a25f2c2bbf60de6cda7e32f305d732733017e711eb474dd5dbf6e007690143a8`, and Rust reproduces
it byte-for-byte; the complete 717-test packet suite is clean with that regression.

Issue #24's guarded live Detour capture is now strict-clean on three isolated packets
(heartbeat, one compressed chase spline, ping fence). The capture proved C++ falls from the
elevated fixture to the lower `.map` plane when static VMap height is unavailable. Rust preserves
the elevation only because Detour itself returned a connected elevated polygon corridor; it does
not lift a lower route from equal endpoint heights, which cannot distinguish disconnected
platforms. Identity, options, flags and transport fields remain strict. The same capture exposed
and fixed a separate lifecycle omission: Rust now mirrors
`Creature::AtEngage`/home-finalize by temporarily adding `UNIT_FLAG_CAN_SWIM` when the movement
template permits water, so `MoveSplineInit` publishes the same `CAN_SWIM` flag as C++ and restores
the out-of-combat flag afterward. Player chase snapshots are keyed by map, instance and GUID, so a
live target that teleports elsewhere cannot feed foreign coordinates to the chaser's Detour map.
M2.2 adds one persistent `wow_movement::MotionMaster` to each legacy `WorldCreature` and advances
it once from the globally owned creature frame, after spline position advancement as in
`Unit::Update`. Random and waypoint execution is now gated by the selected stack entry; active
combat chase has normal priority, interrupts an in-flight wander spline with the existing
C++-shape stop packet in the same global aggro frame, but remains below a represented
highest-priority point/charge generator. The represented source lifecycle is advanced in that
same frame and popped finalizers are applied before resynchronizing, so finite spline/timer
generators release their selector proxy and expose chase/default naturally. Combat reset also
exposes the default generator again. This is a bounded runtime bridge: the owner-dependent
random/waypoint work still runs in the existing concrete generators after stack selection, and
real chase target pathing remains with M2.5.

M2.3 re-audited the existing world startup loader against `WaypointManager::LoadPaths`: the
issue's “no waypoint path loader” diagnosis was stale. The current DB loads 7,698 parent paths
and 142,185 ordered nodes, and 5,419 waypoint spawns resolve a nonzero addon path. The remaining
live defect was cadence: the spline advanced against elapsed wall time while random/waypoint
timers received a fixed configured 10 ms, so scheduler delay could finalize the spline before
the generator timer and stretch patrol re-arming in proportion to runtime lag. The single existing Tokio owner now measures and
clamps the real elapsed frame `diff`, matching the C++ `World::Update(diff)` →
`MapManager::Update(diff)` contract, without adding tasks or holding a lock across an await.
Positive, negative and long-horizon regressions prove random re-arming and multi-node waypoint
progress. The installed release (`3c210cdc…`) passed a connected bot login/stand smoke: the bot
received two `SMSG_ON_MONSTER_MOVE` packets and the server published 627 movement packets over
327 visible-work ticks in the observed window. This closes the bounded live wander/patrol item,
not Detour path exactness, formation/transport behavior, SmartAI callbacks or chase/threat.

M2.4's opening diagnosis — "navmesh loaded but `find_path()` never invoked" — was **stale**.
`wow-recastdetour` is a real vendored Detour build with `findPath`/`findStraightPath`/
`moveAlongSurface`/`raycast` plus a ported `FindSmoothPath`/`FixupCorridor`/`GetSteerTarget`, the
runtime pathfinder thread is created whenever `CONFIG_ENABLE_MMAPS` is on (C++ default `true`),
and both live generators already resolved corridors and launched them through
`MoveSplineInit::MovebyPath`. The real defects were in **what the query returned and when it was
even attempted**, all four contrasted against `PathGenerator.cpp`:

1. `PathGenerator::CreateFilter`/`UpdateFilter` (`PathGenerator.cpp:648-698`) derives the Detour
   filter from the owner; both live call sites passed a hardcoded ground-only creature filter, so
   amphibious owners could never cross `NAV_WATER`/`NAV_MAGMA_SLIME` polygons and combat/evade
   owners never got `NAV_GROUND_STEEP`. The filter is now sampled per creature from
   `CanWalk()`/`CanEnterWater()`/`IsInCombat()`/`IsInEvadeMode()`.
2. `CalculatePath` answers a missing navmesh/tile with `BuildShortcut()` and
   `PATHFIND_NORMAL | PATHFIND_NOT_USING_PATH` (`PathGenerator.cpp:79-86`), which
   `RandomMovementGenerator::SetRandomLocation` happily launches. RustyCore mapped that same case
   to a path *failure*, so wander on unmeshed terrain retried every 100 ms forever instead of
   moving. Only a genuine Detour error stays a failure now.
3. `BuildPolyPath` calls `BuildPointPath` exactly once (`PathGenerator.cpp:287`, `:527`). The Rust
   corridor builder also ran a full `findStraightPath` pass whose result was then discarded — one
   wasted Detour query per request, and its `PATHFIND_SHORTCUT`/`PATHFIND_SHORT` bits leaked into
   the surviving smooth path, which made the callers reject a perfectly good navmesh route. The
   point path is now built once, in the mode `_useStraightPath`/`_useRaycast` selects, and the
   possibly clamped `endPoint` is threaded through while `GetEndPosition()` stays the requested
   destination for the `_forceDestination` comparisons.
4. `CalculatePath` requires `HaveTile(start)` **and** `HaveTile(dest)`; C++ satisfies both because
   `TerrainInfo::LoadMMap` loads each grid's `.mmtile` as the grid loads. RustyCore demand-loads
   from the path request and only consulted the start position, so any destination one tile over
   reported "no navmesh" while the mesh sat on disk. Both endpoints are demand-loaded now.

The three shortcut/failure `PathType` values are also bit-exact again: C++ `BuildShortcut()`
*assigns* `PATHFIND_SHORTCUT` before the caller ORs onto it, so the no-poly and empty-`findPath`
branches are plain `PATHFIND_NOPATH` and the point-path failures are exactly
`SHORTCUT|NOPATH` / `SHORTCUT|SHORT`. A deterministic navmesh fixture (a walkable ring around an
unwalkable centre cell) proves the live waypoint tick walks *around* the hole with real
intermediate points; the same test degrades to the four-point straight line when pathfinding is
disabled, so it is a genuine regression guard.

Four further C++ branches were then closed in the same slice:

5. **The mesh-hole exceptions.** `BuildPolyPath` grants `PATHFIND_NORMAL | PATHFIND_NOT_USING_PATH`
   when the owner `CanFly()` (`PathGenerator.cpp:180,198-202`), and the far-from-poly branch also
   shortcuts for a flying owner or one that `IsFalling()` towards a lower destination — the charge
   case (`:221-240`). Those owner facts are now threaded into the Detour layer. The `CanSwim()`
   halves still need `Map::GetLiquidStatus`; treating "no liquid data" as "not submerged" can only
   withhold a shortcut C++ would grant, never invent one.
6. **`UNIT_STATE_IGNORE_PATHFINDING`** is set from `CREATURE_FLAG_EXTRA_IGNORE_PATHFINDING`
   (0x20000000) as `Creature::Create` does (`Creature.cpp:1154-1155`), through both the create
   lifecycle and the runtime `flags_extra` seam the legacy registration path uses.
7. **Corridor reuse.** The previous `_pathPolyRefs` now travel in the path request, so
   `BuildPolyPath`'s subpath and 80%-prefix-plus-suffix branches (`:291-413`) are reachable, and
   `GetPathPolyByPosition` (`:94-123`) is ported so `GetPolyByLocation` consults that corridor
   before paying for `findNearestPoly` — which also changes the `distToStartPoly` feeding the
   7.0-yard test. Random and chase keep their corridor for the generator's lifetime like C++;
   waypoint and home get a fresh one per call because `MoveSplineInit::MoveTo` constructs a new
   `PathGenerator`. Two edge decisions are explicit: the C++ lookup really uses the full 3D
   `dtVdistSqr` and its literal squared `< 3.0f` threshold (effective radius `sqrt(3)`), so Rust
   does not replace it with the reviewer's 2D metric or a speculative `< 9.0`. A failed
   80%-suffix keeps the complete valid prefix: with no suffix there is no overlap polygon to
   subtract, although C++ unconditionally computes `prefix + 0 - 1`. Rust clamps the point path
   to that retained corridor boundary, recalculates a singleton prefix instead of reproducing
   C++'s zero-length tail underflow, and clamps a successful singleton partial corridor to its
   reachable polygon rather than appending a straight segment across a disconnected gap.
   `BuildPointPath` branches that call C++ `BuildShortcut()` now also clear the retained polygon
   corridor, so a later update cannot reuse path state that `PathGenerator::Clear()` destroyed.
8. **Chase and home now path.** `ChaseMovementGenerator::Update`
   (`ChaseMovementGenerator.cpp:94-240`) and `HomeMovementGenerator::SetTargetLocation`
   (`HomeMovementGenerator.cpp:60-82`) were already faithful Rust ports with **no caller**. Chase
   drives the live tick with the victim's facts snapshotted by the tick driver (players from the
   registry, creature victims from the map, both before the mutable borrow) because the creature
   step has no object accessor; it applies the C++ destination choice, `forceDest = CanFly()`,
   `ShortenPathUntilDist` against the victim, `SetFacing(target)` and the chase walk template.
   Rust deliberately stores the computed move-toward/move-away direction: C++ compares
   `moveToward` with `_movingTowards` but never assigns the field, leaving move-away range checks
   on the wrong bound and able to stop immediately. The direction is committed only after the
   corresponding spline launches; a direction flip discards the old `PathGenerator` corridor
   before querying, while a failed query cannot publish movement that never began. Home
   replaces a *teleport*: the old `Returning` arm assigned `move_target` onto the creature position
   with no spline and no packet. Both have around-obstacle tests against the real navmesh fixture
   that fail when pathfinding is disabled.

The connected gate is now represented by the fail-closed
`detour-chase-around-obstacle` capture flow. It pins a generated MMap tile, disposable
character/spawn identity, exact heartbeat → compressed `SMSG_ON_MONSTER_MOVE` → ping window,
full MonsterMove decoding (normalizing only the process-global spline ID), source/binary
revision provenance, and guarded private-DataDir/database restoration. Its reviewed C++/Rust
pair is strict-CLEAN across all three selected packets with an empty divergence baseline.

Still absent, and explicitly **not** claimed: **point/charge, fleeing and confused** generators are
complete, unit-tested ports with no live trigger — nothing sets `UNIT_STATE_FLEEING`/`CONFUSED`
(there are no fear/confuse aura handlers) and no live caller installs a Point generator, so wiring
tick arms for them would only be exercisable from tests. Also open: mutual chase (needs the
victim's `MotionMaster`), `IsWithinLOS` and `ShortenPathUntilDist`'s LOS test (VMap stub), the
`CanSwim()` mesh-hole halves and `Map::IsUnderWater` (liquid), `Map::GetForceEnabled/Disabled
NavMeshFilterFlags`, `_useRaycast`/`_useStraightPath` (no live caller),
`MovePositionToFirstCollision` + `IsWithinLOS` gating the wander roll, VMAP/liquid-aware
`NormalizePath`, off-mesh links/transports/formation, the reached-home addon/sparring-health reloads,
and C++'s per-instance pathfinder concurrency — every map still serializes
through one pathfinder thread.

M2.5 / issue #25 replaces spawn-frozen threat with the live C++ ownership path. Nonlethal hostile
spell damage now engages the creature and adds effective-damage threat; direct healing forwards
half of effective healing, divided across eligible threatening creatures. `EffectTaunt` matches
the caster to the available highest threat and `SPELL_AURA_MOD_TAUNT` gives the newest active
taunt priority until its DB2 duration expires, restoring an older still-active taunt afterward.
The global creature tick reselects at C++'s 110% melee / 130% ranged thresholds, broadcasts
`SMSG_ATTACK_STOP` on evade, clears threat/tap state, blocks re-aggro while returning, and restores
spawn health only when home movement finalizes. Threat references now also preserve C++'s distinct
`Suppressed` state for targets immune to the attacker's melee school, confused, or held by a
damage-breakable stun; they remain in the threat list but cannot be selected until online again,
and clearing the aura alone does not expire suppression: only new threat from that target or
C++'s explicit `TauntUpdate` reevaluation can reactivate it. An active taunt explicitly bypasses
suppression. `CallAssistance` is once per engagement, delayed by the configured family-assistance
delay, stored on the caller with assistant GUIDs like C++ `AssistDelayEvent`, re-resolved and
restricted by C++ `CanAssistTo` gates when due, and cannot chain from an assistant. Focused
positive/negative regressions also pin ordered, lossless delivery of committed creature combat
events; its per-session backlog is bounded and disconnects a stalled/desynchronized consumer
instead of dropping events or growing without limit. The complete `wow-world`
3119/0 and `wow-entities` 667/0 library suites are clean. A guarded live
`detour-chase-around-obstacle` recapture from `4535a25a` proves the attack-accepted → target
acquisition → chase slice byte/opcode-clean against the retained C++ golden (3/3 packets, no
value/routing/missing/extra differences); the fixture also restored its character, respawn, world
DB, and private DataDir snapshot exactly. That wire window does not exercise heal, taunt,
assistance, or evade; those branches remain covered by focused C++-anchored regressions rather
than dedicated live captures.

M2.6 / issue #26 closes the bounded stock `CombatAI`/`TurretAI` template-spell publication
slice. The globally owned creature frame reads the hydrated template slots and active-difficulty
spell metadata, schedules only the represented instant target shapes with the C++ raw cooldown
rules, and commits an atomic adjacent `SMSG_SPELL_START`/`SMSG_SPELL_GO` pair before the
same-frame melee phase. The final P1 hardening removes GO's unconditional-hit assumption.
It permits publication only for the bounded C++ resolution of a physical
`DmgClass=MELEE` Creature spell whose sole target is a Player attacked from behind, whose spell
and represented effect mechanics are all zero, and whose complete Creature/Player source
authority proves every omitted spell-hit source hit-inert. Completeness does not require every
external source to be empty: persistence/login sources may be nonempty when every exact effect is neutral
to this bounded result, but the reduced runtime's canonical local aura
application/modifier/visible containers still must be empty until their full C++ semantics are
owned. Player authority fails closed across persistence and login/zone reconciliation, map/area
ancestry, guild, skills, active/rewarded/auto-push quests, glyphs, active-specialization traits,
pets and battle-pet slots, FFA/PvP/war-mode state, SpellArea/outdoor/battlefield sources, and
script, legacy/all-rank and SpellLinked hooks. A valid SpellLinked hook blocks its candidate, as
does the absolute trigger ID retained from a rejected loader row.

A Creature-owned uniform roll in `0..=9_999` resolves base `MISS` below `500` (5%) and `HIT`
otherwise. C++ causal order is preserved locally: cast before repeat schedule and hit roll before
the cooldown draw; `NO_ATTACK_MISS` consumes exactly one hit roll before forcing `HIT`. The due
EventMap slot is cleared before cast, so a blocked post-cast schedule cannot create an immediate
retry loop. After publishing an accepted HIT, Rust tombstones before scheduling: C++'s launch
phase next consumes an unconditional critical roll and possible effect-value draws outside this
wire slice. MISS does not enter those target-effect draws and may retain authority for its repeat
delay. Spell, melee and movement randomness share a fail-closed Creature tombstone. A valid
melee swing that reaches the still-unrepresented C++ damage/outcome/proc calculation sets it and
publishes no invented damage or wire. C++ has one process-global RNG whereas Rust has one RNG per
Creature; P1 therefore claims equal distributions and local causal draw order, not the exact
global stream or cross-Creature interleaving. Missing or unsupported metadata, authority, power,
aura, projectile, target, visual, difficulty and optional-payload shapes fail closed with neither
START nor GO. Already-performed deterministic event/reset work remains, while a tombstone blocks
subsequent random-dependent work. This is a **wire/lifecycle subset only**: GO's resolved hit/miss
topology does not apply spell effects, damage, health, aura, or power mutation, and it is not
evidence of the full `Spell` pipeline or M3 combat math.

The first Rust live attempt exposed a real login bridge defect rather than a casting mismatch:
`player_faction_template_like_cpp` remained unset, the player registry published faction template
`0`, and the creature-hostility gate correctly rejected that unrepresented identity before
`AttackStart`. The final HEAD derives the player's faction template from `ChrRacesStore`
when the loaded identity is installed and mirrors it into the canonical `Player`; the regression
covers identity → registry → canonical player without manually seeding a faction. The accredited C++ source
derivation starts at base HEAD `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`, applies the reviewed
one-file patch SHA-256
`ef8b3c29f46fe537e1ae4e826b5610afcd534999f900ec9554ee0534e7847262`, and yields patched HEAD
`8cfed90bf1720dbf8b9dc109113c8d7d9173ff6c`. That patch only corrects the
`ChrSpecialization` index-container bound needed to load the installed DB2 dataset; it does not
touch creature AI, spell selection, casting, or packet serialization.

The final fixture guard uses contract `creature-spell-casting-shell-fixture-v2`. It verifies the
installed stock `AIName=SmartAI` and difficulty-0 `StaticFlags1=0`, CAS-switches the capture
window to `CombatAI` and `0x00100000` (`CREATURE_STATIC_FLAG_NO_MELEE`), then CAS-restores the
exact `SmartAI`/`0` pair and verifies cleanup. Suppressing auto-melee prevents that unrelated path
from consuming melee damage RNG before CombatAI's due spell without disabling its EventMap cast.
The final live authority also loads effective `ChrSpecialization` hotfix rows and corrects
external-ID `AreaTable` WDC4 offsets, so Shattrath area `3697` resolves to map `530` and Terokkar
zone `3519` like C++. OutdoorPvPTF source spell `33377` is admitted only after its effective XP
and outgoing-damage auras plus exact runtime-hook authority prove it hit-inert; the four
Auchindoun dungeon IDs remain fail-closed without C++'s `(Map*, zone)` registration authority.

Both C++ and Rust were recaptured from clean harness HEAD
`42977e9accb24fc3921af075f4122e1f0180f4a2`. The final guarded Cabal Interrogator `22378` /
Eviscerate `15691` import contains exactly the adjacent START/GO pair on both sides with no
accepted divergence. It records one observed **HIT** branch and does not prove that `15691`
always hits. Its current review identities are:

- C++ RAW PKT:
  `b52cc8ba962160be63286e72eb7611c6282b0cdc3a1cee0082fc6d6d7bf2c7b9`;
- Rust RAW tree:
  `9aee309d9ffb2e2e1e5a33167c228ccaa8d1634d917efd026e1a525f2a5db94a`;
- C++ / Rust capture manifests:
  `d40e3615b3337a26a3c4d4e380dc665c23719133ec0b3c7a05febdfd640e849d` /
  `3c942209db52f9f36b3d661477f0cad766e7e2ee49cf1fc97d68a74d996f0da0`;
- filtered C++ PKT:
  `a6b32206e3277e455e25f6aa8e491606aa5cd9449e2bf24245ea9dd5db79d932`;
- normalized Rust tree:
  `cc8d53b06c2727c95990eda80fd095a1a7e390da0af16981429d07176e0c003b`;
- `capture-lineage.json` file:
  `f443539e7857ac27dfb2029012f1e889d92ed27a224f89f7a6247f9510f0479d`.

`diff creature-spell-casting --strict` reports 2 matched packets with zero value, routing,
missing, or extra differences. `verify-required creature-spell-casting` is CLEAN with exact
topology/order and correlated payload semantics.

---

## 1. Historical progress picture (three axes, not one number)

Historical capability snapshot at the audit base above, not measured percentages for #578 or
current HEAD. The current architecture checkpoint reports acceptance boundaries separately.

| Axis | Estimate | Meaning |
|---|---:|---|
| **A. Breadth represented** | ~98% of R8 rows touched | Logic exists/contrasted in the represented model. **Retired as a headline** — rewards breadth over working features. |
| **B. Live-playable core** | **partial, holed, buggy** | Core loop (login→move→melee→loot→quest→vendor→group) is live. The scoped D-C1…D-C9 CRIT integrity track is closed, but combat math and multiple HIGH/MED gameplay/runtime gaps remain. Spells and creature AI have bounded live subsets; most spell effects/AI families, world-interaction and death remain represented-only or absent. |
| **C. Full 1:1 parity** | **low** | Long tail absent: ~108 spell effects, ~255 aura types, content scripts (0/294k LOC), mail/AH/calendar live, BG/arena/instances, ~215 stat/data stores. |

Part 1 of the plan drives **B** to complete; Part 2 drives **C** to complete.

---

## 2. Historical capability matrix and bounded additions

This matrix records the historical subsystem audit and its explicitly dated additions. Its old
paths and statuses are discovery pointers, not proof that the same gap still exists at HEAD.

**WORKS** = mutates live state + persists/broadcasts like C++ · **WORKS⚠** = live + persists
**but has correctness/integrity bugs** (see [EXISTING-CODE-DEFECTS.md](EXISTING-CODE-DEFECTS.md)) ·
**PARTIAL** = subset only · **STUB / REPRESENTED-ONLY** = validates/records intent, no
observable mutation · **ABSENT**.

> ⚠ The scoped D-C1…D-C9 integrity defects are closed, but the adversarial audit still lists
> substantial HIGH/MED gaps in [EXISTING-CODE-DEFECTS.md](EXISTING-CODE-DEFECTS.md). "WORKS"
> means the named path is live; it is not a claim of full 1:1 gameplay parity.

### Core gameplay loop — live, with remaining non-CRIT gaps
| Capability | Status | Evidence / defects |
|---|---|---|
| Auth/BNet SRP6 + world-enter handshake | WORKS | recent `fix(bnet)` commits; played live |
| Player base/stat projection | **PARTIAL (live)** | issue #60 replaces the incorrect `player_levelstats` path with C++-anchored `player_racestats` + `player_classlevelstats`, `GtBaseMP`, `CreateHealth=0` and a shared create/login/equipment/level-up StatSystem projection. Login seeds passive parry/block capability before projection and defers its saved-health clamp until persisted stat auras and represented item/enchantment modifiers are active; covered total-stat aura recalculations retain those item bonuses and emit no pre-CreateObject VALUES delta. The recorded paired C++/Rust login captures match the scoped max-health/mana, five primary stats, armor, base mana, AP and damage fields, including the 3% total-stat racial passive. That evidence does not establish complete login `UpdateObject` or wider unit-mod/aura/item-stat parity, which remain open. |
| Starting skills and skill-rewarded login spells | **WORKS (scoped issue #62)** | `SkillRaceClassInfo` now follows C++ `Availability`/`MinLevel`; default skill rank/max/step follows language, level, mono, tier, always-max and DK rules; loaded rows are normalized like `_LoadSkills`; and the live no-DB-spell login path applies `LearnSkillRewardedSpells` with real spell levels, quest fallback, Riding, masks and actual skill values. Correct WDC4 inline IDs restore Common-compressed `SkillLineAbility` fields. A live Blood Elf Hunter C++/Rust pair yields the same exact 43-spell set under the reviewed unordered-map comparator; bit/count/list/favorites integrity remains strict. Wider skill gain/update/discovery/unlearn runtime remains in L18. |
| Player movement + broadcast to nearby | WORKS⚠ | `movement.rs:310`; trust-client position (D-H10), creature destroy deferred (D-H15), async CREATE race (D-H14) |
| Melee combat (deals damage→death→loot) | **PARTIAL** | `session.rs:47635`; **no damage formula / hit table / armor mitigation** (D-H1, D-H2) — numbers are wrong |
| Global creature runtime (aggro/melee/move→packets) | WORKS (default on) | `world-server/src/main.rs:12682+` |
| Inventory equip/swap/move/destroy (+DB) | WORKS | D-C1/C2 relog metadata and D-C4 atomic swaps are closed; issue #52 adds C++ position/bank/unequip/store/equip gates, container-aware move/merge/swap/destroy, realm-routed errors, and paired installed C++/Rust invalid-gate plus occupied-swap/relog evidence with a strict clean action capture |
| Loot items + money (+DB) | WORKS⚠ | D-C5/C6 concurrent-claim duplication is closed; quest-credit gap (D-H6) and process-abort recovery of detached post-COMMIT continuations remain separate boundaries |
| Quests accept/turn-in core rewards (+DB) | **PARTIAL** | `quest.rs:1359,5569`; kill/explore/item objectives may not auto-advance (D-H4/H5/H6) |
| Vendor buy/sell, Trainer learn, Groups (+DB) | WORKS⚠ | vendor D-C8 and group-capacity D-C9 atomicity are closed; finite-stock oversell (D-H11), trainer validation (D-H9), buyback/refund and wider group parity remain |
| Gossip menus + quest-giver status icons | WORKS | `handlers/quest.rs:1248`; gossip conditions evaluated |
| Item enchant/gem/socket, durability repair, binding | **PARTIAL** | D-C1/D-C2 relog now reloads and serializes the exact persisted 13-slot enchant/random-property state, including the paired exact create-block proof. Broader gem/socket/durability/binding runtime parity remains unproven and belongs to later parity work; issue #52 is the separate move/equip/store validation closure. |
| Bank / equipment-sets / void-storage persistence | **WORKS** | D-C3 is closed: PRs #103, #113 and #115 merged with required CI/review gates. Installed bank, equipment/transmog and void-storage relog QA passed; the committed equipment-set ACK and void-storage query captures are strict C++/Rust CLEAN. Issue #114's documented failure-only all-or-nothing divergence remains intentional and bounded to its stronger transaction contract. |
| UpdateFields / CREATE-block serialization | WORKS | `wow-packet/src/packets/update.rs`; issue #10 re-audited rows 1212–1220 and closed M1.4's bounded value gaps, including selected non-mana creature power and runtime GO ArtKit; canonical per-spawn ParentRotation remains in its documented architecture follow-up |
| Rested XP / offline rest state (XP slice) | **PARTIAL (live)** | issue #81: live accrual, consumption, DB persistence/relog and `SMSG_LOG_XP_GAIN` are capture-clean under the reviewed runtime-counter comparator; full `RestMgr` ownership and rest-area wire remain open |

#### 2026-07-18 bounded rested-XP evidence

The same guarded bot workflow passed against the primary C++ reference and
RustyCore. A DB-controlled 86,400-second offline interval produced wilderness
and resting bonuses of about `14.88` and `60.00`; a live Mana Wyrm kill
(`entry=15274`, map `530`) emitted realm-routed `SMSG_LOG_XP_GAIN` with
`Original=100`, `Reason=Kill`, `Amount=50`, and `GroupBonus=1.0`. XP persisted
`0 -> 100`, rest bonus persisted `300 -> 250`, a full relog observed the saved
state, the disposable fixture was restored, and the persisted 300-second
creature respawn row cleared naturally.

The committed one-packet C++/Rust flow is strict-CLEAN only under the narrowly
reviewed comparator that omits the nonzero lower 40-bit runtime counter of a
Creature Kill victim GUID. It still requires realm routing and exact high type,
realm, map, entry, subtype, server id, and every XP field; malformed or
zero-counter bodies fail. This does not prove a real tavern/city AreaTrigger
walk, nested `RestInfo` update-field wire, honor rest, full group/KillRewarder
fanout, or a per-Player `RestMgr`/`Player::Update` owner. The aggregate
`#PLAYER.12` therefore remains open.

### Engines — PARTIAL / STUB (the gameplay-quality gap)
| Capability | Status | Evidence |
|---|---|---|
| Spell cast → SPELL_START / SPELL_GO | **WORKS (bounded paths; issue #26 P1 verified)** | the represented player-cast handler publishes its existing START/GO path; issue #26 adds one map-owned creature path. P1 no longer presumes HIT: only a physical melee Creature spell against a rear-attacked Player, with zero mechanics and complete source authority proving omitted sources hit-inert, may publish the atomic pair; canonical local aura containers remain empty. Player persistence/login/zone/map/guild/skill/quest/glyph/trait/pet/FFA/SpellArea/hook evidence fails closed, including valid and rejected-trigger SpellLinked rows. The Creature roll maps `<500` to base `MISS` and the rest to `HIT`; local order is cast→schedule and hit→cooldown, with `NO_ATTACK_MISS` still consuming a hit roll. An accepted HIT publishes then tombstones before scheduling because subsequent launch/effect RNG is omitted; MISS may retain authority for its repeat delay. Spell/melee/movement share the tombstone. C++ global-vs-Rust per-Creature RNG parity is distribution/local-order only. Guard v2 recaptured C++ and Rust from clean `42977e9a`, temporarily switching stock `SmartAI`/`0` to `CombatAI`/`NO_MELEE` and restoring it; strict 2/2 and `verify-required` are CLEAN. This does not claim effect execution or the full Spell pipeline. |
| Spell effects dispatched | PARTIAL **~42/150** | `session.rs:48774-49383` |
| Spell cast cost/timing/cooldown | **PARTIAL** | SpellCastTimes/SpellCooldowns/SpellPower DB2 hydrate represented `SpellInfo`; the player path checks/deducts flat + mana-pct costs, while creature `CombatAI` uses raw `RecoveryTime`, floors missing/short values at C++'s 5-second AI default, and re-arms every repeat attempt in `[cooldown, 2×cooldown]`. Full C++ SpellHistory/modifiers and general cast lifecycle remain open. |
| Spell damage calc (coeff/crit/resist/absorb) | **ABSENT** | `SpellEffectDb2Entry` parsed but unused |
| Spell LOS/range/facing/reagent checks | **PARTIAL** | issue #26 revalidates the live canonical creature/victim, C++ min/max range with combat reaches and movement allowance, and map LOS immediately before publication; the final bounded P1 slice additionally requires that the Creature caster is behind its Player target. General player-cast range/facing/reagent coverage and real VMap-backed LOS remain incomplete; the shared VMap foundation is still a stub. |
| Aura apply + client update | PARTIAL | `session.rs:26431` |
| Aura periodic tick (DoT/HoT) + ~255 aura types | **STUB** (~5 types) | `session.rs:27810` expiry only; `unit_subsystems.rs:12-14` |
| Proc system | **ABSENT** | `SpellAuraOptionsEntry` fields unused |
| Channeled / missiles / ground-AOE / DynamicObject | **ABSENT** | no lifecycle state machine |
| Creature AI (threat gen / spell cast / waypoints / SmartAI / text) | **PARTIAL** | random wander, DB waypoint patrol, threat/taunt/assistance/evade, and bounded CombatAI/TurretAI template-spell START/GO publication are live; spell effects, other AI families, text and SmartAI interpretation remain open |
| Creature movement: spline broadcast (SMSG_MONSTER_MOVE) | **PARTIAL** | global legacy tick continuously launches and broadcasts random/waypoint splines on measured elapsed cadence; exact captured compressed-waypoint body is byte-clean |
| MotionMaster tick | **PARTIAL** | persistent per-creature stack is ticked once by the global frame and selects random/waypoint/chase priority; concrete owner-bound bodies and broader generator callbacks remain |
| Pathfinding (Detour navmesh query) | **PARTIAL** (random/waypoint/chase/home live) | real vendored Detour + ported `FindSmoothPath`; owner-derived filter, single `BuildPointPath`, both endpoint tiles demand-loaded, C++-exact no-navmesh shortcut, fly/falling mesh-hole exceptions, `IGNORE_PATHFINDING`, corridor reuse + `GetPathPolyByPosition`. Point/flee/confused have no live trigger; raycast/straight-path unreachable; mutual chase, VMAP LOS and liquid-aware `NormalizePath` absent |
| Canonical map tick (`wow_map::MapManager`) | **STUB** | `wow-map/src/manager.rs:520` no AI/combat side effects |

### World-interaction & death — REPRESENTED-ONLY (handlers record intent, no mutation)
| Capability | Status | Evidence |
|---|---|---|
| GameObject use (doors/chests/**portals**/quest objs) | REPRESENTED-ONLY | `handlers/misc.rs:6378`; portal/chest/transport fall to "not ported"; **#13** |
| Mail send/list/take/delete | **ABSENT** (no handler) | only `QueryNextMailTime` exists |
| Auction list/bid/buy/sell | REPRESENTED-ONLY | `handlers/misc.rs:1862` record-only |
| Player trade | REPRESENTED-ONLY | `handlers/misc.rs:1466` record-only |
| Taxi/flight path | REPRESENTED-ONLY | `handlers/misc.rs:117` no flight movement |
| Death/resurrection/corpse/graveyard | REPRESENTED-ONLY | `handlers/misc.rs:2443,2495`; "graveyard data not implemented" |
| Durability loss on death, ghost flags | **ABSENT** | no KillPlayer hook |
| Hearthstone bind / set home | **STUB** | `character.rs:8926` "TODO: set bind in DB" |
| Item-use → cast spell (`ItemEffect.db2`) | **ABSENT** | store not loaded; no `CMSG_USE_ITEM` executor |

### Simulation foundations — silent subdependencies (block correctness everywhere)
| Foundation | Status | Evidence / impact |
|---|---|---|
| Terrain height (GetHeightZ / ground correction) | **ABSENT** | creatures spawn at DB Z; no ground snap |
| VMap line-of-sight | **STUB** (returns `true`) | `world_object.rs:1551`; spells/pathing tunnel walls |
| Stat-formula GameTables (Gt* crit/dodge/HP) | **PARTIAL** | `CombatRatings.txt` now drives represented crit/dodge/parry/block rating scaling; HP/MP/regen/class stat tables still incomplete |
| `ChrClasses`, `ChrRaces`, `FactionTemplate`, `CharBaseInfo` stores | **PARTIAL (live)** | the first three stores are loaded and login now publishes the race-derived player faction to the registry/canonical Player, closing the faction-0 creature-hostility failure; full `CharBaseInfo` and wider create/stat parity remain open |
| `SpellPower`/`SpellCastTimes`/`SpellCooldowns` stores | **PARTIAL** | loaded into represented spell metadata for normal difficulty; full C++ modifier and runtime integration still incomplete |
| DBC/DB2 store coverage | **~110 / ~325 (34%)** | `cpp-db2-stores.tsv` |
| Player periodic save | **REPRESENTED-PARTIAL** | session timer now uses `CONFIG_INTERVAL_SAVE` / `PlayerSaveInterval` and queues represented `Player::SaveToDB`; installed runtime passed bot login/logout and action/travel/quest-objective preservation QA, while first-save randomization, capture diff, and manual live-client QA remain pending |
| Multi-statement save transactions | **REPRESENTED-PARTIAL** | represented character save now commits the Rust-covered `Player::SaveToDB` statement set in one `SqlTransaction`; full C++ save surface, login/account transaction coupling, capture diff, and manual live-client QA remain pending |
| Respawn DB persistence | **ABSENT** | in-memory queue; respawns lost on restart |
| Quest objective auto-credit (loot item / explore) | **ABSENT** | not hooked to loot/area-trigger |

### Absent systems (whole-domain)
Content **Scripts** (`wow-scripts` 40 ln vs ~294k LOC C++) · Battlegrounds/Arena/Battlefield/
OutdoorPvP · Instances lock/save/difficulty toggle · full Conditions eval · full Phasing
refresh · Weather · Warden · Calendar · Petitions · Pet/Totem AI. Empty crates:
`wow-combat`/`wow-spell`/`wow-achievement`/`wow-social`/`wow-pvp` (1 line each; `wow-ecs` was removed by #298).

---

## 3. Historical playable-blocker notes and bounded fixes

Issue #7's CUF login crash is fixed on its branch: Rust now matches the exact C++
post-add order, and a paired live capture with one non-empty profile pins the
four-packet sequence. The CUF and final phase-shift packets are byte-identical;
the broader M1 exit and manual-client UI validation remain open.

Issue #8 closes the compression-stream lifetime defect: the active C++ `0x400`
threshold now has one persistent deflate owner for the complete physical socket,
including across `WorldSocket::split_for_io`. Installed login QA decoded four
successive compressed packets through one persistent inflater before completing
the stand-state round trip. Original-client/manual UI validation remains part of
the broader M1 exit.

Issue #9 closes the first eleven login-burst audit rows. Seven live gaps are corrected:
global account-data and tutorial resends, battle-pet-lock placement, configured MOTD,
cross-socket packet ordering, contact-list publication, and PlayerCondition-filtered
account-mount partials. The other five rows were already fixed, request-driven, or
explicitly bounded after C++ contrast. An accredited 81-packet two-socket Rust capture
proves the corrected physical order; the installed C++ runtime could not complete the
same bot's second-socket login, so this is intentionally not described as a full
byte-clean login capture.

| # | Bug | Effect | Status |
|---|---|---|---|
| #13 | `CMSG_GAME_OBJ_USE` doesn't cast GO use-spell | **portals do nothing** | open |
| #9–#12 | 33 login-burst divergences (`world-load-audit.md`) | ordering/value parity | #9–#10 merged; #11 implemented and locally validated; #12 open |

---

## 4. Architecture reality

At the 2026-09-05 reviewed local HEAD, the legacy creature runtime still supplies behavior while
canonical Map storage owns active Players and the staged `MapRuntime` applies typed transitions.
It is no longer accurate to call canonical Map an empty storage/runtime skeleton. Nor does that
make it the sole production simulation driver: shared manager synchronization, legacy/canonical
bridges and the remaining Session-driven work are explicit convergence boundaries.

The target remains a modular monolith with one mutable owner per concept, explicit C++ phases and
generation-checked Player identity. `MapRuntime` currently uses staged synchronous commands under
the existing manager synchronization; a dedicated task per map and a production ECS are not
claimed. See [the accepted entity-world ADR](adr-map-runtime-entity-world.md) and
[the current checkpoint](../architecture/session-578-checkpoint.md) for actual cuts and evidence.
Old LOC, warning and test counts elsewhere in this document must not be reused as current metrics.

---

## 5. Definition of "done" going forward (replaces `represented-complete`)

A capability is **done** only when: (1) it runs in the **live runtime**, (2) its wire output
is **capture-clean vs a C++ capture** of the same action — byte/opcode exact unless a narrowly
reviewed comparator omits only an intrinsically runtime-allocated identifier or canonicalizes
only proven unordered C++ collection order, while retaining every stable value/count and failing
malformed input — (3) it has been **exercised on a running server/client**, and (4) C++ refs +
the validating capture/test are cited.
`represented-complete` is no longer a closure state — it means "logic drafted, not yet live".

This gameplay-capability definition does not require a fresh capture for every behavior-preserving
structural commit. Refactors follow the proportional gates in AGENTS.md and their explicit issue
acceptance: preserve bytes, metadata, connection and order with focused evidence; obtain the
required action-specific capture/live evidence when the change or acceptance calls for it.
Report a bounded proof as bounded, and distinguish code tests, historical golden regression,
fresh runtime evidence and full functional parity. Architecture completion requires the complete
#133/#578/#583 contracts (including the operator-optional Wasm delivery and independent physical
acceptance) and #153 audit, not a favorable field count or test total. Neither architecture
closure nor playable M6 closes the full Part-2 parity ledgers.
