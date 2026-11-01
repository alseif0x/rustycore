//! Exact proc-source preparation scenarios.

use super::*;

fn keyed_spell_store(
    keys: impl IntoIterator<Item = (u32, u8)>,
    misc_rows: impl IntoIterator<Item = crate::spell_db2::SpellMiscEntry>,
    effect_rows: impl IntoIterator<Item = crate::spell_db2::SpellEffectDb2Entry>,
) -> SpellStore {
    let keys = keys.into_iter().collect::<Vec<_>>();
    let spell_name_ids = keys
        .iter()
        .map(|(spell_id, _)| *spell_id)
        .collect::<HashSet<_>>();
    let mut spells = SpellStore::from_spell_db2_stores_like_cpp(
        &crate::spell_db2::SpellCategoriesStore::from_entries([]),
        &crate::spell_db2::SpellMiscStore::from_entries(misc_rows),
        &crate::spell_db2::SpellEffectDb2Store::from_entries(effect_rows),
        &crate::spell_db2::SpellShapeshiftStore::from_entries([]),
    );
    spells.spell_info_keys_like_cpp =
        crate::spell_info_keys::SpellInfoKeyStoreLikeCpp::from_candidate_keys_like_cpp(
            keys,
            &spell_name_ids,
        );
    spells
}

fn proc_difficulties(
    entries: impl IntoIterator<Item = (u32, u8)>,
) -> crate::difficulty::DifficultyStore {
    crate::difficulty::DifficultyStore::from_entries(entries.into_iter().map(
        |(id, fallback_difficulty_id)| crate::difficulty::DifficultyEntry {
            id,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id,
            toggle_difficulty_id: 0,
        },
    ))
}

#[test]
fn proc_source_preparation_keeps_key_only_spell_infos() {
    let spells = keyed_spell_store([(702, 2)], [], []);
    let difficulties = proc_difficulties([(0, 2), (2, 0)]);
    let source = SpellProcSourceSpellInfoLikeCpp::from_loaded_spell_like_cpp(
        702,
        2,
        &spells,
        &difficulties,
        &SpellChainStoreLikeCpp::default(),
        &crate::spell_db2::SpellAuraOptionsStore::from_entries([]),
        &crate::spell_db2::SpellClassOptionsStore::from_entries([]),
        &crate::spell_db2::SpellProcsPerMinuteStore::from_entries([]),
    )
    .unwrap()
    .expect("an exact key remains a source without hydrated payload");

    assert!(spells.get(702).is_none());
    assert_eq!(source.spell_id, 702);
    assert_eq!(source.difficulty, 2);
    assert_eq!(source.proc_flags, [0, 0]);
    assert_eq!(source.proc_charges, 0);
    assert_eq!(source.attributes3, 0);
    assert!(source.effects.is_empty());

    let outcome = SpellProcStoreLikeCpp::from_rows_and_stores_like_cpp(
        [SpellProcRowLikeCpp {
            spell_id: 702,
            proc_flags: [PROC_FLAG_KILL_LIKE_CPP, 0],
            ..test_spell_proc_row_like_cpp(702)
        }],
        &spells,
        &ServersideSpellStoreLikeCpp::default(),
        &SpellChainStoreLikeCpp::default(),
        &difficulties,
        &crate::spell_db2::SpellAuraOptionsStore::from_entries([]),
        &crate::spell_db2::SpellClassOptionsStore::from_entries([]),
        &crate::spell_db2::SpellProcsPerMinuteStore::from_entries([]),
    )
    .unwrap();
    assert_eq!(outcome.errors.len(), 0);
    assert_eq!(outcome.loaded_row_count, 1);
    assert!(outcome.store.spell_proc_entry_like_cpp(702, 2).is_some());
}

#[test]
fn proc_source_preparation_uses_all_actual_difficulties_and_dense_effect_slots() {
    let spell_id = 703;
    let mut base_proc = test_spell_effect_db2_entry_like_cpp(
        1,
        spell_id,
        0,
        0,
        spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        0,
    );
    base_proc.effect_aura = aura_types::SPELL_AURA_PROC_TRIGGER_SPELL as i16;
    base_proc.effect_trigger_spell = 900;
    let mut base_later = base_proc.clone();
    base_later.id = 2;
    base_later.effect_index = 2;
    let mut heroic_inert = test_spell_effect_db2_entry_like_cpp(3, spell_id, 2, 0, 0, 0);
    heroic_inert.effect_base_points = 12;

    let spells = keyed_spell_store(
        [(spell_id, 0), (spell_id, 2)],
        [],
        [base_proc, base_later, heroic_inert],
    );
    let difficulties = proc_difficulties([(2, 0)]);
    let aura_options = crate::spell_db2::SpellAuraOptionsStore::from_entries([
        test_spell_aura_options_entry_like_cpp(
            1,
            spell_id,
            0,
            [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP as i32, 0],
            15,
            1,
            0,
            0,
        ),
        test_spell_aura_options_entry_like_cpp(
            2,
            spell_id,
            2,
            [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP as i32, 0],
            35,
            2,
            0,
            0,
        ),
    ]);
    let source = SpellProcSourceSpellInfoLikeCpp::from_loaded_spell_like_cpp(
        spell_id,
        2,
        &spells,
        &difficulties,
        &SpellChainStoreLikeCpp::default(),
        &aura_options,
        &crate::spell_db2::SpellClassOptionsStore::from_entries([]),
        &crate::spell_db2::SpellProcsPerMinuteStore::from_entries([]),
    )
    .unwrap()
    .unwrap();

    assert_eq!(source.difficulty, 2);
    assert_eq!(source.proc_chance, 35.0);
    assert_eq!(source.proc_charges, 2);
    assert_eq!(source.effects.len(), 3);
    assert_eq!(
        source.effects[0].effect, 0,
        "the exact inert row stops fallback"
    );
    assert_eq!(source.effects[0].effect_base_points, 12);
    assert_eq!(
        source.effects[1].effect_index, 1,
        "the missing slot is dense zero"
    );
    assert_eq!(source.effects[1].effect, 0);
    assert_eq!(
        source.effects[2].effect_aura,
        aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
        "the unoverridden slot follows the actual difficulty chain"
    );

    let outcome = SpellProcStoreLikeCpp::from_rows_and_stores_like_cpp(
        [],
        &spells,
        &ServersideSpellStoreLikeCpp::default(),
        &SpellChainStoreLikeCpp::default(),
        &difficulties,
        &aura_options,
        &crate::spell_db2::SpellClassOptionsStore::from_entries([]),
        &crate::spell_db2::SpellProcsPerMinuteStore::from_entries([]),
    )
    .unwrap();
    assert_eq!(outcome.generated_entry_count, 2);
    assert!(
        outcome
            .store
            .spell_proc_entry_like_cpp(spell_id, 0)
            .is_some()
    );
    assert!(
        outcome
            .store
            .spell_proc_entry_like_cpp(spell_id, 2)
            .is_some()
    );
}

#[test]
fn proc_source_preparation_resolves_explicit_rows_and_rank_fallback_from_zero() {
    let spells = keyed_spell_store([(730, 2), (731, 2)], [], []);
    let difficulties = proc_difficulties([(0, 2), (2, 0)]);
    let chains = SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
        [SpellRankEdgeLikeCpp {
            spell_id: 731,
            supercedes_spell_id: 730,
        }],
        |_| true,
    );
    let rows = [SpellProcRowLikeCpp {
        spell_id: -730,
        proc_flags: [PROC_FLAG_KILL_LIKE_CPP, 0],
        ..test_spell_proc_row_like_cpp(-730)
    }];
    let outcome = SpellProcStoreLikeCpp::from_rows_and_stores_like_cpp(
        rows,
        &spells,
        &ServersideSpellStoreLikeCpp::default(),
        &chains,
        &difficulties,
        &crate::spell_db2::SpellAuraOptionsStore::from_entries([]),
        &crate::spell_db2::SpellClassOptionsStore::from_entries([]),
        &crate::spell_db2::SpellProcsPerMinuteStore::from_entries([]),
    )
    .unwrap();

    assert!(outcome.errors.is_empty());
    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(outcome.generated_entry_count, 0);
    assert_eq!(
        outcome
            .store
            .spell_proc_entry_like_cpp(730, 2)
            .unwrap()
            .proc_flags,
        [PROC_FLAG_KILL_LIKE_CPP, 0]
    );
    assert_eq!(
        outcome
            .store
            .spell_proc_entry_like_cpp(731, 2)
            .unwrap()
            .proc_flags,
        [PROC_FLAG_KILL_LIKE_CPP, 0]
    );
    assert!(outcome.store.spell_proc_entry_like_cpp(730, 0).is_none());
}

#[test]
fn proc_source_preparation_uses_highest_scalar_records_and_effective_ppm() {
    let spell_id = 740;
    let mut effect = test_spell_effect_db2_entry_like_cpp(
        1,
        spell_id,
        0,
        0,
        spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        0,
    );
    effect.effect_aura = aura_types::SPELL_AURA_PROC_TRIGGER_SPELL as i16;
    effect.effect_trigger_spell = 901;
    effect.effect_spell_class_mask = [0x80, 0, 0, 0];
    let spells = keyed_spell_store(
        [(spell_id, 0)],
        [test_spell_misc_entry_like_cpp(
            1,
            spell_id,
            0,
            attributes::SPELL_ATTR3_CAN_PROC_FROM_PROCS,
        )],
        [effect],
    );
    let aura_options = crate::spell_db2::SpellAuraOptionsStore::from_entries([
        test_spell_aura_options_entry_like_cpp(3, spell_id, 0, [0, 0], 1, 9, 8, 0),
        test_spell_aura_options_entry_like_cpp(
            10,
            spell_id,
            0,
            [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP as i32, 0],
            100,
            0,
            0,
            55,
        ),
    ]);
    let class_options = crate::spell_db2::SpellClassOptionsStore::from_entries([
        crate::spell_db2::SpellClassOptionsEntry {
            id: 1,
            spell_id: spell_id as i32,
            modal_next_spell: 0,
            spell_class_set: 3,
            spell_class_mask: [1, 0, 0, 0],
        },
        crate::spell_db2::SpellClassOptionsEntry {
            id: 9,
            spell_id: spell_id as i32,
            modal_next_spell: 0,
            spell_class_set: 17,
            spell_class_mask: [2, 0, 0, 0],
        },
    ]);
    let ppm = crate::spell_db2::SpellProcsPerMinuteStore::from_entries([
        crate::spell_db2::SpellProcsPerMinuteEntry {
            id: 55,
            base_proc_rate: 1.75,
            flags: 0,
        },
    ]);
    let outcome = SpellProcStoreLikeCpp::from_rows_and_stores_like_cpp(
        [],
        &spells,
        &ServersideSpellStoreLikeCpp::default(),
        &SpellChainStoreLikeCpp::default(),
        &proc_difficulties([]),
        &aura_options,
        &class_options,
        &ppm,
    )
    .unwrap();

    assert_eq!(outcome.generated_entry_count, 1);
    let entry = outcome
        .store
        .spell_proc_entry_like_cpp(spell_id, 0)
        .unwrap();
    assert_eq!(entry.proc_flags, [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0]);
    assert_eq!(entry.chance, 100.0);
    assert_eq!(entry.spell_family_name, 17);
    assert_eq!(entry.spell_family_mask, [0x80, 0, 0, 0]);
}

#[test]
fn proc_source_preparation_preserves_high_id_attributes_and_trigger_bits() {
    let high_id = 0x8000_0040;
    let zero_id = high_id + 1;
    let mut high_trigger = test_spell_effect_db2_entry_like_cpp(
        1,
        high_id,
        0,
        0,
        spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        0,
    );
    high_trigger.effect_aura = aura_types::SPELL_AURA_PROC_TRIGGER_SPELL as i16;
    high_trigger.effect_trigger_spell = i32::MIN;
    let mut zero_trigger = high_trigger.clone();
    zero_trigger.id = 2;
    zero_trigger.spell_id = zero_id;
    zero_trigger.effect_trigger_spell = 0;
    let spells = keyed_spell_store(
        [(high_id, 0), (zero_id, 0)],
        [
            test_spell_misc_entry_like_cpp(
                1,
                high_id,
                0,
                attributes::SPELL_ATTR3_CAN_PROC_FROM_PROCS,
            ),
            test_spell_misc_entry_like_cpp(
                2,
                zero_id,
                0,
                attributes::SPELL_ATTR3_CAN_PROC_FROM_PROCS,
            ),
        ],
        [high_trigger, zero_trigger],
    );
    let aura_options = crate::spell_db2::SpellAuraOptionsStore::from_entries([
        test_spell_aura_options_entry_like_cpp(
            1,
            high_id,
            0,
            [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP as i32, 0],
            100,
            0,
            0,
            0,
        ),
        test_spell_aura_options_entry_like_cpp(
            2,
            zero_id,
            0,
            [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP as i32, 0],
            100,
            0,
            0,
            0,
        ),
    ]);
    let difficulties = proc_difficulties([]);
    let chains = SpellChainStoreLikeCpp::default();
    let class_options = crate::spell_db2::SpellClassOptionsStore::from_entries([]);
    let ppm = crate::spell_db2::SpellProcsPerMinuteStore::from_entries([]);
    let high_source = SpellProcSourceSpellInfoLikeCpp::from_loaded_spell_like_cpp(
        high_id,
        0,
        &spells,
        &difficulties,
        &chains,
        &aura_options,
        &class_options,
        &ppm,
    )
    .unwrap()
    .unwrap();
    let zero_source = SpellProcSourceSpellInfoLikeCpp::from_loaded_spell_like_cpp(
        zero_id,
        0,
        &spells,
        &difficulties,
        &chains,
        &aura_options,
        &class_options,
        &ppm,
    )
    .unwrap()
    .unwrap();
    let high_implicit = high_source.implicit_proc_source_like_cpp();
    let zero_implicit = zero_source.implicit_proc_source_like_cpp();

    assert_eq!(
        high_source.attributes3,
        attributes::SPELL_ATTR3_CAN_PROC_FROM_PROCS
    );
    assert_eq!(high_source.effects[0].effect_trigger_spell, i32::MIN);
    assert_eq!(high_implicit.effects[0].trigger_spell, 0x8000_0000);
    assert_eq!(zero_implicit.effects[0].trigger_spell, 0);
    assert!(
        implicit_spell_proc_entry_like_cpp(&high_implicit).is_none(),
        "the preserved nonzero trigger and attributes activate the PPM safety guard"
    );
    assert!(implicit_spell_proc_entry_like_cpp(&zero_implicit).is_some());
}

#[test]
fn proc_source_preparation_emits_ppm_guard_entry_that_stops_fallback() {
    let spell_id = 770;
    let mut base_effect = test_spell_effect_db2_entry_like_cpp(
        1,
        spell_id,
        0,
        0,
        spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        0,
    );
    base_effect.effect_aura = aura_types::SPELL_AURA_PROC_TRIGGER_SPELL as i16;
    base_effect.effect_trigger_spell = 900;
    let mut later_effect = base_effect.clone();
    later_effect.id = 2;
    later_effect.difficulty_id = 1;
    let spells = keyed_spell_store(
        [(spell_id, 0), (spell_id, 1)],
        [
            test_spell_misc_entry_like_cpp(
                1,
                spell_id,
                0,
                attributes::SPELL_ATTR3_CAN_PROC_FROM_PROCS,
            ),
            test_spell_misc_entry_like_cpp(2, spell_id, 1, 0),
        ],
        [base_effect, later_effect],
    );
    let aura_options = crate::spell_db2::SpellAuraOptionsStore::from_entries([
        test_spell_aura_options_entry_like_cpp(
            1,
            spell_id,
            0,
            [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP as i32, 0],
            100,
            0,
            0,
            77,
        ),
        test_spell_aura_options_entry_like_cpp(
            2,
            spell_id,
            1,
            [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP as i32, 0],
            25,
            0,
            0,
            0,
        ),
    ]);
    let ppm = crate::spell_db2::SpellProcsPerMinuteStore::from_entries([
        crate::spell_db2::SpellProcsPerMinuteEntry {
            id: 77,
            base_proc_rate: 1.5,
            flags: 0,
        },
    ]);
    let outcome = SpellProcStoreLikeCpp::from_rows_and_stores_like_cpp(
        [],
        &spells,
        &ServersideSpellStoreLikeCpp::default(),
        &SpellChainStoreLikeCpp::default(),
        &proc_difficulties([(1, 0)]),
        &aura_options,
        &crate::spell_db2::SpellClassOptionsStore::from_entries([]),
        &ppm,
    )
    .unwrap();

    assert_eq!(outcome.generated_entry_count, 2);
    let exact = outcome
        .store
        .spell_proc_entry_like_cpp(spell_id, 0)
        .unwrap();
    let fallback = outcome
        .store
        .spell_proc_entry_like_cpp(spell_id, 1)
        .unwrap();
    assert_eq!(exact.chance, 100.0);
    assert_eq!(exact.charges, 0);
    assert_eq!(exact.procs_per_minute, 0.0);
    assert_eq!(fallback.chance, 25.0);
    assert_eq!(
        outcome
            .store
            .spell_proc_entry_with_fallback_like_cpp(spell_id, 0, |_| Some(1))
            .unwrap()
            .chance,
        100.0,
        "the exact generated entry wins even though the fallback has a nonzero chance"
    );
}

#[test]
fn proc_source_preparation_rejects_invalid_effect_shape_before_loading() {
    let spell_id = 780;
    let mut spells = keyed_spell_store([(spell_id, 0)], [], []);
    spells.insert(
        spell_id as i32,
        SpellInfo {
            effects: vec![
                SpellEffectInfo {
                    effect_index: 0,
                    effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
                    ..SpellEffectInfo::default()
                },
                SpellEffectInfo {
                    effect_index: 0,
                    effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
                    ..SpellEffectInfo::default()
                },
            ],
            ..SpellStore::empty_spell_info_like_cpp(spell_id as i32)
        },
    );
    let result = SpellProcStoreLikeCpp::from_rows_and_stores_like_cpp(
        [],
        &spells,
        &ServersideSpellStoreLikeCpp::default(),
        &SpellChainStoreLikeCpp::default(),
        &proc_difficulties([]),
        &crate::spell_db2::SpellAuraOptionsStore::from_entries([]),
        &crate::spell_db2::SpellClassOptionsStore::from_entries([]),
        &crate::spell_db2::SpellProcsPerMinuteStore::from_entries([]),
    );

    assert!(result.is_err());
}

#[test]
fn proc_source_preparation_uses_accepted_server_spell_scalars_and_effects() {
    let spell_id = 790;
    let mut effect = serverside_effect_row(spell_id, 0);
    effect.difficulty_id = 2;
    effect.effect_aura = aura_types::SPELL_AURA_MOD_HIT_CHANCE;
    effect.effect_base_points = -99.9;
    effect.effect_spell_class_mask = [i32::MIN, 0, 0, 0];
    let effect_outcome = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
        [effect],
        |_| false,
        |difficulty| difficulty == 2,
        |_| true,
    );
    assert!(effect_outcome.errors.is_empty());
    let mut server_row = serverside_spell_row(spell_id, 2);
    server_row.proc_flags = [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0];
    server_row.proc_chance = 50;
    server_row.proc_charges = 0x1_0001;
    server_row.proc_cooldown = 321;
    server_row.proc_base_ppm = 2.5;
    server_row.spell_family_name = 0x1_0001;
    let server_store = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [server_row],
        &effect_outcome.store,
        |_| false,
    )
    .store;
    let outcome = SpellProcStoreLikeCpp::from_rows_and_stores_like_cpp(
        [],
        &SpellStore::new(),
        &server_store,
        &SpellChainStoreLikeCpp::default(),
        &proc_difficulties([]),
        &crate::spell_db2::SpellAuraOptionsStore::from_entries([]),
        &crate::spell_db2::SpellClassOptionsStore::from_entries([]),
        &crate::spell_db2::SpellProcsPerMinuteStore::from_entries([]),
    )
    .unwrap();

    assert_eq!(outcome.generated_entry_count, 1);
    let entry = outcome
        .store
        .spell_proc_entry_like_cpp(spell_id, 2)
        .unwrap();
    assert_eq!(entry.proc_flags, [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0]);
    assert_eq!(entry.chance, 50.0);
    assert_eq!(entry.charges, 0x1_0001);
    assert_eq!(entry.cooldown_ms, 321);
    assert_eq!(entry.spell_family_name, 0x1_0001);
    assert_eq!(entry.spell_family_mask, [0x8000_0000, 0, 0, 0]);
    assert_eq!(
        entry.hit_mask, 0,
        "-99.9 truncates to -99 before the hit test"
    );

    let explicit = SpellProcStoreLikeCpp::from_rows_and_stores_like_cpp(
        [test_spell_proc_row_like_cpp(spell_id as i32)],
        &SpellStore::new(),
        &server_store,
        &SpellChainStoreLikeCpp::default(),
        &proc_difficulties([(0, 2), (2, 0)]),
        &crate::spell_db2::SpellAuraOptionsStore::from_entries([]),
        &crate::spell_db2::SpellClassOptionsStore::from_entries([]),
        &crate::spell_db2::SpellProcsPerMinuteStore::from_entries([]),
    )
    .unwrap();
    let explicit_entry = explicit
        .store
        .spell_proc_entry_like_cpp(spell_id, 2)
        .unwrap();
    assert_eq!(
        explicit_entry.proc_flags,
        [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0]
    );
    assert_eq!(explicit_entry.charges, 0x1_0001);
    assert_eq!(explicit_entry.cooldown_ms, 321);
}

#[test]
fn proc_source_preparation_rejects_overlapping_producers() {
    let spell_id = 791;
    let spells = keyed_spell_store([(spell_id, 0)], [], []);
    let server_store = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [serverside_spell_row(spell_id, 0)],
        &ServersideSpellEffectStoreLikeCpp::default(),
        |_| false,
    )
    .store;
    let result = SpellProcStoreLikeCpp::from_rows_and_stores_like_cpp(
        [],
        &spells,
        &server_store,
        &SpellChainStoreLikeCpp::default(),
        &proc_difficulties([]),
        &crate::spell_db2::SpellAuraOptionsStore::from_entries([]),
        &crate::spell_db2::SpellClassOptionsStore::from_entries([]),
        &crate::spell_db2::SpellProcsPerMinuteStore::from_entries([]),
    );

    assert!(result.is_err());
}

#[test]
fn proc_source_preparation_rejects_cyclic_required_difficulty_before_loading() {
    let spell_id = 781;
    let spells = keyed_spell_store([(spell_id, 2)], [], []);
    let result = SpellProcStoreLikeCpp::from_rows_and_stores_like_cpp(
        [],
        &spells,
        &ServersideSpellStoreLikeCpp::default(),
        &SpellChainStoreLikeCpp::default(),
        &proc_difficulties([(1, 2), (2, 1)]),
        &crate::spell_db2::SpellAuraOptionsStore::from_entries([
            test_spell_aura_options_entry_like_cpp(1, spell_id, 0, [1, 0], 10, 0, 0, 0),
        ]),
        &crate::spell_db2::SpellClassOptionsStore::from_entries([]),
        &crate::spell_db2::SpellProcsPerMinuteStore::from_entries([]),
    );

    assert!(result.is_err());
}

#[test]
fn proc_source_preparation_keeps_absent_spell_lookup_out_of_cyclic_difficulties() {
    let spells = keyed_spell_store([(7900, 0)], [], []);
    let result = SpellProcStoreLikeCpp::from_rows_and_stores_like_cpp(
        [test_spell_proc_row_like_cpp(7901)],
        &spells,
        &ServersideSpellStoreLikeCpp::default(),
        &SpellChainStoreLikeCpp::default(),
        &proc_difficulties([(0, 1), (1, 0)]),
        &crate::spell_db2::SpellAuraOptionsStore::from_entries([]),
        &crate::spell_db2::SpellClassOptionsStore::from_entries([]),
        &crate::spell_db2::SpellProcsPerMinuteStore::from_entries([]),
    )
    .expect("an absent spell is known absent without traversing the difficulty graph");

    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.errors[0].spell_id, 7901);
    assert_eq!(
        result.errors[0].kind,
        SpellProcLoadErrorKindLikeCpp::SpellMissing
    );
}

#[test]
fn proc_source_preparation_rejects_needed_cycle_when_source_is_only_outside_chain() {
    let spell_id = 7902;
    let spells = keyed_spell_store([(spell_id, 2)], [], []);
    let result = SpellProcStoreLikeCpp::from_rows_and_stores_like_cpp(
        [test_spell_proc_row_like_cpp(spell_id as i32)],
        &spells,
        &ServersideSpellStoreLikeCpp::default(),
        &SpellChainStoreLikeCpp::default(),
        &proc_difficulties([(0, 1), (1, 0)]),
        &crate::spell_db2::SpellAuraOptionsStore::from_entries([]),
        &crate::spell_db2::SpellClassOptionsStore::from_entries([]),
        &crate::spell_db2::SpellProcsPerMinuteStore::from_entries([]),
    );

    assert!(result.is_err());
}

#[test]
fn proc_source_preparation_does_not_resolve_ranks_after_nonfirst_all_ranks_row() {
    let spells = keyed_spell_store([(7903, 0), (7904, 0), (7905, 2)], [], []);
    let chains = SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
        [
            SpellRankEdgeLikeCpp {
                spell_id: 7904,
                supercedes_spell_id: 7903,
            },
            SpellRankEdgeLikeCpp {
                spell_id: 7905,
                supercedes_spell_id: 7904,
            },
        ],
        |_| true,
    );
    let result = SpellProcStoreLikeCpp::from_rows_and_stores_like_cpp(
        [test_spell_proc_row_like_cpp(-7904)],
        &spells,
        &ServersideSpellStoreLikeCpp::default(),
        &chains,
        &proc_difficulties([(0, 1), (1, 0)]),
        &crate::spell_db2::SpellAuraOptionsStore::from_entries([]),
        &crate::spell_db2::SpellClassOptionsStore::from_entries([]),
        &crate::spell_db2::SpellProcsPerMinuteStore::from_entries([]),
    )
    .expect("the production loader stops when an all-ranks row is not the first rank");

    assert_eq!(result.errors.len(), 1);
    assert_eq!(
        result.errors[0].kind,
        SpellProcLoadErrorKindLikeCpp::AllRanksSpellNotFirstRank
    );
    assert_eq!(result.loaded_row_count, 0);
}
