//! Quest reputation valuation by reward slot.
//!
//! Reference: a5f8da2e `Player.cpp`, RewardReputation(Quest) (6450) and
//! CalculateReputationGain (6321). The represented Rust path retains its
//! intermediate truncation gates, missing-store diagnostics and final-gain
//! rank-cap check, which occurs later than the target C++ base-gain check.
//! Catalog, Player, aura and recruit reads remain lazy application inputs.

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestReputationSource {
    Quest,
    DailyQuest,
    WeeklyQuest,
    MonthlyQuest,
    RepeatableQuest,
}

impl QuestReputationSource {
    pub fn from_flags(daily: bool, weekly: bool, monthly: bool, repeatable: bool) -> Self {
        if daily {
            Self::DailyQuest
        } else if weekly {
            Self::WeeklyQuest
        } else if monthly {
            Self::MonthlyQuest
        } else if repeatable {
            Self::RepeatableQuest
        } else {
            Self::Quest
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestReputationSlotRules {
    pub reward_value: i32,
    pub reward_override: i32,
    pub rank_cap: i32,
    pub spillover_mask: u32,
    pub slot: usize,
}

/// Immutable valuation and the intermediate values used by the application.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuestReputationReward {
    pub base_reputation: i32,
    pub after_low_level: i32,
    pub after_reward_rate: i32,
    pub gain: i32,
    pub gain_rate: f32,
    pub no_quest_bonus: bool,
    pub no_spillover: bool,
    pub reward_table_unavailable: bool,
    pub reward_rate_unavailable: bool,
    pub rank_cap_unresolved: bool,
}

/// Outer `None` from row/rate resolution means the store is absent; inner
/// `None` means the store exists without a matching row. The initial resolver
/// returns (percentage, canonical mutation gain rate) from its original reads.
pub fn calculate_quest_reputation_reward<'a>(
    rules: QuestReputationSlotRules,
    resolve_row: impl FnOnce(u32) -> Option<Option<&'a [i16; 10]>>,
    resolve_initial: impl FnOnce(i32, bool) -> Option<(f32, f32)>,
    resolve_rate: impl FnOnce() -> Option<Option<f32>>,
    apply_recruit: impl FnOnce(f32) -> f32,
    resolve_rank: impl FnOnce() -> Option<u8>,
) -> Option<QuestReputationReward> {
    let (base_reputation, no_quest_bonus, reward_table_unavailable) =
        if rules.reward_override != 0 {
            (rules.reward_override / 100, true, false)
        } else {
            let row = if rules.reward_value < 0 { 2 } else { 1 };
            if let Some(row) = resolve_row(row) {
                let field = rules.reward_value.unsigned_abs() as usize;
                let reputation = row
                    .and_then(|difficulty| difficulty.get(field).copied())
                    .map(i32::from)
                    .unwrap_or(0);
                (reputation, false, false)
            } else {
                (0, false, true)
            }
        };

    if base_reputation == 0 && !reward_table_unavailable {
        return None;
    }

    let (percent_before_reward_rate, gain_rate) = resolve_initial(base_reputation, no_quest_bonus)?;
    let after_low_level = calculate_pct(base_reputation, percent_before_reward_rate);
    if after_low_level == 0 && !reward_table_unavailable {
        return None;
    }

    let (after_reward_rate, percent_after_reward_rate, reward_rate_unavailable) =
        if let Some(rate) = resolve_rate() {
            if let Some(rate) = rate {
                if rate <= 0.0 {
                    return None;
                }
                let percent = percent_before_reward_rate * rate;
                (calculate_pct(base_reputation, percent), percent, false)
            } else {
                (after_low_level, percent_before_reward_rate, false)
            }
        } else {
            (after_low_level, percent_before_reward_rate, true)
        };

    let gain = calculate_pct(base_reputation, apply_recruit(percent_after_reward_rate));
    if gain == 0 && !reward_table_unavailable {
        return None;
    }

    let current_rank = if rules.rank_cap != 0 && gain > 0 {
        resolve_rank()
    } else {
        None
    };
    if current_rank.is_some_and(|rank| i32::from(rank) >= rules.rank_cap) {
        return None;
    }

    let no_spillover = (rules.spillover_mask & (1u32 << rules.slot)) != 0;
    Some(QuestReputationReward {
        base_reputation,
        after_low_level,
        after_reward_rate,
        gain,
        gain_rate,
        no_quest_bonus,
        no_spillover,
        reward_table_unavailable,
        reward_rate_unavailable,
        rank_cap_unresolved: rules.rank_cap != 0 && gain > 0 && current_rank.is_none(),
    })
}

fn calculate_pct(base: i32, percent: f32) -> i32 {
    (base as f32 * percent / 100.0) as i32
}
