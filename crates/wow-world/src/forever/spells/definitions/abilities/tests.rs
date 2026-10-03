//! Synthetic final DB2 relations; not executable/learned Player skills.
use super::*;
use crate::forever::spells::{SpellLoadPlan, SpellTraversal};
use wow_data::{Db2HotfixRemovalStoreLikeCpp, forever_birth::*, forever_spells::*};
use wow_persistence::forever::spells::server::ServerSpellRows;

fn row(id: u32, spell: i32, skill: u16) -> SkillAbilityRecord {
    SkillAbilityRecord {
        id,
        skill_line: skill,
        spell,
        min_skill_rank: i16::MAX,
        class_mask: 0,
        supercedes_spell: -1,
        acquire_method: i32::MIN,
        trivial_rank_high: i16::MIN,
        trivial_rank_low: i16::MAX,
        flags: i32::MIN,
        num_skill_ups: i8::MIN,
        unique_bit: i16::MIN,
        trade_skill_category: i16::MIN,
        skillup_skill_line: -1,
        field_5_5_4_67090_014: [i32::MIN, i32::MAX],
        race_mask: u64::MAX,
    }
}
fn birth(rows: BirthRecords) -> Arc<BirthCatalog> {
    Arc::new(
        rows.finish(
            BirthRecords::default(),
            BirthRecords::default(),
            &Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([]),
        )
        .unwrap(),
    )
}
fn fresh() -> SpellDefinitionSeeds {
    let catalog = SpellRecords::default()
        .finish(
            SpellRecords::default(),
            SpellRecords::default(),
            6,
            SpellLocaleRecords::default(),
            SpellLocaleRecords::default(),
            &Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([]),
        )
        .unwrap();
    SpellLoadPlan::build(Arc::new(catalog))
        .unwrap()
        .with_server_spells(ServerSpellRows::default())
        .unwrap()
}
fn corrected() -> SpellDefinitionSeeds {
    // Prescribed empty source traversal: no order-sensitive records here.
    fresh()
        .with_id_corrections()
        .unwrap()
        .with_global_corrections(SpellTraversal::new(vec![], vec![]))
        .unwrap()
}

#[test]
fn admission_requires_complete_correction_phase_and_rejects_replacement() {
    let input = fresh();
    assert!(input.skill_line_abilities(7).is_none());
    assert_eq!(input.skill_line_ability_counts(), None);
    assert_eq!(input.is_part_of_skill_line(333, 7), None);
    assert!(matches!(
        input.with_skill_line_abilities(birth(BirthRecords::default())),
        Err(SpellDefinitionError::SkillLineAbilitiesRequireGlobalCorrections)
    ));
    assert!(matches!(
        fresh()
            .with_id_corrections()
            .unwrap()
            .with_skill_line_abilities(birth(BirthRecords::default())),
        Err(SpellDefinitionError::SkillLineAbilitiesRequireGlobalCorrections)
    ));
    let result = corrected()
        .with_skill_line_abilities(birth(BirthRecords::default()))
        .unwrap();
    assert_eq!(
        result.skill_line_ability_counts(),
        Some(SkillLineAbilityCounts::default())
    );
    assert_eq!(result.skill_line_abilities(7).unwrap().count(), 0);
    assert_eq!(result.is_part_of_skill_line(333, 7), Some(false));
    assert!(matches!(
        result.with_skill_line_abilities(birth(BirthRecords::default())),
        Err(SpellDefinitionError::SkillLineAbilitiesAlreadyLoaded)
    ));
}

#[test]
fn repeated_spell_relations_retain_ascending_storage_order_and_do_not_require_definitions() {
    let result = corrected()
        .with_skill_line_abilities(birth(BirthRecords {
            abilities: vec![row(30, 7, 3), row(10, 7, 1), row(20, 7, 2)],
            ..Default::default()
        }))
        .unwrap();
    assert_eq!(
        result
            .skill_line_abilities(7)
            .unwrap()
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        vec![10, 20, 30]
    );
    assert_eq!(
        result.skill_line_ability_counts(),
        Some(SkillLineAbilityCounts {
            relations: 3,
            spell_ids: 1,
            unavailable_baseline_abilities: 0,
        })
    );
    assert!(result.is_empty()); // a relation never manufactures a SpellInfo
    assert!(result.get_exact(7, 0).is_none());
}

#[test]
fn zero_negative_and_absent_spell_keys_preserve_source_unsigned_key_conversion() {
    let result = corrected()
        .with_skill_line_abilities(birth(BirthRecords {
            abilities: vec![
                row(1, 0, 1),
                row(2, -1, 2),
                row(3, i32::MIN, 3),
                row(4, 987, 4),
            ],
            ..Default::default()
        }))
        .unwrap();
    for (spell, id) in [(0, 1), (u32::MAX, 2), (i32::MIN as u32, 3), (987, 4)] {
        assert_eq!(
            result
                .skill_line_abilities(spell)
                .unwrap()
                .next()
                .unwrap()
                .id,
            id
        );
    }
    assert_eq!(result.skill_line_ability_counts().unwrap().spell_ids, 4);
    assert!(result.is_empty());
}

#[test]
fn membership_uses_skill_line_not_skillup_or_race_class_rank_admission() {
    let mut ability = row(1, 7, 333);
    ability.skillup_skill_line = 999;
    let result = corrected()
        .with_skill_line_abilities(birth(BirthRecords {
            abilities: vec![ability],
            ..Default::default()
        }))
        .unwrap();
    assert_eq!(result.is_part_of_skill_line(333, 7), Some(true));
    assert_eq!(result.is_part_of_skill_line(999, 7), Some(false));
    assert_eq!(result.is_part_of_skill_line(333, 8), Some(false));
    let retained = result.skill_line_abilities(7).unwrap().next().unwrap();
    assert_eq!(retained.acquire_method, i32::MIN);
    assert_eq!(retained.class_mask, 0);
    assert_eq!(retained.min_skill_rank, i16::MAX);
    assert_eq!(retained.race_mask, u64::MAX);
}

#[test]
fn relation_readers_share_the_original_catalog_allocation_without_payload_clones() {
    let catalog = birth(BirthRecords {
        abilities: vec![row(1, 7, 333)],
        ..Default::default()
    });
    let pointer = catalog.skill_ability(1).unwrap() as *const SkillAbilityRecord;
    let result = corrected()
        .with_skill_line_abilities(catalog.clone())
        .unwrap();
    assert_eq!(Arc::strong_count(&catalog), 2);
    assert_eq!(
        result.skill_line_abilities(7).unwrap().next().unwrap() as *const SkillAbilityRecord,
        pointer
    );
    drop(catalog);
    assert_eq!(
        result.skill_line_abilities(7).unwrap().next().unwrap() as *const SkillAbilityRecord,
        pointer
    );
}

#[test]
fn indexes_use_final_overlays_removals_and_do_not_erase_baseline_uncertainty() {
    let catalog = Arc::new(
        BirthRecords {
            abilities: vec![row(1, 7, 1), row(2, 7, 2), row(3, 7, 3)],
            unknown_ability_records: 5,
            ..Default::default()
        }
        .finish(
            BirthRecords {
                abilities: vec![row(1, 88, 4)],
                ..Default::default()
            },
            BirthRecords {
                abilities: vec![row(1, 9, 5)],
                ..Default::default()
            },
            &Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(ABILITY_HASH, 2, 2)]),
        )
        .unwrap(),
    );
    let result = corrected().with_skill_line_abilities(catalog).unwrap();
    assert_eq!(
        result
            .skill_line_abilities(7)
            .unwrap()
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        vec![3]
    );
    assert_eq!(result.skill_line_abilities(88).unwrap().count(), 0);
    assert_eq!(
        result
            .skill_line_abilities(9)
            .unwrap()
            .next()
            .unwrap()
            .skill_line,
        5
    );
    assert_eq!(
        result.skill_line_ability_counts(),
        Some(SkillLineAbilityCounts {
            relations: 2,
            spell_ids: 2,
            unavailable_baseline_abilities: 5,
        })
    );
}
