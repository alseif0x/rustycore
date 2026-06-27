# Migration: Entities / Corpse

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/`
> **Rust target crate(s):** `crates/wow-world/`, `crates/wow-packet/`, `crates/wow-database/`, `crates/wow-constants/`
> **Layer:** L4 (sub-modules under `entities.md`)
> **Status:** ❌ not started — no `Corpse` type in Rust; the *creature corpse decay* timer (`corpse_despawn_at` on `WorldCreature`) shares the name but is unrelated; player-corpse / bones / GHOST-aura / resurrection sickness flow does not exist
> **Audited vs C++:** ⚠️ partial (header + cpp body of `Corpse.cpp` audited)
> **Last updated:** 2026-05-01

---

## 1. Purpose

`Corpse` is the persisted player-death entity: when a `Player` dies, a `Corpse` of type `CORPSE_RESURRECTABLE_PVE` (or `_PVP`) is spawned at the death position, the player's ghost form is sent home to the nearest graveyard, and the corpse waits to be reclaimed (via spirit healer or run-back). After 3 days unreclaimed (or immediately after release without retrieve), the corpse converts to type `CORPSE_BONES`, which is a non-resurrectable husk that decays after 60 minutes. Corpse data — owner GUID, position, displayId, items list (so the corpse looks like the dead character), race/class/sex/flags, customizations, time-of-death — is persisted to the `corpse` table in the `character` DB so it survives crashes/restarts. Corpse also drives ghost-vision, the `Spirit of Redemption` ghost interactions, resurrection sickness application on retrieve, and forms the loot anchor for skinnable creatures (creature corpses are *not* `Corpse` entities; they're the dead `Creature` itself — `Corpse` is player-only).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Entities/Corpse/Corpse.cpp` | 309 | `prefix` |
| `game/Entities/Corpse/Corpse.h` | 145 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Entities/Corpse/Corpse.h` | 145 | `Corpse` final class, `CorpseType`/`CorpseFlags` enums, `CORPSE_RECLAIM_RADIUS = 39`, `MAX_CORPSE_TYPE = 3` |
| `src/server/game/Entities/Corpse/Corpse.cpp` | 309 | Construct/Create/Update/Save/Delete-from-DB, `LoadCorpseFromDB`, `IsExpired(time_t)`, `BuildValuesCreate`/`BuildValuesUpdate` for `SMSG_UPDATE_OBJECT` |

Adjacent files that drive corpse logic (not in the directory but co-conspirators):

| File | Purpose |
|---|---|
| `src/server/game/Entities/Player/Player.cpp` (`KillPlayer`, `BuildPlayerRepop`, `RepopAtGraveyard`, `ResurrectPlayer`, `BuildCorpse`, `RemovedInsignia`) | Player death/release/resurrect — creates and destroys Corpse entities |
| `src/server/game/Spells/SpellEffects.cpp` (`EffectResurrect`, `EffectResurrectNew`, `EffectSelfResurrect`) | Resurrect spells |
| `src/server/game/Handlers/CombatHandler.cpp` (`HandleReclaimCorpse`, `HandleResurrectResponse`) | `CMSG_RECLAIM_CORPSE`, `CMSG_RESURRECT_RESPONSE` |
| `src/server/game/Handlers/QueryHandler.cpp` (`HandleCorpseQuery`) | `CMSG_QUERY_CORPSE_LOCATION_FROM_CLIENT` → `SMSG_CORPSE_LOCATION` |
| `src/server/game/Maps/Map.cpp` (`ConvertCorpseToBones`, `RemoveOldCorpses`) | Periodic conversion + cleanup |
| `src/server/game/Spells/AuraEffects.cpp` (resurrection sickness aura 15007) | Applied on retrieve at `level >= 10` |
| `src/server/game/Entities/Object/Updates/UpdateFields.h` (`CorpseData`) | The corpse update-field struct (Owner, PartyGUID, GuildGUID, DisplayID, Items[19], RaceID, Class, Sex, Flags, DynamicFlags, FactionTemplate, Customizations, StateSpellVisualKitID) |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Corpse` | class final (WorldObject + GridObject) | The corpse entity |
| `CorpseType` | enum | CORPSE_BONES = 0 / CORPSE_RESURRECTABLE_PVE = 1 / CORPSE_RESURRECTABLE_PVP = 2 |
| `CorpseFlags` | enum (bitflag) | NONE / BONES / UNK1 / PVP / HIDE_HELM / HIDE_CLOAK / SKINNABLE / FFA_PVP |
| `CorpseDynFlags` | enum (in `SharedDefines`) | LOOTABLE (signals dead-creature corpse skinnable; reused field) |
| `UF::CorpseData` | UpdateField struct | Owner, PartyGUID, GuildGUID, DisplayID, Items[19], RaceID, Class, Sex, Flags, DynamicFlags, FactionTemplate, Customizations[], StateSpellVisualKitID |
| `UF::ChrCustomizationChoice` | UpdateField | OptionID + ChoiceID for corpse appearance match |
| `CORPSE_RECLAIM_RADIUS` | constant | 39 yards — client resurrection-dialog radius |

---

## 4. Critical public methods

| Symbol | Purpose | Calls into |
|---|---|---|
| `Corpse::Corpse(CorpseType type)` | Construct, set object type/typemask, mark stationary, `m_time = GameTime::GetGameTime()` | `WorldObject` ctor |
| `Corpse::Create(guidlow, map)` | Plain create with low GUID + map (used on DB load) | `Object::_Create<HighGuid::Corpse>` |
| `Corpse::Create(guidlow, owner)` | Spawn at owner's position, scale 1.0, set ownerGUID, compute `_cellCoord`, inherit phase | `PhasingHandler::InheritPhaseShift` |
| `Corpse::Update(uint32 diff)` | Per-tick: forwards to `WorldObject::Update`; updates `m_loot` if any | `WorldObject::Update` |
| `Corpse::SaveToDB()` | Persist to `corpse` + `corpse_phases` + `corpse_customizations` tables (transaction; deletes existing first) | `CHAR_INS_CORPSE`, `CHAR_INS_CORPSE_PHASES`, `CHAR_INS_CORPSE_CUSTOMIZATIONS` |
| `Corpse::DeleteFromDB(trans)` / `DeleteFromDB(ownerGuid, trans)` | Drop corpse + phases + customizations rows | `CHAR_DEL_CORPSE` (×3 tables) |
| `Corpse::LoadCorpseFromDB(guid, fields)` | Restore corpse from `SELECT posX,posY,posZ,orientation,mapId,displayId,itemCache,race,class,gender,flags,dynFlags,time,corpseType,instanceId,guid FROM corpse WHERE mapId=? AND instanceId=?` | `Object::_Create`, `SetItem`, `Relocate`, `SetFactionTemplate` |
| `Corpse::ResetGhostTime()` | Update `m_time` to "now" (used on resurrect-decline) | `GameTime::GetGameTime` |
| `Corpse::IsExpired(time_t t)` | True if owner's character cache is gone, OR (BONES && >60min old), OR (PVE/PVP && >3 days old) | `sCharacterCache->HasCharacterCacheEntry` |
| `Corpse::SetCustomizations(IteratorPair)` | Mirror player's chr-customizations onto the corpse (so it looks right) | `AddDynamicUpdateFieldValue` |
| `Corpse::GetType() / GetGhostTime() / GetCellCoord()` | Accessors | direct |
| `Corpse::GetCorpseDynamicFlags() / SetCorpseDynamicFlag / RemoveCorpseDynamicFlag / ReplaceAllCorpseDynamicFlags` | Dynamic-flags update-field | `UpdateField` mutators |
| `Corpse::SetItem(slot, item)` | Set one of 19 visible-item slots (so corpse renders armor) | `UpdateField` |
| `Corpse::SetOwnerGUID / SetPartyGUID / SetGuildGUID / SetDisplayId / SetRace / SetClass / SetSex / ReplaceAllFlags / SetFactionTemplate` | Update-field setters | direct |
| `Corpse::BuildValuesCreate(buf, target)` | Serialize for `SMSG_UPDATE_OBJECT` create | `m_objectData->WriteCreate`, `m_corpseData->WriteCreate` |
| `Corpse::BuildValuesUpdate(buf, target)` | Serialize delta updates | similar |
| `Corpse::AddToWorld()` / `RemoveFromWorld()` | Register/unregister in `Map::GetObjectsStore<Corpse>` | direct |

Adjacent (Player/Map) methods that drive the lifecycle:

| Symbol | Purpose |
|---|---|
| `Player::KillPlayer()` | Set ghost flag, `BuildCorpse`, schedule auto-release timer (6 min) |
| `Player::BuildCorpse(...)` | Construct `Corpse` with owner's display, items, customizations |
| `Player::RepopAtGraveyard()` | Teleport ghost to nearest graveyard, leave Corpse at death point |
| `Player::ResurrectPlayer(restorePct, applySickness)` | Revive at corpse, restore HP%, apply Resurrection Sickness if level ≥ 10 |
| `Player::SpawnCorpseBones()` | Convert resurrectable corpse → bones immediately (e.g. on logout while ghost) |
| `Map::ConvertCorpseToBones(ownerGuid, insignia)` | Periodic worker: resurrectable → bones after timer |
| `Map::RemoveOldCorpses()` | Periodic worker: bones older than 60min → deleted |

---

## 5. Module dependencies

**Depends on:**
- `WorldObject` (base): position, phasing, GUID
- `GameTime` (`GetGameTime` for `m_time`)
- `CharacterCache` (`HasCharacterCacheEntry` — corpse expires if owner deleted)
- `PhasingHandler` (`InheritPhaseShift`)
- `Map` / `MapManager` (`GetObjectsStore<Corpse>`, `ConvertCorpseToBones`, `RemoveOldCorpses` periodic ticks)
- `Loot` (creature corpses use this — but those are `Creature`, not `Corpse`; player corpses can be skinnable in some cases)
- DB2: `ChrRacesStore` (for default `FactionID` per race on load)

**Depended on by:**
- `Player` (`KillPlayer`, `BuildCorpse`, `RepopAtGraveyard`, `ResurrectPlayer`, `RemovedInsignia`)
- `Spell` (resurrection effects target the Corpse to find the player)
- `WorldSession` packet handlers: `HandleReclaimCorpse`, `HandleResurrectResponse`, `HandleCorpseQuery`
- `BattlegroundMgr` (PvP corpse insignia)
- `GraveyardManager` / `WorldSafeLocsStore` (DB2 — ghost-walk destination)
- `GhostMaster` interaction (`UNIT_FLAG_GHOST` aura 8326)

---

## 6. SQL / DB queries

| Statement / Source | Purpose | DB |
|---|---|---|
| `CHAR_INS_CORPSE` | Insert corpse row (`SaveToDB`) | character |
| `CHAR_INS_CORPSE_PHASES` | Insert per-phase rows | character |
| `CHAR_INS_CORPSE_CUSTOMIZATIONS` | Insert chr-customizations | character |
| `CHAR_DEL_CORPSE` | Delete corpse row | character |
| `CHAR_DEL_CORPSE_PHASES` | Delete phase rows | character |
| `CHAR_DEL_CORPSE_CUSTOMIZATIONS` | Delete customization rows | character |
| `SELECT posX,posY,posZ,orientation,mapId,displayId,itemCache,race,class,gender,flags,dynFlags,time,corpseType,instanceId,guid FROM corpse WHERE mapId=? AND instanceId=?` | Bulk corpse load on `Map` init | character |
| `SELECT ChrCustomizationOptionID,ChrCustomizationChoiceID FROM corpse_customizations WHERE OwnerGuid=?` | Customization load | character |
| `SELECT PhaseId FROM corpse_phases WHERE OwnerGuid=?` | Phase load | character |

DBC/DB2 stores read by Corpse:

| Store | What it loads | Read by |
|---|---|---|
| `ChrRacesStore` | ChrRaces.db2 | `LoadCorpseFromDB` reads `FactionID` for default `FactionTemplate` |
| `WorldSafeLocsStore` | WorldSafeLocs.db2 | (graveyard for ghost run-back, not Corpse-internal but driven by it) |

---

## 7. Wire-protocol packets

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_UPDATE_OBJECT` (CorpseData create-block) | S → C | `Corpse::BuildValuesCreate` |
| `SMSG_DESTROY_OBJECT` (corpse) | S → C | `RemoveFromWorld` |
| `CMSG_QUERY_CORPSE_LOCATION_FROM_CLIENT` | C → S | `WorldSession::HandleCorpseQuery` |
| `SMSG_CORPSE_LOCATION` | S → C | `Player::SendCorpseReclaimDelay`-adjacent |
| `SMSG_CORPSE_RECLAIM_DELAY` | S → C | `Player::SendCorpseReclaimDelay` |
| `CMSG_RECLAIM_CORPSE` | C → S | `WorldSession::HandleReclaimCorpse` → `Player::ResurrectPlayer` |
| `CMSG_RESURRECT_RESPONSE` | C → S | accept/decline a resurrect spell offer |
| `SMSG_RESURRECT_REQUEST` | S → C | offer from a healer |
| `SMSG_PRE_RESSURRECT` | S → C | (pre-revive UI cue) |
| `SMSG_DEATH_RELEASE_LOC` | S → C | tell client where ghost goes |
| `CMSG_REPOP_REQUEST` | C → S | "release spirit" button |
| `SMSG_DEATH_REWARD` (BG-only) | S → C | death reward in BG |
| `CMSG_QUERY_CORPSE_TRANSPORT` | C → S | corpse-on-transport query |
| `SMSG_CORPSE_TRANSPORT_QUERY` | S → C | response |
| `SMSG_SPIRIT_HEALER_CONFIRM` | S → C | spirit healer interaction |
| `CMSG_AREA_SPIRIT_HEALER_QUEUE` | C → S | BG spirit healer queue |
| `SMSG_AREA_SPIRIT_HEALER_TIME` | S → C | next mass-rez countdown |
| Resurrection sickness: applied as Aura 15007 via `SMSG_AURA_UPDATE` | S → C | `Player::ResurrectPlayer` |
| GHOST aura 8326 (`UNIT_FLAG_GHOST`): applied on death | S → C | `Player::KillPlayer` |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `crates/wow-constants/src/object.rs:24,75` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-core/src/guid.rs` | `file` | 1 | 790 | `exists_active` | file exists |
| `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `crates/wow-constants/src/spell.rs` | `file` | 1 | 569 | `exists_active` | file exists |
| `crates/wow-constants/src/unit.rs` | `file` | 1 | 599 | `exists_active` | file exists |
| `crates/wow-constants/src/object.rs` | `file` | 1 | 173 | `exists_active` | file exists |
| `crates/wow-constants/src/shared.rs` | `file` | 1 | 464 | `exists_active` | file exists |
| `crates/wow-constants/src/item.rs` | `file` | 1 | 1239 | `exists_active` | file exists |
| `crates/wow-packet/src/packets/update_stubs.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-world/src/handlers/loot.rs:172,199-218` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/archived/rustycore_ARCHIVED_20260312`:**
- `crates/wow-constants/src/object.rs:24,75` — `TypeMask::Corpse = 10`, `TypeId::Corpse = 14`
- `crates/wow-core/src/guid.rs` — `HighGuid::Corpse` variant defined (used in nothing yet)
- `crates/wow-constants/src/opcodes.rs` — opcodes enumerated (constants only, no handlers):
  - `QueryCorpseLocationFromClient = 0x3662`
  - `QueryCorpseTransport = 0x3663`
  - `ReclaimCorpse = 0x34db`
  - `CorpseLocation = 0x264f`
  - `CorpseReclaimDelay = 0x274a`
  - `CorpseTransportQuery = 0x2712`
  - `AreaTriggerNoCorpse = 0x2716`
- `crates/wow-constants/src/spell.rs:255` — `Mechanic::NoEdibleCorpses = 165` (ghoul mechanic)
- `crates/wow-constants/src/unit.rs:436` — `DynamicFlags::Corpse = 2` (creature corpse loot indicator)
- `crates/wow-constants/src/object.rs:136-140` — `SummonType::TimedOrCorpseDespawn = 2`, `CorpseDespawn = 5`, `CorpseTimedDespawn = 6` (for TempSummon)
- `crates/wow-constants/src/shared.rs:426` — `Mechanic::Corpse = 1` (also a mechanic constant)
- `crates/wow-constants/src/item.rs:355` — `SkillType::CorpseRecovery = 80` (legacy skill enum)
- `crates/wow-packet/src/packets/update_stubs.rs` — likely contains a `CorpseData` stub (per the AGENTS.md "stubs.rs.txt" catalogue note)
- `crates/wow-world/src/handlers/loot.rs:172,199-218` — uses the term "corpse" in **creature** corpse decay comments; `corpse_despawn_at` is a `WorldCreature` field, not a `Corpse` entity
- `crates/wow-world/src/session.rs:1985-2065` — `corpse_despawn_at` driven creature-corpse-decay loop (also unrelated to player Corpse)

**What's implemented:**
- TypeId/TypeMask/HighGuid constants
- 7 corpse-related opcodes enumerated
- *Creature corpse decay* (the mob-body-disappears timer) — but this is on `WorldCreature`, not a `Corpse` entity
- Nothing else

**What's missing vs C++:**
- **No `Corpse` runtime type at all.** No struct, no entity, no instance ever created.
- No `CorpseType` (BONES/PVE/PVP) — players can't die yet anyway, but even when they do, no corpse will spawn
- No `CorpseFlags` / `CorpseDynFlags` enums
- No `corpse` / `corpse_phases` / `corpse_customizations` table loaders or schemas
- No `CHAR_INS_CORPSE`, `CHAR_DEL_CORPSE`, `CHAR_INS_CORPSE_PHASES`, `CHAR_INS_CORPSE_CUSTOMIZATIONS` prepared statements
- No `Corpse::Create(owner)` — the lifecycle entry point
- No `IsExpired` (no corpse-bones conversion at 3 days, no bones cleanup at 60min)
- No `BuildValuesCreate` / `BuildValuesUpdate` for the wire format
- No `SetItem(slot, item)` to mirror player's worn items onto the corpse model
- No `SetCustomizations` (for character chr-customizations on the corpse)
- No `CMSG_RECLAIM_CORPSE` handler → `Player::ResurrectPlayer`
- No `CMSG_RESURRECT_RESPONSE` handler
- No `CMSG_QUERY_CORPSE_LOCATION_FROM_CLIENT` handler
- No `SMSG_CORPSE_LOCATION`, `SMSG_CORPSE_RECLAIM_DELAY`, `SMSG_RESURRECT_REQUEST`, `SMSG_DEATH_RELEASE_LOC`, `SMSG_PRE_RESSURRECT` builders
- No `CMSG_REPOP_REQUEST` (release spirit)
- No `Map::ConvertCorpseToBones` / `RemoveOldCorpses` background workers
- No GHOST aura (8326) application on death
- No `UNIT_FLAG_GHOST` UI bit
- No Resurrection Sickness aura (15007) application on retrieve
- No corpse-reclaim-radius (39yd) check
- No spirit-healer interaction (mass rez at SMSG_AREA_SPIRIT_HEALER_TIME)
- No `WorldSafeLocsStore` (graveyard table) — ghosts have nowhere to go
- No insignia mechanic (PvP: enemy can take corpse insignia → instant bones conversion)
- No corpse skinnability flag (CORPSE_FLAG_SKINNABLE) for engineering / leatherworking-from-player-corpse interactions
- No 6-minute auto-release timer
- No `SMSG_AURA_UPDATE` of GHOST aura on death
- No `_attic/` content for player corpse — the prior failed integration didn't even attempt it

**Suspicious / likely divergent:**
- `corpse_despawn_at` on `WorldCreature` (creature corpse decay) is named identically to player Corpse semantics but the two are *unrelated*. When `Corpse` lands, careful naming is needed (`creature.corpse_despawn_at` vs `Corpse::ghost_time`/`Corpse::expire_time`).
- `DynamicFlags::Corpse = 2` is set on dead Creatures (signals "lootable") and is also a CorpseDynFlags name used on player Corpses. The bit value differs by entity type — don't conflate.
- `SkillType::CorpseRecovery = 80` is a legacy skill ID for DK starting zone runback; not the same as bones conversion despite the name.
- `HighGuid::Corpse` is defined but no code path constructs a Corpse GUID — when added, ensure `HighGuid::Corpse(mapId, 0, lowGuid)` matches `ObjectGuid::Create<HighGuid::Corpse>(mapId, 0, lowGuid)`.

**Tests existing:** 0.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#ENTITIES_CORPSE.WBS.001** Cerrar la migracion auditada de `game/Entities/Corpse/Corpse.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/Corpse.cpp`
  Rust target: `crates/wow-world`, `crates/wow-packet`, `crates/wow-database`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_CORPSE.WBS.002** Cerrar la migracion auditada de `game/Entities/Corpse/Corpse.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/Corpse.h`
  Rust target: `crates/wow-world`, `crates/wow-packet`, `crates/wow-database`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#COR.1** Define `CorpseType` and `CorpseFlags` enums in `wow-constants` (L)
- [ ] **#COR.2** Define `Corpse` struct in `wow-world` (owner_guid, party_guid, guild_guid, display_id, items: [u32; 19], race, class, sex, flags, dynamic_flags, faction_template, customizations: Vec<ChrCustomizationChoice>, time, type_, cell_coord, position, map_id, instance_id, phase_shift) (M)
- [ ] **#COR.3** Add 6 prepared statements to `wow-database`: `CHAR_INS_CORPSE`, `CHAR_INS_CORPSE_PHASES`, `CHAR_INS_CORPSE_CUSTOMIZATIONS`, `CHAR_DEL_CORPSE`, `CHAR_DEL_CORPSE_PHASES`, `CHAR_DEL_CORPSE_CUSTOMIZATIONS` plus the 3 SELECTs for load (M)
- [ ] **#COR.4** Implement `Corpse::create_for_owner(player)` matching C++ `Create(guidlow, owner)` (L)
- [ ] **#COR.5** Implement `Corpse::save_to_db(tx)` matching C++ `SaveToDB` (delete-then-insert in same txn) (M)
- [ ] **#COR.6** Implement `Corpse::load_from_db(map_id, instance_id)` and bind into MapManager on Map init (M)
- [ ] **#COR.7** Implement `Corpse::is_expired(now)` with the BONES=60min / PVE+PVP=3day rule + CharacterCache check (L)
- [ ] **#COR.8** Implement Map worker `convert_corpses_to_bones` (resurrectable → bones after auto-release) (M)
- [ ] **#COR.9** Implement Map worker `remove_old_corpses` (bones older than 60min → delete) (L)
- [ ] **#COR.10** Implement `Corpse::set_item(slot, item_id)` for worn-equipment mirror (L)
- [ ] **#COR.11** Implement `Corpse::set_customizations(...)` honoring `ChrCustomizationChoice` (L)
- [ ] **#COR.12** Build `SMSG_UPDATE_OBJECT` create-block for Corpse (`CorpseData` write) (M)
- [ ] **#COR.13** Wire `Player::kill_player()` to spawn a Corpse + apply GHOST aura 8326 + 6-min auto-release timer (M, needs Player module)
- [ ] **#COR.14** Wire `Player::repop_at_graveyard()` to teleport to nearest `WorldSafeLocs` row (depends on a graveyard manager) (M)
- [ ] **#COR.15** Implement `CMSG_REPOP_REQUEST` handler → `repop_at_graveyard` (L)
- [ ] **#COR.16** Implement `CMSG_RECLAIM_CORPSE` handler → distance check (`CORPSE_RECLAIM_RADIUS = 39`yd) → `Player::resurrect_player` (M)
- [ ] **#COR.17** Implement `Player::resurrect_player(restore_pct, apply_sickness)` with Resurrection Sickness aura 15007 if level ≥ 10 (M)
- [ ] **#COR.18** Implement `CMSG_RESURRECT_RESPONSE` handler (accept/decline) + `SMSG_RESURRECT_REQUEST` builder (L)
- [ ] **#COR.19** Build `SMSG_CORPSE_LOCATION` (response to `CMSG_QUERY_CORPSE_LOCATION_FROM_CLIENT`) (L)
- [ ] **#COR.20** Build `SMSG_CORPSE_RECLAIM_DELAY` + cooldown logic (player-rez-by-spirit-healer ramp) (M)
- [ ] **#COR.21** Build `SMSG_DEATH_RELEASE_LOC` (graveyard for ghost) (L)
- [ ] **#COR.22** Build `SMSG_PRE_RESSURRECT` (pre-revive UI) (L)
- [ ] **#COR.23** Implement insignia mechanic: enemy clicks PvP corpse → forces bones conversion (S, BG-only) (L)
- [ ] **#COR.24** Implement spirit-healer mass-rez (`SMSG_AREA_SPIRIT_HEALER_TIME` 30s timer) for BGs (M)
- [ ] **#COR.25** Implement `CMSG_QUERY_CORPSE_TRANSPORT` / `SMSG_CORPSE_TRANSPORT_QUERY` for corpses on transports (depends on `entities-transport.md`) (L)
- [ ] **#COR.26** Rename `WorldCreature::corpse_despawn_at` to `creature_corpse_despawn_at` (or similar) before `Corpse` lands to avoid confusion (S, mechanical) (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#ENTITIES_CORPSE.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 454 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/Corpse.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/Corpse.h`. Rust target: `crates/wow-constants`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | `cargo test -p wow-constants && cargo test -p wow-database && cargo test -p wow-packet` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#ENTITIES_CORPSE.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 454 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/Corpse.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/Corpse.h`. Rust target: `crates/wow-constants`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#ENTITIES_CORPSE.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 454 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/Corpse.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/Corpse.h`. Rust target: `crates/wow-constants`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#ENTITIES_CORPSE.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 454 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/Corpse.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/Corpse.h`. Rust target: `crates/wow-constants`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `Corpse::create_for_owner(player)` produces Corpse with matching position, race, class, sex, displayId
- [ ] Test: `Corpse::set_item(slot, item)` round-trips through SaveToDB/LoadCorpseFromDB
- [ ] Test: `Corpse::is_expired` returns false for 59-min-old bones, true for 61-min-old bones
- [ ] Test: `Corpse::is_expired` returns false for 2-day-old PVE corpse, true for 4-day-old PVE corpse
- [ ] Test: `Corpse::is_expired` returns true if owner GUID isn't in CharacterCache (deleted character)
- [ ] Test: `convert_corpses_to_bones` flips type CORPSE_RESURRECTABLE_PVE → CORPSE_BONES after auto-release (6min timer or release click)
- [ ] Test: `remove_old_corpses` deletes bones rows older than 60min from DB and grid
- [ ] Test: Player dies → Corpse created with `CORPSE_FLAG_PVP` if killer was a player, `CORPSE_FLAG_FFA_PVP` if FFA, neither otherwise
- [ ] Test: `Player::resurrect_player` at level ≥ 10 applies Aura 15007 (Resurrection Sickness) for 10min; at level < 10, no aura
- [ ] Test: `CMSG_RECLAIM_CORPSE` outside 39-yard radius is rejected
- [ ] Test: `CMSG_RECLAIM_CORPSE` before reclaim-delay expires is rejected with `SMSG_CORPSE_RECLAIM_DELAY` countdown
- [ ] Test: GHOST aura 8326 is applied on `kill_player()` and removed on `resurrect_player()`
- [ ] Test: BG mass-rez at spirit healer triggers every 30s, resurrecting all dead-and-released players within range
- [ ] Test: PvP corpse insignia taken by enemy → bones conversion is immediate
- [ ] Test: `LoadCorpseFromDB` after server restart restores all unexpired corpses with correct `FactionTemplate` from `ChrRacesStore`
- [ ] Test: `SaveToDB` + `LoadCorpseFromDB` round-trips a Corpse with 19 items, 5 customizations, 3 phases

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 2 files / 454 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/Corpse.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/Corpse.h` | `crates/wow-world/`, `crates/wow-packet/`, `crates/wow-database/`, `crates/wow-constants/` \| ❌ not started — no `Corpse` type in Rust; the *creature corpse decay* timer (`corpse_despawn_at` on `WorldCreature`) shares the name but is unrelated; player-corpse / bones / GHOST-aura / resurrection sickness flow does not exist |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#ENTITIES_CORPSE.DIV.001` | `crates/wow-constants/src/object.rs:24,75` (`missing_declared_path`, 0 Rust lines) | 2 C++ files / 454 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/Corpse.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/Corpse.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#ENTITIES_CORPSE.DIV.002` | `crates/wow-packet/src/packets/update_stubs.rs` (`missing_declared_path`, 0 Rust lines) | 2 C++ files / 454 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/Corpse.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/Corpse.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#ENTITIES_CORPSE.DIV.003` | `crates/wow-world/src/handlers/loot.rs:172,199-218` (`missing_declared_path`, 0 Rust lines) | 2 C++ files / 454 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/Corpse.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Corpse/Corpse.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **Two distinct corpses share the name in WoLK 3.4.3:**
  1. **Player Corpse** (this doc): the persisted `Corpse` entity, type 0/1/2, lives in `corpse` table, drives ghost / reclaim / Resurrection Sickness.
  2. **Creature Corpse**: the dead `Creature` itself with `m_corpseRemoveTime` ticking — *not* a `Corpse` instance, no DB row, decays in seconds-to-minutes.
  Don't conflate. Rust's current `WorldCreature::corpse_despawn_at` is the second meaning. The `Corpse` Rust type when added is the first.
- `m_time` is set at construction (`GameTime::GetGameTime()`). For `CORPSE_RESURRECTABLE_*` it's the death timestamp; for `CORPSE_BONES` it's the conversion timestamp. Same field, two meanings — `IsExpired` switches on `m_type`.
- The 60-minute / 3-day expiry is hard-coded in `Corpse::IsExpired`, not template-driven.
- Resurrection Sickness duration is 10 minutes regardless of level (above 10).
- `CORPSE_RECLAIM_RADIUS` is **39** yards (matches client UI distance threshold for the "Resurrect at corpse" button). Don't use 40.
- Auto-release timer in C++ is hard-coded to 6 minutes (after that, ghost is forced to release; but the corpse stays).
- Reclaim-delay timer scales: 0s on first death, escalates to 60s, 90s, ... up to 30min (Trinity formula in `Player::CalculateCorpseReclaimDelay`). When releasing without retrieving, you eat a delay.
- `ChrRacesStore` lookup is what gives the corpse its faction; without it, the corpse renders as faction-neutral and is invisible to opposite-faction phase. Easy to skip.
- `corpse_phases` stores phaseIDs; PhasingHandler in WoLK 3.4 has fewer phase IDs than retail but they still matter (Wrathgate, BloodElf intro phasing).
- `corpse_customizations` is the **char-customization** mirror so the corpse's hairstyle/face matches the player; if you skip it the corpse will render with default appearance.
- The `Items` array on `CorpseData` is **19 slots**, not the 23 inventory slots — only the 19 visible-equipment positions (head, neck, shoulder, chest, ..., main-hand, off-hand, ranged, tabard).
- Corpses on **transports** require movement-info offset relative to transport, similar to passenger logic. See `entities-transport.md` and `CMSG_QUERY_CORPSE_TRANSPORT`.
- BG corpses have an "insignia" interaction: enemy clicks corpse → forces immediate bones conversion (and traditionally drops some honor/marks). This is `Player::RemovedInsignia`.
- "Bones" model is the GUID's display ID swapped to a generic skeleton; don't rely on the original race displayId once `m_type == CORPSE_BONES`.
- C# reference at `/home/server/woltk-server-core/Source/Game/Entities/Corpse/Corpse.cs` is the canonical fallback when 3.4 ambiguity hits.

---

## 12. C++ → Rust mapping

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Corpse final : WorldObject, GridObject<Corpse>` | `struct Corpse` (in `crates/wow-world/src/corpse.rs`) | composition, no inheritance |
| `enum CorpseType` | `#[repr(u8)] enum CorpseType { Bones, ResurrectablePve, ResurrectablePvp }` | direct |
| `enum CorpseFlags` | `bitflags::bitflags! struct CorpseFlags: u8` | NONE/BONES/UNK1/PVP/HIDE_HELM/HIDE_CLOAK/SKINNABLE/FFA_PVP |
| `enum CorpseDynFlags` | `bitflags::bitflags! struct CorpseDynFlags: u32` | distinct from `CorpseFlags` |
| `time_t m_time` | `i64` (unix epoch seconds) or `Instant` (preferred for monotonic) | Trinity uses `time_t`; for DB persistence Rust must use `i64` |
| `CellCoord _cellCoord` | `GridCoord` (already exists in `map_manager.rs:38`) | reuse |
| `unique_ptr<Loot> m_loot` | `Option<Loot>` | direct |
| `Player* lootRecipient` | `Option<ObjectGuid>` | resolve via PlayerRegistry |
| `UF::CorpseData m_corpseData` | inline fields on `Corpse` struct | C++ uses UpdateField for dirty tracking; in Rust use a dirty-bit set if needed |
| `SetItem(slot, item)` (19 slots) | `fn set_item(&mut self, slot: u8, item: u32)` with `[u32; 19]` field | direct |
| `SetCustomizations(IteratorPair)` | `fn set_customizations(&mut self, choices: Vec<ChrCustomizationChoice>)` | direct |
| `IsExpired(time_t)` | `fn is_expired(&self, now: i64, char_cache: &CharacterCache) -> bool` | needs `CharacterCache` reference |
| `Create(guidlow, owner)` | `fn create_for_owner(player: &Player) -> Corpse` | factory |
| `LoadCorpseFromDB(guid, fields)` | `fn from_db_row(row: &MySqlRow) -> Result<Corpse>` | sqlx pattern |
| `SaveToDB()` | `async fn save_to_db(&self, tx: &mut Transaction<MySql>) -> Result<()>` | async transaction |
| `DeleteFromDB(ownerGuid, trans)` | `async fn delete_from_db(owner_guid: ObjectGuid, tx: &mut Transaction<MySql>) -> Result<()>` | static method |
| `BuildValuesCreate(buf, target)` | `fn write_create(&self, buf: &mut WorldPacket, viewer: ObjectGuid)` | per-viewer flags |
| `Map::ConvertCorpseToBones` | `fn convert_corpse_to_bones(map: &mut MapState, owner_guid: ObjectGuid)` | map-level worker |
| `Map::RemoveOldCorpses` | `async fn remove_old_corpses(map: &mut MapState, now: i64)` | tick worker |
| `CHAR_INS_CORPSE` | `CharStatements::INS_CORPSE` enum variant | register in `wow-database` |
| `CORPSE_RECLAIM_RADIUS = 39` | `pub const CORPSE_RECLAIM_RADIUS: f32 = 39.0;` in `wow-constants` | direct |

---

## 13. §13 Audit (vs `/home/server/woltk-trinity-legacy/`)

| C++ symbol | Found in Rust | File | Verdict |
|---|---|---|---|
| `class Corpse` | no | — | ❌ missing |
| `enum CorpseType` | no | — | ❌ missing |
| `enum CorpseFlags` | no | — | ❌ missing |
| `enum CorpseDynFlags` | partial (one bit reused for creature corpses) | `crates/wow-constants/src/unit.rs:436` `DynamicFlags::Corpse = 2` | ⚠️ creature-only meaning |
| `UF::CorpseData` (UpdateField) | likely stubbed | `crates/wow-packet/src/packets/update_stubs.rs` | ⚠️ to verify, no real builder |
| `Corpse::Create(guidlow, owner)` | no | — | ❌ missing |
| `Corpse::Create(guidlow, map)` | no | — | ❌ missing |
| `Corpse::Update` | no | — | ❌ missing |
| `Corpse::SaveToDB` | no | — | ❌ missing |
| `Corpse::DeleteFromDB` | no | — | ❌ missing |
| `Corpse::LoadCorpseFromDB` | no | — | ❌ missing |
| `Corpse::IsExpired` | no | — | ❌ missing |
| `Corpse::ResetGhostTime` | no | — | ❌ missing |
| `Corpse::SetCustomizations` | no | — | ❌ missing |
| `Corpse::SetItem` | no | — | ❌ missing |
| `Corpse::BuildValuesCreate` / `BuildValuesUpdate` | no | — | ❌ missing |
| `Map::ConvertCorpseToBones` | no | — | ❌ missing |
| `Map::RemoveOldCorpses` | no | — | ❌ missing |
| `Player::KillPlayer` | no | — | ❌ missing |
| `Player::BuildCorpse` | no | — | ❌ missing |
| `Player::RepopAtGraveyard` | no | — | ❌ missing |
| `Player::ResurrectPlayer` | no | — | ❌ missing |
| `Player::SpawnCorpseBones` | no | — | ❌ missing |
| `Player::RemovedInsignia` (PvP) | no | — | ❌ missing |
| `CMSG_RECLAIM_CORPSE` handler | no (opcode constant only) | `crates/wow-constants/src/opcodes.rs:511` | ⚠️ enumerated, no handler |
| `CMSG_QUERY_CORPSE_LOCATION_FROM_CLIENT` handler | no (opcode constant only) | `crates/wow-constants/src/opcodes.rs:468` | ⚠️ enumerated, no handler |
| `CMSG_QUERY_CORPSE_TRANSPORT` handler | no (opcode constant only) | `crates/wow-constants/src/opcodes.rs:469` | ⚠️ enumerated, no handler |
| `CMSG_RESURRECT_RESPONSE` handler | no | — | ❌ missing |
| `CMSG_REPOP_REQUEST` handler | no | — | ❌ missing |
| `SMSG_CORPSE_LOCATION` builder | no (opcode constant only) | `crates/wow-constants/src/opcodes.rs:909` | ⚠️ enumerated, no builder |
| `SMSG_CORPSE_RECLAIM_DELAY` builder | no (opcode constant only) | `crates/wow-constants/src/opcodes.rs:910` | ⚠️ enumerated, no builder |
| `SMSG_CORPSE_TRANSPORT_QUERY` builder | no (opcode constant only) | `crates/wow-constants/src/opcodes.rs:911` | ⚠️ enumerated, no builder |
| `SMSG_RESURRECT_REQUEST` builder | no | — | ❌ missing |
| `SMSG_DEATH_RELEASE_LOC` builder | no | — | ❌ missing |
| `SMSG_PRE_RESSURRECT` builder | no | — | ❌ missing |
| `SMSG_AREA_SPIRIT_HEALER_TIME` builder | no | — | ❌ missing |
| GHOST aura 8326 application | no | — | ❌ missing |
| Resurrection Sickness aura 15007 | no | — | ❌ missing |
| `CHAR_INS_CORPSE` etc. (6 prepared statements) | no | `crates/wow-database/src/statements/character.rs` | ❌ missing |
| `corpse` / `corpse_phases` / `corpse_customizations` table loaders | no | — | ❌ missing |
| `WorldSafeLocsStore` (graveyards DB2) | no | — | ❌ missing |
| `ChrRacesStore` faction lookup | no | — | ❌ missing |
| `CORPSE_RECLAIM_RADIUS = 39` | no | — | ❌ missing |
| `MAX_CORPSE_TYPE = 3` | no | — | ❌ missing |
| `TypeMask::Corpse = 10` | yes | `crates/wow-constants/src/object.rs:24` | ✅ present |
| `TypeId::Corpse = 14` | yes | `crates/wow-constants/src/object.rs:75` | ✅ present |
| `HighGuid::Corpse` | yes | `crates/wow-core/src/guid.rs` | ✅ present (unused) |

**Verdict:** ❌ not started. Surface coverage ≈ 0% (only TypeId/TypeMask/HighGuid constants and 7 idle opcode constants). Players cannot die meaningfully — there is no Corpse to leave behind, no graveyard to walk to, no reclaim path, no Resurrection Sickness, no GHOST form. This blocks all death-related gameplay (PvE wipes, PvP, BGs, dungeons, SH gameplay, world PvP corpse-camping). Highest priority among the three entity-typed sub-docs because every other PvE gameplay loop terminates in death and demands corpse mechanics.

---

*Sub-doc of `entities.md`. Template version: 1.0 (2026-05-01).*
