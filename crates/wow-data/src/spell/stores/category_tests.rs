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
