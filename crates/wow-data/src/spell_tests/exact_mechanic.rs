use super::*;

#[test]
fn selected_spell_mechanic_exact_reader_keeps_zero_presence_and_absence_distinct() {
    let mut store = SpellStore::new();
    let mut base = SpellHitMetadataLikeCpp::default();
    base.spell_mechanic = 29;
    store.insert_spell_hit_metadata_for_difficulty_like_cpp(90_013, 0, base);

    let mut difficulty_four = SpellHitMetadataLikeCpp::default();
    difficulty_four.spell_mechanic = 0;
    store.insert_spell_hit_metadata_for_difficulty_like_cpp(90_013, 4, difficulty_four);

    assert_eq!(
        store.spell_mechanic_for_exact_difficulty(90_013, 4),
        Some(0)
    );
    assert_eq!(
        store.spell_mechanic_for_exact_difficulty(90_013, 0),
        Some(29)
    );
    assert_eq!(store.spell_mechanic_for_exact_difficulty(90_013, 3), None);
    assert_eq!(store.spell_mechanic_for_exact_difficulty(90_014, 0), None);
}
