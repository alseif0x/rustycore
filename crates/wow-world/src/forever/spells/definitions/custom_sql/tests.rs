//! Synthetic source-prefix cases, not full custom attributes or learned spells.
use super::super::{Definition, Key};
use super::*;
use crate::forever::spells::{SpellEffectValues, SpellLoadPlan, SpellTraversal};
use std::sync::Arc;
use wow_data::{Db2HotfixRemovalStoreLikeCpp, forever_birth::BirthRecords, forever_spells::*};
use wow_persistence::forever::spells::server::ServerSpellRows;

fn row(spell: u32, attributes: u32) -> SpellCustomAttributeRow {
    SpellCustomAttributeRow { spell, attributes }
}
fn definition(kinds: &[u32]) -> Definition {
    let mut definition = Definition::empty_server(b"synthetic".to_vec());
    definition.effects = kinds
        .iter()
        .enumerate()
        .map(|(index, &effect)| SpellEffectValues {
            index: index as u32,
            effect,
            ..Default::default()
        })
        .collect();
    definition
}
fn fresh(definitions: impl IntoIterator<Item = (Key, Definition)>) -> SpellDefinitionSeeds {
    let catalog = SpellRecords {
        spell_names: vec![SpellNameRecord {
            id: 900_000,
            name: SpellText::from_locale(6, b"raw".to_vec()).unwrap(),
        }],
        ..Default::default()
    }
    .finish(
        SpellRecords::default(),
        SpellRecords::default(),
        6,
        SpellLocaleRecords::default(),
        SpellLocaleRecords::default(),
        &Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([]),
    )
    .unwrap();
    let mut seeds = SpellLoadPlan::build(Arc::new(catalog))
        .unwrap()
        .with_server_spells(ServerSpellRows::default())
        .unwrap();
    seeds.definitions = definitions.into_iter().collect();
    seeds
}
fn corrected(seeds: SpellDefinitionSeeds) -> SpellDefinitionSeeds {
    // Prescribed synthetic source order only; no native hash-order claim.
    let mut secondary = seeds.definitions.keys().copied().collect::<Vec<_>>();
    secondary.reverse();
    let primary = seeds.definitions.keys().copied().collect();
    seeds
        .with_id_corrections()
        .unwrap()
        .with_global_corrections(SpellTraversal::new(primary, secondary))
        .unwrap()
}
fn admitted(seeds: SpellDefinitionSeeds) -> SpellDefinitionSeeds {
    let birth = BirthRecords::default()
        .finish(
            BirthRecords::default(),
            BirthRecords::default(),
            &Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([]),
        )
        .unwrap();
    corrected(seeds)
        .with_skill_line_abilities(Arc::new(birth))
        .unwrap()
}

#[test]
fn sql_prefix_requires_skill_map_and_rejects_reapplication_even_for_empty_rows() {
    let seeds = fresh([]);
    assert_eq!(seeds.sql_custom_attribute_counts(), None);
    assert!(matches!(
        seeds.with_sql_custom_attributes(vec![]),
        Err(SpellDefinitionError::SqlCustomAttributesRequireSkillLineAbilities)
    ));
    assert!(matches!(
        corrected(fresh([])).with_sql_custom_attributes(vec![]),
        Err(SpellDefinitionError::SqlCustomAttributesRequireSkillLineAbilities)
    ));
    let result = admitted(fresh([]))
        .with_sql_custom_attributes(vec![])
        .unwrap();
    assert_eq!(
        result.sql_custom_attribute_counts(),
        Some(SqlCustomAttributeCounts::default())
    );
    assert!(matches!(
        result.with_sql_custom_attributes(vec![]),
        Err(SpellDefinitionError::SqlCustomAttributesAlreadyApplied)
    ));
}

#[test]
fn empty_batch_is_an_admitted_noop_not_a_full_custom_attributes_phase() {
    let mut original = definition(&[2]);
    original.custom_attributes = 0x80;
    let result = admitted(fresh([((900_001, 0), original)]))
        .with_sql_custom_attributes(vec![])
        .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(
        result.get_exact(900_001, 0).unwrap().custom_attributes(),
        0x80
    );
    assert_eq!(
        result.sql_custom_attribute_counts(),
        Some(SqlCustomAttributeCounts::default())
    );
}

#[test]
fn every_existing_signed_difficulty_receives_all_bits_without_default_manufacture() {
    let keys = [i16::MIN, -1, 1, i16::MAX];
    let seeds = admitted(fresh(
        keys.map(|difficulty| ((900_001, difficulty), definition(&[2]))),
    ));
    let before = seeds
        .corrected_difficulties(900_001)
        .unwrap()
        .map(|view| view.difficulty())
        .collect::<Vec<_>>();
    assert_eq!(before, [i16::MAX, 1, -1, i16::MIN]);
    let result = seeds
        .with_sql_custom_attributes(vec![row(900_001, u32::MAX)])
        .unwrap();
    for difficulty in keys {
        assert_eq!(
            result
                .get_exact(900_001, difficulty)
                .unwrap()
                .custom_attributes(),
            u32::MAX
        );
    }
    assert!(result.get_exact(900_001, 0).is_none());
    assert_eq!(
        result
            .corrected_difficulties(900_001)
            .unwrap()
            .map(|view| view.difficulty())
            .collect::<Vec<_>>(),
        before
    );
    assert_eq!(
        result.sql_custom_attribute_counts(),
        Some(SqlCustomAttributeCounts {
            input_rows: 1,
            counted_rows: 1,
            definition_assignments: 4,
            ..Default::default()
        })
    );
}

#[test]
fn share_damage_rejects_the_entire_word_only_for_difficulties_without_school_damage() {
    let mut damage = definition(&[0, 2]);
    damage.effects[1].base_points = 0.0; // magnitude is not a HasEffect filter
    let mut other = definition(&[1, 6]);
    other.custom_attributes = 0x40;
    let result = admitted(fresh([
        ((900_001, -1), damage),
        ((900_001, 0), other),
        ((900_001, 1), definition(&[])),
        ((900_001, 2), definition(&[0])),
    ]))
    .with_sql_custom_attributes(vec![row(900_001, SHARE_DAMAGE | 0x8000_0000)])
    .unwrap();
    assert_eq!(
        result.get_exact(900_001, -1).unwrap().custom_attributes(),
        0x8000_0008
    );
    assert_eq!(
        result.get_exact(900_001, 0).unwrap().custom_attributes(),
        0x40
    );
    for difficulty in [1, 2] {
        assert_eq!(
            result
                .get_exact(900_001, difficulty)
                .unwrap()
                .custom_attributes(),
            0
        );
    }
    assert_eq!(
        result.sql_custom_attribute_counts(),
        Some(SqlCustomAttributeCounts {
            input_rows: 1,
            counted_rows: 1,
            definition_assignments: 1,
            share_damage_rejected_definitions: 3,
            ..Default::default()
        })
    );
}

#[test]
fn source_counts_existing_rows_even_when_every_difficulty_rejects() {
    let result = admitted(fresh([
        ((900_001, 0), definition(&[0])),
        ((900_001, 1), definition(&[3])),
    ]))
    .with_sql_custom_attributes(vec![
        row(900_001, SHARE_DAMAGE),
        row(900_002, 0),
        row(0, u32::MAX),
    ])
    .unwrap();
    assert_eq!(
        result.sql_custom_attribute_counts(),
        Some(SqlCustomAttributeCounts {
            input_rows: 3,
            counted_rows: 1,
            missing_spell_rows: 2,
            share_damage_rejected_definitions: 2,
            definition_assignments: 0
        })
    );
    assert_eq!(result.len(), 2);
    assert!(result.get_exact(900_002, 0).is_none());
    assert!(result.get_exact(0, 0).is_none());
}

#[test]
fn duplicate_and_zero_rows_are_not_deduplicated_or_misread_as_share_damage() {
    let result = admitted(fresh([((900_001, 0), definition(&[]))]))
        .with_sql_custom_attributes(vec![
            row(900_001, 2),
            row(900_001, SHARE_DAMAGE | 0x4000_0000),
            row(900_001, 4),
            row(900_001, 0),
            row(900_001, 2),
        ])
        .unwrap();
    assert_eq!(result.get_exact(900_001, 0).unwrap().custom_attributes(), 6);
    assert_eq!(
        result.sql_custom_attribute_counts(),
        Some(SqlCustomAttributeCounts {
            input_rows: 5,
            counted_rows: 5,
            definition_assignments: 4,
            share_damage_rejected_definitions: 1,
            ..Default::default()
        })
    );
}

#[test]
fn sql_or_preserves_prior_id_flags_all_attribute_words_negative_slots_and_raw_identity() {
    let mut original = definition(&[2]);
    original.fields.attributes = std::array::from_fn(|index| 1 << index);
    original.negative_effects[31] = true;
    let seeds = admitted(fresh([((404468, 0), original)]));
    let before = seeds.get_exact(404468, 0).unwrap();
    let attributes = before.fields().attributes;
    assert_eq!(before.custom_attributes(), 0x0100_0000); // source ID correction
    let raw = seeds.raw_catalog(); // SQL only mutates the private derived owner
    let raw_pointer = raw.spell_name(900_000).unwrap() as *const SpellNameRecord;
    let result = seeds
        .with_sql_custom_attributes(vec![row(404468, 0x8000_0008)])
        .unwrap();
    let after = result.get_exact(404468, 0).unwrap();
    assert_eq!(after.custom_attributes(), 0x8100_0008);
    assert_eq!(after.fields().attributes, attributes);
    assert!(after.negative_effects()[31]);
    assert_eq!(
        result.raw_catalog().spell_name(900_000).unwrap() as *const SpellNameRecord,
        raw_pointer
    );
    assert_eq!(
        raw.spell_name(900_000).unwrap().name.at(6),
        Some(b"raw".as_slice())
    );
}

#[test]
fn has_effect_checks_the_full_vector_kind_not_aura_value_or_first_slot() {
    let mut original = definition(&[0; 32]);
    original.effects[0].aura = 2; // aura 2 does not mean SCHOOL_DAMAGE
    original.effects[31].effect = 2;
    let result = admitted(fresh([((900_001, 0), original)]))
        .with_sql_custom_attributes(vec![row(900_001, SHARE_DAMAGE)])
        .unwrap();
    let view = result.get_exact(900_001, 0).unwrap();
    assert!(view.has_effect(0)); // pinned IsEffect(kind) is equality, even NONE
    assert!(view.has_effect(2));
    assert!(!view.has_effect(1));
    assert_eq!(view.custom_attributes(), SHARE_DAMAGE);
    assert_eq!(
        result
            .sql_custom_attribute_counts()
            .unwrap()
            .definition_assignments,
        1
    );
}
