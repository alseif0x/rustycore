use super::*;
use crate::SkillAbilityMetadata;
use crate::SkillUpdateState::Unchanged;

fn row(id: u32, method: i32, spell: u32, minimum: i16) -> SkillAbilityMetadata {
    let mut row = ability(id, 10, spell);
    row.acquire_method = method;
    row.min_skill_rank = minimum;
    row
}
fn fixture(rows: Vec<SkillAbilityMetadata>) -> Fixture {
    Fixture::new(vec![line(10, 6, 0, 0)], rows)
}

#[test]
fn exact_acquire_methods_storage_order_and_missing_definitions_control_dispatch() {
    let mut fixture = fixture(vec![
        row(1, 0, 101, 0),
        row(2, 3, 102, 0),
        row(3, 5, 103, 0),
        row(4, 1, 104, 0),
        row(5, 2, 105, 0),
        row(6, 4, 106, 0),
        row(7, 4, 107, 0),
    ]);
    spell_fields(&mut fixture, 106, Some((0, 0, 8)));
    spell_fields(&mut fixture, 107, None);
    let mut skills = allocated([10]);
    activate(&mut skills, 10, 7, 75, Unchanged);
    let fields = skills.fields.clone();
    let mut effects = Effects::default();
    skills
        .learn_skill_rewards(10, 7, 1, fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(
        effects.events,
        [
            Event::Learn(10, 7, Some(Unchanged)),
            Event::Learn(10, 7, Some(Unchanged)),
            Event::PlayerCondition(8),
            Event::AbilityConditions(6),
            Event::Learn(10, 7, Some(Unchanged))
        ]
    );
    assert_eq!(
        effects.grants,
        [(104, 10, false), (105, 10, false), (106, 10, false)]
    );
    assert!(skills.fields.iter().zip(fields.iter()).all(|(a, b)| a == b));
    assert_eq!(skills.status(10), Some((0, Unchanged)));
}

#[test]
fn learned_or_automatic_checks_conditions_before_masks_and_short_circuits_in_source_order() {
    let mut wrong_race = row(3, 4, 103, 0);
    wrong_race.race_mask = 2;
    let mut wrong_class = row(4, 4, 104, 0);
    wrong_class.class_mask = 2;
    let mut ordinary = row(5, 2, 105, 0);
    ordinary.class_mask = 2;
    let mut fixture = fixture(vec![
        row(1, 4, 101, 0),
        row(2, 4, 102, 0),
        wrong_race,
        wrong_class,
        ordinary,
    ]);
    for (spell, condition) in [(101, 11), (102, 12), (104, 14), (105, 99)] {
        spell_fields(&mut fixture, spell, Some((0, 0, condition)));
    }
    let mut skills = allocated([10]);
    let mut effects = Effects {
        deny_player: Some(11),
        deny_ability: Some(2),
        ..Default::default()
    };
    skills
        .learn_skill_rewards(10, 7, 1, fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(
        effects.events,
        [
            Event::PlayerCondition(11),
            Event::PlayerCondition(12),
            Event::AbilityConditions(2),
            Event::AbilityConditions(3),
            Event::PlayerCondition(14),
            Event::AbilityConditions(4)
        ]
    );
    assert!(effects.grants.is_empty());
    assert_eq!(effects.level_queries.get(), 0);
    assert_eq!(effects.world_queries.get(), 0);
}

#[test]
fn rank_comparison_narrows_uint32_value_to_int32_and_preserves_signed_minimum() {
    let fixture = fixture(vec![
        row(1, 1, 101, 1),
        row(2, 1, 102, -1),
        row(3, 1, 103, i16::MIN),
    ]);
    let mut skills = allocated([10]);
    let mut effects = Effects::default();
    skills
        .learn_skill_rewards(10, u32::MAX, 1, fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(effects.events.first(), Some(&Event::Remove(101)));
    assert_eq!(effects.grants, [(102, 10, false), (103, 10, false)]);
}

#[test]
fn required_level_uses_both_spell_and_base_after_conditions_but_before_world_query() {
    let mut fixture = fixture(vec![
        row(1, 4, 101, 0),
        row(2, 4, 102, 0),
        row(3, 2, 103, 0),
    ]);
    for (spell, base, level) in [(101, 1, 5), (102, 6, 0), (103, 2, 3)] {
        spell_fields(&mut fixture, spell, Some((base, level, 0)));
    }
    let mut skills = allocated([10]);
    let mut effects = Effects {
        level: Some(3),
        ..Default::default()
    };
    skills
        .learn_skill_rewards(10, 7, 1, fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(
        effects.events,
        [
            Event::AbilityConditions(1),
            Event::AbilityConditions(2),
            Event::Learn(10, 0, Some(Unchanged))
        ]
    );
    assert_eq!(effects.grants, [(103, 10, false)]);
    assert_eq!(effects.level_queries.get(), 3);
    assert_eq!(effects.world_queries.get(), 1);
}

#[test]
fn source_add_false_does_not_stop_later_entries_and_world_state_is_queried_live() {
    let fixture = fixture(vec![row(1, 2, 101, 0), row(2, 2, 102, 0)]);
    let mut skills = allocated([10]);
    let mut effects = Effects {
        add_false: true,
        flip_world_after_grant: true,
        ..Default::default()
    };
    skills
        .learn_skill_rewards(10, 7, 1, fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(effects.grants, [(101, 10, false), (102, 10, true)]);
    assert_eq!(effects.world_queries.get(), 2);
    assert!(!effects.in_world);
}

#[test]
fn unavailable_condition_retains_completed_prefix_without_later_grants_or_speculative_queries() {
    let mut fixture = fixture(vec![
        row(1, 2, 101, 0),
        row(2, 4, 102, 0),
        row(3, 2, 103, 0),
    ]);
    spell_fields(&mut fixture, 102, Some((0, 0, 9)));
    let mut skills = allocated([10]);
    let mut effects = Effects {
        fail_at: Some(1),
        ..Default::default()
    };
    assert_eq!(
        skills.learn_skill_rewards(10, 7, 1, fixture.sources(), &mut effects),
        Err(SkillSetError::Effect("unavailable required effect"))
    );
    assert_eq!(
        effects.events,
        [
            Event::Learn(10, 0, Some(Unchanged)),
            Event::PlayerCondition(9)
        ]
    );
    assert_eq!(effects.grants, [(101, 10, false)]);
    assert_eq!(effects.level_queries.get(), 1);
    assert_eq!(effects.world_queries.get(), 1);
}
