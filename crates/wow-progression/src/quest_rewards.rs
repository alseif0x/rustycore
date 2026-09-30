//! Quest XP and money valuation, without catalogs or mutable Player state.
//!
//! TrinityCore a5f8da2ebf5424bf0450ca4e08843ecbf72577bd:
//! Player.h:1491 GetQuestLevel; Player.cpp:14534 GetQuestMoneyReward and
//! :14539 GetQuestXPReward; QuestDef.cpp:387 XPValue, :422 MoneyValue,
//! :714 RoundXPValue. Preserve the existing Rust fallback, difficulty-before-
//! lookup gate, zero-base early return, money rounding, and missing rates/auras.

use wow_data_model::quest::QuestRewardRules;

#[cfg(test)]
mod tests;

/// The application has already sampled canonical rewarded membership.
pub fn quest_xp_is_blocked(already_rewarded: bool, is_dungeon_finder: bool) -> bool {
    already_rewarded && !is_dungeon_finder
}

/// Sample Player level only when the quest does not supply a positive level.
pub fn effective_quest_level(
    rules: &QuestRewardRules,
    player_level: impl FnOnce() -> u8,
) -> i32 {
    if rules.quest_level() > 0 {
        rules.quest_level()
    } else {
        i32::from(player_level()).min(rules.max_scaling_level())
    }
}

/// Resolve just the selected row, after the original difficulty gate.
pub fn calculate_quest_xp<'a>(
    quest_level: i32,
    player_level: u8,
    xp_difficulty: u32,
    xp_multiplier: f32,
    min_quest_scaled_xp_ratio: u32,
    resolve_row: impl FnOnce(u32) -> Option<&'a [u32; 10]>,
) -> u32 {
    if xp_difficulty >= 10 {
        return 0;
    }

    let ql = if quest_level == -1 {
        player_level as i32
    } else {
        quest_level
    };
    let Some(row) = resolve_row(ql as u32) else {
        return 0;
    };

    let base_xp = row[xp_difficulty as usize];
    if base_xp == 0 {
        return 0;
    }

    let diff_factor = (2 * (ql - player_level as i32) + 20).clamp(1, 10) as u32;
    let xp = round_xp(diff_factor * base_xp / 10);
    if min_quest_scaled_xp_ratio != 0 {
        xp.max(
            round_xp((base_xp as f32 * xp_multiplier) as u32) * min_quest_scaled_xp_ratio / 100,
        )
    } else {
        xp
    }
}

/// Gathering rewards use the raw Player-level row, without quest reduction.
pub fn player_level_difficulty_xp<'a>(
    player_level: u8,
    xp_difficulty: u32,
    resolve_row: impl FnOnce(u32) -> Option<&'a [u32; 10]>,
) -> u32 {
    if xp_difficulty >= 10 {
        return 0;
    }

    resolve_row(player_level as u32)
        .map(|row| round_xp(row[xp_difficulty as usize]))
        .unwrap_or(0)
}

/// Retained Rust behavior when the application has no QuestXP store.
pub fn fallback_quest_xp(difficulty: u32) -> u32 {
    const XP_TABLE: [u32; 10] = [0, 50, 100, 200, 400, 650, 1000, 1500, 2500, 4000];
    XP_TABLE[difficulty.min(9) as usize]
}

/// Called only after the application has resolved the money row.
pub fn quest_money_value(row: &[u32; 10], difficulty: u32, multiplier: f32) -> u32 {
    let Some(base) = row.get(difficulty as usize).copied() else {
        return 0;
    };
    ((base as f32) * multiplier).round() as u32
}

fn round_xp(xp: u32) -> u32 {
    if xp <= 100 {
        5 * ((xp + 2) / 5)
    } else if xp <= 500 {
        10 * ((xp + 5) / 10)
    } else if xp <= 1000 {
        25 * ((xp + 12) / 25)
    } else {
        50 * ((xp + 25) / 50)
    }
}
