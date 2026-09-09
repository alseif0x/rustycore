//! Behaviour tests for [`super`].
//!
//! Extracted from `spell.rs`, which was 13,736 lines of which
//! 6,177 — 45% — were this one `mod tests`. The production code and its
//! module boundaries are untouched: moving tests moves no invariant. Dedenting by
//! one level lets rustfmt collapse some argument lists onto a single line, which
//! drops their trailing commas; that is the only difference from the original text.

#![cfg(test)]

use super::*;

fn test_skill_line_like_cpp(
    id: u32,
    category_id: i8,
    parent_skill_line_id: u32,
) -> crate::skill_talent::SkillLineEntry {
    crate::skill_talent::SkillLineEntry {
        id,
        display_name: String::new(),
        alternate_verb: String::new(),
        description: String::new(),
        horde_display_name: String::new(),
        override_source_info_display_name: String::new(),
        category_id,
        spell_icon_file_id: 0,
        can_link: 0,
        parent_skill_line_id,
        parent_tier_index: 0,
        flags: 0,
        spell_book_spell_id: 0,
    }
}

fn test_skill_effect_like_cpp(effect_index: u32, skill_id: i32) -> SpellEffectInfo {
    SpellEffectInfo {
        effect_index,
        effect: spell_effect_types::SPELL_EFFECT_SKILL,
        effect_misc_value_1: skill_id,
        ..Default::default()
    }
}

use crate::{Condition, ConditionEntriesByTypeStore};
use wow_constants::{ConditionSourceType, ConditionType};

fn learn_skill_source(
    spell_id: u32,
    difficulty_none: bool,
    effects: Vec<SpellLearnSkillEffectLikeCpp>,
) -> SpellLearnSkillSourceSpellInfoLikeCpp {
    SpellLearnSkillSourceSpellInfoLikeCpp {
        spell_id,
        difficulty_none,
        effects,
    }
}

fn spell_area_row(spell_id: u32) -> SpellAreaRowLikeCpp {
    SpellAreaRowLikeCpp {
        spell_id,
        area_id: 0,
        quest_start: 0,
        quest_start_status: 0,
        quest_end_status: 0,
        quest_end: 0,
        aura_spell: 0,
        race_mask: 0,
        gender: GENDER_NONE_LIKE_CPP,
        flags: 0,
    }
}

fn custom_attr_source(
    spell_id: u32,
    difficulty: u32,
    effect_type: u32,
) -> SpellCustomAttributeSourceSpellInfoLikeCpp {
    SpellCustomAttributeSourceSpellInfoLikeCpp {
        spell_id,
        difficulty,
        effects: vec![SpellEffectInfo {
            effect_index: 0,
            effect: effect_type,
            ..Default::default()
        }],
    }
}

fn learn_source(
    spell_id: u32,
    is_talent: bool,
    is_passive: bool,
    has_skill_step_effect: bool,
    learn_spell_effects: Vec<SpellLearnSpellEffectLikeCpp>,
) -> SpellLearnSourceSpellInfoLikeCpp {
    SpellLearnSourceSpellInfoLikeCpp {
        spell_id,
        difficulty_none: true,
        is_talent,
        is_passive,
        has_skill_step_effect,
        learn_spell_effects,
    }
}

fn test_spell_info_with_aura(spell_id: i32, aura_type: i32) -> SpellInfo {
    SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: Some(aura_type),
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![SpellEffectInfo {
            effect_index: 0,
            effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: aura_type,
            ..SpellEffectInfo::default()
        }],
    }
}

fn test_spell_info_without_aura(spell_id: i32) -> SpellInfo {
    SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: spell_effect_types::SPELL_EFFECT_NONE,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: Vec::new(),
    }
}

fn test_spell_proc_entry_like_cpp() -> SpellProcEntryLikeCpp {
    SpellProcEntryLikeCpp {
        school_mask: 0,
        spell_family_name: 0,
        spell_family_mask: [0, 0, 0, 0],
        proc_flags: [PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP, 0],
        spell_type_mask: 0,
        spell_phase_mask: PROC_SPELL_PHASE_CAST_LIKE_CPP,
        hit_mask: 0,
        attributes_mask: 0,
        disable_effects_mask: 0,
        procs_per_minute: 0.0,
        chance: 0.0,
        cooldown_ms: 0,
        charges: 0,
    }
}

fn test_spell_proc_event_like_cpp(type_mask: u32) -> SpellProcEventInfoLikeCpp {
    SpellProcEventInfoLikeCpp {
        type_mask: [type_mask, 0],
        actor_is_player: false,
        action_target_exists: false,
        action_target_is_honor_or_xp: false,
        proc_spell_has_positive_power_cost: None,
        school_mask: SPELL_SCHOOL_MASK_ALL_LIKE_CPP,
        spell_info: None,
        spell_type_mask: PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP,
        spell_phase_mask: PROC_SPELL_PHASE_CAST_LIKE_CPP,
        hit_mask: PROC_HIT_NORMAL_LIKE_CPP,
    }
}

fn test_spell_proc_store_with_entries_like_cpp(
    entries: impl IntoIterator<Item = (u32, u32, [u32; 2])>,
) -> SpellProcStoreLikeCpp {
    let mut store = SpellProcStoreLikeCpp::default();
    for (spell_id, difficulty, proc_flags) in entries {
        let mut entry = test_spell_proc_entry_like_cpp();
        entry.proc_flags = proc_flags;
        store.proc_entries_by_spell_and_difficulty.insert(
            SpellProcKeyLikeCpp {
                spell_id,
                difficulty,
            },
            entry,
        );
    }
    store
}

fn test_implicit_spell_proc_source_like_cpp() -> ImplicitSpellProcSourceLikeCpp {
    ImplicitSpellProcSourceLikeCpp {
        spell_id: 1000,
        difficulty: 0,
        spell_family_name: 0,
        proc_flags: [PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP, 0],
        proc_chance: 0.0,
        proc_cooldown_ms: 0,
        proc_charges: 0,
        proc_base_ppm: 0.0,
        attributes3: 0,
        effects: Vec::new(),
    }
}

fn test_implicit_proc_effect_like_cpp(
    effect_index: u32,
    aura_type: i32,
    spell_class_mask: [u32; 4],
) -> ImplicitSpellProcEffectLikeCpp {
    ImplicitSpellProcEffectLikeCpp {
        effect_index,
        is_effect: true,
        is_aura: true,
        aura_type,
        spell_class_mask,
        calc_value: 0,
        trigger_spell: 0,
    }
}

fn test_implicit_proc_effect_with_calc_like_cpp(
    effect_index: u32,
    aura_type: i32,
    calc_value: i32,
) -> ImplicitSpellProcEffectLikeCpp {
    let mut effect = test_implicit_proc_effect_like_cpp(effect_index, aura_type, [0, 0, 0, 0]);
    effect.calc_value = calc_value;
    effect
}

fn test_spell_aura_options_entry_like_cpp(
    id: u32,
    spell_id: u32,
    difficulty_id: u8,
    proc_type_mask: [i32; 2],
    proc_chance: u8,
    proc_charges: i32,
    proc_category_recovery: i32,
    spell_procs_per_minute_id: u16,
) -> crate::spell_db2::SpellAuraOptionsEntry {
    crate::spell_db2::SpellAuraOptionsEntry {
        id,
        difficulty_id,
        cumulative_aura: 0,
        proc_category_recovery,
        proc_chance,
        proc_charges,
        spell_procs_per_minute_id,
        proc_type_mask,
        spell_id,
    }
}

fn test_spell_misc_entry_like_cpp(
    id: u32,
    spell_id: u32,
    difficulty_id: u8,
    attributes3: u32,
) -> crate::spell_db2::SpellMiscEntry {
    let mut attributes = [0; 15];
    attributes[3] = attributes3 as i32;
    crate::spell_db2::SpellMiscEntry {
        id,
        attributes,
        difficulty_id,
        casting_time_index: 0,
        duration_index: 0,
        range_index: 0,
        school_mask: 0,
        speed: 0.0,
        launch_delay: 0.0,
        min_duration: 0.0,
        spell_icon_file_data_id: 0,
        active_icon_file_data_id: 0,
        content_tuning_id: 0,
        show_future_spell_player_condition_id: 0,
        spell_id,
    }
}

fn test_spell_effect_db2_entry_like_cpp(
    id: u32,
    spell_id: u32,
    difficulty_id: i32,
    effect_index: i32,
    effect: u32,
    effect_mechanic: i32,
) -> crate::spell_db2::SpellEffectDb2Entry {
    crate::spell_db2::SpellEffectDb2Entry {
        id,
        difficulty_id,
        effect_index,
        effect,
        effect_amplitude: 0.0,
        effect_attributes: 0,
        effect_aura: 0,
        effect_aura_period: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        effect_chain_amplitude: 0.0,
        effect_chain_targets: 0,
        effect_die_sides: 0,
        effect_item_type: 0,
        effect_mechanic,
        effect_points_per_resource: 0.0,
        effect_pos_facing: 0.0,
        effect_real_points_per_level: 0.0,
        effect_trigger_spell: 0,
        bonus_coefficient_from_ap: 0.0,
        pvp_multiplier: 0.0,
        coefficient: 0.0,
        variance: 0.0,
        resource_coefficient: 0.0,
        group_size_base_points_coefficient: 0.0,
        effect_misc_value: [0; 2],
        effect_radius_index: [0; 2],
        effect_spell_class_mask: [0; 4],
        implicit_target: [0; 2],
        spell_id,
    }
}

fn test_spell_proc_row_like_cpp(spell_id: i32) -> SpellProcRowLikeCpp {
    SpellProcRowLikeCpp {
        spell_id,
        school_mask: 0,
        spell_family_name: 0,
        spell_family_mask: [0; 4],
        proc_flags: [0; 2],
        spell_type_mask: 0,
        spell_phase_mask: 0,
        hit_mask: 0,
        attributes_mask: 0,
        disable_effects_mask: 0,
        procs_per_minute: 0.0,
        chance: 0.0,
        cooldown_ms: 0,
        charges: 0,
    }
}

fn test_spell_proc_source_like_cpp(
    spell_id: u32,
    first_rank_spell_id: u32,
    next_rank_spell_id: Option<u32>,
) -> SpellProcSourceSpellInfoLikeCpp {
    SpellProcSourceSpellInfoLikeCpp {
        spell_id,
        difficulty: 0,
        first_rank_spell_id,
        next_rank_spell_id,
        spell_family_name: 0,
        proc_flags: [0; 2],
        proc_charges: 0,
        proc_chance: 0.0,
        proc_cooldown_ms: 0,
        proc_base_ppm: 0.0,
        attributes3: 0,
        effects: Vec::new(),
    }
}

fn serverside_effect_row(spell_id: u32, effect_index: i32) -> ServersideSpellEffectRowLikeCpp {
    ServersideSpellEffectRowLikeCpp {
        spell_id,
        effect_index,
        difficulty_id: 0,
        effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA as i32,
        effect_aura: SPELL_AURA_DUMMY_LIKE_CPP,
        effect_amplitude: 0.0,
        effect_attributes: 0,
        effect_aura_period: 0,
        effect_bonus_coefficient: 0.0,
        effect_chain_amplitude: 0.0,
        effect_chain_targets: 0,
        effect_item_type: 0,
        effect_mechanic: 0,
        effect_points_per_resource: 0.0,
        effect_pos_facing: 0.0,
        effect_real_points_per_level: 0.0,
        effect_trigger_spell: 0,
        bonus_coefficient_from_ap: 0.0,
        pvp_multiplier: 0.0,
        coefficient: 0.0,
        variance: 0.0,
        resource_coefficient: 0.0,
        group_size_base_points_coefficient: 0.0,
        effect_base_points: 1.0,
        effect_misc_value_1: 0,
        effect_misc_value_2: 0,
        effect_radius_index_1: 0,
        effect_radius_index_2: 0,
        effect_spell_class_mask: [0, 0, 0, 0],
        implicit_target_1: 0,
        implicit_target_2: 0,
    }
}

fn serverside_spell_row(spell_id: u32, difficulty_id: u32) -> ServersideSpellRowLikeCpp {
    ServersideSpellRowLikeCpp {
        spell_id,
        difficulty_id,
        category_id: 1,
        dispel: 2,
        mechanic: 3,
        attributes: 4,
        attributes_ex: [5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18],
        stances: 19,
        stances_not: 20,
        targets: 21,
        target_creature_type: 22,
        requires_spell_focus: 23,
        facing_caster_flags: 24,
        caster_aura_state: 25,
        target_aura_state: 26,
        exclude_caster_aura_state: 27,
        exclude_target_aura_state: 28,
        caster_aura_spell: 29,
        target_aura_spell: 30,
        exclude_caster_aura_spell: 31,
        exclude_target_aura_spell: 32,
        caster_aura_type: 33,
        target_aura_type: 34,
        exclude_caster_aura_type: 35,
        exclude_target_aura_type: 36,
        casting_time_index: 37,
        recovery_time: 38,
        category_recovery_time: 39,
        start_recovery_category: 40,
        start_recovery_time: 41,
        interrupt_flags: 42,
        aura_interrupt_flags: [43, 44],
        channel_interrupt_flags: [45, 46],
        proc_flags: [47, 48],
        proc_chance: 49,
        proc_charges: 50,
        proc_cooldown: 51,
        proc_base_ppm: 52.0,
        max_level: 53,
        base_level: 54,
        spell_level: 55,
        duration_index: 56,
        range_index: 57,
        speed: 58.0,
        launch_delay: 59.0,
        stack_amount: 60,
        equipped_item_class: -1,
        equipped_item_sub_class_mask: 62,
        equipped_item_inventory_type_mask: 63,
        content_tuning_id: 64,
        spell_name: format!("Serverside {spell_id}"),
        cone_angle: 65.0,
        cone_width: 66.0,
        max_target_level: 67,
        max_affected_targets: 68,
        spell_family_name: 69,
        spell_family_flags: [70, 71, 72, 73],
        dmg_class: 74,
        prevention_type: 75,
        area_group_id: 76,
        school_mask: 77,
        charge_category_id: 78,
    }
}

fn serverside_spell_info_for_shapeshift(
    stances: u64,
    stances_not: u64,
    attributes: u32,
    attributes_ex2: u32,
) -> ServersideSpellInfoLikeCpp {
    let mut row = serverside_spell_row(7000, 0);
    row.attributes = attributes;
    row.attributes_ex = [0; 14];
    row.attributes_ex[1] = attributes_ex2;
    row.stances = stances;
    row.stances_not = stances_not;
    ServersideSpellInfoLikeCpp {
        row,
        effects: Vec::new(),
    }
}

fn shapeshift_form(flags: i32) -> crate::spell_db2::SpellShapeshiftFormEntry {
    crate::spell_db2::SpellShapeshiftFormEntry {
        id: 1,
        name: "Test Form".to_string(),
        creature_type: 0,
        flags,
        attack_icon_file_id: 0,
        bonus_action_bar: 0,
        combat_round_time: 0,
        damage_variance: 0.0,
        mount_type_id: 0,
        creature_display_id: [0; 4],
        preset_spell_id: [0; crate::spell_db2::MAX_SHAPESHIFT_SPELLS],
    }
}

#[path = "spell_tests/instance.rs"]
mod instance;
#[path = "spell_tests/misc.rs"]
mod misc;
#[path = "spell_tests/spell_1.rs"]
mod spell_1;
#[path = "spell_tests/spell_2.rs"]
mod spell_2;
#[path = "spell_tests/spell_3.rs"]
mod spell_3;
#[path = "spell_tests/spell_4.rs"]
mod spell_4;
#[path = "spell_tests/spell_5.rs"]
mod spell_5;
#[path = "spell_tests/spell_6.rs"]
mod spell_6;
