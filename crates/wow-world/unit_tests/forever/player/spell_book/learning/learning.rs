use super::*;
use wow_data::forever_birth::BirthRecords;
use wow_persistence::forever::spells::SpellLearnRow;

#[test]
fn higher_rank_learns_previous_and_supersedes_before_old_flag_mutation() {
    let fixture = Fixture::new(
        vec![row(1), row(2)],
        BirthRecords {
            abilities: vec![ability(1, 1, 2, 0)],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    effects.world = true;
    book.learn_spell(
        LearnPlayerSpell::new(2, false, 0),
        &fixture.sources(),
        &mut effects,
    )
    .unwrap();
    assert!(book.has_spell(1) && book.has_active_spell(2));
    assert!(!book.has_active_spell(1));
    assert!(effects.events.contains(&Event::Message(
        SpellBookMessage::Superseded { old: 1, new: 2 },
        true,
        true
    )));
    assert!(!effects.events.iter().any(|event| matches!(
        event,
        Event::Message(SpellBookMessage::Learned { spell: 2, .. }, _, _)
    ))); // Add bool suppresses duplicate client learn.
    assert_eq!(effects.events.last(), Some(&Event::Quest(2)));
}

#[test]
fn dependent_relationships_recurse_into_real_book_and_preserve_inactive_override_gates() {
    let fixture = Fixture::new(
        vec![row(1), row(2), row(3)],
        Default::default(),
        vec![],
        vec![],
        vec![
            SpellLearnRow {
                source: 1,
                learned: 2,
                active: false,
            },
            SpellLearnRow {
                source: 1,
                learned: 3,
                active: true,
            },
        ],
    );
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    effects.world = true;
    book.add_spell(
        AddPlayerSpell::learned(1, true, false),
        &fixture.sources(),
        &mut effects,
    )
    .unwrap();
    assert!(book.has_spell(2) && !book.has_active_spell(2));
    assert!(book.spell(2).unwrap().dependent());
    assert!(book.has_active_spell(3));
    assert!(effects.events.contains(&Event::Message(
        SpellBookMessage::Learned {
            spell: 3,
            favorite: false,
            trait_definition: None,
            suppress_messaging: false
        },
        true,
        true
    )));
    assert!(!effects.events.contains(&Event::Quest(2)));
    assert!(effects.events.contains(&Event::Quest(3)));
}

#[test]
fn malformed_order_fails_after_node_insertion_without_cast_or_publication() {
    let fixture = Fixture::new(
        vec![row(1), row(2)],
        BirthRecords {
            abilities: vec![ability(1, 1, 2, 0)],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    effects.bad_order = true;
    assert_eq!(
        book.add_spell(
            AddPlayerSpell::learned(1, true, false),
            &fixture.sources(),
            &mut effects
        ),
        Err(SpellLearningError::Source(
            SpellLearningSourceError::InvalidBookOrder
        ))
    );
    assert!(book.has_active_spell(1));
    assert_eq!(effects.events, [Event::Order]);
}

#[test]
fn casts_and_mount_callbacks_reenter_same_book_without_clobbering_existing_fields() {
    let mut spell = row(1);
    spell.1.attributes[1] = 0x80000000;
    let fixture = Fixture::simple(vec![spell, row(2)]);
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    effects.cast_insert = Some(2);
    effects.mounts.insert(1);
    effects.mount_reenter = true;
    book.add_spell(
        AddPlayerSpell::learned(1, true, false),
        &fixture.sources(),
        &mut effects,
    )
    .unwrap();
    assert_eq!(book.spell(1).unwrap().state(), PlayerSpellState::New);
    assert_eq!(book.spell(2).unwrap().state(), PlayerSpellState::Temporary);
    assert_eq!(
        effects.events,
        [
            Event::Cast(1),
            Event::MountQuery(1),
            Event::Mount(1, true, true)
        ]
    );
}

#[test]
fn criteria_preserve_duplicate_skill_lines_and_mount_follows_all_admitted_learning() {
    let fixture = Fixture::new(
        vec![row(1)],
        BirthRecords {
            abilities: vec![ability(1, 0, 1, 5), ability(2, 0, 1, 5)],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    effects.loading = false;
    effects.mounts.insert(1);
    book.add_spell(
        AddPlayerSpell::learned(1, true, false),
        &fixture.sources(),
        &mut effects,
    )
    .unwrap();
    assert_eq!(
        effects.events,
        [
            Event::Criterion(SpellLearnCriterion::TradeskillSkillLine, 5),
            Event::Criterion(SpellLearnCriterion::SpellFromSkillLine, 5),
            Event::Criterion(SpellLearnCriterion::TradeskillSkillLine, 5),
            Event::Criterion(SpellLearnCriterion::SpellFromSkillLine, 5),
            Event::Criterion(SpellLearnCriterion::LearnOrKnowSpell, 1),
            Event::MountQuery(1),
            Event::Mount(1, true, true)
        ]
    );
}
