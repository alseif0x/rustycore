use std::cell::{Cell, RefCell};

use wow_progression::{
    apply_recruit_a_friend_reputation_bonus, calculate_reputation_gain,
    reputation_gain_percent_before_reward_rate,
};

#[test]
fn reputation_percentage_preserves_signed_modifiers_and_optional_terms() {
    let cases = [
        (100, 25.0, Some(10.0), 135.0),
        (-100, 25.0, Some(10.0), 65.0),
        (0, 25.0, Some(10.0), 65.0),
        (100, 25.0, None, 125.0),
        (100, 0.0, Some(10.0), 110.0),
        (100, -25.0, Some(-10.0), 65.0),
        (-100, -25.0, Some(-10.0), 135.0),
    ];
    for (reputation, generic, faction, expected) in cases {
        assert_eq!(
            reputation_gain_percent_before_reward_rate(reputation, generic, faction, || None),
            Some(expected),
        );
    }
}

#[test]
fn low_level_multiplier_precedes_percentage_guard() {
    let calls = Cell::new(0);
    // The initial -50 percentage is multiplied before the positivity guard.
    assert_eq!(
        reputation_gain_percent_before_reward_rate(100, -150.0, None, || {
            calls.set(calls.get() + 1);
            Some(-2.0)
        }),
        Some(100.0),
    );
    assert_eq!(calls.get(), 1);

    assert_eq!(
        reputation_gain_percent_before_reward_rate(100, -150.0, None, || None),
        None,
    );
    assert_eq!(
        reputation_gain_percent_before_reward_rate(100, 25.0, None, || Some(0.0)),
        None,
    );
    assert_eq!(
        reputation_gain_percent_before_reward_rate(100, 25.0, None, || Some(f32::NAN)),
        None,
    );
    assert_eq!(
        reputation_gain_percent_before_reward_rate(100, f32::NAN, None, || None),
        None,
    );
}

#[test]
fn complete_gain_preserves_lazy_query_order() {
    let queries = RefCell::new(Vec::new());
    let gain = calculate_reputation_gain(
        100,
        || {
            queries.borrow_mut().push("initial");
            reputation_gain_percent_before_reward_rate(100, 20.0, Some(5.0), || {
                queries.borrow_mut().push("low_level");
                Some(0.5)
            })
        },
        || {
            queries.borrow_mut().push("reward_rate");
            Some(1.0)
        },
        || {
            queries.borrow_mut().push("recruit");
            Some(0.1)
        },
    );

    assert_eq!(gain, 68);
    assert_eq!(
        *queries.borrow(),
        vec!["initial", "low_level", "reward_rate", "recruit"],
    );
}

#[test]
fn missing_initial_percentage_skips_reward_and_recruit_queries() {
    let initial_calls = Cell::new(0);
    assert_eq!(
        calculate_reputation_gain(
            100,
            || {
                initial_calls.set(initial_calls.get() + 1);
                None
            },
            || panic!("missing aura proof must skip the faction-rate query"),
            || panic!("missing aura proof must skip the recruit query"),
        ),
        0,
    );
    assert_eq!(initial_calls.get(), 1);
}

#[test]
fn disabled_reward_rates_skip_recruit_after_initial_query() {
    for rate in [0.0, -0.0, -0.5] {
        let queries = RefCell::new(Vec::new());
        let gain = calculate_reputation_gain(
            100,
            || {
                queries.borrow_mut().push("initial");
                Some(125.0)
            },
            || {
                queries.borrow_mut().push("reward_rate");
                Some(rate)
            },
            || panic!("disabled faction rate must skip the recruit query"),
        );
        assert_eq!(gain, 0);
        assert_eq!(*queries.borrow(), vec!["initial", "reward_rate"]);
    }
}

#[test]
fn missing_or_nan_reward_rates_retain_recruit_stage() {
    for (rate, expected) in [(None, 110), (Some(f32::NAN), 0)] {
        let recruit_calls = Cell::new(0);
        let gain = calculate_reputation_gain(
            100,
            || Some(100.0),
            || rate,
            || {
                recruit_calls.set(recruit_calls.get() + 1);
                Some(0.1)
            },
        );
        assert_eq!(gain, expected);
        assert_eq!(recruit_calls.get(), 1);
    }
}

#[test]
fn negative_gains_truncate_only_after_all_percentage_stages() {
    for (reputation, expected) in [(101, 83), (-101, -27), (0, 0)] {
        assert_eq!(
            calculate_reputation_gain(
                reputation,
                || reputation_gain_percent_before_reward_rate(reputation, 20.0, Some(30.0), || {
                    Some(0.5)
                },),
                || None,
                || Some(0.1),
            ),
            expected,
        );
    }
}

#[test]
fn final_cast_preserves_f32_precision_and_saturation() {
    for (reputation, percentage, expected) in [
        (16_777_217, 100.0, 16_777_216),
        (i32::MAX, 200.0, i32::MAX),
        (i32::MIN, 200.0, i32::MIN),
        (100, f32::INFINITY, i32::MAX),
        (-100, f32::INFINITY, i32::MIN),
        (0, f32::INFINITY, 0),
    ] {
        assert_eq!(
            calculate_reputation_gain(reputation, || Some(percentage), || None, || None),
            expected,
        );
    }
}

#[test]
fn shared_recruit_stage_retains_float_percentages_for_quest_rewards() {
    let percent = reputation_gain_percent_before_reward_rate(101, 25.0, None, || None)
        .expect("positive quest percentage");
    assert_eq!(percent, 125.0);
    assert_eq!(
        apply_recruit_a_friend_reputation_bonus(percent, None),
        125.0
    );
    assert_eq!(
        apply_recruit_a_friend_reputation_bonus(percent, Some(0.1)),
        137.5,
    );
    assert_eq!(
        apply_recruit_a_friend_reputation_bonus(percent, Some(-1.5)),
        -62.5,
    );
    assert_eq!(
        calculate_reputation_gain(101, || Some(percent), || Some(1.5), || Some(0.1)),
        208,
    );
}

#[test]
fn nan_initial_percentage_stops_before_rate_and_recruit_resolution() {
    assert_eq!(
        calculate_reputation_gain(
            100,
            || reputation_gain_percent_before_reward_rate(100, 25.0, None, || Some(f32::NAN)),
            || panic!("NaN initial percentage must stop before rate resolution"),
            || panic!("NaN initial percentage must stop before recruit resolution"),
        ),
        0,
    );
}
