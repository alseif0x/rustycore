use super::*;
use crate::spell_db2::{
    SpellCategoriesEntry, SpellCategoriesStore, SpellEffectDb2Store, SpellMiscStore,
    SpellShapeshiftStore,
};

fn category_row(id: u32, spell_id: u32, difficulty_id: u8, category: i16) -> SpellCategoriesEntry {
    SpellCategoriesEntry {
        id,
        difficulty_id,
        category,
        defense_type: 0,
        dispel_type: 0,
        mechanic: 0,
        prevention_type: 0,
        start_recovery_category: 0,
        charge_category: 0,
        spell_id,
    }
}

fn start_recovery_category_row(
    id: u32,
    spell_id: u32,
    difficulty_id: u8,
    category: i16,
) -> SpellCategoriesEntry {
    let mut row = category_row(id, spell_id, difficulty_id, 0);
    row.start_recovery_category = category;
    row
}

fn category_store(rows: impl IntoIterator<Item = SpellCategoriesEntry>) -> SpellStore {
    SpellStore::from_spell_db2_stores_like_cpp(
        &SpellCategoriesStore::from_entries(rows),
        &SpellMiscStore::from_entries([]),
        &SpellEffectDb2Store::from_entries([]),
        &SpellShapeshiftStore::from_entries([]),
    )
}

fn difficulties() -> crate::DifficultyStore {
    crate::DifficultyStore::from_entries([
        crate::DifficultyEntry {
            id: 0,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
        crate::DifficultyEntry {
            id: 4,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
    ])
}

#[test]
fn category_reader_distinguishes_unloaded_from_authoritative_empty() {
    let unhydrated = SpellStore::new();
    assert_eq!(
        unhydrated.category_id_at_difficulty(71_001, 0, None),
        Err(SpellCategoryReadError::MetadataUnavailable)
    );

    let loaded_empty = category_store([]);
    assert_eq!(
        loaded_empty.category_id_at_difficulty(71_001, 0, None),
        Ok(0)
    );
    assert_eq!(
        unhydrated.start_recovery_category_at_difficulty_like_cpp(71_001, 0, None),
        Err(SpellCategoryReadError::MetadataUnavailable)
    );
    assert_eq!(
        loaded_empty.start_recovery_category_at_difficulty_like_cpp(71_001, 0, None),
        Ok(0)
    );
}

#[test]
fn no_reagent_cost_aura_flag_is_spell_attr5_word_five() {
    let spell_id = 71_020;
    let flag = crate::spell::attributes::SPELL_ATTR5_NO_REAGENT_COST_WITH_AURA;
    let mut misc = crate::spell_db2::SpellMiscEntry {
        spell_id,
        ..Default::default()
    };
    misc.attributes[4] = flag as i32;
    let word_four_only = SpellStore::from_spell_db2_stores_like_cpp(
        &SpellCategoriesStore::from_entries([]),
        &SpellMiscStore::from_entries([misc]),
        &SpellEffectDb2Store::from_entries([]),
        &SpellShapeshiftStore::from_entries([]),
    );
    assert!(word_four_only.has_attribute_for_difficulty_like_cpp(
        spell_id as i32,
        0,
        None,
        4,
        flag,
    ));
    assert!(!word_four_only.has_attribute_for_difficulty_like_cpp(
        spell_id as i32,
        0,
        None,
        5,
        flag,
    ));

    let mut misc = crate::spell_db2::SpellMiscEntry {
        spell_id,
        ..Default::default()
    };
    misc.attributes[5] = flag as i32;
    let word_five = SpellStore::from_spell_db2_stores_like_cpp(
        &SpellCategoriesStore::from_entries([]),
        &SpellMiscStore::from_entries([misc]),
        &SpellEffectDb2Store::from_entries([]),
        &SpellShapeshiftStore::from_entries([]),
    );
    assert!(word_five.has_attribute_for_difficulty_like_cpp(spell_id as i32, 0, None, 5, flag,));
}

#[test]
fn start_recovery_category_uses_difficulty_fallback_and_exact_zero_override() {
    let fallback = category_store([start_recovery_category_row(1, 71_010, 0, 133)]);
    assert_eq!(
        fallback.start_recovery_category_at_difficulty_like_cpp(71_010, 4, Some(&difficulties()),),
        Ok(133)
    );

    let exact_zero = category_store([
        start_recovery_category_row(1, 71_011, 0, 133),
        start_recovery_category_row(2, 71_011, 4, 0),
    ]);
    assert_eq!(
        exact_zero.start_recovery_category_at_difficulty_like_cpp(71_011, 4, None),
        Ok(0)
    );
}

#[test]
fn category_reader_resolves_fallback_and_exact_zero_stops_it() {
    let fallback = category_store([category_row(1, 71_002, 0, 19)]);
    assert_eq!(
        fallback.category_id_at_difficulty(71_002, 4, Some(&difficulties())),
        Ok(19)
    );

    let exact_zero = category_store([
        category_row(1, 71_003, 0, 19),
        category_row(2, 71_003, 4, 0),
    ]);
    assert_eq!(exact_zero.category_id_at_difficulty(71_003, 4, None), Ok(0));
}

#[test]
fn category_reader_keeps_highest_record_id_and_full_width_spell_id() {
    let spell_id = 0xF123_4567;
    let store = category_store([
        category_row(4, spell_id, 0, 11),
        category_row(9, spell_id, 0, 23),
    ]);

    assert_eq!(store.category_id_at_difficulty(spell_id, 0, None), Ok(23));
}

#[test]
fn category_reader_requires_graph_only_for_actual_fallback_traversal() {
    let store = category_store([category_row(1, 71_004, 0, 31)]);
    assert_eq!(
        store.category_id_at_difficulty(71_004, 4, None),
        Err(SpellCategoryReadError::Catalog(
            crate::spell::DataSpellCatalogResolutionError::DifficultyUnavailable
        ))
    );

    let cyclic = crate::DifficultyStore::from_entries([crate::DifficultyEntry {
        id: 4,
        instance_type: 0,
        flags: 0,
        fallback_difficulty_id: 4,
        toggle_difficulty_id: 0,
    }]);
    assert_eq!(
        store.category_id_at_difficulty(71_004, 4, Some(&cyclic)),
        Err(SpellCategoryReadError::Catalog(
            crate::spell::DataSpellCatalogResolutionError::DifficultyCycle
        ))
    );
}

#[test]
fn cooldown_readiness_components_use_complete_source_difficulty_metadata() {
    let mut categories = category_row(1, 71_006, 0, 4);
    categories.prevention_type = 1;
    let store = category_store([categories]);
    assert_eq!(
        store.prevention_type_at_difficulty_like_cpp(71_006, 4, Some(&difficulties())),
        Ok(1)
    );
    assert_eq!(
        store.prevention_type_at_difficulty_like_cpp(71_007, 0, None),
        Ok(0),
        "a complete loaded SpellCategories catalog supplies the source default"
    );

    let misc = |id, difficulty_id, speed, launch_delay| crate::spell_db2::SpellMiscEntry {
        id,
        difficulty_id,
        speed,
        launch_delay,
        spell_id: 71_008,
        ..Default::default()
    };
    let store = SpellStore::from_spell_db2_stores_like_cpp(
        &SpellCategoriesStore::from_entries([category_row(3, 71_008, 4, 2)]),
        &SpellMiscStore::from_entries([misc(1, 0, 25.0, 0.0)]),
        &SpellEffectDb2Store::from_entries([]),
        &SpellShapeshiftStore::from_entries([]),
    );
    assert_eq!(
        store.has_hit_delay_at_difficulty_like_cpp(71_008, 4, Some(&difficulties())),
        Ok(true)
    );
    let zero_child = SpellStore::from_spell_db2_stores_like_cpp(
        &SpellCategoriesStore::from_entries([]),
        &SpellMiscStore::from_entries([misc(1, 0, 25.0, 0.0), misc(2, 4, 0.0, 0.0)]),
        &SpellEffectDb2Store::from_entries([]),
        &SpellShapeshiftStore::from_entries([]),
    );
    assert_eq!(
        zero_child.has_hit_delay_at_difficulty_like_cpp(71_008, 4, None),
        Ok(false),
        "an exact child SpellMisc row overrides its parent component"
    );

    let cyclic = crate::DifficultyStore::from_entries([crate::DifficultyEntry {
        id: 4,
        instance_type: 0,
        flags: 0,
        fallback_difficulty_id: 4,
        toggle_difficulty_id: 0,
    }]);
    assert_eq!(
        store.has_hit_delay_at_difficulty_like_cpp(71_008, 4, Some(&cyclic)),
        Err(SpellCategoryReadError::Catalog(
            crate::spell::DataSpellCatalogResolutionError::DifficultyCycle
        ))
    );
}

#[test]
fn complete_spell_misc_catalog_with_no_spell_row_keeps_source_hit_delay_default() {
    let store = SpellStore::from_spell_db2_stores_like_cpp(
        &SpellCategoriesStore::from_entries([]),
        &SpellMiscStore::from_entries([crate::spell_db2::SpellMiscEntry {
            spell_id: 71_010,
            ..Default::default()
        }]),
        &SpellEffectDb2Store::from_entries([]),
        &SpellShapeshiftStore::from_entries([]),
    );

    assert_eq!(
        store.has_hit_delay_at_difficulty_like_cpp(71_011, 4, None),
        Ok(false),
        "a known absent SpellMisc row leaves C++ speed and launch delay at zero"
    );
}

#[test]
fn misc_attribute_reader_distinguishes_unavailable_empty_and_difficulty_fallback() {
    let unavailable = SpellStore::new();
    assert_eq!(
        unavailable.misc_attribute_words_at_difficulty_like_cpp(71_013, 0, None),
        Err(
            crate::spell::SpellInfoPayloadResolutionErrorLikeCpp::AuthorityUnavailable(
                crate::spell::SpellInfoPayloadComponentLikeCpp::Misc
            )
        )
    );

    let complete_empty = SpellStore::from_spell_db2_stores_like_cpp(
        &SpellCategoriesStore::from_entries([]),
        &SpellMiscStore::from_entries([crate::spell_db2::SpellMiscEntry {
            spell_id: 71_014,
            ..Default::default()
        }]),
        &SpellEffectDb2Store::from_entries([]),
        &SpellShapeshiftStore::from_entries([]),
    );
    assert_eq!(
        complete_empty.misc_attribute_words_at_difficulty_like_cpp(71_013, 0, None),
        Ok([0; 15])
    );

    let mut parent = crate::spell_db2::SpellMiscEntry {
        spell_id: 71_015,
        ..Default::default()
    };
    parent.attributes[12] = 0x0080_0000;
    let inherited = SpellStore::from_spell_db2_stores_like_cpp(
        &SpellCategoriesStore::from_entries([]),
        &SpellMiscStore::from_entries([parent]),
        &SpellEffectDb2Store::from_entries([]),
        &SpellShapeshiftStore::from_entries([]),
    );
    assert_eq!(
        inherited
            .misc_attribute_words_at_difficulty_like_cpp(71_015, 4, Some(&difficulties()))
            .unwrap()[12],
        0x0080_0000
    );
}

#[test]
fn a_synthetic_hit_metadata_row_does_not_prove_spell_misc_catalog_completeness() {
    let mut store = SpellStore::new();
    store.insert_spell_hit_metadata_for_difficulty_like_cpp(
        71_012,
        0,
        crate::spell::SpellHitMetadataLikeCpp {
            category_id: 0,
            charge_category_id: 0,
            defense_type: 0,
            spell_mechanic: 0,
            school_mask: 0,
            effect_mechanics: Default::default(),
        },
    );

    assert_eq!(
        store.has_hit_delay_at_difficulty_like_cpp(71_012, 0, None),
        Err(SpellCategoryReadError::MetadataUnavailable)
    );
}

#[test]
fn individual_hit_metadata_insertion_does_not_certify_category_completeness() {
    let mut store = SpellStore::new();
    store.insert_spell_hit_metadata_for_difficulty_like_cpp(
        71_005,
        0,
        crate::spell::SpellHitMetadataLikeCpp {
            category_id: 37,
            charge_category_id: 0,
            defense_type: 0,
            spell_mechanic: 0,
            school_mask: 0,
            effect_mechanics: Default::default(),
        },
    );

    assert_eq!(
        store.category_id_at_difficulty(71_005, 0, None),
        Err(SpellCategoryReadError::MetadataUnavailable)
    );
}
