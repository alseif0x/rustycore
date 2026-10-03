use super::*;
use wow_world::forever::player::PlayerSpellBook;

#[test]
fn numeric_mutation_layout_matches_native_abi() {
    assert_eq!(std::mem::size_of::<Mutation>(), 8);
    assert_eq!(std::mem::align_of::<Mutation>(), 4);
    assert_eq!(std::mem::offset_of!(Mutation, action), 4);
}

#[test]
fn production_provider_borrows_canonical_player_book_after_erase_and_reinsert() {
    let mut book = PlayerSpellBook::default();
    for id in 0..257 {
        book.add_temporary_spell(id);
    }
    for id in (0..257).step_by(3) {
        book.remove_temporary_spell(id);
    }
    for id in (0..257).step_by(3).rev() {
        book.add_temporary_spell(id);
    }
    book.set_spell_favorite(0, true);
    let first = book.source_entries(order).unwrap();
    assert_eq!(first.len(), 257);
    let keys = first
        .iter()
        .map(|(id, entry)| {
            assert!(std::ptr::eq(*entry, book.spell(*id).unwrap()));
            assert!(entry.active());
            *id
        })
        .collect::<Vec<_>>();
    assert!(first.iter().find(|(id, _)| *id == 0).unwrap().1.favorite());
    // Replay is deterministic without retained native state. Independent C++
    // oracle, not this test, proves agreement with the actual PlayerSpellMap.
    assert_eq!(
        book.source_entries(order)
            .unwrap()
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>(),
        keys
    );
}

#[test]
fn history_rejects_impossible_membership_and_wrong_count_without_output_changes() {
    for (history, capacity) in [
        (
            vec![
                Mutation {
                    spell: 7,
                    action: 1
                };
                2
            ],
            1,
        ),
        (
            vec![Mutation {
                spell: 7,
                action: 2,
            }],
            0,
        ),
        (
            vec![Mutation {
                spell: 7,
                action: 0,
            }],
            1,
        ),
        (
            vec![Mutation {
                spell: 7,
                action: 3,
            }],
            1,
        ),
        (
            vec![Mutation {
                spell: 7,
                action: 1,
            }],
            0,
        ),
        (
            vec![Mutation {
                spell: 7,
                action: 1,
            }],
            2,
        ),
    ] {
        let mut output = vec![99; capacity];
        let mut written = 77;
        let code = unsafe {
            rustycore_forever_spell_book_order(
                history.as_ptr(),
                history.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut written,
            )
        };
        assert_eq!(code, 1);
        assert_eq!(written, 0);
        assert!(output.iter().all(|id| *id == 99));
    }
}

#[test]
fn null_contract_and_empty_or_fully_erased_histories() {
    use std::ptr::{null, null_mut};
    let mut written = 77;
    assert_eq!(
        unsafe { rustycore_forever_spell_book_order(null(), 0, null_mut(), 0, &mut written) },
        0
    );
    assert_eq!(written, 0);
    let history = [
        Mutation {
            spell: u32::MAX,
            action: 1,
        },
        Mutation {
            spell: u32::MAX,
            action: 2,
        },
    ];
    assert_eq!(
        unsafe {
            rustycore_forever_spell_book_order(history.as_ptr(), 2, null_mut(), 0, &mut written)
        },
        0
    );
    assert_eq!(written, 0);
    assert_eq!(
        unsafe { rustycore_forever_spell_book_order(null(), 1, null_mut(), 0, &mut written) },
        1
    );
    assert_eq!(written, 0);
    assert_eq!(
        unsafe {
            rustycore_forever_spell_book_order(history.as_ptr(), 1, null_mut(), 1, &mut written)
        },
        1
    );
    assert_eq!(written, 0);
    assert_eq!(
        unsafe { rustycore_forever_spell_book_order(null(), 0, null_mut(), 0, null_mut()) },
        1
    );
}
