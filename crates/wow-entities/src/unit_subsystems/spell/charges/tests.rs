use super::*;

fn history(loaded: bool) -> SpellHistory {
    let mut history = SpellHistory::default();
    history.charges_loaded = loaded;
    history.add_charge_state_like_cpp(7, 10, 20);
    history.add_charge_state_like_cpp(7, 20, 30);
    history.add_charge_state_like_cpp(7, 30, 40);
    history
}

#[test]
fn canonical_restore_pops_tail_and_keeps_empty_category_without_loaded_rows() {
    let mut history = history(false);
    assert_eq!(history.restore_charges(7, 2), 2);
    assert_eq!(
        history
            .charges(7)
            .unwrap()
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![SpellChargeState {
            recharge_start_ms: 10,
            recharge_end_ms: 20
        }]
    );
    assert_eq!(history.restore_charges(7, 2), 1);
    assert!(history.charges.contains_key(&7));
    assert!(history.charges(7).unwrap().is_empty());
    assert!(!history.charges_loaded);
}

#[test]
fn loaded_restore_refuses_unloaded_rows_without_consuming_the_queue() {
    let mut history = history(false);
    let before = history.charges.clone();
    assert!(!history.restore_loaded_charge(7));
    assert_eq!(history.charges, before);
    history.charges_loaded = true;
    assert!(history.restore_loaded_charge(7));
    assert_eq!(
        history.charges(7).unwrap().back().unwrap().recharge_end_ms,
        30
    );
}

#[test]
fn loaded_restore_removes_only_after_a_successful_last_pop() {
    let mut history = history(true);
    assert_eq!(history.restore_charges(7, 2), 2);
    assert!(history.restore_loaded_charge(7));
    assert!(!history.charges.contains_key(&7));
    assert!(!history.restore_loaded_charge(7));
    history.charges.insert(7, VecDeque::new());
    assert!(!history.restore_loaded_charge(7));
    assert!(history.charges.contains_key(&7));
}

#[test]
fn two_rounds_can_consume_twice_the_count_but_return_the_larger_round() {
    let mut history = history(true);
    history.add_charge_state_like_cpp(7, 40, 50);
    let canonical = history.restore_charges(7, 2);
    let mut loaded = 0;
    for _ in 0..2 {
        if history.restore_loaded_charge(7) {
            loaded += 1;
        }
    }
    assert_eq!((canonical, loaded), (2, 2));
    assert_eq!(canonical.max(loaded), 2);
    assert!(!history.charges.contains_key(&7));
}

#[test]
fn nonpositive_batch_and_missing_category_leave_all_history_unchanged() {
    let mut history = history(true);
    let before = history.clone();
    assert_eq!(history.restore_charges(7, 0), 0);
    assert_eq!(history.restore_charges(7, -1), 0);
    assert_eq!(history.restore_charges(8, 3), 0);
    assert!(!history.restore_loaded_charge(8));
    assert_eq!(history, before);
}
