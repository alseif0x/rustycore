//! Spell-store and catalog builders for the spell handler scenarios.
//!
//! Split out of the inline test module under #624; behaviour unchanged.

use super::*;

pub(super) fn basic_spell_store(
    spell_ids: impl IntoIterator<Item = i32>,
) -> Arc<wow_data::SpellStore> {
    let mut spell_store = wow_data::SpellStore::new();
    for spell_id in spell_ids {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: 0,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: Vec::new(),
            },
        );
    }
    Arc::new(spell_store)
}
pub(super) fn spell_store_with_mana_power_cost_like_cpp(
    spell_id: i32,
    mana_cost: i32,
    power_cost_pct: f32,
) -> Arc<wow_data::SpellStore> {
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: vec![wow_data::SpellPowerCostInfoLikeCpp {
                order_index: 0,
                power_type: PowerType::Mana as i8,
                mana_cost,
                mana_cost_per_level: 0,
                mana_per_second: 0,
                power_cost_pct,
                power_cost_max_pct: 0.0,
                power_pct_per_second: 0.0,
                required_aura_spell_id: 0,
                optional_cost: 0,
            }],
            effects: Vec::new(),
        },
    );
    Arc::new(spell_store)
}
pub(super) fn spell_store_with_global_cooldown_and_mana_power_cost_like_cpp(
    spell_id: i32,
    cooldown_ms: u32,
    mana_cost: i32,
    power_cost_pct: f32,
) -> Arc<wow_data::SpellStore> {
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: vec![wow_data::SpellPowerCostInfoLikeCpp {
                order_index: 0,
                power_type: PowerType::Mana as i8,
                mana_cost,
                mana_cost_per_level: 0,
                mana_per_second: 0,
                power_cost_pct,
                power_cost_max_pct: 0.0,
                power_pct_per_second: 0.0,
                required_aura_spell_id: 0,
                optional_cost: 0,
            }],
            effects: Vec::new(),
        },
    );
    Arc::new(spell_store)
}
pub(super) fn spell_store_with_mana_cost_and_missing_focus_like_cpp(
    spell_id: i32,
    mana_cost: i32,
) -> Arc<wow_data::SpellStore> {
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_SUMMON_OBJECT_WILD,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 777,
            power_costs: vec![wow_data::SpellPowerCostInfoLikeCpp {
                order_index: 0,
                power_type: PowerType::Mana as i8,
                mana_cost,
                mana_cost_per_level: 0,
                mana_per_second: 0,
                power_cost_pct: 0.0,
                power_cost_max_pct: 0.0,
                power_pct_per_second: 0.0,
                required_aura_spell_id: 0,
                optional_cost: 0,
            }],
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SUMMON_OBJECT_WILD,
                ..Default::default()
            }],
        },
    );
    Arc::new(spell_store)
}
pub(super) fn spell_store_with_global_cooldown(
    spell_id: i32,
    cooldown_ms: u32,
) -> Arc<wow_data::SpellStore> {
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    Arc::new(spell_store)
}
pub(super) fn mounted_spell_store(spell_id: i32, creature_entry: i32) -> Arc<wow_data::SpellStore> {
    let mut spell_store =
        crate::selected_spell_fixtures::canonical_spell_store([(spell_id as u32, 0)]);
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 77,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOUNTED),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
                effect_base_points: 77,
                effect_misc_value_1: creature_entry,
                ..Default::default()
            }],
        },
    );
    Arc::new(spell_store)
}
pub(super) fn mounted_flying_spell_store(
    spell_id: i32,
    creature_entry: i32,
) -> Arc<wow_data::SpellStore> {
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 77,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOUNTED),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![
                wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
                    effect_base_points: 77,
                    effect_misc_value_1: creature_entry,
                    ..Default::default()
                },
                wow_data::SpellEffectInfo {
                    effect_index: 1,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura:
                        wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED,
                    effect_base_points: 280,
                    ..Default::default()
                },
            ],
        },
    );
    Arc::new(spell_store)
}
pub(super) fn shapeshift_spell_store(spell_id: i32, form_id: i32) -> Arc<wow_data::SpellStore> {
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_SHAPESHIFT),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_SHAPESHIFT,
                effect_misc_value_1: form_id,
                ..Default::default()
            }],
        },
    );
    Arc::new(spell_store)
}
pub(super) fn mounted_spell_store_with_active_shapeshift_aura(
    mount_spell_id: i32,
    shapeshift_spell_id: i32,
    form_id: i32,
) -> Arc<wow_data::SpellStore> {
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        mount_spell_id,
        wow_data::SpellInfo {
            spell_id: mount_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 77,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOUNTED),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
                effect_base_points: 77,
                ..Default::default()
            }],
        },
    );
    spell_store.insert(
        shapeshift_spell_id,
        wow_data::SpellInfo {
            spell_id: shapeshift_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_SHAPESHIFT),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_SHAPESHIFT,
                effect_misc_value_1: form_id,
                ..Default::default()
            }],
        },
    );
    Arc::new(spell_store)
}
pub(super) fn mounted_spell_store_with_transform_spell(
    mount_spell_id: i32,
    transform_spell_id: i32,
    allow_while_mounted: bool,
) -> Arc<wow_data::SpellStore> {
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        mount_spell_id,
        wow_data::SpellInfo {
            spell_id: mount_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 77,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOUNTED),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
                effect_base_points: 77,
                ..Default::default()
            }],
        },
    );
    spell_store.insert(
        transform_spell_id,
        wow_data::SpellInfo {
            spell_id: transform_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_TRANSFORM),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_TRANSFORM,
                ..Default::default()
            }],
        },
    );
    if allow_while_mounted {
        let mut attributes = [0; 15];
        attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_ALLOW_WHILE_MOUNTED;
        spell_store.insert_spell_misc_attributes_like_cpp(transform_spell_id, attributes);
    }
    Arc::new(spell_store)
}
pub(super) fn chr_races_entry_for_test(
    id: u32,
    flags: i32,
) -> wow_data::character_progression::ChrRacesEntry {
    wow_data::character_progression::ChrRacesEntry {
        id,
        client_prefix: String::new(),
        client_file_string: String::new(),
        name: String::new(),
        flags,
        male_display_id: 0,
        female_display_id: 0,
        high_res_male_display_id: 0,
        high_res_female_display_id: 0,
        res_sickness_spell_id: 0,
        splash_sound_id: 0,
        create_screen_file_data_id: 0,
        select_screen_file_data_id: 0,
        low_res_screen_file_data_id: 0,
        altered_form_start_visual_kit_id: [0; 3],
        altered_form_finish_visual_kit_id: [0; 3],
        heritage_armor_achievement_id: 0,
        starting_level: 1,
        ui_display_order: 0,
        playable_race_bit: 0,
        female_skeleton_file_data_id: 0,
        male_skeleton_file_data_id: 0,
        helmet_anim_scaling_race_id: 0,
        transmogrify_disabled_slot_mask: 0,
        faction_id: 0,
        cinematic_sequence_id: 0,
        base_language: 0,
        creature_type: 0,
        alliance: 0,
        race_related: 0,
        unaltered_visual_race_id: 0,
        default_class_id: 0,
        neutral_race_id: 0,
    }
}
pub(super) fn set_transformed_display_mount_check_stores_for_test(
    session: &mut crate::session::WorldSession,
    transformed_display_id: u32,
    model_flags: u32,
    race_flags: i32,
) {
    let display_extra_id = 91;
    let model_id = 92;
    let race_id = 7;
    session.set_creature_display_info_store(Arc::new(
        wow_data::CreatureDisplayInfoStore::from_entries([wow_data::CreatureDisplayInfoEntry {
            id: transformed_display_id,
            model_id,
            extended_display_info_id: display_extra_id,
            creature_model_scale: 1.0,
        }]),
    ));
    session.set_creature_display_info_extra_store(Arc::new(
        wow_data::CreatureDisplayInfoExtraStore::from_entries([
            creature_display_info_extra_for_test(display_extra_id as u32, race_id),
        ]),
    ));
    session.set_creature_model_data_store(Arc::new(
        wow_data::CreatureModelDataStore::from_entries([wow_data::CreatureModelDataEntry {
            id: u32::from(model_id),
            flags: model_flags,
            file_data_id: 0,
            collision_height: 2.0,
            hover_height: 1.0,
            model_scale: 1.0,
            mount_height: 0.0,
        }]),
    ));
    session.set_chr_races_store(Arc::new(
        wow_data::character_progression::ChrRacesStore::from_entries([chr_races_entry_for_test(
            u32::from(race_id as u8),
            race_flags,
        )]),
    ));
}
pub(super) fn mounted_spell_store_with_no_aura_cancel(
    spell_id: i32,
    creature_entry: i32,
) -> Arc<wow_data::SpellStore> {
    let mut spell_store =
        crate::selected_spell_fixtures::canonical_spell_store([(spell_id as u32, 0)]);
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 77,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOUNTED),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
                effect_base_points: 77,
                effect_misc_value_1: creature_entry,
                ..Default::default()
            }],
        },
    );
    let mut attributes = [0; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_NO_AURA_CANCEL;
    spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    Arc::new(spell_store)
}
pub(super) fn channeled_spell_store(spell_id: i32) -> Arc<wow_data::SpellStore> {
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    let mut attributes = [0; 15];
    attributes[1] = wow_data::spell::attributes::SPELL_ATTR1_IS_CHANNELLED;
    spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    Arc::new(spell_store)
}
pub(super) fn channeled_spell_store_with_no_aura_cancel(
    spell_id: i32,
) -> Arc<wow_data::SpellStore> {
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    let mut attributes = [0; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_NO_AURA_CANCEL;
    attributes[1] = wow_data::spell::attributes::SPELL_ATTR1_IS_CHANNELLED;
    spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    Arc::new(spell_store)
}
pub(super) fn mod_scale_spell_store(
    spell_id: i32,
    no_aura_cancel: bool,
) -> Arc<wow_data::SpellStore> {
    let mut spell_store =
        crate::selected_spell_fixtures::canonical_spell_store([(spell_id as u32, 0)]);
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 50,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_SCALE),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_SCALE,
                effect_base_points: 50,
                ..Default::default()
            }],
        },
    );
    if no_aura_cancel {
        let mut attributes = [0; 15];
        attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_NO_AURA_CANCEL;
        spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    }
    Arc::new(spell_store)
}
pub(super) fn mod_speed_no_control_spell_store(
    spell_id: i32,
    no_aura_cancel: bool,
) -> Arc<wow_data::SpellStore> {
    let mut spell_store =
        crate::selected_spell_fixtures::canonical_spell_store([(spell_id as u32, 0)]);
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 50,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_SPEED_NO_CONTROL),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_SPEED_NO_CONTROL,
                effect_base_points: 50,
                ..Default::default()
            }],
        },
    );
    if no_aura_cancel {
        let mut attributes = [0; 15];
        attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_NO_AURA_CANCEL;
        spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    }
    Arc::new(spell_store)
}
pub(super) fn self_res_spell_store(spell_id: i32) -> Arc<wow_data::SpellStore> {
    let mut attributes = [0; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_ALLOW_CAST_WHILE_DEAD as i32;
    let effect = wow_data::spell_db2::SpellEffectDb2Entry {
        id: 1,
        difficulty_id: 0,
        effect_index: 0,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SELF_RESURRECT,
        effect_amplitude: 0.0,
        effect_attributes: 0,
        effect_aura: 0,
        effect_aura_period: 0,
        effect_base_points: -35,
        effect_bonus_coefficient: 0.0,
        effect_chain_amplitude: 0.0,
        effect_chain_targets: 0,
        effect_die_sides: 0,
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
        effect_misc_value: [77, 0],
        effect_radius_index: [0; 2],
        effect_spell_class_mask: [0; 4],
        implicit_target: [0; 2],
        spell_id: spell_id as u32,
    };
    let effective = wow_data::EffectiveCoreSpellDb2StoresLikeCpp::new(
        wow_data::spell_db2::SpellCategoriesStore::from_entries([]),
        wow_data::spell_db2::SpellMiscStore::from_entries([wow_data::spell_db2::SpellMiscEntry {
            id: 1,
            difficulty_id: 0,
            attributes,
            spell_id: spell_id as u32,
            ..Default::default()
        }]),
        wow_data::spell_db2::SpellEffectDb2Store::from_entries([effect]),
        wow_data::spell_db2::SpellShapeshiftStore::from_entries([]),
        wow_data::spell_db2::SpellInterruptsStore::from_entries([]),
        wow_data::spell_db2::SpellCastTimesStore::from_entries([]),
        wow_data::spell_db2::SpellCooldownsStore::from_entries([]),
        wow_data::spell_db2::SpellCastingRequirementsStore::from_entries([]),
        wow_data::spell_db2::SpellPowerStore::from_entries([]),
        wow_data::spell_db2::SpellPowerDifficultyStore::from_entries([]),
    );
    let mut spell_store =
        crate::selected_spell_fixtures::canonical_spell_store([(spell_id as u32, 0)])
            .hydrate_effective_core_db2_like_cpp(effective);
    spell_store.install_effective_cast_costs_like_cpp(
        wow_data::SpellCastCostStoreLikeCpp::from_effective_rows_like_cpp([], []),
    );
    Arc::new(spell_store)
}
