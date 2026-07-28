# RustyCore — Honest Current State (single source of truth)

**Date:** 2026-07-28 · **Base:** `3.4.3` @ `26f4058b` plus local issue #25 threat-runtime closeout.

This document replaces the drifting status snapshots in `_INDEX.md` (2026-05-01, "5–15%"),
the `MIGRATION_ROADMAP.md` §3 inherited table (which tells you not to trust it), and the
1.8 MB append-log `current-session-handoff.md`. It is **grounded in a multi-agent code
audit of HEAD down to subsystem/subdependency level**, not in what prior docs or the
inventory TSV claim. Architecture decisions: [adr-runtime-tick-ownership.md](adr-runtime-tick-ownership.md).
Forward plan: [PORT_PLAN.md](PORT_PLAN.md). Bugs found in already-shipped code:
[EXISTING-CODE-DEFECTS.md](EXISTING-CODE-DEFECTS.md). C#-reference contrast vs C++ (51
findings, 25 open → tracked as GitHub issues #50–#64, index #65; feeds plan ledger L26).
Audit docs kept local/uncommitted: `../audits/csharp-reference-audit.md` +
`../audits/csharp-reference-contrast.md`.

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

## 0. The central architectural truth: the "represented" pattern

Most of the server is built on a **`represented_*_like_cpp` pattern**: a packet handler
**decodes the request, validates the C++ rules, and records the *intent*** — but **defers
the actual game-state mutation to a live runtime layer that mostly does not exist yet.**

- Where the mutation path *was* wired, the feature genuinely **WORKS** (melee combat,
  inventory move/equip/destroy, loot, quest accept/turn-in, vendor, trainer, groups — all
  persist to DB).
- Where only the represented layer exists, the feature **looks handled but does nothing**
  observable (mail, auction, trade, taxi, resurrection, hearthstone bind, GO-use/portals,
  most spell effects, creature AI beyond aggro+melee).

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
revision provenance, and guarded private-DataDir/database restoration. Its requirement remains
`awaiting-real-captures` until the same action has been recorded, reviewed and strictly imported
from both the pinned C++ and the clean committed Rust HEAD.

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
spawn health only when home movement finalizes. `CallAssistance` is once per engagement, delayed
by the configured family-assistance delay, restricted by C++ `CanAssistTo` gates, and cannot
chain from an assistant. Focused positive/negative regressions plus the complete `wow-world`
3119/0 and `wow-entities` 667/0 library suites are clean. A guarded live
`detour-chase-around-obstacle` recapture from `4535a25a` proves the attack-accepted → target
acquisition → chase slice byte/opcode-clean against the retained C++ golden (3/3 packets, no
value/routing/missing/extra differences); the fixture also restored its character, respawn, world
DB, and private DataDir snapshot exactly. That wire window does not exercise heal, taunt,
assistance, or evade; those branches remain covered by focused C++-anchored regressions rather
than dedicated live captures.

---

## 1. The honest progress picture (three axes, not one number)

| Axis | Estimate | Meaning |
|---|---:|---|
| **A. Breadth represented** | ~98% of R8 rows touched | Logic exists/contrasted in the represented model. **Retired as a headline** — rewards breadth over working features. |
| **B. Live-playable core** | **partial, holed, buggy** | Core loop (login→move→melee→loot→quest→vendor→group) is live. The scoped D-C1…D-C9 CRIT integrity track is closed, but combat math and multiple HIGH/MED gameplay/runtime gaps remain. Spells, creature AI, world-interaction and death are represented-only. |
| **C. Full 1:1 parity** | **low** | Long tail absent: ~108 spell effects, ~255 aura types, content scripts (0/294k LOC), mail/AH/calendar live, BG/arena/instances, ~215 stat/data stores. |

Part 1 of the plan drives **B** to complete; Part 2 drives **C** to complete.

---

## 2. Grounded capability matrix (verified to subsystem level)

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
| Player base/stat projection | **PARTIAL (live)** | issue #60 replaces the C# `player_levelstats` path with `player_racestats` + `player_classlevelstats`, `GtBaseMP`, `CreateHealth=0` and a shared create/login/equipment/level-up StatSystem projection. Login now seeds passive parry/block capability before projection and defers its saved-health clamp until persisted stat auras and represented item/enchantment modifiers are active; live total-stat aura recalculations retain those item bonuses and emit no pre-CreateObject VALUES delta. Paired accredited C++/Rust login captures match the scoped max-health/mana, five primary stats, armor, base mana, AP and damage fields exactly, including the 3% total-stat racial passive. The complete login `UpdateObject` still has unrelated field divergences, and wider unit-mod/aura/item-stat parity remains open. |
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
| Spell cast → SPELL_START / SPELL_GO | WORKS | `handlers/spell.rs:203-483` |
| Spell effects dispatched | PARTIAL **~42/150** | `session.rs:48774-49383` |
| Spell cast cost/timing/cooldown | **PARTIAL** | SpellCastTimes/SpellCooldowns/SpellPower DB2 now hydrate represented `SpellInfo`; flat + mana-pct costs are checked/deducted, but full C++ SpellHistory/modifiers/range/LOS/reagents are still absent |
| Spell damage calc (coeff/crit/resist/absorb) | **ABSENT** | `SpellEffectDb2Entry` parsed but unused |
| Spell LOS/range/facing/reagent checks | **ABSENT** | `handlers/spell.rs:341` only checks GCD + active-cast |
| Aura apply + client update | PARTIAL | `session.rs:26431` |
| Aura periodic tick (DoT/HoT) + ~255 aura types | **STUB** (~5 types) | `session.rs:27810` expiry only; `unit_subsystems.rs:12-14` |
| Proc system | **ABSENT** | `SpellAuraOptionsEntry` fields unused |
| Channeled / missiles / ground-AOE / DynamicObject | **ABSENT** | no lifecycle state machine |
| Creature AI (threat gen / spell cast / waypoints / SmartAI / text) | **PARTIAL** | random wander and DB waypoint patrol are live; threat generation, combat spells/text and SmartAI interpretation remain absent |
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
| `ChrClasses`, `FactionTemplate`, `CharBaseInfo` stores | **ABSENT** | class scaling hardcoded; **NPC hostility checks broken** |
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
`wow-combat`/`wow-spell`/`wow-achievement`/`wow-social`/`wow-pvp`/`wow-ecs` (1 line each).

---

## 3. Known live bugs blocking "playable" (open after issues #7 and #8)

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

Three world models coexist. The **legacy global creature loop is the de-facto live runtime**
(works, default-on); the **canonical `wow_map::MapManager` is an empty `Map::Update`
skeleton** (the intended destination, dispatches no AI/combat). The Rust legacy model is
structurally inverted from C++ (sessions historically drove ticks; C++ has the map tick own
creature update). Convergence is incremental (the `_attic/` big-bang died at 176 errors).

**Code health:** `world-server` compiles clean (22 warnings). ~19k test fns. Logic is
concentrated in a 235k-line `wow-world` monolith; the per-domain crate split in the roadmap
(`wow-spell`, `wow-combat`, …) is aspirational, not real.

---

## 5. Definition of "done" going forward (replaces `represented-complete`)

A capability is **done** only when: (1) it runs in the **live runtime**, (2) its wire output
is **capture-clean vs a C++ capture** of the same action — byte/opcode exact unless a narrowly
reviewed comparator omits only an intrinsically runtime-allocated identifier or canonicalizes
only proven unordered C++ collection order, while retaining every stable value/count and failing
malformed input — (3) it has been **exercised on a running server/client**, and (4) C++ refs +
the validating capture/test are cited.
`represented-complete` is no longer a closure state — it means "logic drafted, not yet live".
