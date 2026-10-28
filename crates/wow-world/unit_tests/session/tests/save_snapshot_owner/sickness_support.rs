use super::*;

pub(super) fn sickness_race_row(spell_id: i32) -> wow_data::character_progression::ChrRacesEntry {
    wow_data::character_progression::ChrRacesEntry {
        id: 2,
        client_prefix: String::new(),
        client_file_string: String::new(),
        name: "test race".to_owned(),
        flags: 0,
        male_display_id: 0,
        female_display_id: 0,
        high_res_male_display_id: 0,
        high_res_female_display_id: 0,
        res_sickness_spell_id: spell_id,
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

pub(super) fn sickness_spell_info(
    spell_id: i32,
    effects: &[(u32, i32, i32)],
) -> wow_data::SpellInfo {
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
        effects: effects
            .iter()
            .map(
                |&(effect_index, effect_aura, effect_base_points)| wow_data::SpellEffectInfo {
                    effect_index,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura,
                    effect_base_points,
                    ..Default::default()
                },
            )
            .collect(),
    }
}

pub(super) fn install_sickness_catalogs(
    session: &mut WorldSession,
    spell_id: i32,
    effects: &[(u32, i32, i32)],
) {
    session.set_chr_races_store(std::sync::Arc::new(
        wow_data::character_progression::ChrRacesStore::from_entries([sickness_race_row(spell_id)]),
    ));
    let mut all_effects = vec![(0, 80, 0), (1, 79, 0), (2, 159, 0)];
    for &(index, aura_type, base_amount) in effects {
        if let Some(existing) = all_effects.iter_mut().find(|row| row.0 == index) {
            *existing = (index, aura_type, base_amount);
        } else {
            all_effects.push((index, aura_type, base_amount));
        }
    }
    let mut spell_store = crate::selected_spell_fixtures::canonical_spell_store([(
        spell_id as u32,
        0,
    )]);
    spell_store.insert(spell_id, sickness_spell_info(spell_id, &all_effects));
    spell_store.insert_spell_misc_attributes_like_cpp(spell_id, [0; 15]);
    session.set_spell_store(std::sync::Arc::new(spell_store));
    session.set_difficulty_store(std::sync::Arc::new(wow_data::DifficultyStore::from_ids([
        0,
    ])));
}

pub(super) fn sickness_application(
    caster_guid: wow_core::ObjectGuid,
    spell_id: i32,
    slot: u8,
    effect_mask: u32,
    duration_total: i32,
    duration_remaining: i32,
    applied_at: std::time::Instant,
) -> wow_entities::AuraApplicationLikeCpp {
    wow_entities::AuraApplicationLikeCpp {
        spell_id,
        difficulty_id: 0,
        caster_guid,
        slot,
        duration_total,
        duration_remaining,
        stack_count: 1,
        aura_flags: 0,
        effect_mask,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: None,
        represented_amount: 0,
        represented_effect_amounts: Vec::new(),
        represented_misc_value: None,
        represented_multiplier: 1.0,
        applied_at,
    }
}

pub(super) fn insert_sickness_application(
    session: &mut WorldSession,
    application: wow_entities::AuraApplicationLikeCpp,
    effects: Option<Vec<(u32, i32, i32, i32)>>,
) {
    session
        .with_owned_player_mut_like_cpp(|player| {
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            let slot = application.slot;
            auras.insert_runtime_application_like_cpp(application);
            if let Some(effects) = effects {
                auras.insert_threat_snapshot_like_cpp(
                    slot,
                    wow_entities::AuraThreatSnapshotLikeCpp::new([0; 2], effects),
                );
            }
        })
        .unwrap();
}

pub(super) fn capture_sickness(
    session: &mut WorldSession,
) -> Option<wow_persistence::PlayerLifecycleAuraSpellSave> {
    session
        .prepare_player_save_like_cpp(1_700_000_000)
        .map(|prepared| prepared.request.resurrection_sickness)
        .flatten()
}
