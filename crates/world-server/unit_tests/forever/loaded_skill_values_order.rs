//! Adapter/ABI feedback only; the independent extracted-source oracle is separate.
use super::{order, rustycore_forever_loaded_skill_values_order};
use std::collections::BTreeSet;
use std::ptr::{null, null_mut};

#[test]
fn adapter_preserves_the_first_assignment_key_set_without_retained_owner() {
    let first = [0u32, 1, 65_536, u32::MAX];
    let first_order = order(&first, first.len()).unwrap();
    assert_eq!(first_order.len(), first.len());
    assert_eq!(
        first_order.iter().copied().collect::<BTreeSet<_>>(),
        first.into()
    );

    let second = [7u32, 9, 11, 13];
    let second_order = order(&second, second.len()).unwrap();
    assert_eq!(second_order.len(), second.len());
    assert_eq!(
        second_order.iter().copied().collect::<BTreeSet<_>>(),
        second.into()
    );

    let replayed_first = order(&first, first.len()).unwrap();
    assert_eq!(replayed_first, first_order);
    assert_eq!(replayed_first.len(), first.len());
    assert_eq!(
        replayed_first.iter().copied().collect::<BTreeSet<_>>(),
        first.into()
    );
    assert!(order(&[], 0).unwrap().is_empty());
}

#[test]
fn duplicate_first_assignments_and_capacity_mismatches_are_rejected() {
    assert!(order(&[7, 7], 2).is_err());
    assert!(order(&[7, 7], 1).is_err());
    assert!(order(&[7], 0).is_err());
    assert!(order(&[7], 2).is_err());
    assert!(order(&[], 1).is_err());
}

#[test]
fn invalid_histories_preserve_output_and_reset_written() {
    let duplicate = [7u32, 7];
    let mut output = [99u32, 99];
    let mut written = 77;
    let code = unsafe {
        rustycore_forever_loaded_skill_values_order(
            duplicate.as_ptr(),
            duplicate.len(),
            output.as_mut_ptr(),
            output.len(),
            &mut written,
        )
    };
    assert_eq!(code, 1);
    assert_eq!(written, 0);
    assert_eq!(output, [99, 99]);
}

#[test]
fn null_contract_allows_only_empty_arrays_without_buffers() {
    let mut written = 77;
    assert_eq!(
        unsafe {
            rustycore_forever_loaded_skill_values_order(null(), 0, null_mut(), 0, &mut written)
        },
        0
    );
    assert_eq!(written, 0);

    let input = [1u32];
    let mut output = [99u32];
    written = 77;
    assert_eq!(
        unsafe {
            rustycore_forever_loaded_skill_values_order(
                null(),
                1,
                output.as_mut_ptr(),
                1,
                &mut written,
            )
        },
        1
    );
    assert_eq!(written, 0);
    assert_eq!(output, [99]);

    written = 77;
    assert_eq!(
        unsafe {
            rustycore_forever_loaded_skill_values_order(
                input.as_ptr(),
                1,
                null_mut(),
                1,
                &mut written,
            )
        },
        1
    );
    assert_eq!(written, 0);
    assert_eq!(output, [99]);

    written = 77;
    assert_eq!(
        unsafe {
            rustycore_forever_loaded_skill_values_order(
                input.as_ptr(),
                1,
                output.as_mut_ptr(),
                1,
                null_mut(),
            )
        },
        1
    );
    assert_eq!(written, 77);
    assert_eq!(output, [99]);

    assert_eq!(
        unsafe {
            rustycore_forever_loaded_skill_values_order(null(), 0, null_mut(), 0, null_mut())
        },
        1
    );
}

#[test]
fn native_capacity_failure_does_not_touch_output() {
    let input = [1u32, 2];
    let mut output = [99u32, 99];
    let mut written = 77;
    let code = unsafe {
        rustycore_forever_loaded_skill_values_order(
            input.as_ptr(),
            input.len(),
            output.as_mut_ptr(),
            1,
            &mut written,
        )
    };
    assert_eq!(code, 1);
    assert_eq!(written, 0);
    assert_eq!(output, [99, 99]);
}
