use std::time::{SystemTime, UNIX_EPOCH};

use wow_constants::{
    CreatureFlagsExtra, CreatureFlightMovementType, CreatureGroundMovementType, CreatureTypeFlags,
    DeathState, PowerType, TypeId, TypeMask, UnitDynFlags, UnitFlags, UnitFlags2, UnitState,
    WeaponAttackType, movement::MovementFlag,
};
use wow_core::{ObjectGuid, Position};

use crate::{
    BASE_MAXDAMAGE, BASE_MINDAMAGE, Unit, VehicleAccessory, VehicleSeatAddon, VehicleSeatInfo,
};

pub const CREATURE_REGEN_INTERVAL_MS: u32 = 2_000;
pub const MAX_CREATURE_SPELLS: usize = 8;
pub const DEFAULT_RESPAWN_DELAY_SECS: u32 = 300;
pub const DEFAULT_CORPSE_DELAY_SECS: u32 = 60;
pub const DEFAULT_BOUNDARY_CHECK_TIME_MS: u32 = 2_500;
pub const DEFAULT_MONSTER_SIGHT_DISTANCE: f32 = 50.0;
pub const LOOT_MODE_DEFAULT: u16 = 0x1;
pub const CREATURE_TAPPERS_SOFT_CAP: usize = 5;
pub const CREATURE_NOPATH_EVADE_TIME_MS: u32 = 10_000;
pub const CREATURE_Z_ATTACK_RANGE_LIKE_CPP: f32 = 3.0;
pub const MAX_AGGRO_RESET_TIME_SECS_LIKE_CPP: i64 = 10;
const CREATURE_GROUND_MOVEMENT_TYPE_MAX_LIKE_CPP: u8 = 3;
const CREATURE_FLIGHT_MOVEMENT_TYPE_MAX_LIKE_CPP: u8 = 3;

pub fn game_time_secs_like_cpp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or(0)
}

pub const fn normalize_creature_flight_movement_type_like_cpp(flight_movement_type: u8) -> u8 {
    if flight_movement_type < CREATURE_FLIGHT_MOVEMENT_TYPE_MAX_LIKE_CPP {
        flight_movement_type
    } else {
        CreatureFlightMovementType::None as u8
    }
}

pub const fn normalize_creature_ground_movement_type_like_cpp(ground_movement_type: u8) -> u8 {
    if ground_movement_type < CREATURE_GROUND_MOVEMENT_TYPE_MAX_LIKE_CPP {
        ground_movement_type
    } else {
        CreatureGroundMovementType::Run as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReactState {
    Passive = 0,
    Defensive = 1,
    Aggressive = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MovementGeneratorType {
    Idle = 0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureMovementInform {
    pub movement_type: u8,
    pub movement_id: u32,
}

/// Canonical creature AI state owned by `wow-entities`.
///
/// This mirrors the small legacy runtime state machine used by the world tick
/// without depending on `wow-ai` or `wow-world`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureAiState {
    Idle,
    WalkingRandom,
    WalkingWaypoint,
    InCombat,
    Dead,
    Returning,
}

/// Canonical AI/runtime ownership state for a creature.
///
/// Time fields are abstract monotonic milliseconds supplied by the caller. The
/// entity layer intentionally does not store `Instant` so it remains reusable by
/// world, tests, persistence and packet bridges.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureAiOwnershipState {
    pub state: CreatureAiState,
    pub home_position: Position,
    pub move_target: Option<Position>,
    pub move_start_ms: u64,
    pub move_duration_ms: u32,
    pub spline_id: u32,
    pub wander_delay_ms: u64,
    pub combat_target: Option<ObjectGuid>,
    pub last_swing_ms: u64,
    pub swing_timer_ms: u64,
    pub aggro_radius: f32,
    pub wander_radius: f32,
    pub death_time_ms: Option<u64>,
    pub respawn_time_secs: u64,
    pub corpse_despawn_at_ms: Option<u64>,
    pub display_id: u32,
    pub faction: u32,
    pub npc_flags: u32,
    pub npc_flags2: u32,
    pub unit_flags: u32,
    pub min_damage: u32,
    pub max_damage: u32,
    pub loot_id: u32,
    pub skin_loot_id: u32,
    pub gold_min: u32,
    pub gold_max: u32,
    pub boss_id: Option<u32>,
    pub dungeon_encounter_id: u32,
    pub phase_use_flags: u8,
    pub phase_id: u16,
    pub phase_group_id: u32,
    pub terrain_swap_map: i32,
    pub last_movement_inform: Option<CreatureMovementInform>,
}

impl Default for CreatureAiOwnershipState {
    fn default() -> Self {
        Self {
            state: CreatureAiState::Idle,
            home_position: Position::ZERO,
            move_target: None,
            move_start_ms: 0,
            move_duration_ms: 0,
            spline_id: 1,
            wander_delay_ms: 8_000,
            combat_target: None,
            last_swing_ms: 0,
            swing_timer_ms: 2_000,
            aggro_radius: DEFAULT_MONSTER_SIGHT_DISTANCE,
            wander_radius: 5.0,
            death_time_ms: None,
            respawn_time_secs: u64::from(DEFAULT_RESPAWN_DELAY_SECS),
            corpse_despawn_at_ms: None,
            display_id: 0,
            faction: 0,
            npc_flags: 0,
            npc_flags2: 0,
            unit_flags: 0,
            min_damage: BASE_MINDAMAGE as u32,
            max_damage: BASE_MAXDAMAGE as u32,
            loot_id: 0,
            skin_loot_id: 0,
            gold_min: 0,
            gold_max: 0,
            boss_id: None,
            dungeon_encounter_id: 0,
            phase_use_flags: 0,
            phase_id: 0,
            phase_group_id: 0,
            terrain_swap_map: -1,
            last_movement_inform: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureModelDimensions {
    pub bounding_radius: f32,
    pub combat_reach: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureLifecycleStats {
    pub max_health: u64,
    pub health: u64,
    pub power_type: PowerType,
    pub max_mana: i32,
    pub mana: i32,
    pub min_damage: f32,
    pub max_damage: f32,
}

impl CreatureLifecycleStats {
    pub const fn new(max_health: u64, health: u64, max_mana: i32, mana: i32) -> Self {
        Self {
            max_health,
            health,
            power_type: PowerType::Mana,
            max_mana,
            mana,
            min_damage: BASE_MINDAMAGE,
            max_damage: BASE_MAXDAMAGE,
        }
    }
}

/// Represented subset of TrinityCore `CreatureTemplate`/difficulty data used by
/// `Creature::InitEntry` and `CreateFromProto`.
///
/// ObjectMgr, DB2 model stores, addon/equipment table loading and script binding are deliberately
/// external to this record. Callers pass the already-resolved values that `wow-entities` can own.
#[derive(Debug, Clone, PartialEq)]
pub struct CreatureTemplateLifecycleRecord {
    pub entry: u32,
    pub original_entry: u32,
    pub difficulty_id: u8,
    pub name: String,
    pub ai_name: String,
    pub script_name: String,
    pub unit_class: u8,
    pub faction: u32,
    pub display_id: u32,
    pub model_dimensions: Option<CreatureModelDimensions>,
    pub scale: f32,
    pub speed_walk: f32,
    pub speed_run: f32,
    pub spells: [u32; MAX_CREATURE_SPELLS],
    pub classification: u32,
    pub flags_extra: u32,
    pub creature_type: u32,
    pub type_flags: u32,
    pub movement_type: MovementGeneratorType,
    pub ground_movement_type: u8,
    pub swim_allowed: bool,
    pub flight_movement_type: u8,
    pub min_level: u8,
    pub max_level: u8,
    pub equipment_id: u8,
    pub original_equipment_id: i8,
}

/// Represented subset of TrinityCore `CreatureData` consumed by `Creature::LoadFromDB`.
#[derive(Debug, Clone, PartialEq)]
pub struct CreatureSpawnLifecycleRecord {
    pub spawn_id: u64,
    pub map_id: u32,
    pub instance_id: u32,
    pub position: Position,
    pub home_position: Position,
    pub phase_id: Option<u32>,
    pub phase_group: Option<u32>,
    pub terrain_swap_map: Option<u32>,
    pub spawn_group_id: Option<u32>,
    pub spawn_group_name: Option<String>,
    pub pool_id: Option<u32>,
    pub equipment_id: Option<u8>,
    pub original_equipment_id: Option<i8>,
    pub wander_distance: f32,
    pub respawn_delay: u32,
    pub respawn_time: i64,
    pub movement_type: MovementGeneratorType,
    pub string_id: Option<String>,
    pub is_active: bool,
    pub inactive_by_spawn_group: bool,
    pub duplicate_spawn_found: bool,
    pub add_to_map: bool,
    pub respawn_compatibility_mode: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VehicleKitCreateInputLikeCpp {
    pub vehicle_id: u32,
    pub creature_entry: u32,
    pub loading: bool,
    pub seat_defs: Vec<(i8, VehicleSeatInfo, VehicleSeatAddon)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureFormationInfoLikeCpp {
    pub leader_spawn_id: u64,
    pub follow_dist: f32,
    pub follow_angle_radians: f32,
    pub group_ai: u32,
    pub leader_waypoint_ids: [u32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureSearchFormationOutcomeLikeCpp {
    pub spawn_id: u64,
    pub is_summon: bool,
    pub formation_info_found: bool,
    pub leader_spawn_id: Option<u64>,
    pub add_to_group_requested: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureAimInitializeOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub spawn_id: u64,
    pub aim_create_represented: bool,
    pub motion_initialize_represented: bool,
    pub formation_present: bool,
    pub formation_leader: bool,
    pub formation_move_idle_represented: bool,
    pub motion_initialize_requires_formed_state: bool,
    pub motion_master_initialize_represented: bool,
    pub ai_selected_represented: bool,
    pub ai_initialize_represented: bool,
    pub vehicle_reset_expected: bool,
    pub succeeded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureAddToWorldVehicleResetContextLikeCpp {
    pub is_mechanical_creature: bool,
    pub is_world_boss: bool,
    pub accessories: Vec<VehicleAccessory>,
}

/// Resolved, testable input for TrinityCore `Creature::Create`.
#[derive(Debug, Clone, PartialEq)]
pub struct CreatureCreateLifecycleRecord {
    pub guid: ObjectGuid,
    pub entry: u32,
    pub map_id: u32,
    pub instance_id: u32,
    pub position: Position,
    pub dynamic: bool,
    pub vehicle_id: Option<u32>,
    pub vehicle_kit_create_input: Option<VehicleKitCreateInputLikeCpp>,
    pub add_to_world_vehicle_reset_context: Option<CreatureAddToWorldVehicleResetContextLikeCpp>,
    pub template: CreatureTemplateLifecycleRecord,
    pub spawn: Option<CreatureSpawnLifecycleRecord>,
    pub selected_level: u8,
    pub stats: CreatureLifecycleStats,
    pub selected_display_id: u32,
    pub selected_model_dimensions: Option<CreatureModelDimensions>,
    pub selected_equipment_id: u8,
    pub selected_original_equipment_id: i8,
    pub corpse_delay: u32,
    pub ignore_corpse_decay_ratio: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureLoadFromDbLifecycleRecord {
    pub create: CreatureCreateLifecycleRecord,
    pub spawn: CreatureSpawnLifecycleRecord,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureLifecycleMetadata {
    pub template_entry: u32,
    pub original_entry: u32,
    pub difficulty_id: u8,
    pub ai_name: String,
    pub script_name: String,
    pub unit_class: u8,
    pub classification: u32,
    pub flags_extra: u32,
    pub ground_movement_type: u8,
    pub swim_allowed: bool,
    pub flight_movement_type: u8,
    pub creature_type: u32,
    pub type_flags: u32,
    pub selected_level: u8,
    pub selected_display_id: u32,
    pub selected_model_dimensions: Option<CreatureModelDimensions>,
    pub template_scale: f32,
    pub speed_walk: f32,
    pub speed_run: f32,
    pub spawn_id: u64,
    pub spawn_map_id: u32,
    pub spawn_instance_id: u32,
    pub spawn_position: Position,
    pub home_position: Position,
    pub phase_id: Option<u32>,
    pub phase_group: Option<u32>,
    pub terrain_swap_map: Option<u32>,
    pub spawn_group_id: Option<u32>,
    pub spawn_group_name: Option<String>,
    pub pool_id: Option<u32>,
    pub string_id: Option<String>,
    pub is_spawn_active: bool,
    pub inactive_by_spawn_group: bool,
    pub duplicate_spawn_found: bool,
    pub add_to_map_requested: bool,
    pub map_insertion_requested: bool,
    pub dynamic_spawn: bool,
    pub is_summon_like_cpp: bool,
    pub formation_info: Option<CreatureFormationInfoLikeCpp>,
    pub vehicle_id: Option<u32>,
    pub add_to_world_vehicle_reset_context: Option<CreatureAddToWorldVehicleResetContextLikeCpp>,
    pub equipment_id: u8,
    pub original_equipment_id: i8,
}

impl Default for CreatureLifecycleMetadata {
    fn default() -> Self {
        Self {
            template_entry: 0,
            original_entry: 0,
            difficulty_id: 0,
            ai_name: String::new(),
            script_name: String::new(),
            unit_class: 0,
            classification: 0,
            flags_extra: 0,
            ground_movement_type: CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: CreatureFlightMovementType::None as u8,
            creature_type: 0,
            type_flags: 0,
            selected_level: 0,
            selected_display_id: 0,
            selected_model_dimensions: None,
            template_scale: 1.0,
            speed_walk: 1.0,
            speed_run: 1.0,
            spawn_id: 0,
            spawn_map_id: 0,
            spawn_instance_id: 0,
            spawn_position: Position::ZERO,
            home_position: Position::ZERO,
            phase_id: None,
            phase_group: None,
            terrain_swap_map: None,
            spawn_group_id: None,
            spawn_group_name: None,
            pool_id: None,
            string_id: None,
            is_spawn_active: true,
            inactive_by_spawn_group: false,
            duplicate_spawn_found: false,
            add_to_map_requested: false,
            map_insertion_requested: false,
            dynamic_spawn: false,
            is_summon_like_cpp: false,
            formation_info: None,
            vehicle_id: None,
            add_to_world_vehicle_reset_context: None,
            equipment_id: 0,
            original_equipment_id: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureLifecycleStep {
    SetMapAndPhase,
    ApplyRespawnCompatibility,
    LookupTemplateAndDifficulty,
    RelocateAndValidatePosition,
    InitEntryAndCreateFromProto,
    SelectLevel,
    UpdateLevelDependantStats,
    ApplyAddonEquipmentSparringHoverScriptFlags,
    InitializeThreatManager,
    LoadFromDbSpawnHomeRespawnInactiveChecks,
    SetSpawnHealthDefaultMovementAndStringId,
    AddToMap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureLifecyclePlan {
    steps: Vec<CreatureLifecycleStep>,
}

impl CreatureLifecyclePlan {
    pub fn trinity_create_load_from_db() -> Self {
        Self {
            steps: vec![
                CreatureLifecycleStep::SetMapAndPhase,
                CreatureLifecycleStep::ApplyRespawnCompatibility,
                CreatureLifecycleStep::LookupTemplateAndDifficulty,
                CreatureLifecycleStep::RelocateAndValidatePosition,
                CreatureLifecycleStep::InitEntryAndCreateFromProto,
                CreatureLifecycleStep::SelectLevel,
                CreatureLifecycleStep::UpdateLevelDependantStats,
                CreatureLifecycleStep::ApplyAddonEquipmentSparringHoverScriptFlags,
                CreatureLifecycleStep::InitializeThreatManager,
                CreatureLifecycleStep::LoadFromDbSpawnHomeRespawnInactiveChecks,
                CreatureLifecycleStep::SetSpawnHealthDefaultMovementAndStringId,
                CreatureLifecycleStep::AddToMap,
            ],
        }
    }

    pub fn steps(&self) -> &[CreatureLifecycleStep] {
        &self.steps
    }

    pub fn position_of(&self, step: CreatureLifecycleStep) -> Option<usize> {
        self.steps.iter().position(|candidate| *candidate == step)
    }

    pub fn occurs_before(
        &self,
        before: CreatureLifecycleStep,
        after: CreatureLifecycleStep,
    ) -> bool {
        match (self.position_of(before), self.position_of(after)) {
            (Some(before_index), Some(after_index)) => before_index < after_index,
            _ => false,
        }
    }
}

impl Default for CreatureLifecyclePlan {
    fn default() -> Self {
        Self::trinity_create_load_from_db()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureRuntimeEvadeReason {
    Boundary,
    NoPath,
    ForcedDespawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureRuntimeAction {
    NotifyJustAppeared,
    SaveRespawnTime,
    ClearTarget,
    ClearNpcFlags,
    ClearMount,
    Deactivate,
    ClearAssistanceSearch,
    ClearTapList,
    ResetPlayerDamageReq,
    ResetCannotReachTarget,
    ClearErasableUnitState,
    InitializeMotion,
    ResetAi,
    LoadAddonAndSparring,
    UpdateMovementFlags,
    UpdateLoot,
    RemoveLoot,
    RemoveAllAuras,
    CorpseRemovedAiHook,
    RelocateToRespawnPosition,
    DestroyVisibility,
    UpdateVisibility,
    ResetPickpocketLoot,
    RestoreOriginalEntry,
    SelectLevel,
    ResetDisplay,
    ResetReactState,
    UpdatePool,
    RequestMapRespawn,
    RequestObjectRemove,
    RequestDelayedForcedDespawn,
    BoundaryCheck,
    CombatPulse,
    AiUpdateTick,
    MeleeAttackIfReady,
    RegenerateHealth,
    RegeneratePower,
    Evade(CreatureRuntimeEvadeReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureRuntimePlan {
    actions: Vec<CreatureRuntimeAction>,
}

impl CreatureRuntimePlan {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    pub fn push(&mut self, action: CreatureRuntimeAction) {
        self.actions.push(action);
    }

    pub fn extend<I>(&mut self, actions: I)
    where
        I: IntoIterator<Item = CreatureRuntimeAction>,
    {
        self.actions.extend(actions);
    }

    pub fn actions(&self) -> &[CreatureRuntimeAction] {
        &self.actions
    }

    pub fn contains(&self, action: CreatureRuntimeAction) -> bool {
        self.actions.contains(&action)
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

impl Default for CreatureRuntimePlan {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureRuntimeState {
    pub appeared_notified: bool,
    pub respawn_requested: bool,
    pub remove_corpse_requested: bool,
    pub forced_despawn_pending: bool,
    pub save_respawn_requested: bool,
    pub ai_reset_requested: bool,
    pub visibility_update_requested: bool,
    pub visibility_destroy_requested: bool,
    pub map_respawn_requested: bool,
    pub object_remove_requested: bool,
    pub evade_requested: Option<CreatureRuntimeEvadeReason>,
    pub corpse_removed_count: u32,
    pub loot_updated_count: u32,
    pub loot_removed_count: u32,
    pub pickpocket_reset_count: u32,
    pub has_loot_recipient: bool,
    pub movement_flags: MovementFlag,
}

impl Default for CreatureRuntimeState {
    fn default() -> Self {
        Self {
            appeared_notified: false,
            respawn_requested: false,
            remove_corpse_requested: false,
            forced_despawn_pending: false,
            save_respawn_requested: false,
            ai_reset_requested: false,
            visibility_update_requested: false,
            visibility_destroy_requested: false,
            map_respawn_requested: false,
            object_remove_requested: false,
            evade_requested: None,
            corpse_removed_count: 0,
            loot_updated_count: 0,
            loot_removed_count: 0,
            pickpocket_reset_count: 0,
            has_loot_recipient: false,
            movement_flags: MovementFlag::NONE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureRuntimeUpdateContext {
    pub ai_enabled: bool,
    pub is_engaged: bool,
    pub in_evade_mode: bool,
    pub is_dungeon: bool,
    pub is_raid: bool,
    pub has_map_players: bool,
    pub cannot_reach_target: bool,
    pub allow_cannot_reach_regen: bool,
    pub is_polymorphed: bool,
    pub has_loot: bool,
    pub has_personal_loot: bool,
}

impl Default for CreatureRuntimeUpdateContext {
    fn default() -> Self {
        Self {
            ai_enabled: true,
            is_engaged: false,
            in_evade_mode: false,
            is_dungeon: false,
            is_raid: false,
            has_map_players: false,
            cannot_reach_target: false,
            allow_cannot_reach_regen: true,
            is_polymorphed: false,
            has_loot: false,
            has_personal_loot: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Creature {
    unit: Unit,
    player_damage_req: u32,
    dont_clear_tap_list_on_evade: bool,
    pickpocket_loot_restore: i64,
    corpse_remove_time: i64,
    respawn_time: i64,
    respawn_delay: u32,
    corpse_delay: u32,
    ignore_corpse_decay_ratio: bool,
    wander_distance: f32,
    boundary_check_time: u32,
    combat_pulse_time: u32,
    combat_pulse_delay: u32,
    react_state: ReactState,
    default_movement_type: MovementGeneratorType,
    spawn_id: u64,
    equipment_id: u8,
    original_equipment_id: i8,
    already_call_assistance: bool,
    already_searched_assistance: bool,
    cannot_reach_target: bool,
    cannot_reach_timer: u32,
    melee_damage_school_mask: u32,
    original_entry: u32,
    trigger_just_appeared: bool,
    respawn_compatibility_mode: bool,
    last_damaged_time: i64,
    regenerate_health: bool,
    is_missing_can_swim_flag_out_of_combat: bool,
    gossip_menu_id: u32,
    sparring_health_pct: u8,
    regen_timer: u32,
    spells: [u32; MAX_CREATURE_SPELLS],
    disable_reputation_gain: bool,
    sight_distance: f32,
    combat_distance: f32,
    loot_mode: u16,
    is_temp_world_object: bool,
    grid_unload_cleanup_before_delete_count: u32,
    grid_unload_delete_requested: bool,
    grid_unload_respawn_relocation_requested: bool,
    owned_dynamic_objects: Vec<ObjectGuid>,
    removed_dynamic_objects_from_grid_unload: Vec<ObjectGuid>,
    owned_area_triggers: Vec<ObjectGuid>,
    removed_area_triggers_from_grid_unload: Vec<ObjectGuid>,
    lifecycle_metadata: CreatureLifecycleMetadata,
    runtime_state: CreatureRuntimeState,
    ai_ownership: CreatureAiOwnershipState,
    tap_list: Vec<ObjectGuid>,
    attack_reputation_faction_id: Option<u32>,
    is_contested_guard_faction: bool,
}

impl Creature {
    pub fn new(is_world_object: bool) -> Self {
        let mut unit = Unit::new(is_world_object);
        unit.set_type(TypeId::Unit, TypeMask::OBJECT | TypeMask::UNIT);
        unit.set_power_index(PowerType::Mana, Some(0));
        unit.set_power_index(PowerType::ComboPoints, Some(2));

        Self {
            unit,
            player_damage_req: 0,
            dont_clear_tap_list_on_evade: false,
            pickpocket_loot_restore: 0,
            corpse_remove_time: 0,
            respawn_time: 0,
            respawn_delay: DEFAULT_RESPAWN_DELAY_SECS,
            corpse_delay: DEFAULT_CORPSE_DELAY_SECS,
            ignore_corpse_decay_ratio: false,
            wander_distance: 0.0,
            boundary_check_time: DEFAULT_BOUNDARY_CHECK_TIME_MS,
            combat_pulse_time: 0,
            combat_pulse_delay: 0,
            react_state: ReactState::Aggressive,
            default_movement_type: MovementGeneratorType::Idle,
            spawn_id: 0,
            equipment_id: 0,
            original_equipment_id: 0,
            already_call_assistance: false,
            already_searched_assistance: false,
            cannot_reach_target: false,
            cannot_reach_timer: 0,
            melee_damage_school_mask: 0x1,
            original_entry: 0,
            trigger_just_appeared: true,
            respawn_compatibility_mode: false,
            last_damaged_time: 0,
            regenerate_health: true,
            is_missing_can_swim_flag_out_of_combat: false,
            gossip_menu_id: 0,
            sparring_health_pct: 0,
            regen_timer: CREATURE_REGEN_INTERVAL_MS,
            spells: [0; MAX_CREATURE_SPELLS],
            disable_reputation_gain: false,
            sight_distance: DEFAULT_MONSTER_SIGHT_DISTANCE,
            combat_distance: 0.0,
            loot_mode: LOOT_MODE_DEFAULT,
            is_temp_world_object: false,
            grid_unload_cleanup_before_delete_count: 0,
            grid_unload_delete_requested: false,
            grid_unload_respawn_relocation_requested: false,
            owned_dynamic_objects: Vec::new(),
            removed_dynamic_objects_from_grid_unload: Vec::new(),
            owned_area_triggers: Vec::new(),
            removed_area_triggers_from_grid_unload: Vec::new(),
            lifecycle_metadata: CreatureLifecycleMetadata::default(),
            runtime_state: CreatureRuntimeState::default(),
            ai_ownership: CreatureAiOwnershipState::default(),
            tap_list: Vec::new(),
            attack_reputation_faction_id: None,
            is_contested_guard_faction: false,
        }
    }

    pub fn create_from_lifecycle(record: CreatureCreateLifecycleRecord) -> Self {
        let mut creature = Self::new(false);
        creature.apply_create_lifecycle(record);
        creature
    }

    pub fn load_from_db_lifecycle(record: CreatureLoadFromDbLifecycleRecord) -> Self {
        let mut creature = Self::create_from_lifecycle(record.create);
        creature.apply_load_from_db_lifecycle(&record.spawn);
        creature
    }

    pub fn apply_create_lifecycle(&mut self, record: CreatureCreateLifecycleRecord) {
        let template = &record.template;
        let spawn = record.spawn.as_ref();
        let map_id = spawn.map(|spawn| spawn.map_id).unwrap_or(record.map_id);
        let instance_id = spawn
            .map(|spawn| spawn.instance_id)
            .unwrap_or(record.instance_id);
        let position = spawn.map(|spawn| spawn.position).unwrap_or(record.position);
        let home_position = spawn
            .map(|spawn| spawn.home_position)
            .unwrap_or(record.position);
        let equipment_id = spawn
            .and_then(|spawn| spawn.equipment_id)
            .unwrap_or(record.selected_equipment_id);
        let original_equipment_id = spawn
            .and_then(|spawn| spawn.original_equipment_id)
            .unwrap_or(record.selected_original_equipment_id);

        self.unit.world_mut().object_mut().create(record.guid);
        let _ = self.unit.world_mut().set_map(map_id, instance_id);
        self.unit.world_mut().relocate(position);
        self.unit.world_mut().set_name(template.name.clone());

        self.unit.world_mut().object_mut().set_entry(record.entry);
        self.original_entry = template.original_entry;
        self.unit.world_mut().object_mut().set_scale(template.scale);
        self.unit.set_race(0);
        self.unit.set_class(template.unit_class);
        self.set_faction(template.faction);
        self.set_display_id(
            record.selected_display_id,
            true,
            record
                .selected_model_dimensions
                .or(template.model_dimensions),
        );
        self.spells = template.spells;
        self.equipment_id = equipment_id;
        self.original_equipment_id = original_equipment_id;
        if record.vehicle_id.is_some() {
            // C++ `Creature::CreateFromProto` calls `CreateVehicleKit(vehId, entry, true)` here.
            // The bounded seam creates the local DB2 seat-backed `Vehicle` only when the caller
            // resolved a real `VehicleEntry`; a missing input preserves identity metadata but
            // represents `CreateVehicleKit` returning false.
            let create_input = record.vehicle_kit_create_input;
            let vehicle_id = create_input
                .as_ref()
                .map_or(record.vehicle_id, |input| Some(input.vehicle_id));
            let loading = create_input.as_ref().map_or(true, |input| input.loading);
            let creature_entry = create_input
                .as_ref()
                .map_or(record.entry, |input| input.creature_entry);
            let seat_defs = create_input.map(|input| input.seat_defs);
            self.unit
                .subsystems_mut()
                .vehicle
                .create_vehicle_kit_like_cpp(
                    record.guid,
                    position,
                    vehicle_id,
                    creature_entry,
                    loading,
                    seat_defs,
                );
        }
        self.default_movement_type = spawn
            .map(|spawn| spawn.movement_type)
            .unwrap_or(template.movement_type);
        self.set_corpse_delay(record.corpse_delay, record.ignore_corpse_decay_ratio);
        self.set_respawn_compatibility_mode(!record.dynamic);
        if let Some(spawn) = spawn {
            self.apply_spawn_lifecycle(spawn);
        }

        self.unit.set_level(record.selected_level);
        self.set_power_type(record.stats.power_type);
        self.unit.set_max_health(record.stats.max_health);
        self.unit.set_health(record.stats.health);
        self.unit
            .set_max_power(PowerType::Mana, record.stats.max_mana);
        self.unit.set_power(PowerType::Mana, record.stats.mana);
        self.unit.set_weapon_damage(
            WeaponAttackType::BaseAttack,
            record.stats.min_damage,
            record.stats.max_damage,
        );
        self.ai_ownership.home_position = home_position;
        self.ai_ownership.move_target = None;
        self.ai_ownership.move_start_ms = 0;
        self.ai_ownership.move_duration_ms = 0;
        self.ai_ownership.state = CreatureAiState::Idle;
        self.ai_ownership.death_time_ms = None;
        self.ai_ownership.corpse_despawn_at_ms = None;
        self.ai_ownership.respawn_time_secs = spawn
            .map(|spawn| u64::from(spawn.respawn_delay))
            .unwrap_or(u64::from(DEFAULT_RESPAWN_DELAY_SECS));
        self.ai_ownership.wander_radius = spawn.map(|spawn| spawn.wander_distance).unwrap_or(0.0);
        self.ai_ownership.aggro_radius = DEFAULT_MONSTER_SIGHT_DISTANCE;
        self.ai_ownership.display_id = record.selected_display_id;
        self.ai_ownership.faction = template.faction;
        self.ai_ownership.min_damage = record.stats.min_damage.max(0.0) as u32;
        self.ai_ownership.max_damage = record.stats.max_damage.max(0.0) as u32;

        self.lifecycle_metadata = CreatureLifecycleMetadata {
            template_entry: template.entry,
            original_entry: template.original_entry,
            difficulty_id: template.difficulty_id,
            ai_name: template.ai_name.clone(),
            script_name: template.script_name.clone(),
            unit_class: template.unit_class,
            classification: template.classification,
            flags_extra: template.flags_extra,
            ground_movement_type: normalize_creature_ground_movement_type_like_cpp(
                template.ground_movement_type,
            ),
            swim_allowed: template.swim_allowed,
            flight_movement_type: normalize_creature_flight_movement_type_like_cpp(
                template.flight_movement_type,
            ),
            creature_type: template.creature_type,
            type_flags: template.type_flags,
            selected_level: record.selected_level,
            selected_display_id: record.selected_display_id,
            selected_model_dimensions: record
                .selected_model_dimensions
                .or(template.model_dimensions),
            template_scale: template.scale,
            speed_walk: template.speed_walk,
            speed_run: template.speed_run,
            spawn_id: spawn.map(|spawn| spawn.spawn_id).unwrap_or(0),
            spawn_map_id: map_id,
            spawn_instance_id: instance_id,
            spawn_position: position,
            home_position,
            phase_id: spawn.and_then(|spawn| spawn.phase_id),
            phase_group: spawn.and_then(|spawn| spawn.phase_group),
            terrain_swap_map: spawn.and_then(|spawn| spawn.terrain_swap_map),
            spawn_group_id: spawn.and_then(|spawn| spawn.spawn_group_id),
            spawn_group_name: spawn.and_then(|spawn| spawn.spawn_group_name.clone()),
            pool_id: spawn.and_then(|spawn| spawn.pool_id),
            string_id: spawn.and_then(|spawn| spawn.string_id.clone()),
            is_spawn_active: spawn.map(|spawn| spawn.is_active).unwrap_or(true),
            inactive_by_spawn_group: spawn
                .map(|spawn| spawn.inactive_by_spawn_group)
                .unwrap_or(false),
            duplicate_spawn_found: spawn
                .map(|spawn| spawn.duplicate_spawn_found)
                .unwrap_or(false),
            add_to_map_requested: spawn.map(|spawn| spawn.add_to_map).unwrap_or(false),
            map_insertion_requested: spawn.map(|spawn| spawn.add_to_map).unwrap_or(false),
            dynamic_spawn: record.dynamic,
            is_summon_like_cpp: false,
            formation_info: None,
            vehicle_id: record.vehicle_id,
            add_to_world_vehicle_reset_context: record.add_to_world_vehicle_reset_context,
            equipment_id,
            original_equipment_id,
        };

        self.clear_data_changes();
    }

    pub fn apply_load_from_db_lifecycle(&mut self, spawn: &CreatureSpawnLifecycleRecord) {
        self.apply_spawn_lifecycle(spawn);
        self.lifecycle_metadata.spawn_id = spawn.spawn_id;
        self.lifecycle_metadata.spawn_map_id = spawn.map_id;
        self.lifecycle_metadata.spawn_instance_id = spawn.instance_id;
        self.lifecycle_metadata.spawn_position = spawn.position;
        self.lifecycle_metadata.home_position = spawn.home_position;
        self.lifecycle_metadata.phase_id = spawn.phase_id;
        self.lifecycle_metadata.phase_group = spawn.phase_group;
        self.lifecycle_metadata.terrain_swap_map = spawn.terrain_swap_map;
        self.lifecycle_metadata.spawn_group_id = spawn.spawn_group_id;
        self.lifecycle_metadata.spawn_group_name = spawn.spawn_group_name.clone();
        self.lifecycle_metadata.pool_id = spawn.pool_id;
        self.lifecycle_metadata.string_id = spawn.string_id.clone();
        self.lifecycle_metadata.is_spawn_active = spawn.is_active;
        self.lifecycle_metadata.inactive_by_spawn_group = spawn.inactive_by_spawn_group;
        self.lifecycle_metadata.duplicate_spawn_found = spawn.duplicate_spawn_found;
        self.lifecycle_metadata.add_to_map_requested = spawn.add_to_map;
        self.lifecycle_metadata.map_insertion_requested = spawn.add_to_map;
        if let Some(equipment_id) = spawn.equipment_id {
            self.lifecycle_metadata.equipment_id = equipment_id;
        }
        if let Some(original_equipment_id) = spawn.original_equipment_id {
            self.lifecycle_metadata.original_equipment_id = original_equipment_id;
        }
        self.clear_data_changes();
    }

    fn apply_spawn_lifecycle(&mut self, spawn: &CreatureSpawnLifecycleRecord) {
        self.set_spawn_id(spawn.spawn_id);
        self.set_respawn_compatibility_mode(spawn.respawn_compatibility_mode);
        self.wander_distance = spawn.wander_distance;
        self.set_respawn_delay(spawn.respawn_delay);
        self.set_respawn_time(spawn.respawn_time);
        self.default_movement_type = spawn.movement_type;
        if let Some(equipment_id) = spawn.equipment_id {
            self.equipment_id = equipment_id;
        }
        if let Some(original_equipment_id) = spawn.original_equipment_id {
            self.original_equipment_id = original_equipment_id;
        }
        let _ = self
            .unit
            .world_mut()
            .set_map(spawn.map_id, spawn.instance_id);
        self.unit.world_mut().relocate(spawn.position);
        self.ai_ownership.home_position = spawn.home_position;
        self.ai_ownership.move_target = None;
        self.ai_ownership.respawn_time_secs = u64::from(spawn.respawn_delay);
        self.ai_ownership.wander_radius = spawn.wander_distance;
    }

    pub const fn lifecycle_metadata(&self) -> &CreatureLifecycleMetadata {
        &self.lifecycle_metadata
    }

    pub fn is_world_boss_like_cpp(&self) -> bool {
        if self.is_summon_like_cpp() {
            return false;
        }

        CreatureTypeFlags::from_bits_truncate(self.lifecycle_metadata.type_flags)
            .contains(CreatureTypeFlags::BOSS_MOB)
    }

    pub fn set_type_flags_runtime_like_cpp(&mut self, type_flags: u32) {
        self.lifecycle_metadata.type_flags = type_flags;
    }

    pub fn is_civilian_like_cpp(&self) -> bool {
        CreatureFlagsExtra::from_bits_truncate(self.lifecycle_metadata.flags_extra)
            .contains(CreatureFlagsExtra::CIVILIAN)
    }

    pub fn flight_movement_type_like_cpp(&self) -> u8 {
        self.lifecycle_metadata.flight_movement_type
    }

    /// C++ `Creature::CanWalk()` is true when the movement template allows
    /// ground movement (`Ground != None`).
    pub fn can_walk_like_cpp(&self) -> bool {
        self.lifecycle_metadata.ground_movement_type != CreatureGroundMovementType::None as u8
    }

    /// C++ `Creature::CanEnterWater()` returns true when `Unit::CanSwim()`,
    /// `IsPet()`, or the creature movement template allows swimming.
    pub fn can_enter_water_like_cpp(&self) -> bool {
        if self.can_swim_like_cpp() {
            return true;
        }
        if self.unit.world().object().guid().is_pet() {
            return true;
        }
        self.lifecycle_metadata.swim_allowed
    }

    /// Represented C++ `Unit::CanSwim()` for creature-backed units.
    pub fn can_swim_like_cpp(&self) -> bool {
        let unit_flags = self.unit.unit_flags_like_cpp();
        if unit_flags.contains(UnitFlags::CANT_SWIM) {
            return false;
        }
        if unit_flags.contains(UnitFlags::PLAYER_CONTROLLED) {
            return true;
        }
        if self
            .unit
            .unit_flags2_like_cpp()
            .contains(UnitFlags2::AI_WILL_ONLY_SWIM_IF_TARGET_SWIMS)
        {
            return false;
        }
        if unit_flags.contains(UnitFlags::PET_IN_COMBAT) {
            return true;
        }
        unit_flags.intersects(UnitFlags::RENAME | UnitFlags::CAN_SWIM)
    }

    /// C++ `Creature::CanFly()` returns true when the movement template allows
    /// flight (`Flight != None`) or runtime movement flags say the unit is
    /// flying (`MOVEMENTFLAG_FLYING | MOVEMENTFLAG_DISABLE_GRAVITY`).
    pub fn can_fly_like_cpp(&self) -> bool {
        self.lifecycle_metadata.flight_movement_type != CreatureFlightMovementType::None as u8
            || self.is_flying_like_cpp()
    }

    pub fn is_flying_like_cpp(&self) -> bool {
        self.runtime_state
            .movement_flags
            .intersects(MovementFlag::FLYING | MovementFlag::DISABLE_GRAVITY)
    }

    pub const fn movement_flags_like_cpp(&self) -> MovementFlag {
        self.runtime_state.movement_flags
    }

    pub fn set_movement_flags_runtime_like_cpp(&mut self, movement_flags: MovementFlag) {
        self.runtime_state.movement_flags = movement_flags;
    }

    pub fn add_to_world_vehicle_reset_context_like_cpp(
        &self,
    ) -> Option<&CreatureAddToWorldVehicleResetContextLikeCpp> {
        self.lifecycle_metadata
            .add_to_world_vehicle_reset_context
            .as_ref()
    }

    pub fn set_add_to_world_vehicle_reset_context_like_cpp(
        &mut self,
        context: Option<CreatureAddToWorldVehicleResetContextLikeCpp>,
    ) {
        self.lifecycle_metadata.add_to_world_vehicle_reset_context = context;
    }

    pub const fn is_summon_like_cpp(&self) -> bool {
        self.lifecycle_metadata.is_summon_like_cpp
    }

    pub fn set_summon_like_cpp(&mut self, is_summon: bool) {
        self.lifecycle_metadata.is_summon_like_cpp = is_summon;
    }

    pub const fn formation_info_like_cpp(&self) -> Option<&CreatureFormationInfoLikeCpp> {
        self.lifecycle_metadata.formation_info.as_ref()
    }

    pub fn set_formation_info_like_cpp(&mut self, info: Option<CreatureFormationInfoLikeCpp>) {
        self.lifecycle_metadata.formation_info = info;
    }

    /// Represented C++ `Creature::SearchFormation()` branch.
    ///
    /// C++ anchor: `Creature.cpp:379-389`. This only consumes explicit
    /// caller-provided `FormationInfo` evidence already stored on the creature.
    /// It does not query DB, scan spawn groups, or own a real `FormationMgr`.
    pub fn search_formation_like_cpp(&self) -> CreatureSearchFormationOutcomeLikeCpp {
        let spawn_id = self.spawn_id();
        let is_summon = self.is_summon_like_cpp();
        if is_summon {
            return CreatureSearchFormationOutcomeLikeCpp {
                spawn_id,
                is_summon,
                formation_info_found: self.lifecycle_metadata.formation_info.is_some(),
                leader_spawn_id: None,
                add_to_group_requested: false,
            };
        }

        if spawn_id == 0 {
            return CreatureSearchFormationOutcomeLikeCpp {
                spawn_id,
                is_summon,
                formation_info_found: self.lifecycle_metadata.formation_info.is_some(),
                leader_spawn_id: None,
                add_to_group_requested: false,
            };
        }

        let Some(formation_info) = self.lifecycle_metadata.formation_info else {
            return CreatureSearchFormationOutcomeLikeCpp {
                spawn_id,
                is_summon,
                formation_info_found: false,
                leader_spawn_id: None,
                add_to_group_requested: false,
            };
        };

        CreatureSearchFormationOutcomeLikeCpp {
            spawn_id,
            is_summon,
            formation_info_found: true,
            leader_spawn_id: Some(formation_info.leader_spawn_id),
            add_to_group_requested: true,
        }
    }

    /// Represented C++ `Creature::AIM_Initialize()` / `AIM_Create()` seam.
    ///
    /// C++ anchors: `Creature.cpp:1026-1044` (`AIM_Create`, `AIM_Initialize`)
    /// and `Creature.cpp:1046-1060` (`Motion_Initialize`). This records local
    /// evidence only: it does not instantiate real AI, run `InitializeAI`, call
    /// `CreatureGroup::FormationReset`, query `CreatureGroup::IsFormed`, move a
    /// `MotionMaster`, or reset a vehicle kit. The vehicle reset remains the
    /// following AddToMap seam representing `if (GetVehicleKit()) Reset()`.
    pub fn aim_initialize_like_cpp(&self) -> CreatureAimInitializeOutcomeLikeCpp {
        let spawn_id = self.spawn_id();
        let formation_info = self.formation_info_like_cpp();
        let formation_present = formation_info.is_some();
        let formation_leader = formation_info.is_some_and(|info| info.leader_spawn_id == spawn_id);
        let motion_initialize_requires_formed_state = formation_present && !formation_leader;

        CreatureAimInitializeOutcomeLikeCpp {
            guid: self.guid(),
            spawn_id,
            aim_create_represented: true,
            motion_initialize_represented: true,
            formation_present,
            formation_leader,
            // C++ non-leader formed groups call MoveIdle() and return, but this
            // represented seam has no real CreatureGroup::IsFormed() state yet.
            formation_move_idle_represented: false,
            motion_initialize_requires_formed_state,
            motion_master_initialize_represented: !motion_initialize_requires_formed_state,
            ai_selected_represented: true,
            ai_initialize_represented: true,
            vehicle_reset_expected: self.unit().subsystems().vehicle.kit.is_some(),
            succeeded: true,
        }
    }

    pub fn clear_data_changes(&mut self) {
        self.unit.clear_unit_data_changes();
        self.unit.world_mut().object_mut().clear_update_mask(false);
    }

    pub const fn unit(&self) -> &Unit {
        &self.unit
    }

    pub fn unit_mut(&mut self) -> &mut Unit {
        &mut self.unit
    }

    pub const fn ai_ownership(&self) -> &CreatureAiOwnershipState {
        &self.ai_ownership
    }

    pub fn ai_ownership_mut(&mut self) -> &mut CreatureAiOwnershipState {
        &mut self.ai_ownership
    }

    pub const fn ai_state(&self) -> CreatureAiState {
        self.ai_ownership.state
    }

    pub fn set_ai_state(&mut self, state: CreatureAiState) {
        self.ai_ownership.state = state;
    }

    pub const fn ai_home_position(&self) -> Position {
        self.ai_ownership.home_position
    }

    pub fn set_ai_home_position(&mut self, position: Position) {
        self.ai_ownership.home_position = position;
    }

    pub fn record_ai_movement_inform(&mut self, movement_type: u8, movement_id: u32) {
        self.ai_ownership.last_movement_inform = Some(CreatureMovementInform {
            movement_type,
            movement_id,
        });
    }

    pub fn take_ai_movement_inform(&mut self) -> Option<CreatureMovementInform> {
        self.ai_ownership.last_movement_inform.take()
    }

    pub const fn ai_position(&self) -> Position {
        self.unit.world().position()
    }

    pub fn set_ai_position(&mut self, position: Position) {
        self.unit.world_mut().relocate(position);
    }

    pub const fn ai_guid(&self) -> ObjectGuid {
        self.unit.world().object().guid()
    }

    pub const fn ai_entry(&self) -> u32 {
        self.unit.world().object().entry()
    }

    pub const fn guid(&self) -> ObjectGuid {
        self.ai_guid()
    }

    pub const fn entry(&self) -> u32 {
        self.ai_entry()
    }

    pub fn ai_level(&self) -> u8 {
        self.unit.data().level.clamp(0, u8::MAX as i32) as u8
    }

    pub const fn ai_current_health(&self) -> u64 {
        self.unit.data().health
    }

    pub const fn ai_max_health(&self) -> u64 {
        self.unit.data().max_health
    }

    pub fn ai_is_alive(&self) -> bool {
        self.unit.is_alive()
            && self.ai_current_health() > 0
            && self.ai_ownership.state != CreatureAiState::Dead
    }

    pub fn enter_ai_combat(&mut self, attacker: ObjectGuid) {
        self.ai_ownership.state = CreatureAiState::InCombat;
        self.ai_ownership.combat_target = Some(attacker);
        self.ai_ownership.move_target = None;
        self.unit.set_attacking(Some(attacker));
    }

    pub fn reset_ai_combat(&mut self, now_ms: u64) {
        self.ai_ownership.state = CreatureAiState::Returning;
        self.ai_ownership.combat_target = None;
        self.ai_ownership.move_target = Some(self.ai_ownership.home_position);
        self.ai_ownership.move_start_ms = now_ms;
        self.ai_ownership.death_time_ms = None;
        self.ai_ownership.corpse_despawn_at_ms = None;
        self.unit.set_attacking(None);
        let max_health = self.unit.data().max_health;
        self.unit.set_death_state(DeathState::Alive);
        self.unit.set_health(max_health);
        self.last_damaged_time = 0;
    }

    /// Apply damage and return `true` when this call killed the creature.
    pub fn take_ai_damage(&mut self, damage: u32, now_ms: u64) -> bool {
        if self.apply_ai_damage_before_death_state_like_cpp(damage, now_ms) {
            self.mark_ai_dead(now_ms);
            true
        } else {
            false
        }
    }

    pub fn apply_ai_damage_before_death_state_like_cpp(
        &mut self,
        damage: u32,
        now_ms: u64,
    ) -> bool {
        self.apply_ai_damage_before_death_state_at_game_time_like_cpp(
            damage,
            now_ms,
            game_time_secs_like_cpp(),
        )
    }

    pub fn apply_ai_damage_before_death_state_at_game_time_like_cpp(
        &mut self,
        damage: u32,
        now_ms: u64,
        game_time_secs: i64,
    ) -> bool {
        if !self.ai_is_alive() {
            return false;
        }

        let remaining = self.ai_current_health().saturating_sub(u64::from(damage));
        self.unit.set_health(remaining);
        if remaining > 0
            && damage > 0
            && !self
                .unit
                .subsystems()
                .control
                .owner_guid
                .is_some_and(|owner_guid| owner_guid.is_player())
        {
            self.last_damaged_time =
                game_time_secs.saturating_add(MAX_AGGRO_RESET_TIME_SECS_LIKE_CPP);
        }
        if remaining == 0 {
            self.ai_ownership.state = CreatureAiState::Dead;
            self.ai_ownership.combat_target = None;
            self.ai_ownership.move_target = None;
            self.ai_ownership.death_time_ms = Some(now_ms);
            self.respawn_delay =
                self.ai_ownership.respawn_time_secs.min(u64::from(u32::MAX)) as u32;
            true
        } else {
            false
        }
    }

    pub fn mark_ai_dead(&mut self, now_ms: u64) {
        self.ai_ownership.state = CreatureAiState::Dead;
        self.ai_ownership.combat_target = None;
        self.ai_ownership.move_target = None;
        self.ai_ownership.death_time_ms = Some(now_ms);
        self.unit.set_health(0);
        self.respawn_delay = self.ai_ownership.respawn_time_secs.min(u64::from(u32::MAX)) as u32;
        self.set_death_state_runtime(DeathState::JustDied, now_ms.min(i64::MAX as u64) as i64);
        self.unit.set_health(0);
    }

    pub fn complete_ai_death_state_after_kill_hooks_like_cpp(&mut self, now_ms: u64) {
        if self.ai_ownership.state != CreatureAiState::Dead || self.unit.is_dead() {
            return;
        }
        self.set_death_state_runtime(DeathState::JustDied, now_ms.min(i64::MAX as u64) as i64);
        self.unit.set_health(0);
    }

    pub fn apply_corpse_loot_flags_after_death_state_like_cpp(
        &mut self,
        lootable: bool,
        can_skin: bool,
    ) {
        if lootable {
            self.unit
                .world_mut()
                .object_mut()
                .set_dynamic_flag(UnitDynFlags::Lootable as u32);
        }
        if can_skin {
            self.unit
                .world_mut()
                .object_mut()
                .set_dynamic_flag(UnitDynFlags::CanSkin as u32);
            let mut flags = self.unit.unit_flags_like_cpp();
            flags.insert(UnitFlags::SKINNABLE);
            self.unit.set_unit_flags_like_cpp(flags);
        }
    }

    pub fn respawn_ai(&mut self, now_ms: u64) {
        self.ai_ownership.state = CreatureAiState::Idle;
        self.ai_ownership.combat_target = None;
        self.ai_ownership.move_target = None;
        self.ai_ownership.move_start_ms = now_ms;
        self.ai_ownership.last_swing_ms = now_ms;
        self.ai_ownership.death_time_ms = None;
        self.ai_ownership.corpse_despawn_at_ms = None;
        self.ai_ownership.spline_id = self.ai_ownership.spline_id.saturating_add(1);
        self.unit.set_death_state(DeathState::Alive);
        self.unit.set_health(self.unit.data().max_health);
        self.unit
            .world_mut()
            .relocate(self.ai_ownership.home_position);
        self.unit.set_attacking(None);
        self.last_damaged_time = 0;
    }

    pub fn can_ai_wander(&self) -> bool {
        self.ai_ownership.npc_flags == 0 || (self.ai_ownership.npc_flags & 0x80) == 0
    }

    pub fn try_ai_aggro(&mut self, player_guid: ObjectGuid, player_pos: &Position) -> bool {
        self.try_ai_aggro_with_target_combat_reach_like_cpp(player_guid, player_pos, 0.0)
    }

    pub fn try_ai_aggro_with_target_combat_reach_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        player_pos: &Position,
        player_combat_reach: f32,
    ) -> bool {
        if !self.ai_is_alive() || self.ai_ownership.state == CreatureAiState::InCombat {
            return false;
        }

        if !self.has_react_state(ReactState::Aggressive) {
            return false;
        }

        if self.is_civilian_like_cpp() {
            return false;
        }

        if self.ai_ownership.aggro_radius <= 0.0 {
            return false;
        }

        if self
            .unit
            .unit_flags_like_cpp()
            .contains(UnitFlags::IMMUNE_TO_PC)
        {
            return false;
        }

        if !self.can_start_attack_z_range_like_cpp(player_pos, player_combat_reach) {
            return false;
        }

        if self.ai_position().distance(player_pos) <= self.ai_ownership.aggro_radius {
            self.enter_ai_combat(player_guid);
            true
        } else {
            false
        }
    }

    pub fn can_start_attack_z_range_like_cpp(
        &self,
        target_position: &Position,
        target_combat_reach: f32,
    ) -> bool {
        if self.can_fly_like_cpp() {
            return true;
        }

        let distance_z = ((self.ai_position().z - target_position.z).abs()
            - self.unit.world().combat_reach()
            - target_combat_reach.max(0.0))
        .max(0.0);
        distance_z <= CREATURE_Z_ATTACK_RANGE_LIKE_CPP + self.combat_distance.max(0.0)
    }

    pub fn should_ai_respawn(&self, now_ms: u64) -> bool {
        self.ai_ownership
            .death_time_ms
            .map(|death_ms| {
                now_ms
                    >= death_ms
                        .saturating_add(self.ai_ownership.respawn_time_secs.saturating_mul(1_000))
            })
            .unwrap_or(false)
    }

    pub fn set_ai_corpse_despawn_at(&mut self, corpse_despawn_at_ms: Option<u64>) {
        self.ai_ownership.corpse_despawn_at_ms = corpse_despawn_at_ms;
    }

    pub fn set_ai_identity_runtime(
        &mut self,
        display_id: u32,
        faction: u32,
        npc_flags: u32,
        unit_flags: u32,
    ) {
        self.ai_ownership.display_id = display_id;
        self.ai_ownership.faction = faction;
        self.ai_ownership.npc_flags = npc_flags;
        self.ai_ownership.unit_flags = unit_flags;
        self.unit.set_npc_flags_like_cpp(npc_flags);
        self.set_display_id(display_id, true, None);
        self.set_faction(faction);
    }

    pub fn set_npc_flags2_runtime_like_cpp(&mut self, npc_flags2: u32) {
        self.ai_ownership.npc_flags2 = npc_flags2;
        self.unit.set_npc_flags2_like_cpp(npc_flags2);
    }

    pub fn set_flags_extra_runtime_like_cpp(&mut self, flags_extra: u32) {
        self.lifecycle_metadata.flags_extra = flags_extra;
    }

    pub fn set_ai_identity_names_runtime_like_cpp(
        &mut self,
        ai_name: impl Into<String>,
        script_name: impl Into<String>,
    ) {
        self.lifecycle_metadata.ai_name = ai_name.into();
        self.lifecycle_metadata.script_name = script_name.into();
    }

    pub fn ground_movement_type_like_cpp(&self) -> u8 {
        self.lifecycle_metadata.ground_movement_type
    }

    pub fn set_ground_movement_type_runtime_like_cpp(&mut self, ground_movement_type: u8) {
        self.lifecycle_metadata.ground_movement_type =
            normalize_creature_ground_movement_type_like_cpp(ground_movement_type);
    }

    pub fn swim_allowed_like_cpp(&self) -> bool {
        self.lifecycle_metadata.swim_allowed
    }

    pub fn set_swim_allowed_runtime_like_cpp(&mut self, swim_allowed: bool) {
        self.lifecycle_metadata.swim_allowed = swim_allowed;
    }

    pub fn set_flight_movement_type_runtime_like_cpp(&mut self, flight_movement_type: u8) {
        self.lifecycle_metadata.flight_movement_type =
            normalize_creature_flight_movement_type_like_cpp(flight_movement_type);
    }

    pub fn configure_ai_runtime(
        &mut self,
        home_position: Position,
        aggro_radius: f32,
        wander_radius: f32,
        respawn_time_secs: u64,
    ) {
        self.ai_ownership.home_position = home_position;
        self.ai_ownership.aggro_radius = aggro_radius;
        self.ai_ownership.wander_radius = wander_radius;
        self.ai_ownership.respawn_time_secs = respawn_time_secs;
    }

    pub fn begin_ai_move(&mut self, dst: Position, now_ms: u64) {
        let dist = self.ai_position().distance(&dst);
        let duration_ms = ((dist / 2.5) * 1000.0) as u32;
        self.ai_ownership.move_target = Some(dst);
        self.ai_ownership.move_start_ms = now_ms;
        self.ai_ownership.move_duration_ms = duration_ms.max(500);
        self.ai_ownership.spline_id = self.ai_ownership.spline_id.saturating_add(1);
    }

    pub fn finish_ai_move(&mut self) {
        if let Some(dst) = self.ai_ownership.move_target.take() {
            self.unit.world_mut().relocate(dst);
        }
        self.ai_ownership.move_duration_ms = 0;
    }

    pub fn ai_movement_finished(&self, now_ms: u64) -> bool {
        self.ai_ownership.move_target.is_none()
            || now_ms.saturating_sub(self.ai_ownership.move_start_ms)
                >= u64::from(self.ai_ownership.move_duration_ms)
    }

    // Small compatibility aliases for callers that need canonical values without
    // reaching through Unit/WorldObject internals.
    pub fn level(&self) -> u8 {
        self.ai_level()
    }

    pub const fn current_health(&self) -> u64 {
        self.ai_current_health()
    }

    pub const fn max_health(&self) -> u64 {
        self.ai_max_health()
    }

    pub fn is_alive(&self) -> bool {
        self.ai_is_alive()
    }

    pub const fn position(&self) -> Position {
        self.ai_position()
    }

    pub const fn player_damage_req(&self) -> u32 {
        self.player_damage_req
    }

    pub const fn corpse_remove_time(&self) -> i64 {
        self.corpse_remove_time
    }

    pub const fn respawn_time(&self) -> i64 {
        self.respawn_time
    }

    pub fn set_respawn_time(&mut self, respawn_time: i64) {
        self.respawn_time = respawn_time;
    }

    pub const fn respawn_delay(&self) -> u32 {
        self.respawn_delay
    }

    pub fn set_respawn_delay(&mut self, delay: u32) {
        self.respawn_delay = delay;
    }

    pub const fn corpse_delay(&self) -> u32 {
        self.corpse_delay
    }

    pub fn set_corpse_delay(&mut self, delay: u32, ignore_corpse_decay_ratio: bool) {
        self.corpse_delay = delay;
        if ignore_corpse_decay_ratio {
            self.ignore_corpse_decay_ratio = true;
        }
    }

    pub const fn ignore_corpse_decay_ratio(&self) -> bool {
        self.ignore_corpse_decay_ratio
    }

    pub const fn wander_distance(&self) -> f32 {
        self.wander_distance
    }

    pub const fn boundary_check_time(&self) -> u32 {
        self.boundary_check_time
    }

    pub const fn combat_pulse_time(&self) -> u32 {
        self.combat_pulse_time
    }

    pub const fn combat_pulse_delay(&self) -> u32 {
        self.combat_pulse_delay
    }

    pub const fn react_state(&self) -> ReactState {
        self.react_state
    }

    pub fn set_react_state(&mut self, state: ReactState) {
        self.react_state = state;
    }

    pub fn has_react_state(&self, state: ReactState) -> bool {
        self.react_state == state
    }

    pub const fn default_movement_type(&self) -> MovementGeneratorType {
        self.default_movement_type
    }

    pub const fn spawn_id(&self) -> u64 {
        self.spawn_id
    }

    pub fn set_spawn_id(&mut self, spawn_id: u64) {
        self.spawn_id = spawn_id;
        self.lifecycle_metadata.spawn_id = spawn_id;
    }

    pub const fn equipment_id(&self) -> u8 {
        self.equipment_id
    }

    /// Represented bounded seam for TrinityCore `Creature::LoadEquipment(id, true)` callers.
    ///
    /// This only records the selected equipment id on the canonical creature state and
    /// lifecycle metadata. It does not load `creature_equip_template` items, update
    /// visible item fields, or fan out values updates.
    pub fn set_equipment_id_like_cpp(&mut self, equipment_id: u8) {
        self.equipment_id = equipment_id;
        self.lifecycle_metadata.equipment_id = equipment_id;
    }

    pub const fn original_equipment_id(&self) -> i8 {
        self.original_equipment_id
    }

    pub const fn already_call_assistance(&self) -> bool {
        self.already_call_assistance
    }

    pub const fn already_searched_assistance(&self) -> bool {
        self.already_searched_assistance
    }

    pub const fn cannot_reach_target(&self) -> bool {
        self.cannot_reach_target
    }

    pub fn set_cannot_reach_target_like_cpp(&mut self, cannot_reach: bool) {
        self.cannot_reach_target = cannot_reach;
        if !cannot_reach {
            self.cannot_reach_timer = 0;
        }
    }

    pub const fn cannot_reach_timer(&self) -> u32 {
        self.cannot_reach_timer
    }

    pub fn is_in_evade_mode_like_cpp(&self) -> bool {
        self.unit.has_unit_state(UnitState::EVADE.bits())
    }

    pub fn set_in_evade_mode_like_cpp(&mut self, in_evade_mode: bool) {
        if in_evade_mode {
            self.unit.add_unit_state(UnitState::EVADE.bits());
        } else {
            self.unit.clear_unit_state(UnitState::EVADE.bits());
        }
    }

    pub fn is_evading_attacks_like_cpp(&self) -> bool {
        self.is_in_evade_mode_like_cpp() || self.cannot_reach_target
    }

    pub const fn melee_damage_school_mask(&self) -> u32 {
        self.melee_damage_school_mask
    }

    pub const fn original_entry(&self) -> u32 {
        self.original_entry
    }

    pub const fn trigger_just_appeared(&self) -> bool {
        self.trigger_just_appeared
    }

    pub const fn respawn_compatibility_mode(&self) -> bool {
        self.respawn_compatibility_mode
    }

    pub fn set_respawn_compatibility_mode(&mut self, enabled: bool) {
        self.respawn_compatibility_mode = enabled;
    }

    pub const fn last_damaged_time(&self) -> i64 {
        self.last_damaged_time
    }

    pub fn set_last_damaged_time_like_cpp(&mut self, last_damaged_time: i64) {
        self.last_damaged_time = last_damaged_time;
    }

    pub const fn regenerate_health(&self) -> bool {
        self.regenerate_health
    }

    pub const fn is_missing_can_swim_flag_out_of_combat(&self) -> bool {
        self.is_missing_can_swim_flag_out_of_combat
    }

    pub const fn gossip_menu_id(&self) -> u32 {
        self.gossip_menu_id
    }

    pub const fn sparring_health_pct(&self) -> u8 {
        self.sparring_health_pct
    }

    pub const fn regen_timer(&self) -> u32 {
        self.regen_timer
    }

    pub const fn spells(&self) -> [u32; MAX_CREATURE_SPELLS] {
        self.spells
    }

    pub fn set_spell(&mut self, slot: usize, spell_id: u32) {
        if slot < MAX_CREATURE_SPELLS {
            self.spells[slot] = spell_id;
        }
    }

    pub const fn disable_reputation_gain(&self) -> bool {
        self.disable_reputation_gain
    }

    pub const fn sight_distance(&self) -> f32 {
        self.sight_distance
    }

    pub const fn combat_distance(&self) -> f32 {
        self.combat_distance
    }

    pub fn set_combat_distance_like_cpp(&mut self, combat_distance: f32) {
        self.combat_distance = combat_distance.max(0.0);
    }

    pub const fn loot_mode(&self) -> u16 {
        self.loot_mode
    }

    pub fn reset_loot_mode(&mut self) {
        self.loot_mode = LOOT_MODE_DEFAULT;
    }

    pub const fn is_temp_world_object(&self) -> bool {
        self.is_temp_world_object
    }

    /// C++ `Creature::m_isTempWorldObject` (`Creature.h:365`), toggled by
    /// `Map::SwitchGridContainers<Creature>` after moving between grid/world
    /// containers (`Map.cpp:294-305`).
    pub fn set_temp_world_object_like_cpp(&mut self, on: bool) {
        self.is_temp_world_object = on;
    }

    pub const fn cleanup_before_delete_count(&self) -> u32 {
        self.grid_unload_cleanup_before_delete_count
    }

    pub const fn grid_unload_delete_requested(&self) -> bool {
        self.grid_unload_delete_requested
    }

    pub const fn grid_unload_respawn_relocation_requested(&self) -> bool {
        self.grid_unload_respawn_relocation_requested
    }

    pub fn register_dynamic_object(&mut self, guid: ObjectGuid) {
        self.owned_dynamic_objects.push(guid);
    }

    pub fn unregister_dynamic_object(&mut self, guid: ObjectGuid) {
        self.owned_dynamic_objects
            .retain(|owned_guid| *owned_guid != guid);
    }

    pub fn dynamic_objects(&self) -> &[ObjectGuid] {
        &self.owned_dynamic_objects
    }

    pub fn removed_dynamic_objects_from_grid_unload(&self) -> &[ObjectGuid] {
        &self.removed_dynamic_objects_from_grid_unload
    }

    pub fn register_area_trigger(&mut self, guid: ObjectGuid) {
        self.owned_area_triggers.push(guid);
    }

    pub fn unregister_area_trigger(&mut self, guid: ObjectGuid) {
        self.owned_area_triggers
            .retain(|owned_guid| *owned_guid != guid);
    }

    pub fn area_triggers(&self) -> &[ObjectGuid] {
        &self.owned_area_triggers
    }

    pub fn removed_area_triggers_from_grid_unload(&self) -> &[ObjectGuid] {
        &self.removed_area_triggers_from_grid_unload
    }

    pub fn set_destroyed_object(&mut self, destroyed: bool) {
        self.unit
            .world_mut()
            .object_mut()
            .set_destroyed_object(destroyed);
    }

    pub fn remove_all_dyn_objects(&mut self) {
        self.removed_dynamic_objects_from_grid_unload
            .extend(self.owned_dynamic_objects.drain(..));
    }

    pub fn remove_all_area_triggers(&mut self) {
        self.removed_area_triggers_from_grid_unload
            .extend(self.owned_area_triggers.drain(..));
    }

    pub fn combat_stop(&mut self) {
        self.unit.set_attacking(None);
    }

    pub const fn is_in_combat(&self) -> bool {
        self.unit.attacking().is_some()
    }

    pub fn request_respawn_relocation_from_grid_unload(&mut self) {
        self.grid_unload_respawn_relocation_requested = true;
    }

    pub fn cleanup_before_delete(&mut self) {
        self.grid_unload_cleanup_before_delete_count = self
            .grid_unload_cleanup_before_delete_count
            .saturating_add(1);
    }

    pub fn request_delete_from_grid_unload(&mut self) {
        self.grid_unload_delete_requested = true;
        self.unit.world_mut().clear_current_cell();
    }

    pub fn get_power_index(&self, power: PowerType) -> Option<usize> {
        if power == self.power_type() {
            Some(0)
        } else if power == PowerType::ComboPoints {
            Some(2)
        } else {
            None
        }
    }

    pub fn power_type(&self) -> PowerType {
        power_type_from_u8(self.unit.data().display_power)
    }

    pub fn set_power_type(&mut self, power: PowerType) {
        let old_power = self.power_type();
        if old_power != PowerType::ComboPoints {
            self.unit.set_power_index(old_power, None);
        }
        self.unit.set_display_power(power);
        self.unit.set_power_index(power, Some(0));
        self.unit.set_power_index(PowerType::ComboPoints, Some(2));
    }

    pub fn set_display_id(
        &mut self,
        display_id: u32,
        set_native: bool,
        model: Option<CreatureModelDimensions>,
    ) {
        self.ai_ownership.display_id = display_id;
        self.unit.set_display_id(display_id, set_native);

        if let Some(model) = model {
            let scale = self.unit.world().object().scale() * self.unit.data().display_scale;
            self.unit.set_bounding_radius(model.bounding_radius * scale);
            self.unit.set_combat_reach(model.combat_reach * scale);
        }
    }

    pub fn set_faction(&mut self, faction: u32) {
        self.ai_ownership.faction = faction;
        self.unit.set_faction(faction);
    }

    pub const fn attack_reputation_faction_id_like_cpp(&self) -> Option<u32> {
        self.attack_reputation_faction_id
    }

    pub fn set_attack_reputation_faction_id_like_cpp(&mut self, faction_id: Option<u32>) {
        self.attack_reputation_faction_id = faction_id;
    }

    pub const fn is_contested_guard_like_cpp(&self) -> bool {
        self.is_contested_guard_faction
    }

    pub fn set_contested_guard_like_cpp(&mut self, contested_guard: bool) {
        self.is_contested_guard_faction = contested_guard;
    }

    pub const fn runtime_state(&self) -> &CreatureRuntimeState {
        &self.runtime_state
    }

    pub fn runtime_state_mut(&mut self) -> &mut CreatureRuntimeState {
        &mut self.runtime_state
    }

    pub fn tap_list(&self) -> &[ObjectGuid] {
        &self.tap_list
    }

    pub fn has_loot_recipient(&self) -> bool {
        self.runtime_state.has_loot_recipient
    }

    pub fn is_reputation_gain_disabled(&self) -> bool {
        self.disable_reputation_gain
    }

    pub fn set_disable_reputation_gain(&mut self, disabled: bool) {
        self.disable_reputation_gain = disabled;
    }

    pub fn set_pickpocket_loot_restore(&mut self, restore_time: i64) {
        self.pickpocket_loot_restore = restore_time;
    }

    pub const fn pickpocket_loot_restore(&self) -> i64 {
        self.pickpocket_loot_restore
    }

    pub fn reset_pickpocket_loot_restore(&mut self) {
        self.pickpocket_loot_restore = 0;
        self.runtime_state.pickpocket_reset_count =
            self.runtime_state.pickpocket_reset_count.saturating_add(1);
    }

    pub fn set_dont_clear_tap_list_on_evade(&mut self, dont_clear: bool) {
        if self.spawn_id == 0 {
            self.dont_clear_tap_list_on_evade = dont_clear;
        }
    }

    pub const fn dont_clear_tap_list_on_evade(&self) -> bool {
        self.dont_clear_tap_list_on_evade
    }

    pub fn set_tapped_by_player(&mut self, player_guid: ObjectGuid, group_guids: &[ObjectGuid]) {
        if self.tap_list.len() >= CREATURE_TAPPERS_SOFT_CAP || player_guid == ObjectGuid::EMPTY {
            return;
        }
        self.insert_tapper(player_guid);
        for guid in group_guids {
            if self.tap_list.len() >= CREATURE_TAPPERS_SOFT_CAP {
                break;
            }
            if *guid != ObjectGuid::EMPTY {
                self.insert_tapper(*guid);
            }
        }
        self.runtime_state.has_loot_recipient = !self.tap_list.is_empty();
    }

    pub fn is_tapped_by(&self, player_guid: ObjectGuid) -> bool {
        self.tap_list.contains(&player_guid)
    }

    pub fn clear_tap_list(&mut self) {
        self.tap_list.clear();
        self.runtime_state.has_loot_recipient = false;
    }

    pub fn clear_tap_list_for_evade(&mut self) {
        if !self.dont_clear_tap_list_on_evade {
            self.clear_tap_list();
        }
    }

    fn insert_tapper(&mut self, guid: ObjectGuid) {
        if !self.tap_list.contains(&guid) && self.tap_list.len() < CREATURE_TAPPERS_SOFT_CAP {
            self.tap_list.push(guid);
        }
    }

    pub fn apply_death_transition(&mut self, state: DeathState, now: i64) -> CreatureRuntimePlan {
        self.set_death_state_runtime(state, now)
    }

    pub fn set_death_state_runtime(&mut self, state: DeathState, now: i64) -> CreatureRuntimePlan {
        let mut plan = CreatureRuntimePlan::new();
        self.unit.set_death_state(state);

        match state {
            DeathState::JustDied => {
                self.corpse_remove_time = now.saturating_add(self.corpse_delay as i64);
                let respawn_delay = self.respawn_delay as i64;
                self.respawn_time = if self.respawn_compatibility_mode {
                    now.saturating_add(respawn_delay)
                        .saturating_add(self.corpse_delay as i64)
                } else {
                    now.saturating_add(respawn_delay)
                };
                self.runtime_state.save_respawn_requested = true;
                self.runtime_state.visibility_update_requested = true;
                self.unit.set_target(ObjectGuid::EMPTY);
                self.unit.set_attacking(None);
                self.already_searched_assistance = false;
                self.already_call_assistance = false;
                plan.extend([
                    CreatureRuntimeAction::SaveRespawnTime,
                    CreatureRuntimeAction::ClearTarget,
                    CreatureRuntimeAction::ClearNpcFlags,
                    CreatureRuntimeAction::ClearMount,
                    CreatureRuntimeAction::Deactivate,
                    CreatureRuntimeAction::ClearAssistanceSearch,
                ]);
                self.unit.set_death_state(DeathState::Corpse);
            }
            DeathState::JustRespawned => {
                self.unit.set_health(self.unit.data().max_health);
                self.clear_tap_list();
                self.player_damage_req = 0;
                self.cannot_reach_target = false;
                self.cannot_reach_timer = 0;
                self.respawn_time = 0;
                self.corpse_remove_time = 0;
                self.reset_pickpocket_loot_restore();
                self.reset_loot_mode();
                self.trigger_just_appeared = true;
                self.runtime_state.ai_reset_requested = true;
                self.runtime_state.visibility_update_requested = true;
                plan.extend([
                    CreatureRuntimeAction::ClearTapList,
                    CreatureRuntimeAction::ResetPlayerDamageReq,
                    CreatureRuntimeAction::ResetCannotReachTarget,
                    CreatureRuntimeAction::UpdateMovementFlags,
                    CreatureRuntimeAction::ClearErasableUnitState,
                    CreatureRuntimeAction::InitializeMotion,
                    CreatureRuntimeAction::ResetAi,
                    CreatureRuntimeAction::LoadAddonAndSparring,
                ]);
                self.unit.set_death_state(DeathState::Alive);
            }
            _ => {}
        }

        plan
    }

    pub fn remove_corpse_runtime(
        &mut self,
        now: i64,
        set_spawn_time: bool,
        destroy_for_nearby_players: bool,
    ) -> CreatureRuntimePlan {
        let mut plan = CreatureRuntimePlan::new();
        if self.unit.death_state() != DeathState::Corpse {
            return plan;
        }

        self.runtime_state.remove_corpse_requested = true;
        self.runtime_state.corpse_removed_count =
            self.runtime_state.corpse_removed_count.saturating_add(1);
        self.runtime_state.loot_removed_count =
            self.runtime_state.loot_removed_count.saturating_add(1);
        self.corpse_remove_time = now;
        plan.extend([
            CreatureRuntimeAction::RemoveAllAuras,
            CreatureRuntimeAction::RemoveLoot,
            CreatureRuntimeAction::CorpseRemovedAiHook,
        ]);

        if destroy_for_nearby_players {
            self.runtime_state.visibility_destroy_requested = true;
            plan.push(CreatureRuntimeAction::DestroyVisibility);
        }

        if set_spawn_time {
            self.respawn_time = self
                .respawn_time
                .max(now.saturating_add(self.respawn_delay as i64));
            self.runtime_state.save_respawn_requested = !self.respawn_compatibility_mode;
            if !self.respawn_compatibility_mode {
                plan.push(CreatureRuntimeAction::SaveRespawnTime);
            }
        }

        if self.respawn_compatibility_mode {
            self.unit.set_death_state(DeathState::Dead);
            plan.push(CreatureRuntimeAction::RelocateToRespawnPosition);
        } else {
            self.runtime_state.object_remove_requested = true;
            plan.push(CreatureRuntimeAction::RequestObjectRemove);
        }

        plan
    }

    pub fn respawn_runtime(&mut self, force: bool, now: i64) -> CreatureRuntimePlan {
        let mut plan = CreatureRuntimePlan::new();
        if force {
            if self.unit.is_alive() {
                plan.extend(
                    self.set_death_state_runtime(DeathState::JustDied, now)
                        .actions()
                        .iter()
                        .copied(),
                );
            } else if self.unit.death_state() != DeathState::Corpse {
                self.unit.set_death_state(DeathState::Corpse);
            }
        }

        if self.respawn_compatibility_mode {
            self.runtime_state.visibility_destroy_requested = true;
            plan.push(CreatureRuntimeAction::DestroyVisibility);
            plan.extend(
                self.remove_corpse_runtime(now, false, false)
                    .actions()
                    .iter()
                    .copied(),
            );
            if self.unit.death_state() == DeathState::Dead {
                self.respawn_time = 0;
                self.reset_pickpocket_loot_restore();
                self.runtime_state.loot_removed_count =
                    self.runtime_state.loot_removed_count.saturating_add(1);
                plan.extend([
                    CreatureRuntimeAction::ResetPickpocketLoot,
                    CreatureRuntimeAction::RemoveLoot,
                    CreatureRuntimeAction::RestoreOriginalEntry,
                    CreatureRuntimeAction::SelectLevel,
                ]);
                plan.extend(
                    self.set_death_state_runtime(DeathState::JustRespawned, now)
                        .actions()
                        .iter()
                        .copied(),
                );
                plan.extend([
                    CreatureRuntimeAction::ResetDisplay,
                    CreatureRuntimeAction::ResetReactState,
                    CreatureRuntimeAction::UpdatePool,
                ]);
            }
            self.runtime_state.visibility_update_requested = true;
            plan.push(CreatureRuntimeAction::UpdateVisibility);
        } else if self.spawn_id != 0 {
            self.runtime_state.map_respawn_requested = true;
            self.runtime_state.respawn_requested = true;
            plan.push(CreatureRuntimeAction::RequestMapRespawn);
        }

        plan
    }

    pub fn forced_despawn_runtime(
        &mut self,
        time_ms_to_despawn: u32,
        force_respawn_timer_secs: u32,
        now: i64,
    ) -> CreatureRuntimePlan {
        let mut plan = CreatureRuntimePlan::new();
        if time_ms_to_despawn > 0 {
            self.runtime_state.forced_despawn_pending = true;
            plan.push(CreatureRuntimeAction::RequestDelayedForcedDespawn);
            return plan;
        }

        if self.respawn_compatibility_mode {
            let corpse_delay = self.corpse_delay;
            let respawn_delay = self.respawn_delay;
            let mut override_respawn_time = false;
            self.runtime_state.visibility_destroy_requested = true;
            plan.push(CreatureRuntimeAction::DestroyVisibility);

            if self.unit.is_alive() {
                if force_respawn_timer_secs > 0 {
                    self.corpse_delay = 0;
                    self.respawn_delay = force_respawn_timer_secs;
                    override_respawn_time = true;
                }
                plan.extend(
                    self.set_death_state_runtime(DeathState::JustDied, now)
                        .actions()
                        .iter()
                        .copied(),
                );
            }

            plan.extend(
                self.remove_corpse_runtime(now, !override_respawn_time, false)
                    .actions()
                    .iter()
                    .copied(),
            );
            self.corpse_delay = corpse_delay;
            self.respawn_delay = respawn_delay;
        } else {
            if force_respawn_timer_secs > 0 {
                self.respawn_time = now.saturating_add(force_respawn_timer_secs as i64);
            } else {
                self.respawn_time = now.saturating_add(self.respawn_delay as i64);
            }
            self.runtime_state.save_respawn_requested = true;
            self.runtime_state.object_remove_requested = true;
            plan.extend([
                CreatureRuntimeAction::SaveRespawnTime,
                CreatureRuntimeAction::RequestObjectRemove,
            ]);
        }

        plan
    }

    pub fn all_loot_removed_from_corpse(
        &mut self,
        now: i64,
        decay_rate: f32,
        is_fully_skinned: bool,
    ) -> CreatureRuntimePlan {
        let mut plan = CreatureRuntimePlan::new();
        if self.corpse_remove_time <= now {
            return plan;
        }

        let effective_decay_rate = if self.ignore_corpse_decay_ratio {
            1.0
        } else {
            decay_rate.max(0.0)
        };
        self.corpse_remove_time = if is_fully_skinned {
            now
        } else {
            now.saturating_add((self.corpse_delay as f32 * effective_decay_rate) as i64)
        };
        self.respawn_time = self.respawn_time.max(
            self.corpse_remove_time
                .saturating_add(self.respawn_delay as i64),
        );
        self.runtime_state.remove_corpse_requested = is_fully_skinned;
        plan.push(CreatureRuntimeAction::UpdateLoot);
        plan
    }

    pub fn runtime_update_plan(
        &mut self,
        diff_ms: u32,
        now: i64,
        context: CreatureRuntimeUpdateContext,
    ) -> CreatureRuntimePlan {
        let mut plan = CreatureRuntimePlan::new();

        if context.ai_enabled
            && self.trigger_just_appeared
            && self.unit.death_state() != DeathState::Dead
        {
            self.trigger_just_appeared = false;
            self.runtime_state.appeared_notified = true;
            plan.push(CreatureRuntimeAction::NotifyJustAppeared);
        }

        match self.unit.death_state() {
            DeathState::Dead => {
                if self.respawn_compatibility_mode && self.respawn_time <= now {
                    self.runtime_state.respawn_requested = true;
                    plan.extend(self.respawn_runtime(false, now).actions().iter().copied());
                }
            }
            DeathState::Corpse => {
                if context.has_loot || context.has_personal_loot {
                    self.runtime_state.loot_updated_count =
                        self.runtime_state.loot_updated_count.saturating_add(1);
                    plan.push(CreatureRuntimeAction::UpdateLoot);
                }
                if self.corpse_remove_time <= now {
                    plan.extend(
                        self.remove_corpse_runtime(now, false, true)
                            .actions()
                            .iter()
                            .copied(),
                    );
                }
            }
            DeathState::Alive => {
                if context.ai_enabled && !context.in_evade_mode && context.is_engaged {
                    if consume_timer(&mut self.boundary_check_time, diff_ms) {
                        plan.push(CreatureRuntimeAction::BoundaryCheck);
                        self.boundary_check_time = DEFAULT_BOUNDARY_CHECK_TIME_MS;
                    }
                }

                if self.combat_pulse_delay > 0 && context.is_engaged && context.is_dungeon {
                    if consume_timer(&mut self.combat_pulse_time, diff_ms) {
                        if context.has_map_players {
                            plan.push(CreatureRuntimeAction::CombatPulse);
                        }
                        self.combat_pulse_time = self.combat_pulse_delay.saturating_mul(1_000);
                    }
                }

                plan.push(CreatureRuntimeAction::AiUpdateTick);
                plan.push(CreatureRuntimeAction::MeleeAttackIfReady);

                if consume_timer(&mut self.regen_timer, diff_ms) {
                    let can_regen_health = !context.in_evade_mode
                        && (!context.is_engaged
                            || context.is_polymorphed
                            || (context.cannot_reach_target
                                && (context.allow_cannot_reach_regen || !context.is_raid)));
                    if self.regenerate_health && can_regen_health {
                        plan.push(CreatureRuntimeAction::RegenerateHealth);
                    }
                    plan.push(CreatureRuntimeAction::RegeneratePower);
                    self.regen_timer = CREATURE_REGEN_INTERVAL_MS;
                }

                if context.cannot_reach_target && !context.in_evade_mode && !context.is_raid {
                    self.cannot_reach_target = true;
                    self.cannot_reach_timer = self.cannot_reach_timer.saturating_add(diff_ms);
                    if self.cannot_reach_timer >= CREATURE_NOPATH_EVADE_TIME_MS {
                        self.runtime_state.evade_requested =
                            Some(CreatureRuntimeEvadeReason::NoPath);
                        plan.push(CreatureRuntimeAction::Evade(
                            CreatureRuntimeEvadeReason::NoPath,
                        ));
                    }
                } else {
                    self.cannot_reach_timer = 0;
                }
            }
            _ => {}
        }

        plan
    }
}

fn consume_timer(timer: &mut u32, diff_ms: u32) -> bool {
    if *timer == 0 {
        return true;
    }
    if diff_ms >= *timer {
        *timer = 0;
        true
    } else {
        *timer -= diff_ms;
        false
    }
}

fn power_type_from_u8(power: u8) -> PowerType {
    match power {
        1 => PowerType::Rage,
        2 => PowerType::Focus,
        3 => PowerType::Energy,
        4 => PowerType::Happiness,
        5 => PowerType::Runes,
        6 => PowerType::RunicPower,
        7 => PowerType::SoulShards,
        8 => PowerType::LunarPower,
        9 => PowerType::HolyPower,
        10 => PowerType::AlternatePower,
        11 => PowerType::Maelstrom,
        12 => PowerType::Chi,
        13 => PowerType::Insanity,
        14 => PowerType::ComboPoints,
        15 => PowerType::DemonicFury,
        16 => PowerType::ArcaneCharges,
        17 => PowerType::Fury,
        18 => PowerType::Pain,
        19 => PowerType::Essence,
        20 => PowerType::RuneBlood,
        21 => PowerType::RuneFrost,
        22 => PowerType::RuneUnholy,
        23 => PowerType::AlternateQuest,
        24 => PowerType::AlternateEncounter,
        25 => PowerType::AlternateMount,
        _ => PowerType::Mana,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn formation_info_like_cpp(leader_spawn_id: u64) -> CreatureFormationInfoLikeCpp {
        CreatureFormationInfoLikeCpp {
            leader_spawn_id,
            follow_dist: 7.0,
            follow_angle_radians: 1.25,
            group_ai: 3,
            leader_waypoint_ids: [11, 12],
        }
    }

    #[test]
    fn creature_search_formation_like_cpp_requests_only_with_spawn_and_info() {
        let mut creature = Creature::new(false);
        creature.set_spawn_id(1234);
        creature.set_formation_info_like_cpp(Some(formation_info_like_cpp(77)));

        let outcome = creature.search_formation_like_cpp();

        assert_eq!(outcome.spawn_id, 1234);
        assert!(!outcome.is_summon);
        assert!(outcome.formation_info_found);
        assert_eq!(outcome.leader_spawn_id, Some(77));
        assert!(outcome.add_to_group_requested);
    }

    #[test]
    fn creature_search_formation_like_cpp_skips_summon_and_zero_spawn() {
        let mut summon = Creature::new(false);
        summon.set_spawn_id(1234);
        summon.set_summon_like_cpp(true);
        summon.set_formation_info_like_cpp(Some(formation_info_like_cpp(77)));

        let summon_outcome = summon.search_formation_like_cpp();
        assert!(summon_outcome.is_summon);
        assert!(summon_outcome.formation_info_found);
        assert_eq!(summon_outcome.leader_spawn_id, None);
        assert!(!summon_outcome.add_to_group_requested);

        let mut zero_spawn = Creature::new(false);
        zero_spawn.set_formation_info_like_cpp(Some(formation_info_like_cpp(77)));

        let zero_spawn_outcome = zero_spawn.search_formation_like_cpp();
        assert_eq!(zero_spawn_outcome.spawn_id, 0);
        assert!(!zero_spawn_outcome.is_summon);
        assert!(zero_spawn_outcome.formation_info_found);
        assert_eq!(zero_spawn_outcome.leader_spawn_id, None);
        assert!(!zero_spawn_outcome.add_to_group_requested);
    }

    #[test]
    fn creature_search_formation_like_cpp_skips_missing_formation_info() {
        let mut creature = Creature::new(false);
        creature.set_spawn_id(1234);

        let outcome = creature.search_formation_like_cpp();

        assert_eq!(outcome.spawn_id, 1234);
        assert!(!outcome.is_summon);
        assert!(!outcome.formation_info_found);
        assert_eq!(outcome.leader_spawn_id, None);
        assert!(!outcome.add_to_group_requested);
    }

    #[test]
    fn creature_constructor_matches_cpp_base_state() {
        let creature = Creature::new(false);

        assert_eq!(creature.unit().world().object().type_id(), TypeId::Unit);
        assert_eq!(
            creature.unit().world().object().type_mask(),
            TypeMask::OBJECT | TypeMask::UNIT
        );
        assert!(!creature.unit().world().is_world_object());
        assert_eq!(creature.player_damage_req(), 0);
        assert_eq!(creature.corpse_remove_time(), 0);
        assert_eq!(creature.respawn_time(), 0);
        assert_eq!(creature.respawn_delay(), DEFAULT_RESPAWN_DELAY_SECS);
        assert_eq!(creature.corpse_delay(), DEFAULT_CORPSE_DELAY_SECS);
        assert!(!creature.ignore_corpse_decay_ratio());
        assert_eq!(creature.wander_distance(), 0.0);
        assert_eq!(
            creature.boundary_check_time(),
            DEFAULT_BOUNDARY_CHECK_TIME_MS
        );
        assert_eq!(creature.combat_pulse_time(), 0);
        assert_eq!(creature.combat_pulse_delay(), 0);
        assert_eq!(creature.react_state(), ReactState::Aggressive);
        assert_eq!(
            creature.default_movement_type(),
            MovementGeneratorType::Idle
        );
        assert_eq!(creature.spawn_id(), 0);
        assert_eq!(creature.equipment_id(), 0);
        assert_eq!(creature.original_equipment_id(), 0);
        assert!(!creature.already_call_assistance());
        assert!(!creature.already_searched_assistance());
        assert!(!creature.cannot_reach_target());
        assert_eq!(creature.cannot_reach_timer(), 0);
        assert_eq!(creature.melee_damage_school_mask(), 0x1);
        assert_eq!(creature.original_entry(), 0);
        assert!(creature.trigger_just_appeared());
        assert!(!creature.respawn_compatibility_mode());
        assert_eq!(creature.last_damaged_time(), 0);
        assert!(creature.regenerate_health());
        assert!(!creature.is_missing_can_swim_flag_out_of_combat());
        assert_eq!(creature.gossip_menu_id(), 0);
        assert_eq!(creature.sparring_health_pct(), 0);
        assert_eq!(creature.regen_timer(), CREATURE_REGEN_INTERVAL_MS);
        assert_eq!(creature.spells(), [0; MAX_CREATURE_SPELLS]);
        assert!(!creature.disable_reputation_gain());
        assert_eq!(creature.sight_distance(), DEFAULT_MONSTER_SIGHT_DISTANCE);
        assert_eq!(creature.combat_distance(), 0.0);
        assert_eq!(creature.loot_mode(), LOOT_MODE_DEFAULT);
        assert!(!creature.is_temp_world_object());
        assert_eq!(creature.cleanup_before_delete_count(), 0);
        assert!(!creature.grid_unload_delete_requested());
        assert!(!creature.grid_unload_respawn_relocation_requested());
        assert_eq!(creature.ai_ownership().loot_id, 0);
        assert_eq!(creature.ai_ownership().gold_min, 0);
        assert_eq!(creature.ai_ownership().gold_max, 0);
        assert_eq!(creature.ai_ownership().boss_id, None);
        assert_eq!(creature.ai_ownership().dungeon_encounter_id, 0);
        assert_eq!(creature.ai_ownership().terrain_swap_map, -1);
        assert_eq!(creature.ai_ownership().last_movement_inform, None);
    }

    #[test]
    fn creature_ai_ownership_derives_identity_health_and_position() {
        let mut creature = Creature::new(false);
        let guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            1,
            0,
            0,
            1,
            12345,
        );
        let position = Position::new(1.0, 2.0, 3.0, 4.0);

        creature.unit_mut().world_mut().object_mut().create(guid);
        creature.unit_mut().world_mut().object_mut().set_entry(987);
        creature.unit_mut().world_mut().relocate(position);
        creature.unit_mut().set_level(22);
        creature.unit_mut().set_max_health(40);
        creature.unit_mut().set_health(35);
        creature.set_ai_home_position(position);

        assert_eq!(creature.ai_guid(), guid);
        assert_eq!(creature.ai_entry(), 987);
        assert_eq!(creature.ai_level(), 22);
        assert_eq!(creature.ai_current_health(), 35);
        assert_eq!(creature.ai_max_health(), 40);
        assert_eq!(creature.ai_position(), position);
        assert_eq!(creature.ai_home_position(), position);
    }

    #[test]
    fn creature_ai_ownership_enter_and_reset_combat() {
        let mut creature = Creature::new(false);
        let home = Position::new(10.0, 20.0, 30.0, 1.0);
        let attacker = ObjectGuid::create_player(1, 7);
        creature.unit_mut().set_max_health(80);
        creature.unit_mut().set_health(35);
        creature.set_ai_home_position(home);

        creature.enter_ai_combat(attacker);
        assert!(!creature.take_ai_damage(1, 10));
        assert_eq!(creature.ai_state(), CreatureAiState::InCombat);
        assert_eq!(creature.ai_ownership().combat_target, Some(attacker));
        assert_eq!(creature.unit().attacking(), Some(attacker));
        assert!(creature.last_damaged_time() > 10);

        creature.reset_ai_combat(55);
        assert_eq!(creature.ai_state(), CreatureAiState::Returning);
        assert_eq!(creature.ai_ownership().combat_target, None);
        assert_eq!(creature.unit().attacking(), None);
        assert_eq!(creature.ai_current_health(), 80);
        assert_eq!(creature.ai_ownership().move_target, Some(home));
        assert_eq!(creature.ai_ownership().move_start_ms, 55);
        assert_eq!(creature.last_damaged_time(), 0);
    }

    #[test]
    fn creature_ai_ownership_damage_and_death_syncs_unit_state() {
        let mut creature = Creature::new(false);
        creature.unit_mut().set_max_health(40);
        creature.unit_mut().set_health(40);
        creature.ai_ownership_mut().respawn_time_secs = 30;

        assert_eq!(creature.current_health(), 40);
        assert_eq!(creature.ai_state(), CreatureAiState::Idle);
        assert!(!creature.take_ai_damage(15, 10));
        assert_eq!(creature.current_health(), 25);

        assert!(creature.take_ai_damage(100, 20));
        assert_eq!(creature.current_health(), 0);
        assert_eq!(creature.unit().death_state(), DeathState::Corpse);
        assert_eq!(creature.ai_state(), CreatureAiState::Dead);
        assert_eq!(creature.ai_ownership().death_time_ms, Some(20));
        assert_eq!(
            creature.corpse_remove_time(),
            20 + i64::from(DEFAULT_CORPSE_DELAY_SECS)
        );
        assert_eq!(creature.respawn_time(), 20 + 30);
        assert!(creature.runtime_state().save_respawn_requested);
        assert!(!creature.should_ai_respawn(29_999));
        assert!(creature.should_ai_respawn(30_020));
    }

    #[test]
    fn creature_ai_damage_records_aggro_reset_expiry_like_cpp() {
        let mut creature = Creature::new(false);
        creature.unit_mut().set_max_health(40);
        creature.unit_mut().set_health(40);

        assert!(!creature.apply_ai_damage_before_death_state_at_game_time_like_cpp(15, 10, 1_000));

        assert_eq!(
            creature.last_damaged_time(),
            1_000 + MAX_AGGRO_RESET_TIME_SECS_LIKE_CPP
        );
    }

    #[test]
    fn creature_ai_lethal_damage_does_not_record_aggro_reset_expiry_like_cpp() {
        let mut creature = Creature::new(false);
        creature.unit_mut().set_max_health(40);
        creature.unit_mut().set_health(40);

        assert!(creature.apply_ai_damage_before_death_state_at_game_time_like_cpp(100, 10, 1_000));

        assert_eq!(creature.last_damaged_time(), 0);
    }

    #[test]
    fn creature_ai_damage_does_not_record_player_owned_aggro_reset_like_cpp() {
        let owner = ObjectGuid::create_player(1, 42);
        let mut creature = Creature::new(false);
        creature.unit_mut().set_max_health(40);
        creature.unit_mut().set_health(40);
        creature
            .unit_mut()
            .subsystems_mut()
            .control
            .set_owner_guid(Some(owner));

        assert!(!creature.apply_ai_damage_before_death_state_at_game_time_like_cpp(15, 10, 1_000));

        assert_eq!(creature.last_damaged_time(), 0);
    }

    #[test]
    fn creature_world_boss_uses_type_flags_and_excludes_summons_like_cpp() {
        let mut creature = Creature::new(false);
        creature.set_type_flags_runtime_like_cpp(CreatureTypeFlags::BOSS_MOB.bits());
        assert!(creature.is_world_boss_like_cpp());

        creature.set_summon_like_cpp(true);

        assert!(!creature.is_world_boss_like_cpp());
    }

    #[test]
    fn creature_ai_lethal_damage_can_defer_death_state_until_kill_hooks_like_cpp() {
        let mut creature = Creature::new(false);
        creature.unit_mut().set_max_health(40);
        creature.unit_mut().set_health(40);
        creature.ai_ownership_mut().respawn_time_secs = 30;

        assert!(creature.apply_ai_damage_before_death_state_like_cpp(100, 20));
        assert_eq!(creature.current_health(), 0);
        assert_eq!(creature.ai_state(), CreatureAiState::Dead);
        assert_eq!(creature.ai_ownership().death_time_ms, Some(20));
        assert_eq!(creature.unit().death_state(), DeathState::Alive);
        assert_eq!(creature.corpse_remove_time(), 0);
        assert!(!creature.runtime_state().save_respawn_requested);

        creature.complete_ai_death_state_after_kill_hooks_like_cpp(20);
        assert_eq!(creature.unit().death_state(), DeathState::Corpse);
        assert_eq!(
            creature.corpse_remove_time(),
            20 + i64::from(DEFAULT_CORPSE_DELAY_SECS)
        );
        assert_eq!(creature.respawn_time(), 20 + 30);
        assert!(creature.runtime_state().save_respawn_requested);
    }

    #[test]
    fn creature_corpse_loot_flags_apply_after_death_state_like_cpp() {
        let mut creature = Creature::new(false);
        creature.unit_mut().set_max_health(40);
        creature.unit_mut().set_health(40);
        creature.apply_ai_damage_before_death_state_like_cpp(100, 20);
        creature.complete_ai_death_state_after_kill_hooks_like_cpp(20);

        creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, true);

        assert!(
            creature
                .unit()
                .world()
                .object()
                .has_dynamic_flag(UnitDynFlags::Lootable as u32)
        );
        assert!(
            creature
                .unit()
                .world()
                .object()
                .has_dynamic_flag(UnitDynFlags::CanSkin as u32)
        );
        assert!(
            creature
                .unit()
                .unit_flags_like_cpp()
                .contains(UnitFlags::SKINNABLE)
        );
    }

    #[test]
    fn creature_ai_ownership_respawn_aggro_and_corpse_timer() {
        let mut creature = Creature::new(false);
        let home = Position::new(10.0, 20.0, 30.0, 1.0);
        let attacker = ObjectGuid::create_player(1, 7);
        creature.unit_mut().set_max_health(80);
        creature.unit_mut().set_health(80);
        creature.set_ai_home_position(home);
        creature.set_ai_position(Position::new(11.0, 20.0, 30.0, 1.0));
        creature.ai_ownership_mut().aggro_radius = 5.0;

        assert!(!creature.try_ai_aggro(attacker, &Position::new(30.0, 20.0, 30.0, 0.0)));
        assert!(creature.try_ai_aggro(attacker, &Position::new(12.0, 20.0, 30.0, 0.0)));
        assert_eq!(creature.ai_state(), CreatureAiState::InCombat);

        creature.mark_ai_dead(100);
        creature.set_ai_corpse_despawn_at(Some(130));
        creature.set_last_damaged_time_like_cpp(1_010);
        assert_eq!(creature.ai_ownership().corpse_despawn_at_ms, Some(130));
        creature.respawn_ai(200);
        assert!(creature.is_alive());
        assert_eq!(creature.current_health(), 80);
        assert_eq!(creature.position(), home);
        assert_eq!(creature.ai_state(), CreatureAiState::Idle);
        assert_eq!(creature.ai_ownership().combat_target, None);
        assert_eq!(creature.ai_ownership().corpse_despawn_at_ms, None);
        assert_eq!(creature.last_damaged_time(), 0);
    }

    #[test]
    fn creature_try_ai_aggro_requires_aggressive_react_state_like_cpp() {
        let mut creature = Creature::new(false);
        let player = ObjectGuid::create_player(1, 7);
        let creature_pos = Position::new(10.0, 20.0, 30.0, 0.0);
        let player_pos = Position::new(11.0, 20.0, 30.0, 0.0);
        creature.unit_mut().set_max_health(80);
        creature.unit_mut().set_health(80);
        creature.set_ai_position(creature_pos);
        creature.ai_ownership_mut().aggro_radius = 5.0;

        // C++ `CreatureAI::MoveInLineOfSight` gates normal proximity aggro on
        // `HasReactState(REACT_AGGRESSIVE)` before `CanStartAttack`.
        creature.set_react_state(ReactState::Passive);
        assert!(!creature.try_ai_aggro(player, &player_pos));
        assert_eq!(creature.ai_state(), CreatureAiState::Idle);

        creature.set_react_state(ReactState::Defensive);
        assert!(!creature.try_ai_aggro(player, &player_pos));
        assert_eq!(creature.ai_state(), CreatureAiState::Idle);

        creature.set_react_state(ReactState::Aggressive);
        assert!(creature.try_ai_aggro(player, &player_pos));
        assert_eq!(creature.ai_state(), CreatureAiState::InCombat);
    }

    #[test]
    fn creature_try_ai_aggro_rejects_non_positive_radius_like_cpp() {
        let mut creature = Creature::new(false);
        let player = ObjectGuid::create_player(1, 7);
        let creature_pos = Position::new(10.0, 20.0, 30.0, 0.0);
        creature.unit_mut().set_max_health(80);
        creature.unit_mut().set_health(80);
        creature.set_ai_position(creature_pos);

        // Rust uses aggro_radius=0 for non-aggro neutral spawns (for example
        // faction 35). The legacy session path already rejected that before
        // calling into creature AI; the global map-owned path must keep the
        // same C++ CanStartAttack-style no-aggro gate.
        creature.ai_ownership_mut().aggro_radius = 0.0;
        assert!(!creature.try_ai_aggro(player, &creature_pos));
        assert_eq!(creature.ai_state(), CreatureAiState::Idle);

        creature.ai_ownership_mut().aggro_radius = -1.0;
        assert!(!creature.try_ai_aggro(player, &creature_pos));
        assert_eq!(creature.ai_state(), CreatureAiState::Idle);

        creature.ai_ownership_mut().aggro_radius = 1.0;
        assert!(creature.try_ai_aggro(player, &creature_pos));
        assert_eq!(creature.ai_state(), CreatureAiState::InCombat);
    }

    #[test]
    fn creature_try_ai_aggro_rejects_immune_to_pc_like_cpp() {
        let mut creature = Creature::new(false);
        let player = ObjectGuid::create_player(1, 7);
        let creature_pos = Position::new(10.0, 20.0, 30.0, 0.0);
        let player_pos = Position::new(11.0, 20.0, 30.0, 0.0);
        creature.unit_mut().set_max_health(80);
        creature.unit_mut().set_health(80);
        creature.set_ai_position(creature_pos);
        creature.ai_ownership_mut().aggro_radius = 5.0;

        // C++ `Creature::CanStartAttack` rejects `IsImmuneToPC()` when the
        // target has `UNIT_FLAG_PLAYER_CONTROLLED`; this helper only scans
        // player candidates, so the target side is implied here.
        creature
            .unit_mut()
            .set_unit_flags_like_cpp(UnitFlags::IMMUNE_TO_PC);
        assert!(!creature.try_ai_aggro(player, &player_pos));
        assert_eq!(creature.ai_state(), CreatureAiState::Idle);

        creature
            .unit_mut()
            .set_unit_flags_like_cpp(UnitFlags::IMMUNE_TO_NPC);
        assert!(creature.try_ai_aggro(player, &player_pos));
        assert_eq!(creature.ai_state(), CreatureAiState::InCombat);
    }

    #[test]
    fn creature_try_ai_aggro_rejects_excessive_z_distance_like_cpp() {
        let player = ObjectGuid::create_player(1, 7);
        let creature_pos = Position::new(10.0, 20.0, 30.0, 0.0);

        let mut rejected = Creature::new(false);
        rejected.unit_mut().set_max_health(80);
        rejected.unit_mut().set_health(80);
        rejected.unit_mut().set_combat_reach(1.0);
        rejected.set_ai_position(creature_pos);
        rejected.ai_ownership_mut().aggro_radius = 10.0;
        assert!(!rejected.try_ai_aggro_with_target_combat_reach_like_cpp(
            player,
            &Position::new(12.0, 20.0, 34.6, 0.0),
            0.5,
        ));
        assert_eq!(rejected.ai_state(), CreatureAiState::Idle);

        let mut accepted = Creature::new(false);
        accepted.unit_mut().set_max_health(80);
        accepted.unit_mut().set_health(80);
        accepted.unit_mut().set_combat_reach(1.0);
        accepted.set_ai_position(creature_pos);
        accepted.ai_ownership_mut().aggro_radius = 10.0;
        assert!(accepted.try_ai_aggro_with_target_combat_reach_like_cpp(
            player,
            &Position::new(12.0, 20.0, 34.5, 0.0),
            0.5,
        ));
        assert_eq!(accepted.ai_state(), CreatureAiState::InCombat);

        let mut combat_distance = Creature::new(false);
        combat_distance.unit_mut().set_max_health(80);
        combat_distance.unit_mut().set_health(80);
        combat_distance.unit_mut().set_combat_reach(1.0);
        combat_distance.set_combat_distance_like_cpp(1.0);
        combat_distance.set_ai_position(creature_pos);
        combat_distance.ai_ownership_mut().aggro_radius = 10.0;
        assert!(
            combat_distance.try_ai_aggro_with_target_combat_reach_like_cpp(
                player,
                &Position::new(12.0, 20.0, 35.5, 0.0),
                0.5,
            )
        );
        assert_eq!(combat_distance.ai_state(), CreatureAiState::InCombat);

        let mut flying = Creature::new(false);
        flying.unit_mut().set_max_health(80);
        flying.unit_mut().set_health(80);
        flying.unit_mut().set_combat_reach(1.0);
        flying.set_ai_position(creature_pos);
        flying.ai_ownership_mut().aggro_radius = 100.0;
        flying.set_flight_movement_type_runtime_like_cpp(
            CreatureFlightMovementType::DisableGravity as u8,
        );
        assert!(
            flying.try_ai_aggro_with_target_combat_reach_like_cpp(
                player,
                &Position::new(12.0, 20.0, 60.0, 0.0),
                0.5,
            ),
            "C++ Creature::CanFly bypasses the z-distance gate for Flight != None"
        );
        assert_eq!(flying.ai_state(), CreatureAiState::InCombat);

        let mut dynamic_disable_gravity = Creature::new(false);
        dynamic_disable_gravity.unit_mut().set_max_health(80);
        dynamic_disable_gravity.unit_mut().set_health(80);
        dynamic_disable_gravity.unit_mut().set_combat_reach(1.0);
        dynamic_disable_gravity.set_ai_position(creature_pos);
        dynamic_disable_gravity.ai_ownership_mut().aggro_radius = 100.0;
        dynamic_disable_gravity.set_movement_flags_runtime_like_cpp(MovementFlag::DISABLE_GRAVITY);
        assert!(dynamic_disable_gravity.is_flying_like_cpp());
        assert!(
            dynamic_disable_gravity.try_ai_aggro_with_target_combat_reach_like_cpp(
                player,
                &Position::new(12.0, 20.0, 60.0, 0.0),
                0.5,
            ),
            "C++ Unit::IsFlying makes Creature::CanFly true for DISABLE_GRAVITY"
        );

        let mut dynamic_flying = Creature::new(false);
        dynamic_flying.unit_mut().set_max_health(80);
        dynamic_flying.unit_mut().set_health(80);
        dynamic_flying.unit_mut().set_combat_reach(1.0);
        dynamic_flying.set_ai_position(creature_pos);
        dynamic_flying.ai_ownership_mut().aggro_radius = 100.0;
        dynamic_flying.set_movement_flags_runtime_like_cpp(MovementFlag::FLYING);
        assert!(dynamic_flying.is_flying_like_cpp());
        assert!(
            dynamic_flying.try_ai_aggro_with_target_combat_reach_like_cpp(
                player,
                &Position::new(12.0, 20.0, 60.0, 0.0),
                0.5,
            ),
            "C++ Unit::IsFlying makes Creature::CanFly true for FLYING"
        );

        let mut dynamic_can_fly_only = Creature::new(false);
        dynamic_can_fly_only.unit_mut().set_max_health(80);
        dynamic_can_fly_only.unit_mut().set_health(80);
        dynamic_can_fly_only.unit_mut().set_combat_reach(1.0);
        dynamic_can_fly_only.set_ai_position(creature_pos);
        dynamic_can_fly_only.ai_ownership_mut().aggro_radius = 100.0;
        dynamic_can_fly_only.set_movement_flags_runtime_like_cpp(MovementFlag::CAN_FLY);
        assert!(
            !dynamic_can_fly_only.is_flying_like_cpp(),
            "C++ Unit::IsFlying ignores MOVEMENTFLAG_CAN_FLY by itself"
        );
        assert!(
            !dynamic_can_fly_only.try_ai_aggro_with_target_combat_reach_like_cpp(
                player,
                &Position::new(12.0, 20.0, 60.0, 0.0),
                0.5,
            )
        );

        let mut invalid_flight = Creature::new(false);
        invalid_flight.unit_mut().set_max_health(80);
        invalid_flight.unit_mut().set_health(80);
        invalid_flight.unit_mut().set_combat_reach(1.0);
        invalid_flight.set_ai_position(creature_pos);
        invalid_flight.ai_ownership_mut().aggro_radius = 100.0;
        invalid_flight
            .set_flight_movement_type_runtime_like_cpp(CREATURE_FLIGHT_MOVEMENT_TYPE_MAX_LIKE_CPP);
        assert_eq!(
            invalid_flight.flight_movement_type_like_cpp(),
            CreatureFlightMovementType::None as u8
        );
        assert!(
            !invalid_flight.try_ai_aggro_with_target_combat_reach_like_cpp(
                player,
                &Position::new(12.0, 20.0, 60.0, 0.0),
                0.5,
            )
        );
    }

    #[test]
    fn creature_accessibility_capabilities_follow_movement_template_like_cpp() {
        let mut creature = Creature::new(false);

        assert!(creature.can_walk_like_cpp());
        assert!(creature.can_enter_water_like_cpp());
        assert!(!creature.can_fly_like_cpp());

        creature.set_ground_movement_type_runtime_like_cpp(CreatureGroundMovementType::None as u8);
        creature.set_swim_allowed_runtime_like_cpp(true);
        assert!(!creature.can_walk_like_cpp());
        assert!(creature.can_enter_water_like_cpp());

        creature.set_swim_allowed_runtime_like_cpp(false);
        assert!(!creature.can_enter_water_like_cpp());

        creature.set_flight_movement_type_runtime_like_cpp(
            CreatureFlightMovementType::DisableGravity as u8,
        );
        assert!(creature.can_fly_like_cpp());
    }

    #[test]
    fn creature_can_enter_water_honors_unit_can_swim_like_cpp() {
        let mut creature = Creature::new(false);
        creature.set_swim_allowed_runtime_like_cpp(false);
        assert!(!creature.can_enter_water_like_cpp());

        creature
            .unit_mut()
            .set_unit_flags_like_cpp(UnitFlags::CAN_SWIM);
        assert!(creature.can_enter_water_like_cpp());

        creature
            .unit_mut()
            .set_unit_flags_like_cpp(UnitFlags::CAN_SWIM | UnitFlags::CANT_SWIM);
        assert!(!creature.can_enter_water_like_cpp());
    }

    #[test]
    fn creature_try_ai_aggro_rejects_civilian_like_cpp() {
        let mut creature = Creature::new(false);
        let player = ObjectGuid::create_player(1, 7);
        let creature_pos = Position::new(10.0, 20.0, 30.0, 0.0);
        let player_pos = Position::new(11.0, 20.0, 30.0, 0.0);
        creature.unit_mut().set_max_health(80);
        creature.unit_mut().set_health(80);
        creature.set_ai_position(creature_pos);
        creature.ai_ownership_mut().aggro_radius = 5.0;

        // C++ `Creature::CanStartAttack` returns false immediately for
        // `IsCivilian()`, which is backed by `CREATURE_FLAG_EXTRA_CIVILIAN`.
        creature.set_flags_extra_runtime_like_cpp(CreatureFlagsExtra::CIVILIAN.bits());
        assert!(creature.is_civilian_like_cpp());
        assert!(!creature.try_ai_aggro(player, &player_pos));
        assert_eq!(creature.ai_state(), CreatureAiState::Idle);

        creature.set_flags_extra_runtime_like_cpp(0);
        assert!(!creature.is_civilian_like_cpp());
        assert!(creature.try_ai_aggro(player, &player_pos));
        assert_eq!(creature.ai_state(), CreatureAiState::InCombat);
    }

    #[test]
    fn creature_ai_ownership_wander_and_packet_metadata_are_canonical() {
        let mut creature = Creature::new(false);
        assert!(creature.can_ai_wander());
        creature.ai_ownership_mut().npc_flags = 0x80;
        assert!(!creature.can_ai_wander());

        creature.set_display_id(1234, true, None);
        creature.set_faction(35);
        creature.ai_ownership_mut().unit_flags = 0x20;
        creature.ai_ownership_mut().min_damage = 5;
        creature.ai_ownership_mut().max_damage = 9;

        assert_eq!(creature.ai_ownership().display_id, 1234);
        assert_eq!(creature.ai_ownership().faction, 35);
        assert_eq!(creature.ai_ownership().unit_flags, 0x20);
        assert_eq!(creature.ai_ownership().min_damage, 5);
        assert_eq!(creature.ai_ownership().max_damage, 9);
    }

    #[test]
    fn creature_ai_movement_inform_records_cpp_type_and_id_payload() {
        let mut creature = Creature::new(false);

        creature.record_ai_movement_inform(15, 8);
        assert_eq!(
            creature.ai_ownership().last_movement_inform,
            Some(CreatureMovementInform {
                movement_type: 15,
                movement_id: 8,
            })
        );
        assert_eq!(
            creature.take_ai_movement_inform(),
            Some(CreatureMovementInform {
                movement_type: 15,
                movement_id: 8,
            })
        );
        assert_eq!(creature.ai_ownership().last_movement_inform, None);
    }

    #[test]
    fn creature_power_index_matches_cpp_stat_system() {
        let mut creature = Creature::new(false);

        assert_eq!(creature.get_power_index(PowerType::Mana), Some(0));
        assert_eq!(creature.get_power_index(PowerType::ComboPoints), Some(2));
        assert_eq!(creature.get_power_index(PowerType::Energy), None);

        creature.set_power_type(PowerType::Energy);
        assert_eq!(creature.power_type(), PowerType::Energy);
        assert_eq!(creature.get_power_index(PowerType::Energy), Some(0));
        assert_eq!(creature.get_power_index(PowerType::Mana), None);
        assert_eq!(creature.get_power_index(PowerType::ComboPoints), Some(2));
    }

    #[test]
    fn creature_respawn_and_corpse_setters_match_cpp_fields() {
        let mut creature = Creature::new(false);

        creature.set_respawn_delay(45);
        creature.set_respawn_time(1234);
        creature.set_corpse_delay(10, true);
        creature.set_respawn_compatibility_mode(true);
        creature.set_spawn_id(99);

        assert_eq!(creature.respawn_delay(), 45);
        assert_eq!(creature.respawn_time(), 1234);
        assert_eq!(creature.corpse_delay(), 10);
        assert!(creature.ignore_corpse_decay_ratio());
        assert!(creature.respawn_compatibility_mode());
        assert_eq!(creature.spawn_id(), 99);
    }

    #[test]
    fn creature_display_with_model_updates_unit_dimensions_like_cpp() {
        let mut creature = Creature::new(false);

        creature.unit_mut().world_mut().object_mut().set_scale(2.0);
        creature.set_display_id(
            1234,
            true,
            Some(CreatureModelDimensions {
                bounding_radius: 0.3,
                combat_reach: 1.5,
            }),
        );

        let scale = 2.0 * crate::DEFAULT_PLAYER_DISPLAY_SCALE;
        assert_eq!(creature.unit().data().display_id, 1234);
        assert_eq!(creature.unit().data().native_display_id, 1234);
        assert_eq!(creature.unit().data().bounding_radius, 0.3 * scale);
        assert_eq!(creature.unit().data().combat_reach, 1.5 * scale);
    }

    #[test]
    fn creature_react_state_and_faction_use_unit_fields() {
        let mut creature = Creature::new(false);

        creature.set_react_state(ReactState::Passive);
        creature.set_faction(35);

        assert!(creature.has_react_state(ReactState::Passive));
        assert_eq!(creature.unit().data().faction_template, 35);
    }

    #[test]
    fn creature_grid_unload_helpers_apply_represented_state() {
        let victim = wow_core::ObjectGuid::new(1, 2);
        let dynamic_object = wow_core::ObjectGuid::new(1, 3);
        let area_trigger = wow_core::ObjectGuid::new(1, 4);
        let mut creature = Creature::new(false);
        creature.unit_mut().set_attacking(Some(victim));
        creature.unit_mut().world_mut().set_current_cell(7, 8);
        creature.register_dynamic_object(dynamic_object);
        creature.register_area_trigger(area_trigger);

        creature.set_destroyed_object(true);
        creature.remove_all_dyn_objects();
        creature.remove_all_area_triggers();
        creature.combat_stop();
        creature.request_respawn_relocation_from_grid_unload();
        creature.cleanup_before_delete();
        creature.request_delete_from_grid_unload();

        assert!(creature.unit().world().object().is_destroyed_object());
        assert!(creature.dynamic_objects().is_empty());
        assert_eq!(
            creature.removed_dynamic_objects_from_grid_unload(),
            &[dynamic_object]
        );
        assert!(creature.area_triggers().is_empty());
        assert_eq!(
            creature.removed_area_triggers_from_grid_unload(),
            &[area_trigger]
        );
        assert_eq!(creature.unit().attacking(), None);
        assert!(creature.grid_unload_respawn_relocation_requested());
        assert_eq!(creature.cleanup_before_delete_count(), 1);
        assert!(creature.grid_unload_delete_requested());
        assert_eq!(creature.unit().world().current_cell(), None);
        assert!(!creature.unit().world().object().is_in_grid());
    }

    fn creature_lifecycle_template() -> CreatureTemplateLifecycleRecord {
        let mut spells = [0; MAX_CREATURE_SPELLS];
        spells[0] = 133;
        spells[3] = 116;
        CreatureTemplateLifecycleRecord {
            entry: 1001,
            original_entry: 9001,
            difficulty_id: 2,
            name: "lifecycle wolf".to_string(),
            ai_name: "SmartAI".to_string(),
            script_name: "npc_lifecycle_wolf".to_string(),
            unit_class: 1,
            faction: 14,
            display_id: 2001,
            model_dimensions: Some(CreatureModelDimensions {
                bounding_radius: 0.4,
                combat_reach: 1.2,
            }),
            scale: 1.5,
            speed_walk: 1.0,
            speed_run: 1.14286,
            spells,
            classification: 3,
            flags_extra: 0x10,
            creature_type: 9,
            type_flags: 0x20,
            movement_type: MovementGeneratorType::Idle,
            ground_movement_type: CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: CreatureFlightMovementType::DisableGravity as u8,
            min_level: 70,
            max_level: 72,
            equipment_id: 4,
            original_equipment_id: -4,
        }
    }

    fn creature_lifecycle_spawn() -> CreatureSpawnLifecycleRecord {
        CreatureSpawnLifecycleRecord {
            spawn_id: 44_000,
            map_id: 571,
            instance_id: 3,
            position: Position::new(1.0, 2.0, 3.0, 4.0),
            home_position: Position::new(5.0, 6.0, 7.0, 1.0),
            phase_id: Some(169),
            phase_group: Some(12),
            terrain_swap_map: Some(609),
            spawn_group_id: Some(77),
            spawn_group_name: Some("lifecycle group".to_string()),
            pool_id: Some(88),
            equipment_id: Some(9),
            original_equipment_id: Some(-9),
            wander_distance: 12.5,
            respawn_delay: 45,
            respawn_time: 123_456,
            movement_type: MovementGeneratorType::Idle,
            string_id: Some("creature-string".to_string()),
            is_active: false,
            inactive_by_spawn_group: true,
            duplicate_spawn_found: true,
            add_to_map: true,
            respawn_compatibility_mode: true,
        }
    }

    fn vehicle_seat_def(
        seat_index: i8,
        can_enter_or_exit: bool,
    ) -> (i8, VehicleSeatInfo, VehicleSeatAddon) {
        (
            seat_index,
            VehicleSeatInfo {
                id: 10_000 + u32::from(seat_index.unsigned_abs()),
                attachment_offset: Position::ZERO,
                can_enter_or_exit,
                usable_by_override: false,
                can_control: false,
                disables_gravity: false,
                passenger_not_selectable: false,
                keep_pet: false,
            },
            VehicleSeatAddon::default(),
        )
    }

    fn creature_lifecycle_create_record() -> CreatureCreateLifecycleRecord {
        CreatureCreateLifecycleRecord {
            guid: ObjectGuid::new(8, 1001),
            entry: 1001,
            map_id: 571,
            instance_id: 3,
            position: Position::new(1.0, 2.0, 3.0, 4.0),
            dynamic: false,
            vehicle_id: Some(101),
            vehicle_kit_create_input: Some(VehicleKitCreateInputLikeCpp {
                vehicle_id: 101,
                creature_entry: 1001,
                loading: true,
                seat_defs: vec![vehicle_seat_def(0, true), vehicle_seat_def(2, false)],
            }),
            add_to_world_vehicle_reset_context: None,
            template: creature_lifecycle_template(),
            spawn: None,
            selected_level: 71,
            stats: CreatureLifecycleStats::new(5_000, 4_500, 1_000, 750),
            selected_display_id: 3001,
            selected_model_dimensions: Some(CreatureModelDimensions {
                bounding_radius: 0.5,
                combat_reach: 2.0,
            }),
            selected_equipment_id: 6,
            selected_original_equipment_id: -6,
            corpse_delay: 90,
            ignore_corpse_decay_ratio: true,
        }
    }

    #[test]
    fn creature_lifecycle_create_applies_template_stats_and_clean_baseline() {
        let creature = Creature::create_from_lifecycle(creature_lifecycle_create_record());

        assert_eq!(
            creature.unit().world().object().guid(),
            ObjectGuid::new(8, 1001)
        );
        assert_eq!(creature.unit().world().object().entry(), 1001);
        assert_eq!(creature.unit().world().map_id(), 571);
        assert_eq!(creature.unit().world().instance_id(), 3);
        assert_eq!(
            creature.unit().world().position(),
            Position::new(1.0, 2.0, 3.0, 4.0)
        );
        assert_eq!(creature.unit().data().race, 0);
        assert_eq!(creature.unit().data().class_id, 1);
        assert_eq!(creature.unit().data().faction_template, 14);
        assert_eq!(creature.unit().data().display_id, 3001);
        assert_eq!(creature.unit().data().native_display_id, 3001);
        assert_eq!(creature.unit().world().object().scale(), 1.5);
        assert_eq!(
            creature.unit().data().bounding_radius,
            0.5 * 1.5 * crate::DEFAULT_PLAYER_DISPLAY_SCALE
        );
        assert_eq!(
            creature.unit().data().combat_reach,
            2.0 * 1.5 * crate::DEFAULT_PLAYER_DISPLAY_SCALE
        );
        assert_eq!(creature.spells()[0], 133);
        assert_eq!(creature.spells()[3], 116);
        assert_eq!(creature.equipment_id(), 6);
        assert_eq!(creature.original_equipment_id(), -6);
        let kit = creature.unit().subsystems().vehicle.kit.as_ref().unwrap();
        assert_eq!(kit.kit_id(), 101);
        assert!(kit.active());
        assert!(!kit.installed());
        assert_eq!(kit.seat_count(), 2);
        assert_eq!(kit.usable_seat_num(), 1);
        let create_outcome = creature
            .unit()
            .subsystems()
            .vehicle
            .last_create_outcome
            .as_ref()
            .unwrap();
        assert_eq!(create_outcome.kit_id, Some(101));
        assert!(create_outcome.created);
        assert_eq!(create_outcome.seat_count, 2);
        assert_eq!(create_outcome.usable_seat_num, 1);
        assert!(create_outcome.unit_update_flag_vehicle_represented);
        assert!(create_outcome.unit_type_mask_vehicle_represented);
        assert!(!create_outcome.send_set_vehicle_rec_id_represented);
        assert!(create_outcome.set_spellclick_or_player_vehicle_npc_flag_represented);
        assert!(!create_outcome.remove_spellclick_or_player_vehicle_npc_flag_represented);
        assert!(create_outcome.update_display_power_represented);
        assert!(create_outcome.init_movement_info_for_base_represented);
        assert_eq!(creature.lifecycle_metadata().vehicle_id, Some(101));
        assert_eq!(creature.unit().data().level, 71);
        assert_eq!(creature.unit().data().max_health, 5_000);
        assert_eq!(creature.unit().data().health, 4_500);
        assert_eq!(creature.unit().get_max_power(PowerType::Mana), 1_000);
        assert_eq!(creature.unit().get_power(PowerType::Mana), 750);
        assert_eq!(
            creature.unit().weapon_damage(WeaponAttackType::BaseAttack),
            [BASE_MINDAMAGE, BASE_MAXDAMAGE]
        );
        assert_eq!(creature.corpse_delay(), 90);
        assert!(creature.ignore_corpse_decay_ratio());
        assert!(creature.respawn_compatibility_mode());
        assert_eq!(creature.lifecycle_metadata().template_entry, 1001);
        assert_eq!(creature.lifecycle_metadata().original_entry, 9001);
        assert_eq!(creature.lifecycle_metadata().difficulty_id, 2);
        assert_eq!(creature.lifecycle_metadata().ai_name, "SmartAI");
        assert_eq!(
            creature.lifecycle_metadata().script_name,
            "npc_lifecycle_wolf"
        );
        assert_eq!(creature.lifecycle_metadata().classification, 3);
        assert_eq!(
            creature.lifecycle_metadata().flight_movement_type,
            CreatureFlightMovementType::DisableGravity as u8
        );
        assert_eq!(creature.unit().changed_object_type_mask(), 0);
    }

    #[test]
    fn creature_lifecycle_create_without_spawn_applies_dynamic_respawn_compatibility() {
        let mut record = creature_lifecycle_create_record();
        record.dynamic = false;
        record.spawn = None;
        let static_creature = Creature::create_from_lifecycle(record);
        assert!(static_creature.respawn_compatibility_mode());

        let mut record = creature_lifecycle_create_record();
        record.dynamic = true;
        record.spawn = None;
        let dynamic_creature = Creature::create_from_lifecycle(record);
        assert!(!dynamic_creature.respawn_compatibility_mode());
    }

    #[test]
    fn aim_initialize_like_cpp_represents_normal_creature_without_formation_or_vehicle() {
        let mut create = creature_lifecycle_create_record();
        create.vehicle_id = None;
        create.vehicle_kit_create_input = None;
        let creature = Creature::load_from_db_lifecycle(CreatureLoadFromDbLifecycleRecord {
            create,
            spawn: creature_lifecycle_spawn(),
        });

        let outcome = creature.aim_initialize_like_cpp();

        assert_eq!(outcome.guid, creature.guid());
        assert_eq!(outcome.spawn_id, 44_000);
        assert!(outcome.aim_create_represented);
        assert!(outcome.motion_initialize_represented);
        assert!(!outcome.formation_present);
        assert!(!outcome.formation_leader);
        assert!(!outcome.formation_move_idle_represented);
        assert!(!outcome.motion_initialize_requires_formed_state);
        assert!(outcome.motion_master_initialize_represented);
        assert!(outcome.ai_selected_represented);
        assert!(outcome.ai_initialize_represented);
        assert!(!outcome.vehicle_reset_expected);
        assert!(outcome.succeeded);
    }

    #[test]
    fn aim_initialize_like_cpp_reports_formation_leader_and_non_leader_without_move_idle() {
        let mut create = creature_lifecycle_create_record();
        create.vehicle_id = None;
        create.vehicle_kit_create_input = None;
        let spawn = creature_lifecycle_spawn();
        let mut leader = Creature::load_from_db_lifecycle(CreatureLoadFromDbLifecycleRecord {
            create: create.clone(),
            spawn: spawn.clone(),
        });
        leader.set_formation_info_like_cpp(Some(CreatureFormationInfoLikeCpp {
            leader_spawn_id: spawn.spawn_id,
            follow_dist: 8.0,
            follow_angle_radians: 0.75,
            group_ai: 4,
            leader_waypoint_ids: [21, 22],
        }));

        let leader_outcome = leader.aim_initialize_like_cpp();
        assert!(leader_outcome.formation_present);
        assert!(leader_outcome.formation_leader);
        assert!(!leader_outcome.formation_move_idle_represented);
        assert!(!leader_outcome.motion_initialize_requires_formed_state);
        assert!(leader_outcome.motion_master_initialize_represented);

        let mut non_leader_spawn = spawn;
        non_leader_spawn.spawn_id = 44_001;
        let mut non_leader = Creature::load_from_db_lifecycle(CreatureLoadFromDbLifecycleRecord {
            create,
            spawn: non_leader_spawn,
        });
        non_leader.set_formation_info_like_cpp(Some(CreatureFormationInfoLikeCpp {
            leader_spawn_id: 44_000,
            follow_dist: 8.0,
            follow_angle_radians: 0.75,
            group_ai: 4,
            leader_waypoint_ids: [21, 22],
        }));

        let non_leader_outcome = non_leader.aim_initialize_like_cpp();
        assert!(non_leader_outcome.formation_present);
        assert!(!non_leader_outcome.formation_leader);
        assert!(!non_leader_outcome.formation_move_idle_represented);
        assert!(non_leader_outcome.motion_initialize_requires_formed_state);
        assert!(!non_leader_outcome.motion_master_initialize_represented);
    }

    #[test]
    fn creature_lifecycle_vehicle_entry_missing_preserves_identity_without_local_kit_like_cpp() {
        let mut record = creature_lifecycle_create_record();
        record.vehicle_id = Some(909);
        record.vehicle_kit_create_input = None;

        let creature = Creature::create_from_lifecycle(record);

        assert_eq!(creature.lifecycle_metadata().vehicle_id, Some(909));
        assert!(creature.unit().subsystems().vehicle.kit.is_none());
        let outcome = creature
            .unit()
            .subsystems()
            .vehicle
            .last_create_outcome
            .as_ref()
            .unwrap();
        assert_eq!(outcome.kit_id, Some(909));
        assert!(!outcome.created);
        assert_eq!(outcome.seat_count, 0);
        assert_eq!(outcome.usable_seat_num, 0);
        assert!(!outcome.unit_update_flag_vehicle_represented);
        assert!(!outcome.unit_type_mask_vehicle_represented);
        assert!(!outcome.send_set_vehicle_rec_id_represented);
        assert!(!outcome.set_spellclick_or_player_vehicle_npc_flag_represented);
        assert!(!outcome.remove_spellclick_or_player_vehicle_npc_flag_represented);
        assert!(!outcome.update_display_power_represented);
        assert!(!outcome.init_movement_info_for_base_represented);
    }

    #[test]
    fn creature_lifecycle_create_applies_resolved_base_weapon_damage() {
        let mut record = creature_lifecycle_create_record();
        record.stats.min_damage = 3.5;
        record.stats.max_damage = 7.25;

        let creature = Creature::create_from_lifecycle(record);

        assert_eq!(
            creature.unit().weapon_damage(WeaponAttackType::BaseAttack),
            [3.5, 7.25]
        );
    }

    #[test]
    fn creature_lifecycle_load_from_db_applies_spawn_bridge_state() {
        let create = creature_lifecycle_create_record();
        let spawn = creature_lifecycle_spawn();
        let creature =
            Creature::load_from_db_lifecycle(CreatureLoadFromDbLifecycleRecord { create, spawn });

        assert_eq!(creature.spawn_id(), 44_000);
        assert_eq!(creature.wander_distance(), 12.5);
        assert_eq!(creature.respawn_delay(), 45);
        assert_eq!(creature.respawn_time(), 123_456);
        assert_eq!(
            creature.default_movement_type(),
            MovementGeneratorType::Idle
        );
        assert_eq!(creature.unit().world().map_id(), 571);
        assert_eq!(creature.unit().world().instance_id(), 3);
        assert_eq!(
            creature.unit().world().position(),
            Position::new(1.0, 2.0, 3.0, 4.0)
        );
        assert!(creature.respawn_compatibility_mode());
        assert_eq!(creature.equipment_id(), 9);
        assert_eq!(creature.original_equipment_id(), -9);
        let metadata = creature.lifecycle_metadata();
        assert_eq!(metadata.home_position, Position::new(5.0, 6.0, 7.0, 1.0));
        assert_eq!(metadata.phase_id, Some(169));
        assert_eq!(metadata.terrain_swap_map, Some(609));
        assert_eq!(metadata.spawn_group_id, Some(77));
        assert_eq!(
            metadata.spawn_group_name.as_deref(),
            Some("lifecycle group")
        );
        assert_eq!(metadata.string_id.as_deref(), Some("creature-string"));
        assert!(metadata.add_to_map_requested);
        assert!(metadata.map_insertion_requested);
        assert!(metadata.duplicate_spawn_found);
        assert!(!metadata.is_spawn_active);
        assert!(metadata.inactive_by_spawn_group);
        assert_eq!(creature.unit().changed_object_type_mask(), 0);
    }

    #[test]
    fn creature_lifecycle_health_is_clamped_to_max_health() {
        let mut record = creature_lifecycle_create_record();
        record.stats.max_health = 100;
        record.stats.health = 150;

        let creature = Creature::create_from_lifecycle(record);

        assert_eq!(creature.unit().data().max_health, 100);
        assert_eq!(creature.unit().data().health, 100);
    }

    #[test]
    fn creature_lifecycle_plan_preserves_trinity_critical_order() {
        let plan = CreatureLifecyclePlan::trinity_create_load_from_db();

        assert!(plan.occurs_before(
            CreatureLifecycleStep::LookupTemplateAndDifficulty,
            CreatureLifecycleStep::InitEntryAndCreateFromProto
        ));
        assert!(plan.occurs_before(
            CreatureLifecycleStep::RelocateAndValidatePosition,
            CreatureLifecycleStep::InitEntryAndCreateFromProto
        ));
        assert!(plan.occurs_before(
            CreatureLifecycleStep::SelectLevel,
            CreatureLifecycleStep::UpdateLevelDependantStats
        ));
        assert!(plan.occurs_before(
            CreatureLifecycleStep::UpdateLevelDependantStats,
            CreatureLifecycleStep::AddToMap
        ));
        assert_eq!(plan.steps().last(), Some(&CreatureLifecycleStep::AddToMap));
    }

    #[test]
    fn creature_lifecycle_create_with_spawn_cleans_object_and_unit_masks() {
        let mut record = creature_lifecycle_create_record();
        record.spawn = Some(creature_lifecycle_spawn());

        let creature = Creature::create_from_lifecycle(record);

        assert_eq!(creature.unit().values_update().changed_object_type_mask, 0);
        assert_eq!(
            creature
                .unit()
                .world()
                .object()
                .values_update()
                .changed_object_type_mask,
            0
        );
        assert_eq!(creature.spawn_id(), 44_000);
    }

    #[test]
    fn creature_runtime_just_died_sets_corpse_respawn_and_clears_combat_bridge_state() {
        let now = 10_000;
        let victim = ObjectGuid::new(1, 2);
        let player = ObjectGuid::new(1, 3);
        let mut creature = Creature::new(false);
        creature.set_respawn_compatibility_mode(true);
        creature.set_respawn_delay(45);
        creature.set_corpse_delay(15, false);
        creature.unit_mut().set_target(victim);
        creature.unit_mut().set_attacking(Some(victim));
        creature.set_tapped_by_player(player, &[]);

        let plan = creature.set_death_state_runtime(DeathState::JustDied, now);

        assert_eq!(creature.unit().death_state(), DeathState::Corpse);
        assert_eq!(creature.corpse_remove_time(), now + 15);
        assert_eq!(creature.respawn_time(), now + 45 + 15);
        assert_eq!(creature.unit().data().target, ObjectGuid::EMPTY);
        assert_eq!(creature.unit().attacking(), None);
        assert!(creature.runtime_state().save_respawn_requested);
        assert!(plan.contains(CreatureRuntimeAction::SaveRespawnTime));
        assert!(plan.contains(CreatureRuntimeAction::ClearTarget));

        let mut non_compat = Creature::new(false);
        non_compat.set_respawn_compatibility_mode(false);
        non_compat.set_respawn_delay(45);
        non_compat.set_corpse_delay(15, false);
        non_compat.set_death_state_runtime(DeathState::JustDied, now);
        assert_eq!(non_compat.respawn_time(), now + 45);
        assert_eq!(non_compat.corpse_remove_time(), now + 15);
        assert_eq!(non_compat.unit().death_state(), DeathState::Corpse);
    }

    #[test]
    fn creature_runtime_just_respawned_resets_represented_runtime_state() {
        let player = ObjectGuid::new(1, 3);
        let mut creature = Creature::new(false);
        creature.unit_mut().set_max_health(250);
        creature.unit_mut().set_health(1);
        creature.unit_mut().set_death_state(DeathState::Corpse);
        creature.player_damage_req = 42;
        creature.cannot_reach_target = true;
        creature.cannot_reach_timer = 900;
        creature.set_respawn_time(123);
        creature.corpse_remove_time = 99;
        creature.set_pickpocket_loot_restore(777);
        creature.loot_mode = 0x4;
        creature.set_tapped_by_player(player, &[]);

        let plan = creature.set_death_state_runtime(DeathState::JustRespawned, 5_000);

        assert_eq!(creature.unit().death_state(), DeathState::Alive);
        assert_eq!(creature.unit().data().health, 250);
        assert!(creature.tap_list().is_empty());
        assert_eq!(creature.player_damage_req(), 0);
        assert!(!creature.cannot_reach_target());
        assert_eq!(creature.cannot_reach_timer(), 0);
        assert_eq!(creature.respawn_time(), 0);
        assert_eq!(creature.corpse_remove_time(), 0);
        assert_eq!(creature.pickpocket_loot_restore(), 0);
        assert_eq!(creature.loot_mode(), LOOT_MODE_DEFAULT);
        assert!(creature.trigger_just_appeared());
        assert!(plan.contains(CreatureRuntimeAction::ClearTapList));
        assert!(plan.contains(CreatureRuntimeAction::ResetAi));
    }

    #[test]
    fn creature_runtime_forced_despawn_immediate_matches_compat_and_noncompat_bridges() {
        let now = 20_000;
        let mut compat = Creature::new(false);
        compat.set_respawn_compatibility_mode(true);
        compat.set_respawn_delay(300);
        compat.set_corpse_delay(60, false);

        let plan = compat.forced_despawn_runtime(0, 42, now);

        assert_eq!(compat.unit().death_state(), DeathState::Dead);
        assert_eq!(compat.respawn_delay(), 300);
        assert_eq!(compat.corpse_delay(), 60);
        assert_eq!(compat.respawn_time(), now + 42);
        assert_eq!(compat.corpse_remove_time(), now);
        assert!(plan.contains(CreatureRuntimeAction::DestroyVisibility));
        assert!(plan.contains(CreatureRuntimeAction::RelocateToRespawnPosition));

        let mut delayed = Creature::new(false);
        let delayed_plan = delayed.forced_despawn_runtime(500, 0, now);
        assert!(delayed.runtime_state().forced_despawn_pending);
        assert!(delayed_plan.contains(CreatureRuntimeAction::RequestDelayedForcedDespawn));

        let mut non_compat = Creature::new(false);
        non_compat.set_respawn_compatibility_mode(false);
        non_compat.set_respawn_delay(55);
        let non_compat_plan = non_compat.forced_despawn_runtime(0, 0, now);
        assert_eq!(non_compat.respawn_time(), now + 55);
        assert!(non_compat.runtime_state().save_respawn_requested);
        assert!(non_compat.runtime_state().object_remove_requested);
        assert!(non_compat_plan.contains(CreatureRuntimeAction::SaveRespawnTime));
        assert!(non_compat_plan.contains(CreatureRuntimeAction::RequestObjectRemove));
    }

    #[test]
    fn creature_runtime_all_loot_removed_updates_corpse_and_respawn_like_trinity() {
        let now = 1_000;
        let mut creature = Creature::new(false);
        creature.set_corpse_delay(60, false);
        creature.set_respawn_delay(300);
        creature.corpse_remove_time = now + 600;
        creature.set_respawn_time(now + 100);

        let plan = creature.all_loot_removed_from_corpse(now, 0.5, false);

        assert_eq!(creature.corpse_remove_time(), now + 30);
        assert_eq!(creature.respawn_time(), now + 330);
        assert!(plan.contains(CreatureRuntimeAction::UpdateLoot));

        creature.corpse_remove_time = now + 600;
        creature.set_respawn_time(now + 1_000);
        creature.all_loot_removed_from_corpse(now, 0.5, true);
        assert_eq!(creature.corpse_remove_time(), now);
        assert_eq!(creature.respawn_time(), now + 1_000);

        creature.set_corpse_delay(60, true);
        creature.corpse_remove_time = now + 600;
        creature.set_respawn_time(0);
        creature.all_loot_removed_from_corpse(now, 0.01, false);
        assert_eq!(creature.corpse_remove_time(), now + 60);
    }

    #[test]
    fn creature_runtime_tap_list_group_soft_cap_and_evade_clear_rules() {
        let player = ObjectGuid::new(1, 1);
        let group = [
            ObjectGuid::new(1, 2),
            ObjectGuid::new(1, 3),
            ObjectGuid::new(1, 4),
            ObjectGuid::new(1, 5),
            ObjectGuid::new(1, 6),
        ];
        let mut creature = Creature::new(false);

        creature.set_tapped_by_player(player, &group);

        assert_eq!(creature.tap_list().len(), CREATURE_TAPPERS_SOFT_CAP);
        assert!(creature.is_tapped_by(player));
        assert!(creature.is_tapped_by(group[0]));
        assert!(!creature.is_tapped_by(group[4]));
        assert!(creature.has_loot_recipient());

        creature.set_dont_clear_tap_list_on_evade(true);
        assert!(creature.dont_clear_tap_list_on_evade());
        creature.clear_tap_list_for_evade();
        assert_eq!(creature.tap_list().len(), CREATURE_TAPPERS_SOFT_CAP);
        creature.clear_tap_list();
        assert!(creature.tap_list().is_empty());

        let mut spawned_creature = Creature::new(false);
        spawned_creature.set_spawn_id(99);
        spawned_creature.set_dont_clear_tap_list_on_evade(true);
        assert!(!spawned_creature.dont_clear_tap_list_on_evade());
    }

    #[test]
    fn creature_evading_attacks_matches_cpp_evade_or_cannot_reach() {
        let mut creature = Creature::new(false);

        assert!(!creature.is_in_evade_mode_like_cpp());
        assert!(!creature.is_evading_attacks_like_cpp());

        creature.set_in_evade_mode_like_cpp(true);
        assert!(creature.is_in_evade_mode_like_cpp());
        assert!(creature.is_evading_attacks_like_cpp());

        creature.set_in_evade_mode_like_cpp(false);
        assert!(!creature.is_evading_attacks_like_cpp());

        creature.set_cannot_reach_target_like_cpp(true);
        assert!(creature.cannot_reach_target());
        assert!(creature.is_evading_attacks_like_cpp());

        creature.cannot_reach_timer = 500;
        creature.set_cannot_reach_target_like_cpp(false);
        assert!(!creature.cannot_reach_target());
        assert_eq!(creature.cannot_reach_timer(), 0);
        assert!(!creature.is_evading_attacks_like_cpp());
    }

    #[test]
    fn creature_runtime_update_plan_covers_dead_corpse_and_alive_branches() {
        let now = 50_000;
        let mut dead = Creature::new(false);
        dead.set_respawn_compatibility_mode(true);
        dead.set_respawn_time(now);
        dead.unit_mut().set_death_state(DeathState::Dead);
        let dead_plan = dead.runtime_update_plan(1, now, CreatureRuntimeUpdateContext::default());
        assert!(dead_plan.contains(CreatureRuntimeAction::ResetAi));
        assert_eq!(dead.unit().death_state(), DeathState::Alive);

        let mut corpse = Creature::new(false);
        corpse.set_respawn_compatibility_mode(true);
        corpse.unit_mut().set_death_state(DeathState::Corpse);
        corpse.corpse_remove_time = now;
        let corpse_plan = corpse.runtime_update_plan(
            1,
            now,
            CreatureRuntimeUpdateContext {
                has_loot: true,
                ..CreatureRuntimeUpdateContext::default()
            },
        );
        assert!(corpse_plan.contains(CreatureRuntimeAction::UpdateLoot));
        assert!(corpse_plan.contains(CreatureRuntimeAction::RelocateToRespawnPosition));
        assert_eq!(corpse.unit().death_state(), DeathState::Dead);

        let mut alive = Creature::new(false);
        alive.boundary_check_time = 10;
        alive.combat_pulse_delay = 2;
        alive.combat_pulse_time = 1;
        alive.regen_timer = 1;
        alive.cannot_reach_timer = CREATURE_NOPATH_EVADE_TIME_MS - 5;
        let alive_plan = alive.runtime_update_plan(
            10,
            now,
            CreatureRuntimeUpdateContext {
                is_engaged: true,
                is_dungeon: true,
                has_map_players: true,
                cannot_reach_target: true,
                ..CreatureRuntimeUpdateContext::default()
            },
        );
        assert!(alive_plan.contains(CreatureRuntimeAction::NotifyJustAppeared));
        assert!(alive_plan.contains(CreatureRuntimeAction::BoundaryCheck));
        assert!(alive_plan.contains(CreatureRuntimeAction::CombatPulse));
        assert!(alive_plan.contains(CreatureRuntimeAction::RegeneratePower));
        assert!(alive_plan.contains(CreatureRuntimeAction::Evade(
            CreatureRuntimeEvadeReason::NoPath
        )));
    }
}
