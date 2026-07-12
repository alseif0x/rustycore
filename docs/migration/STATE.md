# RustyCore — Honest Current State (single source of truth)

**Date:** 2026-06-27 · **Base:** `develop` @ `d171117c` (audited HEAD, not a stale checkpoint).

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

---

## 1. The honest progress picture (three axes, not one number)

| Axis | Estimate | Meaning |
|---|---:|---|
| **A. Breadth represented** | ~98% of R8 rows touched | Logic exists/contrasted in the represented model. **Retired as a headline** — rewards breadth over working features. |
| **B. Live-playable core** | **partial, holed, buggy** | Core loop (login→move→melee→loot→quest→vendor→group) is live, **but carries CRIT data-loss/dupe bugs** (D-track) and wrong combat math. Spells, creature AI, world-interaction, death are represented-only. |
| **C. Full 1:1 parity** | **low** | Long tail absent: ~108 spell effects, ~255 aura types, content scripts (0/294k LOC), mail/AH/calendar live, BG/arena/instances, ~215 stat/data stores. |

Part 1 of the plan drives **B** to complete; Part 2 drives **C** to complete.

---

## 2. Grounded capability matrix (verified to subsystem level)

**WORKS** = mutates live state + persists/broadcasts like C++ · **WORKS⚠** = live + persists
**but has correctness/integrity bugs** (see [EXISTING-CODE-DEFECTS.md](EXISTING-CODE-DEFECTS.md)) ·
**PARTIAL** = subset only · **STUB / REPRESENTED-ONLY** = validates/records intent, no
observable mutation · **ABSENT**.

> ⚠ **An adversarial audit of the "WORKS" surface found bugs in nearly all of it** — see
> [EXISTING-CODE-DEFECTS.md](EXISTING-CODE-DEFECTS.md) (D-track). "WORKS" below means the
> path is live, not that it is correct or safe against data loss/dupe.

### Core gameplay loop — live, but D-track bugs inside
| Capability | Status | Evidence / defects |
|---|---|---|
| Auth/BNet SRP6 + world-enter handshake | WORKS | recent `fix(bnet)` commits; played live |
| Player movement + broadcast to nearby | WORKS⚠ | `movement.rs:310`; trust-client position (D-H10), creature destroy deferred (D-H15), async CREATE race (D-H14) |
| Melee combat (deals damage→death→loot) | **PARTIAL** | `session.rs:47635`; **no damage formula / hit table / armor mitigation** (D-H1, D-H2) — numbers are wrong |
| Global creature runtime (aggro/melee/move→packets) | WORKS (default on) | `world-server/src/main.rs:12682+` |
| Inventory equip/swap/move/destroy (+DB) | WORKS⚠ | `character.rs:11611,11762`; **enchant/random-prop lost on relog (D-C1/C2)**, swap not transactional (D-C4) |
| Loot items + money (+DB) | WORKS⚠ | `loot.rs:1073,1262`; **dupe races (D-C5/C6)**, quest-credit gap (D-H6) |
| Quests accept/turn-in core rewards (+DB) | **PARTIAL** | `quest.rs:1359,5569`; kill/explore/item objectives may not auto-advance (D-H4/H5/H6) |
| Vendor buy/sell, Trainer learn, Groups (+DB) | WORKS⚠ | atomicity (D-C8), oversell (D-H11), trainer skips validation (D-H9), group race (D-C9) |
| Gossip menus + quest-giver status icons | WORKS | `handlers/quest.rs:1248`; gossip conditions evaluated |
| Item enchant/gem/socket, durability repair, binding | WORKS⚠ | `wow-entities/src/item.rs`; correct in-memory, but **not reloaded from DB (D-C1)** |
| Bank / equipment-sets / void-storage persistence | **STUB** | recorded in-memory, **never saved** (D-C3/M6/M7) — full loss on logout |
| UpdateFields / CREATE-block serialization | WORKS | `wow-packet/src/packets/update.rs` (capture-diffed); minor value gaps in M1.4 |

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
| Creature AI (threat gen / spell cast / waypoints / SmartAI / text) | **STUB** | `wow-ai/src/lib.rs` = selection only; "stand+aggro+melee"; SmartAI never interpreted |
| Creature movement: spline broadcast (SMSG_MONSTER_MOVE) | **STUB** | spline computed in `wow-movement/spline.rs` but never serialized/sent |
| MotionMaster tick | **STUB** | `motion_master.rs:319` `update()` exists, **never called**; generators called inline |
| Pathfinding (Detour navmesh query) | **STUB** | navmesh loaded; `find_path()` never invoked → straight-line |
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

## 3. Known live bugs blocking "playable" (open GitHub issues)

| # | Bug | Effect | Status |
|---|---|---|---|
| #7 | `SMSG_LOAD_CUF_PROFILES` → client Lua error | **Bags never open** | proven RustyCore-specific |
| #8 | persistent-deflate desync vs C++ on large SMSG | spells/UpdateObject empty | worked around (compression off, `fa86e19e`) |
| #13 | `CMSG_GAME_OBJ_USE` doesn't cast GO use-spell | **portals do nothing** | open |
| #9–#12 | 33 login-burst divergences (`world-load-audit.md`) | ordering/value parity | mostly open |

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
is **byte/opcode-clean vs a C++ capture** of the same action, (3) it has been **exercised on
a running server/client**, and (4) C++ refs + the validating capture/test are cited.
`represented-complete` is no longer a closure state — it means "logic drafted, not yet live".
