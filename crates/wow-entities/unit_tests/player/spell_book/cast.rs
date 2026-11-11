use super::*;

#[test]
fn context_is_exact_five_slots_with_equality_before_zero_and_no_initial_id() {
    let mut context = Context::default();
    assert!(!context.add(0));
    assert_eq!(context.visited, [0; 5]);
    for id in [u32::MAX, 1, 2, 3, 4] {
        assert!(context.add(id));
    }
    assert!(!context.add(5));
    assert!(!context.add(1));
    assert!(!context.add(0));
    assert_eq!(context.visited, [u32::MAX, 1, 2, 3, 4]);
}
