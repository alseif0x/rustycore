//! Adapter/ABI feedback only. The independent extracted-source oracle is separate.
use super::*;
use wow_entities::{PlayerSkills, SkillStatusOrderError, SkillUpdateState};

#[test]
fn canonical_borrowed_statuses_use_native_replay_with_duplicate_zero_and_wide_ids() {
    let skills = PlayerSkills::initialize([10, 10, 0, 65536, u32::MAX]);
    let entries = skills.source_statuses(order).unwrap();
    assert_eq!(entries.len(), 4);
    let keys = entries
        .iter()
        .map(|entry| entry.skill())
        .collect::<Vec<_>>();
    assert_eq!(
        keys.iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>(),
        [0, 10, 65536, u32::MAX].into()
    );
    for entry in &entries {
        let expected_slot = match entry.skill() {
            10 => 0,
            0 => 2,
            65536 => 3,
            u32::MAX => 4,
            _ => unreachable!(),
        };
        assert_eq!(entry.slot(), expected_slot);
        assert_eq!(entry.state(), SkillUpdateState::Unchanged);
        assert_eq!(entry.fields().rank(), 0);
        assert!(!skills.has_skill(entry.skill()));
        assert!(std::ptr::eq(
            entry.fields(),
            &skills.fields()[expected_slot]
        ));
    }
    assert_eq!(skills.status(10), Some((0, SkillUpdateState::Unchanged)));
    assert_eq!(skills.fields()[3].line(), 0);
    // Determinism is not source-order proof; the authored source oracle must run separately.
    assert_eq!(
        skills
            .source_statuses(order)
            .unwrap()
            .iter()
            .map(|entry| entry.skill())
            .collect::<Vec<_>>(),
        keys
    );
}

#[test]
fn replay_accepts_more_than_300_physical_keys_and_has_no_retained_native_owner() {
    // Numeric ABI stress, not proof that a canonical Player admits these keys.
    let history = (0..12000).chain([65536, u32::MAX]).collect::<Vec<u32>>();
    let first = order(&history, history.len()).unwrap();
    assert_eq!(first.len(), 12002);
    assert_eq!(
        first
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>(),
        history.iter().copied().collect()
    );
    assert_eq!(order(&history, history.len()).unwrap(), first);
    assert_eq!(order(&[u32::MAX, 0, 65536], 3).unwrap().len(), 3);
    assert!(order(&[], 0).unwrap().is_empty());
    assert_eq!(order(&history, history.len()).unwrap(), first);
}

#[test]
fn duplicate_and_capacity_failures_preserve_output_and_reset_written() {
    for (history, capacity) in [
        (vec![7, 7], 2),
        (vec![7, 7], 1),
        (vec![7], 0),
        (vec![7], 2),
        (vec![], 1),
    ] {
        let mut output = vec![99; capacity];
        let mut written = 77;
        let code = unsafe {
            rustycore_forever_skill_status_order(
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
fn null_contract_allows_empty_arrays_and_rejects_missing_nonempty_buffers_or_written() {
    use std::ptr::{null, null_mut};
    let mut written = 77;
    assert_eq!(
        unsafe { rustycore_forever_skill_status_order(null(), 0, null_mut(), 0, &mut written) },
        0
    );
    assert_eq!(written, 0);
    let history = [7];
    let mut output = [99];
    for (input, length, target, capacity) in [
        (null(), 1, output.as_mut_ptr(), 1),
        (history.as_ptr(), 1, null_mut(), 1),
        (null(), 1, null_mut(), 0),
    ] {
        written = 77;
        assert_eq!(
            unsafe {
                rustycore_forever_skill_status_order(input, length, target, capacity, &mut written)
            },
            1
        );
        assert_eq!(written, 0);
        assert_eq!(output, [99]);
    }
    assert_eq!(
        unsafe {
            rustycore_forever_skill_status_order(
                history.as_ptr(),
                1,
                output.as_mut_ptr(),
                1,
                null_mut(),
            )
        },
        1
    );
    assert_eq!(output, [99]);
    assert_eq!(
        unsafe { rustycore_forever_skill_status_order(null(), 0, null_mut(), 0, null_mut()) },
        1
    );
}

#[test]
fn rust_adapter_surfaces_native_duplicate_and_count_rejections() {
    assert!(order(&[7, 7], 2).is_err());
    assert!(order(&[7], 0).is_err());
    assert!(order(&[7], 2).is_err());
    assert!(order(&[], 1).is_err());
    let skills = PlayerSkills::initialize([7, 8]);
    assert!(matches!(
        skills.source_statuses(|_, count| order(&[7, 7], count)),
        Err(SkillStatusOrderError::Source(_))
    ));
}
