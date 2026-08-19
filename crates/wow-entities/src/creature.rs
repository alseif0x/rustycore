use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use wow_constants::{
    Class, CreatureChaseMovementType, CreatureFlagsExtra, CreatureFlightMovementType,
    CreatureGroundMovementType, CreatureRandomMovementType, CreatureStaticFlags,
    CreatureStaticFlags4, CreatureType, CreatureTypeFlags, DeathState, PowerType, ShapeShiftForm,
    SheathState, TypeId, TypeMask, UnitDynFlags, UnitFlags, UnitFlags2, UnitFlags3, UnitMoveType,
    UnitPvpFlags, UnitStandStateType, UnitState, WeaponAttackType, movement::MovementFlag,
};
use wow_core::{ObjectGuid, Position};
use wow_loot::{
    CreatureLoot, LootInstallOutcome, OwnedLootAuthority, OwnedLootAuthorityStamp,
    OwnedLootSnapshot,
};

use crate::{
    BASE_MAXDAMAGE, BASE_MINDAMAGE, MoveFallPlan, MovementGeneratorKind,
    UNIT_MASK_CONTROLABLE_GUARDIAN, UNIT_MASK_GUARDIAN, UNIT_MASK_MINION, UNIT_MASK_PET,
    UNIT_MASK_TOTEM, UNIT_MASK_VEHICLE, Unit, VehicleAccessory, VehicleSeatAddon, VehicleSeatInfo,
    VisibilityDistanceTypeLikeCpp, VisibleItemValues,
};

pub const CREATURE_REGEN_INTERVAL_MS: u32 = 2_000;
pub const MAX_CREATURE_SPELLS: usize = 8;
pub const DEFAULT_RESPAWN_DELAY_SECS: u32 = 300;
pub const DEFAULT_CORPSE_DELAY_SECS: u32 = 60;
pub const DEFAULT_BOUNDARY_CHECK_TIME_MS: u32 = 2_500;
pub const DEFAULT_MONSTER_SIGHT_DISTANCE: f32 = 50.0;
pub const MAX_SPELL_SCHOOL_LIKE_CPP: u8 = 7;
pub const LOOT_MODE_DEFAULT: u16 = 0x1;
pub const CREATURE_TAPPERS_SOFT_CAP: usize = 5;
pub const CREATURE_NOPATH_EVADE_TIME_MS: u32 = 10_000;
pub const CREATURE_Z_ATTACK_RANGE_LIKE_CPP: f32 = 3.0;
pub const MAX_AGGRO_RESET_TIME_SECS_LIKE_CPP: i64 = 10;
const CREATURE_GROUND_MOVEMENT_TYPE_MAX_LIKE_CPP: u8 = 3;
const CREATURE_FLIGHT_MOVEMENT_TYPE_MAX_LIKE_CPP: u8 = 3;
const CREATURE_CHASE_MOVEMENT_TYPE_MAX_LIKE_CPP: u8 = 3;
const CREATURE_RANDOM_MOVEMENT_TYPE_MAX_LIKE_CPP: u8 = 3;
pub const DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP: u32 = 180_000;

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

pub const fn normalize_creature_chase_movement_type_like_cpp(chase_movement_type: u8) -> u8 {
    if chase_movement_type < CREATURE_CHASE_MOVEMENT_TYPE_MAX_LIKE_CPP {
        chase_movement_type
    } else {
        CreatureChaseMovementType::Run as u8
    }
}

pub const fn normalize_creature_random_movement_type_like_cpp(random_movement_type: u8) -> u8 {
    if random_movement_type < CREATURE_RANDOM_MOVEMENT_TYPE_MAX_LIKE_CPP {
        random_movement_type
    } else {
        CreatureRandomMovementType::Walk as u8
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
    Random = 1,
    Waypoint = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureMovementInform {
    pub movement_type: u8,
    pub movement_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureSpellClickInform {
    pub clicker: ObjectGuid,
    pub spell_click_handled: bool,
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
    pub wander_steps_remaining: u8,
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
    pub trainer_class: u8,
    pub unit_flags: u32,
    pub unit_flags2: u32,
    pub unit_flags3: u32,
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
    pub last_spell_click_inform: Option<CreatureSpellClickInform>,
    /// C++ `CreatureAI::JustReachedHome()`, fired only from
    /// `HomeMovementGenerator<Creature>::DoFinalize` when the generator ended
    /// naturally (`HomeMovementGenerator.cpp:145-157`). It is a distinct AI
    /// callback, not a `MovementInform`.
    pub just_reached_home_pending: bool,
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
            wander_steps_remaining: 0,
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
            trainer_class: 0,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
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
            just_reached_home_pending: false,
            last_spell_click_inform: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureModelDimensions {
    pub bounding_radius: f32,
    pub combat_reach: f32,
}

/// Live creature stats consumed by C++ `SpellCastLogData::Initialize`.
///
/// The represented creature runtime currently has no separate UnitMods rail,
/// so these values are the authoritative totals seeded by
/// `Creature::UpdateLevelDependantStats`. Callers that later represent a stat
/// modifier must update this snapshot at the same mutation boundary.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CreatureCombatLogStatsLikeCpp {
    pub attack_power: i32,
    pub ranged_attack_power: i32,
    pub spell_power: i32,
    pub armor: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureLifecycleStats {
    pub max_health: u64,
    pub health: u64,
    pub power_type: PowerType,
    /// C++ `UnitData::BaseMana`; independent of the selected display power.
    pub base_mana: i32,
    pub max_power: i32,
    pub power: i32,
    pub min_damage: f32,
    pub max_damage: f32,
    pub combat_log: CreatureCombatLogStatsLikeCpp,
}

impl CreatureLifecycleStats {
    pub const fn new(max_health: u64, health: u64, max_mana: i32, mana: i32) -> Self {
        Self {
            max_health,
            health,
            power_type: PowerType::Mana,
            base_mana: max_mana,
            max_power: max_mana,
            power: mana,
            min_damage: BASE_MINDAMAGE,
            max_damage: BASE_MAXDAMAGE,
            combat_log: CreatureCombatLogStatsLikeCpp {
                attack_power: 0,
                ranged_attack_power: 0,
                spell_power: 0,
                armor: 0,
            },
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
    pub required_expansion: u8,
    pub unit_class: u8,
    pub trainer_class: u8,
    pub faction: u32,
    pub npc_flags: u64,
    pub display_id: u32,
    pub model_dimensions: Option<CreatureModelDimensions>,
    pub scale: f32,
    pub speed_walk: f32,
    pub speed_run: f32,
    pub spells: [u32; MAX_CREATURE_SPELLS],
    pub classification: u32,
    pub damage_school: u8,
    pub unit_flags: u32,
    pub unit_flags2: u32,
    pub unit_flags3: u32,
    pub flags_extra: u32,
    pub static_flags: [u32; 8],
    pub creature_type: u32,
    pub type_flags: u32,
    pub loot_id: u32,
    pub skin_loot_id: u32,
    pub gold_min: u32,
    pub gold_max: u32,
    pub movement_type: MovementGeneratorType,
    pub ground_movement_type: u8,
    pub swim_allowed: bool,
    pub flight_movement_type: u8,
    pub rooted: bool,
    pub chase_movement_type: u8,
    pub random_movement_type: u8,
    pub interaction_pause_timer_ms: u32,
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

/// Represented subset of Trinity `CreatureAddon`.
///
/// C++ source:
/// - `CreatureData.h::CreatureAddon`
/// - `Creature::GetCreatureAddon`
/// - `Creature::LoadCreaturesAddon`
///
/// This record intentionally carries only fields that `wow-entities` currently models locally,
/// plus `PathId` as a data seam for C++ addon movement selection.
/// DB loading, template-vs-spawn fallback, path runtime, anim kit packet fanout,
/// and full runtime visibility routing are follow-up runtime gaps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureAddonLifecycleRecordLikeCpp {
    pub path_id: u32,
    pub mount_display_id: u32,
    pub stand_state: UnitStandStateType,
    pub vis_flags: u8,
    pub anim_tier: u8,
    pub sheath_state: SheathState,
    pub pvp_flags: UnitPvpFlags,
    pub emote: u32,
    pub ai_anim_kit_id: u16,
    pub movement_anim_kit_id: u16,
    pub melee_anim_kit_id: u16,
    pub visibility_distance_type: VisibilityDistanceTypeLikeCpp,
    pub auras: Vec<u32>,
    pub aura_applications: Vec<CreatureAddonAuraApplicationLikeCpp>,
}

impl Default for CreatureAddonLifecycleRecordLikeCpp {
    fn default() -> Self {
        Self {
            path_id: 0,
            mount_display_id: 0,
            stand_state: UnitStandStateType::Stand,
            vis_flags: 0,
            anim_tier: 0,
            sheath_state: SheathState::Unarmed,
            pvp_flags: UnitPvpFlags::empty(),
            emote: 0,
            ai_anim_kit_id: 0,
            movement_anim_kit_id: 0,
            melee_anim_kit_id: 0,
            visibility_distance_type: VisibilityDistanceTypeLikeCpp::Normal,
            auras: Vec::new(),
            aura_applications: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureAddonAuraApplicationLikeCpp {
    pub spell_id: u32,
    pub effect_mask: u32,
    pub flags: u32,
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
    pub selected_virtual_items: [(i32, u16, u16); 3],
    pub corpse_delay: u32,
    pub ignore_corpse_decay_ratio: bool,
    pub addon: Option<CreatureAddonLifecycleRecordLikeCpp>,
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
    pub required_expansion: u8,
    pub unit_class: u8,
    pub trainer_class: u8,
    pub classification: u32,
    pub damage_school: u8,
    pub flags_extra: u32,
    pub static_flags: [u32; 8],
    pub ground_movement_type: u8,
    pub swim_allowed: bool,
    pub flight_movement_type: u8,
    pub rooted: bool,
    pub chase_movement_type: u8,
    pub random_movement_type: u8,
    pub interaction_pause_timer_ms: u32,
    pub creature_type: u32,
    pub type_flags: u32,
    pub selected_level: u8,
    pub selected_display_id: u32,
    pub selected_model_dimensions: Option<CreatureModelDimensions>,
    pub spawn_health: Option<u64>,
    pub spawn_mana: Option<i32>,
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
    pub addon: Option<CreatureAddonLifecycleRecordLikeCpp>,
}

impl Default for CreatureLifecycleMetadata {
    fn default() -> Self {
        Self {
            template_entry: 0,
            original_entry: 0,
            difficulty_id: 0,
            ai_name: String::new(),
            script_name: String::new(),
            required_expansion: 0,
            unit_class: 0,
            trainer_class: 0,
            classification: 0,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            flags_extra: 0,
            static_flags: [0; 8],
            ground_movement_type: CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: CreatureFlightMovementType::None as u8,
            rooted: false,
            chase_movement_type: CreatureChaseMovementType::Run as u8,
            random_movement_type: CreatureRandomMovementType::Walk as u8,
            interaction_pause_timer_ms: DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            creature_type: 0,
            type_flags: 0,
            selected_level: 0,
            selected_display_id: 0,
            selected_model_dimensions: None,
            spawn_health: None,
            spawn_mana: None,
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
            addon: None,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureSpellFocusStateLikeCpp {
    pub spell_id: Option<u32>,
    pub delay_ms: u32,
    pub target: ObjectGuid,
    pub orientation: f32,
    pub ai_does_not_face_target: bool,
}

impl Default for CreatureSpellFocusStateLikeCpp {
    fn default() -> Self {
        Self {
            spell_id: None,
            delay_ms: 0,
            target: ObjectGuid::EMPTY,
            orientation: 0.0,
            ai_does_not_face_target: false,
        }
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
    ReleaseSpellFocus,
    CancelSpellFocusReacquire,
    ClearTarget,
    ClearNpcFlags,
    ClearMount,
    Deactivate,
    ClearAssistanceSearch,
    MoveFall,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureDeathFallContextLikeCpp {
    pub is_underwater: bool,
    pub has_valid_ground_height: bool,
    pub vertical_delta: f32,
    pub movement_id: u32,
    pub duration_ms: u32,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CreatureOwnedLoot {
    gold: u32,
    unlooted_count: u32,
}

impl CreatureOwnedLoot {
    pub const fn new(gold: u32, unlooted_count: u32) -> Self {
        Self {
            gold,
            unlooted_count,
        }
    }

    pub const fn gold(&self) -> u32 {
        self.gold
    }

    pub const fn unlooted_count(&self) -> u32 {
        self.unlooted_count
    }

    pub const fn is_looted_like_cpp(&self) -> bool {
        self.gold == 0 && self.unlooted_count == 0
    }
}

fn creature_owned_loot_from_snapshot(snapshot: &OwnedLootSnapshot) -> CreatureOwnedLoot {
    CreatureOwnedLoot::new(snapshot.loot.coins, u32::from(snapshot.loot.unlooted_count))
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
    waypoint_path_id: u32,
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
    unit_type_mask: u32,
    gossip_menu_id: u32,
    sparring_health_pct: f32,
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
    spell_focus: CreatureSpellFocusStateLikeCpp,
    combat_log_stats: CreatureCombatLogStatsLikeCpp,
    /// Monotonic identity for the creature loot-producing lifetime. Async
    /// `Unit::Kill` generation captures this value and may install its pools
    /// only while the same death lifetime is still current. Corpse removal
    /// and respawn advance it, preventing an old generator from winning an
    /// ABA race against a later incarnation of the same spawn GUID.
    loot_lifecycle_revision: u64,
    loot_authority: OwnedLootAuthority,
    shared_loot: Option<CreatureOwnedLoot>,
    personal_loot: HashMap<ObjectGuid, CreatureOwnedLoot>,
}

impl Creature {
    pub fn new(is_world_object: bool) -> Self {
        let mut unit = Unit::new(is_world_object);
        unit.set_type(TypeId::Unit, TypeMask::OBJECT | TypeMask::UNIT);
        unit.set_power_index(PowerType::Mana, Some(0));
        unit.set_power_index(PowerType::ComboPoints, Some(2));
        unit.subsystems_mut()
            .combat
            .initialize_threat_list_capability(true);

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
            waypoint_path_id: 0,
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
            unit_type_mask: 0,
            gossip_menu_id: 0,
            sparring_health_pct: 0.0,
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
            spell_focus: CreatureSpellFocusStateLikeCpp::default(),
            combat_log_stats: CreatureCombatLogStatsLikeCpp::default(),
            loot_lifecycle_revision: 0,
            loot_authority: OwnedLootAuthority::new(),
            shared_loot: None,
            personal_loot: HashMap::new(),
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
        // C++ `Creature::InitEntry` sets walk/run from creature_template and
        // swim/flight to 1.0 immediately after haste/time-rate defaults
        // (`Creature.cpp:540-550`).
        self.unit.set_mod_casting_speed_like_cpp(1.0);
        self.unit.set_mod_spell_haste_like_cpp(1.0);
        self.unit.set_mod_haste_like_cpp(1.0);
        self.unit.set_mod_ranged_haste_like_cpp(1.0);
        self.unit.set_mod_haste_regen_like_cpp(1.0);
        self.unit.set_mod_time_rate_like_cpp(1.0);
        self.unit
            .set_speed_rate_like_cpp(UnitMoveType::Walk, template.speed_walk);
        self.unit
            .set_speed_rate_like_cpp(UnitMoveType::Run, template.speed_run);
        self.unit.set_speed_rate_like_cpp(UnitMoveType::Swim, 1.0);
        self.unit.set_speed_rate_like_cpp(UnitMoveType::Flight, 1.0);
        self.unit.set_can_dual_wield_like_cpp(
            CreatureFlagsExtra::from_bits_truncate(template.flags_extra)
                .contains(CreatureFlagsExtra::USE_OFFHAND_ATTACK),
        );
        self.spells = template.spells;
        self.equipment_id = equipment_id;
        self.original_equipment_id = original_equipment_id;
        for (index, &(item_id, item_appearance_mod_id, item_visual)) in
            record.selected_virtual_items.iter().enumerate()
        {
            let visible = (item_id != 0 || item_appearance_mod_id != 0 || item_visual != 0)
                .then_some(VisibleItemValues {
                    item_id,
                    item_appearance_mod_id,
                    item_visual,
                });
            self.unit.set_virtual_item(index, visible);
        }
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
            let outcome = self
                .unit
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
            if outcome.unit_type_mask_vehicle_represented {
                self.add_unit_type_mask_like_cpp(UNIT_MASK_VEHICLE);
            }
        }
        self.default_movement_type = spawn
            .map(|spawn| spawn.movement_type)
            .unwrap_or(template.movement_type);
        self.sync_motion_default_generator_like_cpp();
        self.set_corpse_delay(record.corpse_delay, record.ignore_corpse_decay_ratio);
        self.set_respawn_compatibility_mode(!record.dynamic);
        if let Some(spawn) = spawn {
            self.apply_spawn_lifecycle(spawn);
        }

        self.unit.set_level(record.selected_level);
        self.set_power_type(record.stats.power_type);
        self.unit.set_max_health(record.stats.max_health);
        self.unit.set_health(record.stats.health);
        self.unit.set_create_mana_like_cpp(record.stats.base_mana);
        self.unit
            .set_max_power(record.stats.power_type, record.stats.max_power);
        self.unit
            .set_power(record.stats.power_type, record.stats.power);
        self.unit.set_weapon_damage(
            WeaponAttackType::BaseAttack,
            record.stats.min_damage,
            record.stats.max_damage,
        );
        self.combat_log_stats = record.stats.combat_log;
        self.set_melee_damage_school_like_cpp(template.damage_school);
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
        self.ai_ownership.npc_flags = template.npc_flags as u32;
        self.ai_ownership.npc_flags2 = (template.npc_flags >> 32) as u32;
        self.ai_ownership.trainer_class = template.trainer_class;
        self.ai_ownership.unit_flags = template.unit_flags;
        self.ai_ownership.unit_flags2 = template.unit_flags2;
        self.ai_ownership.unit_flags3 = template.unit_flags3;
        self.ai_ownership.min_damage = record.stats.min_damage.max(0.0) as u32;
        self.ai_ownership.max_damage = record.stats.max_damage.max(0.0) as u32;
        // C++ `Creature::GetLootId` first honors an object-local `m_lootId` override and then
        // reads the selected `CreatureDifficulty::LootID`; death/skinning generation reads
        // GoldMin/GoldMax/SkinLootID from that same difficulty (`Creature.cpp:1317-1323`,
        // `Unit.cpp:10545-10575,10675`). Keep those values on the runtime creature instead of
        // falling back to `CreatureAiOwnershipState`'s zero defaults.
        self.ai_ownership.loot_id = template.loot_id;
        self.ai_ownership.skin_loot_id = template.skin_loot_id;
        self.ai_ownership.gold_min = template.gold_min;
        self.ai_ownership.gold_max = template.gold_max;
        self.unit
            .set_npc_flags_like_cpp(self.ai_ownership.npc_flags);
        self.unit
            .set_npc_flags2_like_cpp(self.ai_ownership.npc_flags2);
        self.unit
            .set_unit_flags_like_cpp(UnitFlags::from_bits_truncate(template.unit_flags));
        self.unit
            .set_unit_flags2_like_cpp(UnitFlags2::from_bits_truncate(template.unit_flags2));
        self.unit
            .set_unit_flags3_like_cpp(UnitFlags3::from_bits_truncate(template.unit_flags3));
        self.lifecycle_metadata.ground_movement_type =
            normalize_creature_ground_movement_type_like_cpp(template.ground_movement_type);
        self.load_creatures_addon_represented_like_cpp(record.addon.as_ref());

        let init_entry_static_flags = self.init_entry_static_flags_like_cpp(template);
        let init_entry_rooted = CreatureStaticFlags::from_bits_truncate(init_entry_static_flags[0])
            .contains(CreatureStaticFlags::SESSILE);

        self.lifecycle_metadata = CreatureLifecycleMetadata {
            template_entry: template.entry,
            original_entry: template.original_entry,
            difficulty_id: template.difficulty_id,
            ai_name: template.ai_name.clone(),
            script_name: template.script_name.clone(),
            required_expansion: template.required_expansion,
            unit_class: template.unit_class,
            trainer_class: template.trainer_class,
            classification: template.classification,
            damage_school: template.damage_school,
            flags_extra: template.flags_extra,
            static_flags: init_entry_static_flags,
            ground_movement_type: normalize_creature_ground_movement_type_like_cpp(
                template.ground_movement_type,
            ),
            swim_allowed: template.swim_allowed,
            flight_movement_type: normalize_creature_flight_movement_type_like_cpp(
                template.flight_movement_type,
            ),
            rooted: init_entry_rooted,
            chase_movement_type: normalize_creature_chase_movement_type_like_cpp(
                template.chase_movement_type,
            ),
            random_movement_type: normalize_creature_random_movement_type_like_cpp(
                template.random_movement_type,
            ),
            interaction_pause_timer_ms: template.interaction_pause_timer_ms,
            creature_type: template.creature_type,
            type_flags: template.type_flags,
            selected_level: record.selected_level,
            selected_display_id: record.selected_display_id,
            selected_model_dimensions: record
                .selected_model_dimensions
                .or(template.model_dimensions),
            spawn_health: Some(record.stats.health),
            spawn_mana: Some(record.stats.power),
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
            addon: record.addon,
        };

        self.set_template_rooted_like_cpp(init_entry_rooted);

        // C++ `Creature::Create` (`Creature.cpp:1154-1155`) adds
        // `UNIT_STATE_IGNORE_PATHFINDING` for templates carrying
        // `CREATURE_FLAG_EXTRA_IGNORE_PATHFINDING` (0x20000000), and
        // `PathGenerator::CalculatePath` then skips the navmesh entirely for
        // them (`PathGenerator.cpp:80`).
        self.refresh_ignore_pathfinding_state_like_cpp();

        self.refresh_threat_list_capability_like_cpp();
        self.clear_data_changes();
    }

    fn init_entry_static_flags_like_cpp(
        &self,
        template: &CreatureTemplateLifecycleRecord,
    ) -> [u32; 8] {
        let mut static_flags = template.static_flags;
        let flags_extra = CreatureFlagsExtra::from_bits_truncate(template.flags_extra);

        let mut primary = CreatureStaticFlags::from_bits_truncate(static_flags[0]);
        primary.set(
            CreatureStaticFlags::NO_XP,
            template.creature_type == CreatureType::Critter as u32
                || self.has_unit_type_mask_like_cpp(UNIT_MASK_PET)
                || self.has_unit_type_mask_like_cpp(UNIT_MASK_TOTEM)
                || flags_extra.contains(CreatureFlagsExtra::NO_XP),
        );
        static_flags[0] = primary.bits();

        let mut flags4 = CreatureStaticFlags4::from_bits_truncate(static_flags[3]);
        flags4.set(
            CreatureStaticFlags4::TREAT_AS_RAID_UNIT_FOR_HELPFUL_SPELLS,
            CreatureTypeFlags::from_bits_truncate(template.type_flags)
                .contains(CreatureTypeFlags::TREAT_AS_RAID_UNIT),
        );
        static_flags[3] = flags4.bits();

        static_flags
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
        self.sync_motion_default_generator_like_cpp();
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

    pub fn set_spawn_health_like_cpp(&mut self) {
        self.unit.set_health(
            self.lifecycle_metadata
                .spawn_health
                .unwrap_or(self.unit.data().max_health),
        );
        if let Some(spawn_mana) = self.lifecycle_metadata.spawn_mana {
            self.unit.set_power(PowerType::Mana, spawn_mana);
        }
    }

    pub fn set_required_expansion_runtime_like_cpp(&mut self, required_expansion: u8) {
        self.lifecycle_metadata.required_expansion = required_expansion;
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

    /// C++ `Creature::RefreshCanSwimFlag`: remember whether `UNIT_FLAG_CAN_SWIM`
    /// was absent out of combat, then add it while engaged when the movement
    /// template otherwise allows the creature to enter water.
    pub fn refresh_can_swim_flag_like_cpp(&mut self, recheck: bool) {
        if !self.is_missing_can_swim_flag_out_of_combat || recheck {
            self.is_missing_can_swim_flag_out_of_combat = !self
                .unit
                .unit_flags_like_cpp()
                .contains(UnitFlags::CAN_SWIM);
        }

        if self.is_missing_can_swim_flag_out_of_combat && self.can_enter_water_like_cpp() {
            let flags = self.unit.unit_flags_like_cpp() | UnitFlags::CAN_SWIM;
            self.unit.set_unit_flags_like_cpp(flags);
        }
    }

    pub fn restore_can_swim_flag_after_home_like_cpp(&mut self) {
        if self.is_missing_can_swim_flag_out_of_combat {
            let flags = self.unit.unit_flags_like_cpp() & !UnitFlags::CAN_SWIM;
            self.unit.set_unit_flags_like_cpp(flags);
        }
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

    pub fn can_hover_like_cpp(&self) -> bool {
        self.lifecycle_metadata.ground_movement_type == CreatureGroundMovementType::Hover as u8
            || self.is_hovering_like_cpp()
    }

    pub fn is_hovering_like_cpp(&self) -> bool {
        self.runtime_state
            .movement_flags
            .contains(MovementFlag::HOVER)
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

    pub const fn unit_type_mask_like_cpp(&self) -> u32 {
        self.unit_type_mask
    }

    pub const fn has_unit_type_mask_like_cpp(&self, mask: u32) -> bool {
        self.unit_type_mask & mask != 0
    }

    pub fn add_unit_type_mask_like_cpp(&mut self, mask: u32) {
        self.unit_type_mask |= mask;
        self.refresh_threat_list_capability_like_cpp();
    }

    pub fn remove_unit_type_mask_like_cpp(&mut self, mask: u32) {
        self.unit_type_mask &= !mask;
        self.refresh_threat_list_capability_like_cpp();
    }

    pub fn can_have_threat_list_like_cpp(&self) -> bool {
        if CreatureFlagsExtra::from_bits_truncate(self.lifecycle_metadata.flags_extra)
            .contains(CreatureFlagsExtra::TRIGGER)
        {
            return false;
        }
        if self.has_unit_type_mask_like_cpp(UNIT_MASK_PET | UNIT_MASK_TOTEM) {
            return false;
        }
        if self.has_unit_type_mask_like_cpp(UNIT_MASK_MINION | UNIT_MASK_GUARDIAN) {
            return false;
        }
        true
    }

    fn refresh_threat_list_capability_like_cpp(&mut self) {
        let can_have_threat_list = self.can_have_threat_list_like_cpp();
        self.unit
            .subsystems_mut()
            .combat
            .initialize_threat_list_capability(can_have_threat_list);
    }

    pub const fn is_totem_unit_type_like_cpp(&self) -> bool {
        self.has_unit_type_mask_like_cpp(UNIT_MASK_TOTEM)
    }

    pub const fn is_guardian_unit_type_like_cpp(&self) -> bool {
        self.has_unit_type_mask_like_cpp(UNIT_MASK_GUARDIAN)
    }

    pub const fn is_controlable_guardian_unit_type_like_cpp(&self) -> bool {
        self.has_unit_type_mask_like_cpp(UNIT_MASK_CONTROLABLE_GUARDIAN)
    }

    pub const fn is_vehicle_unit_type_like_cpp(&self) -> bool {
        self.has_unit_type_mask_like_cpp(UNIT_MASK_VEHICLE)
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

    pub const fn spell_focus_state_like_cpp(&self) -> CreatureSpellFocusStateLikeCpp {
        self.spell_focus
    }

    pub fn set_represented_spell_focus_like_cpp(
        &mut self,
        spell_id: u32,
        target: ObjectGuid,
        orientation: f32,
        ai_does_not_face_target: bool,
    ) {
        self.spell_focus = CreatureSpellFocusStateLikeCpp {
            spell_id: Some(spell_id),
            delay_ms: 0,
            target: self.unit.data().target,
            orientation,
            ai_does_not_face_target,
        };
        if ai_does_not_face_target {
            self.unit.add_unit_state(UnitState::FOCUSING.bits());
        }
        let new_target = if ai_does_not_face_target {
            ObjectGuid::EMPTY
        } else {
            target
        };
        self.unit.set_target(new_target);
    }

    pub fn has_spell_focus_like_cpp(&self, focus_spell_id: Option<u32>) -> bool {
        if self.unit.is_dead() {
            return false;
        }

        match focus_spell_id {
            Some(focus_spell_id) => self.spell_focus.spell_id == Some(focus_spell_id),
            None => self.spell_focus.spell_id.is_some() || self.spell_focus.delay_ms != 0,
        }
    }

    pub fn set_target_like_cpp(&mut self, guid: ObjectGuid) {
        if self.has_spell_focus_like_cpp(None) {
            self.spell_focus.target = guid;
        } else {
            self.unit.set_target(guid);
        }
    }

    pub fn release_spell_focus_like_cpp(
        &mut self,
        focus_spell_id: Option<u32>,
        with_delay: bool,
        is_pet: bool,
        cannot_turn: bool,
    ) {
        let Some(active_spell_id) = self.spell_focus.spell_id else {
            return;
        };
        if focus_spell_id.is_some_and(|focus_spell_id| focus_spell_id != active_spell_id) {
            return;
        }

        if self.spell_focus.ai_does_not_face_target {
            self.unit.clear_unit_state(UnitState::FOCUSING.bits());
        }

        if is_pet {
            if !cannot_turn {
                self.reacquire_spell_focus_target_like_cpp(cannot_turn);
            }
        } else {
            self.spell_focus.delay_ms = if with_delay { 1000 } else { 1 };
        }
        self.spell_focus.spell_id = None;
    }

    pub fn reacquire_spell_focus_target_like_cpp(&mut self, cannot_turn: bool) {
        if !self.has_spell_focus_like_cpp(None) {
            return;
        }

        self.unit.set_target(self.spell_focus.target);
        if cannot_turn {
            // C++ skips target-facing/orientation restore when CannotTurn() is true.
        }
        self.spell_focus.delay_ms = 0;
    }

    pub fn do_not_reacquire_spell_focus_target_like_cpp(&mut self) {
        self.spell_focus.delay_ms = 0;
        self.spell_focus.spell_id = None;
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

    /// Records C++ `CreatureAI::JustReachedHome()`
    /// (`HomeMovementGenerator.cpp:156`).
    pub fn record_ai_just_reached_home(&mut self) {
        self.ai_ownership.just_reached_home_pending = true;
    }

    pub fn take_ai_just_reached_home(&mut self) -> bool {
        std::mem::take(&mut self.ai_ownership.just_reached_home_pending)
    }

    pub fn record_ai_spell_click_inform(&mut self, clicker: ObjectGuid, spell_click_handled: bool) {
        self.ai_ownership.last_spell_click_inform = Some(CreatureSpellClickInform {
            clicker,
            spell_click_handled,
        });
    }

    pub fn take_ai_spell_click_inform(&mut self) -> Option<CreatureSpellClickInform> {
        self.ai_ownership.last_spell_click_inform.take()
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
        // C++ `Creature::AtEngage` refreshes this before any chase spline is
        // built, so `MoveSplineInit` observes the effective combat capability.
        self.refresh_can_swim_flag_like_cpp(false);
        self.ai_ownership.state = CreatureAiState::InCombat;
        self.ai_ownership.combat_target = Some(attacker);
        self.ai_ownership.move_target = None;
        self.unit.set_attacking(Some(attacker));
    }

    pub fn reset_ai_combat(&mut self, now_ms: u64) {
        if self.ai_ownership.state == CreatureAiState::Dead
            || self.unit.is_dead()
            || self.unit.data().health == 0
        {
            self.advance_loot_lifecycle_revision_like_cpp();
        }
        self.ai_ownership.state = CreatureAiState::Returning;
        self.ai_ownership.combat_target = None;
        self.ai_ownership.move_target = Some(self.ai_ownership.home_position);
        self.ai_ownership.move_start_ms = now_ms;
        self.ai_ownership.death_time_ms = None;
        self.ai_ownership.corpse_despawn_at_ms = None;
        self.unit.set_attacking(None);
        self.last_damaged_time = 0;
    }

    /// Apply damage and return `true` when this call killed the creature.
    pub fn take_ai_damage(&mut self, damage: u32, now_ms: u64) -> bool {
        self.take_ai_damage_at_game_time_like_cpp(damage, now_ms, game_time_secs_like_cpp())
    }

    pub fn take_ai_damage_at_game_time_like_cpp(
        &mut self,
        damage: u32,
        now_ms: u64,
        game_time_secs: i64,
    ) -> bool {
        if self.apply_ai_damage_before_death_state_at_game_time_like_cpp(
            damage,
            now_ms,
            game_time_secs,
        ) {
            self.mark_ai_dead_at_game_time_like_cpp(now_ms, game_time_secs);
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
            self.advance_loot_lifecycle_revision_like_cpp();
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
        self.mark_ai_dead_at_game_time_like_cpp(now_ms, game_time_secs_like_cpp());
    }

    pub fn mark_ai_dead_at_game_time_like_cpp(&mut self, now_ms: u64, game_time_secs: i64) {
        if self.ai_ownership.state != CreatureAiState::Dead {
            self.advance_loot_lifecycle_revision_like_cpp();
        }
        self.ai_ownership.state = CreatureAiState::Dead;
        self.ai_ownership.combat_target = None;
        self.ai_ownership.move_target = None;
        self.ai_ownership.death_time_ms = Some(now_ms);
        self.ai_ownership.corpse_despawn_at_ms =
            Some(now_ms.saturating_add(u64::from(self.corpse_delay).saturating_mul(1_000)));
        self.unit.set_health(0);
        self.respawn_delay = self.ai_ownership.respawn_time_secs.min(u64::from(u32::MAX)) as u32;
        self.set_death_state_runtime(DeathState::JustDied, game_time_secs);
        self.unit.set_health(0);
    }

    pub fn complete_ai_death_state_after_kill_hooks_like_cpp(
        &mut self,
        now_ms: u64,
        game_time_secs: i64,
    ) {
        if self.ai_ownership.state != CreatureAiState::Dead || self.unit.is_dead() {
            return;
        }
        self.ai_ownership.death_time_ms = Some(now_ms);
        self.ai_ownership.corpse_despawn_at_ms =
            Some(now_ms.saturating_add(u64::from(self.corpse_delay).saturating_mul(1_000)));
        self.set_death_state_runtime(DeathState::JustDied, game_time_secs);
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
        !self.is_template_rooted_like_cpp()
            && (self.ai_ownership.npc_flags == 0 || (self.ai_ownership.npc_flags & 0x80) == 0)
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
        self.try_ai_aggro_with_effective_range_like_cpp(
            player_guid,
            player_pos,
            player_combat_reach,
            self.ai_ownership.aggro_radius,
        )
    }

    pub fn try_ai_aggro_with_effective_range_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        player_pos: &Position,
        player_combat_reach: f32,
        effective_aggro_range: f32,
    ) -> bool {
        if !self.ai_is_alive()
            || matches!(
                self.ai_ownership.state,
                CreatureAiState::InCombat | CreatureAiState::Returning | CreatureAiState::Dead
            )
        {
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

        if self.ai_position().distance(player_pos) <= effective_aggro_range.max(0.0) {
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
        self.unit
            .set_unit_flags_like_cpp(UnitFlags::from_bits_truncate(unit_flags));
        self.set_display_id(display_id, true, None);
        self.set_faction(faction);
    }

    pub fn set_trainer_class_runtime_like_cpp(&mut self, trainer_class: u8) {
        self.ai_ownership.trainer_class = trainer_class;
        self.lifecycle_metadata.trainer_class = trainer_class;
    }

    pub const fn trainer_class_like_cpp(&self) -> u8 {
        self.ai_ownership.trainer_class
    }

    pub fn set_npc_flags2_runtime_like_cpp(&mut self, npc_flags2: u32) {
        self.ai_ownership.npc_flags2 = npc_flags2;
        self.unit.set_npc_flags2_like_cpp(npc_flags2);
    }

    pub fn set_unit_flags2_runtime_like_cpp(&mut self, unit_flags2: u32) {
        self.ai_ownership.unit_flags2 = unit_flags2;
        self.unit
            .set_unit_flags2_like_cpp(UnitFlags2::from_bits_truncate(unit_flags2));
    }

    pub fn set_unit_flags3_runtime_like_cpp(&mut self, unit_flags3: u32) {
        self.ai_ownership.unit_flags3 = unit_flags3;
        self.unit
            .set_unit_flags3_like_cpp(UnitFlags3::from_bits_truncate(unit_flags3));
    }

    pub fn set_flags_extra_runtime_like_cpp(&mut self, flags_extra: u32) {
        self.lifecycle_metadata.flags_extra = flags_extra;
        // C++ derives `UNIT_STATE_IGNORE_PATHFINDING` from
        // `CREATURE_FLAG_EXTRA_IGNORE_PATHFINDING` once, in `Creature::Create`
        // (`Creature.cpp:1154-1155`). This setter is the runtime seam through
        // which legacy registrations apply template `flags_extra` without going
        // through the create lifecycle, so it has to reach the same state —
        // otherwise the same template would use the navmesh on one path and skip
        // it on the other.
        self.refresh_ignore_pathfinding_state_like_cpp();
    }

    /// Applies the `CREATURE_FLAG_EXTRA_IGNORE_PATHFINDING` →
    /// `UNIT_STATE_IGNORE_PATHFINDING` derivation of C++ `Creature::Create`
    /// (`Creature.cpp:1154-1155`).
    ///
    /// C++ only ever *adds* the state there, and deliberately excludes it from
    /// `UNIT_STATE_ALL_ERASABLE` so it survives respawn, so this never clears a
    /// state the flag does not ask for.
    fn refresh_ignore_pathfinding_state_like_cpp(&mut self) {
        if CreatureFlagsExtra::from_bits_truncate(self.lifecycle_metadata.flags_extra)
            .contains(CreatureFlagsExtra::IGNORE_PATHFINDING)
        {
            self.unit
                .add_unit_state(UnitState::IGNORE_PATHFINDING.bits());
        }
    }

    pub fn set_static_flags_runtime_like_cpp(&mut self, static_flags: [u32; 8]) {
        self.lifecycle_metadata.static_flags = static_flags;
    }

    pub fn is_template_rooted_like_cpp(&self) -> bool {
        CreatureStaticFlags::from_bits_truncate(self.lifecycle_metadata.static_flags[0])
            .contains(CreatureStaticFlags::SESSILE)
    }

    /// Mirrors `Creature::CanGiveExperience`: critters, pets, totems, and
    /// templates carrying `CREATURE_FLAG_EXTRA_NO_XP` cannot reward kill XP.
    pub fn can_give_experience_like_cpp(&self) -> bool {
        !CreatureStaticFlags::from_bits_truncate(self.lifecycle_metadata.static_flags[0])
            .contains(CreatureStaticFlags::NO_XP)
            && !CreatureFlagsExtra::from_bits_truncate(self.lifecycle_metadata.flags_extra)
                .contains(CreatureFlagsExtra::NO_XP)
            && !self.has_unit_type_mask_like_cpp(UNIT_MASK_PET | UNIT_MASK_TOTEM)
    }

    pub fn set_template_rooted_like_cpp(&mut self, rooted: bool) {
        self.lifecycle_metadata.rooted = rooted;
        let mut flags =
            CreatureStaticFlags::from_bits_truncate(self.lifecycle_metadata.static_flags[0]);
        flags.set(CreatureStaticFlags::SESSILE, rooted);
        self.lifecycle_metadata.static_flags[0] = flags.bits();
        if rooted {
            self.unit.add_unit_state(UnitState::ROOT.bits());
            self.runtime_state
                .movement_flags
                .remove(MovementFlag::MASK_MOVING);
            self.runtime_state.movement_flags.insert(MovementFlag::ROOT);
        } else {
            self.unit.clear_unit_state(UnitState::ROOT.bits());
            self.runtime_state.movement_flags.remove(MovementFlag::ROOT);
        }
    }

    pub fn can_melee_like_cpp(&self) -> bool {
        !CreatureStaticFlags::from_bits_truncate(self.lifecycle_metadata.static_flags[0])
            .contains(CreatureStaticFlags::NO_MELEE_FLEE)
    }

    pub fn set_ai_identity_names_runtime_like_cpp(
        &mut self,
        ai_name: impl Into<String>,
        script_name: impl Into<String>,
    ) {
        self.lifecycle_metadata.ai_name = ai_name.into();
        self.lifecycle_metadata.script_name = script_name.into();
    }

    /// Represented seam for TrinityCore `Creature::LoadFromDB`, which stores
    /// `CreatureData::StringId` in `m_stringIds[1]` after spawn health/default
    /// movement initialization.
    pub fn set_spawn_string_id_runtime_like_cpp(&mut self, string_id: Option<String>) {
        self.lifecycle_metadata.string_id = string_id;
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

    pub const fn chase_movement_type_like_cpp(&self) -> u8 {
        self.lifecycle_metadata.chase_movement_type
    }

    pub fn set_chase_movement_type_runtime_like_cpp(&mut self, chase_movement_type: u8) {
        self.lifecycle_metadata.chase_movement_type =
            normalize_creature_chase_movement_type_like_cpp(chase_movement_type);
    }

    pub const fn random_movement_type_like_cpp(&self) -> u8 {
        self.lifecycle_metadata.random_movement_type
    }

    pub fn set_random_movement_type_runtime_like_cpp(&mut self, random_movement_type: u8) {
        self.lifecycle_metadata.random_movement_type =
            normalize_creature_random_movement_type_like_cpp(random_movement_type);
    }

    pub const fn interaction_pause_timer_ms_like_cpp(&self) -> u32 {
        self.lifecycle_metadata.interaction_pause_timer_ms
    }

    pub fn set_interaction_pause_timer_ms_runtime_like_cpp(
        &mut self,
        interaction_pause_timer_ms: u32,
    ) {
        self.lifecycle_metadata.interaction_pause_timer_ms = interaction_pause_timer_ms;
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

    pub fn set_default_movement_type_runtime_like_cpp(
        &mut self,
        movement_type: MovementGeneratorType,
    ) {
        self.default_movement_type = movement_type;
        self.sync_motion_default_generator_like_cpp();
    }

    fn sync_motion_default_generator_like_cpp(&mut self) {
        let kind = match self.default_movement_type {
            MovementGeneratorType::Idle => MovementGeneratorKind::Idle,
            MovementGeneratorType::Random => MovementGeneratorKind::Random,
            MovementGeneratorType::Waypoint => MovementGeneratorKind::Waypoint,
        };
        self.unit
            .subsystems_mut()
            .motion
            .initialize_default_generator_like_cpp(kind);
    }

    pub const fn waypoint_path_id_like_cpp(&self) -> u32 {
        self.waypoint_path_id
    }

    pub fn load_path_like_cpp(&mut self, path_id: u32) {
        self.waypoint_path_id = path_id;
    }

    pub fn apply_creatures_addon_lifecycle_like_cpp(
        &mut self,
        addon: Option<&CreatureAddonLifecycleRecordLikeCpp>,
    ) -> bool {
        self.load_creatures_addon_represented_like_cpp(addon)
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

    /// Represented bounded seam for TrinityCore `Creature::InitEntry`.
    ///
    /// C++ stores `CreatureData::equipmentId` in `m_originalEquipmentId`
    /// before `LoadEquipment(id)` mutates the selected equipment id, notably
    /// when DB value `-1` means "pick a random equipment template".
    pub fn set_original_equipment_id_like_cpp(&mut self, original_equipment_id: i8) {
        self.original_equipment_id = original_equipment_id;
        self.lifecycle_metadata.original_equipment_id = original_equipment_id;
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

    pub fn melee_damage_school_like_cpp(&self) -> u8 {
        if self.melee_damage_school_mask == 0 {
            return wow_constants::spell::SpellSchools::Normal as u8;
        }
        self.melee_damage_school_mask
            .trailing_zeros()
            .min(u32::from(MAX_SPELL_SCHOOL_LIKE_CPP.saturating_sub(1))) as u8
    }

    pub fn set_melee_damage_school_like_cpp(&mut self, school: u8) {
        let school = school.min(MAX_SPELL_SCHOOL_LIKE_CPP.saturating_sub(1));
        self.melee_damage_school_mask = 1_u32 << school;
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

    pub const fn sparring_health_pct(&self) -> f32 {
        self.sparring_health_pct
    }

    pub fn set_sparring_health_pct_like_cpp(&mut self, pct: f32) {
        self.sparring_health_pct = pct.clamp(0.0, 100.0);
    }

    pub fn is_charmed_owned_by_player_or_player_like_cpp(&self) -> bool {
        let control = &self.unit.subsystems().control;
        control
            .owner_guid
            .is_some_and(|owner_guid| owner_guid.is_player())
            || control.charmer_guid.is_some_and(|charmer_guid| {
                charmer_guid.is_player() || control.controlled_by_player
            })
    }

    /// C++ contrast: `Creature::CalculateDamageForSparring(Unit*, uint32)`.
    ///
    /// This helper keeps the current represented scope explicit: the caller
    /// tells us whether the attacker is a creature and whether the attacker is
    /// player-controlled. The victim-side player-control check is local.
    pub fn calculate_damage_for_sparring_like_cpp(
        &self,
        attacker_is_creature: bool,
        attacker_is_charmed_owned_by_player_or_player: bool,
        damage: u32,
    ) -> u32 {
        if self.sparring_health_pct <= 0.0
            || !attacker_is_creature
            || attacker_is_charmed_owned_by_player_or_player
            || self.is_charmed_owned_by_player_or_player_like_cpp()
        {
            return damage;
        }

        let health = self.unit.data().health;
        let max_health = self.unit.data().max_health;
        if max_health == 0
            || (health as f32 * 100.0 / max_health as f32) <= self.sparring_health_pct
        {
            return 0;
        }

        let sparring_health = (max_health as f32 * self.sparring_health_pct / 100.0) as u64;
        if health.saturating_sub(u64::from(damage)) <= sparring_health {
            return health
                .saturating_sub(sparring_health)
                .min(u64::from(u32::MAX)) as u32;
        }

        if u64::from(damage) >= health {
            return health.saturating_sub(1).min(u64::from(u32::MAX)) as u32;
        }

        damage
    }

    /// C++ contrast: `Creature::ShouldFakeDamageFrom(Unit*)`.
    pub fn should_fake_damage_from_like_cpp(
        &self,
        attacker_is_creature: bool,
        attacker_is_charmed_owned_by_player_or_player: bool,
    ) -> bool {
        if self.sparring_health_pct <= 0.0
            || !attacker_is_creature
            || attacker_is_charmed_owned_by_player_or_player
            || self.is_charmed_owned_by_player_or_player_like_cpp()
        {
            return false;
        }

        let max_health = self.unit.data().max_health;
        max_health != 0
            && (self.unit.data().health as f32 * 100.0 / max_health as f32)
                <= self.sparring_health_pct
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

    pub const fn combat_log_stats_like_cpp(&self) -> CreatureCombatLogStatsLikeCpp {
        self.combat_log_stats
    }

    /// Replace the live totals consumed by C++ `SpellCastLogData::Initialize`.
    ///
    /// Loaders and future represented UnitMods mutations must update this at
    /// the same boundary as the corresponding canonical Unit state.
    pub fn set_combat_log_stats_like_cpp(&mut self, stats: CreatureCombatLogStatsLikeCpp) {
        self.combat_log_stats = stats;
    }

    /// C++ `SpellCastLogData::Initialize` selects ranged AP only for the
    /// hunter unit class and base-attack AP for every other class.
    pub fn combat_log_attack_power_like_cpp(&self) -> i32 {
        if self.unit.data().class_id == Class::Hunter as u8 {
            self.combat_log_stats.ranged_attack_power
        } else {
            self.combat_log_stats.attack_power
        }
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

    pub const fn loot_lifecycle_revision_like_cpp(&self) -> u64 {
        self.loot_lifecycle_revision
    }

    fn advance_loot_lifecycle_revision_like_cpp(&mut self) -> u64 {
        // Never wrap to an earlier lifetime: even an unreachable counter
        // exhaustion must remain fail-closed for async generation tokens.
        self.loot_lifecycle_revision = self.loot_lifecycle_revision.saturating_add(1).max(1);
        self.loot_lifecycle_revision
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

    pub const fn loot_authority_like_cpp(&self) -> &OwnedLootAuthority {
        &self.loot_authority
    }

    /// Bind this runtime mirror to the same C++ `Creature::loot` authority as
    /// another coexisting map model.
    pub fn rebind_loot_authority_like_cpp(&mut self, authority: OwnedLootAuthority) -> bool {
        if self.loot_authority.shares_storage_like_cpp(&authority) {
            self.sync_loot_summaries_from_authority_like_cpp();
            return false;
        }

        // Invalidate leases retained against a displaced mirror before the
        // replacement authority can serve another claim for the same C++
        // object.
        self.loot_authority.detach_like_cpp();
        self.loot_authority = authority;
        self.sync_loot_summaries_from_authority_like_cpp();
        true
    }

    /// Rebinds only if this mirror still owns the authority observed by the
    /// caller. This is the object-local compare/exchange boundary used when
    /// the legacy and canonical map models are reconciled without holding
    /// both map locks at once.
    pub fn rebind_loot_authority_if_current_like_cpp(
        &mut self,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        if !self.loot_authority.shares_storage_like_cpp(expected) {
            return None;
        }

        if self.loot_authority.stamp_like_cpp() != expected_stamp {
            return None;
        }

        if self.loot_authority.shares_storage_like_cpp(&authority) {
            self.sync_loot_summaries_from_authority_like_cpp();
            return Some(false);
        }

        if !self.loot_authority.detach_if_stamp_like_cpp(expected_stamp) {
            return None;
        }

        self.loot_authority = authority;
        self.sync_loot_summaries_from_authority_like_cpp();
        Some(true)
    }

    /// Assigns authority identity to a temporary whole-entity snapshot.
    ///
    /// The snapshot may share its old `Arc` with a live legacy entity, so it
    /// must not detach that allocation. Only the later CAS against the actual
    /// owning entity is allowed to invalidate a displaced authority.
    pub fn adopt_loot_authority_for_snapshot_like_cpp(&mut self, authority: OwnedLootAuthority) {
        self.loot_authority = authority;
        self.sync_loot_summaries_from_authority_like_cpp();
    }

    pub fn share_loot_authority_like_cpp(&mut self, authority: OwnedLootAuthority) {
        self.rebind_loot_authority_like_cpp(authority);
    }

    pub fn initialize_loot_authority_like_cpp(
        &mut self,
        shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> LootInstallOutcome {
        let outcome = self.loot_authority.initialize_like_cpp(shared, personal);
        self.sync_loot_summaries_from_authority_like_cpp();
        outcome
    }

    pub fn initialize_shared_loot_authority_like_cpp(
        &mut self,
        loot: CreatureLoot,
    ) -> LootInstallOutcome {
        let outcome = self.loot_authority.initialize_shared_like_cpp(loot);
        self.sync_loot_summaries_from_authority_like_cpp();
        outcome
    }

    pub fn upsert_personal_loot_authority_like_cpp(
        &mut self,
        player: ObjectGuid,
        loot: CreatureLoot,
        replace: bool,
    ) -> LootInstallOutcome {
        let outcome = self
            .loot_authority
            .upsert_personal_like_cpp(player, loot, replace);
        self.sync_loot_summaries_from_authority_like_cpp();
        outcome
    }

    pub fn replace_loot_authority_like_cpp(
        &mut self,
        shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> u64 {
        let generation = self.loot_authority.replace_like_cpp(shared, personal);
        self.sync_loot_summaries_from_authority_like_cpp();
        generation
    }

    pub fn sync_loot_summaries_from_authority_like_cpp(&mut self) {
        self.shared_loot = self
            .loot_authority
            .shared_snapshot_like_cpp()
            .map(|snapshot| creature_owned_loot_from_snapshot(&snapshot));
        self.personal_loot = self
            .loot_authority
            .personal_snapshots_like_cpp()
            .into_iter()
            .map(|(player, snapshot)| (player, creature_owned_loot_from_snapshot(&snapshot)))
            .collect();
    }

    pub const fn shared_loot_like_cpp(&self) -> Option<&CreatureOwnedLoot> {
        self.shared_loot.as_ref()
    }

    pub fn set_shared_loot_like_cpp(&mut self, loot: CreatureOwnedLoot) {
        self.shared_loot = Some(loot);
    }

    pub fn clear_shared_loot_like_cpp(&mut self) {
        self.shared_loot = None;
    }

    pub fn personal_loot_like_cpp(&self, guid: ObjectGuid) -> Option<&CreatureOwnedLoot> {
        self.personal_loot.get(&guid)
    }

    pub fn loot_for_player_like_cpp(&self, guid: ObjectGuid) -> Option<&CreatureOwnedLoot> {
        if self.personal_loot.is_empty() {
            return self.shared_loot.as_ref();
        }

        self.personal_loot.get(&guid)
    }

    pub fn set_personal_loot_like_cpp(&mut self, guid: ObjectGuid, loot: CreatureOwnedLoot) {
        self.personal_loot.insert(guid, loot);
    }

    pub fn clear_personal_loot_like_cpp(&mut self) {
        self.personal_loot.clear();
    }

    pub fn personal_loot_count_like_cpp(&self) -> usize {
        self.personal_loot.len()
    }

    pub fn clear_loot_like_cpp(&mut self) {
        self.advance_loot_lifecycle_revision_like_cpp();
        self.loot_authority.retire_like_cpp();
        self.shared_loot = None;
        self.personal_loot.clear();
    }

    pub fn is_fully_looted_like_cpp(&self) -> bool {
        if !self.loot_authority.is_pristine_like_cpp() {
            return self.loot_authority.is_fully_looted_like_cpp();
        }

        if self
            .shared_loot
            .as_ref()
            .is_some_and(|loot| !loot.is_looted_like_cpp())
        {
            return false;
        }

        for loot in self.personal_loot.values() {
            if !loot.is_looted_like_cpp() {
                return false;
            }
        }

        true
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
        self.set_death_state_runtime_with_fall_like_cpp(state, now, None)
    }

    pub fn set_death_state_runtime_with_fall_like_cpp(
        &mut self,
        state: DeathState,
        now: i64,
        death_fall: Option<CreatureDeathFallContextLikeCpp>,
    ) -> CreatureRuntimePlan {
        let mut plan = CreatureRuntimePlan::new();
        self.unit.set_death_state(state);

        match state {
            DeathState::JustDied => {
                if self.ai_ownership.state != CreatureAiState::Dead {
                    self.advance_loot_lifecycle_revision_like_cpp();
                }
                let needs_falling = death_fall.filter(|context| {
                    (self.is_flying_like_cpp() || self.is_hovering_like_cpp())
                        && !context.is_underwater
                });
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
                self.release_spell_focus_like_cpp(None, false, false, false);
                self.do_not_reacquire_spell_focus_target_like_cpp();
                self.unit.set_target(ObjectGuid::EMPTY);
                self.unit.set_attacking(None);
                self.unit.subsystems_mut().combat.end_all_combat();
                self.unit.subsystems_mut().combat.clear_attackers();
                if self
                    .unit
                    .is_non_melee_spell_cast_like_cpp(false, false, false, true)
                {
                    self.unit.interrupt_non_melee_spells(None, false, true);
                }
                {
                    let subsystems = self.unit.subsystems_mut();
                    subsystems.vehicle.exit_vehicle();
                    for summon_slot in &mut subsystems.control.summon_slots {
                        *summon_slot = ObjectGuid::EMPTY;
                    }
                    subsystems.control.remove_all_controlled();
                }
                if !self.unit.world().object().guid().is_pet() {
                    let mut flags = self.unit.unit_flags_like_cpp();
                    flags.remove(UnitFlags::PET_IN_COMBAT);
                    self.unit.set_unit_flags_like_cpp(flags);
                }
                self.unit
                    .subsystems_mut()
                    .auras
                    .remove_all_auras_on_death_like_cpp();
                self.unit
                    .subsystems_mut()
                    .auras
                    .clear_all_reactives_like_cpp();
                self.unit.subsystems_mut().auras.clear_diminishings();
                let stop_on_death = if self.unit.subsystems().vehicle.vehicle_guid.is_some() {
                    false
                } else {
                    self.unit.subsystems_mut().motion.stop_on_death()
                };
                if stop_on_death {
                    self.unit.clear_unit_state(UnitState::MOVING.bits());
                }
                self.unit.set_health(0);
                self.unit.set_power(self.power_type(), 0);
                self.unit.set_emote_state_like_cpp(0);
                self.unit
                    .set_stand_state_like_cpp(UnitStandStateType::Stand);
                self.unit.set_npc_flags_like_cpp(0);
                self.unit.set_npc_flags2_like_cpp(0);
                self.unit.set_mount_display_id(0);
                self.unit.world_mut().set_active(false);
                self.already_searched_assistance = false;
                self.already_call_assistance = false;
                self.runtime_state
                    .movement_flags
                    .remove(MovementFlag::HOVER | MovementFlag::DISABLE_GRAVITY);
                plan.extend([
                    CreatureRuntimeAction::SaveRespawnTime,
                    CreatureRuntimeAction::ReleaseSpellFocus,
                    CreatureRuntimeAction::CancelSpellFocusReacquire,
                    CreatureRuntimeAction::ClearTarget,
                    CreatureRuntimeAction::ClearNpcFlags,
                    CreatureRuntimeAction::ClearMount,
                    CreatureRuntimeAction::Deactivate,
                    CreatureRuntimeAction::ClearAssistanceSearch,
                ]);
                if let Some(context) = needs_falling {
                    let has_root_or_stun_state = self
                        .unit
                        .has_unit_state((UnitState::ROOT | UnitState::STUNNED).bits());
                    let fall_plan = self.unit.subsystems_mut().motion.move_fall_like_cpp(
                        context.movement_id,
                        context.duration_ms,
                        context.has_valid_ground_height,
                        context.vertical_delta,
                        has_root_or_stun_state,
                        false,
                    );
                    if matches!(fall_plan, MoveFallPlan::SplineStarted) {
                        plan.push(CreatureRuntimeAction::MoveFall);
                    }
                }
                self.unit.set_death_state(DeathState::Corpse);
            }
            DeathState::JustRespawned => {
                self.advance_loot_lifecycle_revision_like_cpp();
                let motion_initialize_outcome = self.aim_initialize_like_cpp();
                let is_pet = self.unit.world().object().guid().is_pet();
                if is_pet {
                    self.unit.set_health(self.unit.data().max_health);
                } else {
                    self.set_spawn_health_like_cpp();
                    self.unit
                        .world_mut()
                        .object_mut()
                        .replace_all_dynamic_flags(0);
                    self.unit
                        .set_npc_flags_like_cpp(self.ai_ownership.npc_flags);
                    self.unit
                        .set_npc_flags2_like_cpp(self.ai_ownership.npc_flags2);
                    let mut flags = UnitFlags::from_bits_truncate(self.ai_ownership.unit_flags);
                    flags.remove(UnitFlags::SKINNABLE | UnitFlags::IN_COMBAT);
                    self.unit.set_unit_flags_like_cpp(flags);
                    self.unit
                        .set_unit_flags2_like_cpp(UnitFlags2::from_bits_truncate(
                            self.ai_ownership.unit_flags2,
                        ));
                    self.unit
                        .set_unit_flags3_like_cpp(UnitFlags3::from_bits_truncate(
                            self.ai_ownership.unit_flags3,
                        ));
                    self.set_melee_damage_school_like_cpp(self.lifecycle_metadata.damage_school);
                }
                self.unit.clear_unit_state(UnitState::ALL_ERASABLE.bits());
                self.clear_tap_list();
                self.player_damage_req = 0;
                self.cannot_reach_target = false;
                self.cannot_reach_timer = 0;
                self.respawn_time = 0;
                self.corpse_remove_time = 0;
                self.reset_pickpocket_loot_restore();
                self.reset_loot_mode();
                let addon = self.lifecycle_metadata.addon.clone();
                self.load_creatures_addon_represented_like_cpp(addon.as_ref());
                self.trigger_just_appeared = true;
                self.runtime_state.ai_reset_requested = true;
                self.runtime_state.visibility_update_requested = true;
                if motion_initialize_outcome.motion_master_initialize_represented {
                    self.unit
                        .subsystems_mut()
                        .motion
                        .direct_initialize_like_cpp();
                }
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

    fn load_creatures_addon_represented_like_cpp(
        &mut self,
        addon: Option<&CreatureAddonLifecycleRecordLikeCpp>,
    ) -> bool {
        let Some(addon) = addon else {
            return false;
        };

        if addon.mount_display_id != 0 {
            self.unit.set_mount_display_id(addon.mount_display_id);
        }
        self.unit.set_stand_state_like_cpp(addon.stand_state);
        self.unit.replace_all_vis_flags_like_cpp(addon.vis_flags);
        self.unit.set_anim_tier_like_cpp(addon.anim_tier);
        if self.can_hover_like_cpp() {
            self.runtime_state
                .movement_flags
                .insert(MovementFlag::HOVER);
        }
        self.unit.set_sheath_like_cpp(addon.sheath_state);
        self.unit.replace_all_pvp_flags_like_cpp(addon.pvp_flags);
        self.unit.replace_all_pet_flags_like_cpp(0);
        self.unit.set_shapeshift_form_like_cpp(ShapeShiftForm::None);
        if addon.emote != 0 {
            self.unit.set_emote_state_like_cpp(addon.emote);
        }
        if addon.path_id != 0 {
            self.waypoint_path_id = addon.path_id;
        }
        self.unit.set_ai_anim_kit_id_like_cpp(addon.ai_anim_kit_id);
        self.unit
            .set_movement_anim_kit_id_like_cpp(addon.movement_anim_kit_id);
        self.unit
            .set_melee_anim_kit_id_like_cpp(addon.melee_anim_kit_id);
        if addon.visibility_distance_type != VisibilityDistanceTypeLikeCpp::Normal {
            self.unit
                .world_mut()
                .set_visibility_distance_override_like_cpp(addon.visibility_distance_type);
            let mut flags2 = self.unit.unit_flags2_like_cpp();
            flags2.remove(
                UnitFlags2::LARGE_AOI | UnitFlags2::GIGANTIC_AOI | UnitFlags2::INFINITE_AOI,
            );
            match addon.visibility_distance_type {
                VisibilityDistanceTypeLikeCpp::Large => flags2.insert(UnitFlags2::LARGE_AOI),
                VisibilityDistanceTypeLikeCpp::Gigantic => flags2.insert(UnitFlags2::GIGANTIC_AOI),
                VisibilityDistanceTypeLikeCpp::Infinite => flags2.insert(UnitFlags2::INFINITE_AOI),
                _ => {}
            }
            self.unit.set_unit_flags2_like_cpp(flags2);
        }
        let self_guid = self.unit.world().object().guid();
        if addon.aura_applications.is_empty() {
            for spell_id in &addon.auras {
                self.unit
                    .subsystems_mut()
                    .auras
                    .add_self_cast_addon_aura_like_cpp(*spell_id, self_guid);
            }
        } else {
            for aura in &addon.aura_applications {
                self.unit
                    .subsystems_mut()
                    .auras
                    .add_self_cast_addon_aura_application_like_cpp(
                        aura.spell_id,
                        self_guid,
                        aura.effect_mask,
                        aura.flags,
                    );
            }
        }
        true
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

        // C++ drops the corpse's object-owned Loot during removal (directly
        // in compatibility mode and via object destruction otherwise). Rust
        // leases can outlive the map reference, so retire it at the lifecycle
        // boundary before an in-flight async claim can commit.
        self.clear_loot_like_cpp();
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
#[path = "creature_tests.rs"]
mod tests;
