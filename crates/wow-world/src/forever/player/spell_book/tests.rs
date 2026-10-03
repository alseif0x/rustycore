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

#[test]
fn provider_receives_real_membership_and_output_borrows_canonical_state() {
    let mut book = PlayerSpellBook::default();
    book.add_temporary_spell(30);
    book.add_temporary_spell(10);
    book.remove_temporary_spell(30);
    book.add_temporary_spell(30);
    book.set_spell_favorite(10, true);
    let result = book
        .source_entries(|events, count| {
            assert_eq!(
                events,
                [
                    SpellBookMutation::Insert(30),
                    SpellBookMutation::Insert(10),
                    SpellBookMutation::Erase(30),
                    SpellBookMutation::Insert(30)
                ]
            );
            assert_eq!(count, 2);
            Ok::<_, ()>(vec![10, 30])
        })
        .unwrap();
    assert_eq!(
        result.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        [10, 30]
    );
    assert!(std::ptr::eq(result[0].1, book.spell(10).unwrap()));
    assert!(result[0].1.favorite());
}

#[test]
fn bad_provider_output_and_failure_never_mutate_the_book() {
    let mut book = PlayerSpellBook::default();
    book.add_temporary_spell(1);
    book.add_temporary_spell(2);
    for keys in [vec![], vec![1], vec![1, 1], vec![1, 3], vec![1, 2, 3]] {
        assert_eq!(
            book.source_entries(|_, _| Ok::<_, ()>(keys)).unwrap_err(),
            SpellBookOrderError::InvalidKeySet
        );
    }
    assert_eq!(
        book.source_entries(|_, _| Err::<Vec<u32>, _>(42))
            .unwrap_err(),
        SpellBookOrderError::Source(42)
    );
    assert_eq!(book.len(), 2);
    assert_eq!(book.membership.len(), 2);
    assert!(book.has_active_spell(1) && book.has_active_spell(2));
    let empty = PlayerSpellBook::default();
    assert!(
        empty
            .source_entries(|_, _| Ok::<_, ()>(vec![]))
            .unwrap()
            .is_empty()
    );
}
