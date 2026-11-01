use super::*;

const SPELL_ID: u32 = 73_101;

fn visual_row(
    id: u32,
    spell_id: u32,
    difficulty_id: u8,
    caster_player_condition_id: u32,
    caster_unit_condition_id: u16,
) -> crate::spell_db2::SpellXSpellVisualEntry {
    crate::spell_db2::SpellXSpellVisualEntry {
        id,
        difficulty_id,
        spell_visual_id: id + 1000,
        probability: 1.0,
        flags: 0,
        priority: 0,
        spell_icon_file_id: 0,
        active_icon_file_id: 0,
        viewer_unit_condition_id: 0,
        viewer_player_condition_id: 0,
        caster_unit_condition_id,
        caster_player_condition_id,
        spell_id,
    }
}

fn difficulty(id: u32, fallback_difficulty_id: u8) -> crate::difficulty::DifficultyEntry {
    crate::difficulty::DifficultyEntry {
        id,
        instance_type: 0,
        flags: 0,
        fallback_difficulty_id,
        toggle_difficulty_id: 0,
    }
}

fn spell_store_from_effect_rows(
    rows: impl IntoIterator<Item = crate::spell_db2::SpellEffectDb2Entry>,
) -> SpellStore {
    SpellStore::from_spell_db2_stores_like_cpp(
        &crate::spell_db2::SpellCategoriesStore::from_entries([]),
        &crate::spell_db2::SpellMiscStore::from_entries([]),
        &crate::spell_db2::SpellEffectDb2Store::from_entries(rows),
        &crate::spell_db2::SpellShapeshiftStore::from_entries([]),
    )
}

fn override_effect(
    record_id: u32,
    source_spell_id: u32,
    difficulty_id: i32,
    effect_index: i32,
    effect_type: u32,
    target_spell_id: u32,
) -> crate::spell_db2::SpellEffectDb2Entry {
    let mut row = test_spell_effect_db2_entry_like_cpp(
        record_id,
        source_spell_id,
        difficulty_id,
        effect_index,
        effect_type,
        0,
    );
    row.effect_aura = 403;
    row.effect_misc_value[0] = target_spell_id as i32;
    row
}

#[test]
fn visual_metadata_resolution_uses_the_first_reachable_whole_vector() {
    let difficulties = crate::difficulty::DifficultyStore::from_entries([
        difficulty(4, 3),
        difficulty(3, 0),
        difficulty(0, 0),
    ]);
    let fallback_store =
        crate::spell_db2::SpellXSpellVisualStore::from_entries([visual_row(10, SPELL_ID, 0, 0, 0)]);
    let fallback = resolve_spell_visual_rows(&fallback_store, SPELL_ID, 4, Some(&difficulties))
        .expect("4 -> 3 -> 0 fallback resolves");
    assert_eq!(fallback.iter().map(|row| row.id).collect::<Vec<_>>(), [10]);

    let whole_vector_store = crate::spell_db2::SpellXSpellVisualStore::from_entries([
        visual_row(10, SPELL_ID, 0, 0, 0),
        visual_row(30, SPELL_ID, 3, 1, 0),
        visual_row(31, SPELL_ID, 3, 2, 0),
    ]);
    let rows = resolve_spell_visual_rows(&whole_vector_store, SPELL_ID, 4, Some(&difficulties))
        .expect("the first nonempty difficulty resolves");
    assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), [31, 30]);

    let zero_fallback = crate::difficulty::DifficultyStore::from_entries([difficulty(0, 3)]);
    let only_three =
        crate::spell_db2::SpellXSpellVisualStore::from_entries([visual_row(31, SPELL_ID, 3, 0, 0)]);
    let rows = resolve_spell_visual_rows(&only_three, SPELL_ID, 0, Some(&zero_fallback))
        .expect("difficulty zero follows its actual fallback");
    assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), [31]);

    let zero_overlay = crate::spell_db2::SpellXSpellVisualStore::from_entries([
        visual_row(10, SPELL_ID, 0, 0, 0),
        visual_row(31, SPELL_ID, 3, 0, 0),
    ]);
    let rows = resolve_spell_visual_rows(&zero_overlay, SPELL_ID, 0, Some(&zero_fallback))
        .expect("the exact difficulty-zero vector is the overlay");
    assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), [10]);
}

#[test]
fn visual_metadata_resolution_distinguishes_no_rows_from_needed_graph_errors() {
    let empty = crate::spell_db2::SpellXSpellVisualStore::from_entries([]);
    assert!(
        resolve_spell_visual_rows(&empty, SPELL_ID, 4, None)
            .expect("known no rows do not need a difficulty graph")
            .is_empty()
    );

    let rows =
        crate::spell_db2::SpellXSpellVisualStore::from_entries([visual_row(1, SPELL_ID, 0, 0, 0)]);
    assert_eq!(
        resolve_spell_visual_rows(&rows, SPELL_ID, 4, None),
        Err(DataSpellCatalogResolutionError::DifficultyUnavailable)
    );

    let cycle =
        crate::difficulty::DifficultyStore::from_entries([difficulty(4, 5), difficulty(5, 4)]);
    assert_eq!(
        resolve_spell_visual_rows(&rows, SPELL_ID, 4, Some(&cycle)),
        Err(DataSpellCatalogResolutionError::DifficultyCycle)
    );
}

#[test]
fn visual_metadata_order_uses_condition_then_record_id_only() {
    let mut older_equal_condition = visual_row(10, SPELL_ID, 0, 50, 0);
    older_equal_condition.probability = 0.0;
    older_equal_condition.flags = u8::MAX;
    older_equal_condition.priority = i32::MIN;
    older_equal_condition.viewer_unit_condition_id = 7;
    older_equal_condition.viewer_player_condition_id = 8;

    let mut newer_equal_condition = visual_row(11, SPELL_ID, 0, 50, 0);
    newer_equal_condition.probability = 0.25;
    newer_equal_condition.flags = 0;
    newer_equal_condition.priority = i32::MAX;
    newer_equal_condition.viewer_unit_condition_id = 0;
    newer_equal_condition.viewer_player_condition_id = 0;

    let store = crate::spell_db2::SpellXSpellVisualStore::from_entries([
        visual_row(20, SPELL_ID, 0, 3, 0),
        older_equal_condition,
        newer_equal_condition,
    ]);
    let rows = resolve_spell_visual_rows(&store, SPELL_ID, 0, None)
        .expect("the exact visual vector resolves without unrelated graph data");
    assert_eq!(
        rows.iter().map(|row| row.id).collect::<Vec<_>>(),
        [11, 10, 20]
    );
}

#[test]
fn visual_metadata_selection_uses_effective_presence_and_reports_needed_predicates() {
    let rows = [
        visual_row(1, SPELL_ID, 0, 71, 72),
        visual_row(2, SPELL_ID, 0, 0, 0),
    ];
    let absent_records = crate::ConditionPresenceCatalog::from_effective_ids([], []);
    assert_eq!(
        select_spell_visual_id(
            &[&rows[0]],
            Some(&absent_records),
            SpellVisualCasterKind::Player
        ),
        Ok(1),
        "nonzero condition IDs whose records are absent do not constrain selection"
    );
    assert_eq!(
        select_spell_visual_id(&[], None, SpellVisualCasterKind::NoCaster),
        Ok(0)
    );
    assert_eq!(
        select_spell_visual_id(&[&rows[0]], None, SpellVisualCasterKind::Player),
        Err(SpellVisualSelectionError::ConditionPresenceUnavailable)
    );

    let zero_player = crate::ConditionPresenceCatalog::from_effective_ids([0], []);
    assert_eq!(
        select_spell_visual_id(
            &[&rows[1]],
            Some(&zero_player),
            SpellVisualCasterKind::Player
        ),
        Err(SpellVisualSelectionError::PlayerConditionEvaluationUnavailable { condition_id: 0 }),
        "ID zero is a present record when the effective catalog contains it"
    );
    assert_eq!(
        select_spell_visual_id(
            &[&rows[1], &rows[0]],
            Some(&zero_player),
            SpellVisualCasterKind::OtherUnit,
        ),
        Ok(1),
        "a present player condition skips for a non-player caster"
    );

    let zero_unit = crate::ConditionPresenceCatalog::from_effective_ids([], [0]);
    assert_eq!(
        select_spell_visual_id(&[&rows[1]], Some(&zero_unit), SpellVisualCasterKind::Player),
        Err(SpellVisualSelectionError::UnitConditionEvaluationUnavailable { condition_id: 0 })
    );
    assert_eq!(
        select_spell_visual_id(
            &[&rows[1], &rows[0]],
            Some(&zero_unit),
            SpellVisualCasterKind::NoCaster,
        ),
        Ok(1),
        "a present unit condition with ID zero skips for no caster"
    );

    let known_constraints = crate::ConditionPresenceCatalog::from_effective_ids([7], [9]);
    let constrained = [
        visual_row(3, SPELL_ID, 0, 7, 9),
        visual_row(4, SPELL_ID, 0, 80, 90),
    ];
    assert_eq!(
        select_spell_visual_id(
            &[&constrained[0], &constrained[1]],
            Some(&known_constraints),
            SpellVisualCasterKind::NoCaster,
        ),
        Ok(4),
        "no caster skips present player and unit constraints"
    );
    assert_eq!(
        select_spell_visual_id(
            &[&constrained[0], &constrained[1]],
            Some(&known_constraints),
            SpellVisualCasterKind::Player,
        ),
        Err(SpellVisualSelectionError::PlayerConditionEvaluationUnavailable { condition_id: 7 }),
        "a needed predicate errors before a lower unconditional row"
    );

    let unit_constrained = [
        visual_row(6, SPELL_ID, 0, 80, 9),
        visual_row(7, SPELL_ID, 0, 81, 90),
    ];
    assert_eq!(
        select_spell_visual_id(
            &[&unit_constrained[0], &unit_constrained[1]],
            Some(&known_constraints),
            SpellVisualCasterKind::NoCaster,
        ),
        Ok(7),
        "no caster skips a present unit condition when the player condition is absent"
    );

    let only_constraints = [visual_row(5, SPELL_ID, 0, 7, 0)];
    assert_eq!(
        select_spell_visual_id(
            &[&only_constraints[0]],
            Some(&known_constraints),
            SpellVisualCasterKind::OtherUnit,
        ),
        Ok(0),
        "known but caster-incompatible rows produce no visual"
    );
}

#[test]
fn visual_metadata_regular_override_potential_keeps_raw_winners_and_all_aura_carriers() {
    let high_target = 0xF123_4567;
    let area_target = 20_584;
    let store = spell_store_from_effect_rows([
        override_effect(
            10,
            0xF000_0001,
            300,
            4,
            spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            high_target,
        ),
        override_effect(
            20,
            100,
            0,
            0,
            spell_effect_types::SPELL_EFFECT_PERSISTENT_AREA_AURA,
            area_target,
        ),
    ]);
    assert!(store.may_override_spell_visual(high_target));
    assert!(store.may_override_spell_visual(area_target));
    assert!(!store.may_override_spell_visual(73_199));

    let shadowed_target = 8_326;
    let mut inert_winner = override_effect(
        31,
        200,
        0,
        0,
        spell_effect_types::SPELL_EFFECT_NONE,
        shadowed_target,
    );
    inert_winner.effect = 0;
    let shadowed = spell_store_from_effect_rows([
        override_effect(
            30,
            200,
            0,
            0,
            spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            shadowed_target,
        ),
        inert_winner,
    ]);
    assert!(
        !shadowed.may_override_spell_visual(shadowed_target),
        "the highest-ID inert slot winner suppresses the older active aura"
    );
}

#[test]
fn visual_metadata_regular_override_potential_reads_dynamic_payloads_and_legacy_primary() {
    let explicit_target = 0x8123_4567;
    let mut store = SpellStore::new();
    let mut explicit = test_spell_info_without_aura(73_201);
    explicit.effects = vec![SpellEffectInfo {
        effect: spell_effect_types::SPELL_EFFECT_PERSISTENT_AREA_AURA,
        effect_aura: 403,
        effect_misc_value_1: explicit_target as i32,
        ..SpellEffectInfo::default()
    }];
    store.insert(explicit.spell_id, explicit);
    assert!(store.may_override_spell_visual(explicit_target));

    let mut legacy = test_spell_info_with_aura(73_202, 403);
    legacy.effects.clear();
    store.insert(legacy.spell_id, legacy);
    assert!(store.may_override_spell_visual(73_203));
}

#[test]
fn visual_metadata_serverside_effect_admission_uses_regular_spell_info_keys() {
    let regular_spell_id = 73_303;
    let orphan_name_spell_id = 73_304;
    let names = crate::spell_db2::SpellNameStore::from_entries([
        crate::spell_db2::SpellNameEntry {
            id: regular_spell_id,
            name: "regular".to_string(),
        },
        crate::spell_db2::SpellNameEntry {
            id: orphan_name_spell_id,
            name: "name-only orphan".to_string(),
        },
    ]);
    assert!(names.get(orphan_name_spell_id).is_some());

    let effective_name_ids =
        std::collections::HashSet::from([regular_spell_id, orphan_name_spell_id]);
    let mut regular_spells = SpellStore::new();
    regular_spells.spell_info_keys_like_cpp =
        crate::spell_info_keys::SpellInfoKeyStoreLikeCpp::from_candidate_keys_like_cpp(
            [(regular_spell_id, 0)],
            &effective_name_ids,
        );
    assert!(regular_spells.contains_spell_info_any_difficulty_like_cpp(regular_spell_id));
    assert!(!regular_spells.contains_spell_info_any_difficulty_like_cpp(orphan_name_spell_id));

    let outcome = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
        [
            serverside_effect_row(regular_spell_id, 0),
            serverside_effect_row(orphan_name_spell_id, 0),
        ],
        |spell_id| regular_spells.contains_spell_info_any_difficulty_like_cpp(spell_id),
        |_| true,
        |_| true,
    );
    assert_eq!(outcome.loaded_effect_count, 1);
    assert_eq!(outcome.errors.len(), 1);
    assert_eq!(
        outcome.errors[0].kind,
        ServersideSpellEffectLoadErrorKindLikeCpp::RegularSpellAlreadyLoaded
    );
    assert!(
        outcome
            .store
            .effects_for_spell_difficulty_like_cpp(orphan_name_spell_id, 0)
            .is_some()
    );
}

#[test]
fn visual_metadata_serverside_override_potential_requires_an_accepted_spell_payload() {
    let target_spell_id = 15_007;
    let mut effect = serverside_effect_row(73_301, 0);
    effect.effect = spell_effect_types::SPELL_EFFECT_APPLY_AURA as i32;
    effect.effect_aura = 403;
    effect.effect_misc_value_1 = target_spell_id as i32;
    let effects = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
        [effect],
        |_| false,
        |_| true,
        |_| true,
    );

    let accepted = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [serverside_spell_row(73_301, 0)],
        &effects.store,
        |_| false,
    )
    .store;
    assert!(accepted.may_override_spell_visual(target_spell_id));
    assert!(!ServersideSpellStoreLikeCpp::default().may_override_spell_visual(target_spell_id));

    let orphan_effects =
        ServersideSpellStoreLikeCpp::from_rows_like_cpp([], &effects.store, |_| false).store;
    assert!(
        !orphan_effects.may_override_spell_visual(target_spell_id),
        "effect rows without an accepted serverside_spell row are not authority"
    );

    let accepted_without_effect = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [serverside_spell_row(73_302, 0)],
        &ServersideSpellEffectStoreLikeCpp::default(),
        |_| false,
    )
    .store;
    assert!(!accepted_without_effect.may_override_spell_visual(target_spell_id));
}

#[test]
#[ignore = "requires installed 3.4.3 DB2 files in RUSTYCORE_VISUAL_DATA_DIR"]
fn visual_file_diagnostic() {
    let data_dir = std::env::var("RUSTYCORE_VISUAL_DATA_DIR")
        .expect("set RUSTYCORE_VISUAL_DATA_DIR to the installed data directory");
    let effects = crate::spell_db2::SpellEffectDb2Store::load_complete(&data_dir, "enUS")
        .expect("load complete SpellEffect.db2 file data");
    let visuals = crate::spell_db2::SpellXSpellVisualStore::load_complete(&data_dir, "enUS")
        .expect("load complete SpellXSpellVisual.db2 file data");
    println!(
        "FILE-only complete counts: SpellEffect={} SpellXSpellVisual={}; SQL overlays/removals are not applied, real effective-dataset proof is pending",
        effects.len(),
        visuals.len()
    );

    let mut aura_rows = effects
        .entries_like_cpp()
        .filter(|row| i32::from(row.effect_aura) == 403)
        .collect::<Vec<_>>();
    aura_rows
        .sort_unstable_by_key(|row| (row.spell_id, row.difficulty_id, row.effect_index, row.id));
    println!("Aura403 file rows: RecordID SpellID Difficulty EffectIndex Misc1 Misc2");
    for row in aura_rows {
        println!(
            "Aura403\t{}\t{}\t{}\t{}\t{}\t{}",
            row.id,
            row.spell_id,
            row.difficulty_id,
            row.effect_index,
            row.effect_misc_value[0],
            row.effect_misc_value[1]
        );
    }

    let regular = SpellStore::from_spell_db2_stores_like_cpp(
        &crate::spell_db2::SpellCategoriesStore::from_entries([]),
        &crate::spell_db2::SpellMiscStore::from_entries([]),
        &effects,
        &crate::spell_db2::SpellShapeshiftStore::from_entries([]),
    );
    for target_spell_id in [8_326, 20_584, 15_007] {
        println!(
            "FILE-only regular override potential\t{target_spell_id}\t{}",
            regular.may_override_spell_visual(target_spell_id)
        );

        let mut rows = visuals
            .entries_like_cpp()
            .filter(|row| row.spell_id == target_spell_id)
            .collect::<Vec<_>>();
        rows.sort_unstable_by(|left, right| {
            right
                .caster_player_condition_id
                .cmp(&left.caster_player_condition_id)
                .then_with(|| right.id.cmp(&left.id))
        });
        println!(
            "Source-order visual rows\t{target_spell_id}\tRecordID Difficulty SpellVisualID CasterPlayerConditionID CasterUnitConditionID ViewerPlayerConditionID ViewerUnitConditionID Probability Flags Priority"
        );
        for row in rows {
            println!(
                "Visual\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:?}\t{}\t{}",
                row.id,
                row.difficulty_id,
                row.spell_visual_id,
                row.caster_player_condition_id,
                row.caster_unit_condition_id,
                row.viewer_player_condition_id,
                row.viewer_unit_condition_id,
                row.probability,
                row.flags,
                row.priority
            );
        }
    }
}
