use super::*;
use std::collections::HashMap;

/// Immutable table fixtures only; production catalogs remain in wow-data.
struct XpRows {
    rows: HashMap<u32, [u32; 10]>,
}

impl XpRows {
    fn calculate_xp(
        &self,
        quest_level: i32,
        player_level: u8,
        difficulty: u32,
        multiplier: f32,
        min_ratio: u32,
    ) -> u32 {
        calculate_quest_xp(quest_level, player_level, difficulty, multiplier, min_ratio,
            |level| self.rows.get(&level))
    }

    fn player_level_difficulty_xp(&self, player_level: u8, difficulty: u32) -> u32 {
        player_level_difficulty_xp(player_level, difficulty, |level| self.rows.get(&level))
    }
}

#[test]
fn round_xp_value_matches_cpp_thresholds() {
    assert_eq!(round_xp(1), 0);
    assert_eq!(round_xp(3), 5);
    assert_eq!(round_xp(102), 100);
    assert_eq!(round_xp(106), 110);
    assert_eq!(round_xp(511), 500);
    assert_eq!(round_xp(513), 525);
    assert_eq!(round_xp(1024), 1000);
    assert_eq!(round_xp(1026), 1050);
}

#[test]
fn player_level_difficulty_xp_uses_raw_player_level_row_like_cpp() {
    let mut rows = HashMap::new();
    rows.insert(42, [0, 1, 11, 101, 511, 1026, 0, 0, 0, 0]);
    let store = XpRows { rows };

    assert_eq!(store.player_level_difficulty_xp(42, 1), 0);
    assert_eq!(store.player_level_difficulty_xp(42, 2), 10);
    assert_eq!(store.player_level_difficulty_xp(42, 3), 100);
    assert_eq!(store.player_level_difficulty_xp(42, 4), 500);
    assert_eq!(store.player_level_difficulty_xp(42, 5), 1050);
    assert_eq!(store.player_level_difficulty_xp(41, 5), 0);
    assert_eq!(store.player_level_difficulty_xp(42, 10), 0);
}

#[test]
fn calculate_xp_missing_quest_level_returns_zero_like_cpp() {
    let mut rows = HashMap::new();
    rows.insert(42, [0, 100, 0, 0, 0, 0, 0, 0, 0, 0]);
    let store = XpRows { rows };

    assert_eq!(store.calculate_xp(41, 42, 1, 1.0, 0), 0);
}

#[test]
fn calculate_xp_min_scaled_ratio_raises_grey_quest_like_cpp() {
    let mut rows = HashMap::new();
    rows.insert(42, [0, 1000, 0, 0, 0, 0, 0, 0, 0, 0]);
    let store = XpRows { rows };

    assert_eq!(store.calculate_xp(42, 80, 1, 2.0, 0), 100);
    assert_eq!(store.calculate_xp(42, 80, 1, 2.0, 50), 1000);
}

#[test]
fn quest_xp_blocks_rewarded_non_df_and_allows_rewarded_df() {
    assert!(quest_xp_is_blocked(true, false));
    assert!(!quest_xp_is_blocked(true, true));
    assert!(!quest_xp_is_blocked(false, false));
    assert!(!quest_xp_is_blocked(false, true));
}

#[test]
fn effective_level_samples_player_only_for_nonpositive_quest_levels() {
    let fixed = QuestRewardRules::new(42, 0, false, 1, 1.0, 1, 1.0);
    assert_eq!(effective_quest_level(&fixed, || panic!("positive level must not sample Player")), 42);

    let scaled = QuestRewardRules::new(-1, 70, false, 1, 1.0, 1, 1.0);
    let mut calls = 0;
    assert_eq!(effective_quest_level(&scaled, || { calls += 1; 80 }), 70);
    assert_eq!(calls, 1);
    let zero_cap = QuestRewardRules::new(0, 0, false, 1, 1.0, 1, 1.0);
    assert_eq!(effective_quest_level(&zero_cap, || 80), 0);
}

#[test]
fn invalid_xp_difficulty_skips_lookup_after_player_argument_is_sampled() {
    let mut player_samples = 0;
    let xp = calculate_quest_xp(42, { player_samples += 1; 80 }, 10, 1.0, 0,
        |_| panic!("invalid difficulty must not look up a row"));
    assert_eq!(xp, 0);
    assert_eq!(player_samples, 1);
    assert_eq!(player_level_difficulty_xp(42, u32::MAX,
        |_| panic!("invalid raw difficulty must not look up a row")), 0);
}

#[test]
fn fallback_retains_fixed_table_and_clamps_invalid_difficulty() {
    for (difficulty, expected) in [0, 50, 100, 200, 400, 650, 1000, 1500, 2500, 4000]
        .into_iter().enumerate()
    {
        assert_eq!(fallback_quest_xp(difficulty as u32), expected);
    }
    assert_eq!(fallback_quest_xp(10), 4000);
    assert_eq!(fallback_quest_xp(u32::MAX), 4000);
}

#[test]
fn xp_minus_one_uses_player_row_and_negative_levels_keep_unsigned_lookup_key() {
    let row = [0, 513, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut keys = Vec::new();
    assert_eq!(calculate_quest_xp(-1, 42, 1, 1.0, 0, |key| {
        keys.push(key);
        Some(&row)
    }), 525);
    assert_eq!(keys, vec![42]);
    assert_eq!(calculate_quest_xp(-2, 42, 1, 1.0, 0, |key| {
        keys.push(key);
        None
    }), 0);
    assert_eq!(keys, vec![42, (-2_i32) as u32]);
}

#[test]
fn xp_zero_base_and_nan_multiplier_preserve_early_return_and_cast_order() {
    let empty = [0; 10];
    assert_eq!(calculate_quest_xp(42, 80, 1, f32::NAN, 50, |_| Some(&empty)), 0);
    let row = [0, 1000, 0, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(calculate_quest_xp(42, 80, 1, f32::NAN, 50, |_| Some(&row)), 100);
    assert_eq!(calculate_quest_xp(42, 80, 1, -2.0, 50, |_| Some(&row)), 100);
    assert_eq!(calculate_quest_xp(42, 80, 1, 20.0, 0, |_| Some(&row)), 100);
}

#[test]
fn xp_reduction_rounds_integer_quotient_before_scaled_minimum() {
    let row = [0, 106, 0, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(calculate_quest_xp(42, 80, 1, 1.0, 0, |_| Some(&row)), 10);
    assert_eq!(calculate_quest_xp(42, 80, 1, 1.0, 50, |_| Some(&row)), 55);
}

#[test]
fn money_keeps_rounding_nan_cast_and_zero_invalid_difficulty_behavior() {
    let row = [0, 3, u32::MAX, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(quest_money_value(&row, 1, 1.5), 5);
    assert_eq!(quest_money_value(&row, 1, f32::NAN), 0);
    assert_eq!(quest_money_value(&row, 1, -1.0), 0);
    assert_eq!(quest_money_value(&row, 2, 2.0), u32::MAX);
    assert_eq!(quest_money_value(&row, 0, 4.0), 0);
    assert_eq!(quest_money_value(&row, 10, 1.0), 0);
    assert_eq!(quest_money_value(&row, u32::MAX, 1.0), 0);
}
