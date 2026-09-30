use super::*;
use wow_constants::CurrencyTypesFlagsB;

fn currency_entry() -> CurrencyTypesEntry {
    CurrencyTypesEntry {
        id: 394,
        category_id: 0,
        inventory_icon_file_id: 0,
        spell_weight: 0,
        spell_category: 0,
        max_qty: 0,
        max_earnable_per_week: 0,
        quality: 0,
        faction_id: 0,
        award_condition_id: 0,
        flags: CurrencyTypesFlags::empty(),
        flags_b: CurrencyTypesFlagsB::empty(),
    }
}

fn currency() -> PlayerCurrency {
    PlayerCurrency {
        state: PlayerCurrencyState::Unchanged,
        quantity: 10,
        weekly_quantity: 4,
        tracked_quantity: 6,
        increased_cap_quantity: 3,
        earned_quantity: 8,
        flags: 9,
    }
}

#[test]
fn quest_gain_applies_weekly_then_quantity_caps_to_all_enabled_counters() {
    let entry = CurrencyTypesEntry {
        max_qty: 15,
        max_earnable_per_week: 12,
        flags: CurrencyTypesFlags::TRACK_QUANTITY,
        flags_b: CurrencyTypesFlagsB::USE_TOTAL_EARNED_FOR_EARNED,
        ..currency_entry()
    };
    let mut currency = currency();
    let delta = currency
        .apply_quest_gain(394, 20, &entry, CurrencyGainSourceLikeCpp::QuestReward)
        .unwrap();
    assert_eq!(currency, PlayerCurrency {
        state: PlayerCurrencyState::Changed,
        quantity: 15,
        weekly_quantity: 9,
        tracked_quantity: 11,
        increased_cap_quantity: 3,
        earned_quantity: 13,
        flags: 9,
    });
    assert_eq!(delta, CurrencyGainDelta {
        currency_id: 394,
        quantity: 15,
        amount: 5,
        weekly_quantity: Some(9),
        max_quantity: Some(15),
        total_earned: Some(13),
        suppress_chat_log: false,
    });

    let mut weekly_limited = PlayerCurrency { quantity: 0, weekly_quantity: 11, ..currency };
    let delta = weekly_limited
        .apply_quest_gain(394, 20, &entry, CurrencyGainSourceLikeCpp::WeeklyQuestReward)
        .unwrap();
    assert_eq!(delta.amount, 1);
    assert_eq!(weekly_limited.quantity, 1);
    assert_eq!(weekly_limited.weekly_quantity, 12);
}

#[test]
fn quest_gain_zero_or_exhausted_caps_leave_every_field_unchanged() {
    let base = currency_entry();
    for (entry, amount) in [
        (base, 0),
        (CurrencyTypesEntry { max_earnable_per_week: 4, ..base }, 7),
        (CurrencyTypesEntry { max_earnable_per_week: 3, ..base }, 7),
        (CurrencyTypesEntry { max_qty: 10, ..base }, 7),
        (CurrencyTypesEntry { max_qty: 9, ..base }, 7),
    ] {
        for state in [PlayerCurrencyState::New, PlayerCurrencyState::Removed] {
            let before = PlayerCurrency { state, ..currency() };
            let mut current = before;
            assert_eq!(current.apply_quest_gain(
                394, amount, &entry, CurrencyGainSourceLikeCpp::QuestReward,
            ), None);
            assert_eq!(current, before);
        }
    }
}

#[test]
fn quest_gain_preserves_new_and_marks_all_other_states_changed() {
    for state in [
        PlayerCurrencyState::New,
        PlayerCurrencyState::Unchanged,
        PlayerCurrencyState::Changed,
        PlayerCurrencyState::Removed,
    ] {
        let mut currency = PlayerCurrency { state, ..currency() };
        let delta = currency
            .apply_quest_gain(394, 2, &currency_entry(), CurrencyGainSourceLikeCpp::DailyQuestReward)
            .unwrap();
        assert_eq!(currency.state, if state == PlayerCurrencyState::New {
            PlayerCurrencyState::New
        } else {
            PlayerCurrencyState::Changed
        });
        assert_eq!(currency.quantity, 12);
        assert_eq!(currency.flags, 9);
        assert_eq!(currency.increased_cap_quantity, 3);
        assert_eq!(delta.amount, 2);
    }
}

#[test]
fn quest_gain_uses_dynamic_maximum_and_saturating_cap_increase() {
    let entry = CurrencyTypesEntry {
        max_qty: 10,
        flags: CurrencyTypesFlags::DYNAMIC_MAXIMUM
            | CurrencyTypesFlags::IGNORE_MAX_QTY_ON_LOAD
            | CurrencyTypesFlags::UPDATE_VERSION_IGNORE_MAX,
        ..currency_entry()
    };
    let mut current = currency();
    assert_eq!(current.max_quantity(&entry), 13);
    let delta = current
        .apply_quest_gain(394, 10, &entry, CurrencyGainSourceLikeCpp::QuestReward)
        .unwrap();
    assert_eq!(delta.amount, 3);
    assert_eq!(delta.max_quantity, Some(13));

    let saturated = CurrencyTypesEntry { max_qty: u32::MAX - 1, ..entry };
    assert_eq!(current.max_quantity(&saturated), u32::MAX);
    let dynamic_only = CurrencyTypesEntry { max_qty: 0, ..entry };
    assert_eq!(current.max_quantity(&dynamic_only), 3);
    assert_eq!(current.max_quantity(&currency_entry()), 0);
}

#[test]
fn quest_gain_scaler_controls_weekly_presence_but_keeps_raw_quantity() {
    let entry = CurrencyTypesEntry {
        max_earnable_per_week: 1000,
        flags: CurrencyTypesFlags::SCALER_100 | CurrencyTypesFlags::SUPPRESS_CHAT_MESSAGES,
        ..currency_entry()
    };
    let mut current = PlayerCurrency { weekly_quantity: 98, ..currency() };
    let first = current
        .apply_quest_gain(394, 1, &entry, CurrencyGainSourceLikeCpp::QuestReward)
        .unwrap();
    assert_eq!(first.weekly_quantity, None);
    assert!(first.suppress_chat_log);
    let second = current
        .apply_quest_gain(394, 1, &entry, CurrencyGainSourceLikeCpp::QuestReward)
        .unwrap();
    assert_eq!(second.weekly_quantity, Some(100));
    assert_eq!(second.max_quantity, None);
    assert_eq!(second.total_earned, None);

    let version_only = CurrencyTypesEntry {
        flags: CurrencyTypesFlags::SUPPRESS_CHAT_MESSAGE_ON_VERSION_CHANGE,
        ..entry
    };
    let delta = current
        .apply_quest_gain(394, 1, &version_only, CurrencyGainSourceLikeCpp::QuestReward)
        .unwrap();
    assert!(!delta.suppress_chat_log);
    assert_eq!(delta.weekly_quantity, Some(101));
}

#[test]
fn quest_gain_ignore_caps_skips_weekly_tracking_and_earned_mutations() {
    let entry = CurrencyTypesEntry {
        max_qty: 10,
        max_earnable_per_week: 4,
        flags: CurrencyTypesFlags::TRACK_QUANTITY | CurrencyTypesFlags::DYNAMIC_MAXIMUM,
        flags_b: CurrencyTypesFlagsB::USE_TOTAL_EARNED_FOR_EARNED,
        ..currency_entry()
    };
    for source in [
        CurrencyGainSourceLikeCpp::QuestRewardIgnoreCaps,
        CurrencyGainSourceLikeCpp::WorldQuestRewardIgnoreCaps,
    ] {
        let before = currency();
        let mut current = before;
        let delta = current.apply_quest_gain(394, 20, &entry, source).unwrap();
        assert_eq!(current, PlayerCurrency {
            state: PlayerCurrencyState::Changed,
            quantity: 30,
            ..before
        });
        assert_eq!(delta.amount, 20);
        assert_eq!(delta.weekly_quantity, Some(4));
        assert_eq!(delta.max_quantity, Some(13));
        assert_eq!(delta.total_earned, Some(8));
    }
    for source in [
        CurrencyGainSourceLikeCpp::QuestReward,
        CurrencyGainSourceLikeCpp::WorldQuestReward,
        CurrencyGainSourceLikeCpp::DailyQuestReward,
        CurrencyGainSourceLikeCpp::WeeklyQuestReward,
    ] {
        let before = currency();
        let mut current = before;
        assert_eq!(current.apply_quest_gain(394, 20, &entry, source), None);
        assert_eq!(current, before);
    }
}

#[test]
fn quest_gain_saturates_counters_without_rewriting_reported_applied_amount() {
    let entry = CurrencyTypesEntry {
        max_qty: u32::MAX,
        max_earnable_per_week: u32::MAX,
        flags: CurrencyTypesFlags::TRACK_QUANTITY,
        flags_b: CurrencyTypesFlagsB::USE_TOTAL_EARNED_FOR_EARNED,
        ..currency_entry()
    };
    let mut current = PlayerCurrency {
        quantity: u32::MAX - 1,
        weekly_quantity: u32::MAX - 2,
        tracked_quantity: u32::MAX - 3,
        earned_quantity: u32::MAX - 4,
        ..currency()
    };
    let delta = current
        .apply_quest_gain(394, 10, &entry, CurrencyGainSourceLikeCpp::QuestReward)
        .unwrap();
    assert_eq!(current.quantity, u32::MAX);
    assert_eq!(current.weekly_quantity, u32::MAX);
    assert_eq!(current.tracked_quantity, u32::MAX);
    assert_eq!(current.earned_quantity, u32::MAX);
    assert_eq!(delta.amount, 10);
    assert_eq!(delta.quantity, u32::MAX);
    assert_eq!(delta.weekly_quantity, Some(u32::MAX));
    assert_eq!(delta.total_earned, Some(u32::MAX));
}

#[test]
fn quest_gain_without_counter_flags_preserves_existing_auxiliary_values() {
    let before = currency();
    let mut current = before;
    let delta = current
        .apply_quest_gain(394, 5, &currency_entry(), CurrencyGainSourceLikeCpp::QuestReward)
        .unwrap();
    assert_eq!(current.weekly_quantity, before.weekly_quantity);
    assert_eq!(current.tracked_quantity, before.tracked_quantity);
    assert_eq!(current.earned_quantity, before.earned_quantity);
    assert_eq!(delta.weekly_quantity, Some(4));
    assert_eq!(delta.total_earned, None);
}
