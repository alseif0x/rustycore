use super::*;
use std::cell::Cell;

#[test]
fn level_overrides_keep_the_original_thresholds_and_caps() {
    for (level, blood_fury, burst) in [
        (0, 400, 400), (59, 400, 400), (60, 400, 400),
        (61, 390, 396), (75, 250, 340), (90, 100, 340), (255, 100, 340),
    ] {
        assert_eq!(energize_spell_amount(24_571, 400, true, level, || panic!("not Injector")), blood_fury);
        assert_eq!(energize_spell_amount(24_532, 400, true, level, || panic!("not Injector")), burst);
    }
    assert_eq!(energize_spell_amount(24_571, 400, false, 0, || panic!("not player")), 400);
}

#[test]
fn engineering_lookup_is_lazy_and_called_once_only_for_a_player_injector() {
    let calls = Cell::new(0);
    for (spell_id, player) in [(1, true), (67_490, false), (24_571, true)] {
        energize_spell_amount(spell_id, 100, player, 60, || {
            calls.set(calls.get() + 1);
            Some(1)
        });
    }
    assert_eq!(calls.get(), 0);
    assert_eq!(energize_spell_amount(67_490, 100, true, 60, || {
        calls.set(calls.get() + 1);
        Some(1)
    }), 125);
    assert_eq!(calls.get(), 1);
}

#[test]
fn injector_preserves_missing_zero_nonzero_skill_and_float_truncation() {
    assert_eq!(energize_spell_amount(67_490, 7, true, 80, || None), 7);
    assert_eq!(energize_spell_amount(67_490, 7, true, 80, || Some(0)), 7);
    assert_eq!(energize_spell_amount(67_490, 7, true, 80, || Some(1)), 8);
    assert_eq!(energize_spell_amount(67_490, -7, true, 80, || Some(u16::MAX)), -8);
    assert_eq!(energize_spell_amount(67_490, 0, true, 80, || Some(1)), 0);
}

#[test]
fn ordinary_spell_keeps_its_amount_after_the_callers_level_sample() {
    let events = std::cell::RefCell::new(Vec::new());
    let level = { events.borrow_mut().push("level"); 80 };
    let amount = energize_spell_amount(1, -123, true, level, || {
        events.borrow_mut().push("engineering");
        None
    });
    assert_eq!(amount, -123);
    assert_eq!(*events.borrow(), vec!["level"]);
}
