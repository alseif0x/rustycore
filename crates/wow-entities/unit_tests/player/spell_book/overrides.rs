use super::*;

fn panic_order(_: &[SpellBookMutation], _: usize) -> Result<Vec<u32>, &'static str> {
    panic!("absent group must not call provider")
}

#[test]
fn override_group_erasure_resets_history_but_partial_erase_preserves_bucket_witness() {
    let mut book = PlayerSpellBook::default();
    book.remove_override_spell(0, 1);
    assert!(
        book.source_override_spells(0, panic_order)
            .unwrap()
            .is_empty()
    );
    book.add_override_spell(0, 0);
    book.add_override_spell(0, 1);
    book.add_override_spell(0, 1);
    book.remove_override_spell(0, 999);
    book.remove_override_spell(0, 0);
    book.source_override_spells(0, |history, count| {
        assert_eq!(count, 1);
        assert_eq!(
            history,
            [
                SpellBookMutation::Insert(0),
                SpellBookMutation::Insert(1),
                SpellBookMutation::Erase(0)
            ]
        );
        Ok::<_, &'static str>(vec![1])
    })
    .unwrap();
    book.remove_override_spell(0, 1);
    assert!(
        book.source_override_spells(0, panic_order)
            .unwrap()
            .is_empty()
    );
    book.add_override_spell(0, 1);
    book.source_override_spells(0, |history, _| {
        assert_eq!(history, [SpellBookMutation::Insert(1)]);
        Ok::<_, &'static str>(vec![1])
    })
    .unwrap();
}
