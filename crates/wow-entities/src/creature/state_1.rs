//! Creature lifecycle and runtime state state definitions, part 1 of 2.
//!
//! Separated from the creature.rs root under #636. Behaviour is preserved.

use super::*;

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

pub(super) const CREATURE_GROUND_MOVEMENT_TYPE_MAX_LIKE_CPP: u8 = 3;

pub(super) const CREATURE_FLIGHT_MOVEMENT_TYPE_MAX_LIKE_CPP: u8 = 3;

pub(super) const CREATURE_CHASE_MOVEMENT_TYPE_MAX_LIKE_CPP: u8 = 3;

pub(super) const CREATURE_RANDOM_MOVEMENT_TYPE_MAX_LIKE_CPP: u8 = 3;

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
    pub(super) steps: Vec<CreatureLifecycleStep>,
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
    pub(super) actions: Vec<CreatureRuntimeAction>,
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
    pub(super) gold: u32,
    pub(super) unlooted_count: u32,
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

pub(super) fn creature_owned_loot_from_snapshot(snapshot: &OwnedLootSnapshot) -> CreatureOwnedLoot {
    CreatureOwnedLoot::new(snapshot.loot.coins, u32::from(snapshot.loot.unlooted_count))
}
