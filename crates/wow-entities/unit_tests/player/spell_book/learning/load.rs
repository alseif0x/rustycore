use super::fixtures::{Effects, Event, Fixture, SpellFacts, insert};
use crate::{
    PlayerSpellBook, PlayerSpellState, SpellLearningError, SpellLearningSourceError, SpellLoadRow,
};

fn fixture(spells: Vec<SpellFacts>) -> Fixture {
    Fixture {
        spells,
        lines: vec![],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    }
}

fn row(spell: u32, active: bool, disabled: bool) -> SpellLoadRow {
    SpellLoadRow {
        spell,
        active,
        disabled,
    }
}

#[test]
fn ordered_rows_reuse_add_spell_then_favorite_physical_entries() {
    let mut loading_custom = SpellFacts::plain(5, 5);
    let definition = loading_custom.definition.as_mut().unwrap();
    definition.custom = 0x0080_0000;
    definition
        .has_effect
        .iter_mut()
        .find(|(kind, _)| *kind == 36)
        .unwrap()
        .1 = true;

    let mut false_admission = SpellFacts::plain(7, 7);
    false_admission
        .definition
        .as_mut()
        .unwrap()
        .has_effect
        .iter_mut()
        .find(|(kind, _)| *kind == 44)
        .unwrap()
        .1 = true;

    let sources = fixture(vec![
        SpellFacts::plain(0, 0),
        SpellFacts::plain(6, 6),
        false_admission,
        SpellFacts::plain(100, 100),
        loading_custom,
        SpellFacts::plain(u32::MAX, u32::MAX),
    ]);
    assert_eq!(
        sources.spells.iter().filter(|spell| spell.id == 5).count(),
        1
    );
    let custom_definition = sources
        .spells
        .iter()
        .find(|spell| spell.id == 5)
        .unwrap()
        .definition
        .as_ref()
        .unwrap();
    assert_eq!(custom_definition.custom, 0x0080_0000);
    assert!(
        custom_definition
            .has_effect
            .iter()
            .any(|(kind, present)| *kind == 36 && *present)
    );
    let source = sources.sources();
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 100, PlayerSpellState::Unchanged, true, false);
    insert(&mut book, 200, PlayerSpellState::Removed, false, false);
    insert(&mut book, 300, PlayerSpellState::Temporary, true, true);
    let mut effects = Effects::quiet();

    book.load_spells(
        &[
            row(100, true, false),
            row(5, true, false),
            row(7, true, false),
            row(7, true, false),
            row(6, true, true),
            row(0, false, false),
            row(u32::MAX, false, false),
        ],
        &[100, 200, 300, 5, 7, 7, 6, 0, u32::MAX, 999],
        &source,
        &mut effects,
    )
    .unwrap();

    assert_eq!(book.len(), 8);
    assert!(book.has_spell(100));
    assert_eq!(
        book.spell(100).unwrap().state(),
        PlayerSpellState::Unchanged
    );
    assert!(book.spell(100).unwrap().favorite());

    let loaded = book.spell(5).unwrap();
    assert_eq!(loaded.state(), PlayerSpellState::Unchanged);
    assert!(loaded.active());
    assert!(!loaded.disabled());
    assert!(!loaded.dependent());
    assert_eq!(loaded.trait_data(), None);
    assert!(loaded.favorite());
    assert!(!effects.events.contains(&Event::Cast(5)));

    let disabled = book.spell(6).unwrap();
    assert!(disabled.active());
    assert!(disabled.disabled());
    assert!(!disabled.dependent());
    assert_eq!(disabled.state(), PlayerSpellState::Unchanged);
    assert!(disabled.favorite());

    assert_eq!(book.spell(7).unwrap().state(), PlayerSpellState::Unchanged);
    assert!(book.spell(7).unwrap().favorite());
    assert_eq!(
        effects
            .events
            .iter()
            .filter(|event| **event == Event::Cast(7))
            .count(),
        1
    );
    assert!(effects.events.contains(&Event::MountQuery(0)));
    assert!(effects.events.contains(&Event::MountQuery(u32::MAX)));
    let mount_five = effects
        .events
        .iter()
        .position(|event| *event == Event::MountQuery(5))
        .unwrap();
    let cast_seven = effects
        .events
        .iter()
        .position(|event| *event == Event::Cast(7))
        .unwrap();
    let mount_zero = effects
        .events
        .iter()
        .position(|event| *event == Event::MountQuery(0))
        .unwrap();
    let mount_max = effects
        .events
        .iter()
        .position(|event| *event == Event::MountQuery(u32::MAX))
        .unwrap();
    assert!(mount_five < cast_seven && cast_seven < mount_zero && mount_zero < mount_max);

    assert_eq!(book.spell(0).unwrap().state(), PlayerSpellState::Unchanged);
    assert_eq!(
        book.spell(u32::MAX).unwrap().state(),
        PlayerSpellState::Unchanged
    );
    assert!(book.spell(200).unwrap().favorite());
    assert_eq!(book.spell(200).unwrap().state(), PlayerSpellState::Removed);
    assert!(!book.has_spell(200));
    assert!(book.spell(300).unwrap().favorite());
    assert_eq!(
        book.spell(300).unwrap().state(),
        PlayerSpellState::Temporary
    );
    assert!(!book.has_spell(300));
    assert!(book.spell(999).is_none());
}

#[test]
fn allowed_rows_preserve_new_and_changed_states_and_reactivate_removed_nodes() {
    let sources = fixture(vec![
        SpellFacts::plain(10, 10),
        SpellFacts::plain(11, 11),
        SpellFacts::plain(12, 12),
    ]);
    let source = sources.sources();
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 10, PlayerSpellState::New, false, false);
    insert(&mut book, 11, PlayerSpellState::Changed, false, false);
    insert(&mut book, 12, PlayerSpellState::Removed, false, false);
    let mut effects = Effects {
        world: true,
        ..Effects::quiet()
    };

    book.load_spells(
        &[
            row(10, true, false),
            row(11, true, false),
            row(12, true, false),
        ],
        &[],
        &source,
        &mut effects,
    )
    .unwrap();

    assert_eq!(book.spell(10).unwrap().state(), PlayerSpellState::New);
    assert_eq!(book.spell(11).unwrap().state(), PlayerSpellState::Changed);
    // AddSpell removes a Removed entry and inserts its admitted replacement as Changed.
    assert_eq!(book.spell(12).unwrap().state(), PlayerSpellState::Changed);
    for spell in [10, 11, 12] {
        assert!(book.spell(spell).unwrap().active());
        assert!(!book.spell(spell).unwrap().favorite());
    }
}

#[test]
fn mandatory_effect_error_keeps_row_prefix_and_skips_favorites() {
    let sources = fixture(vec![
        SpellFacts::plain(10, 10),
        SpellFacts::plain(20, 20),
        SpellFacts::plain(30, 30),
    ]);
    let source = sources.sources();
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 40, PlayerSpellState::Unchanged, true, false);
    book.entries.get_mut(&40).unwrap().favorite = true;
    let mut effects = Effects {
        fail: Some(Event::MountQuery(20)),
        ..Effects::quiet()
    };

    let result = book.load_spells(
        &[
            row(10, true, false),
            row(20, true, false),
            row(30, true, false),
        ],
        &[10, 40],
        &source,
        &mut effects,
    );

    assert_eq!(
        result,
        Err(SpellLearningError::Effect("required effect failed"))
    );
    assert!(book.spell(10).is_some());
    assert!(book.spell(20).is_some());
    assert!(book.spell(30).is_none());
    assert!(!book.spell(10).unwrap().favorite());
    assert!(book.spell(40).unwrap().favorite());
}

#[test]
fn out_of_world_invalid_row_retains_global_cleanup_hold_and_prefix() {
    let mut invalid = SpellFacts::plain(20, 20);
    invalid.valid = false;
    let sources = fixture(vec![
        SpellFacts::plain(10, 10),
        invalid,
        SpellFacts::plain(30, 30),
    ]);
    let source = sources.sources();
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 40, PlayerSpellState::Unchanged, true, false);
    book.entries.get_mut(&40).unwrap().favorite = true;
    let mut effects = Effects::quiet();

    let result = book.load_spells(
        &[
            row(10, true, false),
            row(20, true, false),
            row(30, true, false),
        ],
        &[10, 40],
        &source,
        &mut effects,
    );

    assert!(matches!(
        result,
        Err(SpellLearningError::Source(
            SpellLearningSourceError::UnauthorizedGlobalCleanup
        ))
    ));
    assert!(book.spell(10).is_some());
    assert!(book.spell(20).is_none());
    assert!(book.spell(30).is_none());
    assert!(!book.spell(10).unwrap().favorite());
    assert!(book.spell(40).unwrap().favorite());
}
