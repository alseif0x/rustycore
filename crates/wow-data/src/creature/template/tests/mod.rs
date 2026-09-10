//! Creature template regressions.
//!
//! Separated from the creature_template.rs root under #664.

use super::*;

use rand::{SeedableRng, rngs::StdRng};

use crate::CreatureModelInfoLikeCpp;

use super::*;

struct FixedCreatureModelRandomLikeCpp {
    weighted_roll: f32,
    other_gender_zero: bool,
}

impl CreatureModelSelectionRandomLikeCpp for FixedCreatureModelRandomLikeCpp {
    fn weighted_model_roll_like_cpp(&mut self, _total_weight: f32) -> f32 {
        self.weighted_roll
    }

    fn other_gender_roll_zero_like_cpp(&mut self) -> bool {
        self.other_gender_zero
    }
}

fn model_info_store_for_display_selection_tests_like_cpp(
    entries: impl IntoIterator<Item = CreatureModelInfoLikeCpp>,
) -> CreatureModelInfoStoreLikeCpp {
    CreatureModelInfoStoreLikeCpp::from_entries(entries)
}

fn model_info_like_cpp(
    display_id: u32,
    other_gender_display_id: u32,
    is_trigger: bool,
) -> CreatureModelInfoLikeCpp {
    CreatureModelInfoLikeCpp {
        display_id,
        bounding_radius: 0.0,
        combat_reach: 1.5,
        display_id_other_gender: other_gender_display_id,
        is_trigger,
    }
}

pub(crate) fn creature_template_lifecycle_record_for_test(
    entry: u32,
) -> CreatureTemplateLifecycleRecordLikeCpp {
    CreatureTemplateLifecycleRecordLikeCpp {
        entry,
        name: format!("template_{entry}"),
        ai_name: String::new(),
        script_name: String::new(),
        required_expansion: 0,
        faction: 35,
        npc_flags: 0,
        speed_walk: 1.0,
        speed_run: 1.14286,
        scale: 1.0,
        classification: 0,
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        unit_flags: 0,
        unit_flags2: 0,
        unit_flags3: 0,
        creature_type: 0,
        family: 0,
        trainer_class: 0,
        unit_class: 1,
        vehicle_id: 0,
        movement_type: 0,
        ground_movement_type: CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: CreatureFlightMovementType::None as u8,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms: DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        flags_extra: 0,
        string_id: String::new(),
        regen_health: true,
        spells: [0; MAX_CREATURE_SPELLS_LIKE_CPP],
        models: Vec::new(),
    }
}

fn base_difficulty_record() -> CreatureDifficultyRecordLikeCpp {
    CreatureDifficultyRecordLikeCpp {
        entry: 1,
        difficulty_id: 0,
        min_level: 1,
        max_level: 1,
        health_scaling_expansion: 0,
        health_modifier: 1.0,
        mana_modifier: 1.0,
        armor_modifier: 1.0,
        damage_modifier: 1.0,
        creature_difficulty_id: 0,
        type_flags: 0,
        type_flags2: 0,
        loot_id: 0,
        pickpocket_loot_id: 0,
        skin_loot_id: 0,
        gold_min: 0,
        gold_max: 0,
        static_flags: [0; 8],
    }
}

fn addon_row(owner_id: u64) -> CreatureAddonRowLikeCpp {
    CreatureAddonRowLikeCpp {
        owner_id,
        path_id: 0,
        mount: 0,
        stand_state: 0,
        anim_tier: 0,
        vis_flags: 0,
        sheath_state: 0,
        pvp_flags: 0,
        emote: 0,
        ai_anim_kit: 0,
        movement_anim_kit: 0,
        melee_anim_kit: 0,
        visibility_distance_type: 0,
        auras: String::new(),
    }
}

mod scenarios_1;
mod scenarios_2;
