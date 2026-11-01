use super::*;

const SPELL_ID: u32 = 73_001;
const OTHER_SPELL_ID: u32 = 73_002;

fn effect_row(
    record_id: u32,
    spell_id: u32,
    difficulty_id: i32,
    effect_index: i32,
    effect: u32,
    base_points: i32,
) -> crate::spell_db2::SpellEffectDb2Entry {
    let mut row = test_spell_effect_db2_entry_like_cpp(
        record_id,
        spell_id,
        difficulty_id,
        effect_index,
        effect,
        0,
    );
    row.effect_base_points = base_points;
    row
}

fn spell_store(
    rows: impl IntoIterator<Item = crate::spell_db2::SpellEffectDb2Entry>,
) -> SpellStore {
    SpellStore::from_spell_db2_stores_like_cpp(
        &crate::spell_db2::SpellCategoriesStore::from_entries([]),
        &crate::spell_db2::SpellMiscStore::from_entries([]),
        &crate::spell_db2::SpellEffectDb2Store::from_entries(rows),
        &crate::spell_db2::SpellShapeshiftStore::from_entries([]),
    )
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

#[test]
fn indexed_rows_choose_highest_record_id_and_keep_inert_rows_out_of_active_views() {
    let store = spell_store([
        effect_row(10, SPELL_ID, 0, 0, 2, 10),
        effect_row(30, SPELL_ID, 0, 0, 0, 30),
        effect_row(20, SPELL_ID, 0, 1, 3, 20),
    ]);

    let exact = store
        .effect_rows_for_exact_difficulty(SPELL_ID as i32, 0)
        .expect("exact rows retained");
    assert_eq!(
        exact
            .iter()
            .map(|row| (row.effect_index, row.effect, row.effect_base_points))
            .collect::<Vec<_>>(),
        [(1, 3, 20), (0, 0, 30)],
    );

    let selected = resolve_spell_effect_rows(&store, SPELL_ID as i32, 0, None)
        .expect("selected rows include inert entries");
    assert_eq!(
        selected
            .iter()
            .map(|row| (row.effect_index, row.effect, row.effect_base_points))
            .collect::<Vec<_>>(),
        [(0, 0, 30), (1, 3, 20)],
    );

    let legacy = store
        .effects_for_difficulty_like_cpp(SPELL_ID as i32, 0, None)
        .expect("active projection");
    assert_eq!(
        legacy
            .iter()
            .map(|row| (row.effect_index, row.effect))
            .collect::<Vec<_>>(),
        [(1, 3)],
    );
    assert_eq!(
        store
            .get(SPELL_ID as i32)
            .expect("active base SpellInfo")
            .effects()
            .iter()
            .map(|row| (row.effect_index, row.effect))
            .collect::<Vec<_>>(),
        [(1, 3)],
    );
}

#[test]
fn surviving_lower_record_id_wins_after_higher_row_is_removed() {
    // Hydration receives the already-effective DB2 store after tombstones.
    let store = spell_store([effect_row(10, SPELL_ID, 0, 0, 2, 41)]);

    let effects = resolve_spell_effect_rows(&store, SPELL_ID as i32, 0, None)
        .expect("effective rows resolve");
    assert_eq!(effects.len(), 1);
    assert_eq!(
        (effects[0].effect_index, effects[0].effect_base_points),
        (0, 41)
    );
}

#[test]
fn indexed_effects_inherit_independently_and_explicit_inert_rows_override() {
    let store = spell_store([
        effect_row(1, SPELL_ID, 0, 0, 2, 10),
        effect_row(2, SPELL_ID, 0, 1, 2, 11),
        effect_row(3, SPELL_ID, 0, 2, 2, 12),
        effect_row(4, SPELL_ID, 0, 3, 2, 13),
        effect_row(5, SPELL_ID, 3, 0, 0, 30),
        effect_row(6, SPELL_ID, 3, 1, 3, 31),
        effect_row(7, SPELL_ID, 4, 2, 4, 42),
    ]);
    let difficulties = crate::difficulty::DifficultyStore::from_entries([
        difficulty(4, 3),
        difficulty(3, 0),
        difficulty(0, 0),
    ]);

    let effects = resolve_spell_effect_rows(&store, SPELL_ID as i32, 4, Some(&difficulties))
        .expect("indexed fallback resolves");
    assert_eq!(
        effects
            .iter()
            .map(|row| (row.effect_index, row.effect, row.effect_base_points))
            .collect::<Vec<_>>(),
        [(0, 0, 30), (1, 3, 31), (2, 4, 42), (3, 2, 13)],
    );
}

#[test]
fn zero_difficulty_can_follow_its_actual_fallback_without_a_fallback_row() {
    let store = spell_store([effect_row(1, SPELL_ID, 3, 2, 7, 73)]);
    let difficulties = crate::difficulty::DifficultyStore::from_entries([difficulty(0, 3)]);

    let effects = resolve_spell_effect_rows(&store, SPELL_ID as i32, 0, Some(&difficulties))
        .expect("zero difficulty follows its stored fallback");
    assert_eq!(effects.len(), 1);
    assert_eq!(
        (effects[0].effect_index, effects[0].effect_base_points),
        (2, 73)
    );
}

#[test]
fn inert_only_exact_bucket_stops_selected_resolution_but_legacy_view_falls_back() {
    let store = spell_store([
        effect_row(1, SPELL_ID, 0, 0, 2, 10),
        effect_row(2, SPELL_ID, 3, 0, 0, 20),
    ]);
    let difficulties =
        crate::difficulty::DifficultyStore::from_entries([difficulty(3, 0), difficulty(0, 0)]);

    let selected = resolve_spell_effect_rows(&store, SPELL_ID as i32, 3, Some(&difficulties))
        .expect("exact inert row is known");
    assert_eq!(selected.len(), 1);
    assert_eq!((selected[0].effect_index, selected[0].effect), (0, 0));

    let legacy = store
        .effects_for_difficulty_like_cpp(SPELL_ID as i32, 3, Some(&difficulties))
        .expect("legacy projection reaches active base row");
    assert_eq!(legacy.len(), 1);
    assert_eq!(
        (legacy[0].effect_index, legacy[0].effect_base_points),
        (0, 10)
    );
}

#[test]
fn missing_difficulty_stops_and_unknown_or_cyclic_needed_graphs_error() {
    let base_store = spell_store([effect_row(1, SPELL_ID, 0, 0, 2, 10)]);
    let known_missing = crate::difficulty::DifficultyStore::from_entries([difficulty(0, 0)]);
    assert!(
        resolve_spell_effect_rows(&base_store, SPELL_ID as i32, 4, Some(&known_missing))
            .expect("known missing requested difficulty stops")
            .is_empty()
    );
    assert_eq!(
        resolve_spell_effect_rows(&base_store, SPELL_ID as i32, 4, None),
        Err(SpellEffectResolutionError::Catalog(
            DataSpellCatalogResolutionError::DifficultyUnavailable
        )),
    );

    let cyclic_store = spell_store([effect_row(2, SPELL_ID, 7, 0, 2, 70)]);
    let cycle =
        crate::difficulty::DifficultyStore::from_entries([difficulty(0, 3), difficulty(3, 0)]);
    assert_eq!(
        resolve_spell_effect_rows(&cyclic_store, SPELL_ID as i32, 0, Some(&cycle)),
        Err(SpellEffectResolutionError::Catalog(
            DataSpellCatalogResolutionError::DifficultyCycle
        )),
    );
}

#[test]
fn exact_rows_and_known_empty_spell_metadata_do_not_need_irrelevant_graphs() {
    let exact_store = spell_store([
        effect_row(1, SPELL_ID, 4, 0, 2, 44),
        effect_row(2, OTHER_SPELL_ID, 0, 0, 3, 99),
    ]);
    let exact = resolve_spell_effect_rows(&exact_store, SPELL_ID as i32, 4, None)
        .expect("exact row does not need a difficulty graph");
    assert_eq!(exact.len(), 1);
    assert_eq!(exact[0].effect_base_points, 44);

    let empty = resolve_spell_effect_rows(&exact_store, 73_003, 4, None)
        .expect("no rows for this spell are known without a graph");
    assert!(empty.is_empty());
}

#[test]
fn dynamic_base_effects_are_used_only_when_no_raw_rows_exist() {
    let mut store = SpellStore::new();
    let mut info = test_spell_info_without_aura(SPELL_ID as i32);
    info.effects = vec![SpellEffectInfo {
        effect_index: 2,
        effect: 8,
        effect_base_points: 82,
        ..SpellEffectInfo::default()
    }];
    store.insert(SPELL_ID as i32, info);

    let exact = store
        .effect_rows_for_exact_difficulty(SPELL_ID as i32, 0)
        .expect("dynamic base vector is explicit at difficulty zero");
    assert_eq!(exact.len(), 1);
    assert_eq!(exact[0].effect_base_points, 82);

    let difficulties = crate::difficulty::DifficultyStore::from_entries([difficulty(4, 0)]);
    let resolved = resolve_spell_effect_rows(&store, SPELL_ID as i32, 4, Some(&difficulties))
        .expect("dynamic base vector follows the known chain");
    assert_eq!(resolved.len(), 1);
    assert_eq!(
        (resolved[0].effect_index, resolved[0].effect_base_points),
        (2, 82)
    );

    let raw_store = spell_store([effect_row(3, SPELL_ID, 3, 1, 9, 93)]);
    let mut mixed_store = raw_store;
    mixed_store.insert(
        SPELL_ID as i32,
        test_spell_info_with_aura(SPELL_ID as i32, 17),
    );
    assert_eq!(
        mixed_store.effect_rows_for_exact_difficulty(SPELL_ID as i32, 0),
        None,
        "raw metadata at any difficulty prevents a dynamic vector from inventing base rows",
    );
}

#[test]
fn malformed_dynamic_effect_vectors_remain_unavailable() {
    let mut duplicate_store = SpellStore::new();
    let mut duplicate_info = test_spell_info_without_aura(SPELL_ID as i32);
    duplicate_info.effects = vec![
        SpellEffectInfo {
            effect_index: 1,
            effect: 2,
            ..SpellEffectInfo::default()
        },
        SpellEffectInfo {
            effect_index: 1,
            effect: 3,
            ..SpellEffectInfo::default()
        },
    ];
    duplicate_store.insert(SPELL_ID as i32, duplicate_info);
    assert_eq!(
        resolve_spell_effect_rows(&duplicate_store, SPELL_ID as i32, 0, None),
        Err(SpellEffectResolutionError::DuplicateEffectIndex { effect_index: 1 }),
    );

    let mut out_of_range_store = SpellStore::new();
    let mut out_of_range_info = test_spell_info_without_aura(SPELL_ID as i32);
    out_of_range_info.effects = vec![SpellEffectInfo {
        effect_index: 32,
        effect: 2,
        ..SpellEffectInfo::default()
    }];
    out_of_range_store.insert(SPELL_ID as i32, out_of_range_info);
    assert_eq!(
        resolve_spell_effect_rows(&out_of_range_store, SPELL_ID as i32, 0, None),
        Err(SpellEffectResolutionError::InvalidEffectIndex { effect_index: 32 }),
    );
}

#[test]
fn malformed_raw_effect_indices_are_preserved_and_rejected() {
    let too_large = spell_store([effect_row(1, SPELL_ID, 0, 32, 2, 32)]);
    let too_large_rows = too_large
        .effect_rows_for_exact_difficulty(SPELL_ID as i32, 0)
        .expect("raw row is retained");
    assert_eq!(too_large_rows[0].effect_index, 32);
    assert_eq!(
        resolve_spell_effect_rows(&too_large, SPELL_ID as i32, 0, None),
        Err(SpellEffectResolutionError::InvalidEffectIndex { effect_index: 32 }),
    );

    let negative = spell_store([effect_row(2, SPELL_ID, 0, -1, 2, 99)]);
    let negative_rows = negative
        .effect_rows_for_exact_difficulty(SPELL_ID as i32, 0)
        .expect("raw row is retained");
    assert_eq!(negative_rows[0].effect_index, u32::MAX);
    assert_eq!(
        resolve_spell_effect_rows(&negative, SPELL_ID as i32, 0, None),
        Err(SpellEffectResolutionError::InvalidEffectIndex {
            effect_index: u32::MAX,
        }),
    );
}

#[test]
fn attributes_are_resolved_by_the_data_catalog_owner() {
    let mut store = SpellStore::new();
    let mut attributes = [0; 15];
    attributes[6] = 0x40;
    store.insert_spell_misc_attributes_for_difficulty_like_cpp(SPELL_ID as i32, 3, attributes);
    let difficulties = crate::difficulty::DifficultyStore::from_entries([difficulty(4, 3)]);

    assert_eq!(
        resolve_spell_attributes(&store, SPELL_ID as i32, 4, Some(&difficulties)),
        Ok(Some(attributes)),
    );
}
