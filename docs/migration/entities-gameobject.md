# Migration: Entities / GameObject

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/`
> **Rust target crate(s):** `crates/wow-world/`, `crates/wow-packet/`, `crates/wow-data/`, `crates/wow-database/`
> **Layer:** L4 (sub-modules under `entities.md`)
> **Status:** ❌ not started — no `GameObject` type exists in Rust; only wire-format stub `GameObjectCreateData` for spawn broadcast and an `SEL_GAMEOBJECTS_IN_RANGE` query
> **Audited vs C++:** ⚠️ partial (header-level audit, type-data union sampled)
> **Last updated:** 2026-05-01

---

## 1. Purpose

`GameObject` is the catch-all entity for static or semi-static interactable world objects: chests, herb/mineral nodes, doors, mailboxes, fishing bobbers, fishing holes, fishing nodes, traps, spell-focus altars, portals/teleporters, ritual circles, capture flags/banners, control zones, destructible siege buildings, runes, gathering goobers, mana gems, signposts, chairs, transports (MO + static elevator), text plaques, summoning circles, NewFlag (BG flags), and a long list more — 50+ `GameObjectType` variants. It inherits from `WorldObject` plus `GridObject<GameObject>` plus `MapObject`, owns a polymorphic `m_goTypeImpl: unique_ptr<GameObjectTypeBase>` for per-type behavior plus a `GameObjectValue` union for per-type runtime state, and drives a 4-state lifecycle (`GO_NOT_READY` → `GO_READY` → `GO_ACTIVATED` → `GO_JUST_DEACTIVATED` → `GO_READY`).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Entities/GameObject/GameObject.cpp` | 4488 | `prefix` |
| `game/Entities/GameObject/GameObject.h` | 516 | `prefix` |
| `game/Entities/GameObject/GameObjectData.h` | 1412 | `prefix` |
| `game/Entities/GameObject/QuaternionData.h` | 41 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Entities/GameObject/GameObject.h` | 516 | `GameObject` class, `GameObjectTypeBase` virtual, `GameObjectValue` union, `LootState` enum, `FlagState` enum, type-specific custom commands (SetTransportAutoCycleBetweenStopFrames, SetNewFlagState, SetControlZoneValue) |
| `src/server/game/Entities/GameObject/GameObject.cpp` | 4488 | All type-specific behaviors (door open/close, chest loot, trap pulse, fishing bobber, ritual, capture point, destructible building, transport rotation, NewFlag), `Use(Unit*)` 50-case switch, `Update`, `Respawn`, save/delete, model/collision |
| `src/server/game/Entities/GameObject/GameObjectData.h` | 1412 | `GameObjectTemplate` (huge tagged union over 50+ type structs: door, button, questgiver, chest, binder, generic, trap, chair, spellFocus, text, goober, transport, areaDamage, camera, mapObject, moTransport, duelArbiter, fishingNode, summoningRitual, mailbox, doNotUse_GuardPost, guildBank, spellCaster, meetingStone, flagStand, fishingHole, flagDrop, miniGame, lotteryKiosk, capturePoint, AuraGenerator, dungeonDifficulty, barberChair, destructibleBuilding, guildBankFiller, trapDoor, newFlag, NewFlagDrop, GarrisonBuilding, GarrisonPlot, ClientCreature, ClientItem, capturePointConverted, phaseableMO, gatheringNode, ItemForge, UILink, controlZone, ...), helpers: `GetLockId`, `GetLootId`, `GetGossipMenuId`, `GetCharges`, `GetAutoCloseTime`, `GetServerOnly`, `GetEntranceMap`, etc.; `GameObjectTemplateAddon`, `GameObjectAddon`, `GameObjectData`, `GameObjectActions` enum (45 actions) |
| `src/server/game/Entities/GameObject/QuaternionData.h` | ~50 | `QuaternionData { x, y, z, w }` plus `fromEulerAnglesZYX` |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `GameObject` | class (WorldObject + GridObject + MapObject) | The entity itself |
| `GameObjectTypeBase` | virtual class | Per-type behavior plug-in (`Update`, `OnStateChanged`, `OnRelocated`, `IsNeverVisibleFor`, `ActivateObject`) |
| `GameObjectType::SetTransportAutoCycleBetweenStopFrames` | command | Toggle MO-transport stop-frame cycling |
| `GameObjectType::SetNewFlagState` | command | BG flag state push (InBase/Taken/Dropped/Respawning) |
| `GameObjectType::SetControlZoneValue` | command | Wintergrasp-style control zone update |
| `FlagState` | enum class (uint8) | InBase / Taken / Dropped / Respawning |
| `LootState` | enum | GO_NOT_READY / GO_READY / GO_ACTIVATED / GO_JUST_DEACTIVATED |
| `GameObjectValue` | C union | Per-type runtime: FishingHole.MaxOpens, ControlZone.OPvPObj*, Building.Health/MaxHealth, CapturePoint.LastTeamCapture/State/AssaultTimer |
| `GameObjectTemplate` | struct | `gameobject_template` row + tagged-union of 50+ type structs |
| `GameObjectTemplateAddon` | struct (extends `GameObjectOverride`) | Mingold/Maxgold + 5×ArtKits + WorldEffectID + AIAnimKitID |
| `GameObjectAddon` | struct | `gameobject_addon`: `ParentRotation`, invisibility type/value, world effect, AI anim kit |
| `GameObjectData` | struct (extends `SpawnData`) | `gameobject` spawn row: rotation quaternion, animProgress, goState, artKit |
| `GameObjectLocale` | struct | Localized name/castBarCaption/unk1 |
| `QuaternionData` | struct | (x, y, z, w) plus packed-rotation conversion |
| `GameobjectTypes` | enum | The 50+ GAMEOBJECT_TYPE_* tag values |
| `GOState` | enum | GO_STATE_ACTIVE / GO_STATE_READY / GO_STATE_DESTROYED / GO_STATE_TRANSPORT_* |
| `GameObjectFlags` | enum (bitflag) | IN_USE / LOCKED / INTERACT_COND / TRANSPORT / NOT_SELECTABLE / NODESPAWN / DAMAGED / DESTROYED / MAP_OBJECT |
| `GameObjectActions` | enum class (uint32) | 45 actions: AnimateCustom0-3, Disturb, Lock/Unlock, Open/Close, Toggle, Destroy, Rebuild, Despawn, MakeInert/Active, ArtKit0-4, GoTo*Floor, PlayAnimKit, PlayOneShotAnimKit, StopAnimKit, PlaySpellVisual, SetTapList, SetTappedToChallengePlayers, etc. |
| `GameObjectDestructibleState` | enum | INTACT / DAMAGED / DESTROYED |

---

## 4. Critical public methods

| Symbol | Purpose | Calls into |
|---|---|---|
| `GameObject::Create(entry, map, pos, rotation, animProgress, goState, artKit, dynamic, spawnid)` | Construct from template + spawn data | `LoadGameObjectTemplate`, sets `m_localRotation`, `UpdatePackedRotation` |
| `GameObject::CreateGameObject(entry, map, pos, rotation, animProgress, goState, artKit)` | Static factory (transient) | `Create` |
| `GameObject::CreateGameObjectFromDB(spawnId, map, addToMap)` | Spawn from `gameobject` row | `LoadFromDB` |
| `GameObject::Update(uint32 p_time)` | Per-tick: state machine, restock, trap pulse, fishing bobber, transport rotate, capture progress | dispatches to `m_goTypeImpl->Update` and per-type code |
| `GameObject::Use(Unit* user)` | The big switch — chest open, door toggle, chair sit, mailbox, ritual contribute, trap activate, fishing cast, button press, portal, ... | `SendLoot`, `UseDoorOrButton`, `SetGoState`, `CastSpell`, `SummonGameObject`, taxi/binder gossip |
| `GameObject::SetGoState(GOState)` | State setter; broadcasts UPDATE_OBJECT mask change; calls `m_goTypeImpl->OnStateChanged` | `EnableCollision` |
| `GameObject::UseDoorOrButton(time_to_restore, alternative, user)` | Open door/button + schedule auto-close | `SwitchDoorOrButton` |
| `GameObject::ResetDoorOrButton()` | Auto-close after timer | — |
| `GameObject::ActivateObject(action, param, spellCaster, spellId, effectIndex)` | One of 45 `GameObjectActions` (script/spell trigger) | per-action switch |
| `GameObject::SetLootState(state, unit)` | Drive lifecycle (READY → ACTIVATED → JUST_DEACTIVATED → READY) | scheduling |
| `GameObject::Respawn()` | Bring back from despawn | `m_respawnTime`, `Refresh` |
| `GameObject::DespawnOrUnsummon(delay, forceRespawn)` | Schedule despawn | — |
| `GameObject::Refresh()` | Re-add to world after temp-despawn | — |
| `GameObject::Delete()` | Final removal | `RemoveFromOwner`, `RemoveFromWorld` |
| `GameObject::SetParentRotation(QuaternionData)` | Path/transport rotation | `SetLocalRotation` |
| `GameObject::SetLocalRotationAngles(z, y, x)` | Convenience setter | `UpdatePackedRotation` |
| `GameObject::UpdatePackedRotation()` | Pack quaternion into 64-bit `m_packedRotation` (X:22 / Y:21 / Z:21) | wire format |
| `GameObject::ModifyHealth(change, attackerOrHealer, spellId)` | Destructible-building HP | `SetDestructibleState` |
| `GameObject::SetDestructibleState(state, attackerOrHealer, setHealth)` | INTACT/DAMAGED/DESTROYED transition | building scripts |
| `GameObject::AssaultCapturePoint(player)` / `UpdateCapturePoint()` / `CanInteractWithCapturePoint(target)` | BG capture-point flow | OPvP scripts |
| `GameObject::ClearLoot` / `IsFullyLooted` / `OnLootRelease` / `IsLootAllowedFor` | Loot housekeeping | `Loot` |
| `GameObject::AddUniqueUse(player)` / `AddUse` | Track usage (ritual contributors, gathering quotas) | — |
| `GameObject::TriggeringLinkedGameObject(trapEntry, target)` | Door → linked-trap | `SummonGameObject` |
| `GameObject::SaveToDB(mapid, spawnDifficulties)` | Persist spawn (GM tools) | `WORLD_INS_GAMEOBJECT` |
| `GameObject::DeleteFromDB(spawnId)` | Remove spawn + addon + spawngroup + linked-respawn + game-event | 7× `WORLD_DEL_*` |
| `GameObject::SetFlagState(state, player)` | NewFlag BG flag state | `SetNewFlagState` command |
| `GameObject::GetFishLoot(lootOwner)` / `GetFishLootJunk` | Fishing pool resolution | `LootStore` |
| `GameObject::LookupFishingHoleAround(range)` | Fishing-hole bobber-snap | `Map::IsInRange` |
| `GameObject::IsAtInteractDistance(player, spellInfo)` | Range gate for `Use` | uses `InteractRadiusOverride` from template |
| `GameObject::MeetsInteractCondition(user)` | `ConditionMgr` gate | `ConditionMgr::IsObjectMeetingNotGroupedConditions` |
| `GameObject::SetGoStateFor(state, viewer)` | Per-player state override | `m_perPlayerState` |
| `GameObject::SetSpellVisualId(visualId, activatorGuid)` | One-shot spell visual on the GO | broadcast |
| `GameObject::EnableCollision(enable)` | Toggle GO collision in VMap | — |

---

## 5. Module dependencies

**Depends on:**
- `WorldObject` (base): position, phasing, visibility
- `ObjectMgr`: `GameObjectTemplate`, `GameObjectTemplateAddon`, `GameObjectAddon`, `GameObjectData`, `GameObjectLocale`, `GameObjectOverride`, fishing pools
- `Map` / `MapManager`: `AddToMap`, `RemoveFromMap`, grid tracking
- `LootMgr` (`gameobject_loot_template`, `fishing_loot_template`)
- `ConditionMgr` (template `conditionID1` per type)
- `GameObjectAI` / `ScriptMgr` (per-script GO behavior)
- `VMapManager` / `MMapManager` (collision + pathing model file)
- DB2: `GameObjectsStore` (server-side overrides), `GameObjectDisplayInfoStore`, `LockStore` (chest/door locks), `TransportAnimationStore` (for MO transports)
- `OPvPCapturePoint` (control zone)
- `Battleground` (NewFlag, capture flags)
- `BattlegroundMgr`, `OutdoorPvPMgr`
- `ScriptMgr::OnGameObjectCreate` / `OnGameObjectUpdate` / `OnGameObjectStateChanged`

**Depended on by:**
- `WorldSession::HandleGameObjectUseOpcode`, `HandleGameObjectQueryOpcode`, `HandleGameObjectReportUse`
- `Spell::EffectGameObjectAction`, `Spell::EffectSummonGameObject`, `Spell::EffectOpenLock` (chest/door)
- `Player`: fishing, mailbox check (`Player::GetGameObjectIfCanInteractWith`), banker, taxi nodes, ritual contribute, mining/herbing skill (uses `LockType`)
- `Quest` (objective: gather GO, click GO, defend GO)
- `Battleground` (flags, doors, banners), `OutdoorPvP`, `Wintergrasp`/`StrandOfTheAncients` scripts
- `Transport` (subclass for MO transports — see `entities-transport.md`)

---

## 6. SQL / DB queries

| Statement / Source | Purpose | DB |
|---|---|---|
| `WORLD_DEL_GAMEOBJECT` | Delete spawn row | world |
| `WORLD_INS_GAMEOBJECT` | Insert spawn (`SaveToDB`) | world |
| `WORLD_DEL_GAMEOBJECT_ADDON` | Drop addon on delete | world |
| `WORLD_DEL_SPAWNGROUP_MEMBER` | Remove from spawn group | world |
| `WORLD_DEL_EVENT_GAMEOBJECT` | Drop game-event tie | world |
| `WORLD_DEL_LINKED_RESPAWN` (×2 directions) | Drop linked-respawn refs | world |
| `WORLD_DEL_LINKED_RESPAWN_MASTER` (×2) | Drop linked-respawn master refs | world |
| Bulk loads (read at startup by `ObjectMgr`): `gameobject_template`, `gameobject_template_addon`, `gameobject_addon`, `gameobject_override`, `gameobject` (spawns), `gameobject_questender`, `gameobject_queststarter`, `gameobject_loot_template`, `fishing_loot_template`, `points_of_interest`, `transports` (table) | World-DB seed | world |

DBC/DB2 stores read by GameObject:

| Store | What it loads | Read by |
|---|---|---|
| `LockStore` | Lock.db2 | chest/door open requirements |
| `GameObjectDisplayInfoStore` | GameObjectDisplayInfo.db2 | model bounding box, collision |
| `GameObjectsStore` | GameObjects.db2 (server-side override DB2) | template overrides |
| `TransportAnimationStore` | TransportAnimation.db2 | MO transport interp |
| `TransportRotationStore` | TransportRotation.db2 | MO transport rot interp |
| `TaxiPathNodeStore` | TaxiPathNode.db2 | MO transport path |
| `SpellFocusObjectStore` | SpellFocusObject.db2 | spell-focus altars (cooking fire, anvil, ...) |

---

## 7. Wire-protocol packets

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_QUERY_GAMEOBJECT_RESPONSE` | S → C | `GameObjectTemplate::BuildQueryData` |
| `CMSG_QUERY_GAMEOBJECT` | C → S | `WorldSession::HandleGameObjectQueryOpcode` |
| `CMSG_GAME_OBJ_USE` | C → S | `WorldSession::HandleGameObjectUseOpcode` |
| `CMSG_GAME_OBJ_REPORT_USE` | C → S | quest-use ping |
| `SMSG_GAMEOBJECT_DESPAWN` | S → C | `GameObject::SendGameObjectDespawn` |
| `SMSG_GAMEOBJECT_CUSTOM_ANIM` | S → C | `GameObject::SendCustomAnim` |
| `SMSG_GAME_OBJECT_ACTIVATE_ANIM_KIT` | S → C | `SetAnimKitId` |
| `SMSG_GAME_OBJECT_PLAY_SPELL_VISUAL` | S → C | `SetSpellVisualId` |
| `SMSG_GAME_OBJECT_RESET_STATE` | S → C | (some scripts) |
| `SMSG_DESTRUCTIBLE_BUILDING_DAMAGE` | S → C | `ModifyHealth` on type 33 |
| `SMSG_FISH_NOT_HOOKED` / `SMSG_FISH_ESCAPED` | S → C | bobber outcome |
| `SMSG_LOOT_RESPONSE` | S → C | chest/herb/skin loot UI |
| `SMSG_PAGE_TEXT_QUERY_RESPONSE` | S → C | text-plaque pages (type 9) |
| `SMSG_GOSSIP_MESSAGE` | S → C | questgiver / spirit-healer GOs |
| `SMSG_BATTLEGROUND_PLAYER_POSITIONS` (flag carriers) | S → C | NewFlag state |
| `SMSG_CAPTURE_POINT_UPDATE` | S → C | OPvP capture-point UI |
| `SMSG_PLAY_OBJECT_SOUND` | S → C | mailbox click, etc. |
| `SMSG_TRANSPORT_*` (see `entities-transport.md`) | both | MO-transport path/passenger |
| `SMSG_UPDATE_OBJECT` (CreateObject for GO) | S → C | spawn broadcast — currently the only path Rust covers |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-packet/src/packets/update.rs` | `file` | 1 | 3072 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/character.rs:2109,2454` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-world/src/session.rs:264,444` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-world/src/session.rs:996,1196` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-database/src/statements/world.rs:79,253` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `crates/wow-constants/src/object.rs:22,72` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-core/src/guid.rs` | `file` | 1 | 790 | `exists_active` | file exists |
| `crates/wow-packet/src/packets/query.rs` | `file` | 1 | 616 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/archived/rustycore_ARCHIVED_20260312`:**
- `crates/wow-packet/src/packets/update.rs` — `GameObjectCreateData` (lines 1271-1395). Has: `guid, entry, display_id, go_type: u8, position, rotation: [f32; 4], anim_progress, state: i8, faction_template, scale`. Implements `write_values_create` (matches client `GameObjectFieldData.WriteCreate`) and `packed_rotation()` (X:22/Y:21/Z:21 layout recorded by the old note; canonical validation not established by that comment).
- `crates/wow-world/src/handlers/character.rs:2109,2454` — `send_nearby_gameobjects(map_id, position, zone_id)`. Loads from `SEL_GAMEOBJECTS_IN_RANGE`, sets `HighGuid::GameObject`, builds a `GameObjectCreateData`, pushes a `create_gameobject_block`. **No** `GameObject` runtime entity, no `Use`, no state machine, no template caching.
- `crates/wow-world/src/session.rs:264,444` — `WorldSession.visible_gameobjects: HashSet<ObjectGuid>` for visibility window tracking.
- `crates/wow-world/src/session.rs:996,1196` — `send_nearby_gameobjects` invocation; `ClientOpcodes::QueryGameObject` dispatch + decode of `QueryGameObject` request (response builder is incomplete).
- `crates/wow-database/src/statements/world.rs:79,253` — `SEL_GAMEOBJECTS_IN_RANGE` prepared statement registered.
- `crates/wow-constants/src/opcodes.rs` — opcodes: `QueryGameObject`, `GameObjectActivateAnimKit`, plus the create/destroy via the generic `UpdateObject` pipeline.
- `crates/wow-constants/src/object.rs:22,72` — `TypeMask::GameObject = 8`, `TypeId::GameObject = 11`.
- `crates/wow-core/src/guid.rs` — `HighGuid::GameObject` variant present.
- `crates/wow-packet/src/packets/query.rs` — `QueryGameObject` request packet decoder.

**What's implemented:**
- Spawn broadcast: nearby `gameobject` rows are read, packed-rotation computed, a CREATE block goes out. Client renders the model, the old packed-rotation parity claim relied on a code comment and is not accepted evidence.
- Visibility window (`visible_gameobjects`) so the same GO doesn't get re-broadcast.
- `QueryGameObject` opcode is wired (request decoded; response not built).

**What's missing vs C++:**
- **No `GameObject` runtime entity at all.** No `struct GameObject` in `wow-world`. There is *no* server-side state for any spawned game object beyond "the client knows it exists at position P".
- No `GameObjectTemplate` POD, no template cache, no per-type union/struct.
- No `GameObjectTypeBase` virtual / no per-type behavior plug-ins.
- No `LootState` / `GOState` lifecycle. State is whatever was in the spawn row, frozen.
- 0 of 50+ `GameobjectTypes` handled. **Chests can't be opened, doors can't be toggled, mailboxes don't respond, fishing bobbers don't trigger loot, traps don't pulse, capture points don't tick, destructible buildings don't take damage, BG flags don't track carriers.**
- `CMSG_GAME_OBJ_USE` is not handled (probably falls through to a default branch).
- `QueryGameObject` response is not built. Clients will see `unknown gameobject <id>` in tooltip.
- No `GameObjectAddon` / `GameObjectTemplateAddon` / `GameObjectOverride` loading.
- No `gameobject_loot_template` / `fishing_loot_template` consumption.
- No 45-action `GameObjectActions` dispatch (no spell-effect can manipulate a GO).
- No `Lock.db2` consumption (chest/door key checks).
- No `SaveToDB` / `DeleteFromDB` (GM tools can't add/remove GOs).
- No `m_perPlayerState` (per-viewer GO state — phased buttons/doors).
- No fishing-hole snap, no fishing-bobber-ready timer.
- No transport functionality (separate doc: `entities-transport.md`).
- No `Refresh` / `Respawn` / `DespawnOrUnsummon` cycle.
- No `m_loot` / `m_personalLoot` / `m_tapList` / `IsLootAllowedFor`.
- No `m_SkillupList` (gathering nodes that grant skill-up only once per player).
- No collision toggle (`EnableCollision`).
- No `SetGoStateFor` per-viewer state override.
- No `MeetsInteractCondition`.

**Suspicious / likely divergent:**
- `GameObjectCreateData` writes `ParentRotation` as identity (0,0,0,1) always per a code comment. This is wrong for non-static MO-transports and any GO with a `gameobject_addon.parent_rotation` row. Comment notes this; needs fixing once `GameObjectAddon` lands.
- `PercentHealth` (anim_progress) is a `u8` in the wire write but C++ template has it as `uint32 animprogress` for some types and as health-percent for destructibles — same byte slot, two semantics. Make sure conversion path is right per type.
- `ArtKit: int32(0)` is hardcoded; C++ honors `gameobject.artKit` plus 5× `gameobject_template_addon.ArtKits` array. Currently no GO will ever change ArtKit.
- `WorldEffects.Size: 0` always — Wintergrasp keeps/buildings rely on world-effect IDs to render damaged textures.
- Faction comes from `gameobject` spawn row in current code; C++ reads `m_gameObjectData->FactionTemplate` which is a wire field on the GO itself, set from `GetFaction()` (`FactionTemplate` in `gameobject_template`).
- No `Flags` field is set in the create block (always 0); doors will never appear locked, capture flags will never appear in-use.

**Tests existing:** 0 tests for game-object behavior. Some indirect coverage in `send_nearby_gameobjects` integration scenarios.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#ENTITIES_GAMEOBJECT.WBS.001** Partir y cerrar la migracion auditada de `game/Entities/GameObject/GameObject.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.cpp`
  Rust target: `crates/wow-world`, `crates/wow-packet`, `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 4488 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_GAMEOBJECT.WBS.002** Partir y cerrar la migracion auditada de `game/Entities/GameObject/GameObject.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.h`
  Rust target: `crates/wow-world`, `crates/wow-packet`, `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 516 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_GAMEOBJECT.WBS.003** Partir y cerrar la migracion auditada de `game/Entities/GameObject/GameObjectData.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObjectData.h`
  Rust target: `crates/wow-world`, `crates/wow-packet`, `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1412 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_GAMEOBJECT.WBS.004** Cerrar la migracion auditada de `game/Entities/GameObject/QuaternionData.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/QuaternionData.h`
  Rust target: `crates/wow-world`, `crates/wow-packet`, `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#GO.1** Define `GameObjectTemplate` enum-of-structs (one variant per `GameobjectTypes`, sharing common header: entry, type, displayId, name, size, ContentTuningId) in `wow-data` (XL — split per type group)
  - [ ] **#GO.1a** Group A: container-like (Door, Button, QuestGiver, Chest, Generic, Goober, Chair, Mailbox) (M)
  - [ ] **#GO.1b** Group B: combat (Trap, SpellFocus, SpellCaster, NewFlag, NewFlagDrop, ControlZone, DestructibleBuilding) (M)
  - [ ] **#GO.1c** Group C: utility (FishingNode, FishingHole, Goober, Camera, MeetingStone, FlagStand, FlagDrop, BarberChair, Binder, GuildBank, AuraGenerator, MapObject, GatheringNode, ItemForge, UILink) (M)
  - [ ] **#GO.1d** Group D: transports (Transport, MoTransport, TrapDoor, PhaseableMO) — coordinate with `entities-transport.md` (M)
- [ ] **#GO.2** Define `GameObjectTemplateAddon`, `GameObjectAddon`, `GameObjectOverride` POD + loaders (M)
- [ ] **#GO.3** Define `GameObjectData` spawn POD (rotation quaternion, animProgress, goState, artKit) and bind to `gameobject` row (L)
- [ ] **#GO.4** Define `GameObject` runtime entity in `wow-world` (struct holding `&'static GameObjectTemplate`, spawn data, current `LootState`, current `GOState`, owner GUID, respawn timer, value-union, per-player state map) (H)
- [ ] **#GO.5** Wire `GameObject` storage into `MapManager` analogous to `WorldCreature` (per-grid-cell `HashMap<ObjectGuid, GameObject>`) (M)
- [ ] **#GO.6** Implement `LootState` 4-state machine + `GOState` setter with broadcast (M)
- [ ] **#GO.7** Implement `Update(diff)`: trap pulse, fishing-bobber timer, restock, destructible repair, capture progress (H)
- [ ] **#GO.8** Implement `Use(user)` — the big switch — minimum viable cases (H, splittable):
  - [ ] **#GO.8a** Door / Button (open/close + auto-close timer + linkedTrap trigger) (M)
  - [ ] **#GO.8b** Chest (lock check via `Lock.db2`, loot generation from `gameobject_loot_template`, per-player loot, restock) (M)
  - [ ] **#GO.8c** Mailbox (open mail UI; tie to mail handlers) (L)
  - [ ] **#GO.8d** QuestGiver / FlagStand-as-questgiver (gossip flow) (L)
  - [ ] **#GO.8e** Goober (consumable script trigger) (L)
  - [ ] **#GO.8f** Chair (sit, multi-slot allocation) (L)
  - [ ] **#GO.8g** FishingNode (cast-bobber + outcome timer + fishing-hole snap + skill check) (M)
  - [ ] **#GO.8h** SpellFocus (cooking fire, anvil — passive; no Use) (L)
  - [ ] **#GO.8i** SpellCaster (cast spell on user) (L)
  - [ ] **#GO.8j** SummoningRitual (multi-player contribute → summon target) (M)
  - [ ] **#GO.8k** Binder (set hearthstone bind) (L)
  - [ ] **#GO.8l** Other 30+ types: stub or deferred (S)
- [ ] **#GO.9** Implement `ActivateObject(action, ...)` with all 45 `GameObjectActions` (H)
- [ ] **#GO.10** Build `SMSG_QUERY_GAMEOBJECT_RESPONSE` — currently only the request is decoded (M)
- [ ] **#GO.11** Implement `SaveToDB` / `DeleteFromDB` for GM tools (M)
- [ ] **#GO.12** Implement `Lock.db2` consumption + lock requirements per chest/door (M)
- [ ] **#GO.13** Implement `gameobject_loot_template` + `fishing_loot_template` loaders + per-GO loot state (M)
- [ ] **#GO.14** Implement destructible building HP / `SetDestructibleState` / `ModifyHealth` (M)
- [ ] **#GO.15** Implement capture-point ticking + `SMSG_CAPTURE_POINT_UPDATE` (M)
- [ ] **#GO.16** Implement NewFlag (BG flag) state machine + `SetFlagState` (M)
- [ ] **#GO.17** Implement `m_perPlayerState` (per-viewer GO state) (M)
- [ ] **#GO.18** Fix `ParentRotation` to consume `gameobject_addon.parent_rotation` instead of identity quaternion (L)
- [ ] **#GO.19** Fix `Flags`, `ArtKit`, `WorldEffects` in `GameObjectCreateData` to honor template + addon (L)
- [ ] **#GO.20** Implement collision toggle + VMap binding (H — depends on VMap port)
- [ ] **#GO.21** Implement `MeetsInteractCondition` + `IsAtInteractDistance` honoring `InteractRadiusOverride` per type (L)
- [ ] **#GO.22** Implement gathering-node skillup-list (`m_SkillupList`) (L)
- [ ] **#GO.23** Implement `m_perPlayerState` smooth-phasing visibility (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#ENTITIES_GAMEOBJECT.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 4 files / 6457 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObjectData.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.h`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | `cargo test -p wow-data && cargo test -p wow-database && cargo test -p wow-packet` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#ENTITIES_GAMEOBJECT.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 4 files / 6457 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObjectData.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.h`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#ENTITIES_GAMEOBJECT.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 4 files / 6457 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObjectData.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.h`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#ENTITIES_GAMEOBJECT.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 4 files / 6457 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObjectData.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.h`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: contrast `GameObject::packed_rotation()` against canonical C++ packing for a controlled set of rotations; do not treat the source comment as prior validation
- [ ] Test: door `Use` sets state ACTIVE, schedules auto-close at `template.door.autoClose` ms, then resets to READY
- [ ] Test: button with `linkedTrap` triggers the linked trap GO entry
- [ ] Test: chest `Use` consumes a charge if `template.chest.consumable == 1`; otherwise re-loots after `chestRestockTime`
- [ ] Test: chest with `template.chest.questID > 0` is invisible to players who haven't accepted the quest
- [ ] Test: chest `Lock.db2` requirement fails Use without picklock skill or key item
- [ ] Test: trap with `playerCast == 1` casts the trap spell when a player walks within `template.trap.radius`
- [ ] Test: trap with `charges > 0` despawns after the last charge
- [ ] Test: fishing bobber state goes NOT_READY → READY in `FISHING_BOBBER_READY_TIME` (5s)
- [ ] Test: fishing bobber snaps to nearest `GAMEOBJECT_TYPE_FISHINGHOLE` within range
- [ ] Test: fishing-hole `MaxOpens` decrements; despawns when 0
- [ ] Test: destructible building INTACT → DAMAGED at 50% HP, → DESTROYED at 0 HP, with correct `SMSG_DESTRUCTIBLE_BUILDING_DAMAGE`
- [ ] Test: capture-point assault progresses at correct tick rate, fires `SMSG_CAPTURE_POINT_UPDATE`
- [ ] Test: NewFlag `SetFlagState(Taken)` flips visibility to all but carrier
- [ ] Test: `ActivateObject(Despawn)` removes the GO and schedules respawn at `m_respawnDelayTime`
- [ ] Test: `ActivateObject(Lock)` sets `GO_FLAG_LOCKED`; `Unlock` clears it
- [ ] Test: `SetGoStateFor(state, viewer)` makes only `viewer` see the new state
- [ ] Test: GM-spawned GO via `SaveToDB` reloads identically across server restart

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 4 files / 6457 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObjectData.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.h` | `crates/wow-world/`, `crates/wow-packet/`, `crates/wow-data/`, `crates/wow-database/` \| ❌ not started — no `GameObject` type exists in Rust; only wire-format stub `GameObjectCreateData` for spawn broadcast and an `SEL_GAMEOBJECTS_IN_RANGE` query |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#ENTITIES_GAMEOBJECT.DIV.001` | `crates/wow-world/src/handlers/character.rs:2109,2454` (`missing_declared_path`, 0 Rust lines) | 4 C++ files / 6457 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObjectData.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#ENTITIES_GAMEOBJECT.DIV.002` | `crates/wow-world/src/session.rs:264,444` (`missing_declared_path`, 0 Rust lines) | 4 C++ files / 6457 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObjectData.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#ENTITIES_GAMEOBJECT.DIV.003` | `crates/wow-world/src/session.rs:996,1196` (`missing_declared_path`, 0 Rust lines) | 4 C++ files / 6457 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObjectData.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#ENTITIES_GAMEOBJECT.DIV.004` | `crates/wow-database/src/statements/world.rs:79,253` (`missing_declared_path`, 0 Rust lines) | 4 C++ files / 6457 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObjectData.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#ENTITIES_GAMEOBJECT.DIV.005` | `crates/wow-constants/src/object.rs:22,72` (`missing_declared_path`, 0 Rust lines) | 4 C++ files / 6457 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObjectData.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- The big tagged union in `GameObjectTemplate` (`door {}`, `button {}`, `chest {}`, ...) maps cleanly to a Rust `enum` but be careful: many entries share field names with different semantics across types (e.g. `linkedTrap` in `door`/`button`/`chest`, `cooldown` in `trap`/`goober`, `questID` everywhere). Keep field names exact per the C++ struct so SQL loaders stay translateable.
- `m_packedRotation` layout is `Z:bits[0..21]`, `Y:bits[21..42]`, `X:bits[42..64]` with sign-baked-into-w (`w_sign = sgn(W)`); `Y` and `Z` use 21 bits (2^20 magnitude), `X` uses 22. The Rust `packed_rotation()` was claimed correct by an unverified comment. That claim is not accepted; contrast the canonical packing and test exact bytes before changing it.
- `GO_FLAG_TRANSPORT` is set on both static (type 11) and MO (type 15) transports; check `GetGoType()` to disambiguate.
- A door's *initial* state is `template.door.startOpen` (0=closed, 1=open). Once `UseDoorOrButton` flips it, the auto-close goes back to the *opposite* of the initial state — easy to invert.
- `m_perPlayerState` (per-player GOState override) is critical for phased BG/quest content — a door looks open for some players, closed for others. C++ uses `unique_ptr` of a hashmap allocated lazily; in Rust prefer `Option<HashMap<...>>` to avoid the alloc when unused.
- `consumable` chests don't decrement automatically on every Use — only when **fully looted** does the consumable counter tick. Easy to mis-wire.
- `GameObject::Use` for `GAMEOBJECT_TYPE_QUESTGIVER` calls `Player::PrepareQuestMenu` then sends gossip — not the gossip path used by Creature questgivers. Different code branch, same UX.
- Capture-point GOs (`type 42`) used to be `type 29` (`CONTROL_ZONE`) in earlier versions; both exist in 3.4 — don't drop the old code path.
- `GAMEOBJECT_TYPE_NEW_FLAG` and `GAMEOBJECT_TYPE_NEW_FLAG_DROP` only exist in 8.x+; in 3.4 BG flags are `GAMEOBJECT_TYPE_FLAGSTAND` and `GAMEOBJECT_TYPE_FLAGDROP`. Verify against the WoLK 3.4.3 client before adding.
- GO with `unique_users` set tracks individual contributors; that's how summoning-circle "contribute" multi-player ritual UI knows when 4 players have clicked.
- Always use `IsAtInteractDistance(player, spell)` for the range check; raw `GetDistance` ignores `template.<type>.InteractRadiusOverride`.

---

## 12. C++ → Rust mapping

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class GameObject : WorldObject, GridObject<GameObject>, MapObject` | `struct GameObject { template_ref: &'static GameObjectTemplate, spawn: GameObjectSpawn, state: GameObjectRuntime }` | composition; `template_ref` interned |
| `class GameObjectTypeBase` virtual | trait `GameObjectTypeImpl` (Update/OnStateChanged/OnRelocated/IsNeverVisibleFor/ActivateObject) | one impl per `GameObjectType` enum variant |
| `union GameObjectValue` | `enum GameObjectValue { FishingHole { max_opens: u32 }, ControlZone { opvp: Arc<OPvPCapturePoint> }, Building { hp: u32, max_hp: u32 }, CapturePoint { last_team: TeamId, state: BgCapState, assault_timer: u32 } }` | tagged union |
| `GameObjectTemplate` w/ embedded union | `pub enum GameObjectTemplate { Door(DoorTemplate), Button(ButtonTemplate), Chest(ChestTemplate), Trap(TrapTemplate), ... }` plus `pub struct GameObjectHeader { entry, type_, display_id, name, size, content_tuning_id }` | one struct per type, sharing header |
| `GOState` enum | `#[repr(i8)] enum GoState { Active, Ready, Destroyed, TransportActive, TransportStopped, ... }` | matches wire byte |
| `LootState` enum | `enum LootState { NotReady, Ready, Activated, JustDeactivated }` | direct |
| `GameObjectFlags` (bitflag) | `bitflags::bitflags! struct GameObjectFlags: u32` | direct |
| `enum class GameObjectActions : uint32` | `#[repr(u32)] enum GameObjectAction` | 45 variants |
| `QuaternionData` | already implicit as `[f32; 4]` in `GameObjectCreateData`; promote to `struct Quaternion { x, y, z, w }` for clarity | — |
| `int64 m_packedRotation` | already `i64` returned by `packed_rotation()` | keep |
| `unique_ptr<Loot> m_loot` | `Option<Loot>` | direct |
| `unordered_map<ObjectGuid, unique_ptr<Loot>> m_personalLoot` | `Option<HashMap<ObjectGuid, Loot>>` | lazy |
| `GuidUnorderedSet m_SkillupList` | `HashSet<ObjectGuid>` | direct |
| `GuidSet m_unique_users` | `HashSet<ObjectGuid>` | ritual contributors |
| `ChairSlotAndUser ChairListSlots` | `BTreeMap<u32, ObjectGuid>` | small N (≤5) |
| `unique_ptr<unordered_map<ObjectGuid, PerPlayerState>> m_perPlayerState` | `Option<HashMap<ObjectGuid, PerPlayerState>>` | lazy alloc preserved |
| `WORLD_INS_GAMEOBJECT` | `WorldStatements::INS_GAMEOBJECT` | register in `wow-database` |
| `Use(Unit*)` | `fn use_(&mut self, user: ObjectGuid, ctx: &mut WorldCtx)` | match on `template_ref` variant |
| `Update(uint32)` | `fn tick(&mut self, diff_ms: u32, ctx: &mut TickCtx)` | dispatched via `GameObjectTypeImpl` |

---

## 13. §13 Audit (vs `/home/server/woltk-trinity-legacy/`)

| C++ symbol | Found in Rust | File | Verdict |
|---|---|---|---|
| `class GameObject` | no | — | ❌ missing |
| `class GameObjectTypeBase` | no | — | ❌ missing |
| `enum LootState` | no | — | ❌ missing |
| `enum GOState` | partial (used as `i8` in `GameObjectCreateData`) | `crates/wow-packet/src/packets/update.rs:1282` | ⚠️ wire-only, no enum |
| `enum GameobjectTypes` | partial (used as `u8` in `GameObjectCreateData`) | `crates/wow-packet/src/packets/update.rs:1278` | ⚠️ wire-only, no enum |
| `enum GameObjectFlags` | no | — | ❌ missing |
| `enum class GameObjectActions` | no | — | ❌ missing |
| `enum class FlagState` | no | — | ❌ missing |
| `enum GameObjectDestructibleState` | no | — | ❌ missing |
| `union GameObjectValue` | no | — | ❌ missing |
| `struct GameObjectTemplate` (any of 50+ type structs) | no | — | ❌ missing |
| `struct GameObjectTemplateAddon` | no | — | ❌ missing |
| `struct GameObjectAddon` | no | — | ❌ missing |
| `struct GameObjectOverride` | no | — | ❌ missing |
| `struct GameObjectData` (spawn) | partial (read by `SEL_GAMEOBJECTS_IN_RANGE`, projected into `GameObjectCreateData`) | `crates/wow-world/src/handlers/character.rs:2109` | ⚠️ only fields needed for create-block |
| `struct GameObjectLocale` | no | — | ❌ missing |
| `struct QuaternionData` | no (raw `[f32; 4]`) | — | ⚠️ untyped |
| `GameObject::Create` | no | — | ❌ missing |
| `GameObject::Update` | no | — | ❌ missing |
| `GameObject::Use` | no | — | ❌ missing — `CMSG_GAME_OBJ_USE` unhandled |
| `GameObject::SetGoState` | no | — | ❌ missing |
| `GameObject::UseDoorOrButton` | no | — | ❌ missing |
| `GameObject::ActivateObject` | no | — | ❌ missing (45 actions) |
| `GameObject::SetLootState` | no | — | ❌ missing |
| `GameObject::Respawn` / `DespawnOrUnsummon` | no | — | ❌ missing |
| `GameObject::ModifyHealth` / `SetDestructibleState` | no | — | ❌ missing |
| `GameObject::AssaultCapturePoint` | no | — | ❌ missing |
| `GameObject::SaveToDB` / `DeleteFromDB` | no | — | ❌ missing |
| `GameObject::SetParentRotation` | no | — | ❌ missing |
| `GameObject::UpdatePackedRotation` | yes (function-only, no struct) | `crates/wow-packet/src/packets/update.rs:1352` `packed_rotation()` | ✅ wire-format match |
| `GameObject::GetFishLoot` | no | — | ❌ missing |
| `GameObject::LookupFishingHoleAround` | no | — | ❌ missing |
| `GameObject::IsAtInteractDistance` | no | — | ❌ missing |
| `GameObject::MeetsInteractCondition` | no | — | ❌ missing |
| `GameObject::SetFlagState` | no | — | ❌ missing |
| `GameObject::SetGoStateFor` (per-player) | no | — | ❌ missing |
| `m_perPlayerState` | no | — | ❌ missing |
| `m_loot` / `m_personalLoot` / `m_tapList` | no | — | ❌ missing |
| `m_SkillupList` | no | — | ❌ missing |
| `m_unique_users` | no | — | ❌ missing |
| `WORLD_INS_GAMEOBJECT` / `WORLD_DEL_GAMEOBJECT` | no | — | ❌ missing |
| `SEL_GAMEOBJECTS_IN_RANGE` | yes | `crates/wow-database/src/statements/world.rs:79,253` | ✅ present |
| `CMSG_QUERY_GAMEOBJECT` request decoder | yes | `crates/wow-packet/src/packets/query.rs` | ✅ decoder only |
| `SMSG_QUERY_GAMEOBJECT_RESPONSE` builder | no | — | ❌ missing |
| `CMSG_GAME_OBJ_USE` | no | — | ❌ missing |
| `CMSG_GAME_OBJ_REPORT_USE` | no | — | ❌ missing |
| `SMSG_GAMEOBJECT_DESPAWN` | no | — | ❌ missing |
| `SMSG_GAMEOBJECT_CUSTOM_ANIM` | no | — | ❌ missing |
| `SMSG_GAME_OBJECT_ACTIVATE_ANIM_KIT` | partial (opcode constant only) | `crates/wow-constants/src/opcodes.rs:973` | ⚠️ enumerated, no handler |
| `SMSG_DESTRUCTIBLE_BUILDING_DAMAGE` | no | — | ❌ missing |
| `SMSG_FISH_NOT_HOOKED` / `SMSG_FISH_ESCAPED` | no | — | ❌ missing |
| `SMSG_CAPTURE_POINT_UPDATE` | no | — | ❌ missing |
| `Lock.db2` consumption | no | — | ❌ missing |
| `GameObjectDisplayInfo.db2` consumption | no | — | ❌ missing |
| `gameobject_loot_template` | no | — | ❌ missing |
| `fishing_loot_template` | no | — | ❌ missing |
| `TypeId::GameObject = 11` | yes | `crates/wow-constants/src/object.rs:72` | ✅ present |
| `TypeMask::GameObject = 8` | yes | `crates/wow-constants/src/object.rs:22` | ✅ present |
| `HighGuid::GameObject` | yes | `crates/wow-core/src/guid.rs` | ✅ present |
| `GameObjectCreateData` (spawn broadcast) | yes | `crates/wow-packet/src/packets/update.rs:1271` | ✅ wire-format only |

**Verdict:** ❌ not started. Surface coverage ≈ 2% (spawn broadcast only). The client sees GOs in the world but every interaction (Use/Query/Activate/Damage/Capture) is a no-op or broken. This is the largest gap in the entity layer after Player.

---

*Sub-doc of `entities.md`. Template version: 1.0 (2026-05-01).*
