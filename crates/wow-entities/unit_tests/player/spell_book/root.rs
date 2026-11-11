use super::*;

#[test]
fn queries_retain_source_state_active_and_disabled_rules() {
    let mut book = PlayerSpellBook::default();
    assert!(!book.has_spell(0));
    assert!(!book.has_active_spell(0));
    for state in [
        PlayerSpellState::Unchanged,
        PlayerSpellState::Changed,
        PlayerSpellState::New,
        PlayerSpellState::Removed,
        PlayerSpellState::Temporary,
    ] {
        for active in [false, true] {
            for disabled in [false, true] {
                let (entry, _) = book.try_emplace(0);
                *entry = PlayerSpellEntry {
                    state,
                    active,
                    disabled,
                    ..Default::default()
                };
                let known = state != PlayerSpellState::Removed && !disabled;
                assert_eq!(book.has_spell(0), known);
                assert_eq!(book.has_active_spell(0), known && active);
            }
        }
    }
    assert_eq!(book.membership, [SpellBookMutation::Insert(0)]);
}

#[test]
fn temporary_spell_has_value_initialized_favorite_and_trait_and_accepts_zero() {
    let mut book = PlayerSpellBook::default();
    book.add_temporary_spell(0);
    let entry = book.spell(0).unwrap();
    assert_eq!(entry.state(), PlayerSpellState::Temporary);
    assert!(entry.active());
    assert!(!entry.disabled() && !entry.dependent() && !entry.favorite());
    assert_eq!(entry.trait_data(), None);
    assert!(book.has_spell(0) && book.has_active_spell(0));
}

#[test]
fn temporary_add_never_overwrites_any_existing_node() {
    let mut book = PlayerSpellBook::default();
    for (id, state) in [
        PlayerSpellState::Unchanged,
        PlayerSpellState::Changed,
        PlayerSpellState::New,
        PlayerSpellState::Removed,
        PlayerSpellState::Temporary,
    ]
    .into_iter()
    .enumerate()
    {
        let before = PlayerSpellEntry {
            state,
            active: false,
            dependent: true,
            disabled: true,
            favorite: true,
            trait_data: Some(PlayerSpellTrait::new(-2, -3)),
        };
        *book.try_emplace(id as u32).0 = before;
        book.add_temporary_spell(id as u32);
        assert_eq!(book.spell(id as u32), Some(&before));
    }
    assert_eq!(book.membership.len(), 5);
}

#[test]
fn temporary_removal_only_erases_temporary_nodes_and_keeps_history() {
    let mut book = PlayerSpellBook::default();
    book.remove_temporary_spell(9);
    for (id, state) in [
        PlayerSpellState::Unchanged,
        PlayerSpellState::Changed,
        PlayerSpellState::New,
        PlayerSpellState::Removed,
        PlayerSpellState::Temporary,
    ]
    .into_iter()
    .enumerate()
    {
        book.try_emplace(id as u32).0.state = state;
        book.remove_temporary_spell(id as u32);
        assert_eq!(
            book.spell(id as u32).is_none(),
            state == PlayerSpellState::Temporary
        );
    }
    assert_eq!(book.len(), 4);
    assert_eq!(book.membership.last(), Some(&SpellBookMutation::Erase(4)));
    book.remove_temporary_spell(4);
    book.add_temporary_spell(4);
    assert_eq!(book.membership.len(), 7);
    assert_eq!(book.membership.last(), Some(&SpellBookMutation::Insert(4)));
}

#[test]
fn favorite_changes_only_unchanged_state_even_for_same_value() {
    let mut book = PlayerSpellBook::default();
    book.set_spell_favorite(88, true);
    assert!(book.is_empty());
    for (id, state) in [
        PlayerSpellState::Unchanged,
        PlayerSpellState::Changed,
        PlayerSpellState::New,
        PlayerSpellState::Removed,
        PlayerSpellState::Temporary,
    ]
    .into_iter()
    .enumerate()
    {
        let id = id as u32;
        book.try_emplace(id).0.state = state;
        book.set_spell_favorite(id, false);
        assert_eq!(
            book.spell(id).unwrap().state(),
            if state == PlayerSpellState::Unchanged {
                PlayerSpellState::Changed
            } else {
                state
            }
        );
        book.set_spell_favorite(id, true);
        assert!(book.spell(id).unwrap().favorite());
    }
    assert_eq!(book.membership.len(), 5);
}

#[test]
fn signed_trait_fields_narrow_like_source_bit_fields() {
    for (input, expected) in [
        (0, 0),
        (0x7fffff, 0x7fffff),
        (0x800000, -0x800000),
        (0xffffff, -1),
        (0x1000000, 0),
        (i32::MAX, -1),
        (i32::MIN, 0),
        (-1, -1),
    ] {
        let value = PlayerSpellTrait::new(input, 128);
        assert_eq!(value.definition_id(), expected);
        assert_eq!(value.rank(), -128);
    }
    assert_eq!(PlayerSpellTrait::new(7, 255).rank(), -1);
    assert_eq!(PlayerSpellTrait::new(7, 256).rank(), 0);
    assert_eq!(PlayerSpellTrait::new(7, -129).rank(), 127);
}
