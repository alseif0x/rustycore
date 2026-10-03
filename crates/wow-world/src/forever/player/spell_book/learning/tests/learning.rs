use super::*;
use wow_data::forever_birth::BirthRecords;
use wow_persistence::forever::spells::{SpellLearnRow, SpellRequiredRow};

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
fn new_lower_rank_captured_active_return_is_not_its_final_active_field() {
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
    insert(&mut book, 2, PlayerSpellState::Unchanged, true, true);
    let mut effects = Effects::quiet();
    effects.world = true;
    assert!(
        book.add_spell(
            AddPlayerSpell::learned(1, true, false),
            &fixture.sources(),
            &mut effects
        )
        .unwrap()
    );
    // Source rank loop tests old.active, not old.disabled/HasSpell.
    assert!(!book.spell(1).unwrap().active());
    assert!(effects.events.contains(&Event::Message(
        SpellBookMessage::Superseded { old: 1, new: 2 },
        true,
        false
    )));
}
#[test]
fn existing_lower_rank_sees_known_next_and_preserves_superseded_transition_return() {
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
    insert(&mut book, 1, PlayerSpellState::Unchanged, true, false);
    insert(&mut book, 2, PlayerSpellState::New, false, false); // HasSpell does not require active.
    let mut effects = Effects::quiet();
    effects.world = true;
    assert!(
        !book
            .add_spell(
                AddPlayerSpell::learned(1, true, false),
                &fixture.sources(),
                &mut effects
            )
            .unwrap()
    );
    assert_eq!(
        effects.events,
        [Event::Message(
            SpellBookMessage::Superseded { old: 1, new: 2 },
            false,
            false
        )]
    );
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
fn disabled_learn_snapshots_flags_then_enables_next_and_required_without_quest_for_disabled() {
    let fixture = Fixture::new(
        vec![row(1), row(2), row(3)],
        BirthRecords {
            abilities: vec![ability(1, 1, 2, 0)],
            ..Default::default()
        },
        vec![],
        vec![SpellRequiredRow {
            spell: 3,
            required: 1,
        }],
        vec![],
    );
    let mut book = PlayerSpellBook::default();
    for id in 1..=3 {
        insert(&mut book, id, PlayerSpellState::Unchanged, false, true);
    }
    book.entries.get_mut(&1).unwrap().favorite = true;
    let mut effects = Effects::quiet();
    effects.world = true;
    book.learn_spell(
        LearnPlayerSpell::new(1, true, 7),
        &fixture.sources(),
        &mut effects,
    )
    .unwrap();
    for id in 1..=3 {
        assert!(book.has_spell(id) && !book.has_active_spell(id));
    }
    assert!(book.spell(1).unwrap().favorite());
    assert!(
        !effects
            .events
            .iter()
            .any(|event| matches!(event, Event::Quest(_)))
    );
    assert!(!effects.events.iter().any(|event| matches!(
        event,
        Event::Message(SpellBookMessage::Learned { .. }, _, _)
    )));
}
#[test]
fn learn_sends_captured_favorite_trait_suppression_and_duplicate_still_updates_quest() {
    let fixture = Fixture::simple(vec![row(1)]);
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::Removed, true, false);
    book.entries.get_mut(&1).unwrap().favorite = true;
    let mut effects = Effects::quiet();
    effects.world = true;
    let mut request = LearnPlayerSpell::new(1, false, 0);
    request.suppress_messaging = true;
    request.trait_data = Some(PlayerSpellTrait::new(-4, 2));
    book.learn_spell(request, &fixture.sources(), &mut effects)
        .unwrap();
    assert!(effects.events.contains(&Event::Message(
        SpellBookMessage::Learned {
            spell: 1,
            favorite: true,
            trait_definition: Some(-4),
            suppress_messaging: true
        },
        true,
        true
    )));
    effects.events.clear();
    book.learn_spell(request, &fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(effects.events, [Event::Quest(1)]);
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

#[test]
fn previous_rank_skill_reentry_inserts_original_before_outer_try_emplace_and_forces_changed() {
    let mut first = row(1);
    first.3.push(spell_effect(118, 0, 5, 1.));
    let fixture = Fixture::new(
        vec![first, row(2)],
        BirthRecords {
            abilities: vec![ability(1, 1, 2, 0)],
            skill_lines: vec![line(5, 7)],
            race_class: vec![rc(1, 5, 0)],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
    let sources = fixture.sources();
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    effects.skill_reenter = Some((5, 2));
    effects.reentry_sources = Some(&sources);
    assert!(
        book.add_spell(
            AddPlayerSpell::learned(2, true, false),
            &sources,
            &mut effects
        )
        .unwrap()
    );
    assert_eq!(book.spell(2).unwrap().state(), PlayerSpellState::Changed);
    assert!(!book.spell(2).unwrap().dependent());
    assert!(book.has_active_spell(2) && book.has_spell(1) && !book.has_active_spell(1));
    assert_eq!(
        book.membership,
        [SpellBookMutation::Insert(1), SpellBookMutation::Insert(2)]
    );
}

#[test]
fn cyclic_sql_learning_terminates_via_real_existing_flags_not_a_blanket_id_guard() {
    let fixture = Fixture::new(
        vec![row(1), row(2)],
        Default::default(),
        vec![],
        vec![],
        vec![
            SpellLearnRow {
                source: 1,
                learned: 2,
                active: true,
            },
            SpellLearnRow {
                source: 2,
                learned: 1,
                active: true,
            },
        ],
    );
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    assert!(
        book.add_spell(
            AddPlayerSpell::learned(1, true, false),
            &fixture.sources(),
            &mut effects
        )
        .unwrap()
    );
    assert!(book.has_active_spell(1) && book.has_active_spell(2));
    assert!(book.spell(1).unwrap().dependent() && book.spell(2).unwrap().dependent());
    assert_eq!(
        book.membership,
        [SpellBookMutation::Insert(1), SpellBookMutation::Insert(2)]
    );
    assert_eq!(effects.events, [Event::MountQuery(2), Event::MountQuery(1)]);
}
