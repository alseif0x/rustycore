use super::*;
use crate::forever::player::SkillUpdateState::{Changed, Deleted, New, Unchanged};

#[test]
fn deactivate_preserves_line_resets_all_other_fields_and_keeps_source_spell_removal_order() {
    let fixture = Fixture::new(
        vec![line(10, 6, 0, 0)],
        vec![
            ability(3, 10, 100, 101),
            ability(1, 10, 100, 101),
            ability(2, 10, 0, u32::MAX),
        ],
    );
    for state in [Unchanged, Changed, New, Deleted] {
        let mut skills = allocated([10]);
        activate(&mut skills, 10, 7, 75, state);
        skills.fields[0].temporary_bonus = -5;
        skills.fields[0].permanent_bonus = 10;
        skills.fields[0].starting_rank = 3;
        let mut effects = Effects::default();
        skills
            .set_skill(update(10, 20, 0, 300), &fixture.sources(), &mut effects)
            .unwrap();
        assert_eq!(
            effects.events,
            [
                Event::Enchant(10, 7, 0, if state == Deleted { 0 } else { 7 }),
                Event::Remove(100),
                Event::Remove(u32::MAX),
                Event::Remove(100)
            ]
        );
        assert_eq!(
            skills.status(10),
            Some((0, if state == New { Unchanged } else { Deleted }))
        );
        let field = skills.fields[0];
        assert_eq!(
            (
                field.line(),
                field.step(),
                field.rank(),
                field.starting_rank(),
                field.maximum(),
                field.temporary_bonus(),
                field.permanent_bonus()
            ),
            (10, 0, 0, 1, 0, 0, 0)
        );
    }
}

#[test]
fn full_inventory_preserves_primary_skill_and_previous_moves_but_still_syncs_children() {
    let fixture = Fixture::new(vec![line(10, 11, 0, 0), line(20, 6, 10, 4)], rewards(&[20]));
    let mut skills = allocated([10, 20]);
    activate(&mut skills, 10, 75, 150, New);
    activate(&mut skills, 20, 10, 75, New);
    skills.professions = [10, 0];
    let mut effects = Effects {
        full_at: Some(20),
        ..Default::default()
    };
    assert_eq!(
        skills.set_skill(update(10, 0, 0, 0), &fixture.sources(), &mut effects),
        Ok(SkillSetOutcome::InventoryFull)
    );
    assert_eq!(
        &effects.events[..3],
        &[Event::Store(19), Event::Store(20), Event::Full]
    );
    assert!(!effects.events.contains(&Event::Store(21)));
    assert_eq!(
        (
            skills.pure_value(10),
            skills.pure_maximum(10),
            skills.status(10)
        ),
        (75, 150, Some((0, New)))
    );
    assert_eq!(skills.profession_lines(), &[10, 0]);
    assert_eq!((skills.pure_value(20), skills.pure_maximum(20)), (75, 150));
    assert!(matches!(
        effects.events.get(3),
        Some(Event::Learn(20, 75, Some(New)))
    ));
}

#[test]
fn successful_profession_unlearn_moves_three_second_slot_items_before_enchants_and_child_clear() {
    let fixture = Fixture::new(vec![line(10, 11, 0, 0), line(20, 6, 10, 4)], vec![]);
    let mut skills = allocated([10, 20]);
    activate(&mut skills, 10, 75, 150, New);
    activate(&mut skills, 20, 75, 150, Changed);
    skills.professions = [99, 10];
    let mut effects = Effects::default();
    skills
        .set_skill(update(10, 0, 0, 0), &fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(
        effects.events,
        [
            Event::Store(22),
            Event::Store(23),
            Event::Store(24),
            Event::Enchant(10, 75, 0, 75),
            Event::Enchant(20, 75, 0, 75)
        ]
    );
    assert_eq!(skills.profession_lines(), &[99, 0]);
    assert_eq!(skills.status(10), Some((0, Unchanged)));
    assert_eq!(skills.status(20), Some((1, Deleted)));
    assert_eq!((skills.pure_value(10), skills.pure_value(20)), (0, 0));
}

#[test]
fn zero_rank_stored_request_is_noop_and_missing_classic_parent_forces_child_deactivation() {
    let fixture = Fixture::new(vec![line(10, 11, 0, 0), line(20, 6, 10, 4)], vec![]);
    let mut skills = allocated([10, 20]);
    let fields = skills.fields.clone();
    let mut effects = Effects::default();
    skills
        .set_skill(update(10, 20, 0, 300), &fixture.sources(), &mut effects)
        .unwrap();
    assert!(skills.fields.iter().zip(fields.iter()).all(|(a, b)| a == b));
    assert!(effects.events.is_empty());
    activate(&mut skills, 20, 10, 75, New);
    skills
        .set_skill(update(20, 4, 1, 300), &fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(effects.events, [Event::Enchant(20, 10, 0, 10)]);
    assert_eq!(skills.status(20), Some((1, Unchanged)));
    assert_eq!(skills.pure_value(10), 0);
}
