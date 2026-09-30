use super::{
    AccessorObjectKind, AddToMapOutcome, CellCoord, CreatureAimInitializeOutcomeLikeCpp,
    CreatureSearchFormationOutcomeLikeCpp, DynamicMapTreeModelMutationOutcomeLikeCpp,
    DynamicObjectCasterViewpointOutcomeLikeCpp, GameObjectDeleteOutcomeLikeCpp,
    GameObjectLifecycleError, GameObjectSummonTypeLikeCpp, GridCoord, LootState,
    ObjectAccessorError, ObjectGuid, PersonalPhaseUnregisterTrackedObjectOutcomeLikeCpp, Player,
    PlayerRemoveFromWorldViewpointCleanupOutcomeLikeCpp, SpawnId, UnitAddToWorldOutcomeLikeCpp,
    UnitRemoveFromWorldOutcomeLikeCpp, VehicleKitAddToWorldResetOutcomeLikeCpp,
    VehicleKitInstallOutcomeLikeCpp, VehicleKitRemoveOutcomeLikeCpp, WorldObject,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapObjectStoreError {
    InvalidRecord(ObjectAccessorError),
    WrongMap {
        guid: ObjectGuid,
        expected_map_id: u32,
        expected_instance_id: u32,
        actual_map_id: u32,
        actual_instance_id: u32,
    },
}

impl From<ObjectAccessorError> for MapObjectStoreError {
    fn from(error: ObjectAccessorError) -> Self {
        Self::InvalidRecord(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectCollisionEnableOutcomeLikeCpp {
    pub requested_enable: bool,
    pub represented_model_present: bool,
    pub previous_collision_enabled: Option<bool>,
    pub new_collision_enabled: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureZoneScriptCreateOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub represented_callback: bool,
    pub script_dispatch_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectZoneScriptCreateOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub represented_callback_boundary: bool,
    pub script_dispatch_represented: bool,
    pub object_store_present_before_callback: bool,
    pub spawn_index_present_before_callback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectZoneScriptRemoveOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub represented_callback_boundary: bool,
    pub script_dispatch_represented: bool,
    pub model_remove_pending_before_callback: bool,
    pub spawn_index_present_before_callback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectAddToOwnerOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub owner_guid: ObjectGuid,
    pub owner_found_as_unit_like: bool,
    pub gameobject_found: bool,
    pub owner_guid_before: ObjectGuid,
    pub owner_guid_after: ObjectGuid,
    pub gameobject_owner_empty_before: bool,
    pub registered_owned_gameobject: bool,
    pub owner_guid_set: bool,
    pub cooldown_start_represented: bool,
    pub creature_ai_callback_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectAddToOwnerSlotOutcomeLikeCpp {
    pub add_owner: GameObjectAddToOwnerOutcomeLikeCpp,
    pub slot: usize,
    pub slot_previous_guid: ObjectGuid,
    pub slot_set: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectPrepareOwnerSlotForSummonOutcomeLikeCpp {
    pub owner_guid: ObjectGuid,
    pub slot: usize,
    pub spell_id: u32,
    pub owner_found_as_unit_like: bool,
    pub slot_guid_before: ObjectGuid,
    pub slot_had_guid: bool,
    pub gameobject_found: bool,
    pub recast_spell_id_cleared: bool,
    pub unit_pointer_owner_match: bool,
    pub remove_from_owner: Option<GameObjectRemoveFromOwnerOutcomeLikeCpp>,
    pub respawn_time_cleared: bool,
    pub delete_outcome: Option<GameObjectDeleteOutcomeLikeCpp>,
    pub slot_cleared: bool,
    pub cooldown_event_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameObjectSummonObjectForOwnerSlotStatusLikeCpp {
    MissingOwner,
    LowGuidUnavailable,
    CreateFailed,
    AddToMapOrOwnerFailed,
    CreatedAddedAndSlotted,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectSummonObjectForOwnerSlotOutcomeLikeCpp {
    pub owner_guid: ObjectGuid,
    pub slot: usize,
    pub spell_id: u32,
    pub template_entry: u32,
    pub status: GameObjectSummonObjectForOwnerSlotStatusLikeCpp,
    pub guid: Option<ObjectGuid>,
    pub low_guid: Option<i64>,
    pub create_error: Option<GameObjectLifecycleError>,
    pub add_to_map: Option<AddToMapOutcome>,
    pub add_owner_slot: Option<GameObjectAddToOwnerSlotOutcomeLikeCpp>,
    pub respawn_time_secs: Option<i32>,
    pub caster_faction: Option<u32>,
    pub caster_level: Option<u32>,
    pub phase_inherit_represented: bool,
    pub execute_log_represented: bool,
    pub cooldown_event_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldObjectSummonGameObjectStatusLikeCpp {
    MissingSummoner,
    SummonerNotInWorld,
    LowGuidUnavailable,
    CreateFailed,
    AddToMapFailed,
    CreatedAddedToMap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldObjectSummonGameObjectOutcomeLikeCpp {
    pub summoner_guid: ObjectGuid,
    pub template_entry: u32,
    pub summon_type: GameObjectSummonTypeLikeCpp,
    pub status: WorldObjectSummonGameObjectStatusLikeCpp,
    pub guid: Option<ObjectGuid>,
    pub low_guid: Option<i64>,
    pub create_error: Option<GameObjectLifecycleError>,
    pub add_to_map: Option<AddToMapOutcome>,
    pub add_owner: Option<GameObjectAddToOwnerOutcomeLikeCpp>,
    pub respawn_time_secs: i64,
    pub phase_inherit_represented: bool,
    pub spawned_by_default_forced_false: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellEffectSummonObjectWildStatusLikeCpp {
    MissingCaster,
    LowGuidUnavailable,
    CreateFailed,
    AddToMapFailed,
    CreatedAddedToMap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellEffectSummonObjectWildOutcomeLikeCpp {
    pub caster_guid: ObjectGuid,
    pub spell_id: u32,
    pub template_entry: u32,
    pub status: SpellEffectSummonObjectWildStatusLikeCpp,
    pub guid: Option<ObjectGuid>,
    pub low_guid: Option<i64>,
    pub create_error: Option<GameObjectLifecycleError>,
    pub add_to_map: Option<AddToMapOutcome>,
    pub respawn_time_secs: Option<i32>,
    pub phase_inherit_represented: bool,
    pub execute_log_represented: bool,
    pub owner_linked: bool,
    pub flagdrop_type: bool,
    pub flagdrop_player_branch_reached: bool,
    pub flagdrop_battleground_update_represented: bool,
    pub linked_trap_guid: Option<ObjectGuid>,
    pub linked_trap_side_effect_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitRemoveGameObjectsBySpellOutcomeLikeCpp {
    pub owner_guid: ObjectGuid,
    pub spell_id: u32,
    pub delete_requested: bool,
    pub owner_found_as_unit_like: bool,
    pub owned_entries_before: usize,
    pub matched_entries: usize,
    pub owner_guid_cleared: usize,
    pub respawn_time_cleared: usize,
    pub owner_list_entries_removed: usize,
    pub delete_outcomes: usize,
    pub object_slot_cleanup_represented: bool,
    pub aura_cleanup_represented: bool,
    pub cooldown_event_represented: bool,
    pub creature_ai_callback_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectRemoveFromOwnerOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub owner_guid_before: ObjectGuid,
    pub owner_guid_after: ObjectGuid,
    pub owner_found_as_unit_like: bool,
    pub cleared_owner: bool,
    pub spell_id: u32,
    pub unit_side_effects_represented: bool,
    pub unit_owned_gameobject_list_removed: bool,
    pub unit_object_slot_cleared: bool,
    pub aura_cleanup_represented: bool,
    pub aura_cleanup_removed_count: usize,
    pub cooldown_event_represented: bool,
    pub creature_ai_callback_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectRemoveLinkedTrapOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub linked_trap_guid: Option<ObjectGuid>,
    pub owner_present_before_linked_trap_remove: bool,
    pub linked_trap_removed: bool,
    pub linked_trap_remove_queued: bool,
    pub linked_trap_missing_or_self: bool,
    pub linked_trap_cycle_guarded: bool,
    pub despawn_or_unsummon_scheduler_represented: bool,
    pub object_accessor_fanout_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureZoneScriptRemoveOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub represented_callback: bool,
    pub script_dispatch_represented: bool,
}

/// Represented key for the map-owned C++ `_dynamicTree` model-registration seam.
///
/// C++ `DynamicMapTree` stores `GameObjectModel` object references/pointers. Rust does
/// not model real `GameObjectModel` or collision geometry in this bounded slice, so
/// the deterministic stand-in key is the owning object GUID. Duplicate insertion is
/// guarded as a no-op to avoid count drift; this is intentionally safer than raw
/// pointer duplicate behavior and is not a claim of exact model object identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RepresentedGameObjectModelKeyLikeCpp {
    pub owner_guid: ObjectGuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicMapTreeModelMutationStatusLikeCpp {
    Inserted,
    AlreadyPresent,
    Removed,
    Missing,
}

/// Represented result for C++ `DynamicMapTree::{insert,remove}` via Map facades.
///
/// Anchors: `DynamicTree.cpp:72-82,115-127`, `Map.h:457-460`.
/// Real `GameObjectModel`, RegularGrid/BIH, LOS/intersection/height,
/// AddToWorld/RemoveFromWorld wiring, transport delayed-add, `GO_FLAG_MAP_OBJECT`,
/// collision enable/disable, ObjectAccessor/session/fanout/scripts/AI/DB remain out
/// of scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicMapTreeModelMutationOutcomeLikeCpp {
    pub key: RepresentedGameObjectModelKeyLikeCpp,
    pub status: DynamicMapTreeModelMutationStatusLikeCpp,
    pub model_count_before: usize,
    pub model_count_after: usize,
    pub unbalanced_before: u32,
    pub unbalanced_after: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameObjectUpdateModelStatusLikeCpp {
    Updated,
    MissingGameObject,
    WrongKind,
    NotInWorld,
}

/// Represented map-owned result for C++ `GameObject::UpdateModel()`.
///
/// C++ anchor: `GameObject.cpp:3867-3880`. This helper operates only on the
/// canonical `Map::entity_world` exact typed GameObject record, consumes explicit
/// caller-provided `CreateModel()` evidence, and mutates only represented local
/// model/flag/collision evidence plus the map-owned represented DynamicMapTree
/// key set. It does not infer from display/template/DB and does not call
/// `EnableCollision()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectUpdateModelOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub status: GameObjectUpdateModelStatusLikeCpp,
    pub old_model_present: bool,
    pub old_model_registered: bool,
    pub old_model_remove: Option<DynamicMapTreeModelMutationOutcomeLikeCpp>,
    pub new_has_model: bool,
    pub new_is_map_object: bool,
    pub new_model_insert: Option<DynamicMapTreeModelMutationOutcomeLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameObjectSetDisplayIdStatusLikeCpp {
    Updated,
    MissingGameObject,
    WrongKind,
}

/// Represented map-owned result for C++ `GameObject::SetDisplayId(uint32)`.
///
/// C++ anchor: `GameObject.cpp:3817-3820`. This preserves statement order over
/// canonical exact typed `Map::entity_world` GameObject records: write
/// `GameObjectData::DisplayID` first, then call represented `UpdateModel()`.
/// The model creation evidence remains caller-provided and is never inferred
/// from display/template/DB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectSetDisplayIdOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub status: GameObjectSetDisplayIdStatusLikeCpp,
    pub previous_display_id: Option<i32>,
    pub new_display_id: Option<i32>,
    pub update_model: Option<GameObjectUpdateModelOutcomeLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameObjectSetGoStateStatusLikeCpp {
    Updated,
    MissingGameObject,
    WrongKind,
}

/// Represented map-owned result for C++ `GameObject::SetGoState(GOState)`.
///
/// C++ anchor: `GameObject.cpp:3771-3793`. This preserves statement order over
/// canonical exact typed `Map::entity_world` GameObject records: capture old state,
/// write `GameObjectData::State`, then run only the represented `m_model &&
/// !IsTransport() && IsInWorld()` collision branch. AI/type implementation hooks,
/// real `GameObjectModel`, BIH/LOS, ObjectAccessor/session fanout, scripts and DB
/// inference remain out of scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectSetGoStateOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub status: GameObjectSetGoStateStatusLikeCpp,
    pub previous_state: Option<i8>,
    pub new_state: Option<i8>,
    pub represented_model_present: bool,
    pub transport_type: bool,
    pub in_world_for_collision_branch: Option<bool>,
    pub collision_enable: Option<GameObjectCollisionEnableOutcomeLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameObjectSetLootStateStatusLikeCpp {
    Updated,
    MissingGameObject,
    WrongKind,
}

/// Represented map-owned result for C++ `GameObject::SetLootState(LootState, Unit*)`.
///
/// C++ anchor: `GameObject.cpp:3683-3709`. This preserves statement order over
/// canonical exact typed `Map::entity_world` GameObject records: write local loot
/// state/unit GUID first, expose the unimplemented AI hook as evidence, then
/// represent only explicit-caller-evidence restock and represented `m_model` collision.
/// It does not execute real AI, infer `Loot::IsChanged()`, create real
/// `GameObjectModel`/BIH geometry, fan out ObjectAccessor/session/script/DB effects, or
/// resolve a real `Unit*` from the supplied GUID evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectSetLootStateOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub status: GameObjectSetLootStateStatusLikeCpp,
    pub previous_loot_state: Option<LootState>,
    pub new_loot_state: Option<LootState>,
    pub previous_loot_state_unit_guid: Option<ObjectGuid>,
    pub new_loot_state_unit_guid: Option<ObjectGuid>,
    pub previous_restock_time: Option<i64>,
    pub new_restock_time: Option<i64>,
    pub ai_on_loot_state_changed_not_represented: bool,
    pub restock_armed: bool,
    pub represented_model_present: bool,
    pub door_type_early_return: bool,
    pub collision_enable: Option<GameObjectCollisionEnableOutcomeLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddObjectToRemoveListOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub queued: bool,
    pub duplicate: bool,
    pub missing_or_stale: bool,
    pub unsupported_kind: Option<AccessorObjectKind>,
    pub cleanup_before_delete_count: usize,
}

pub type RemoveListOutcomeLikeCpp = AddObjectToRemoveListOutcomeLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveNonPlayerMutationStatusLikeCpp {
    Mutated,
    MissingRecord,
    PlayerUnsupported,
    NotActiveObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveNonPlayerUnloadLockOutcomeLikeCpp {
    pub spawn_id: SpawnId,
    pub respawn_grid: Option<GridCoord>,
    pub respawn_grid_missing: bool,
    pub invalid_respawn_position: bool,
    pub lock_incremented: bool,
    pub lock_decremented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveNonPlayerMutationOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub status: ActiveNonPlayerMutationStatusLikeCpp,
    pub inserted_in_active_set: bool,
    pub removed_from_active_set: bool,
    pub spawn_id_zero_or_unsupported: bool,
    pub unload_lock: Option<ActiveNonPlayerUnloadLockOutcomeLikeCpp>,
}

pub type AddToActiveOutcomeLikeCpp = ActiveNonPlayerMutationOutcomeLikeCpp;
pub type RemoveFromActiveOutcomeLikeCpp = ActiveNonPlayerMutationOutcomeLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddToMapPostAddToWorldOutcomeLikeCpp {
    pub initialize_object_represented: bool,
    pub pending_move_state_cleared: bool,
    pub no_pending_move_state: bool,
    pub add_to_active_represented: bool,
    pub add_to_active_skipped_runtime_gap: bool,
    pub add_to_active: Option<AddToActiveOutcomeLikeCpp>,
    pub set_is_new_object_true: bool,
    pub update_object_visibility_on_create_represented: bool,
    pub update_object_visibility_on_create_runtime_gap: bool,
    pub set_is_new_object_false: bool,
    pub final_is_new_object: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddToMapOutcome {
    pub guid: ObjectGuid,
    pub cell: CellCoord,
    pub grid: GridCoord,
    pub inserted: bool,
    pub already_in_world: bool,
    pub grid_created: bool,
    pub grid_loaded: bool,
    pub inserted_into_cell: bool,
    pub gameobject_model_insert: Option<DynamicMapTreeModelMutationOutcomeLikeCpp>,
    pub gameobject_collision_enable: Option<GameObjectCollisionEnableOutcomeLikeCpp>,
    pub gameobject_zone_script_create: Option<GameObjectZoneScriptCreateOutcomeLikeCpp>,
    pub gameobject_store_inserted_before_add_to_world: Option<bool>,
    pub gameobject_spawn_indexed_before_add_to_world: Option<bool>,
    pub creature_store_inserted_before_add_to_world: Option<bool>,
    pub creature_spawn_indexed_before_add_to_world: Option<bool>,
    pub creature_unit_add_to_world: Option<UnitAddToWorldOutcomeLikeCpp>,
    pub creature_search_formation: Option<CreatureSearchFormationOutcomeLikeCpp>,
    pub creature_aim_initialize: Option<CreatureAimInitializeOutcomeLikeCpp>,
    pub creature_vehicle_reset: Option<VehicleKitAddToWorldResetOutcomeLikeCpp>,
    pub creature_vehicle_install: Option<VehicleKitInstallOutcomeLikeCpp>,
    pub creature_zone_script_create: Option<CreatureZoneScriptCreateOutcomeLikeCpp>,
    pub add_to_map_tail: Option<AddToMapPostAddToWorldOutcomeLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AddToMapError {
    InvalidCoordinates { guid: ObjectGuid, x: f32, y: f32 },
    Store(MapObjectStoreError),
}

impl From<MapObjectStoreError> for AddToMapError {
    fn from(error: MapObjectStoreError) -> Self {
        Self::Store(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveFromMapVisibilityOnDestroyOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub cxx_in_world: bool,
    pub update_object_visibility_on_destroy_represented: bool,
    pub update_object_visibility_on_destroy_runtime_gap: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveFromMapOutcome {
    pub guid: ObjectGuid,
    pub cell: CellCoord,
    pub grid: GridCoord,
    pub was_in_world: bool,
    pub cxx_in_world: bool,
    pub was_active: bool,
    pub remove_from_active: Option<RemoveFromActiveOutcomeLikeCpp>,
    pub removed_from_cell: bool,
    pub delete_from_world: bool,
    pub dynamic_object_caster_viewpoint: Option<DynamicObjectCasterViewpointOutcomeLikeCpp>,
    pub dynamic_object_remove_cleanup: Option<DynamicObjectRemoveCleanupOutcomeLikeCpp>,
    pub gameobject_zone_script_remove: Option<GameObjectZoneScriptRemoveOutcomeLikeCpp>,
    pub gameobject_remove_from_owner: Option<GameObjectRemoveFromOwnerOutcomeLikeCpp>,
    pub gameobject_model_remove: Option<DynamicMapTreeModelMutationOutcomeLikeCpp>,
    pub gameobject_linked_trap_remove: Option<GameObjectRemoveLinkedTrapOutcomeLikeCpp>,
    pub creature_zone_script_remove: Option<CreatureZoneScriptRemoveOutcomeLikeCpp>,
    pub creature_vehicle_remove: Option<VehicleKitRemoveOutcomeLikeCpp>,
    pub player_viewpoint_cleanup: Option<PlayerRemoveFromWorldViewpointCleanupOutcomeLikeCpp>,
    pub creature_unit_remove_from_world: Option<UnitRemoveFromWorldOutcomeLikeCpp>,
    pub creature_remove_formation: Option<CreatureRemoveFormationOutcomeLikeCpp>,
    pub personal_phase_unregister: PersonalPhaseUnregisterTrackedObjectOutcomeLikeCpp,
    pub visibility_on_destroy: RemoveFromMapVisibilityOnDestroyOutcomeLikeCpp,
    /// The exact canonical Player value retained by a non-delete Map transfer.
    /// Other object families remain represented by `object` below.
    pub player: Option<Box<Player>>,
    pub object: Option<WorldObject>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureRemoveFormationOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub spawn_id: SpawnId,
    pub leader_spawn_id: Option<SpawnId>,
    pub had_group: bool,
    pub removed_member: bool,
    pub removed_group: bool,
    pub remaining_members: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicObjectRemoveCleanupOutcomeLikeCpp {
    pub had_aura: bool,
    pub removed_aura_pending_delete: bool,
    pub unbound_caster: Option<ObjectGuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveFromMapError {
    ObjectNotFound { guid: ObjectGuid },
    ResetMap(MapBindingError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapObjectRelocationOutcome {
    pub guid: ObjectGuid,
    pub old_cell: CellCoord,
    pub new_cell: CellCoord,
    pub old_grid: GridCoord,
    pub new_grid: GridCoord,
    pub moved_between_cells: bool,
    pub loaded_grid: bool,
    pub created_grid: bool,
    pub relocated: bool,
    pub blocked_by_unloaded_grid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MapObjectRelocationError {
    ObjectNotFound { guid: ObjectGuid },
    InvalidCoordinates { guid: ObjectGuid, x: f32, y: f32 },
    Record(ObjectAccessorError),
    Store(MapObjectStoreError),
}
