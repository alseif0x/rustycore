//! Creature lifecycle and runtime state state definitions, part 2 of 2.
//!
//! Separated from the creature.rs root under #636. Behaviour is preserved.

use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Creature {
    pub(super) unit: Unit,
    pub(super) player_damage_req: u32,
    pub(super) dont_clear_tap_list_on_evade: bool,
    pub(super) pickpocket_loot_restore: i64,
    pub(super) corpse_remove_time: i64,
    pub(super) respawn_time: i64,
    pub(super) respawn_delay: u32,
    pub(super) corpse_delay: u32,
    pub(super) ignore_corpse_decay_ratio: bool,
    pub(super) wander_distance: f32,
    pub(super) boundary_check_time: u32,
    pub(super) combat_pulse_time: u32,
    pub(super) combat_pulse_delay: u32,
    pub(super) react_state: ReactState,
    pub(super) default_movement_type: MovementGeneratorType,
    pub(super) waypoint_path_id: u32,
    pub(super) spawn_id: u64,
    pub(super) equipment_id: u8,
    pub(super) original_equipment_id: i8,
    pub(super) already_call_assistance: bool,
    pub(super) already_searched_assistance: bool,
    pub(super) cannot_reach_target: bool,
    pub(super) cannot_reach_timer: u32,
    pub(super) melee_damage_school_mask: u32,
    pub(super) original_entry: u32,
    pub(super) trigger_just_appeared: bool,
    pub(super) respawn_compatibility_mode: bool,
    pub(super) last_damaged_time: i64,
    pub(super) regenerate_health: bool,
    pub(super) is_missing_can_swim_flag_out_of_combat: bool,
    pub(super) unit_type_mask: u32,
    pub(super) gossip_menu_id: u32,
    pub(super) sparring_health_pct: f32,
    pub(super) regen_timer: u32,
    pub(super) spells: [u32; MAX_CREATURE_SPELLS],
    pub(super) disable_reputation_gain: bool,
    pub(super) sight_distance: f32,
    pub(super) combat_distance: f32,
    pub(super) loot_mode: u16,
    pub(super) is_temp_world_object: bool,
    pub(super) grid_unload_cleanup_before_delete_count: u32,
    pub(super) grid_unload_delete_requested: bool,
    pub(super) grid_unload_respawn_relocation_requested: bool,
    pub(super) owned_dynamic_objects: Vec<ObjectGuid>,
    pub(super) removed_dynamic_objects_from_grid_unload: Vec<ObjectGuid>,
    pub(super) owned_area_triggers: Vec<ObjectGuid>,
    pub(super) removed_area_triggers_from_grid_unload: Vec<ObjectGuid>,
    pub(super) lifecycle_metadata: CreatureLifecycleMetadata,
    pub(super) runtime_state: CreatureRuntimeState,
    pub(super) ai_ownership: CreatureAiOwnershipState,
    pub(super) tap_list: Vec<ObjectGuid>,
    pub(super) attack_reputation_faction_id: Option<u32>,
    pub(super) is_contested_guard_faction: bool,
    pub(super) spell_focus: CreatureSpellFocusStateLikeCpp,
    pub(super) combat_log_stats: CreatureCombatLogStatsLikeCpp,
    /// Monotonic identity for the creature loot-producing lifetime. Async
    /// `Unit::Kill` generation captures this value and may install its pools
    /// only while the same death lifetime is still current. Corpse removal
    /// and respawn advance it, preventing an old generator from winning an
    /// ABA race against a later incarnation of the same spawn GUID.
    pub(super) loot_lifecycle_revision: u64,
    pub(super) loot_authority: OwnedLootAuthority,
    pub(super) shared_loot: Option<CreatureOwnedLoot>,
    pub(super) personal_loot: HashMap<ObjectGuid, CreatureOwnedLoot>,
}

pub(super) fn consume_timer(timer: &mut u32, diff_ms: u32) -> bool {
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

pub(super) fn power_type_from_u8(power: u8) -> PowerType {
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
