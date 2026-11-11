use super::*;
use crate::forever::spells::skill_set_test_spell_fields;
use wow_persistence::forever::creation::SkillTierRow;

#[test]
fn birth_catalog_child_and_ability_iterators_preserve_storage_order() {
    // Retain the unsorted real-row inputs split from bonus/deactivation/reward cases.
    let fixture = Fixture::new(
        vec![
            line(1, 6, 0, 0),
            line(30, 6, 1, 0),
            line(20, 6, 1, 0),
            line(10, 6, 1, 0),
            line(11, 6, 10, 0),
        ],
        vec![
            ability(3, 10, 100, 101),
            ability(1, 10, 100, 101),
            ability(2, 10, 0, u32::MAX),
        ],
    );
    assert_eq!(
        SkillCatalog::child_lines(fixture.birth.as_ref(), 1)
            .map(|line| line.id)
            .collect::<Vec<_>>(),
        [10, 20, 30]
    );
    assert_eq!(
        SkillCatalog::child_lines(fixture.birth.as_ref(), 10)
            .map(|line| line.id)
            .collect::<Vec<_>>(),
        [11]
    );
    assert_eq!(
        SkillCatalog::abilities_for_skill(fixture.birth.as_ref(), 10)
            .map(|row| (row.id, row.spell as u32))
            .collect::<Vec<_>>(),
        [(1, 101), (2, u32::MAX), (3, 101)]
    );
    assert_eq!(fixture.sources().first_spell_in_chain(101), 100);
}

#[test]
fn skill_set_source_preserves_first_rc_match_and_selected_tier() {
    let mut first = [0; 16];
    first[0] = 75;
    first[1] = 150;
    let mut other = [0; 16];
    other[0] = 999;
    let fixture = Fixture::with_rc(
        vec![line(10, 6, 0, 0), line(20, 6, 10, 2)],
        rewards(&[10, 20]),
        vec![rc(1, 10, 1), rc(2, 10, 2)],
        vec![
            SkillTierRow {
                id: 1,
                values: other,
            },
            SkillTierRow {
                id: 2,
                values: first,
            },
        ],
        vec![2, 1],
    );
    let sources = fixture.sources();
    let selected = sources.skill_race_class_info(10).unwrap().unwrap();
    assert_eq!((selected.id, selected.skill, selected.tier), (2, 10, 2));
    assert_eq!(sources.skill_tier_value(selected.tier as u32, 0), Some(75));
    assert_eq!(sources.skill_tier_value(selected.tier as u32, 1), Some(150));
    assert_eq!(sources.skill_tier_value(1, 0), Some(999));
    assert_eq!(sources.skill_line(20).unwrap().parent_tier_index, 2);
}

#[test]
fn skill_set_source_projects_effective_reward_spell_metadata_and_absence() {
    let mut fixture = Fixture::new(
        vec![line(10, 6, 0, 0)],
        (1..=7)
            .rev()
            .map(|id| ability(id, 10, 0, 100 + id))
            .collect(),
    );
    for (spell, fields) in [
        (101, Some((1, 5, 0))),
        (102, Some((6, 0, 0))),
        (103, Some((2, 3, 0))),
        (106, Some((0, 0, 8))),
        (107, None),
    ] {
        skill_set_test_spell_fields(&mut fixture.spells, spell, fields);
    }
    let sources = fixture.sources();
    assert_eq!(
        sources
            .abilities_for_skill(10)
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        [1, 2, 3, 4, 5, 6, 7]
    );
    for (id, expected) in [
        (101, (1, 5, 0)),
        (102, (6, 0, 0)),
        (103, (2, 3, 0)),
        (106, (0, 0, 8)),
    ] {
        let spell = sources.reward_spell(id).unwrap().unwrap();
        assert_eq!(
            (
                spell.base_level,
                spell.spell_level,
                spell.future_condition_id
            ),
            expected
        );
    }
    assert!(sources.reward_spell(107).unwrap().is_none());
}
