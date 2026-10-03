use super::*;
use std::collections::BTreeSet;
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp,
    forever_birth::{BirthRecords, SkillLineRecord, SkillRaceClassRecord},
};

#[test]
fn abi_layout_and_empty_inputs_keep_explicit_native_contract() {
    assert_eq!(size_of::<Key>(), 8);
    assert_eq!(align_of::<Key>(), 4);
    assert_eq!(std::mem::offset_of!(Key, record), 4);
    assert!(replay(&[]).unwrap().is_empty());
}

#[test]
fn invalid_pointer_capacity_or_storage_history_never_writes_a_partial_output() {
    let valid = [
        Key {
            skill: 5,
            record: 1,
        },
        Key {
            skill: 5,
            record: 2,
        },
    ];
    for (pointer, length, capacity) in [
        (std::ptr::null(), 2, 2),
        (valid.as_ptr(), 2, 1),
        (valid.as_ptr(), 2, 3),
    ] {
        let mut sentinel = [99u32; 3];
        let mut written = 77;
        assert_eq!(
            unsafe {
                rustycore_forever_birth_skill_lookup(
                    pointer,
                    length,
                    sentinel.as_mut_ptr(),
                    capacity,
                    &mut written,
                )
            },
            1
        );
        assert_eq!(sentinel, [99; 3]);
        assert_eq!(written, 0);
    }
    for inputs in [
        [
            Key {
                skill: 5,
                record: 2,
            },
            Key {
                skill: 5,
                record: 1,
            },
        ],
        [
            Key {
                skill: 5,
                record: 1,
            },
            Key {
                skill: 6,
                record: 1,
            },
        ],
    ] {
        let mut sentinel = [99u32; 2];
        let mut written = 77;
        assert_eq!(
            unsafe {
                rustycore_forever_birth_skill_lookup(
                    inputs.as_ptr(),
                    2,
                    sentinel.as_mut_ptr(),
                    2,
                    &mut written,
                )
            },
            1
        );
        assert_eq!(sentinel, [99; 2]);
        assert_eq!(written, 0);
        assert!(replay(&inputs).is_err());
    }
    let mut written = 77;
    assert_eq!(
        unsafe {
            rustycore_forever_birth_skill_lookup(
                valid.as_ptr(),
                2,
                std::ptr::null_mut(),
                2,
                &mut written,
            )
        },
        1
    );
    assert_eq!(written, 0);
    assert_eq!(
        unsafe {
            rustycore_forever_birth_skill_lookup(
                valid.as_ptr(),
                2,
                std::ptr::null_mut(),
                2,
                std::ptr::null_mut(),
            )
        },
        1
    );
}

#[test]
fn native_equal_range_grouping_is_stateless_exact_set_and_repeatable_not_guessed_storage_order() {
    let inputs = (0..512u32)
        .map(|id| Key {
            skill: if id % 3 == 0 { u32::MAX } else { id % 19 },
            record: id,
        })
        .collect::<Vec<_>>();
    let expected = replay(&inputs).unwrap();
    assert_eq!(
        expected.iter().copied().collect::<BTreeSet<_>>(),
        (0..512).collect()
    );
    assert!(
        expected
            .windows(2)
            .all(|ids| inputs[ids[0] as usize].skill <= inputs[ids[1] as usize].skill)
    );
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let inputs = &inputs;
            let expected = &expected;
            scope.spawn(move || assert_eq!(&replay(inputs).unwrap(), expected));
        }
    });
    // Determinism/set checks do not prove exact reference ordering; the
    // independent pointer-payload differential remains separate acceptance.
}

#[test]
fn producer_uses_final_storage_order_and_excludes_rc_rows_with_missing_skill_lines() {
    let line = SkillLineRecord {
        id: 10,
        category: 6,
        spell_icon_file: 0,
        can_link: 0,
        parent_skill: 0,
        parent_tier_index: 0,
        flags: 0,
        spell_book_spell: 0,
        expansion_name_shared_string: 0,
        horde_expansion_name_shared_string: 0,
    };
    let rc = |id, skill| SkillRaceClassRecord {
        id,
        skill,
        class_mask: 0,
        flags: 0,
        availability: 0,
        min_level: 100,
        tier: 0,
        race_mask: 0,
    };
    let rows = BirthRecords {
        skill_lines: vec![line],
        race_class: vec![rc(8, 10), rc(2, 99), rc(1, 10)],
        ..Default::default()
    };
    let birth = rows
        .finish(
            Default::default(),
            Default::default(),
            &Db2HotfixRemovalStoreLikeCpp::default(),
        )
        .unwrap();
    let actual = order(&birth).unwrap();
    assert_eq!(
        actual,
        replay(&[
            Key {
                skill: 10,
                record: 1
            },
            Key {
                skill: 10,
                record: 8
            }
        ])
        .unwrap()
    );
    assert_eq!(
        actual.iter().copied().collect::<BTreeSet<_>>(),
        BTreeSet::from([1, 8])
    );
}
