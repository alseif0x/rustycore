# Migration: Entities

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Entities/`
> **Rust target crate(s):** `crates/wow-entities/` (PROPOSED — does not exist yet), partials in `crates/wow-core/`, `crates/wow-world/`, `crates/wow-ai/`, `crates/wow-data/`
> **Layer:** L4 (Entity layer — depends on Maps/Grids L3, consumed by Combat/Spells/AI L5+)
> **Status:** 🔧 broken (rewrite needed) — there is no `wow-entities` crate; entity state is scattered as flat fields inside `WorldSession` (Player) and `WorldCreature` (MapManager), no polymorphic hierarchy, no `UpdateMask` dirty-bit system, no `Item`/`GameObject`/`Pet`/`DynamicObject` entities at all
> **Audited vs C++:** ✅ complete (2026-05-01)
> **Last updated:** 2026-05-01

---

## 1. Purpose

The Entities mega-module is the *object model* of TrinityCore: every "thing" the player can interact with (player characters, creatures, NPCs, game objects, items, pets, totems, dynamic spell effects, transports, area triggers, conversations, scene objects, corpses, vehicles) inherits from a common `Object` base. Entities encapsulate identity (`ObjectGuid`), spatial state (`Position`, map/phase), the polymorphic `UpdateFields` payload that the wire protocol broadcasts via `SMSG_UPDATE_OBJECT`, and the lifecycle invariants (`AddToWorld` / `RemoveFromWorld` / `IsInWorld`) that every other system relies on. Without a faithful Entities layer, Maps cannot index, Combat has no targets, Spells have no casters, AI has no body, and inventory has no items.

---

## 2. C++ canonical files

All paths relative to `/home/server/woltk-trinity-legacy/`.

### 2.1 Object/ (base hierarchy + UpdateFields infrastructure)

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/game/Entities/Object/Object.h` | 845 | `Object` and `WorldObject` declarations; `CreateObjectBits`, `FindCreatureOptions`, virtual lifecycle hooks |
| `src/server/game/Entities/Object/Object.cpp` | 3798 | Lifecycle (`AddToWorld`/`RemoveFromWorld`), update-block builders, visibility queries, summon factories |
| `src/server/game/Entities/Object/ObjectGuid.h` | ~600 | `ObjectGuid` 128-bit GUID layout, `HighGuid`, `TypeMask`, `TypeID`, hash impls |
| `src/server/game/Entities/Object/ObjectGuid.cpp` | ~250 | GUID parsing, packed serialization, generator helpers |
| `src/server/game/Entities/Object/ObjectDefines.h` | ~250 | Constants: `MAX_AGGRO_RADIUS`, `DEFAULT_VISIBILITY_DISTANCE`, type masks |
| `src/server/game/Entities/Object/Position.h` | ~280 | `Position`, `WorldLocation`, distance/arc/angle helpers |
| `src/server/game/Entities/Object/Position.cpp` | ~150 | Position math implementations |
| `src/server/game/Entities/Object/G3DPosition.hpp` | ~80 | Conversions to/from G3D `Vector3`/`Vector4` for VMap |
| `src/server/game/Entities/Object/MovementInfo.h` | ~280 | `MovementInfo` POD: flags, transport, swim pitch, fall time, jump |
| `src/server/game/Entities/Object/GridObject.h` | ~80 | Intrusive `GridReference` mixin for grid linkage |
| `src/server/game/Entities/Object/ObjectPosSelector.h` | ~120 | Helper to find non-overlapping nearby positions |
| `src/server/game/Entities/Object/ObjectPosSelector.cpp` | ~210 | Position selector implementation |
| `src/server/game/Entities/Object/SmoothPhasing.h` | ~60 | Phase-shift smooth fade-in metadata |
| `src/server/game/Entities/Object/SmoothPhasing.cpp` | ~80 | SmoothPhasing impl |
| `src/server/game/Entities/Object/Updates/UpdateData.h` | 67 | `UpdateData` packet builder accumulating create/values/destroy blocks |
| `src/server/game/Entities/Object/Updates/UpdateData.cpp` | 72 | `UpdateData::BuildPacket` (compresses + writes `SMSG_UPDATE_OBJECT`) |
| `src/server/game/Entities/Object/Updates/UpdateField.h` | ~700 | `UpdateField<T>` mutator/observer template, `UpdateFieldFlag` |
| `src/server/game/Entities/Object/Updates/UpdateField.cpp` | ~80 | UpdateField helpers |
| `src/server/game/Entities/Object/Updates/UpdateFields.h` | 943 | DECLARED structs for `ObjectData`, `UnitData`, `PlayerData`, `ItemData`, `ContainerData`, `GameObjectData`, `DynamicObjectData`, `CorpseData`, `AreaTriggerData`, `SceneObjectData`, `ConversationData` (1500+ logical fields) |
| `src/server/game/Entities/Object/Updates/UpdateFields.cpp` | 5097 | Field write/read impls + descriptor tables consumed by `BuildValuesUpdate` |
| `src/server/game/Entities/Object/Updates/UpdateMask.h` | ~150 | Bit-mask helper for changed-field tracking |
| `src/server/game/Entities/Object/Updates/ViewerDependentValues.h` | ~250 | Per-viewer field overrides (faction-relative, group-relative) |

### 2.2 Unit/ (combat-capable base)

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/game/Entities/Unit/Unit.h` | 1953 | `Unit` declaration: stats, combat, threat, movement, spell auras |
| `src/server/game/Entities/Unit/Unit.cpp` | 13620 | Combat math, aura application, death/resurrection, threat list, vehicle code |
| `src/server/game/Entities/Unit/UnitDefines.h` | ~600 | `UnitFlags`, `UnitFlags2`, `UnitState`, `MovementFlags`, `WeaponAttackType`, `Stats`, `Powers` |
| `src/server/game/Entities/Unit/StatSystem.cpp` | ~1200 | Stat recalc on aura/equip change (Player + Creature paths) |
| `src/server/game/Entities/Unit/CharmInfo.h` | ~120 | Pet/charm ability bar metadata |
| `src/server/game/Entities/Unit/CharmInfo.cpp` | ~280 | CharmInfo impl |
| `src/server/game/Entities/Unit/enuminfo_UnitDefines.cpp` | ~120 | Generated enum reflection |

### 2.3 Player/ (PC + storage subsystems)

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/game/Entities/Player/Player.h` | 3189 | `Player` declaration: ~600 public methods, inventory slots, quest log, action bars, spec, talents |
| `src/server/game/Entities/Player/Player.cpp` | 29358 | Player impl (largest file in TC): create/save/load, inventory ops, quests, kills, XP, instancing |
| `src/server/game/Entities/Player/CinematicMgr.h/.cpp` | ~150 / ~280 | Cinematic camera path playback for the player |
| `src/server/game/Entities/Player/CollectionMgr.h/.cpp` | ~250 / ~600 | Toy/heirloom/transmog collection |
| `src/server/game/Entities/Player/CUFProfile.h` | ~80 | Compact unit-frame profiles (raid frames) |
| `src/server/game/Entities/Player/EquipmentSet.h` | ~120 | Saved equipment outfits |
| `src/server/game/Entities/Player/KillRewarder.h/.cpp` | ~80 / ~280 | XP/honor/loot distribution on kill |
| `src/server/game/Entities/Player/PlayerTaxi.h/.cpp` | ~120 / ~250 | Flight-path bitmask + active route |
| `src/server/game/Entities/Player/RestMgr.h/.cpp` | ~80 / ~180 | Rest XP accumulation in inns |
| `src/server/game/Entities/Player/SceneMgr.h/.cpp` | ~100 / ~280 | Cinematic scene scripting per-player |
| `src/server/game/Entities/Player/SocialMgr.h/.cpp` | ~120 / ~380 | Friend/ignore/RAF lists |
| `src/server/game/Entities/Player/TradeData.h/.cpp` | ~150 / ~280 | Pending player↔player trade window |

### 2.4 Creature/

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/game/Entities/Creature/Creature.h` | 547 | `Creature` declaration |
| `src/server/game/Entities/Creature/Creature.cpp` | 3568 | Creature lifecycle, loot, formation, respawn |
| `src/server/game/Entities/Creature/CreatureData.h` | ~600 | `CreatureTemplate`, `CreatureLevelStats`, `CreatureModel`, `CreatureAddon` POD |
| `src/server/game/Entities/Creature/CreatureGroups.h/.cpp` | ~150 / ~400 | Patrol/escort formation manager |
| `src/server/game/Entities/Creature/GossipDef.h/.cpp` | ~250 / ~600 | Gossip menu / quest giver UI structs |
| `src/server/game/Entities/Creature/TemporarySummon.h/.cpp` | ~120 / ~450 | `TempSummon` lifetime modes (timed, dead, despawn) |
| `src/server/game/Entities/Creature/Trainer.h/.cpp` | ~100 / ~280 | Class trainer spell list |
| `src/server/game/Entities/Creature/enuminfo_CreatureData.cpp` | ~280 | Generated enum reflection |

### 2.5 GameObject/

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/game/Entities/GameObject/GameObject.h` | 516 | `GameObject` declaration: chests, doors, traps, banners, fishing nodes |
| `src/server/game/Entities/GameObject/GameObject.cpp` | 4488 | GO state machine (Ready→Active→Despawn), use, loot, transport |
| `src/server/game/Entities/GameObject/GameObjectData.h` | ~280 | `GameObjectTemplate`, `GameObjectAddon`, type-specific data union |
| `src/server/game/Entities/GameObject/QuaternionData.h` | ~60 | Wrapper for rotation quaternion (Euler→quat conversion) |

### 2.6 Pet/

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/game/Entities/Pet/Pet.h` | 167 | `Pet` declaration (extends `Guardian`) |
| `src/server/game/Entities/Pet/Pet.cpp` | 1954 | Pet load/save, talents, happiness, hunter/warlock/dk specific |
| `src/server/game/Entities/Pet/PetDefines.h` | ~180 | `PetType`, `PetSlot`, `PetSaveMode` |

### 2.7 Item/ (Item, Bag, Container)

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/game/Entities/Item/Item.h` | 378 | `Item` declaration: bind state, stacking, enchants, durability |
| `src/server/game/Entities/Item/Item.cpp` | 2199 | Item lifecycle, durability decay, soulbinding, refund |
| `src/server/game/Entities/Item/ItemTemplate.h/.cpp` | ~600 / ~120 | `ItemTemplate` struct mirroring `item_template` SQL |
| `src/server/game/Entities/Item/ItemDefines.h` | ~280 | `InventoryType`, `ItemClass`, `ItemSubclass`, bonding constants |
| `src/server/game/Entities/Item/ItemEnchantmentMgr.h/.cpp` | ~100 / ~300 | Random property/enchant rolling |
| `src/server/game/Entities/Item/Container/Bag.h` | ~100 | `Bag` (slot container, extends `Item`) |
| `src/server/game/Entities/Item/Container/Bag.cpp` | ~230 | Bag slot mgmt |
| `src/server/game/Entities/Item/enuminfo_ItemDefines.cpp` | ~200 | Generated enum reflection |

### 2.8 Other entity sub-types

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/game/Entities/DynamicObject/DynamicObject.h` | ~120 | `DynamicObject` (persistent AoE spell field) |
| `src/server/game/Entities/DynamicObject/DynamicObject.cpp` | ~330 | DynObj lifecycle, periodic tick, viewer-dependent visibility |
| `src/server/game/Entities/AreaTrigger/AreaTrigger.h` | ~250 | `AreaTrigger` server-side spell volume |
| `src/server/game/Entities/AreaTrigger/AreaTrigger.cpp` | ~900 | AT movement (sphere/box/polygon), spline, action list |
| `src/server/game/Entities/AreaTrigger/AreaTriggerTemplate.h/.cpp` | ~180 / ~250 | Template + actions |
| `src/server/game/Entities/Conversation/Conversation.h/.cpp` | ~120 / ~380 | Multi-actor dialogue trigger |
| `src/server/game/Entities/Corpse/Corpse.h/.cpp` | ~120 / ~280 | Player/PC corpse persisted to DB |
| `src/server/game/Entities/Vehicle/Vehicle.h/.cpp` | ~250 / ~900 | Vehicle seat manager (passenger/accessory mgmt) |
| `src/server/game/Entities/Vehicle/VehicleDefines.h` | ~80 | `VehicleSeatFlags`, `VehicleExitParameters` |
| `src/server/game/Entities/Transport/Transport.h/.cpp` | ~200 / ~700 | Movable transport (boat, zeppelin) carrying passengers |
| `src/server/game/Entities/SceneObject/SceneObject.h/.cpp` | ~80 / ~180 | Per-player scripted scene visual |
| `src/server/game/Entities/Totem/Totem.h/.cpp` | ~80 / ~200 | Shaman/druid totem (extends `Minion`) |
| `src/server/game/Entities/Taxi/TaxiPathGraph.h/.cpp` | ~80 / ~280 | Flight network shortest-path solver (consumed by `PlayerTaxi`) |

**Total C++ lines in Entities/:** ~95,000 (excluding blanks/comments) — by far the heaviest mega-module in TrinityCore.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Object` | class (abstract) | Root: holds `m_objectTypeId`, `m_objectType` mask, `m_guid`, `m_values` (UpdateFields blob), `m_inWorld` flag |
| `WorldObject` | class : `Object`, `WorldLocation` | Adds map/phase/zone/area/visibility/`Acore::AnyData` event helpers |
| `Unit` | class : `WorldObject` | Combat-capable: stats, threat, auras, movement, vehicle, charm |
| `Player` | class : `Unit` | Human-controlled unit: client session, quests, inventory, talents, gossip |
| `Creature` | class : `Unit`, `GridObject<Creature>` | NPCs/mobs: AI, formation, vendor/quest-giver flags, loot |
| `Pet` | class : `Guardian` (Unit subtype) | Player pet (hunter/warlock/dk/mage water elemental) |
| `Guardian` | class : `Minion` | Owned, controlled creature with stats inherited from owner |
| `Minion` | class : `TempSummon` | Auto-following owned summon |
| `TempSummon` | class : `Creature` | Time-limited summoned creature |
| `Totem` | class : `Minion` | Stationary shaman totem |
| `GameObject` | class : `WorldObject`, `GridObject<GameObject>` | Static-ish interactive: doors, chests, transports, traps |
| `Item` | class : `Object` | Inventory item (NOT a `WorldObject`) — has `OwnerGUID`, `ContainerGUID` |
| `Bag` | class : `Item` | Container item with N slots |
| `DynamicObject` | class : `WorldObject`, `GridObject<DynamicObject>` | Server-side persistent spell area (e.g., Blizzard) |
| `AreaTrigger` | class : `WorldObject`, `GridObject<AreaTrigger>` | Movable spell volume (sphere/box/polygon/spline) |
| `Conversation` | class : `WorldObject`, `GridObject<Conversation>` | Multi-actor scripted dialogue |
| `Corpse` | class : `WorldObject`, `GridObject<Corpse>` | Player corpse (resurrectable, durable) |
| `SceneObject` | class : `WorldObject` | Single-player visual scene |
| `Vehicle` | class : `TransportBase` | Seat manager attached to a `Unit` or `Creature` |
| `Transport` | class : `GameObject`, `TransportBase` | Movable transport with passengers |
| `MovementInfo` | struct | Position + flags + transport + swim/fall — wire-format movement payload |
| `WorldLocation` | struct : `Position` | `Position` + `m_mapId` + `m_phaseMask` |
| `ObjectGuid` | struct (128-bit) | `HighGuid` (Player/Creature/Pet/GameObject/Item/Vehicle/...) + low/realm/server/map/entry/counter |
| `HighGuid` | enum class : `uint8` | 6-bit type tag (Player=0, Creature=1, Pet=2, GameObject=3, ...) |
| `TypeID` | enum : `uint8` | Object type enumeration (TYPEID_OBJECT/UNIT/PLAYER/GAMEOBJECT/...) |
| `TypeMask` | enum : `uint32` | Bit mask for `is-a` checks (TYPEMASK_UNIT|TYPEMASK_PLAYER) |
| `CreateObjectBits` | struct | Flags for `BuildCreateUpdateBlockForPlayer` (NoBirthAnim, EnablePortals, etc.) |
| `UpdateData` | class | Accumulator: create-block / values-block / out-of-range GUIDs → `SMSG_UPDATE_OBJECT` |
| `UpdateFields` (`UF::*Data`) | structs | One per type: `ObjectData`, `UnitData`, `PlayerData`, `ItemData`, `GameObjectData`, `DynamicObjectData`, `CorpseData`, `AreaTriggerData`, `SceneObjectData`, `ConversationData`, `ContainerData`, `ActivePlayerData` |
| `UpdateField<T>` | template | Mutator wrapper that flips a dirty bit on write |
| `UpdateMask` | class | Bit-mask of dirty UpdateFields indices |
| `UpdateFieldFlag` | enum : `uint8` | `None / Owner / PartyMember / UnitAll / Itemowner / SpecialInfo / ViewerDependent` |
| `ItemTemplate` | struct | `item_template` SQL row (entry, class, subclass, stats, bondtype, ilevel, ...) |
| `CreatureTemplate` | struct | `creature_template` SQL row |
| `GameObjectTemplate` | struct | `gameobject_template` SQL row |
| `CreatureAddon` | struct | Per-spawn delta (auras, mount, bytes1/2) |
| `CharmInfo` | struct | Pet ability slots + charm flags |
| `KillRewarder` | class | XP/honor/loot orchestrator on enemy death |
| `PlayerTaxi` | struct | Flight bitmask + queued path |
| `EquipmentSet` | struct | Saved equipment loadout |
| `TradeData` | struct | Active trade window state |
| `RestMgr` | class | Rest XP accrual |
| `SocialMgr` | class | Friend/ignore lists |
| `SmoothPhasing` | struct | Per-viewer fade-in/out phase metadata |
| `ObjectPosSelector` | class | Helper for non-overlapping summon placement |
| `FindCreatureOptions` | struct | `Acore::AnyOf` predicate bundle for creature searches |
| `FindGameObjectOptions` | struct | Same for GO searches |
| `Powers` | enum | Mana/Rage/Energy/RunicPower/Focus/etc. |
| `WeaponAttackType` | enum | BaseAttack / OffAttack / RangedAttack |
| `UnitState` | enum (bitmask) | InCombat / Stunned / Rooted / Charging / Casting / Charmed |
| `UnitFlags` / `UnitFlags2` | enum (bitmask) | NetworkObject / NonAttackable / Disarmed / Pacified / Silenced |
| `MovementFlags` | enum (bitmask) | Forward / Strafe / TurnLeft / Falling / Swimming / Hover / Flying |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `Object::AddToWorld()` | Sets `m_inWorld=true`; ASSERTs not already in world | derived overrides; called by `Map::AddToMap` |
| `Object::RemoveFromWorld()` | Clears `m_inWorld`; tear-down hook | derived overrides; called by `Map::RemoveFromMap` |
| `Object::IsInWorld()` | Invariant predicate — gated by ASSERT in many code paths | — |
| `Object::GetGUID()` | Stable identity of the object | — |
| `Object::GetTypeId()` / `GetObjectTypeId()` | Polymorphic type tag (no RTTI used) | — |
| `Object::BuildCreateUpdateBlockForPlayer(UpdateData*, Player*)` | Emits `UPDATETYPE_CREATE_OBJECT` block (full state for newly-visible target) | `BuildMovementUpdate`, `BuildValuesUpdate` |
| `Object::BuildValuesUpdateBlockForPlayer(UpdateData*, Player*)` | Emits `UPDATETYPE_VALUES` (only dirty fields) | `BuildValuesUpdate` (virtual per-type) |
| `Object::BuildOutOfRangeUpdateBlock(UpdateData*)` | Marks object for client destroy | — |
| `Object::SendUpdateToPlayer(Player*)` | One-shot push of own state to a single viewer | `BuildCreateUpdate...`, `Player::SendDirectMessage` |
| `WorldObject::AddToWorld() override` | Updates grid linkage + visibility | `Map::AddToActive`, `UpdateObjectVisibility` |
| `WorldObject::RemoveFromWorld() override` | Removes from grid, destroys for nearby | `Map::RemoveFromActive`, `DestroyForNearbyPlayers` |
| `WorldObject::Update(uint32 diff)` | Per-tick update (movement, events) | derived overrides |
| `WorldObject::GetMap()` / `GetMapId()` | Accessor for owning Map | — |
| `WorldObject::GetPosition()` / `Relocate(...)` | Position read/write | `UpdateObjectVisibility` |
| `WorldObject::UpdateObjectVisibility(bool forced)` | Recomputes visible-to set vs nearby Players | `Map::PlayerRelocation`, `Trinity::AIRelocationNotifier` |
| `WorldObject::IsWithinDistInMap(WorldObject const*, float, bool)` | Same-map distance check (3D w/ size) | `Position::distance` |
| `WorldObject::IsWithinLOS(float x,y,z, ...)` | Line-of-sight via VMap | `VMapMgr2::isInLineOfSight` |
| `WorldObject::SummonCreature(uint32 entry, Position const&, ...)` | Spawn a `TempSummon` on this map | `Map::SummonCreature`, `TempSummon::InitSummon` |
| `WorldObject::SummonGameObject(...)` | Spawn a GO | `Map::SummonGameObject` |
| `WorldObject::SetPhaseMask(uint32, bool update)` | Phase shift; client visibility re-sync | `UpdateObjectVisibility` |
| `Unit::DealDamage(Unit* victim, uint32 dmg, ...)` | Combat damage entry point | `Unit::Kill`, `Unit::AttackerStateUpdate` |
| `Unit::CastSpell(Unit*, uint32 spellId, ...)` | Spell cast init | `Spell::prepare`, `SpellMgr::GetSpellInfo` |
| `Unit::SetHealth(uint32)` / `ModifyHealth(int32)` | HP write — flips UpdateFields | `UpdateFields::UnitData::Health` |
| `Unit::Kill(Unit* victim, bool durabilityLoss)` | Death routine: drops loot, fires script, despawns | `KillRewarder::Reward`, `Creature::SetLootRecipient` |
| `Unit::AddAura(uint32 spellId, Unit* target)` | Apply aura | `AuraApplication`, `SpellAuraEffects` |
| `Unit::SetSpeed(UnitMoveType, float)` | Speed change with client packet | `SMSG_FORCE_*_SPEED_CHANGE` |
| `Player::Create(ObjectGuid::LowType, CharacterCreateInfo*)` | Construct new character (race/class/customizations) | `Player::SetCreateStats`, `PlayerInfo` |
| `Player::SaveToDB(bool create, bool logout)` | Persist all character state to `characters.*` tables | ~30 prepared statements |
| `Player::LoadFromDB(ObjectGuid, CharacterDatabaseQueryHolder const&)` | Re-hydrate from DB on login | many `_LoadXxx` helpers |
| `Player::TeleportTo(uint32 mapId, float x,y,z, float o, uint32 options)` | Inter-map / intra-map teleport | `Map::CreateMap`, `TransferPending` |
| `Player::AddItem(uint32 itemId, uint32 count)` | High-level inventory add | `StoreNewItemInBestSlots`, `Item::CreateItem` |
| `Player::StoreNewItem(ItemPosCountVec const&, uint32, bool, ItemRandomBonusListId)` | Low-level store w/ slot resolution | `_StoreItem`, `Item::AddToWorld` |
| `Player::CanStoreNewItem(...)` / `CanEquipItem(...)` | Inventory predicate (returns `InventoryResult`) | — |
| `Player::DestroyItem(uint8 bag, uint8 slot, bool update)` | Remove item, update equipment | `Item::RemoveFromWorld` |
| `Player::AddQuest(Quest const*, Object*)` | Accept quest | `QuestStatusData`, `_LoadQuestStatus` |
| `Player::GiveXP(uint32 xp, Unit* victim)` | XP grant + level-up trigger | `GiveLevel`, `UpdateAllStats` |
| `Player::SendDirectMessage(WorldPacket const*)` | Send packet to this player's session | `WorldSession::SendPacket` |
| `Creature::Create(ObjectGuid::LowType, Map*, uint32 entry, Position const&, CreatureData const*)` | Spawn from template | `InitEntry`, `SelectLevel`, `LoadCreaturesAddon` |
| `Creature::AIM_Initialize(CreatureAI*)` | Attach AI strategy | `FactorySelector::SelectAI` |
| `Creature::Update(uint32 diff)` | Tick: AI, movement, regen, spell timers | `CreatureAI::UpdateAI`, `Unit::Update` |
| `Creature::SetLootRecipient(Unit* unit)` | Mark first-tagger for loot | — |
| `GameObject::Create(...)` | Spawn GO from template | `LoadGameObjectFromDB` |
| `GameObject::Update(uint32 diff)` | GO state machine tick | `OnUse`, `OnLoot`, `Despawn` |
| `GameObject::Use(Unit* user)` | Player interacts (open chest, click banner) | `GameObjectAI::OnGossipHello`, scripts |
| `Item::Create(ObjectGuid::LowType, uint32 itemEntry, ItemContext, Player const* owner)` | Construct new Item instance | `SetEntry`, `SetItemRandomBonusList` |
| `Item::SaveToDB(CharacterDatabaseTransaction trans)` | Persist `item_instance`, enchants, gems | `CHAR_REP_ITEM_INSTANCE` |
| `Item::LoadFromDB(ObjectGuid::LowType, ObjectGuid ownerGuid, Field*, uint32 entry)` | Hydrate item | — |
| `Item::SetBinding(bool b)` | Soulbind | — |

---

## 5. Module dependencies

**Depends on:**
- **Maps** (`src/server/game/Maps/`) — every `WorldObject` requires a `Map*` to live in; `Object::AddToWorld` is invoked from `Map::AddToMap`. `WorldObject::GetMap()`, `Map::PlayerRelocation`.
- **Grids** (`src/server/game/Grids/`) — `GridObject<T>` mixin links each entity to a `Cell`/`NGrid`; visibility queries iterate grid notifiers.
- **DataStores / DB2** — `MapEntry`, `ChrClassesEntry`, `ChrRacesEntry`, `ItemSparseEntry`, `CreatureDisplayInfoEntry`, `GameObjectDisplayInfoEntry`, `FactionTemplateEntry`. Loaded via `DBCStores`/`DB2Stores`.
- **Database (CharacterDatabase + WorldDatabase)** — `Player`, `Item`, `Pet`, `Corpse` persist to character DB; `Creature`/`GameObject` templates+spawns load from world DB.
- **Spells** (`src/server/game/Spells/`) — `Unit::CastSpell`, `AuraApplication`, `SpellInfo`, `SpellMgr` lookups.
- **Combat** (`src/server/game/Combat/`) — `ThreatManager`, `CombatManager`, `HostileRefMgr` are members of `Unit`.
- **Movement** (`src/server/game/Movement/`) — `MotionMaster`, `MovementGenerator`, `Spline`. Each `Unit` owns a `MotionMaster*`.
- **AI** (`src/server/game/AI/`) — `CreatureAI`, `GameObjectAI`, `PlayerAI`. Selected via `FactorySelector`.
- **Loot** (`src/server/game/Loot/`) — `Creature::loot`, `GameObject::loot`, `Item::loot` (Group loot, Master Loot).
- **Quests** (`src/server/game/Quest/`) — `Player::AddQuest`, `Quest::CompleteQuest`, `KillCredit` propagate via `Player`.
- **Scripts** (`src/server/game/Scripting/`) — `ScriptMgr` invokes `OnPlayerLogin`, `OnCreatureCreate`, `OnGameObjectUse`, etc.
- **Battlegrounds / Instances / Outdoor PvP** — query `Player::GetBattleground()`, `Player::GetInstance()`.
- **Networking / Packets** — `WorldSession::SendPacket`, `UpdateData::BuildPacket`, every `SMSG_*` originating from a `Player::Send*` method.

**Depended on by:**
- Essentially every other game-server module: **Combat**, **Spells**, **Quests**, **Loot**, **Mail**, **Auctions**, **Trade**, **Group**, **Guild**, **Chat**, **Social**, **Achievements**, **Reputation**, **Talents**, **Pets**, **Battlegrounds**, **Instances**, **Phasing**, **OutdoorPvP**, **Calendar**, **AHBot**, **Scripts/ScriptAI**, **Warden**, **Tickets**.
- The **WorldSession** packet handlers in `src/server/game/Handlers/*Handler.cpp` operate on `_player` (a `Player*`) for almost every gameplay opcode.

---

## 6. SQL / DB queries (if any)

Player persistence is the single largest user of `CharacterDatabase`. Below is a representative slice from `src/server/database/Database/Implementation/CharacterDatabase.cpp` (~250+ statements; ~80 specifically for `Player`/`Item`/`Pet`/`Corpse`).

### 6.1 Character (Player core)

| Statement | Purpose | DB |
|---|---|---|
| `CHAR_SEL_CHARACTER` | Load a single character row by GUID (~80 columns: stats, flags, position, money, XP, etc.) | character |
| `CHAR_INS_CHARACTER` | Insert new character on creation | character |
| `CHAR_UPD_CHARACTER` | `Player::SaveToDB` main UPDATE | character |
| `CHAR_DEL_CHARACTER` | Hard delete | character |
| `CHAR_SEL_CHARACTER_AURAS` | Load active auras (`character_aura`) | character |
| `CHAR_SEL_CHARACTER_SPELL` | Known spells (`character_spell`) | character |
| `CHAR_SEL_CHARACTER_QUESTSTATUS` | Active quests (`character_queststatus`) | character |
| `CHAR_SEL_CHARACTER_QUESTSTATUS_REWARDED` | Completed quests | character |
| `CHAR_SEL_CHARACTER_REPUTATION` | Faction reputation values | character |
| `CHAR_SEL_CHARACTER_INVENTORY` | All inventory rows (`character_inventory` JOIN `item_instance`) | character |
| `CHAR_SEL_CHARACTER_ACTIONS` | Action-bar slot bindings | character |
| `CHAR_SEL_CHARACTER_SKILLS` | Skill ranks (`character_skills`) | character |
| `CHAR_SEL_CHARACTER_TALENTS` | Talent allocations | character |
| `CHAR_SEL_CHARACTER_GLYPHS` | Glyph slots | character |
| `CHAR_SEL_CHARACTER_HOMEBIND` | Hearthstone bind point | character |
| `CHAR_SEL_CHARACTER_CUSTOMIZATIONS` | Race/face/hair/etc. customizations | character |
| `CHAR_SEL_CHAR_POSITION` | Position-only fast load (used for redirect on map unload) | character |

### 6.2 Item / Container

| Statement | Purpose | DB |
|---|---|---|
| `CHAR_SEL_ITEM_INSTANCE` | Load `item_instance` row(s) | character |
| `CHAR_REP_ITEM_INSTANCE` | REPLACE — used by `Item::SaveToDB` | character |
| `CHAR_DEL_ITEM_INSTANCE` | Delete on destroy | character |
| `CHAR_INS_CHARACTER_INVENTORY` | Bind item to slot | character |
| `CHAR_DEL_CHARACTER_INVENTORY` | Wipe slot | character |
| `CHAR_SEL_ITEM_INSTANCE_GEMS` | Sockets | character |
| `CHAR_INS_ITEM_INSTANCE_GEMS` | Insert sockets | character |
| `CHAR_SEL_ITEM_BG_TRADE` | BG / Mythic+ trade-window history | character |

### 6.3 Pet / Corpse / Misc

| Statement | Purpose | DB |
|---|---|---|
| `CHAR_SEL_CHAR_PET` | Load active pet | character |
| `CHAR_REP_CHAR_PET` | Save pet | character |
| `CHAR_SEL_CHAR_PET_AURAS` | Pet auras | character |
| `CHAR_SEL_CORPSE` | Load player corpse for re-spawn | character |
| `CHAR_INS_CORPSE` | Persist corpse on death | character |
| `CHAR_DEL_CORPSE` | Despawn cleanup | character |

### 6.4 World DB (creature/GO templates + spawns)

| Statement / Table | Purpose | DB |
|---|---|---|
| `WORLD_SEL_CREATURE_TEMPLATE` (loader query) | `creature_template` rows → `CreatureTemplate` map | world |
| `WORLD_SEL_CREATURE` | `creature` (spawn rows) | world |
| `WORLD_SEL_CREATURE_ADDON` | `creature_addon` per-spawn deltas | world |
| `WORLD_SEL_GAMEOBJECT_TEMPLATE` | `gameobject_template` | world |
| `WORLD_SEL_GAMEOBJECT` | `gameobject` (spawn rows) | world |
| `WORLD_SEL_ITEM_TEMPLATE` | `item_template` | world |

### 6.5 DBC / DB2 stores consumed

| Store | What it loads | Read by |
|---|---|---|
| `MapStore` | Map.db2 | `WorldObject` (for `MapEntry`) |
| `ChrRacesStore` | ChrRaces.db2 | `Player::Create` |
| `ChrClassesStore` | ChrClasses.db2 | `Player::Create`, `Player::SetCreateStats` |
| `CreatureDisplayInfoStore` | CreatureDisplayInfo.db2 | `Creature::SetDisplayId` |
| `GameObjectDisplayInfoStore` | GameObjectDisplayInfo.db2 | `GameObject::SetDisplayId` |
| `ItemSparseStore` | Item-sparse.db2 | `ItemTemplate` |
| `FactionTemplateStore` | FactionTemplate.db2 | `WorldObject::IsHostileTo` |
| `EmotesStore` | Emotes.db2 | `Unit::HandleEmoteCommand` |
| `PowerTypeStore` | PowerType.db2 | `Unit` power init |

---

## 7. Wire-protocol packets (if any)

The Entities module is the *origin* of the most-frequently sent server packet: `SMSG_UPDATE_OBJECT`. All entity state visible to a client funnels through `UpdateData::BuildPacket`.

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_UPDATE_OBJECT` | server → client | `UpdateData::BuildPacket` (called from `Map::SendInitialVisiblePackets`, `Object::SendUpdateToPlayer`, `WorldObject::UpdateObjectVisibility`) |
| `SMSG_DESTROY_OBJECT` | server → client | `Object::SendOutOfRangeForPlayer`, `Player::ClearVisibleObject` |
| `SMSG_COMPRESSED_UPDATE_OBJECT` | server → client | `UpdateData::BuildPacket` (zlib path) |
| `SMSG_NAME_QUERY_RESPONSE` | server → client | `WorldSession::HandleNameQueryOpcode` (uses `ObjectAccessor::FindPlayer`) |
| `CMSG_NAME_QUERY` | client → server | requests `Player`/`Creature` name by GUID |
| `SMSG_CREATURE_QUERY_RESPONSE` | server → client | `WorldSession::HandleCreatureQueryOpcode` |
| `CMSG_CREATURE_QUERY` | client → server | template lookup |
| `SMSG_GAMEOBJECT_QUERY_RESPONSE` | server → client | `WorldSession::HandleGameObjectQueryOpcode` |
| `CMSG_GAMEOBJECT_QUERY` | client → server | template lookup |
| `SMSG_ITEM_QUERY_SINGLE_RESPONSE` | server → client | item template lookup |
| `CMSG_ITEM_QUERY_SINGLE` | client → server | item template lookup |
| `SMSG_INITIAL_SPELLS` | server → client | `Player::SendInitialSpells` (entity state → client) |
| `SMSG_TALENTS_INFO` | server → client | `Player::SendTalentsInfoData` |
| `SMSG_INITIALIZE_FACTIONS` | server → client | `ReputationMgr::SendInitialReputations` (Player rep state) |
| `SMSG_ACTION_BUTTONS` | server → client | `Player::SendInitialActionButtons` |
| `SMSG_LOGIN_SETTIMESPEED` | server → client | sent at `Player::SendInitWorldStates` |
| `SMSG_NEW_WORLD` / `SMSG_TRANSFER_PENDING` | server → client | `Player::TeleportTo` cross-map handoff |
| `SMSG_FORCE_RUN_SPEED_CHANGE` etc. | server → client | `Unit::SetSpeed` |
| `SMSG_AURA_UPDATE` | server → client | `Unit::SendAuraUpdate` |
| `SMSG_ATTACK_START` / `SMSG_ATTACK_STOP` | server → client | `Unit::Attack` / `Unit::AttackStop` |
| `SMSG_AI_REACTION` | server → client | `CreatureAI::AttackStart` |
| `SMSG_PARTY_KILL_LOG` / `SMSG_PARTY_MEMBER_STATS` | server → client | `Group::Update` reads `Player` fields |

The wire format of `SMSG_UPDATE_OBJECT` is essentially: per-object `(updateType, guid, mask, dirty fields...)` where the field layout is dictated by `UpdateFields.h` (per-type struct, ~1500 logical fields total across all object types).

---

## 8. Current state in RustyCore

**Files in `/home/server/rustycore`:**

| File | Lines | Coverage of C++ Entities |
|---|---|---|
| `crates/wow-core/src/guid.rs` | 790 | `ObjectGuid`, `HighGuid`, `TypeId` — covers ~70% of `ObjectGuid.h/.cpp` |
| `crates/wow-core/src/position.rs` | 190 | `Position` w/ distance/arc — ~50% of `Position.h/.cpp`. **Missing:** `WorldLocation` (no map/phase). |
| `crates/wow-core/src/lib.rs` | 7 | re-exports |
| `crates/wow-core/src/time.rs` | 166 | unrelated (clocks) |
| `crates/wow-world/src/session.rs` | 3138 | **Player state mixed into `WorldSession`** as flat fields: `player_guid`, `player_race`, `player_class`, `player_level`, `player_gender`, `player_gold`, `player_xp`, `player_position`, `player_name`, etc. **No `Player` struct exists.** |
| `crates/wow-ai/src/lib.rs` | 346 | `CreatureAI` struct doubles as the Creature entity itself: holds position, HP, combat target, wander logic. |
| `crates/wow-data/src/area_trigger.rs` | 312 | Static `AreaTriggerData` table + point-in-shape queries. **No live `AreaTrigger` entity** (no movement/spline/action runtime). |

**There is NO `crates/wow-entities` crate.** Entity types live as ad-hoc fields/structs across at least 4 unrelated crates.

**What's implemented:**
- `ObjectGuid` 128-bit layout with `HighGuid` discriminant; pack/unpack to bytes; counter helpers.
- `Position` with distance, arc, `point_at_distance`. Tested.
- `CreatureAI` minimal state machine: Idle/Wander/Combat/Dead, take_damage, respawn, melee swing timer.
- `AreaTriggerStore` static lookup for sphere/box/polygon containment (read-only template, no live triggers).
- Player flat state: name, race/class/level/gender, gold, XP, position, known spells, taxi nodes (some), action buttons.

**What's missing vs C++:**
- **The entire polymorphic hierarchy.** No `Object`/`WorldObject`/`Unit`/`Player`/`Creature`/`Pet` distinction. No `is-a` reasoning beyond `HighGuid` tagging.
- **`Player` struct as an entity.** Currently every Player field hangs directly off `WorldSession`. This conflates "the network connection" with "the in-world avatar"; logout/relogin/character-change semantics are ambiguous, and no other system can hold a `&Player` reference that is independent of the session.
- **`Creature` struct as an entity.** `CreatureAI` is doing double duty — there's no separation between the AI controller and the controlled body.
- **`Item`, `Bag`, `Container`** — none. No inventory entity model. (Some inventory state may exist as `Vec<u32>` of item ids at best.)
- **`GameObject`** — none.
- **`Pet`, `Totem`, `TempSummon`, `Guardian`, `Minion`** — none.
- **`DynamicObject`** — none.
- **`Conversation`, `SceneObject`, `Corpse`, `Vehicle`, `Transport`** — none.
- **`UpdateFields` system.** No `UpdateData`/`UpdateMask`/`UpdateField<T>` analogue. `SMSG_UPDATE_OBJECT` is built by hand per-opcode (when at all). No dirty-bit tracking. No per-viewer (owner/party/all/special) field-flag filtering.
- **`MovementInfo` parsing/serialization** — partial in `wow-packet`, not unified per entity.
- **Lifecycle invariants.** No `is_in_world` flag, no `add_to_world`/`remove_from_world` ASSERTed boundary. Entities are created and visibility-broadcast without a uniform contract.
- **Visibility / phasing pipeline.** `UpdateObjectVisibility`, `SmoothPhasing`, `PhaseShift` are absent.
- **DB persistence for entities other than Player session-fields.** `Item::SaveToDB`, `Pet::SaveToDB`, `Corpse::SaveToDB` paths do not exist.
- **Scripting hooks.** `ScriptMgr::OnCreatureCreate`/`OnGameObjectUse`/etc. — none.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- `CreatureAI` carries `Position` and HP, but `wow-world::session.rs` carries the player's `Position`, and `wow-map`/`MapManager` likely also caches entity positions for grid lookup. Three sources of truth for one fact.
- The handful of `SMSG_UPDATE_OBJECT`-shaped packets currently emitted (if any) almost certainly skip the create-block / values-block / out-of-range distinction; on first-sight vs. delta the client behavior will diverge.
- `ObjectGuid` realm/server/map/entry sub-fields exist in code but are not tied to any HighGuid that actually uses them (e.g. the C++ uses these for `HighGuid::Transport`, `HighGuid::GameObject` only).
- `player_position` in `WorldSession` is `Option<Position>` — unsetting it (logout, transfer) is a different state than "alive at (0,0,0)". This is fragile and unlike the C++ which uses `IsInWorld()` as the gate.

**Tests existing:**
- `crates/wow-core/src/position.rs` — 6 unit tests (distance, arc, zero-position).
- `crates/wow-core/src/guid.rs` — handful of pack/unpack tests.
- `crates/wow-ai/src/lib.rs` — basic `CreatureAI` state-transition tests (if present).
- **Zero integration tests** for entity lifecycle, `UpdateFields`, or polymorphic dispatch.

---

## 9. Migration sub-tasks

Numbered for cross-reference from `MIGRATION_ROADMAP.md` §5. Complexity: **L** (<1h), **M** (1–4h), **H** (4–12h), **XL** (>12h, split).

- [ ] **#ENTITIES.1** Create new crate `crates/wow-entities/` with workspace wiring, depending on `wow-core`, `wow-data`, `wow-constants` (L)
- [ ] **#ENTITIES.2** Define `TypeId` and `TypeMask` enums; ensure parity with `Object.h` (L)
- [ ] **#ENTITIES.3** Define `WorldLocation { position, map_id, phase_mask }` in `wow-core` and propagate (M)
- [ ] **#ENTITIES.4** Design polymorphism strategy: trait `Entity` for shared interface (`guid`, `type_id`, `add_to_world`, `remove_from_world`, `is_in_world`, `update`) + concrete structs; document why (no inheritance) (M)
- [ ] **#ENTITIES.5** Implement `Object` base struct: `guid`, `type_id`, `type_mask`, `in_world: bool`, `values: UpdateValuesBlob` — with debug assertions matching C++ ASSERTs (M)
- [ ] **#ENTITIES.6** Implement `WorldObject` struct embedding `Object` + `WorldLocation` + `visibility_distance` + `phase_mask`. Verify constructor, getters (M)
- [ ] **#ENTITIES.7** Port `MovementInfo` POD with full flags/transport/swim/fall/jump fields; round-trip (de)serialize against captured wire bytes (M)
- [ ] **#ENTITIES.8** Design `UpdateField<T>` equivalent: a `Tracked<T>` wrapper that flips a bit in a `UpdateMask` on `set` (H)
- [ ] **#ENTITIES.9** Generate (or hand-port) the `UnitData` / `PlayerData` / `ObjectData` / `ItemData` / `ContainerData` / `GameObjectData` / `DynamicObjectData` / `CorpseData` / `AreaTriggerData` / `SceneObjectData` / `ConversationData` field structs from `UpdateFields.h` — name, type, index, flags (XL — split per-type) (XL)
- [ ] **#ENTITIES.10** Implement `UpdateData` accumulator: `add_create_block`, `add_values_block`, `add_destroy_guid`, `build_packet` → `SMSG_UPDATE_OBJECT` (or compressed variant) (H)
- [ ] **#ENTITIES.11** Implement `BuildCreateUpdateBlockForPlayer` / `BuildValuesUpdateBlockForPlayer` per-type, with `UpdateFieldFlag` filtering (Owner / PartyMember / UnitAll / SpecialInfo / ViewerDependent) (H)
- [ ] **#ENTITIES.12** Implement `Unit` struct: stats array, power array, threat-list reference, motion master reference, aura container reference (H)
- [ ] **#ENTITIES.13** Extract `Player` from `WorldSession`: create `Player` struct owning all per-character fields; `WorldSession` keeps only `Option<PlayerHandle>` (XL — split: 12a fields, 12b methods, 12c handlers rewire) (XL)
- [ ] **#ENTITIES.14** Implement `Player::create` matching C++ `Player::Create(CharacterCreateInfo*)` (race/class default stats, starting items, starting spells) (H)
- [ ] **#ENTITIES.15** Implement `Player::save_to_db` covering at minimum `characters`, `character_inventory`, `character_spell`, `character_skills`, `character_action`, `character_aura`, `character_queststatus`, `character_homebind` (XL)
- [ ] **#ENTITIES.16** Implement `Player::load_from_db` reverse of #15 (XL)
- [ ] **#ENTITIES.17** Implement `Player::teleport_to(map, x, y, z, o)` with cross-map (`SMSG_TRANSFER_PENDING` + `SMSG_NEW_WORLD`) and intra-map fast paths (H)
- [ ] **#ENTITIES.18** Implement `Item` struct + `Bag` (Item subtype with N slots) + `item_instance` SQL persistence (H)
- [ ] **#ENTITIES.19** Implement `Player::can_store_new_item`, `store_new_item`, `destroy_item`, `equip_item` returning `InventoryResult` enum (H)
- [ ] **#ENTITIES.20** Replace flat `CreatureAI` with `Creature` struct (Unit subtype) + separate `CreatureAI` controller; preserve current AI behavior (H)
- [ ] **#ENTITIES.21** Implement `Creature::create` from `CreatureTemplate`, with `CreatureLevelStats` selection by level/rank (M)
- [ ] **#ENTITIES.22** Implement `GameObject` struct + `GameObjectTemplate` + state machine (Ready → Active → Despawn) (H)
- [ ] **#ENTITIES.23** Implement `GameObject::use` (chest, door, banner) hook into scripts (M)
- [ ] **#ENTITIES.24** Implement `Pet` (extends `Guardian`/`Minion`/`TempSummon`); summon/dismiss/save/load (H)
- [ ] **#ENTITIES.25** Implement `Totem` (Minion subtype, stationary) (M)
- [ ] **#ENTITIES.26** Implement `DynamicObject` (persistent AoE spell field) — lifecycle + tick (M)
- [ ] **#ENTITIES.27** Migrate live `AreaTrigger` entity (movement, spline, action list); existing `area_trigger.rs` becomes the static template store (H)
- [ ] **#ENTITIES.28** Implement `Corpse` entity + `corpse` SQL persistence; tie into Player death flow (M)
- [ ] **#ENTITIES.29** Implement `Conversation`, `SceneObject` (low priority, unblocks newer scripts) (M)
- [ ] **#ENTITIES.30** Implement `Vehicle` seat manager attached to `Unit`/`Creature` (H)
- [ ] **#ENTITIES.31** Implement `Transport` (movable GO carrying passengers) (H)
- [ ] **#ENTITIES.32** Wire the `add_to_world` / `remove_from_world` lifecycle into `MapManager`: every entity insertion goes through `Map::add_to_map` which calls `entity.add_to_world()`; symmetrical for removal. Add `debug_assert!` invariants matching C++ (H)
- [ ] **#ENTITIES.33** Implement `update_object_visibility(forced: bool)` per-entity; integrate with grid relocation notifiers (H)
- [ ] **#ENTITIES.34** Migrate handlers in `wow-world/src/handlers/` from reading `WorldSession::player_*` fields to reading `WorldSession::player.as_ref()` (XL — touches every handler file; do incrementally) (XL)
- [ ] **#ENTITIES.35** Add `ScriptMgr` hooks: `on_player_login`, `on_player_logout`, `on_creature_create`, `on_creature_kill`, `on_gameobject_use`, `on_item_equip` (M)

---

## 10. Regression tests to write

- [ ] Test: `Object::AddToWorld` then `RemoveFromWorld` invariants — second `add` panics; `remove` without prior `add` panics; `is_in_world` matches.
- [ ] Test: `ObjectGuid` round-trip through 16-byte wire format for every `HighGuid` variant (Player/Creature/Pet/GameObject/Item/Vehicle/Transport).
- [ ] Test: `Position::distance`, `is_within_dist`, `has_in_arc`, `point_at_distance` — values match TC C++ within 1e-4 (compare against captured C++ outputs).
- [ ] Test: `Player::create` for each of the 10 classic race/class combos produces correct starting stats, starting items, starting spells (table-driven from `playercreateinfo.*`).
- [ ] Test: `Player::save_to_db` then `Player::load_from_db` round-trip — every field equal (golden-file test).
- [ ] Test: `Item::create` + `save_to_db` + `load_from_db` round-trip; enchants and gems preserved.
- [ ] Test: `Player::can_store_new_item` returns correct `InventoryResult` for full bag, mismatched bag-family, conjured-stack, unique item, BoP equipped.
- [ ] Test: `UpdateData::build_packet` for a single Creature create-block produces byte-identical output to a captured TC `SMSG_UPDATE_OBJECT` payload (golden bytes).
- [ ] Test: `UpdateMask` only marks fields that changed; clean entity emits no `UPDATETYPE_VALUES` block.
- [ ] Test: `UpdateFieldFlag::Owner` filtering — non-owner viewer does NOT receive owner-only fields (e.g. `PlayerData::InvSlots`).
- [ ] Test: `ViewerDependentValues` — faction-relative `Bytes2` differs between hostile and friendly viewer.
- [ ] Test: `Creature::take_damage` past zero triggers `die` exactly once; `should_respawn` after timer; respawn restores HP and position.
- [ ] Test: `Player::teleport_to` cross-map transition emits `SMSG_TRANSFER_PENDING` then `SMSG_NEW_WORLD` in order; intra-map skips both.
- [ ] Test: Visibility — `Player.add_to_world` triggers `BuildCreateUpdateBlockForPlayer` for every other Player within `default_visibility_distance` and only those.
- [ ] Test: `GameObject::use` — chest opens once, loot rolls, second use within cooldown is rejected.
- [ ] Test: `Pet::save_to_db` then dismiss + summon = `load_from_db` restores HP/auras/talents.
- [ ] Test: Phase mask — entity with `phase_mask = 0x1` invisible to viewer with `phase_mask = 0x2`.

---

## 11. Notes / gotchas

- **`UpdateFields` is the most expensive port.** `UpdateFields.h` is 943 lines of struct declarations; `UpdateFields.cpp` is **5097 lines** of generated descriptor tables. Across all object types there are **~1500 logical fields**. A naive port is multi-week. Strongly consider code-generating from `UpdateFields.h` rather than hand-writing.
- **C++ inheritance is non-trivial.** `Pet : Guardian : Minion : TempSummon : Creature : Unit : WorldObject : Object` — that's a 7-deep single chain plus `GridObject<T>` mixin. `Vehicle : TransportBase` is a *separate* base used as a member, not a parent. Rust will need composition + traits + a `TypeId` discriminant; do NOT try to mirror the chain.
- **Multi-source-of-truth risk.** Position currently lives in `wow-world::session`, `wow-ai::CreatureAI`, and is also tracked by `MapManager`. Pick `Map` as the authoritative spatial index and have entities expose `position()`. Anything else will desync under load.
- **`IsInWorld()` is a load-bearing invariant.** TC sprinkles `ASSERT(IsInWorld())` and `if (!IsInWorld()) return;` across thousands of call sites. Skipping this in Rust will manifest as use-after-free-equivalent logic bugs (entities updated after grid removal).
- **`AddToWorld` / `RemoveFromWorld` are virtual and chain.** `WorldObject::AddToWorld` calls `Object::AddToWorld` then registers with grid; subclasses override and call parent. Forgetting to chain breaks visibility — write a regression test specifically for this.
- **`Player.cpp` is 29,358 lines.** Do not migrate in one sitting. Slice by responsibility (creation, inventory, quests, talents, social, taxi, instance, save/load). Each slice is its own #ENTITIES.x sub-task.
- **`KillRewarder` is shared XP / loot policy** between Creature and Player and lives under Player/. It transitively touches Group, Quest, and Achievement systems — do not migrate it standalone before those exist as stubs.
- **`UpdateFieldFlag` viewer-dependent values** — fields like `UnitData::Bytes2` (faction colors) and `UnitData::Health` (group sees percent, others see range) require a *per-target* render of the values block. The C++ does this via `ViewerDependentValues<T>`. Easy to miss; will manifest as health-bar mismatches or faction-color leaks.
- **WoLK 3.4.3-specific:** `SMSG_UPDATE_OBJECT` field layout in 3.4.3 client differs from earlier 3.3.5a TC ports — the `UpdateFields.h` we're porting is the **3.4.3** one, NOT a leak from 3.3.5a. Verify against `woltk-trinity-legacy` only.
- **`Item` is NOT a `WorldObject`.** It's `Object`-direct. It has no Position, no Map. This breaks the assumption "everything is in a grid." Bag/Item must be discoverable by GUID via `ObjectAccessor`/inventory linkage, not by spatial query.
- **`Transport` is BOTH a `GameObject` AND `TransportBase`** (a separate hierarchy for things-with-passengers). This is the only multi-inheritance case in Entities; in Rust, model `Transport` as a `GameObject` that holds a `TransportBase` field (composition), exposing the seat-manager interface via trait.
- **Lazy save** — `Player::SaveToDB` is called periodically (every N minutes) AND on logout AND on certain transitions. The current Rust code has no save-throttle; a port must implement it or risk DB churn.
- **Pet auras persist across logout.** `pet_aura` table. Easy to forget; pet abilities feel broken without it.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Object` | `struct Object` in `crates/wow-entities/src/object.rs` | Embed in subtypes (composition); expose via `Entity` trait |
| `class WorldObject : Object, WorldLocation` | `struct WorldObject { object: Object, location: WorldLocation }` | No inheritance; methods on `WorldObject` do `self.object.foo()` |
| `class Unit : WorldObject` | `struct Unit { world_object: WorldObject, stats: ..., powers: ..., aura_holder: ..., motion: ... }` | Composition; `impl Unit { ... }` for combat methods |
| `class Player : Unit` | `struct Player { unit: Unit, inventory: Inventory, quest_log: QuestLog, talents: TalentTab, ... }` | Largest struct; split internals into sub-modules |
| `class Creature : Unit, GridObject<Creature>` | `struct Creature { unit: Unit, template: Arc<CreatureTemplate>, ai: Box<dyn CreatureAI>, ... }` | AI behind trait object |
| `class Pet : Guardian` | `struct Pet { guardian: Guardian, ... }` (Guardian composes Minion composes TempSummon composes Creature) | Composition chain; fewer indirections than C++ inheritance chain |
| `class GameObject` | `struct GameObject { world_object: WorldObject, template: Arc<GameObjectTemplate>, state: GameObjectState, ... }` | State enum for Ready/Active/Despawn |
| `class Item : Object` | `struct Item { object: Object, owner_guid: ObjectGuid, container_guid: Option<ObjectGuid>, slot: u8, ... }` | NOT a `WorldObject` |
| `class Bag : Item` | `struct Bag { item: Item, slots: Vec<Option<ItemHandle>> }` | Composition |
| `class DynamicObject` | `struct DynamicObject { world_object: WorldObject, caster: ObjectGuid, spell_id: u32, radius: f32, duration: u32 }` | — |
| `class AreaTrigger` | `struct AreaTrigger { world_object: WorldObject, template: Arc<AreaTriggerTemplate>, shape: TriggerShape, spline: Option<Spline>, ... }` | The existing `wow-data::area_trigger` becomes the template store |
| `class Corpse` | `struct Corpse { world_object: WorldObject, owner_guid: ObjectGuid, items: Vec<u32>, decay_timer: u32 }` | — |
| `class Vehicle : TransportBase` | `struct Vehicle { base: TransportBase, owner: EntityRef, seats: Vec<VehicleSeat> }` | Held as `Option<Vehicle>` on the owning Unit |
| `class Transport : GameObject, TransportBase` | `struct Transport { game_object: GameObject, base: TransportBase, path: Spline, passengers: HashSet<ObjectGuid> }` | The only multi-inheritance case → composition |
| `Object*` raw pointer | `EntityHandle` — opaque GUID + weak reference into the map's slotmap | Avoid `Arc<RwLock<Entity>>` per-entity; use ECS-style storage in `Map` |
| `Player*` (long-lived ref) | `&Player` borrowed from `Map::get_player(guid)` for the duration of a call | No long-lived refs; re-resolve each tick |
| `std::map<ObjectGuid, Object*>` | `HashMap<ObjectGuid, EntityKey>` (slotmap key) | Inside `Map`, not global |
| `ObjectGuid` (128-bit) | `wow_core::ObjectGuid` | Already implemented; verify HighGuid coverage |
| `MovementInfo` | `struct MovementInfo` | POD-style; serde or hand-rolled wire (de)ser |
| `UpdateData` | `struct UpdateData { create_blocks: Vec<Bytes>, values_blocks: Vec<Bytes>, destroy_guids: Vec<ObjectGuid> }` | `build_packet() -> WorldPacket` |
| `UpdateField<T>` | `struct Tracked<T> { value: T, dirty_bit: u16 }` + `UpdateMask` | `set(&mut self, v: T)` flips bit |
| `UpdateMask` | `struct UpdateMask(BitVec)` | One per object type; size = field count |
| `class CreatureAI` (virtual) | `trait CreatureAI` | Already exists in `wow-ai`; rename current impl to e.g. `DefaultCreatureAI` |
| `class GameObjectAI` (virtual) | `trait GameObjectAI` | New |
| `class PlayerAI` (virtual) | `trait PlayerAI` | New (used for charm/possession) |
| `void Foo::Update(uint32 diff)` | `fn update(&mut self, diff: Duration)` (or `u32` ms) | Pick `Duration` consistently project-wide |
| `virtual void AddToWorld()` | `fn add_to_world(&mut self)` on `Entity` trait | `debug_assert!(!self.in_world)` to mirror C++ ASSERT |
| `bool IsInWorld() const` | `fn is_in_world(&self) -> bool` | — |
| `Player::SaveToDB(bool create, bool logout)` | `async fn save_to_db(&self, db: &CharDb, create: bool, logout: bool) -> Result<()>` | Async; transactional |
| `class TC_GAME_API X` macro | `pub struct X` with `#[cfg(...)]` if visibility split needed | — |
| `friend class` | `pub(crate)` fields / `pub(super)` | Reduce surface area |
| `enum class HighGuid : uint8` | `#[repr(u8)] enum HighGuid` | Already done |
| `std::shared_ptr<Item>` (rare) | `Arc<Item>` for cross-system shares (mail, trade) | Most items are `Box<Item>` owned by their container |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

Cross-checked C++ canonical sources at `/home/server/woltk-trinity-legacy/src/server/game/Entities/` (Object/Object.{h,cpp}, Unit/Unit.{h,cpp}, Player/Player.{h,cpp}, Creature/Creature.{h,cpp}, GameObject/GameObject.{h,cpp}, Item/Item.{h,cpp}) against current Rust state in `crates/wow-core/`, `crates/wow-world/`, `crates/wow-packet/`, `crates/wow-ai/`. Verdict: **🔧 broken — no entity layer exists**, only a flat-field session and a flat-field creature record.

### Object (`Object.{h,cpp}` — 845 + 3798 lines)

C++ surface: `AddToWorld()` / `RemoveFromWorld()` / `IsInWorld()` invariants, `BuildCreateUpdateBlockForPlayer`, `BuildValuesUpdateBlockForPlayer`, `BuildOutOfRangeUpdateBlock`, `SendUpdateToPlayer`, `m_objectType` mask, `m_values` (UpdateFields blob), `GetGUID()` / `GetTypeId()`. Roughly 60 public methods. Rust: **none**. There is no `Object` struct, no `is_in_world()` predicate, no `add_to_world()` lifecycle. `crates/wow-core/src/guid.rs:112` defines `ObjectGuid` and `crates/wow-packet/src/world_packet.rs:422` has working `write_packed_guid`/`read_packed_guid` (tested) — that is all of `Object` that exists. Coverage: ~5% (GUID identity only).

### Unit (`Unit.{h,cpp}` — 1953 + 13620 lines)

C++ surface: `DealDamage`, `Kill`, `CastSpell`, `SetHealth` / `ModifyHealth` / `SetMaxHealth`, `AddAura` / `RemoveAura`, `SetSpeed`, `Attack` / `AttackerStateUpdate`, `EnterCombat` / `LeaveCombat`, threat manager, motion master, charm/possess plumbing, vehicle. **The 13620-line monster.** Rust: **none of it as a `Unit` type**. `crates/wow-world/src/map_manager.rs:52 WorldCreature` has only `health`, `max_health`, `take_damage(damage) -> bool` (line 176), `enter_combat(attacker)`, `die()`, `try_aggro`, `respawn`. No absorbs, no immunes, no reflections, no school masks, no resistances, no aura container, no threat list, no motion master integration with VMap, no spell-cast pipeline. **Worst divergence in this file (and possibly the project): `Unit::DealDamage` is the universe-touching combat entry point in C++ — multiple absorb passes, immune checks, school resist, reflect-back-to-caster, aura proc dispatch, threat update, kill detection — and the Rust replacement is `WorldCreature::take_damage(u32) -> bool` returning true on death. No Player-side counterpart exists.** Coverage: <2%.

### Player (`Player.{h,cpp}` — 3189 + 29358 lines, ~600 public methods)

C++ surface: `Create(CharacterCreateInfo*)`, `SaveToDB(create, logout)`, `LoadFromDB(guid, holder)`, `TeleportTo`, `GiveXP` / `GiveLevel`, `AddItem` / `StoreNewItem` / `CanStoreNewItem` / `CanEquipItem` / `DestroyItem`, `AddQuest`, `Update(diff)` (~50 sub-systems ticked: regen, dueling, tradeskill timers, GM cmds, save throttle, pvp timer, drunk decay, weather, taxi, contested-area, pet sync, instance-bind sliding-window, …), `SendDirectMessage`, etc. Rust: **no `Player` struct exists.** Player state is scattered as ~70 flat fields directly on `WorldSession` (`crates/wow-world/src/session.rs:181` `player_gold`, `:182` `player_xp`, `:189` `player_guid`, `:197` `inventory_items: HashMap<u8, InventoryItem>`, `:225` `player_position`, `:228` `player_name`, `:272` `player_quests`, etc.). The 4611-line `handlers/character.rs` operates directly on these fields. Login flow exists (`handle_continue_player_login` at `:1330`, ~400 lines including initial spells, action buttons, item load), and an idle save loop calls `save_player_gold` (`session.rs:615`), but `Player::Update(diff)` per-tick driver is **missing**: `tick_combat_sync` / `tick_creatures_sync` / `tick_active_spell_cast` / `tick_auras` exist as ~4 isolated ticks, vs C++'s ~50 sub-system orchestrator. **Conflates network connection with in-world avatar.** Coverage: ~15% (login + flat field bag + 4 ticks).

### Creature (`Creature.{h,cpp}` — 547 + 3568 lines)

C++ surface: `Create(guidlow, map, entry, position, CreatureData)`, `AIM_Initialize`, `Update(diff)`, `SetLootRecipient`, formation, respawn, vendor/quest-giver flags, loot, addon spell auras. Rust: `crates/wow-world/src/map_manager.rs:52 WorldCreature` is the closest analogue — 134 lines (52..285) covering position, hp, combat target, wander timer, melee swing timer, respawn timer, idle/wander/combat/dead state. AI separation does not exist (`crates/wow-ai/src/lib.rs:211 take_damage` is on the AI struct itself). No formation, no loot recipient, no addon spells, no `AIM_Initialize` strategy slot. Coverage: ~25% (movement + basic combat + respawn).

### GameObject (`GameObject.{h,cpp}` — 516 + 4488 lines)

C++ surface: `Create`, `Update(diff)` state machine (Ready → Active → Despawn), `Use(Unit*)` (open chest, click banner, fishing, doodad), loot, transport carry, lock check, OnGossipHello hook. Rust: **none.** No `GameObject` struct, no chest/door/banner code anywhere. Coverage: 0%.

### Replication / UpdateFields infrastructure

C++ has `UpdateFields.h` (943 lines of struct decls) + `UpdateFields.cpp` (5097 lines of descriptor tables) + `UpdateField<T>` mutator template + `UpdateMask` bit-vector + `ViewerDependentValues<T>` + `UpdateData::BuildPacket` accumulator. Roughly **1500 logical fields** across all object types with per-field `Owner / PartyMember / UnitAll / Itemowner / SpecialInfo / ViewerDependent` flags. Rust: **no UpdateMask dirty tracking at all.** `crates/wow-packet/src/packets/update.rs` (3072 lines) hand-rolls full create blocks per-type — `PlayerCreateData::write_values_create` (`:268`), `CreatureCreateData` (`:1060`), `GameObjectCreateData` (`:1295`), `UpdateObject::ServerPacket` (`:1714`) — but **every replication is a full create block; there is no values-only delta path, no per-viewer field filtering, no dirty bit on field write**. `wow-constants/src/update.rs:24` defines an `UpdateFieldFlag` bitflags wrapper but nothing reads it. **This is the worst divergence in the entire entities surface: every replication update will be a full re-create, which will visibly desync stats/auras and grow the bandwidth ~10-100×.** Note: the `flags = 0x03` (Owner|PartyMember) write at `update.rs:274` is hardcoded for self vs others — there is no per-viewer "is in same raid" computation.

### ObjectGuid wire format

`crates/wow-core/src/guid.rs:112` `ObjectGuid` is 128-bit, has `HighGuid` enum (Player/Creature/Pet/Item/GameObject/DynamicObject/Corpse/AreaTrigger/Transport/Party/Guild/SceneObject/Conversation/Cast — full coverage at `:7`), `to_raw_bytes`/`from_raw_bytes` round-trip (tested at `:719`), `is_player`/`is_creature`/etc. predicates. Packed-GUID 1-byte mask + variable-length form lives in `wow-packet/src/world_packet.rs:422` with 4 round-trip tests. **GUID layer is the single solid foundation in the entities mega-module — coverage ~70%, no critical wire-format divergence detected.**

### Worst divergence

**`UpdateMask` / dirty-bit replication is entirely missing.** Every entity broadcast goes out as a full create block instead of a delta values block. Combined with the absence of `Player::Update(diff)` orchestration and the lack of any `Unit` / `Item` / `GameObject` / `Pet` / `DynamicObject` types, the Rust port currently has **no entity layer in the C++ sense** — only a session-with-flat-fields façade and a creature record in `MapManager`. Roughly 95% of the ~95k C++ Entities/ lines have no Rust counterpart. Migration effort estimate: 6-12 person-months to reach behavioral parity for the WoLK 3.4.3 feature set.
