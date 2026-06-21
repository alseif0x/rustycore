# Creature Port Completion Plan - no gaps

**Date:** 2026-06-21
**Base:** `develop` at `ea586d2f`
**Canonical source:** `/home/server/woltk-trinity-legacy`
**Scope:** WoW Wrath of the Lich King Classic 3.4.3.54261 creature runtime and creature-facing NPC behavior.

This document is the operating plan for finishing the Creature port without carrying hidden gaps. It supersedes stale status text in `docs/migration/entities-creature.md` for execution planning. That older file remains useful as broad inventory, but this plan is the checklist to close.

## Non-negotiable rules

1. C++ is the source of truth. For Creature work, the canonical paths are:
   - `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/`
   - `/home/server/woltk-trinity-legacy/src/server/game/Movement/`
   - `/home/server/woltk-trinity-legacy/src/server/game/Maps/`
   - direct dependencies in `Entities/Unit`, `Entities/Object`, `AI`, `Scripting`, `Conditions`, `Loot`, `Pools`, `Events`, `Phasing`, and `Grids`.
2. C# is not authoritative for Creature. Any Rust code or docs mentioning "C# order", "C# reference", "C# format", or equivalent in a Creature path is a migration bug until proven otherwise. The fix is: contrast the behavior in detail against C++, replace the behavior if needed, and remove the C# reference.
3. No gameplay patch is accepted as "fixed" unless it has:
   - exact C++ file and line references,
   - Rust implementation references,
   - positive tests,
   - negative/regression tests for the crash or bug class,
   - real-client verification when update-object, visibility, interaction, movement, or login-world-entry packets are involved.
4. No diagnostic flag may be required for normal login or Creature rendering. Diagnostic flags can exist only for temporary investigation and must be documented as temporary.
5. If a missing dependency is found while porting Creature, implement the dependency or split it into an explicitly blocking subtask in this plan. Do not leave an invisible "later" gap.
6. If C++ appears buggy, document it as a `C++ parity bug candidate`, reproduce it, propose the smallest compatible fix, and ask before intentionally diverging from C++ behavior.
7. Every slice must compile, keep the suite green, and leave the tree clean before moving to the next slice.

## C++ anchors already rechecked

These are not the whole surface, only the anchors that define the shape of the port:

| Area | C++ anchor | Why it matters |
|---|---|---|
| Entry initialization | `Creature.cpp:491-575` (`Creature::InitEntry`) | Template/difficulty/model/equipment/speed/static flags/default movement are set here. |
| Entry update | `Creature.cpp:577-685` (`Creature::UpdateEntry`) | Faction, npc flags, unit flags, stats, react state, immunities, movement flags, addon and sparring load happen here. |
| Creature creation | `Creature.cpp:1062-1105` and following (`Creature::Create`) | Map binding, DB phasing, respawn compatibility, relocate before proto, terrain status and template validation happen before full creation. |
| DB load | `Creature.cpp:1815-1923` (`Creature::LoadFromDB`) | Spawn id, duplicate handling, spawn-group/respawn state, health, default movement and `AddToMap` order. |
| Main update | `Creature.cpp:696-760` and following (`Creature::Update`) | Creature update handles just-appeared hooks, movement flags, death/respawn and live AI/combat behavior. |
| Addon | `Creature.cpp:2734-2797` (`Creature::LoadCreaturesAddon`) | Mount, stand state, visibility, hover, sheath, kits, path id and addon auras. |
| Looted corpse | `Creature.cpp:2942-2968` and following (`Creature::AllLootRemovedFromCorpse`) | Corpse decay behavior after loot/skinning. |
| Motion init | `Creature.cpp:1026-1060`, `MotionMaster.cpp:97-130`, `MotionMaster.cpp:1207-1214` | `AIM_Create` initializes motion before AI selection, and `MotionMaster` selects default generator. |
| Default generator selection | `MovementGenerator.cpp:41-60` | Idle/random/waypoint generator creation. |
| Random movement | `RandomMovementGenerator.cpp:77-222` and following | Random reference position, LOS/pathfinding, walk/run, timers and formation signaling. |
| Waypoint movement | `WaypointMovementGenerator.cpp:120-210`, `267-380`, `433-455` | DB path load, delay, AI waypoint hooks, next-node computation and home-position updates. |
| Map tick owner | `Map.cpp:717-760` and following (`Map::Update`) | Sessions update first, then respawns, then ObjectUpdater for creatures/pets, then object updates and map drains. |
| Map respawns | `Map.cpp:2187-2225` and related respawn methods | Respawn state belongs to the map, not the session. |

## Current Rust surface to audit

Do not assume any of these are correct because they exist. Each must be checked against the C++ anchors above:

| Rust area | Current files |
|---|---|
| Entity lifecycle data | `crates/wow-entities/src/creature.rs` |
| Legacy runtime creature state | `crates/wow-world/src/map_manager.rs` |
| Session-local runtime and packet paths | `crates/wow-world/src/session.rs` |
| DB loaded-grid lifecycle resolver | `crates/world-server/src/creature_loaded_grid.rs` |
| Spawn-store loader | `crates/world-server/src/spawn_store_loader.rs` |
| Login/grid bridge and runtime loop wiring | `crates/world-server/src/main.rs` |
| Canonical map/ECS mirror | `crates/wow-map/src/manager.rs`, `crates/wow-map/src/map.rs` |
| AI foundation | `crates/wow-ai/src/lib.rs` |
| Data/global stores | `crates/wow-data`, `crates/wow-database`, `docs/migration/globals.md` |

## Definition of done for any Creature slice

A Creature slice is done only when all of this is true:

1. The C++ behavior is cited with exact file/line refs.
2. The Rust code maps to the same order and same state transitions, not merely similar outputs.
3. Tests prove the normal case and at least one failure/regression case.
4. If the slice affects packets or what the client sees, a real 3.4.3.54261 client has been tested without diagnostic flags.
5. Existing C#-derived comments in the touched area are treated as bugs and must be removed or rewritten with exact C++ references after the behavior is re-audited.
6. Docs and inventory are updated in the same commit.
7. No new TODO/gap remains in the touched C++ surface. If the surface requires a dependency, the dependency is either implemented in the same slice or becomes the next blocking slice in this plan.

## Work breakdown

### CREATURE-P0 - Build the parity matrix first

**Goal:** create the exhaustive Creature matrix before more implementation.

**Actions:**
- Create `docs/migration/inventory/creature-port-matrix.tsv`.
- Enumerate every relevant symbol from:
  - `Entities/Creature/Creature.cpp`
  - `Entities/Creature/Creature.h`
  - `Entities/Creature/CreatureData.h`
  - `Entities/Creature/CreatureGroups.*`
  - `Entities/Creature/TemporarySummon.*`
  - `Entities/Creature/GossipDef.*`
  - `Entities/Creature/Trainer.*`
  - `Movement/MotionMaster.cpp`
  - `Movement/MovementGenerator.cpp`
  - `Movement/MovementGenerators/RandomMovementGenerator.*`
  - `Movement/MovementGenerators/WaypointMovementGenerator.*`
  - map/object/unit functions directly called by Creature.
- For every row record:
  - C++ file:line range,
  - Rust file/function,
  - status: `missing`, `represented`, `runtime-live`, `tested`, `manual-verified`, `excluded-wotlk`,
  - dependencies,
  - tests,
  - notes.
- Add an explicit C#-contamination column for any Rust/doc path that was ported from C# or still cites C#. Every positive entry becomes a bug row until the C++ contrast is complete and the C# reference is removed.

**Done when:**
- The matrix covers all Creature C++ files listed above.
- `entities-creature.md` points to the matrix and this plan.
- Nothing is marked complete without a test or manual proof reference.

### CREATURE-P1 - Treat and remove C# contamination as bugs

**Goal:** stop reasoning from the wrong source before touching more runtime code. A C# reference in Creature is not documentation; it is a bug marker.

**Actions:**
- Search Creature-adjacent code/docs for `C#`, `c#`, `CSharp`, `format`, and old diagnostic references.
- For each finding, create or update a matrix bug row with:
  - exact Rust/doc path,
  - current claim,
  - exact C++ behavior and file:line refs,
  - required Rust change,
  - test coverage,
  - status.
- If the Rust behavior already matches C++, remove the C# reference and replace it with the C++ file:line refs.
- If the Rust behavior does not match C++, reimplement it from C++ and remove the C# reference in the same slice.
- If the finding is unrelated to Creature, mark it out of scope in the Creature matrix and leave the fix to the owning module.
- If it is a temporary diagnostic, remove it from normal runtime or keep it behind an explicitly temporary debug gate with an owner and removal condition.

**Done when:**
- No Creature production path or Creature migration plan line depends on a C# claim.
- No touched Creature code/docs keep a C# reference after the C++ contrast is complete.
- Every C#-contamination row in the matrix is either fixed or assigned to another non-Creature module with evidence.

### CREATURE-P2 - Exact load/create lifecycle

**Goal:** make every spawn enter Rust through the same lifecycle order as C++.

**C++ refs:**
- `Creature::InitEntry` - `Creature.cpp:491-575`
- `Creature::UpdateEntry` - `Creature.cpp:577-685`
- `Creature::Create` - `Creature.cpp:1062+`
- `Creature::LoadFromDB` - `Creature.cpp:1815-1923`
- `Creature::LoadCreaturesAddon` - `Creature.cpp:2734-2797`

**Actions:**
- Verify all `CreatureData`, `CreatureTemplate`, `CreatureDifficulty`, addon, movement-template, display/model, equipment, static-flag and phasing fields are loaded.
- Ensure lifecycle order is exactly:
  1. spawn lookup and duplicate/alive/dead handling,
  2. map binding and DB phase init,
  3. relocate and terrain status,
  4. template/difficulty/model/equipment,
  5. level/stats/health/mana/resistances,
  6. faction/npc flags/unit flags/dynamic flags,
  7. movement flags/addon/sparring/immunities,
  8. AI/motion initialization,
  9. map insertion and canonical mirror.
- Audit all spawn paths:
  - initial grid load,
  - login grid mirror,
  - respawn,
  - game event spawn,
  - pool/condition spawn,
  - summon/temp summon,
  - GM/debug spawn if present.
- Ensure legacy and canonical creature state cannot diverge after any of those paths.

**Tests:**
- Lifecycle record golden tests for template, spawn, addon and movement decisions.
- Duplicate spawn/dead spawn tests.
- Login grid load mirror test.
- Respawn rebuild preserves default movement, waypoint path, flags and addon-derived state.
- C++ vs Rust log diff for at least one known area with static, random and waypoint creatures.

**Done when:**
- Every C++ lifecycle field in the matrix is `runtime-live` or explicitly blocked by a dependency that is implemented next.
- The real client can enter a creature-dense area without diagnostic flags.

### CREATURE-P3 - Map-owned creature runtime

**Goal:** remove the inverted session-owned creature runtime and follow C++ map ownership.

**C++ refs:**
- `Map::Update` - `Map.cpp:717-760+`
- `Creature::Update` - `Creature.cpp:696+`
- `Map` respawn storage - `Map.cpp:2187+`

**Actions:**
- Finish the global legacy creature tick path so it owns movement/lifecycle updates once per map.
- Keep the single-owner invariant: session tick and global tick must never both advance the same creature.
- Move session-local creature lifecycle leftovers into map-owned state or delete them once replaced.
- Ensure no packets are sent under map locks.
- Ensure legacy and canonical locks are never nested.
- Make the experimental global creature runtime the normal runtime only after real-client proof.

**Tests:**
- Two sessions on same map tick one creature once.
- No movement/lifecycle double-send with two sessions.
- No tick when owner is `Session`; no session tick when owner is `GlobalLegacy`.
- Fanout reaches all eligible sessions and rejects wrong map/instance.
- Real-client login and multi-session visibility test.

**Done when:**
- Creature update is map-owned in production.
- Session code no longer contains active creature lifecycle/movement ownership except for per-player visibility and per-player combat.

### CREATURE-P4 - C++ MotionMaster and default movement generators

**Goal:** replace ad-hoc creature movement with the C++ MotionMaster model.

**C++ refs:**
- `MotionMaster::Initialize`, `InitializeDefault`, `DirectInitialize` - `MotionMaster.cpp:97-130`, `1207-1214`
- generator factories - `MovementGenerator.cpp:41-60`
- random movement - `RandomMovementGenerator.cpp:77-222+`
- waypoint movement - `WaypointMovementGenerator.cpp:120-455+`

**Actions:**
- Model MotionMaster slots and default generator selection.
- Port idle/random/waypoint default generator behavior exactly enough for live creatures.
- Preserve `m_defaultMovementType`, `m_wanderDistance`, `_waypointPathId`, current waypoint info and home position.
- Implement random movement:
  - reference position,
  - wander distance,
  - 2..10 wander steps,
  - LOS check,
  - pathfinding failure retry timers,
  - walk/run decision from movement template,
  - formation movement signal.
- Implement waypoint movement:
  - DB path lookup via addon path id,
  - one-node non-repeat behavior,
  - initial delay,
  - node delays,
  - AI hooks (`WaypointStarted`, `WaypointReached`, `WaypointPathEnded`),
  - home position updates,
  - forward/backward path flags.
- Remove any "random movement if timer says so" shortcuts that do not map to a C++ generator state.

**Tests:**
- Static/sessile/rooted NPCs never wander.
- Random movement only for C++-eligible creatures.
- Waypoint creature starts after the same delay, follows same node sequence and updates current waypoint info.
- Movement packets are generated only when C++ would launch a spline.
- Real-client: static NPCs stay still; known waypoint NPCs move.

**Done when:**
- Movement state is generator-based, not a flat boolean/timer approximation.

### CREATURE-P5 - AI lifecycle and creature brain

**Goal:** make Creature AI creation and update follow C++ order and hooks.

**C++ refs:**
- `Creature::AIM_Create` - `Creature.cpp:1026-1032`
- `Creature::AIM_Initialize` - `Creature.cpp:1035-1043`
- `Creature::Update` - `Creature.cpp:696+`
- `AI` and `Scripting` dependencies under `/src/server/game/AI/` and `/src/server/game/Scripting/`.

**Actions:**
- Port AI selection (`FactorySelector::SelectAI`) for core creature AI classes required by current DB.
- Port JustAppeared/InitializeAI/JustRespawned/JustDied/MovementInform hooks.
- Port MoveInLineOfSight and aggro start rules for basic hostile creatures.
- Wire script-name and SmartAI hooks only when their data loaders exist.
- Implement missing dependencies encountered here rather than stubbing them silently.

**Tests:**
- Passive NPC does not aggro.
- Hostile NPC aggro range follows C++.
- Waypoint AI hooks fire once and in order.
- Respawn calls AI hooks in C++ order.

**Done when:**
- Basic creature AI updates from the map tick and no longer relies on per-session approximations.

### CREATURE-P6 - Creature combat, threat, death and respawn

**Goal:** port creature-owned combat exactly, while keeping player-owned combat where C++ owns it.

**C++ refs:**
- `Creature::Update` - creature `DoMeleeAttackIfReady`/AI update behavior.
- `Unit` combat/threat/death functions called by Creature.
- `Creature::setDeathState`, `Creature::Respawn`, `Creature::RemoveCorpse`, `Creature::AllLootRemovedFromCorpse`.

**Actions:**
- Separate player auto-attack ownership from creature melee ownership.
- Port creature victim selection, threat, melee readiness, damage, sparring, evade/leash, cannot-reach state and assistance calls.
- Port death state transitions:
  - alive,
  - just died,
  - corpse,
  - dead,
  - just respawned.
- Port corpse decay, loot removed behavior, skinning special cases and respawn timers.
- Port linked respawn, spawn-group active delay, pool interaction and persistence.

**Tests:**
- One creature attacks one player once per swing, not per session.
- Two players near one creature do not double-resolve melee.
- Death creates corpse state and later respawn state in C++ order.
- Looted corpse decay follows C++ timing gates.
- Respawn preserves movement/addon/path state.

**Done when:**
- Creature combat/death/respawn is map-owned, deterministic and multi-session safe.

### CREATURE-P7 - Visibility, object updates and create packets

**Goal:** make the client receive exactly the Creature create/update/remove packets it expects.

**C++ refs:**
- `Object::BuildCreateUpdateBlockForPlayer`
- `Object::BuildValuesUpdateBlockForPlayer`
- `Map::SendObjectUpdates`
- `Player::UpdateVisibilityOf`
- `WorldObject::SendMessageToSetInRange` and `MessageDistDeliverer`.

**Actions:**
- Audit every Creature update-field mask and create-block field against C++.
- Treat C#/diagnostic packet ordering assumptions in Creature create/update as bugs; contrast against C++ and remove the references.
- Implement per-session `HaveAtClient` as the final gate, not a registry approximation.
- Ensure CREATE/DESTROY modifies the receiver session's visible set.
- Ensure movement update packets use the same spline/movement layout as C++.
- Compare Rust vs C++ logs for:
  - create count,
  - GUIDs,
  - object type,
  - movement block,
  - update-field mask,
  - packet order during world entry.

**Tests:**
- Golden bit/byte tests for Creature create/update fields.
- Real-client login to creature-dense zones without crash.
- Real-client NPC visibility, despawn, respawn and movement fanout.

**Done when:**
- Normal login/world entry works without `RUSTYCORE_LOGIN_UPDATEOBJECT_DIAGNOSTIC` or equivalent flags.

### CREATURE-P8 - NPC interactions attached to Creature

**Goal:** finish the creature-facing gameplay that players immediately touch.

**C++ refs:**
- `GossipDef.*`
- `Trainer.*`
- Creature methods for vendor/gossip/trainer/taxi/banker/battlemaster roles.
- `ObjectMgr` loaders for `npc_vendor`, `npc_trainer`, `gossip_menu`, quest starters/enders and related condition tables.

**Actions:**
- Port and wire:
  - gossip hello/select/complete,
  - questgiver status and quest lists,
  - quest accept/complete/reward entry points,
  - vendor inventory, buy, sell and reference vendors,
  - trainer list, learn spell, requirement checks and money,
  - repair/banker/taxi/stable/binder/spirit healer where present in WotLK,
  - battlemaster entry points if the client exposes them.
- Every role must use C++ flags and condition checks. No default-true conditions for production NPC menus.
- Inventory/bag UI failures must be handled here when caused by item/vendor/player-storage gaps.

**Tests:**
- Real-client:
  - open gossip NPC,
  - see quest markers and quest details,
  - open vendor inventory,
  - buy/sell one safe test item,
  - open trainer list,
  - learn one safe test spell if DB/account allows,
  - open bag/inventory before and after interaction.
- Unit tests for vendor reference expansion, trainer spell state and condition rejection.

**Done when:**
- The common NPC interaction loop works in the real client without crash or silent no-op.

### CREATURE-P9 - Formations, summons, pets, vehicles and transports

**Goal:** cover Creature subclasses/companions and group motion instead of treating all NPCs as isolated static units.

**C++ refs:**
- `CreatureGroups.*`
- `TemporarySummon.*`
- `Pet` and `Vehicle` dependencies used by Creature.
- `Creature::Motion_Initialize` formation branch.

**Actions:**
- Port formation leader/member storage and reset/signal behavior.
- Port temp summon timers and owner relation.
- Port creature-backed pets/totems/guardians only to the WotLK scope needed by client/DB.
- Port vehicle kit reset and creature vehicle state used by world entry and NPC behavior.
- Port transport creature home-position handling.

**Tests:**
- Formation follower waits/moves like C++.
- Summon despawns by timer/death condition.
- Vehicle creature initializes without corrupting create/update packets.
- Transport creature has correct map/home position.

**Done when:**
- Creature group/summon paths in the matrix are not marked `missing`.

### CREATURE-P10 - Conditions, pools, events, phasing and scripting integration

**Goal:** make spawned creatures and NPC options appear/disappear under the same world rules as C++.

**Actions:**
- Port Creature consumers of:
  - Conditions,
  - Pools,
  - GameEvents,
  - SpawnGroups,
  - Phasing,
  - Scripts/SmartAI,
  - Weather/script hooks reached from map update.
- Do not fake these as always true. If a dependency is missing, implement the dependency surface needed by Creature.

**Tests:**
- Event-spawn creature appears/disappears.
- Condition-hidden vendor/gossip option stays hidden.
- Phase-hidden creature is not visible to a player in the wrong phase.
- Pool spawn does not duplicate or orphan respawn state.

**Done when:**
- Creature spawn and interaction visibility is driven by the same conditions as C++.

### CREATURE-P11 - C++ vs Rust runtime diff harness

**Goal:** stop guessing during client crashes.

**Actions:**
- Add temporary, opt-in structured logs for C++ and Rust comparison:
  - world entry creature grid load,
  - loaded spawn IDs,
  - created GUIDs,
  - movement type/path/wander values,
  - update-object create size and field-mask summary,
  - visibility CREATE/DESTROY decisions,
  - movement packets.
- Keep logs controlled by a debug-only config/env var and never required for production behavior.
- Produce one captured C++ successful login log and one Rust successful login log for the same character/map.
- Store the analysis summary in `docs/migration/current-session-handoff.md` or a dedicated `docs/migration/creature-runtime-diff-*.md`.

**Done when:**
- We can answer "what differed from C++ before the crash" from logs instead of trial-and-error toggles.

### CREATURE-P12 - Closure audit

**Goal:** declare Creature complete only when the matrix proves it.

**Checks:**
- `docs/migration/inventory/creature-port-matrix.tsv` has no `missing`, `represented-only`, or unowned `blocked` rows for WotLK Creature.
- `rg -n "C#|c#|RUSTYCORE_LOGIN_UPDATEOBJECT_DIAGNOSTIC|diagnostic" crates/wow-world crates/world-server crates/wow-entities` has no Creature production behavior dependency.
- No Creature packet, movement, runtime or NPC-service code relies on a diagnostic flag.
- Tests pass:
  - `PROTOC=/home/cdmonio/.local/protoc/bin/protoc cargo test -p wow-world`
  - `PROTOC=/home/cdmonio/.local/protoc/bin/protoc cargo test -p world-server`
  - `PROTOC=/home/cdmonio/.local/protoc/bin/protoc cargo test -p wow-map`
  - targeted tests for `wow-entities`, `wow-data`, `wow-ai`, `wow-loot` if touched.
- Real-client verification passes:
  - login to a creature-dense area,
  - NPCs visible,
  - static NPCs stay static,
  - waypoint/random NPCs move,
  - bag/inventory opens,
  - gossip opens,
  - questgiver status/list works,
  - vendor inventory opens,
  - trainer list opens,
  - combat/death/respawn works,
  - second client sees the same creature movement/respawn.

**Done when:**
- Creature can be marked `runtime-live + manual-verified` in the migration matrix and the old stale status in `entities-creature.md` is replaced.

## Immediate next slice

Start with **CREATURE-P0 + CREATURE-P1**, not more runtime guessing:

1. Create the exhaustive matrix.
2. Mark every current Creature behavior that still cites C# or diagnostics.
3. Pick the first missing C++ function in the matrix that is on the world-entry crash path.
4. Implement that function 1:1 from C++ with tests.

The likely first code target after the matrix is **CREATURE-P2/P7 overlap**: exact C++ create/update object data for Creature during world entry. That is the path currently capable of crashing the 3.4.3.54261 client, so it should be driven by C++/Rust log diff, not by toggles.
