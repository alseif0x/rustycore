use super::super::learning_fixtures as f;
use super::*;
use wow_data::forever_birth::BirthRecords;
fn row(spell: u32, required: u32) -> SpellRequiredRow {
    SpellRequiredRow { spell, required }
}
#[test]
fn admission_respects_rank_identity_self_duplicates_and_observed_equal_key_order() {
    let s = f::seeds(
        [1, 2, 3, 4]
            .into_iter()
            .map(|id| ((id, 0), f::definition(vec![])))
            .collect(),
        Default::default(),
        BirthRecords {
            abilities: vec![f::ability(1, 1, 2)],
            ..Default::default()
        },
    )
    .with_spell_ranks()
    .unwrap()
    .with_spell_required(vec![
        row(1, 2),
        row(3, 3),
        row(9, 1),
        row(3, 9),
        row(3, 4),
        row(3, 1),
        row(3, 4),
        row(4, 1),
    ])
    .unwrap();
    assert_eq!(
        s.spells_required_for(3).unwrap().collect::<Vec<_>>(),
        [4, 1]
    );
    assert_eq!(s.spells_requiring(1).unwrap().collect::<Vec<_>>(), [3, 4]);
    assert_eq!(s.is_spell_requiring(3, 4), Some(true));
    assert_eq!(s.is_spell_requiring(4, 3), Some(false));
    assert_eq!(
        s.required_spell_counts(),
        Some(RequiredSpellCounts {
            input_rows: 8,
            missing_source: 1,
            missing_required: 1,
            same_rank_chain: 2,
            duplicates: 1,
            relations: 3
        })
    );
    assert!(matches!(
        s.with_spell_required(vec![]),
        Err(SpellRequiredError::AlreadyApplied)
    ));
}
#[test]
fn source_does_not_dag_normalize_requirement_cycles_or_discard_high_id_bits() {
    let s = f::seeds(
        [1, u32::MAX]
            .into_iter()
            .map(|id| ((id, 0), f::definition(vec![])))
            .collect(),
        Default::default(),
        Default::default(),
    )
    .with_spell_ranks()
    .unwrap()
    .with_spell_required(vec![row(1, u32::MAX), row(u32::MAX, 1)])
    .unwrap();
    assert_eq!(
        s.spells_required_for(1).unwrap().collect::<Vec<_>>(),
        [u32::MAX]
    );
    assert_eq!(
        s.spells_requiring(1).unwrap().collect::<Vec<_>>(),
        [u32::MAX]
    );
}
#[test]
fn unloaded_and_loaded_empty_requirements_have_distinct_admission() {
    let s = f::seeds(vec![], Default::default(), Default::default());
    assert!(s.spells_required_for(1).is_none());
    assert!(matches!(
        s.with_spell_required(vec![]),
        Err(SpellRequiredError::RequiresRanks)
    ));
    let s = f::seeds(vec![], Default::default(), Default::default())
        .with_spell_ranks()
        .unwrap()
        .with_spell_required(vec![])
        .unwrap();
    assert_eq!(s.spells_required_for(1).unwrap().count(), 0);
    assert_eq!(s.required_spell_counts(), Some(Default::default()));
}
