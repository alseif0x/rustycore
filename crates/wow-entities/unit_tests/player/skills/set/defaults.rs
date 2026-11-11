use super::*;

#[test]
fn ordinary_capacity_return_does_not_abort_the_remaining_source_default_calls() {
    let mut fixture = Fixture::new(vec![line(10, 6, 0, 0), line(20, 6, 0, 0)], rewards(&[20]));
    fixture.defaults = vec![rc(1, 10, 0), rc(2, 20, 0)];
    fixture
        .default_answers
        .extend([((1, 1), update(10, 0, 1, 5)), ((2, 1), update(20, 0, 1, 5))]);
    let mut skills = allocated([0, 20]);
    let mut effects = Effects::default();
    skills
        .learn_default_skills(fixture.sources(), &mut effects)
        .unwrap();
    assert!(skills.status(10).is_none()); // Slot-zero source return.
    assert_eq!(skills.pure_value(20), 1);
    assert_eq!(effects.grants, [(1020, 20, false)]);
}
