//! Unit values, visibility and health-revision state state definitions, part 1 of 1.
//!
//! Separated from the unit.rs root under #636. Behaviour is preserved.

use super::*;

pub const MAX_MOVE_TYPE: usize = 9;

pub const MAX_ATTACK: usize = 3;

pub const MAX_POWERS: usize = 26;

pub const MAX_POWERS_PER_CLASS: usize = 10;

pub const BASE_MINDAMAGE: f32 = 1.0;

pub const BASE_MAXDAMAGE: f32 = 2.0;

pub const DEFAULT_PLAYER_DISPLAY_SCALE: f32 = 1.0;

pub const AUTO_SHOT_SPELL_ID: u32 = 75;

pub const SPELL_AURA_MOD_UNATTACKABLE_LIKE_CPP: i32 = 93;

pub const SPELL_AURA_DISABLE_ATTACKING_EXCEPT_ABILITIES_LIKE_CPP: i32 = 264;

pub const SPELL_AURA_MOD_STALKED_LIKE_CPP: i32 = 68;

pub const SPELL_AURA_MOD_DETECT_RANGE_LIKE_CPP: i32 = 91;

pub const SPELL_AURA_MOD_DETECTED_RANGE_LIKE_CPP: i32 = 152;

pub const SPELL_AURA_INTERRUPT_FLAG_ATTACKING_LIKE_CPP: u32 = 0x0000_1000;

pub const SPELL_AURA_INTERRUPT_FLAG_STANDING_LIKE_CPP: u32 = 0x0004_0000;

pub const SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP: u32 = 0x0040_0000;

pub const MAX_VISIBILITY_AURA_TYPES_LIKE_CPP: usize = 38;

pub const MAX_PLAYER_STEALTH_DETECT_RANGE_LIKE_CPP: f32 = 30.0;

pub const GHOST_VISIBILITY_ALIVE_LIKE_CPP: u32 = 0x1;

pub const UNIT_DATA_PARENT_BIT: usize = 0;

pub const UNIT_DATA_HEALTH_BIT: usize = 5;

pub const UNIT_DATA_MAX_HEALTH_BIT: usize = 6;

pub const UNIT_DATA_DISPLAY_ID_BIT: usize = 7;

pub const UNIT_DATA_DISPLAY_POWER_BIT: usize = 28;

pub const UNIT_DATA_LEVEL_BIT: usize = 30;

pub const UNIT_DATA_FACTION_TEMPLATE_BIT: usize = 40;

pub const UNIT_DATA_FLAGS_BIT: usize = 41;

pub const UNIT_DATA_FLAGS2_BIT: usize = 42;

pub const UNIT_DATA_FLAGS3_BIT: usize = 43;

pub const UNIT_DATA_BOUNDING_RADIUS_BIT: usize = 46;

pub const UNIT_DATA_COMBAT_REACH_BIT: usize = 47;

pub const UNIT_DATA_DISPLAY_SCALE_BIT: usize = 48;

pub const UNIT_DATA_NATIVE_DISPLAY_ID_BIT: usize = 49;

pub const UNIT_DATA_NATIVE_DISPLAY_SCALE_BIT: usize = 50;

pub const UNIT_DATA_MOUNT_DISPLAY_ID_BIT: usize = 51;

pub const UNIT_DATA_STAND_STATE_PARENT_BIT: usize = 32;

pub const UNIT_DATA_STAND_STATE_BIT: usize = 56;

pub const UNIT_DATA_VIS_FLAGS_BIT: usize = 58;

pub const UNIT_DATA_ANIM_TIER_BIT: usize = 59;

pub const UNIT_DATA_SHEATHE_STATE_BIT: usize = 77;

pub const UNIT_DATA_PVP_FLAGS_BIT: usize = 78;

pub const UNIT_DATA_PET_FLAGS_BIT: usize = 79;

pub const UNIT_DATA_SHAPESHIFT_FORM_BIT: usize = 80;

pub const UNIT_DATA_CRITTER_BIT: usize = 13;

pub const UNIT_DATA_BATTLE_PET_COMPANION_GUID_BIT: usize = 20;

pub const UNIT_DATA_BATTLE_PET_COMPANION_NAME_TIMESTAMP_BIT: usize = 100;

pub const UNIT_DATA_TARGET_BIT: usize = 19;

pub const UNIT_DATA_RACE_BIT: usize = 24;

pub const UNIT_DATA_CLASS_ID_BIT: usize = 25;

pub const UNIT_DATA_PLAYER_CLASS_ID_BIT: usize = 26;

pub const UNIT_DATA_SEX_BIT: usize = 27;

pub const UNIT_DATA_NPC_FLAGS_PARENT_BIT: usize = 113;

pub const UNIT_DATA_NPC_FLAGS_FIRST_BIT: usize = 114;

pub const UNIT_DATA_MOD_CASTING_SPEED_BIT: usize = 65;

pub const UNIT_DATA_MOD_SPELL_HASTE_BIT: usize = 66;

pub const UNIT_DATA_MOD_HASTE_BIT: usize = 67;

pub const UNIT_DATA_MOD_RANGED_HASTE_BIT: usize = 68;

pub const UNIT_DATA_MOD_HASTE_REGEN_BIT: usize = 69;

pub const UNIT_DATA_MOD_TIME_RATE_BIT: usize = 70;

pub const UNIT_DATA_MODS_PARENT_BIT: usize = 64;

pub const UNIT_DATA_EMOTE_STATE_BIT: usize = 72;

pub const UNIT_DATA_HOVER_HEIGHT_BIT: usize = 94;

pub const UNIT_DATA_POWER_PARENT_BIT: usize = 116;

pub const UNIT_DATA_POWER_FIRST_BIT: usize = 137;

pub const UNIT_DATA_MAX_POWER_FIRST_BIT: usize = 147;

pub const UNIT_DATA_BASE_MANA_BIT: usize = 75;

pub const UNIT_DATA_VIRTUAL_ITEMS_PARENT_BIT: usize = 167;

pub const UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT: usize = 168;

pub const UNIT_DATA_WILD_BATTLE_PET_LEVEL_BIT: usize = 99;

pub const BASE_MOVE_SPEED: [f32; MAX_MOVE_TYPE] =
    [2.5, 7.0, 4.5, 4.722222, 2.5, 3.141594, 7.0, 4.5, 3.14];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitDataValues {
    pub health: u64,
    pub max_health: u64,
    pub display_id: i32,
    pub critter: ObjectGuid,
    pub battle_pet_companion_guid: ObjectGuid,
    pub battle_pet_companion_name_timestamp: u32,
    pub target: ObjectGuid,
    pub race: u8,
    pub class_id: u8,
    pub player_class_id: u8,
    pub sex: u8,
    pub display_power: u8,
    pub level: i32,
    pub faction_template: i32,
    pub flags: u32,
    pub flags2: u32,
    pub flags3: u32,
    pub bounding_radius: f32,
    pub combat_reach: f32,
    pub display_scale: f32,
    pub native_display_id: i32,
    pub native_display_scale: f32,
    pub mount_display_id: i32,
    pub stand_state: u8,
    pub vis_flags: u8,
    pub anim_tier: u8,
    pub sheathe_state: u8,
    pub pvp_flags: u8,
    pub pet_flags: u8,
    pub shapeshift_form: u8,
    pub mod_casting_speed: f32,
    pub mod_spell_haste: f32,
    pub mod_haste: f32,
    pub mod_ranged_haste: f32,
    pub mod_haste_regen: f32,
    pub mod_time_rate: f32,
    pub emote_state: i32,
    pub hover_height: f32,
    pub wild_battle_pet_level: i32,
    pub npc_flags: [u32; 2],
    pub base_mana: i32,
    pub power: [i32; MAX_POWERS_PER_CLASS],
    pub max_power: [i32; MAX_POWERS_PER_CLASS],
    pub virtual_items: [VisibleItemValues; MAX_ATTACK],
}

impl Default for UnitDataValues {
    fn default() -> Self {
        Self {
            health: 0,
            max_health: 0,
            display_id: 0,
            critter: ObjectGuid::EMPTY,
            battle_pet_companion_guid: ObjectGuid::EMPTY,
            battle_pet_companion_name_timestamp: 0,
            target: ObjectGuid::EMPTY,
            race: 0,
            class_id: 0,
            player_class_id: 0,
            sex: Gender::Male as u8,
            display_power: PowerType::Mana as u8,
            level: 0,
            faction_template: 0,
            flags: 0,
            flags2: 0,
            flags3: 0,
            bounding_radius: 0.0,
            combat_reach: 0.0,
            display_scale: 0.0,
            native_display_id: 0,
            native_display_scale: 0.0,
            mount_display_id: 0,
            stand_state: UnitStandStateType::Stand as u8,
            vis_flags: 0,
            anim_tier: 0,
            sheathe_state: SheathState::Unarmed as u8,
            pvp_flags: 0,
            pet_flags: 0,
            shapeshift_form: ShapeShiftForm::None as u8,
            mod_casting_speed: 1.0,
            mod_spell_haste: 1.0,
            mod_haste: 1.0,
            mod_ranged_haste: 1.0,
            mod_haste_regen: 1.0,
            mod_time_rate: 1.0,
            emote_state: 0,
            hover_height: 1.0,
            wild_battle_pet_level: 0,
            npc_flags: [0; 2],
            base_mana: 0,
            power: [0; MAX_POWERS_PER_CLASS],
            max_power: [0; MAX_POWERS_PER_CLASS],
            virtual_items: [VisibleItemValues::default(); MAX_ATTACK],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnitDataUpdate {
    pub mask: UpdateMask,
    pub values: UnitDataValues,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnitValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataUpdate>,
    pub unit_data: Option<UnitDataUpdate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitRemoveFromWorldOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub was_in_world: bool,
    pub during_remove_entered: bool,
    pub ai_on_despawn_represented: bool,
    pub vehicle_remove: Option<VehicleKitRemoveOutcomeLikeCpp>,
    pub leave_world_cleanup_represented: bool,
    pub world_object_removed: bool,
    pub during_remove_cleared: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitAddToWorldOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub world_object_added: bool,
    pub is_in_world_after: bool,
    pub motion_master_add_to_world: MotionMasterAddToWorldOutcomeLikeCpp,
    pub removed_enter_world_auras: Vec<crate::AppliedAuraRef>,
    pub aura_interrupt_flags_enter_world: u32,
}

impl UnitValuesUpdate {
    pub const fn has_data(&self) -> bool {
        self.changed_object_type_mask != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitAttackStartOutcome {
    NewTarget { previous: Option<ObjectGuid> },
    MeleeStartedSameTarget,
    MeleeStoppedSameTarget,
    NoChangeSameTarget,
    InvalidSelfTarget,
    InvalidDeadAttacker,
    InvalidDeadVictim,
    InvalidVictimNotInWorld,
    InvalidMountedAttacker,
    InvalidAttackerEvading,
    InvalidVictimGameMaster,
    InvalidVictimEvading,
    InvalidAttackTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitAttackStopOutcome {
    Stopped { victim: ObjectGuid },
    NoVictim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitSharedVisionSetWorldObjectRequestLikeCpp {
    pub unit_guid: ObjectGuid,
    pub on: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitSharedVisionUpdateOutcomeLikeCpp {
    pub player_guid: ObjectGuid,
    pub inserted_or_removed: bool,
    pub set_world_object: Option<UnitSharedVisionSetWorldObjectRequestLikeCpp>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnitAttackContextLikeCpp {
    pub attacker_is_mounted_player: bool,
    pub attacker_is_evading_creature: bool,
    pub victim_is_game_master_player: bool,
    pub victim_is_evading_creature: bool,
    pub controlled_creatures_with_ai: Vec<ObjectGuid>,
    pub visibility_represented: bool,
    pub attacker_can_see_or_detect_target: bool,
    pub victim_unit_state: u32,
    pub attacker_unit_flags: u32,
    pub victim_unit_flags: u32,
    pub attacker_is_player_uber: bool,
    pub relation_represented: bool,
    pub attacker_is_hostile_to_victim: bool,
    pub victim_is_hostile_to_attacker: bool,
    pub attacker_is_friendly_to_victim: bool,
    pub victim_is_friendly_to_attacker: bool,
    pub attacker_has_affecting_player: bool,
    pub victim_has_affecting_player: bool,
    pub victim_is_pet: bool,
    pub victim_affecting_player_is_mounted: bool,
    pub player_creature_reputation_represented: bool,
    pub creature_is_contested_guard: bool,
    pub player_has_contested_pvp_flag: bool,
    pub creature_has_forced_reputation_rank: bool,
    pub player_at_war_with_creature_faction: bool,
    pub player_player_duel_in_progress: bool,
    pub sanctuary_represented: bool,
    pub attacker_in_sanctuary: bool,
    pub victim_in_sanctuary: bool,
    pub pvp_represented: bool,
    pub victim_is_pvp: bool,
    pub attacker_is_ffa_pvp: bool,
    pub victim_is_ffa_pvp: bool,
    pub attacker_has_pvp_unk1_flag: bool,
    pub victim_has_pvp_unk1_flag: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitVisibilityDetectionStateLikeCpp {
    pub(super) never_visible_for_seer: bool,
    pub(super) seer_can_never_see_target: bool,
    pub(super) always_visible_for_seer: bool,
    pub(super) seer_can_always_see_target: bool,
    pub(super) target_owner_group_visible_for_seer: bool,
    pub(super) always_detectable_for_seer: bool,
    pub(super) invisible_due_to_despawn: bool,
    pub(super) private_object_owner: ObjectGuid,
    pub(super) seer_private_object_owner: ObjectGuid,
    pub(super) seer_group_visible_for_private_owner: bool,
    pub(super) object_id_visibility_conditions_met: bool,
    pub(super) server_side_visibility_gm: u32,
    pub(super) server_side_visibility_detect_gm: u32,
    pub(super) server_side_visibility_ghost: u32,
    pub(super) server_side_visibility_detect_ghost: u32,
    pub(super) ghost_visible_to_seer_by_group: bool,
    pub(super) seer_can_always_see_target_guid: ObjectGuid,
    pub(super) invisibility_flags: u64,
    pub(super) invisibility: [i32; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
    pub(super) invisibility_detect_flags: u64,
    pub(super) invisibility_detect: [i32; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
    pub(super) stealth_flags: u64,
    pub(super) stealth: [i32; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
    pub(super) stealth_detect: [i32; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
}

impl Default for UnitVisibilityDetectionStateLikeCpp {
    fn default() -> Self {
        Self {
            never_visible_for_seer: false,
            seer_can_never_see_target: false,
            always_visible_for_seer: false,
            seer_can_always_see_target: false,
            target_owner_group_visible_for_seer: false,
            always_detectable_for_seer: false,
            invisible_due_to_despawn: false,
            private_object_owner: ObjectGuid::EMPTY,
            seer_private_object_owner: ObjectGuid::EMPTY,
            seer_group_visible_for_private_owner: false,
            object_id_visibility_conditions_met: true,
            server_side_visibility_gm: 0,
            server_side_visibility_detect_gm: 0,
            server_side_visibility_ghost: GHOST_VISIBILITY_ALIVE_LIKE_CPP,
            server_side_visibility_detect_ghost: GHOST_VISIBILITY_ALIVE_LIKE_CPP,
            ghost_visible_to_seer_by_group: false,
            seer_can_always_see_target_guid: ObjectGuid::EMPTY,
            invisibility_flags: 0,
            invisibility: [0; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
            invisibility_detect_flags: 0,
            invisibility_detect: [0; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
            stealth_flags: 0,
            stealth: [0; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
            stealth_detect: [0; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
        }
    }
}

/// Shared sequence identity for the health/max-health/death timeline of one entity
/// incarnation. Holding this token keeps the allocation alive, so its identity
/// cannot be reused while a compare/exchange is in flight.
#[derive(Debug, Clone)]
pub struct HealthStateRevisionAuthorityLikeCpp {
    pub(super) allocator: Arc<AtomicU64>,
}

impl HealthStateRevisionAuthorityLikeCpp {
    pub(super) fn new() -> Self {
        Self {
            allocator: Arc::new(AtomicU64::new(0)),
        }
    }

    pub(super) fn next(&self) -> u64 {
        let previous = self
            .allocator
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current.checked_add(1)
            })
            .expect("health-state revision allocator exhausted");
        previous
            .checked_add(1)
            .expect("health-state revision allocator exhausted")
    }

    pub fn shares_storage_like_cpp(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.allocator, &other.allocator)
    }
}

#[derive(Debug, Clone)]
pub(super) struct HealthStateRevisionLikeCpp {
    pub(super) value: u64,
    /// Clones retain a local state version while allocating future revisions
    /// from one sequence shared by the entity incarnation.
    pub(super) authority: HealthStateRevisionAuthorityLikeCpp,
}

impl HealthStateRevisionLikeCpp {
    pub(super) fn new() -> Self {
        Self {
            value: 0,
            authority: HealthStateRevisionAuthorityLikeCpp::new(),
        }
    }

    pub(super) fn advance(&mut self) {
        self.value = self.authority.next();
    }
}

impl PartialEq for HealthStateRevisionLikeCpp {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    pub(super) world: WorldObject,
    pub(super) data: UnitDataValues,
    pub(super) unit_data_changes: UpdateMask,
    pub(super) death_state: DeathState,
    /// Monotonic version of the represented health/max-health/death tuple. Cross-owner
    /// compatibility bridges use it as an optimistic CAS token so a delayed
    /// mirror update cannot overwrite a newer damage, heal, death, or
    /// resurrection transition.
    pub(super) health_state_revision_like_cpp: HealthStateRevisionLikeCpp,
    pub(super) unit_state: u32,
    pub(super) base_attack_speed: [u32; MAX_ATTACK],
    pub(super) mod_attack_speed_pct: [f32; MAX_ATTACK],
    pub(super) attack_timer: [u32; MAX_ATTACK],
    pub(super) weapon_damage: [[f32; 2]; MAX_ATTACK],
    pub(super) can_dual_wield: bool,
    pub(super) can_parry: bool,
    pub(super) can_block: bool,
    pub(super) emote_state: u32,
    pub(super) movement_flags: MovementFlag,
    pub(super) movement_time: u32,
    pub(super) speed_rate: [f32; MAX_MOVE_TYPE],
    /// C++ `Unit::m_movementCounter`, the sequence shared by movement-control packets.
    pub(super) movement_counter_like_cpp: u32,
    /// C++ `Unit::_movementForces->GetModMagnitude()`. A missing force container reads 1.0.
    pub(super) movement_force_mod_magnitude_like_cpp: f32,
    pub(super) ai_anim_kit_id: u16,
    pub(super) movement_anim_kit_id: u16,
    pub(super) melee_anim_kit_id: u16,
    pub(super) power_index: [Option<usize>; MAX_POWERS],
    pub(super) visibility_detection: UnitVisibilityDetectionStateLikeCpp,
    pub(super) subsystems: UnitSubsystems,
}

pub(super) fn power_slot(power: PowerType) -> Option<usize> {
    let value = power as i8;
    (0..MAX_POWERS as i8)
        .contains(&value)
        .then_some(value as usize)
}
