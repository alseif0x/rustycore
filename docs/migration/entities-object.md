# Migration: Entities / Object (base + UpdateMask infrastructure)

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/`
> **Rust target crate(s):** `crates/wow-core/` (GUID, Position partial), `crates/wow-world/` (lifecycle, UpdateMask — both **absent**), `crates/wow-packet/` (wire format, partial)
> **Layer:** L4 (sub-modules — rooted under `entities.md`)
> **Status:** 🔧 broken (rewrite needed)
> **Audited vs C++:** ✅ complete (2026-05-01)
> **Last updated:** 2026-05-01

**Parent doc:** [`entities.md`](entities.md). **Sibling sub-docs:** [`entities-unit.md`](entities-unit.md) · [`entities-vehicle.md`](entities-vehicle.md) · [`entities-transport.md`](entities-transport.md). **Cross-ref for global registries (`ObjectMgr`, `ObjectAccessor`):** [`globals.md`](globals.md).

---

## 1. Purpose

`Object` is the *root* of every replicated thing in the world: it pairs a stable identity (`ObjectGuid`) with a polymorphic, dirty-tracked field blob (`m_values`) that is broadcast to clients via `SMSG_UPDATE_OBJECT`. Every Creature, Player, GameObject, Item, DynamicObject, AreaTrigger, Corpse, SceneObject, and Conversation inherits from it. `WorldObject` (a separate mid-tier class derived from `Object` + `WorldLocation`) adds map/phase/zone state, visibility, and the `Add/RemoveFromWorld` lifecycle.

The accompanying **UpdateMask / UpdateField** subsystem (`Updates/`) is the wire-replication primitive: a write to any tracked field flips a bit in a per-object `UpdateMask`, and on the next visibility tick TrinityCore emits only the dirty blocks. This is the single highest-volume server-to-client path in the protocol; **getting it wrong manifests as everything from invisible mobs to leaked health values to faction-color desync.**

This sub-doc covers exclusively the `Object/` directory. Subclasses (`Unit`, `Player`, `Creature`, `GameObject`, `Item`, etc.) live in their own sub-docs.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Entities/Object/G3DPosition.hpp` | 29 | `prefix` |
| `game/Entities/Object/GridObject.h` | 37 | `prefix` |
| `game/Entities/Object/MovementInfo.h` | 204 | `prefix` |
| `game/Entities/Object/Object.cpp` | 3798 | `prefix` |
| `game/Entities/Object/Object.h` | 845 | `prefix` |
| `game/Entities/Object/ObjectDefines.h` | 122 | `prefix` |
| `game/Entities/Object/ObjectGuid.cpp` | 811 | `prefix` |
| `game/Entities/Object/ObjectGuid.h` | 492 | `prefix` |
| `game/Entities/Object/ObjectPosSelector.cpp` | 152 | `prefix` |
| `game/Entities/Object/ObjectPosSelector.h` | 154 | `prefix` |
| `game/Entities/Object/Position.cpp` | 209 | `prefix` |
| `game/Entities/Object/Position.h` | 223 | `prefix` |
| `game/Entities/Object/SmoothPhasing.cpp` | 67 | `prefix` |
| `game/Entities/Object/SmoothPhasing.h` | 58 | `prefix` |
| `game/Entities/Object/Updates/UpdateData.cpp` | 72 | `prefix` |
| `game/Entities/Object/Updates/UpdateData.h` | 67 | `prefix` |
| `game/Entities/Object/Updates/UpdateField.cpp` | 63 | `prefix` |
| `game/Entities/Object/Updates/UpdateField.h` | 991 | `prefix` |
| `game/Entities/Object/Updates/UpdateFields.cpp` | 5097 | `prefix` |
| `game/Entities/Object/Updates/UpdateFields.h` | 943 | `prefix` |
| `game/Entities/Object/Updates/UpdateMask.h` | 164 | `prefix` |
| `game/Entities/Object/Updates/ViewerDependentValues.h` | 367 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

### 2.1 Core hierarchy

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Entities/Object/Object.h` | 845 | `Object` + `WorldObject` declarations; `CreateObjectBits`; `FindCreatureOptions`; virtual lifecycle hooks |
| `src/server/game/Entities/Object/Object.cpp` | 3798 | Lifecycle (`AddToWorld`/`RemoveFromWorld`), update-block builders (`BuildCreateUpdateBlockForPlayer`, `BuildValuesUpdateBlockForPlayer`), visibility queries, summon factories |
| `src/server/game/Entities/Object/ObjectGuid.h` | 492 | `ObjectGuid` 128-bit layout; `HighGuid` enum (~50 variants); `TypeID` (14); `TypeMask`; hash impls |
| `src/server/game/Entities/Object/ObjectGuid.cpp` | 811 | GUID parsing, packed serialization, generator helpers (`ObjectGuidGenerator`) |
| `src/server/game/Entities/Object/ObjectDefines.h` | 122 | `MAX_AGGRO_RADIUS`, `DEFAULT_VISIBILITY_DISTANCE`, type masks |
| `src/server/game/Entities/Object/Position.h` | 223 | `Position`, `WorldLocation`, distance/arc/angle helpers |
| `src/server/game/Entities/Object/Position.cpp` | 209 | Position math implementations |
| `src/server/game/Entities/Object/G3DPosition.hpp` | 80 | Conversions to/from G3D `Vector3`/`Vector4` for VMap |
| `src/server/game/Entities/Object/MovementInfo.h` | 204 | `MovementInfo` POD: flags, transport, swim pitch, fall time, jump |
| `src/server/game/Entities/Object/GridObject.h` | 37 | Intrusive `GridReference` mixin for grid linkage |
| `src/server/game/Entities/Object/ObjectPosSelector.h` | 154 | Helper to find non-overlapping nearby positions |
| `src/server/game/Entities/Object/ObjectPosSelector.cpp` | 152 | Position selector implementation |
| `src/server/game/Entities/Object/SmoothPhasing.h` | 58 | Phase-shift smooth fade-in metadata |
| `src/server/game/Entities/Object/SmoothPhasing.cpp` | 67 | SmoothPhasing impl |

### 2.2 Updates/ (replication primitives — the load-bearing missing piece)

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Entities/Object/Updates/UpdateData.h` | 67 | `UpdateData` packet builder accumulating create / values / destroy blocks |
| `src/server/game/Entities/Object/Updates/UpdateData.cpp` | 72 | `UpdateData::BuildPacket` (compresses + writes `SMSG_UPDATE_OBJECT` / `SMSG_COMPRESSED_UPDATE_OBJECT`) |
| `src/server/game/Entities/Object/Updates/UpdateField.h` | 991 | `UpdateField<T>`, `UpdateFieldArray<T,N>`, `DynamicUpdateField<T>`, `OptionalUpdateField<T>`, `UpdateFieldFlag`, setter wrappers |
| `src/server/game/Entities/Object/Updates/UpdateField.cpp` | 63 | UpdateField helpers |
| `src/server/game/Entities/Object/Updates/UpdateFields.h` | 943 | DECLARED structs for `ObjectData`, `UnitData`, `PlayerData`, `ActivePlayerData`, `ItemData`, `ContainerData`, `GameObjectData`, `DynamicObjectData`, `CorpseData`, `AreaTriggerData`, `SceneObjectData`, `ConversationData` (~1500 logical fields) |
| `src/server/game/Entities/Object/Updates/UpdateFields.cpp` | 5097 | Field write/read impls + descriptor tables consumed by `BuildValuesUpdate` |
| `src/server/game/Entities/Object/Updates/UpdateMask.h` | 164 | `UpdateMask<Bits>` template: 32-bit blocks + blocks-mask, `Set`/`Reset`/`SetAll`/`ResetAll`/`IsAnySet`/`operator&=`/`operator|=` |
| `src/server/game/Entities/Object/Updates/ViewerDependentValues.h` | 367 | Per-viewer field overrides (faction-relative, group-relative, owner-only) |

**Total in Object/:** ~13,800 lines, of which **~7,200 are the Updates/ replication subsystem** that has zero analogue in Rust.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Object` | class (abstract) | Root: `m_objectTypeId`, `m_objectType` mask, `m_guid`, `m_values` (UpdateFields blob), `m_inWorld`, `m_isNewObject`, `m_isDestroyedObject` |
| `WorldObject` | class : `Object`, `WorldLocation` | Adds map/phase/zone/area/visibility, `EventProcessor m_Events`, `Acore::AnyData` helpers |
| `WorldLocation` | struct : `Position` | `Position` + `m_mapId` |
| `Position` | struct | `x`, `y`, `z`, `o` (orientation) + distance/arc/angle math |
| `MovementInfo` | struct | Position + flags + transport + swim/fall + jump (wire-format movement payload) |
| `ObjectGuid` | struct (128-bit) | `HighGuid` (6 bits) + low/realm/server/map/entry/counter packed into 16 bytes |
| `ObjectGuidGenerator<HighGuid>` | template | Per-HighGuid sequential counter |
| `HighGuid` | enum class : `uint8` | ~50 variants: `Null`, `Player`, `Item`, `Transport`, `Conversation`, `Creature`, `Vehicle`, `Pet`, `GameObject`, `DynamicObject`, `AreaTrigger`, `Corpse`, `LootObject`, `SceneObject`, `Scenario`, `Party`, `Guild`, `WowAccount`, `Spell`, `Mail`, `BattlePet`, ... |
| `TypeID` | enum : `uint8` (0..13) | `TYPEID_OBJECT`, `_ITEM`, `_CONTAINER`, `_AZERITE_*`, `_UNIT`, `_PLAYER`, `_ACTIVE_PLAYER`, `_GAMEOBJECT`, `_DYNAMICOBJECT`, `_CORPSE`, `_AREATRIGGER`, `_SCENEOBJECT`, `_CONVERSATION` |
| `TypeMask` | enum : `uint16` | Bit mask: `TYPEMASK_OBJECT/ITEM/CONTAINER/UNIT/PLAYER/...`; `TYPEMASK_SEER = PLAYER\|UNIT\|DYNAMICOBJECT` |
| `CreateObjectBits` | struct | 18 bit-flags for `BuildCreateUpdateBlockForPlayer` (NoBirthAnim, EnablePortals, PlayHoverAnim, MovementUpdate, MovementTransport, Stationary, CombatVictim, ServerTime, Vehicle, AnimKit, Rotation, AreaTrigger, GameObject, SmoothPhasing, ThisIsYou, SceneObject, ActivePlayer, Conversation) |
| `UpdateData` | class | Accumulator: `m_blockCount`, `m_data` ByteBuffer, `m_outOfRangeGUIDs` set, `m_destroyGUIDs` set; emits `SMSG_UPDATE_OBJECT` |
| `UpdateMask<Bits>` | template class | Two-level bitmask: `BlockCount = ceil(Bits/32)`, `BlocksMaskCount = ceil(BlockCount/32)`; `Set`/`Reset`/`SetAll`/`ResetAll`/`IsAnySet`/`operator&=`/`operator\|=` |
| `UpdateField<T>` | template | Wrapper that flips a dirty bit on `set()` |
| `UpdateFieldArray<T, N>` | template | Fixed-size array of tracked values |
| `DynamicUpdateField<T>` | template | Resizable tracked vector |
| `OptionalUpdateField<T>` | template | Tracked `Optional` |
| `UpdateFieldFlag` | enum : `uint8` | `None / Owner / PartyMember / UnitAll / ItemOwner / SpecialInfo / ViewerDependent` — filters which fields each viewer receives |
| `UF::ObjectData` | struct | Common fields: `EntryID`, `DynamicFlags`, `Scale`, `Guid` |
| `UF::UnitData` / `PlayerData` / `ActivePlayerData` / `ItemData` / `ContainerData` / `GameObjectData` / `DynamicObjectData` / `CorpseData` / `AreaTriggerData` / `SceneObjectData` / `ConversationData` | structs | Per-type field bundles (covered in respective sub-docs) |
| `ViewerDependentValue<F>` | template | Per-viewer override hooks for fields like `UnitData::Bytes2` (faction colors), `UnitData::Health` (group sees percent) |
| `SmoothPhasing` | struct | Per-viewer fade-in/out phase metadata |
| `FindCreatureOptions` / `FindGameObjectOptions` | struct | `Acore::AnyOf` predicate bundle for nearby-search APIs |
| `ObjectPosSelector` | class | Non-overlapping summon placement around an anchor |

---

## 4. Critical public methods / functions

### 4.1 `Object`

| Symbol | Purpose | Calls into |
|---|---|---|
| `Object::AddToWorld()` (virtual) | `m_inWorld = true`; `ASSERT(!m_inWorld)` enforces single-add | derived overrides; called by `Map::AddToMap` |
| `Object::RemoveFromWorld()` (virtual) | Tear-down hook; clears `m_inWorld` | derived overrides; called by `Map::RemoveFromMap` |
| `Object::IsInWorld() const` | Load-bearing predicate (gated by ASSERT in thousands of call sites) | — |
| `Object::GetGUID() const` | Stable identity | — |
| `Object::GetTypeId() const` / `isType(mask)` | Polymorphic type tag without RTTI | — |
| `Object::BuildCreateUpdateBlockForPlayer(UpdateData*, Player*)` | Emits `UPDATETYPE_CREATE_OBJECT` block (full state for newly-visible target) | `BuildMovementUpdate`, `BuildValuesUpdate` |
| `Object::BuildValuesUpdateBlockForPlayer(UpdateData*, Player*)` | Emits `UPDATETYPE_VALUES` (only dirty fields) | `BuildValuesUpdate` (virtual per-type) |
| `Object::BuildValuesUpdateBlockForPlayerWithFlag(UpdateData*, UF::UpdateFieldFlag, Player*)` | Forces a subset by flag (used on owner-flag change) | — |
| `Object::BuildOutOfRangeUpdateBlock(UpdateData*)` | Marks object for client destroy on next batch | — |
| `Object::BuildDestroyUpdateBlock(UpdateData*)` | Variant: hard destroy | — |
| `Object::SendUpdateToPlayer(Player*)` | One-shot push of own state to a single viewer | `BuildCreateUpdateBlockForPlayer`, `Player::SendDirectMessage` |
| `Object::DestroyForPlayer(Player*)` (virtual) | Sends `SMSG_DESTROY_OBJECT` | — |
| `Object::ClearUpdateMask(bool remove)` (virtual) | Resets dirty bits after a broadcast tick | `m_values.ClearChangesMask` |
| `Object::SetEntry/GetEntry`, `SetObjectScale/GetObjectScale`, `SetDynamicFlag/HasDynamicFlag` | Common-field accessors backed by `UF::ObjectData` | `SetUpdateFieldValue` |

### 4.2 `WorldObject`

| Symbol | Purpose | Calls into |
|---|---|---|
| `WorldObject::AddToWorld()` (override) | Updates grid linkage + visibility | `Map::AddToActive`, `UpdateObjectVisibility` |
| `WorldObject::RemoveFromWorld()` (override) | Removes from grid, destroys for nearby | `Map::RemoveFromActive`, `DestroyForNearbyPlayers` |
| `WorldObject::Update(uint32 diff)` (virtual) | Per-tick update (movement, events) | derived overrides; `m_Events.Update` |
| `WorldObject::GetMap()` / `GetMapId()` | Owning map accessor | — |
| `WorldObject::GetPosition()` / `Relocate(...)` | Position read/write | `UpdateObjectVisibility` |
| `WorldObject::UpdateObjectVisibility(bool forced)` | Recomputes visible-to set vs nearby Players | `Map::PlayerRelocation`, `Trinity::AIRelocationNotifier` |
| `WorldObject::IsWithinDistInMap(...)` / `IsWithinDist3d` / `IsWithinDist2d` | Same-map distance checks | `Position::distance` |
| `WorldObject::IsWithinLOS(float x,y,z, ...)` | Line-of-sight via VMap | `VMapMgr2::isInLineOfSight` |
| `WorldObject::SummonCreature(uint32 entry, Position const&, ...)` | Spawn a `TempSummon` on this map | `Map::SummonCreature`, `TempSummon::InitSummon` |
| `WorldObject::SummonGameObject(...)` | Spawn a GO | `Map::SummonGameObject` |
| `WorldObject::SetPhaseShift(...)` | Phase shift; client visibility re-sync | `UpdateObjectVisibility` |
| `WorldObject::DestroyForNearbyPlayers()` | Client cleanup on remove | iterate visible players |
| `WorldObject::GetNameForLocaleIdx(LocaleConstant)` (pure virtual) | Localized display name | derived |

### 4.3 `UpdateData` / `UpdateMask`

| Symbol | Purpose | Calls into |
|---|---|---|
| `UpdateData::AddUpdateBlock()` | Increments block count for ByteBuffer | — |
| `UpdateData::AddOutOfRangeGUID(ObjectGuid)` | Queues destroy notice | — |
| `UpdateData::AddDestroyObject(ObjectGuid)` | Queues hard destroy | — |
| `UpdateData::BuildPacket(WorldPacket*)` | Compresses (zlib) when over threshold; writes `SMSG_UPDATE_OBJECT` or `SMSG_COMPRESSED_UPDATE_OBJECT` | — |
| `UpdateData::Clear()` | Reset accumulator | — |
| `UpdateMask<Bits>::Set(uint32 index)` | Mark field dirty (also flips parent block-mask bit) | — |
| `UpdateMask<Bits>::Reset(uint32 index)` | Clear bit; clears parent block-mask bit if last | — |
| `UpdateMask<Bits>::SetAll()` / `ResetAll()` | Bulk operations (used on create vs. clean tick) | — |
| `UpdateMask<Bits>::IsAnySet() const` | Has-any-dirty predicate | — |
| `UpdateMask<Bits>::operator&=` / `operator|=` | Mask combinators (used for viewer-flag filtering) | — |
| `UpdateField<T>::operator=` / setter | Writes value AND flips dirty bit | — |
| `SetUpdateFieldValue(setter, value)` (free fn) | Generic write helper | — |
| `SetUpdateFieldFlagValue(setter, flag)` | OR a flag into a tracked bitfield | — |
| `RemoveUpdateFieldFlagValue(setter, flag)` | AND-NOT a flag | — |

### 4.4 `ObjectGuid` / `Position`

| Symbol | Purpose | Calls into |
|---|---|---|
| `ObjectGuid::Create<HighGuid>(args...)` | Construct a GUID with the right field layout per-type | — |
| `ObjectGuid::IsEmpty()` / `IsAnyTypeCreature()` / `IsPlayer()` etc. | Type predicates | — |
| `ObjectGuid::WriteAsPacked(ByteBuffer&)` / `ReadAsPacked(...)` | Wire-format compact GUID encoding | — |
| `ObjectGuid::GetCounter()` / `GetEntry()` / `GetMapId()` / `GetRealmId()` | Sub-field extraction (per HighGuid layout) | — |
| `Position::GetExactDist2d/3d(...)` / `IsInDist(...)` / `HasInArc(...)` / `GetAngle(...)` / `RelocateOffset(...)` | Spatial math | — |
| `WorldLocation::WorldRelocate(uint32 mapId, float x,y,z,o)` | Cross-map relocate primitive | — |

---

## 5. Module dependencies

**Depends on:**
- `shared/` (`Define.h`, `Errors.h`, `EventProcessor.h`, `Optional.h`, `Duration.h`)
- `Maps/` — `Map*` is held by `WorldObject`; `AddToWorld` is invoked from `Map::AddToMap`
- `Grids/` — `GridObject<T>` mixin; visibility iteration via grid notifiers
- `MovementInfo` consumers in `Movement/` (spline, motion master)
- `PhaseShift` (`src/server/game/Phasing/`) — held inside `WorldObject`
- `DataStores` — `MapEntry`, `AreaTableEntry` for zone/area resolution
- `World` — `sWorld->getRate(RATE_*)` for visibility distance configuration

**Depended on by:** **everything**. Every entity sub-class (`Unit`, `Player`, `Creature`, `Pet`, `GameObject`, `Item`, `Bag`, `DynamicObject`, `AreaTrigger`, `Conversation`, `Corpse`, `SceneObject`, `Vehicle`, `Transport`) inherits from `Object` or `WorldObject`. The `WorldSession` packet handlers in `src/server/game/Handlers/*Handler.cpp` operate on `Object*`/`Player*`/`Unit*` pointers obtained via `ObjectAccessor`. Every `SMSG_*` originating from the server passes through `UpdateData` for state replication.

---

## 6. SQL / DB queries (if any)

`Object/` itself emits no SQL — persistence lives in subclasses. It does, however, consume DBC/DB2 stores for the data its fields reference.

| Store | What it loads | Read by |
|---|---|---|
| `MapStore` | `Map.db2` | `WorldObject::GetMapId` / `MapEntry` lookups |
| `AreaTableStore` | `AreaTable.db2` | `WorldObject::GetZoneAndAreaId` |
| `FactionTemplateStore` | `FactionTemplate.db2` | `WorldObject::IsHostileTo` (via virtual `GetFaction`) |
| `PhaseStore` / `PhaseXPhaseGroupStore` | `Phase.db2`, `PhaseXPhaseGroup.db2` | `WorldObject::SetPhaseShift` |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_UPDATE_OBJECT` | server → client | `UpdateData::BuildPacket` (called from `Map::SendInitialVisiblePackets`, `Object::SendUpdateToPlayer`, `WorldObject::UpdateObjectVisibility`) |
| `SMSG_COMPRESSED_UPDATE_OBJECT` | server → client | `UpdateData::BuildPacket` zlib path (when payload > threshold) |
| `SMSG_DESTROY_OBJECT` | server → client | `Object::DestroyForPlayer`, `Object::SendOutOfRangeForPlayer` |

The wire format is essentially: per-object `(updateType: u8, packed_guid, mask_block_count: u8, blocks: [u32], dirty_field_values: ...)` where the field layout is dictated by `UpdateFields.h` (per-type struct, ~1500 logical fields total across all object types). The two-level `UpdateMask` (block-mask of which 32-bit blocks have any set bit, then per-block bitmasks) is what is actually serialized — clients walk it the same way.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-core` | `crate_dir` | 4 | 1153 | `exists_active` | crate exists |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `crates/wow-core/src/guid.rs` | `file` | 1 | 790 | `exists_active` | file exists |
| `crates/wow-core/src/position.rs` | `file` | 1 | 190 | `exists_active` | file exists |
| `crates/wow-core/src/lib.rs` | `file` | 1 | 7 | `exists_active` | file exists |
| `crates/wow-packet/src/world_packet.rs` | `file` | 1 | 673 | `exists_active` | file exists |
| `crates/wow-packet/src/update.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-ai` | `crate_dir` | 1 | 346 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**

| File | Lines | Coverage of C++ Object/ |
|---|---|---|
| `crates/wow-core/src/guid.rs` | 790 | `ObjectGuid`, `HighGuid`, `TypeId` — covers ~70% of `ObjectGuid.h/.cpp`. Pack/unpack to bytes; counter helpers; per-HighGuid sub-field extraction. |
| `crates/wow-core/src/position.rs` | 190 | `Position` w/ distance, arc, `point_at_distance` — ~50% of `Position.h/.cpp`. **Missing:** `WorldLocation` (no map/phase pairing). |
| `crates/wow-core/src/lib.rs` | 7 | re-exports |
| `crates/wow-packet/src/world_packet.rs` | (within wow-packet) | Bit-packing primitives + packed-GUID helpers |
| `crates/wow-packet/src/update.rs` | (within wow-packet) | `UpdateObject`, `CreatureCreateData`, `PlayerCreateData` packet builders — **hand-rolled per opcode, not driven by an `UpdateMask`**. No `UpdateData` accumulator. |

**What's implemented:**
- `ObjectGuid` 128-bit layout with `HighGuid` discriminant; pack/unpack to bytes; sequential counter helpers.
- `Position` with distance/arc/`point_at_distance`. Tested (6 unit tests).
- Hand-built `UpdateObject` packets in `wow-packet::update` for the ~3 cases the server currently emits (creature create, player create, partial movement).

**What's missing vs C++:**
- **`Object` struct.** No root entity type at all. Every entity-like thing (the player flat fields on `WorldSession`, the `WorldCreature` in `MapManager`, `CreatureAI` in `wow-ai`) implements identity ad-hoc.
- **`WorldObject` struct.** No `WorldLocation` (map_id + position). `Position` exists in isolation.
- **`m_inWorld` / `AddToWorld` / `RemoveFromWorld` lifecycle.** No invariant boundary; entities are inserted into the `MapManager` and removed without a uniform contract. `IsInWorld()` ASSERTs that gate thousands of C++ paths simply do not exist.
- **`UpdateMask` system entirely.** No bit-mask, no `UpdateField<T>` wrapper, no dirty-bit tracking. Every state write goes directly to fields; the next packet re-serializes from scratch instead of emitting a delta.
- **`UpdateFieldFlag` viewer filtering.** `Owner / PartyMember / UnitAll / ItemOwner / SpecialInfo / ViewerDependent` — none. Health, faction-colors, inventory slots, talent points, etc. are either always sent to everyone (privacy/accuracy bug) or never sent (UI breakage).
- **`UpdateData` accumulator.** No batched per-tick per-viewer aggregator; no compressed variant; no destroy-list channel.
- **`CreateObjectBits` (18 sub-flags).** Hand-rolled `CreatureCreateData` covers a fraction of these; flags like `ServerTime`, `ThisIsYou`, `ActivePlayer`, `SmoothPhasing`, `Conversation` are not represented.
- **`ViewerDependentValues<T>` per-viewer overrides.** Faction-relative colors, group-share health-percent, owner-only inventory: all absent.
- **`MovementInfo`** unified parsing/serialization. Partial in `wow-packet`, not unified per entity.
- **`WorldObject::UpdateObjectVisibility`.** No notion of visibility-set diffing on relocate; the current `MapManager::get_visible_creatures` 3×3-grid window is a coarse stand-in.
- **`WorldObject::SummonCreature/SummonGameObject` factories.** No GameObject summoning at all; creature spawning bypasses the entity-summon contract.
- **`SmoothPhasing`.** Not modeled.
- **`ObjectPosSelector`.** Not modeled (will manifest as overlapping summons).

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- `wow_core::ObjectGuid` exposes realm/server/map/entry sub-fields for every variant, but the C++ layout uses different fields per HighGuid (e.g. `Transport` carries map+counter, `GameObject` carries map+entry+counter, `Item` carries realm+counter). Round-trip parity for non-Player/Creature variants is unverified.
- `wow_core::Position` exposes `.x .y .z .orientation` (project convention per `AGENTS.md`); C++ uses `.GetPositionX/Y/Z/O()` accessors that resolve to the same fields, but any `point_at_distance` math should be byte-compared against C++ output before trusting.
- `wow-packet::update::UpdateObject` likely diverges from `SMSG_UPDATE_OBJECT` wire format in 3.4.3 because the field index/offset layout is generated from `UpdateFields.cpp` descriptor tables that we have not ported.
- The `WorldCreature::take_damage(u32) -> bool` method (`map_manager.rs:176`) silently writes `current_hp` without flipping any dirty bit; clients can only learn about HP changes if some unrelated handler explicitly emits a packet — there is no automatic replication trigger.

**Tests existing:**
- `crates/wow-core/src/position.rs` — 6 unit tests (distance, arc, zero-position).
- `crates/wow-core/src/guid.rs` — pack/unpack tests for the common HighGuid variants.
- **Zero tests** for `UpdateMask`, `UpdateField`, `UpdateData`, viewer-flag filtering, `WorldObject` lifecycle, or visibility diffing — because none of those types exist.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#ENTITIES_OBJECT.WBS.001** Cerrar la migracion auditada de `game/Entities/Object/G3DPosition.hpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/G3DPosition.hpp`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.002** Cerrar la migracion auditada de `game/Entities/Object/GridObject.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/GridObject.h`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.003** Cerrar la migracion auditada de `game/Entities/Object/MovementInfo.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/MovementInfo.h`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.004** Partir y cerrar la migracion auditada de `game/Entities/Object/Object.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Object.cpp`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 3798 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.005** Partir y cerrar la migracion auditada de `game/Entities/Object/Object.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Object.h`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 845 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.006** Cerrar la migracion auditada de `game/Entities/Object/ObjectDefines.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/ObjectDefines.h`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.007** Partir y cerrar la migracion auditada de `game/Entities/Object/ObjectGuid.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/ObjectGuid.cpp`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 811 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.008** Cerrar la migracion auditada de `game/Entities/Object/ObjectGuid.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/ObjectGuid.h`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.009** Cerrar la migracion auditada de `game/Entities/Object/ObjectPosSelector.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/ObjectPosSelector.cpp`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.010** Cerrar la migracion auditada de `game/Entities/Object/ObjectPosSelector.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/ObjectPosSelector.h`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.011** Cerrar la migracion auditada de `game/Entities/Object/Position.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Position.cpp`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.012** Cerrar la migracion auditada de `game/Entities/Object/Position.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Position.h`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.013** Cerrar la migracion auditada de `game/Entities/Object/SmoothPhasing.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/SmoothPhasing.cpp`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.014** Cerrar la migracion auditada de `game/Entities/Object/SmoothPhasing.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/SmoothPhasing.h`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.015** Cerrar la migracion auditada de `game/Entities/Object/Updates/UpdateData.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/UpdateData.cpp`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.016** Cerrar la migracion auditada de `game/Entities/Object/Updates/UpdateData.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/UpdateData.h`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.017** Cerrar la migracion auditada de `game/Entities/Object/Updates/UpdateField.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/UpdateField.cpp`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.018** Partir y cerrar la migracion auditada de `game/Entities/Object/Updates/UpdateField.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/UpdateField.h`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 991 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.019** Partir y cerrar la migracion auditada de `game/Entities/Object/Updates/UpdateFields.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/UpdateFields.cpp`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 5097 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.020** Partir y cerrar la migracion auditada de `game/Entities/Object/Updates/UpdateFields.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/UpdateFields.h`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 943 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.021** Cerrar la migracion auditada de `game/Entities/Object/Updates/UpdateMask.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/UpdateMask.h`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_OBJECT.WBS.022** Cerrar la migracion auditada de `game/Entities/Object/Updates/ViewerDependentValues.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/ViewerDependentValues.h`
  Rust target: `crates/wow-core`, `crates/wow-world`, `crates/wow-packet`, `crates/wow-ai`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numbered as `OBJECT.x` for cross-reference from `MIGRATION_ROADMAP.md` §5. Complexity: **L** (<1h), **M** (1–4h), **H** (4–12h), **XL** (>12h, split).

- [ ] **#OBJECT.1** Create `crates/wow-entities/` workspace crate; depend on `wow-core`, `wow-data`, `wow-constants`. (L)
- [ ] **#OBJECT.2** Define `TypeId` (14 variants) and `TypeMask` (16-bit) enums; constants in `wow-constants`. (L)
- [ ] **#OBJECT.3** Define `WorldLocation { position: Position, map_id: u32, phase_mask: u32 }` in `wow-core::position`; thread through call sites. (M)
- [ ] **#OBJECT.4** Decide polymorphism strategy: `trait Entity` for shared interface (`guid`, `type_id`, `add_to_world`, `remove_from_world`, `is_in_world`, `update`) + concrete structs composed via `Object { ... }` / `WorldObject { object: Object, location: WorldLocation }`. Document explicitly that we are NOT mirroring C++ inheritance. (M)
- [ ] **#OBJECT.5** Implement `Object` struct: `guid`, `type_id`, `type_mask`, `in_world: bool`, `is_new_object: bool`, `is_destroyed: bool`, `values: ObjectValues` (the dirty-tracked blob from #OBJECT.8). Add `debug_assert!(!self.in_world)` on `add_to_world` to mirror the C++ `ASSERT`. (M)
- [ ] **#OBJECT.6** Implement `WorldObject` struct: embed `Object`, add `WorldLocation`, `visibility_distance`, `phase_mask`, `event_processor`. Implement `add_to_world` / `remove_from_world` chain that calls `Object`'s version. (M)
- [ ] **#OBJECT.7** Port `MovementInfo` (`flags`, `flags2`, `time`, `position`, `transport: Option<TransportData>`, `pitch`, `fall_time`, `jump`); round-trip (de)serialize against captured 3.4.3 wire bytes. (M)
- [ ] **#OBJECT.8** Implement `UpdateMask`: two-level (block-mask of 32-bit blocks + per-block 32-bit bitmasks). Port `Set/Reset/SetAll/ResetAll/IsAnySet/operator&=/operator|=`. Generic over `const BITS: u32` (or runtime-sized `BitVec` if generics are awkward). (H)
- [ ] **#OBJECT.9** Implement `UpdateField<T>`: a `Tracked<T>` wrapper that holds `value: T`, `bit_index: u16`, and a back-reference (or owning context) to the parent `UpdateMask`. `set(&mut self, ctx: &mut UpdateMaskCtx, v: T)` flips the dirty bit. (H)
- [ ] **#OBJECT.10** Implement `UpdateFieldArray<T, const N: usize>`, `DynamicUpdateField<T>` (Vec-backed, length-tracking), `OptionalUpdateField<T>`. (M)
- [ ] **#OBJECT.11** Define `UpdateFieldFlag` (`None / Owner / PartyMember / UnitAll / ItemOwner / SpecialInfo / ViewerDependent`); attach as a per-field metadata table; build mask combinators that filter by viewer-relationship. (M)
- [ ] **#OBJECT.12** Port `UF::ObjectData` (the 4 common fields: `EntryID`, `DynamicFlags`, `Scale`, `Guid`) as the first end-to-end `UpdateField<T>` consumer; wire into `Object` struct. (M)
- [ ] **#OBJECT.13** Implement `UpdateData` accumulator: `add_create_block`, `add_values_block`, `add_destroy_guid`, `add_out_of_range_guid`, `build_packet` → `SMSG_UPDATE_OBJECT` (or `SMSG_COMPRESSED_UPDATE_OBJECT` over the configurable size threshold). (H)
- [ ] **#OBJECT.14** Implement `Object::build_create_update_block_for_player(&UpdateData, &Player)` (`UPDATETYPE_CREATE_OBJECT`). (H)
- [ ] **#OBJECT.15** Implement `Object::build_values_update_block_for_player(&UpdateData, &Player)` and `..._with_flag` (`UPDATETYPE_VALUES`). (H)
- [ ] **#OBJECT.16** Implement `Object::build_destroy_update_block` and `Object::send_out_of_range_for_player`. Emit `SMSG_DESTROY_OBJECT`. (M)
- [ ] **#OBJECT.17** Implement `Object::clear_update_mask(remove: bool)` to reset dirty bits at the end of the broadcast tick. (L)
- [ ] **#OBJECT.18** Implement `WorldObject::update_object_visibility(forced: bool)` integrated with `MapManager` grid relocation; emit create-blocks for newly-visible, out-of-range for newly-hidden. (H)
- [ ] **#OBJECT.19** Wire `Map::add_to_map` / `Map::remove_from_map` so they invoke `entity.add_to_world()` / `remove_from_world()` symmetrically. Add `debug_assert!` invariants matching C++ ASSERTs. (H)
- [ ] **#OBJECT.20** Replace `WorldCreature::take_damage(u32) -> bool` (`map_manager.rs:176`) with a path that goes through a tracked HP `UpdateField<u64>` so dirty bits flip and the next visibility tick replicates the change. (M, but blocks Unit migration)
- [ ] **#OBJECT.21** Port `ViewerDependentValues<F>` for at minimum `UnitData::Bytes2` (faction colors), `UnitData::Health` (group-share percent), `PlayerData::InvSlots` (owner-only). (H)
- [ ] **#OBJECT.22** Port `SmoothPhasing` per-viewer fade metadata; integrate with phase-mask change. (M)
- [ ] **#OBJECT.23** Code-generate the per-type `UF::*Data` field structs from `UpdateFields.h` (XL — split per-type and tracked under each sub-doc). The `Object/` doc covers only `ObjectData`; `UnitData`/`PlayerData`/`ItemData`/`GameObjectData`/etc. land in their respective sub-docs.

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#ENTITIES_OBJECT.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 22 files / 14965 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/UpdateFields.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Object.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/UpdateField.h`. Rust target: `crates/wow-ai`, `crates/wow-core`, `crates/wow-packet`, `crates/wow-world`. | `cargo test -p wow-ai && cargo test -p wow-core && cargo test -p wow-packet` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#ENTITIES_OBJECT.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 22 files / 14965 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/UpdateFields.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Object.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/UpdateField.h`. Rust target: `crates/wow-ai`, `crates/wow-core`, `crates/wow-packet`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#ENTITIES_OBJECT.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 22 files / 14965 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/UpdateFields.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Object.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/UpdateField.h`. Rust target: `crates/wow-ai`, `crates/wow-core`, `crates/wow-packet`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#ENTITIES_OBJECT.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 22 files / 14965 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/UpdateFields.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Object.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/UpdateField.h`. Rust target: `crates/wow-ai`, `crates/wow-core`, `crates/wow-packet`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `Object::add_to_world` then `remove_from_world` invariants — second `add` panics; `remove` without prior `add` panics; `is_in_world` matches.
- [ ] Test: `ObjectGuid` round-trip through 16-byte wire format for every `HighGuid` variant (`Player`, `Creature`, `Pet`, `GameObject`, `Item`, `Vehicle`, `Transport`, `Conversation`, `Corpse`, `AreaTrigger`, `DynamicObject`).
- [ ] Test: `ObjectGuid::write_as_packed` + `read_as_packed` round-trip matches C++ output (golden bytes).
- [ ] Test: `Position` distance/arc/`has_in_arc`/`point_at_distance` match TC C++ output within 1e-4 (golden table).
- [ ] Test: `WorldLocation::world_relocate` updates `map_id` and `position` together; bare `Position::set` does not.
- [ ] Test: `UpdateMask::set(i)` flips bit `i`, the parent block-mask bit for `i/32`, and `is_any_set()` returns true.
- [ ] Test: `UpdateMask::reset(i)` clears bit `i`; clears parent block-mask bit only when last bit in the block.
- [ ] Test: `UpdateMask::set_all` followed by `reset_all` → `is_any_set()` is false.
- [ ] Test: `UpdateMask` `&=` / `|=` operator parity with C++ (`UpdateMaskHelpers::GetBlockIndex`/`GetBlockFlag` math).
- [ ] Test: `UpdateField<u32>::set(v)` flips the corresponding dirty bit; reading the value returns `v`.
- [ ] Test: `UpdateData::build_packet` for a single create-block produces byte-identical output to a captured TC `SMSG_UPDATE_OBJECT` payload (golden bytes).
- [ ] Test: `UpdateData::build_packet` triggers compressed variant when the uncompressed payload exceeds the threshold (`SMSG_COMPRESSED_UPDATE_OBJECT`).
- [ ] Test: `clear_update_mask(remove=false)` resets dirty bits on a clean entity → next tick emits no `UPDATETYPE_VALUES` block.
- [ ] Test: `UpdateFieldFlag::Owner` filtering — non-owner viewer does NOT receive owner-only fields (e.g. `PlayerData::InvSlots`); owner does.
- [ ] Test: `UpdateFieldFlag::PartyMember` filtering — same group viewer receives party-only fields; non-group does not.
- [ ] Test: `ViewerDependentValues` — faction-relative `Bytes2` differs between hostile and friendly viewer; health-percent vs. range differs between group and non-group viewer.
- [ ] Test: `WorldObject::update_object_visibility(forced=true)` after a `relocate` that crosses a grid boundary emits create-blocks for newly-visible peers and out-of-range for newly-hidden peers; no spurious blocks for unchanged peers.
- [ ] Test: Phase mask — entity with `phase_mask = 0x1` is invisible to viewer with `phase_mask = 0x2`; visible with `phase_mask = 0x3`.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 22 files / 14965 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/UpdateFields.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Object.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/UpdateField.h` | `crates/wow-core/` (GUID, Position partial), `crates/wow-world/` (lifecycle, UpdateMask — both **absent**), `crates/wow-packet/` (wire format, partial) \| 🔧 broken (rewrite needed) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#ENTITIES_OBJECT.DIV.001` | `crates/wow-packet/src/update.rs` (`missing_declared_path`, 0 Rust lines) | 22 C++ files / 14965 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/UpdateFields.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Object.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/UpdateField.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **`UpdateFields` is the single most expensive port in the project.** `UpdateFields.h` is 943 lines of struct declarations; `UpdateFields.cpp` is 5097 lines of generated descriptor tables. Across all object types there are ~1500 logical fields. A naive hand-port is multi-week. **Strongly consider code-generating from `UpdateFields.h`** rather than hand-writing.
- **`UpdateMask` is two-level**, not flat. The `_blocksMask` is a bitmask over the 32-bit `_blocks` themselves; serialization writes only blocks whose bit is set in `_blocksMask`. A single-level naive port will produce a packet that is technically correct but ~30% larger and that mis-parses on the client because the count prefix is wrong.
- **`AddToWorld` / `RemoveFromWorld` chain through the inheritance tree.** `WorldObject::AddToWorld` calls `Object::AddToWorld` then registers with the grid; subclasses override and call parent. In Rust without inheritance, this becomes a sequence of explicit method calls — write a regression test specifically for "every level was visited."
- **`IsInWorld()` is load-bearing.** TC sprinkles `ASSERT(IsInWorld())` and `if (!IsInWorld()) return;` across thousands of call sites. Skipping this in Rust will manifest as use-after-free-equivalent logic bugs (entities updated after grid removal, dirty-bit flips on torn-down state).
- **`UpdateFieldFlag::ViewerDependent` is per-viewer**, NOT per-object. The values block must be re-rendered per-target, not cached. The C++ does this via `ViewerDependentValues<T>` template specializations. Easy to miss; will manifest as health-bar mismatches or faction-color leaks.
- **`SMSG_UPDATE_OBJECT` field layout is 3.4.3-specific.** Earlier 3.3.5a TC ports have a different field index map; verify against `woltk-trinity-legacy` (the 3.4.3 source) only.
- **Compressed vs uncompressed threshold** is configurable at C++ runtime. In Rust, mirror the same default and expose a `wow-config` knob.
- **`Object` is NOT polymorphic via RTTI.** TC uses `m_objectTypeId` (a u8) + `m_objectType` (a u16 mask) + `ToPlayer()`/`ToCreature()`/etc. helpers that downcast unsafely. In Rust, the equivalent is an enum `EntityRef` or a slotmap key + sidecar type tag — never raw downcasts.
- **`Item` is NOT a `WorldObject`.** It inherits from `Object` directly, has no `Position`, no `Map`. This breaks the assumption "every entity is in a grid." `Bag`/`Item` must be discoverable by GUID via inventory linkage, not by spatial query.
- **`SmoothPhasing` ≠ `PhaseShift`.** `PhaseShift` is the active-phase set on the entity; `SmoothPhasing` is the per-viewer fade-in/out animation metadata. Easy to conflate.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Object` | `struct Object` in `crates/wow-entities/src/object.rs` (PROPOSED) | Embed in subtypes via composition; expose via `Entity` trait |
| `class WorldObject : Object, WorldLocation` | `struct WorldObject { object: Object, location: WorldLocation, visibility_distance: f32, phase_mask: u32, events: EventProcessor }` | Composition; methods do `self.object.foo()` |
| `struct WorldLocation : Position` | `struct WorldLocation { position: Position, map_id: u32 }` | New type in `wow-core` |
| `Position { x, y, z, o }` | `Position { x, y, z, orientation }` | Already exists in `wow-core::position` (note: project uses `.orientation`, not `.o`) |
| `MovementInfo` | `struct MovementInfo { flags, flags2, time, position, transport: Option<TransportInfo>, pitch, fall_time, jump }` | POD; manual wire (de)ser |
| `ObjectGuid` (128-bit) | `wow_core::ObjectGuid` | Already implemented; verify HighGuid coverage and packed-encoding parity |
| `enum class HighGuid : uint8` | `#[repr(u8)] enum HighGuid` | Already done; verify all ~50 variants present |
| `enum TypeID` (14) | `#[repr(u8)] enum TypeId` | New; in `wow-constants` |
| `enum TypeMask` (16-bit) | `bitflags! struct TypeMask : u16` | New; in `wow-constants` |
| `struct CreateObjectBits` (18 bool fields) | `bitflags! struct CreateObjectBits : u32` | One bit per flag |
| `class UpdateData` | `struct UpdateData { create_blocks: Vec<u8>, values_blocks: Vec<u8>, destroy_guids: Vec<ObjectGuid>, out_of_range_guids: Vec<ObjectGuid> }` | `build_packet() -> WorldPacket`; compress over threshold |
| `template<uint32 Bits> class UpdateMask` | `struct UpdateMask<const BITS: u32> { blocks_mask: [u32; ...], blocks: [u32; ...] }` | Generic over const; alternatively runtime-sized `BitVec` |
| `template<typename T> class UpdateField<T>` | `struct UpdateField<T> { value: T, bit_index: u16 }` + setter via `&mut UpdateMaskCtx` | `set` flips dirty bit |
| `template<typename T, std::size_t N> UpdateFieldArray` | `struct UpdateFieldArray<T, const N: usize> { values: [T; N], base_bit: u16 }` | — |
| `template<typename T> DynamicUpdateField<T>` | `struct DynamicUpdateField<T> { values: Vec<T>, base_bit: u16 }` | Length tracked in mask |
| `template<typename T> OptionalUpdateField<T>` | `struct OptionalUpdateField<T> { value: Option<T>, bit_index: u16 }` | — |
| `enum UpdateFieldFlag : uint8` | `bitflags! struct UpdateFieldFlag : u8` | `OWNER / PARTY_MEMBER / UNIT_ALL / ITEM_OWNER / SPECIAL_INFO / VIEWER_DEPENDENT` |
| `UF::ObjectData` | `struct ObjectData { entry_id: UpdateField<u32>, dynamic_flags: UpdateField<u32>, scale: UpdateField<f32>, guid: UpdateField<ObjectGuid> }` | Embedded inside `Object` |
| `template<typename F> ViewerDependentValue<F>` | `trait ViewerDependentValue { fn render_for(&self, viewer: &Player) -> Self::Output; }` | Trait per field requiring per-viewer rendering |
| `Object*` raw pointer | `EntityHandle` (opaque key into `MapManager` slotmap) | Avoid `Arc<RwLock<Entity>>` per-entity; use slotmap-style storage |
| `Player*` (long-lived ref) | `&Player` borrowed from `Map::get_player(guid)` per-call | No long-lived refs; re-resolve each tick |
| `std::unordered_map<Player*, UpdateData>` | `HashMap<ObjectGuid, UpdateData>` | Player-keyed accumulator |
| `virtual void AddToWorld()` | `fn add_to_world(&mut self)` on `Entity` trait | `debug_assert!(!self.is_in_world())` to mirror C++ ASSERT |
| `bool IsInWorld() const` | `fn is_in_world(&self) -> bool` | — |
| `void Foo::Update(uint32 diff)` | `fn update(&mut self, diff: Duration)` | Pick `Duration` consistently project-wide |
| `EventProcessor m_Events` | `struct EventProcessor { ... }` (TBD — may live in `wow-events` crate) | Per-entity event queue |
| `class SmoothPhasing` | `struct SmoothPhasing { entries: HashMap<ObjectGuid, FadeInfo> }` | Per-entity per-viewer |
| `class ObjectPosSelector` | `struct PosSelector { center: Position, occupied: Vec<(Position, f32)> }` + `fn pick(&mut self, radius: f32) -> Option<Position>` | Used for non-overlapping summons |

---

## 13. Audit (2026-05-01)

Cross-checked C++ canonical sources at `/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/` (`Object.{h,cpp}`, `ObjectGuid.{h,cpp}`, `Position.{h,cpp}`, `MovementInfo.h`, `Updates/UpdateMask.h`, `Updates/UpdateField.h`, `Updates/UpdateFields.h`, `Updates/UpdateData.{h,cpp}`, `Updates/ViewerDependentValues.h`) against current Rust state in `crates/wow-core/src/{guid.rs, position.rs}`, `crates/wow-packet/src/update.rs`, `crates/wow-world/src/map_manager.rs`. Verdict: **🔧 broken — foundation is missing**, and every downstream sub-doc (`entities-unit.md`, `entities-vehicle.md`, the future `entities-creature.md`, `entities-player.md`, `entities-item.md`, `entities-gameobject.md`) inherits the breakage.

### `Object` (`Object.{h,cpp}` — 845 + 3798 lines)
**Coverage in Rust:** 0%. No `Object` struct exists. Identity (`m_guid`) is carried implicitly by ad-hoc structs (`WorldCreature::guid`, `WorldSession::player_guid`); type tagging (`m_objectTypeId`, `m_objectType`) is replaced by HighGuid inspection at call sites. Lifecycle (`AddToWorld` / `RemoveFromWorld` / `IsInWorld`) is **absent** — `MapManager` inserts/removes creatures without an invariant boundary.

### `WorldObject` (declared in `Object.h`, defined in `Object.cpp`)
**Coverage in Rust:** 0%. No `WorldLocation` struct (no map+position pairing). Visibility recomputation is approximated by `MapManager::get_visible_creatures` returning a 3×3-grid window; `UpdateObjectVisibility(forced)` semantics (newly-visible vs. newly-hidden diffing, create-block emission, out-of-range emission) do not exist. `SummonCreature` / `SummonGameObject` factories are not wired through this layer.

### `ObjectGuid` (`ObjectGuid.{h,cpp}` — 492 + 811 lines)
**Coverage in Rust:** ~70% via `crates/wow-core/src/guid.rs`. `HighGuid` enum present; common variants (Player/Creature/Pet/GameObject/Item/Vehicle/Transport) round-trip. Untested for less-common variants (`Conversation`, `Corpse`, `AreaTrigger`, `DynamicObject`, `LootObject`, `Scenario`, `PetBattle`). `WriteAsPacked` / `ReadAsPacked` parity vs. C++ wire bytes is not byte-verified by golden test.

### `Position` / `WorldLocation` (`Position.{h,cpp}` — 223 + 209 lines)
**Coverage in Rust:** ~50% via `crates/wow-core/src/position.rs`. `Position { x, y, z, orientation }` with distance/arc math present, 6 unit tests. `WorldLocation` (Position + map_id) **does not exist** — `WorldCreature::position` is bare `Position`; map id is carried separately in `MapManager` keys. Cross-map relocate has no unified primitive.

### `MovementInfo` (`MovementInfo.h` — 204 lines)
**Coverage in Rust:** partial in `wow-packet`. Movement opcodes are decoded ad-hoc per-handler in `crates/wow-world/src/handlers/movement.rs`; there is no canonical `MovementInfo` struct shared between client→server parsing and server→client `SMSG_UPDATE_OBJECT` movement-block emission.

### `UpdateMask` (`Updates/UpdateMask.h` — 164 lines)
**Coverage in Rust:** **0%**. No analogue. **This is the core replication primitive** — without it, `SMSG_UPDATE_OBJECT` cannot emit deltas, only full re-serializations. The two-level `_blocksMask` + `_blocks` layout, the per-bit `Set/Reset` operations that maintain block-mask consistency, and the `IsAnySet`/`operator&=`/`operator|=` combinators all need to be ported before any per-type `UF::*Data` can be wired up.

### `UpdateField<T>` / `UpdateFieldFlag` (`Updates/UpdateField.h` — 991 lines)
**Coverage in Rust:** **0%**. No `UpdateField<T>` wrapper, no `UpdateFieldFlag` viewer-filter taxonomy. Every field write in `WorldCreature` (e.g. `take_damage` mutating `current_hp`, `respawn` resetting `current_hp = max_hp`) directly mutates the field with no replication trigger. The `Owner / PartyMember / UnitAll / ItemOwner / SpecialInfo / ViewerDependent` filtering that determines which fields each viewer receives is entirely absent.

### `UpdateFields` per-type structs (`Updates/UpdateFields.{h,cpp}` — 943 + 5097 lines)
**Coverage in Rust:** **0%**. None of `ObjectData`, `UnitData`, `PlayerData`, `ActivePlayerData`, `ItemData`, `ContainerData`, `GameObjectData`, `DynamicObjectData`, `CorpseData`, `AreaTriggerData`, `SceneObjectData`, `ConversationData` exist. ~1500 logical fields total. The hand-rolled `CreatureCreateData` / `PlayerCreateData` in `wow-packet::update` cover a small fraction of `UnitData` and `PlayerData` respectively, and even those skip the dirty-bit machinery — they re-emit full state every time.

### `UpdateData` (`Updates/UpdateData.{h,cpp}` — 67 + 72 lines)
**Coverage in Rust:** **0%**. No accumulator, no compressed variant, no destroy/out-of-range channel. Each `SMSG_UPDATE_OBJECT` is built ad-hoc per call site rather than batched per-tick per-viewer.

### `ViewerDependentValues` (`Updates/ViewerDependentValues.h` — 367 lines)
**Coverage in Rust:** **0%**. Per-viewer field overrides (faction-relative `Bytes2`, group-share `Health` percent, owner-only `InvSlots`) do not exist. This will manifest as health-bar privacy leaks, wrong faction colors, and inventory visible to non-owners once a real entity layer ships without it.

### Down-stream blast radius
Because `Object` and `UpdateMask` are missing, **every sibling sub-doc inherits 🔧 status**:

- `entities-unit.md` — `Unit::DealDamage` is replaced by `WorldCreature::take_damage(u32) -> bool` at `crates/wow-world/src/map_manager.rs:176` which silently mutates `current_hp` without replication. Every other Unit field (auras, threat, charm, vehicle base, modifiers, immunities, school masks, school resistances) has the same problem: there is nowhere to flip a dirty bit.
- `entities-vehicle.md` — passenger seat changes have no `UpdateField<Passenger>` to mutate; clients learn about mounts only from out-of-band packets.
- `entities-transport.md` — same as Vehicle, plus the `TransportBase` global-frame transform is not represented.
- Future `entities-creature.md`, `entities-player.md`, `entities-item.md`, `entities-gameobject.md`, `entities-pet.md`, `entities-areatrigger.md`, `entities-corpse.md`, `entities-dynamicobject.md` — all blocked on `Object` + `UpdateMask`.

### Verdict
**🔧 broken — must be ported before any sibling entity sub-doc can ship a passing audit.** `Object` and the `Updates/` infrastructure are the foundation of WoW's wire-replication contract; everything visible to a client funnels through them. The rough order of operations is: `WorldLocation` (#OBJECT.3) → `Object`/`WorldObject` skeleton (#OBJECT.5/.6) → `UpdateMask` (#OBJECT.8) → `UpdateField<T>` (#OBJECT.9) → `UF::ObjectData` first vertical slice (#OBJECT.12) → `UpdateData` (#OBJECT.13) → per-type field structs (#OBJECT.23, deferred to sub-docs). Until that path lands, the rest of the entities tree is plumbing without a substrate.

---

*Template version: 1.0 (2026-05-01).* Last updated 2026-05-01.
