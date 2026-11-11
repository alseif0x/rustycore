use super::*;

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
