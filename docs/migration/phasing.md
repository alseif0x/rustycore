# Migration: Phasing

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Phasing/` (`PhaseShift`, `PhasingHandler`, `PersonalPhaseTracker`)
> **Rust target crate(s):** `crates/wow-world/` (PhasingHandler logic, per-map MultiPersonalPhaseTracker), `crates/wow-data/` (load C++ `Phase.db2`-seeded phase info / `phase_area` / `terrain_swap_defaults` / `terrain_worldmap`), `crates/wow-packet/src/packets/misc.rs` (already has `PhaseShiftChange` stub). No dedicated `wow-phasing` crate yet.
> **Layer:** L7 (Game systems — depends on Conditions L7, Maps L4, Entities/Unit L4, Aura/Spell L5; depended on by Visibility/Grid loading L4)
> **Status:** 🔧 broken — core `PhaseShift` state and the real SMSG_PHASE_SHIFT_CHANGE packet shape exist, but login still uses the default empty phase shift and the runtime lifecycle is incomplete. No `PhasingHandler::OnAreaChange`, no condition-driven phase suppression, no personal phases, no full terrain-swap evaluation, no controlled-unit propagation, no SPELL_AURA_PHASE / SPELL_AURA_PHASE_GROUP integration.
> **Audited vs C++:** ✅ audited 2026-05-01 (status confirmed 🔧 — hardcoded-Unphased bug located at `misc.rs:1628`)
> **Last updated:** 2026-05-01

---

## 1. Purpose

Phasing is TrinityCore's per-object visibility partition: each `WorldObject` carries a `PhaseShift` (set of phase IDs + flags + suppressed shifts + visible-map IDs + UI-map IDs) and two objects only see each other if their `PhaseShift::CanSee` predicate holds. It is the mechanism behind quest-driven world changes, instance terrain swaps, garrison/scenario private spawns, cosmetic-only phases (Cosmetic flag), and `Personal` phases (per-owner spawn instances inside a regular grid). Personal phases additionally need a parallel `MultiPersonalPhaseTracker` per `Map` that loads/unloads private spawns when the owner enters/exits.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Phasing/PersonalPhaseTracker.cpp` | 202 | `prefix` |
| `game/Phasing/PersonalPhaseTracker.h` | 85 | `prefix` |
| `game/Phasing/PhaseShift.cpp` | 203 | `prefix` |
| `game/Phasing/PhaseShift.h` | 139 | `prefix` |
| `game/Phasing/PhasingHandler.cpp` | 701 | `prefix` |
| `game/Phasing/PhasingHandler.h` | 91 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/game/Phasing/PhaseShift.h` | 139 | `PhaseShiftFlags`, `PhaseFlags`, `PhaseShift` class with `PhaseRef`/`VisibleMapIdRef`/`UiMapPhaseIdRef`, FlatSet-backed `Phases` container; `DEFAULT_PHASE = 169` |
| `src/server/game/Phasing/PhaseShift.cpp` | 203 | `AddPhase`/`RemovePhase`/`AddVisibleMapId`/`AddUiMapPhaseId`, refcount bookkeeping (`NonCosmeticReferences`, `CosmeticReferences`, `PersonalReferences`, `DefaultReferences`), `CanSee` algorithm, `UpdateUnphasedFlag`, `UpdatePersonalGuid` |
| `src/server/game/Phasing/PhasingHandler.h` | 91 | Static façade: `AddPhase`/`AddPhaseGroup`/`AddVisibleMapId`/`ResetPhaseShift`/`InheritPhaseShift`/`OnMapChange`/`OnAreaChange`/`OnConditionChange`/`SendToPlayer`/`InitDbPhaseShift`/`InitDbPersonalOwnership`/`InitDbVisibleMapId`/`GetTerrainMapId`/`SetAlwaysVisible`/`SetInversed`/`PrintToChat`/`FormatPhases`/`IsPersonalPhase` |
| `src/server/game/Phasing/PhasingHandler.cpp` | 701 | All static methods, the `ControlledUnitVisitor` helper that recursively propagates phase changes through `Unit::m_Controlled`, `m_SummonSlot`, vehicle passengers; SMSG_PHASE_SHIFT_CHANGE serialization; chat-print debug helpers |
| `src/server/game/Phasing/PersonalPhaseTracker.h` | 85 | `PersonalPhaseSpawns` (objects + grid set + optional countdown), `PlayerPersonalPhasesTracker` (per-owner phase→spawns map), `MultiPersonalPhaseTracker` (per-map tracker keyed by owner GUID) |
| `src/server/game/Phasing/PersonalPhaseTracker.cpp` | 202 | `LoadGrid`/`UnloadGrid`, `RegisterTrackedObject`, `OnOwnerPhasesChanged`, `MarkAllPhasesForDeletion`, `Update` (1-minute despawn countdown), `IsGridLoadedForPhase`/`SetGridLoadedForPhase` |

Out-of-tree touchpoints:
- `src/server/game/Globals/ObjectMgr.cpp` — `LoadPhases` (seeds phase info from `sPhaseStore`, loads `phase_area`), `LoadTerrainSwapDefaults` (loads `terrain_swap_defaults`, `terrain_worldmap`).
- `src/server/game/Maps/Map.cpp` — owns `MultiPersonalPhaseTracker _multiPersonalPhaseTracker` and calls its `LoadGrid`/`UnloadGrid`/`Update` from grid lifecycle hooks.
- `src/server/game/Spells/Auras/SpellAuraEffects.cpp` — `SPELL_AURA_PHASE` / `SPELL_AURA_PHASE_GROUP` apply/remove handlers call `PhasingHandler::AddPhase` / `AddPhaseGroup`.
- `src/server/game/Conditions/ConditionMgr.cpp` — `CONDITION_SOURCE_TYPE_PHASE` and `CONDITION_SOURCE_TYPE_TERRAIN_SWAP` rule storage.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `PhaseShift` | class | The per-object phase state (set of `PhaseRef`, set of visible map IDs, set of UI-map phase IDs, flags, personal owner GUID); copyable — every `WorldObject` has a primary `PhaseShift` and a `SuppressedPhaseShift` |
| `PhaseShift::PhaseRef` | struct | `Id (u16)`, `Flags (PhaseFlags)`, `References (i32)`, `AreaConditions: const std::vector<Condition>*` (back-pointer to ConditionMgr-owned vector); ordered by `Id` (FlatSet) |
| `PhaseShift::VisibleMapIdRef` | struct | `References`, `VisibleMapInfo: const TerrainSwapInfo*` (terrain swap metadata) |
| `PhaseShift::UiMapPhaseIdRef` | struct | `References` only (UI map phase IDs are pulled from `TerrainSwapInfo::UiMapPhaseIDs`) |
| `PhaseShiftFlags` | enum u32 (flags) | `None=0`, `AlwaysVisible=0x01` (ignore phasing entirely), `Inverse=0x02` (see if any phase NOT shared), `InverseUnphased=0x04`, `Unphased=0x08` (default — match Unphased peers), `NoCosmetic=0x10` (ignore cosmetic-only intersections) |
| `PhaseFlags` | enum u16 (flags) | `None=0x0`, `Cosmetic=0x1` (decorative-only intersection), `Personal=0x2` (per-owner spawn — must combine with `PhaseShift::PersonalGuid`) |
| `PhasingHandler` | static class | Façade: every callsite that mutates phases goes through here so controlled units (pets/vehicles/totems) inherit; also the only place that emits SMSG_PHASE_SHIFT_CHANGE |
| `PhasingHandler::ControlledUnitVisitor` | private inner class | Walks `Unit::m_Controlled`, `m_SummonSlot`, `Vehicle::Seats[].Passenger` with a "visited" small_vector to avoid revisiting in cycles |
| `PhaseAreaInfo` | struct (defined in ObjectMgr) | One row of `phase_area`: `PhaseInfo: PhaseInfoStruct const*`, `Conditions: ConditionContainer`, `SubAreaExclusions: unordered_set<u32>` |
| `PhaseInfoStruct` | struct (defined in ObjectMgr) | One phase entry seeded from `sPhaseStore`: `Id`, `Areas` |
| `TerrainSwapInfo` | struct | One row of `terrain_swap_defaults`/`terrain_worldmap`: `Id` (alt map ID), `UiMapPhaseIDs: vector<u32>` |
| `PHASE_USE_FLAGS_*` | constants | `NORMAL=0`, `ALWAYS_VISIBLE=1`, `INVERSE=2` (DB column flags consumed by `InitDbPhaseShift`) |
| `DEFAULT_PHASE = 169` | constant | The "no phase" fallback used by `Unphased` flag interactions |
| `PersonalPhaseSpawns` | struct | `unordered_set<WorldObject*> Objects`, `unordered_set<u16> Grids`, `Optional<Milliseconds> DurationRemaining` (1 min default after owner leaves before despawn) |
| `PlayerPersonalPhasesTracker` | struct | One per personal-phase owner: `unordered_map<phaseId, PersonalPhaseSpawns> _spawns` |
| `MultiPersonalPhaseTracker` | struct (per-Map) | `unordered_map<ObjectGuid, PlayerPersonalPhasesTracker> _playerData`; entry point for grid load/unload |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `PhasingHandler::AddPhase(WorldObject*, u32 phaseId, bool updateVisibility)` | Public single-phase add; creates `ControlledUnitVisitor` and recurses into pets/vehicles/totems | `PhaseShift::AddPhase`, `Unit::OnPhaseChange`, `Unit::RemoveNotOwnSingleTargetAuras`, `UpdateVisibilityIfNeeded` |
| `PhasingHandler::AddPhaseGroup(WorldObject*, u32 phaseGroupId, bool)` | Resolve `phaseGroupId` via `sDB2Manager.GetPhasesForGroup` then add each constituent phase | `PhaseShift::AddPhase` per element |
| `PhasingHandler::OnMapChange(WorldObject*)` | Re-evaluate `terrain_swap_defaults` against current map and condition state, populate `VisibleMapIds` and `UiMapPhaseIds`, suppress those whose conditions fail | iterate `sObjectMgr->GetTerrainSwaps()`, `ConditionMgr::IsObjectMeetingNotGroupedConditions(CONDITION_SOURCE_TYPE_TERRAIN_SWAP, ...)` |
| `PhasingHandler::OnAreaChange(WorldObject*)` | Walk area→parent-area chain, look up `phase_area` rows, apply each phase that meets its `ConditionContainer`, then re-add SPELL_AURA_PHASE / SPELL_AURA_PHASE_GROUP auras still active | `sObjectMgr->GetPhasesForArea`, `ConditionMgr::IsObjectMeetToConditions`, `Unit::GetAuraEffectsByType(SPELL_AURA_PHASE)`, `ControlledUnitVisitor::VisitControlledOf`, `InheritPhaseShift` |
| `PhasingHandler::OnConditionChange(WorldObject*, bool updateVisibility=true)` | Re-evaluate every existing phase's `AreaConditions`; move newly-failing phases to `SuppressedPhaseShift`; promote newly-passing phases back; same for `VisibleMapIds`; preserve aura-driven phases | `ConditionMgr`, `PhaseShift::AddPhase/RemovePhase/AddVisibleMapId/RemoveVisibleMapId/AddUiMapPhaseId/RemoveUiMapPhaseId` |
| `PhasingHandler::ResetPhaseShift(WorldObject*)` | Clear both shift and suppressed shift (used on logout/world change) | `PhaseShift::Clear` |
| `PhasingHandler::InheritPhaseShift(target, source)` | Copy phase state from owner to controlled unit (pet, summon, vehicle passenger) | direct copy of both shifts |
| `PhasingHandler::SendToPlayer(Player const*, PhaseShift const&)` | Build `WorldPackets::Misc::PhaseShiftChange` (ClientGUID, Flags, PersonalGUID, list of `{flags, id}` phases, list of VisibleMapIDs, list of UiMapPhaseIDs) and send | `Player::SendDirectMessage` |
| `PhasingHandler::InitDbPhaseShift(PhaseShift&, u8 phaseUseFlags, u16 phaseId, u32 phaseGroupId)` | Build the static phase state of a DB-spawned creature/gameobject from `phaseUseFlags` (DB column) | `PhaseShift::AddPhase`, sets `IsDbPhaseShift = true` |
| `PhasingHandler::InitDbPersonalOwnership(PhaseShift&, ObjectGuid const&)` | Stamp the personal owner so a creature's personal phase resolves to that player only | sets `PersonalGuid` |
| `PhasingHandler::InitDbVisibleMapId(PhaseShift&, i32 visibleMapId)` | Apply explicit DB-driven terrain swap | `PhaseShift::AddVisibleMapId` |
| `PhasingHandler::GetTerrainMapId(PhaseShift const&, u32 mapId, TerrainInfo const*, x, y)` | Decide which map ID to actually sample for collision / area lookups when a terrain swap is active at coordinates | `TerrainInfo::GetAreaId` per visible map id, picks first match |
| `PhasingHandler::SetAlwaysVisible(WorldObject*, bool, bool)` | Apply/remove `PhaseShiftFlags::AlwaysVisible` (used by GMs and special invisible-but-everywhere npcs) | `UpdateVisibilityIfNeeded` |
| `PhasingHandler::IsPersonalPhase(u32 phaseId)` | DB2 lookup: phase has `PhaseEntryFlags::Personal` set | `sPhaseStore` |
| `PhaseShift::CanSee(PhaseShift const&)` | The actual visibility test executed by `WorldObject::IsWithinSightDist` & friends | iterates phase intersections honouring all four PhaseShiftFlags (AlwaysVisible / Inverse / InverseUnphased / Unphased / NoCosmetic) |
| `PhaseShift::AddPhase(u32 id, PhaseFlags, AreaConditions*, refs=1)` | Insert or refcount-bump a phase, update bucket counters (`NonCosmeticReferences` / `CosmeticReferences` / `PersonalReferences` / `DefaultReferences`), refresh `Unphased` flag | `ModifyPhasesReferences`, `UpdateUnphasedFlag` |
| `PhaseShift::RemovePhase(u32 id)` | Refcount-decrement; physically erase when references hit zero; return `EraseResult{iterator, erased}` | `ModifyPhasesReferences`, `UpdateUnphasedFlag` |
| `MultiPersonalPhaseTracker::LoadGrid(PhaseShift const&, NGridType&, Map*, Cell const&)` | When a player with personal phases enters a grid, load private spawns for those phases into that grid | `PlayerPersonalPhasesTracker::SetGridLoadedForPhase` |
| `MultiPersonalPhaseTracker::OnOwnerPhaseChanged(WorldObject const*, NGridType*, Map*, Cell const&)` | Despawn old phase spawns and load new phase spawns when the owner's phase set changes | `MarkAllPhasesForDeletion`, `LoadGrid` |
| `MultiPersonalPhaseTracker::Update(Map*, u32 diff)` | Tick — counts down `PersonalPhaseSpawns::DurationRemaining`, despawns expired phases (default 1 min after owner leaves) | `PlayerPersonalPhasesTracker::Update`, `DespawnPhase` |

---

## 5. Module dependencies

**Depends on:**
- `Conditions` — every `PhaseRef::AreaConditions` is a non-owning pointer into the ConditionMgr-owned vector for `CONDITION_SOURCE_TYPE_PHASE`; visible-map evaluation uses `CONDITION_SOURCE_TYPE_TERRAIN_SWAP`. `OnAreaChange` / `OnConditionChange` are 100% condition-driven.
- `DB2 stores` — `sPhaseStore` (Phase.db2 — provides `PhaseEntryFlags::Cosmetic` / `Personal`), `sDB2Manager.GetPhasesForGroup` (PhaseXPhaseGroup.db2), `sAreaTableStore` (parent-area chain walk).
- `ObjectMgr` — `GetPhasesForArea(areaId)` returns `vector<PhaseAreaInfo>` for a given area; `GetTerrainSwaps()` returns `terrain_swap_defaults`+`terrain_worldmap` joined; `GetTerrainSwapInfo(swapMapId)` for direct lookup.
- `Unit / Vehicle / Pet` — `Unit::m_Controlled`, `m_SummonSlot`, `Vehicle::Seats` are walked by `ControlledUnitVisitor`; `Unit::OnPhaseChange` is a virtual hook fired after every phase mutation; `Unit::RemoveNotOwnSingleTargetAuras(true)` cleans up auras whose target now phase-shifted away.
- `Spell aura system` — `SPELL_AURA_PHASE` (single phaseId in `MiscValueB`) and `SPELL_AURA_PHASE_GROUP` (phaseGroupId in `MiscValueB`) are the two aura effects that call `PhasingHandler::AddPhase` on apply / `RemovePhase` on remove.
- `Map / Grid` — `Map` owns `MultiPersonalPhaseTracker`; grid load/unload is the only event that loads private personal-phase spawns.
- `WorldPackets::Misc::PhaseShiftChange` — wire format for SMSG_PHASE_SHIFT_CHANGE.
- `WorldPackets::Party::PartyMemberPhaseStates` — emitted to party members by `FillPartyMemberPhase`.

**Depended on by:**
- `WorldObject::IsWithinSightDist` / `Object::CanSeeOrDetect` chain — the universal visibility predicate calls `PhaseShift::CanSee`.
- `Map::AddToMap` / grid notifiers — every spawn placement consults phases.
- `GameObject::IsConditionMeet` and gathering / quest / loot triggers — all check phase intersection against the looter.
- `Conversation`, `AreaTrigger`, `Scene`, `Garrison`, `Scenario` modules — push personal phases on mission start, pop on completion.
- `SmartScript` `SMART_ACTION_ADD_PHASE` / `REMOVE_PHASE` / `RANDOM_PHASE_GROUP` — script-driven phase mutation.

**Cross-module blockers:**
- See `conditions.md`: `#COND.4`, `#COND.7`, `#COND.17`, `#COND.18`, `#COND.20` and `#COND.29` are the minimum ConditionMgr surface needed before `#PHASE.8`, `#PHASE.15`, `#PHASE.16`, `#PHASE.17` and `#PHASE.29` can become fully wired. Until `ConditionsReference` exists, Rust phasing may keep injected predicates for unit tests but cannot claim parity with C++ `PhaseRef::AreaConditions`.
- See `maps.md`: `#MAPS.6`, `#MAPS.7`, `#MAPS.8`, `#MAPS.9`, `#MAPS.10`, `#MAPS.13`, `#MAPS.15`, `#MAPS.16`, `#MAPS.17`, `#MAPS.18` and `#MAPS.20` are the minimum Map/Grid surface for `#PHASE.22`, `#PHASE.23` and the remaining `#PHASE.26` visibility/grid notifier work. `MultiPersonalPhaseTracker` cannot be complete without C++-style grid load/unload and nearby-cell visitation.

---

## 6. SQL / DB queries (if any)

The Phasing module itself does no direct SQL; ObjectMgr loads its data:

| Statement / Source | Purpose | DB |
|---|---|---|
| `for (PhaseEntry const* phase : sPhaseStore)` | Seed `PhaseInfoStruct` per phaseId into `_phaseInfoById` | DB2 |
| `SELECT AreaId, PhaseId FROM phase_area` | Load `vector<PhaseAreaInfo>` per area; C++ `ConditionMgr::LoadConditions` later attaches `conditions WHERE SourceTypeOrReferenceId = 26 (CONDITION_SOURCE_TYPE_PHASE)` into `PhaseAreaInfo::Conditions` | world |
| `SELECT MapId, TerrainSwapMap FROM terrain_swap_defaults` | Default terrain swaps applied by `OnMapChange` to all players on that map (subject to conditions) | world |
| `SELECT TerrainSwapMap, UiMapPhaseId FROM terrain_worldmap` | UI-map phase IDs attached to a terrain swap (rendered as alt minimap) | world |
| `SELECT SourceTypeOrReferenceId, SourceGroup, SourceEntry, … FROM conditions WHERE SourceTypeOrReferenceId IN (25,26)` | Backing conditions for `CONDITION_SOURCE_TYPE_TERRAIN_SWAP` (25) and `CONDITION_SOURCE_TYPE_PHASE` (26) — owned by ConditionMgr, non-owning pointed-to by `PhaseRef::AreaConditions` | world |

DB2/DBC stores read by phasing code:

| Store | What it loads | Read by |
|---|---|---|
| `sPhaseStore` | Phase.db2 (`Id`, `Flags = Cosmetic / Personal`) | `PhasingHandler::GetPhaseFlags`, `IsPersonalPhase` |
| `sPhaseXPhaseGroupStore` (via `sDB2Manager._phasesByGroup`) | PhaseXPhaseGroup.db2 (groupId → vector<phaseId>) | `PhasingHandler::AddPhaseGroup`, `OnAreaChange` aura phase-group expansion |
| `sAreaTableStore` | AreaTable.db2 (`ID`, `ParentAreaID`) | `PhasingHandler::OnAreaChange` parent-area chain |
| `sMapStore` | Map.db2 — informs `TerrainSwapInfo` validation in ObjectMgr | indirectly via ObjectMgr |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_PHASE_SHIFT_CHANGE` (0x2578) | server → client | `PhasingHandler::SendToPlayer` (after every phase change, after teleport, after aura phase apply/remove) — payload: PackedGuid Client, u32 PhaseShiftFlags, i32 PhaseCount, PackedGuid PersonalGUID, [u16 Flags, u16 Id]\*, i32 VisibleMapIDsByteCount, u16\* VisibleMapIDs, i32 PreloadMapIDsByteCount, u16\* PreloadMapIDs (always empty in 3.4.3 path), i32 UiMapPhaseIDsByteCount, u16\* UiMapPhaseIDs |
| `SMSG_PARTY_MEMBER_FULL_STATE` / `PartyMemberPhaseStates` block | server → client | `PhasingHandler::FillPartyMemberPhase` — embedded in `WorldPackets::Party::PartyMemberFullState` so party UI can colour out-of-phase members |
| `SMSG_CONTROL_UPDATE` (indirect) | server → client | When `OnPhaseChange` triggers a visibility refresh, pets/charm controls may need re-sending |

No CMSG opcode is consumed by Phasing directly — phase state is purely server-driven (auras, area entry, scripts, GM commands).

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `crates/wow-packet/src/packets/misc.rs` | `file` | 1 | 2613 | `exists_active` | file exists |
| `crates/wow-phasing` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-packet/src/packets/misc.rs` — `PhaseShiftChange` now carries flags, personal GUID, phase refs, visible map IDs, preload map IDs and UI map phase IDs with the C++ byte-count prefixes.
- `crates/wow-world/src/phasing.rs` — C++ `PhasingHandler` pure slices plus the `SendToPlayer` packet build step from a real `PhaseShift`.

**What's implemented:**
- SMSG_PHASE_SHIFT_CHANGE wire serializer plus `wow-world::phasing::phase_shift_change_for_player_like_cpp`, mirroring C++ `PhasingHandler::SendToPlayer(Player const*, PhaseShift const&)`.
- `wow-constants::PhaseShiftFlags` / `PhaseFlags` mirror the C++ enum bit values.
- `wow-entities::PhaseShift` now carries C++-like phase refs, `PersonalGuid`, visible map id refs and UI map phase id refs with refcount semantics.
- `wow-entities::WorldObject` owns C++ `_phaseShift` / `_suppressedPhaseShift` equivalents and exposes mutable accessors for both.
- `PhaseShift::can_see` now mirrors the C++ pure predicate for `Unphased`, `AlwaysVisible`, `Inverse`, `InverseUnphased`, `NoCosmetic`, and personal phases.
- Session DB visibility for creature/gameobject spawns now applies `PhaseShift::can_see` before sending create blocks, using the same DB phase initialization helper as the represented/canonical spawn state.
- `SMSG_PARTY_MEMBER_FULL_STATE` now carries C++ `PartyMemberPhaseStates` from the member phase snapshot instead of an always-empty phase block.
- `wow-data::PhaseInfoStore` seeds C++ `_phaseInfoById` from `PhaseStore`, matching current `ObjectMgr::LoadPhases`.
- `AreaTable.db2` is loaded with hotfix overlays for `ParentAreaID`, and `phase_area` rows are loaded into C++-like `PhaseInfoStore` area buckets with parent `SubAreaExclusions`.
- `wow-world::phasing` implements the pure C++ `PhasingHandler::ResetPhaseShift`, `InheritPhaseShift`, `SetAlwaysVisible`, and `SetInversed` slices.
- `wow-world::phasing` implements the core C++ `PhasingHandler::AddPhase` / `RemovePhase` / phase-group / visible-map mutations plus `OnMapChange` and `OnAreaChange` rebuild passes with injected condition/aura inputs; `ControlledUnitVisitor` selection/visited rules are ported, but runtime world-object recursion and canonical ConditionMgr wiring are not wired yet.
- Terrain swap metadata loading exists for `terrain_worldmap` / `terrain_swap_defaults`, including C++ DB2+hotfix `UiMapXMapArt.PhaseID` validation for `IsUiMapPhase`.
- `Phase.db2` and `PhaseXPhaseGroup.db2` are loaded with hotfix overlays, exposing C++-like personal/cosmetic phase checks and `GetPhasesForGroup`.
- Creature spawn `terrainSwapMap` is validated against `Map.ParentMapID` and applied to the creature `PhaseShift` visible-map ids.
- `wow-world::phasing` ports C++ `PhasingHandler::FormatPhases` and the C++ `PrintToChat` argument formatter as `PhaseShiftChatSnapshot`; actual GM/chat command wiring remains pending until the Rust command layer exists.

**What's missing vs C++:**
- Full `PhasingHandler` façade — only reset/inherit/visibility-flag helpers and packet build are implemented.
- Canonical `ConditionContainer` storage/evaluation inside `PhaseAreaInfo`.
- `phase_area` condition attachment; the old plan reference to `phase_definitions` does not apply to this C++ branch because phase info is seeded from `sPhaseStore`.
- `CONDITION_SOURCE_TYPE_PHASE` and `CONDITION_SOURCE_TYPE_TERRAIN_SWAP` integration with the (also-missing) ConditionMgr.
- `OnAreaChange` / `OnMapChange` / `OnConditionChange` lifecycle hooks.
- Aura integration (`SPELL_AURA_PHASE`, `SPELL_AURA_PHASE_GROUP`).
- `ControlledUnitVisitor` recursive propagation through pets/vehicles/totems.
- `MultiPersonalPhaseTracker` per-map; private spawn lifecycle.
- Full integration of `PhaseShift::can_see` into every canonical map/grid notifier codepath; session DB spawn visibility and the map creature visibility query can now apply the predicate, and the session `ObjectAccessor` player object preserves represented player phase, but full player phase ownership is still represented until live `Player` phase lifecycle hooks land.
- GameObject/transport terrain swap application is still missing because Rust does not yet have the canonical C++ `GameObject::Create` / transport runtime path wired.
- Live refresh triggers for `PartyMemberPhaseStates` when a party member's phase changes are still tied to the remaining phase lifecycle hooks.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- The constant SMSG_PHASE_SHIFT_CHANGE payload uses `PhaseShiftFlags::Unphased = 0x08` always; once any aura-phase or area-phase is applied, that field has to flip off `Unphased` and on `NoCosmetic`/etc. as appropriate, otherwise client-side phase intersection drifts.
- Canonical `WorldObject::CanSeeOrDetect` parity is not complete until live player phase mutation hooks and every grid notifier/filter callsite use the same predicate; DB spawn send paths, the map creature visibility query, and the `ObjectAccessor` player snapshot now carry/apply phase, but non-DB/runtime object visibility still needs audit.
- The packet writes `PreloadMapIDs` count even though we never preload any swap maps — fine for now but must align with eventual terrain-swap implementation to avoid double-counting bytes.

**Tests existing:**
- 0 tests targeting phasing.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#PHASING.WBS.001** Cerrar la migracion auditada de `game/Phasing/PersonalPhaseTracker.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PersonalPhaseTracker.cpp`
  Rust target: `crates/wow-world`, `crates/wow-data`, `crates/wow-phasing`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PHASING.WBS.002** Cerrar la migracion auditada de `game/Phasing/PersonalPhaseTracker.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PersonalPhaseTracker.h`
  Rust target: `crates/wow-world`, `crates/wow-data`, `crates/wow-phasing`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PHASING.WBS.003** Cerrar la migracion auditada de `game/Phasing/PhaseShift.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PhaseShift.cpp`
  Rust target: `crates/wow-world`, `crates/wow-data`, `crates/wow-phasing`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PHASING.WBS.004** Cerrar la migracion auditada de `game/Phasing/PhaseShift.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PhaseShift.h`
  Rust target: `crates/wow-world`, `crates/wow-data`, `crates/wow-phasing`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PHASING.WBS.005** Partir y cerrar la migracion auditada de `game/Phasing/PhasingHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PhasingHandler.cpp`
  Rust target: `crates/wow-world`, `crates/wow-data`, `crates/wow-phasing`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 701 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PHASING.WBS.006** Cerrar la migracion auditada de `game/Phasing/PhasingHandler.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PhasingHandler.h`
  Rust target: `crates/wow-world`, `crates/wow-data`, `crates/wow-phasing`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

- [x] **#PHASE.1** Define `PhaseShiftFlags` (u32 bitflags) and `PhaseFlags` (u16 bitflags) in `crates/wow-constants/src/phasing.rs` matching the C++ enums byte-for-byte (L)
- [x] **#PHASE.2** Define `PhaseShift` struct with C++-like phase refs, visible-map refs and UI-map-phase refs (implemented in `wow-entities::PhaseShift`; personal GUID depth still belongs to later façade work) (M)
- [x] **#PHASE.3** Implement `PhaseShift::add_phase` / `remove_phase` / `add_visible_map_id` / `remove_visible_map_id` / `add_ui_map_phase_id` / `remove_ui_map_phase_id` with C++ refcount semantics for represented fields (M)
- [x] **#PHASE.4** Implement `PhaseShift::can_see` honouring `AlwaysVisible`, `Inverse`, `InverseUnphased`, `Unphased`, `NoCosmetic`; cover the `Personal` interaction (must also match `PersonalGuid`) (H)
- [x] **#PHASE.5** Add `phase_shift` and `suppressed_phase_shift` fields to the WorldObject base (actual Rust owner: `crates/wow-entities/src/world_object.rs`) and propagate through Player / Creature / GameObject / DynamicObject / AreaTrigger via their embedded `WorldObject`/`Unit` base (M)
- [x] **#PHASE.6** Define `PhaseInfoStruct`, `PhaseAreaInfo` (with `SubAreaExclusions: HashSet<u32>`, `Conditions: ConditionContainer`), `TerrainSwapInfo` (with `UiMapPhaseIDs: Vec<u32>`) data structs in `crates/wow-data/src/phasing.rs` (M; `PhaseInfoStruct` / `PhaseAreaInfo` / phase condition container exist; `TerrainSwapInfo` is in `terrain_swap.rs`)
- [x] **#PHASE.7** Implement current C++ phase-info seed (`sPhaseStore` → `_phaseInfoById`; old `phase_definitions` plan entry corrected because this branch no longer loads that table) (L)
- [ ] **#PHASE.8** Implement loader for `phase_area` (areaId → `Vec<PhaseAreaInfo>`) joined with `conditions WHERE SourceTypeOrReferenceId = 26` (M; partial: `phase_area` rows + AreaTable parent `SubAreaExclusions` are loaded, and C++ `ConditionMgr::addToPhases` attachment semantics are ported; global startup/reload wiring remains open)
- [x] **#PHASE.9** Implement loader for `terrain_swap_defaults` (mapId → `Vec<TerrainSwapInfo>`) (L)
- [x] **#PHASE.10** Implement loader for `terrain_worldmap` (terrainSwapMapId → `Vec<UiMapPhaseId>`) and merge into `TerrainSwapInfo::UiMapPhaseIDs`, validating via C++ DB2+hotfix `UiMapXMapArt.PhaseID` (L)
- [x] **#PHASE.11** Wire up `Phase.db2` (PhaseStore) loading in `crates/wow-data` and expose `is_personal_phase(phaseId)` / `is_cosmetic_phase(phaseId)` (M)
- [x] **#PHASE.12** Wire up `PhaseXPhaseGroup.db2` and expose `get_phases_for_group(groupId) -> Option<&Vec<u32>>` equivalent (M)
- [ ] **#PHASE.13** Implement `PhasingHandler::add_phase` / `remove_phase` static façade including `ControlledUnitVisitor` recursion through pets, summon slots, and vehicle passengers (H; partial: single-object C++ core mutation plus `ControlledUnitVisitor` selection/visited rules implemented in `wow-world::phasing`; live ObjectAccessor-style recursion still open)
- [ ] **#PHASE.14** Implement `PhasingHandler::add_phase_group` / `remove_phase_group` using `get_phases_for_group` (M; partial: single-object group mutation implemented in `wow-world::phasing`; controlled-unit traversal remains under #PHASE.13)
- [ ] **#PHASE.15** Implement `PhasingHandler::on_area_change` (parent-area walk via AreaTable.db2, condition evaluation per `PhaseAreaInfo`, suppression bookkeeping, aura re-application) (XL; partial: parent walk, sub-area exclusions, active/suppressed buckets and injected aura phase/group re-application implemented; canonical ConditionMgr/aura/runtime hook/control propagation remain open)
- [ ] **#PHASE.16** Implement `PhasingHandler::on_map_change` (terrain-swap evaluation against `CONDITION_SOURCE_TYPE_TERRAIN_SWAP`, populate `VisibleMapIds` + `UiMapPhaseIds`) (H; partial: terrain-swap rebuild pass implemented with injected predicate; canonical ConditionMgr/runtime hook remains open)
- [ ] **#PHASE.17** Implement `PhasingHandler::on_condition_change` (re-evaluate every existing phase's `AreaConditions`, move to/from `SuppressedPhaseShift`, preserve aura-driven phases, mirror the visible-map suppression loop) (H; partial: C++ core active/suppressed phase and visible-map mutation implemented with injected condition predicates; canonical ConditionMgr `AreaConditions` handles, runtime hook, Unit `OnPhaseChange`/controlled propagation and aura dispatcher integration remain open)
- [x] **#PHASE.18** Implement `PhasingHandler::reset_phase_shift`, `inherit_phase_shift`, `set_always_visible`, `set_inversed` (M)
- [x] **#PHASE.19** Replace the constant SMSG_PHASE_SHIFT_CHANGE in `crates/wow-packet/src/packets/misc.rs` with a real serializer/builder that takes `&PhaseShift` and writes flags + phase list + personal GUID + visible-map-id list + ui-map-phase-id list (M; runtime hooks still tracked in #PHASE.15/#PHASE.16/#PHASE.17/#PHASE.26)
- [ ] **#PHASE.20** Hook up `SPELL_AURA_PHASE` apply/remove and `SPELL_AURA_PHASE_GROUP` apply/remove in the (yet-to-be-fully-built) aura effect dispatcher to call `PhasingHandler::add_phase` / `add_phase_group` (M, depends on aura system)
- [x] **#PHASE.21** Implement `PersonalPhaseSpawns` (objects + grids + optional `Duration` countdown) and `PlayerPersonalPhasesTracker` (per-owner) in `crates/wow-world/src/phasing/personal.rs` (H)
- [ ] **#PHASE.22** Implement `MultiPersonalPhaseTracker` on each `Map` with `LoadGrid` / `UnloadGrid` / `RegisterTrackedObject` / `OnOwnerPhaseChanged` / `MarkAllPhasesForDeletion` / `Update(diff)` (XL — splittable per method; partial: C++ core multi-owner state machine plus `MapInstance` ownership/load/unload/register/update/remove-list hooks implemented; actual DB personal spawn loader and live grid lifecycle callsites remain open)
- [ ] **#PHASE.23** Wire `MultiPersonalPhaseTracker::update` into the per-map tick (1-minute countdown semantics from `PersonalPhaseSpawns::DELETE_TIME_DEFAULT`) (M; partial: `MapInstance::update_personal_phases_like_cpp` queues expired objects and `remove_personal_phase_objects_like_cpp` removes them; server tick callsite remains open)
- [ ] **#PHASE.24** Implement `InitDbPhaseShift` / `InitDbPersonalOwnership` / `InitDbVisibleMapId` so creature/gameobject DB rows that carry `phaseUseFlags` / `PhaseId` / `PhaseGroup` / `terrainSwapMap` columns produce correct phase state at spawn (M; partial: C++ helper semantics implemented; visible creature spawn/respawn path now selects and applies `phaseUseFlags`/`phaseid`/`phasegroup` plus `terrainSwapMap`; visible gameobject path now records represented `GameObject::GetPhaseShift()` from the same DB columns until canonical GO ownership lands; remaining spawn paths still need areatrigger/transport wiring and DB personal owner loader)
- [x] **#PHASE.25** Implement `GetTerrainMapId(phaseShift, mapId, terrain, x, y)` so collision / area lookups select the alt map when a swap is active at coordinates (`terrain_map_id_for_phase_shift_like_cpp` + `TerrainGridFilesLikeCpp` / `TerrainGridFileIndexLikeCpp` in `crates/wow-world/src/map_manager.rs`) (M)
- [ ] **#PHASE.26** Integrate `PhaseShift::can_see` into the Rust visibility / grid-notifier path (the C++ `WorldObject::CanSeeOrDetect` chain) so phased objects actually become invisible (H; partial: session creature/gameobject DB create paths now filter through `PhaseShift::can_see` against represented player phase, `MapManager::get_visible_creatures_in_phase` mirrors the C++ grid searcher phase filter for canonical creature queries, and the `ObjectAccessor` player snapshot preserves represented player phase; remaining live player phase lifecycle hooks, non-DB object paths, and all grid notifier callsites stay open)
- [x] **#PHASE.27** Implement `FillPartyMemberPhase` for `SMSG_PARTY_MEMBER_FULL_STATE` so the group UI knows when a member is out of phase (`PartyMemberPhaseStates` serializer + `party_member_phase_states_like_cpp` + player-registry snapshot feeding group full-state packets; live phase-change resend hooks remain tracked by #PHASE.15/#PHASE.17/#PHASE.26) (M)
- [x] **#PHASE.28** Add `PrintToChat` / `FormatPhases` admin/debug helpers (`format_phases_like_cpp` + `print_to_chat_snapshot_like_cpp`; concrete GM/chat command integration remains tracked outside the pure phasing port because Rust has no equivalent command layer yet) (L)
- [ ] **#PHASE.29** Wire `OnConditionChange` to be re-fired by relevant condition triggers (quest state change, aura apply, item add — all in the C++ `Condition::Meets` triggers) (M, depends on Conditions module)
- [x] **#PHASE.30** Documentation: cross-link `phasing.md` ↔ `conditions.md` ↔ `maps.md` (`MultiPersonalPhaseTracker` ownership and ConditionMgr phase/terrain-swap blockers) (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#PHASING.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 6 files / 1421 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PhasingHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PhaseShift.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PersonalPhaseTracker.cpp`. Rust target: `crates/wow-data`, `crates/wow-world`. | `cargo test -p wow-data && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#PHASING.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 6 files / 1421 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PhasingHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PhaseShift.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PersonalPhaseTracker.cpp`. Rust target: `crates/wow-data`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#PHASING.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 6 files / 1421 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PhasingHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PhaseShift.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PersonalPhaseTracker.cpp`. Rust target: `crates/wow-data`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#PHASING.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 6 files / 1421 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PhasingHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PhaseShift.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PersonalPhaseTracker.cpp`. Rust target: `crates/wow-data`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `PhaseShift::add_phase` then `remove_phase` returns container to empty AND clears `PersonalGuid` when last personal phase removed.
- [ ] Test: refcount semantics — adding the same phase 3× and removing 2× leaves it visible; removing the 3rd erases it.
- [ ] Test: `CanSee` matrix — Unphased+Unphased=true; Unphased+Phased=false; PhaseA+PhaseA=true; PhaseA+PhaseB=false; Inverse+PhaseA vs PhaseB = true.
- [ ] Test: `CanSee` with `AlwaysVisible` flag — sees everyone, seen by everyone regardless of other state.
- [ ] Test: `CanSee` with `NoCosmetic` — two objects sharing only a Cosmetic-flagged phase return false.
- [ ] Test: `Personal` phase intersection requires matching `PersonalGuid` on both sides (different owners → not visible to each other even with same phaseId).
- [ ] Test: `OnAreaChange` walks parent-area chain — placing a player in a sub-area whose parent has a phase entry applies the parent phase.
- [ ] Test: `OnAreaChange` honours `SubAreaExclusions` — a phase tied to area A but excluded for sub-area B does not apply when player stands in B.
- [x] Test: `OnConditionChange` moves phases and visible-map IDs between `Phases`/`VisibleMapIds` and `SuppressedPhaseShift` when injected C++ condition predicates flip, preserving refcounts for represented fields.
- [ ] Test: `SPELL_AURA_PHASE` apply adds phase to ALL controlled units (pet, vehicle passengers); remove pulls them all back.
- [ ] Test: SMSG_PHASE_SHIFT_CHANGE wire format — given a known PhaseShift fixture, the bytes match the legacy C++ binary output.
- [x] Test: `MultiPersonalPhaseTracker` despawns personal-phase spawns 60 s after the owner's phase set no longer contains them.
- [x] Test: `MultiPersonalPhaseTracker::LoadGrid` only loads spawns whose phase the owner currently holds.
- [ ] Test: `terrain_swap_defaults` — entering a map with a terrain swap whose conditions pass produces `VisibleMapIds = [swapMapId]` and the matching `UiMapPhaseIds`; failing conditions instead populate `SuppressedPhaseShift`.
- [ ] Test: `GetTerrainMapId` — when `VisibleMapIds` contains the alt and the alt has area data at (x,y), return alt; else return the source.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 6 files / 1421 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PhasingHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PhaseShift.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PersonalPhaseTracker.cpp` | `crates/wow-world/` (PhasingHandler logic, per-map MultiPersonalPhaseTracker), `crates/wow-data/` (load C++ `Phase.db2`-seeded phase info / `phase_area` / `terrain_swap_defaults` / `terrain_worldmap`), `crates/wow-packet/src/packets/misc.rs` (already has `PhaseShiftChange` stub). No dedicated `wow-phasing` crate yet. \| 🔧 broken — SMSG_PHASE_SHIFT_CHANGE is still incomplete and runtime hooks remain open: `PhasingHandler::OnAreaChange`, condition-driven phase suppression, personal phases, full terrain-swap evaluation, controlled-unit propagation, and SPELL_AURA_PHASE / SPELL_AURA_PHASE_GROUP integration. |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#PHASING.DIV.001` | `crates/wow-phasing` (`missing_declared_path`, 0 Rust lines) | 6 C++ files / 1421 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PhasingHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PhaseShift.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Phasing/PersonalPhaseTracker.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- `DEFAULT_PHASE = 169`: special value treated by the client as "no phase". Several scripted areas place objects in phase 169 explicitly — do not collapse it to `None`.
- The `Unphased` flag on `PhaseShiftFlags` is automatically maintained by `UpdateUnphasedFlag` based on `NonCosmeticReferences == 0`. Setting it manually is a bug source.
- `PhaseRef::AreaConditions` is a non-owning back-pointer into a `std::vector<Condition>` owned by `ConditionMgr`. In Rust this should be `Weak<Vec<Condition>>` or a stable `ConditionsRef` handle; `ConditionMgr` reload must invalidate them safely.
- The `ControlledUnitVisitor` deliberately skips passing through Player units inside a vehicle so that "Player inside nested vehicle" does not phase the root vehicle. Replicate this exclusion exactly.
- `OnConditionChange` is the only place that handles **both** directions (suppress and un-suppress). Naively re-running `OnAreaChange` is wrong because it re-applies area phases without preserving aura-driven and script-driven phases.
- The `PersonalPhaseSpawns::DELETE_TIME_DEFAULT = 1 min` is what gives "phase persistence" — leaving the phase, then re-entering within the minute, finds spawns still in place. Reducing this kills smooth questing UX.
- `Phase.db2` flags `Cosmetic` and `Personal` come from the **client** DB2; servers cannot invent them. Make sure the parser reads both.
- `terrain_worldmap` is *only* used to resolve `UiMapPhaseIDs`; it does NOT add visible map ids of its own.
- `IsDbPhaseShift` is a one-way flag set by `InitDbPhaseShift`: the C++ short-circuits some `OnAreaChange`/`OnConditionChange` paths for static DB-driven creatures so they don't re-evaluate dynamic conditions every tick.
- `PhasingHandler::SendToPlayer` overload without args sends the player's own phase shift — used after teleport. The 2-arg version is used in `FillPartyMemberPhase` to encode another player's shift.
- WoLK 3.4.3-specific: opcode `0x2578`, packet body sequence is exactly the one stubbed in `crates/wow-packet/src/packets/misc.rs`. Newer expansions add `PreloadMapIDs` block contents that 3.4.3 always leaves empty.
- Phasing bugs are *visibility* bugs — they don't crash, they manifest as "I can't see the quest NPC" or "two copies of the NPC stacked". Diagnose with the chat helper `PhasingHandler::PrintToChat`.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class PhaseShift` | `struct PhaseShift` (in `crates/wow-entities/src/world_object.rs`) | Plain struct; `Clone` for `InheritPhaseShift` semantics |
| `Trinity::Containers::FlatSet<PhaseRef>` | `BTreeMap<u32, PhaseRef>` | `PhaseRef` ordering is by `Id` only; iteration is ordered for deterministic packet output |
| `std::map<u32, VisibleMapIdRef>` | `BTreeMap<u32, VisibleMapIdRef>` | Ordered iteration matters for packet output |
| `EnumFlag<PhaseShiftFlags>` | `bitflags!` macro on a `u32` newtype | Use `.contains` / `.insert` / `.remove` |
| `ObjectGuid PersonalGuid` | `ObjectGuid` (in `wow-core`) | `ObjectGuid::EMPTY` when no personal owner |
| `class PhasingHandler` (all-static) | `pub mod phasing_handler { pub fn add_phase(...) ... }` | Free functions in a module — no need to keep the static-class shape |
| `ControlledUnitVisitor` | `struct ControlledUnitVisitor { visited: SmallVec<[*const WorldObject; 8]> }` | Use `&Unit` references; `SmallVec` of guids if pointer-stable references aren't ergonomic |
| `std::vector<Condition> const* AreaConditions` | `Option<Weak<Vec<Condition>>>` or `ConditionsRef` handle | Non-owning borrow into ConditionMgr-owned storage |
| `MultiPersonalPhaseTracker _multiPersonalPhaseTracker` (member of `Map`) | Field on `Map` struct: `personal_phases: MultiPersonalPhaseTracker` | Composition, not inheritance |
| `PersonalPhaseSpawns::DELETE_TIME_DEFAULT = 1min` | `const DELETE_TIME_DEFAULT: Duration = Duration::from_secs(60);` | — |
| `PhaseShift::CanSee` | `impl PhaseShift { pub fn can_see(&self, other: &Self) -> bool }` | Pure function, easy to unit-test exhaustively |
| `WorldPackets::Misc::PhaseShiftChange` | `crates/wow-packet/src/packets/misc.rs::PhaseShiftChange` + `wow-world::phasing::phase_shift_change_for_player_like_cpp` | Wire serializer and C++ `SendToPlayer` build step exist; lifecycle send hooks remain open |
| `SPELL_AURA_PHASE` aura handler | Aura effect arm in the spell engine | Calls `phasing_handler::add_phase` on apply, `remove_phase` on remove |
| `sObjectMgr->GetPhasesForArea` | `data::phasing::get_phases_for_area(area_id) -> Option<&[PhaseAreaInfo]>` | Loaded once at startup; immutable for the life of the world |
| `sDB2Manager.GetPhasesForGroup` | `db2::phase_group::phases_for_group(group_id) -> Option<&[u32]>` | Read from `PhaseXPhaseGroup.db2` |
| `sPhaseStore.LookupEntry(phaseId)` | `db2::phase::lookup(phase_id) -> Option<&PhaseEntry>` | `Phase.db2` |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

**Verdict: 🔧 confirmed broken — exact silent-default location identified.**

The hardcoded "always Unphased" stub claimed by §8 is real and lives at `crates/wow-packet/src/packets/misc.rs:1611–1638`. Verbatim:

```rust
impl ServerPacket for PhaseShiftChange {
    const OPCODE: ServerOpcodes = ServerOpcodes::PhaseShiftChange;
    fn write(&self, pkt: &mut crate::WorldPacket) {
        pkt.write_packed_guid(&self.player_guid);
        pkt.write_uint32(0x08); // PhaseShiftFlags::Unphased  ← HARDCODED
        pkt.write_int32(0);     // Phases.Count = 0          ← HARDCODED
        pkt.write_packed_guid(&ObjectGuid::EMPTY); // PersonalGUID = empty
        pkt.write_int32(0);     // VisibleMapIDs * 2
        pkt.write_int32(0);     // PreloadMapIDs * 2
        pkt.write_int32(0);     // UiMapPhaseIDs * 2
    }
}
```

The struct only carries a `player_guid`; there is no `PhaseShift` field. The single caller is `crates/wow-world/src/handlers/character.rs:4425` (`self.send_packet(&PhaseShiftChange::default_for(guid))`) inside the post-login init pipeline — emitted exactly once and never updated.

**Silent-default consequences confirmed:**

- Every `WorldObject::is_within_sight` path implicitly returns "phase-compatible" because no `PhaseShift::can_see` exists (zero hits for `PhaseShift` outside of `wow-packet`).
- Quest-driven phased duplicates (Borean Tundra D.E.H.T.A., Death Knight starting zone) will render every variant simultaneously once those NPCs land.
- `SMSG_PARTY_MEMBER_FULL_STATE` writes `PhaseShiftFlags = 0` (`packets/party.rs:281`) so party UI never marks anyone out-of-phase.
- `OnAreaChange` / `OnMapChange` / `OnConditionChange` are not called anywhere — the packet is fire-and-forget at login. Crossing into phased areas re-sends nothing.
- DB table `phase_area` is loaded, but its `ConditionContainer` attachment is still a placeholder until canonical ConditionMgr support is wired; terrain swap tables are loaded; phase info is seeded from `Phase.db2` like this C++ branch rather than from the legacy `phase_definitions` table.
- `Phase.db2` and `PhaseXPhaseGroup.db2` are parsed, but phase/group validation is not yet applied to creature/gameobject/transport spawn rows.

**Coupling to ConditionMgr:** §8's claim is correct that this can't be fixed properly without ConditionMgr (also ❌). #PHASE.15 / #PHASE.16 / #PHASE.17 explicitly take a `ConditionContainer`. Treat #PHASE.* as blocked-on #COND.1–#COND.20.

**Quick-fix scope:** §11 note about WoLK 3.4.3 packet body is correct — fixing #PHASE.19 alone (replace constant payload with `&PhaseShift` source) is M, but yields nothing visible until #PHASE.4 (`can_see`) and #PHASE.26 (visibility integration) land. Do not patch the stub in isolation.
