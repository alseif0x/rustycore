use super::*;

fn regular_store(
    rows: impl IntoIterator<Item = crate::spell_db2::SpellEffectDb2Entry>,
) -> SpellStore {
    SpellStore::from_spell_db2_stores_like_cpp(
        &crate::spell_db2::SpellCategoriesStore::from_entries([]),
        &crate::spell_db2::SpellMiscStore::from_entries([]),
        &crate::spell_db2::SpellEffectDb2Store::from_entries(rows),
        &crate::spell_db2::SpellShapeshiftStore::from_entries([]),
    )
}

fn regular_effect(
    record_id: u32,
    spell_id: u32,
    difficulty_id: i32,
    effect_index: i32,
    effect: u32,
    mechanic: i32,
    target_spell_id: u32,
) -> crate::spell_db2::SpellEffectDb2Entry {
    let mut row = test_spell_effect_db2_entry_like_cpp(
        record_id,
        spell_id,
        difficulty_id,
        effect_index,
        effect,
        mechanic,
    );
    row.effect_aura = 403;
    row.effect_misc_value[0] = target_spell_id as i32;
    row
}

fn blank_server_effect(index: i32, difficulty_id: u32) -> ServersideSpellEffectLikeCpp {
    ServersideSpellEffectLikeCpp {
        effect_index: index,
        difficulty_id,
        effect: 0,
        effect_aura: 0,
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
        effect_base_points: 0,
        effect_misc_value: [0; 2],
        effect_radius_index: [0; 2],
        effect_spell_class_mask: [0; 4],
        implicit_target: [0; 2],
    }
}

#[test]
fn regular_effect_rows_normalize_difficulty_before_slot_winner_selection() {
    let full_target = 0xF123_4567;
    let shadowed_target = 0xE234_5678;
    let negative_difficulty_target = 0xD345_6789;
    let store = regular_store([
        regular_effect(
            10,
            81_001,
            256,
            0,
            spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            11,
            shadowed_target,
        ),
        regular_effect(
            20,
            81_001,
            0,
            0,
            spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            29,
            full_target,
        ),
        regular_effect(
            30,
            81_002,
            256,
            0,
            spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            31,
            shadowed_target,
        ),
        regular_effect(
            40,
            81_002,
            0,
            0,
            spell_effect_types::SPELL_EFFECT_NONE,
            41,
            shadowed_target,
        ),
        regular_effect(
            50,
            81_003,
            -1,
            2,
            spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            53,
            negative_difficulty_target,
        ),
    ]);

    let normalized = store
        .effect_rows_for_exact_difficulty(81_001, 0)
        .expect("raw 256 and zero share the source Difficulty key");
    let winner = normalized
        .iter()
        .find(|effect| effect.effect_index == 0)
        .expect("the winner remains at effect index zero");
    assert_eq!(winner.effect, spell_effect_types::SPELL_EFFECT_APPLY_AURA);
    assert_eq!(winner.effect_mechanic, 29);
    assert_eq!(winner.effect_misc_value_1, full_target as i32);
    let hit = store
        .hit_metadata_for_difficulty_like_cpp(81_001, 0, None)
        .expect("the normalized effect contributes hit metadata");
    assert_eq!(hit.effect_mechanics.get(&0), Some(&29));
    assert!(store.may_override_spell_visual(full_target));
    assert!(!store.may_override_spell_visual(shadowed_target));

    let inert_winner = store
        .effect_rows_for_exact_difficulty(81_002, 0)
        .expect("the inert winner is retained");
    assert_eq!(inert_winner[0].effect, spell_effect_types::SPELL_EFFECT_NONE);
    assert_eq!(inert_winner[0].effect_mechanic, 41);
    assert!(!store.may_override_spell_visual(shadowed_target));

    let wrapped = store
        .effect_rows_for_exact_difficulty(81_003, u8::MAX)
        .expect("signed -1 casts to Difficulty 255");
    assert_eq!(wrapped[0].effect_index, 2);
    assert_eq!(wrapped[0].effect_misc_value_1, negative_difficulty_target as i32);
    let hit = store
        .hit_metadata_for_difficulty_like_cpp(81_003, u8::MAX, None)
        .expect("the wrapped difficulty also indexes hit metadata");
    assert_eq!(hit.effect_mechanics.get(&2), Some(&53));
    assert!(store.may_override_spell_visual(negative_difficulty_target));
}

#[test]
fn server_effect_difficulty_256_joins_difficulty_none_without_validation() {
    let mut effect = serverside_effect_row(82_001, 0);
    effect.difficulty_id = 256;
    let outcome = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
        [effect],
        |_| false,
        |_| false,
        |_| true,
    );

    assert_eq!(outcome.loaded_effect_count, 1);
    assert!(outcome.errors.is_empty());
    assert!(outcome
        .store
        .effects_for_spell_difficulty_like_cpp(82_001, 0)
        .is_some());
    assert!(outcome
        .store
        .effects_for_spell_difficulty_like_cpp(82_001, 256)
        .is_none());

    let spells = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [serverside_spell_row(82_001, 0)],
        &outcome.store,
        |_| false,
    );
    let info = spells
        .store
        .get_serverside_spell_like_cpp(82_001, 0)
        .expect("the canonical difficulty-zero parent joins the effect");
    assert_eq!(info.effects.len(), 1);
    assert_eq!(info.effects[0].difficulty_id, 0);
}

#[test]
fn server_effect_and_parent_aliases_join_at_difficulty_231_in_query_order() {
    let mut effect = serverside_effect_row(82_002, 2);
    effect.difficulty_id = 999;
    effect.effect_aura = 403;
    effect.effect_misc_value_1 = 0xC456_789A_u32 as i32;
    let mut validated_difficulties = Vec::new();
    let effects = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
        [effect],
        |_| false,
        |difficulty_id| {
            validated_difficulties.push(difficulty_id);
            difficulty_id == 231
        },
        |_| true,
    );

    assert_eq!(validated_difficulties, [231]);
    assert!(effects.errors.is_empty());
    assert!(effects
        .store
        .effects_for_spell_difficulty_like_cpp(82_002, 231)
        .is_some());
    assert!(effects
        .store
        .effects_for_spell_difficulty_like_cpp(82_002, 999)
        .is_none());

    let mut first_parent = serverside_spell_row(82_002, 999);
    first_parent.spell_name = "first alias".to_string();
    first_parent.category_id = 101;
    let mut last_parent = serverside_spell_row(82_002, 231);
    last_parent.spell_name = "last alias".to_string();
    last_parent.category_id = 202;
    let spells = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [first_parent, last_parent],
        &effects.store,
        |_| false,
    );

    assert_eq!(spells.loaded_spell_count, 2);
    assert!(spells.errors.is_empty());
    assert_eq!(
        spells.store.serverside_spell_names,
        vec![
            (82_002, "first alias".to_string()),
            (82_002, "last alias".to_string())
        ]
    );
    assert!(spells
        .store
        .get_serverside_spell_like_cpp(82_002, 999)
        .is_none());
    let info = spells
        .store
        .get_serverside_spell_like_cpp(82_002, 231)
        .expect("raw 999 and 231 parent rows share one canonical key");
    assert_eq!(info.row.difficulty_id, 231);
    assert_eq!(info.row.spell_name, "first alias");
    assert_eq!(info.row.category_id, 202);
    assert_eq!(info.effects.len(), 3);
    assert_eq!(info.effects[0], blank_server_effect(0, 231));
    assert_eq!(info.effects[1], blank_server_effect(1, 231));
    assert_eq!(info.effects[2].effect_index, 2);
    assert_eq!(info.effects[2].difficulty_id, 231);
    assert_eq!(info.effects[2].effect_aura, 403);
    assert!(spells.store.may_override_spell_visual(0xC456_789A));

    let unvalidated_parent = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [serverside_spell_row(82_003, 999)],
        &ServersideSpellEffectStoreLikeCpp::default(),
        |_| false,
    );
    assert_eq!(unvalidated_parent.loaded_spell_count, 1);
    assert!(unvalidated_parent.errors.is_empty());
    assert!(unvalidated_parent
        .store
        .get_serverside_spell_like_cpp(82_003, 231)
        .is_some());
    assert!(unvalidated_parent
        .store
        .get_serverside_spell_like_cpp(82_003, 999)
        .is_none());
}

#[test]
fn server_visual_slots_follow_input_order_and_orphans_have_no_potential() {
    let target_spell_id = 82_004;
    let mut active = serverside_effect_row(82_005, 0);
    active.effect_aura = 403;
    active.effect_misc_value_1 = target_spell_id as i32;
    let mut inert = serverside_effect_row(82_005, 0);
    inert.effect = spell_effect_types::SPELL_EFFECT_NONE as i32;

    let active_then_inert = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
        [active.clone(), inert.clone()],
        |_| false,
        |_| true,
        |_| true,
    );
    let replaced = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [serverside_spell_row(82_005, 0)],
        &active_then_inert.store,
        |_| false,
    );
    assert_eq!(
        replaced
            .store
            .get_serverside_spell_like_cpp(82_005, 0)
            .unwrap()
            .effects[0]
            .effect,
        0
    );
    assert!(!replaced.store.may_override_spell_visual(target_spell_id));

    let inert_then_active = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
        [inert, active],
        |_| false,
        |_| true,
        |_| true,
    );
    let retained = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [serverside_spell_row(82_005, 0)],
        &inert_then_active.store,
        |_| false,
    );
    assert!(retained.store.may_override_spell_visual(target_spell_id));

    let orphan = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [],
        &inert_then_active.store,
        |_| false,
    );
    assert!(!orphan.store.may_override_spell_visual(target_spell_id));
}

#[test]
fn negative_server_effect_index_is_rejected_before_slot_construction() {
    let outcome = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
        [serverside_effect_row(82_006, -1)],
        |_| false,
        |_| true,
        |_| true,
    );

    assert_eq!(outcome.loaded_effect_count, 0);
    assert_eq!(outcome.errors.len(), 1);
    assert_eq!(
        outcome.errors[0].kind,
        ServersideSpellEffectLoadErrorKindLikeCpp::EffectIndexOutOfRange
    );
    assert!(outcome.store.effects_by_spell_and_difficulty.is_empty());
}
