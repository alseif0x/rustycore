use super::*;
use std::cell::RefCell;

fn rules() -> QuestReputationSlotRules {
    QuestReputationSlotRules {
        reward_value: 4,
        reward_override: 1200,
        rank_cap: 0,
        spillover_mask: 0,
        slot: 0,
    }
}

#[test]
fn source_priority_is_daily_weekly_monthly_repeatable_then_quest() {
    for bits in 0u8..16 {
        let daily = bits & 1 != 0;
        let weekly = bits & 2 != 0;
        let monthly = bits & 4 != 0;
        let repeatable = bits & 8 != 0;
        let expected = match bits {
            1 | 3 | 5 | 7 | 9 | 11 | 13 | 15 => QuestReputationSource::DailyQuest,
            2 | 6 | 10 | 14 => QuestReputationSource::WeeklyQuest,
            4 | 12 => QuestReputationSource::MonthlyQuest,
            8 => QuestReputationSource::RepeatableQuest,
            0 => QuestReputationSource::Quest,
            _ => unreachable!(),
        };
        assert_eq!(QuestReputationSource::from_flags(daily, weekly, monthly, repeatable), expected);
    }
}

#[test]
fn override_skips_row_lookup_and_divides_toward_zero_before_gain() {
    for (reward_override, expected) in [(1250, 12), (-1250, -12), (100, 1), (-100, -1)] {
        let reward = calculate_quest_reputation_reward(
            QuestReputationSlotRules { reward_override, reward_value: i32::MIN, ..rules() },
            |_| panic!("an override must not read the reward table"),
            |base, no_bonus| {
                assert_eq!(base, expected);
                assert!(no_bonus);
                Some((100.0, 2.5))
            },
            || None,
            |percent| percent,
            || panic!("no cap must not read rank"),
        ).unwrap();
        assert_eq!(reward.base_reputation, expected);
        assert_eq!(reward.gain, expected);
        assert_eq!(reward.gain_rate, 2.5);
        assert!(!reward.reward_table_unavailable);
    }
    for reward_override in [99, -99] {
        assert_eq!(calculate_quest_reputation_reward(
            QuestReputationSlotRules { reward_override, ..rules() },
            |_| panic!("small overrides still bypass DB2"),
            |_, _| panic!("zero base must stop before Player reads"),
            || panic!("zero base must stop before rates"),
            |_| panic!("zero base must stop before RAF"),
            || panic!("zero base must stop before rank"),
        ), None);
    }
}

#[test]
fn signed_reward_value_selects_row_but_row_controls_reputation_sign() {
    let positive = [0, 0, 0, 0, 250, 0, 0, 0, 0, 0];
    let negative = [0, 0, 0, 0, -350, 0, 0, 0, 0, 0];
    for (reward_value, row_id, expected) in [(4, 1, 250), (-4, 2, -350)] {
        let reward = calculate_quest_reputation_reward(
            QuestReputationSlotRules { reward_value, reward_override: 0, ..rules() },
            |row| {
                assert_eq!(row, row_id);
                Some(Some(if row == 1 { &positive } else { &negative }))
            },
            |base, no_bonus| {
                assert_eq!(base, expected);
                assert!(!no_bonus);
                Some((100.0, 1.0))
            },
            || Some(None),
            |percent| percent,
            || panic!("no cap"),
        ).unwrap();
        assert_eq!(reward.gain, expected);
        assert!(!reward.reward_table_unavailable);
        assert!(!reward.reward_rate_unavailable);
    }
}

#[test]
fn missing_row_zero_field_and_invalid_index_stop_before_initial_resolution() {
    let row = [0; 10];
    for reward_value in [0, 4, 10, -10, i32::MIN] {
        for present_row in [None, Some(&row)] {
            assert_eq!(calculate_quest_reputation_reward(
                QuestReputationSlotRules { reward_value, reward_override: 0, ..rules() },
                |row_id| {
                    assert_eq!(row_id, if reward_value < 0 { 2 } else { 1 });
                    Some(present_row)
                },
                |_, _| panic!("resolved zero must skip initial reads"),
                || panic!("resolved zero must skip rate reads"),
                |_| panic!("resolved zero must skip RAF"),
                || panic!("resolved zero must skip rank"),
            ), None);
        }
    }
}

#[test]
fn absent_reward_table_retains_zero_diagnostic_and_late_reads() {
    let calls = RefCell::new(Vec::new());
    let reward = calculate_quest_reputation_reward(
        QuestReputationSlotRules { reward_override: 0, reward_value: i32::MIN, rank_cap: 5, ..rules() },
        |row| { assert_eq!(row, 2); calls.borrow_mut().push("row"); None },
        |base, no_bonus| {
            assert_eq!(base, 0);
            assert!(!no_bonus);
            calls.borrow_mut().push("initial");
            Some((100.0, 1.5))
        },
        || { calls.borrow_mut().push("rate"); None },
        |percent| { calls.borrow_mut().push("recruit"); percent },
        || panic!("zero final gain must skip rank"),
    ).unwrap();
    assert_eq!(*calls.borrow(), vec!["row", "initial", "rate", "recruit"]);
    assert_eq!(reward.base_reputation, 0);
    assert_eq!(reward.after_low_level, 0);
    assert_eq!(reward.after_reward_rate, 0);
    assert_eq!(reward.gain, 0);
    assert!(reward.reward_table_unavailable);
    assert!(reward.reward_rate_unavailable);
    assert!(!reward.rank_cap_unresolved);
}

#[test]
fn complete_slot_preserves_lazy_order_and_all_intermediate_values() {
    let row = [0, 0, 0, 0, 101, 0, 0, 0, 0, 0];
    let calls = RefCell::new(Vec::new());
    let reward = calculate_quest_reputation_reward(
        QuestReputationSlotRules {
            reward_override: 0, rank_cap: 5, spillover_mask: 1 << 2, slot: 2, ..rules()
        },
        |key| { assert_eq!(key, 1); calls.borrow_mut().push("row"); Some(Some(&row)) },
        |base, no_bonus| {
            assert_eq!(base, 101);
            assert!(!no_bonus);
            calls.borrow_mut().push("initial");
            Some((125.0, 2.0))
        },
        || { calls.borrow_mut().push("rate"); Some(Some(1.5)) },
        |percent| {
            assert_eq!(percent, 187.5);
            calls.borrow_mut().push("recruit");
            percent * 1.1
        },
        || { calls.borrow_mut().push("rank"); Some(4) },
    ).unwrap();
    assert_eq!(*calls.borrow(), vec!["row", "initial", "rate", "recruit", "rank"]);
    assert_eq!(reward, QuestReputationReward {
        base_reputation: 101,
        after_low_level: 126,
        after_reward_rate: 189,
        gain: 208,
        gain_rate: 2.0,
        no_quest_bonus: false,
        no_spillover: true,
        reward_table_unavailable: false,
        reward_rate_unavailable: false,
        rank_cap_unresolved: false,
    });
}

#[test]
fn missing_initial_or_low_level_truncated_zero_skip_rate_and_recruit() {
    for initial in [None, Some((0.1, 1.0))] {
        assert_eq!(calculate_quest_reputation_reward(
            QuestReputationSlotRules { reward_override: 100, ..rules() },
            |_| panic!("override"),
            |_, _| initial,
            || panic!("initial failure or intermediate zero skips rate"),
            |_| panic!("initial failure or intermediate zero skips RAF"),
            || panic!("initial failure or intermediate zero skips rank"),
        ), None);
    }
}

#[test]
fn nonpositive_reward_rate_rejects_before_recruit_even_with_missing_table() {
    for rate in [0.0, -0.0, -0.5] {
        for reward_override in [0, 1200] {
            assert_eq!(calculate_quest_reputation_reward(
                QuestReputationSlotRules { reward_override, ..rules() },
                |_| None,
                |_, _| Some((100.0, 1.0)),
                || Some(Some(rate)),
                |_| panic!("disabled rate must skip RAF"),
                || panic!("disabled rate must skip rank"),
            ), None);
        }
    }
}

#[test]
fn absent_rate_store_and_missing_rate_row_have_distinct_diagnostics() {
    for (rate, unavailable) in [(None, true), (Some(None), false)] {
        let reward = calculate_quest_reputation_reward(
            rules(), |_| panic!("override"), |_, _| Some((50.0, 1.0)),
            || rate, |percent| { assert_eq!(percent, 50.0); percent }, || panic!("no cap"),
        ).unwrap();
        assert_eq!(reward.after_low_level, 6);
        assert_eq!(reward.after_reward_rate, 6);
        assert_eq!(reward.gain, 6);
        assert_eq!(reward.reward_rate_unavailable, unavailable);
    }
}

#[test]
fn later_percentages_recalculate_from_base_without_chaining_integer_casts() {
    let reward = calculate_quest_reputation_reward(
        QuestReputationSlotRules { reward_override: 300, ..rules() },
        |_| panic!("override"), |_, _| Some((50.0, 1.0)),
        || Some(Some(1.5)), |percent| percent * 1.5, || panic!("no cap"),
    ).unwrap();
    assert_eq!(reward.base_reputation, 3);
    assert_eq!(reward.after_low_level, 1);
    assert_eq!(reward.after_reward_rate, 2);
    assert_eq!(reward.gain, 3);

    let rescued = calculate_quest_reputation_reward(
        QuestReputationSlotRules { reward_override: 100, rank_cap: 5, ..rules() },
        |_| panic!("override"), |_, _| Some((100.0, 1.0)),
        || Some(Some(0.5)), |percent| percent * 3.0, || Some(4),
    ).unwrap();
    assert_eq!(rescued.after_reward_rate, 0);
    assert_eq!(rescued.gain, 1);
}

#[test]
fn nan_rate_reaches_recruit_and_preserves_resolved_vs_missing_table_zero_gate() {
    for reward_override in [0, 1200] {
        let calls = RefCell::new(Vec::new());
        let reward = calculate_quest_reputation_reward(
            QuestReputationSlotRules { reward_override, rank_cap: 5, ..rules() },
            |_| None, |_, _| Some((100.0, 1.0)), || Some(Some(f32::NAN)),
            |percent| { assert!(percent.is_nan()); calls.borrow_mut().push("recruit"); percent },
            || panic!("NaN casts to zero and never queries rank"),
        );
        assert_eq!(*calls.borrow(), vec!["recruit"]);
        if reward_override == 0 {
            let reward = reward.unwrap();
            assert_eq!(reward.gain, 0);
            assert!(reward.reward_table_unavailable);
            assert!(!reward.reward_rate_unavailable);
        } else {
            assert_eq!(reward, None);
        }
    }
}

#[test]
fn rank_cap_is_read_only_after_positive_final_gain_and_reports_missing_rank() {
    for rank in [Some(4), Some(5), Some(6), None] {
        let reward = calculate_quest_reputation_reward(
            QuestReputationSlotRules { rank_cap: 5, ..rules() },
            |_| panic!("override"), |_, _| Some((100.0, 1.0)),
            || Some(None), |percent| percent, || rank,
        );
        if matches!(rank, Some(5 | 6)) {
            assert_eq!(reward, None);
        } else {
            assert_eq!(reward.unwrap().rank_cap_unresolved, rank.is_none());
        }
    }
    for (reward_override, recruit_multiplier, expected) in [(-1200, 1.0, -12), (1200, -1.0, -12)] {
        let reward = calculate_quest_reputation_reward(
            QuestReputationSlotRules { reward_override, rank_cap: 5, ..rules() },
            |_| panic!("override"), |_, _| Some((100.0, 1.0)),
            || None, |percent| percent * recruit_multiplier,
            || panic!("nonpositive final gain must skip rank even if base was positive"),
        ).unwrap();
        assert_eq!(reward.gain, expected);
        assert!(!reward.rank_cap_unresolved);
    }
    assert_eq!(calculate_quest_reputation_reward(
        QuestReputationSlotRules { rank_cap: 5, ..rules() },
        |_| panic!("override"), |_, _| Some((100.0, 1.0)),
        || None, |_| 0.0, || panic!("final zero skips rank"),
    ), None);
}

#[test]
fn casts_preserve_f32_precision_and_saturation_for_signed_gains() {
    let precise = calculate_quest_reputation_reward(
        QuestReputationSlotRules { reward_override: 1_677_721_700, ..rules() },
        |_| panic!("override"), |_, _| Some((100.0, 1.0)),
        || None, |percent| percent, || panic!("no cap"),
    ).unwrap();
    assert_eq!(precise.base_reputation, 16_777_217);
    assert_eq!(precise.after_low_level, 16_777_216);
    assert_eq!(precise.gain, 16_777_216);
    for (reward_override, expected) in [(1200, i32::MAX), (-1200, i32::MIN)] {
        let reward = calculate_quest_reputation_reward(
            QuestReputationSlotRules { reward_override, ..rules() },
            |_| panic!("override"), |_, _| Some((f32::INFINITY, 1.0)),
            || None, |percent| percent, || panic!("no cap"),
        ).unwrap();
        assert_eq!(reward.after_low_level, expected);
        assert_eq!(reward.after_reward_rate, expected);
        assert_eq!(reward.gain, expected);
    }
}
