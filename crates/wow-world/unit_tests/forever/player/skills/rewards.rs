use super::*;
use crate::forever::player::SkillUpdateState::Unchanged;
use wow_data::forever_birth::SkillAbilityRecord;

fn row(id: u32, method: i32, spell: u32, minimum: i16) -> SkillAbilityRecord {
    let mut row = ability(id, 10, 0, spell);
    row.acquire_method = method;
    row.min_skill_rank = minimum;
    row
}
fn fixture(rows: Vec<SkillAbilityRecord>) -> Fixture {
    Fixture::new(vec![line(10, 6, 0, 0)], rows)
}

#[test]
fn only_automatic_rank_removes_the_exact_spell_not_first_rank_and_other_methods_ignore_minimum() {
    let mut ranked = row(1, 1, 101, 75);
    ranked.supercedes_spell = 100;
    let fixture = fixture(vec![
        ranked,
        row(2, 2, 102, 200),
        row(3, 4, 103, 200),
        row(4, 1, 101, 75),
    ]);
    assert_eq!(fixture.spells.first_spell_in_chain(101), 100);
    let mut skills = allocated([10]);
    let mut effects = Effects::default();
    skills
        .learn_skill_rewards(10, 74, 1, &fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(
        effects.events,
        [
            Event::Remove(101),
            Event::Learn(10, 0, Some(Unchanged)),
            Event::AbilityConditions(3),
            Event::Learn(10, 0, Some(Unchanged)),
            Event::Remove(101)
        ]
    );
    assert_eq!(effects.grants, [(102, 10, false), (103, 10, false)]);
    assert_eq!(effects.world_queries.get(), 2);
}

#[test]
fn grouping_uses_skillup_but_grant_from_skill_uses_original_skillline_and_signed_id_bits() {
    let mut normal = row(1, 2, 101, 0);
    normal.skillup_skill_line = 30;
    let mut signed = row(2, 2, u32::MAX, 0);
    signed.skillup_skill_line = -1;
    let fixture = fixture(vec![signed, normal]);
    let mut skills = allocated([10]);
    let mut effects = Effects::default();
    skills
        .learn_skill_rewards(10, 7, 1, &fixture.sources(), &mut effects)
        .unwrap();
    assert!(effects.grants.is_empty());
    skills
        .learn_skill_rewards(30, 7, 1, &fixture.sources(), &mut effects)
        .unwrap();
    skills
        .learn_skill_rewards(u32::MAX, 7, 1, &fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(effects.grants, [(101, 10, false), (u32::MAX, 10, false)]);
}

#[test]
fn race_masks_use_target_mapping_and_class_zero_or_signed_wildcards_without_specificity_filter() {
    let mut matching = row(1, 2, 101, 0);
    matching.race_mask = 1 << 32;
    matching.class_mask = -1;
    let mut wrong = row(2, 2, 102, 0);
    wrong.race_mask = 1 << 33;
    let fixture = fixture(vec![matching, wrong, row(3, 2, 103, 0)]);
    let mut skills = allocated([10]);
    let mut effects = Effects::default();
    skills
        .learn_skill_rewards(10, 7, 95, &fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(effects.grants, [(101, 10, false), (103, 10, false)]);
}
