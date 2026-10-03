//! Production ABI + domain key admission, not applied Unit/client acceptance.
use super::*;
use wow_world::forever::player::PlayerSpellBook;

#[test]
fn production_set_provider_reads_canonical_group_after_partial_erase_and_reinsert() {
    let mut book = PlayerSpellBook::default();
    for id in 0..257 {
        book.add_override_spell(0, id);
    }
    for id in (0..257).step_by(3) {
        book.remove_override_spell(0, id);
    }
    for id in (0..257).step_by(3).rev() {
        book.add_override_spell(0, id);
    }
    book.add_override_spell(0, u32::MAX);
    let keys = book.source_override_spells(0, override_order).unwrap();
    assert_eq!(keys.len(), 258);
    assert!(keys.contains(&0) && keys.contains(&u32::MAX));
    assert!(keys.iter().all(|&id| book.has_override_spell(0, id)));
    assert_eq!(
        keys,
        book.source_override_spells(0, override_order).unwrap()
    );
    assert!(book.is_empty()); // replacements do not require known PlayerSpells
}

#[test]
fn deleting_final_override_resets_native_group_history() {
    let mut book = PlayerSpellBook::default();
    for id in 0..257 {
        book.add_override_spell(1, id);
    }
    for id in 0..257 {
        book.remove_override_spell(1, id);
    }
    assert!(
        book.source_override_spells(1, override_order)
            .unwrap()
            .is_empty()
    );
    let ids = [u32::MAX, 7, 131, 257, 0, 4, 12];
    let mut fresh = PlayerSpellBook::default();
    for id in ids {
        book.add_override_spell(1, id);
        fresh.add_override_spell(1, id);
    }
    assert_eq!(
        book.source_override_spells(1, override_order).unwrap(),
        fresh.source_override_spells(1, override_order).unwrap()
    );
}

#[test]
fn set_history_and_capacity_errors_leave_outputs_untouched() {
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
            rustycore_forever_spell_override_order(
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
fn set_null_empty_and_fully_erased_contract() {
    use std::ptr::{null, null_mut};
    let mut written = 77;
    assert_eq!(
        unsafe { rustycore_forever_spell_override_order(null(), 0, null_mut(), 0, &mut written) },
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
            rustycore_forever_spell_override_order(history.as_ptr(), 2, null_mut(), 0, &mut written)
        },
        0
    );
    assert_eq!(written, 0);
    assert_eq!(
        unsafe { rustycore_forever_spell_override_order(null(), 1, null_mut(), 0, &mut written) },
        1
    );
    assert_eq!(written, 0);
    assert_eq!(
        unsafe {
            rustycore_forever_spell_override_order(history.as_ptr(), 1, null_mut(), 1, &mut written)
        },
        1
    );
    assert_eq!(written, 0);
    assert_eq!(
        unsafe { rustycore_forever_spell_override_order(null(), 0, null_mut(), 0, null_mut()) },
        1
    );
}
