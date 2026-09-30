//! Deterministic stages of C++ `Player::CalculateReputationGain`.
//!
//! Source: target C++ `a5f8da2e`, `Player.cpp:6321-6397`. Callers retain
//! source-specific aura, configuration, faction-rate and recruit resolution.
//! The lazy scalar inputs preserve the existing query order and early returns;
//! this module owns no Player state, catalog, registry or publication.

/// Percentage before the faction reward rate, shared with the quest reward
/// pipeline. A missing faction modifier skips its addition; a missing low-level
/// multiplier skips its multiplication, matching the original conditional stages.
pub fn reputation_gain_percent_before_reward_rate(
    reputation: i32,
    generic_aura_modifier: f32,
    faction_aura_modifier: Option<f32>,
    resolve_low_level_multiplier: impl FnOnce() -> Option<f32>,
) -> Option<f32> {
    let mut percent = 100.0f32;
    let mut reputation_modifier = generic_aura_modifier;
    if let Some(modifier) = faction_aura_modifier {
        reputation_modifier += modifier;
    }
    percent += if reputation > 0 {
        reputation_modifier
    } else {
        -reputation_modifier
    };

    if let Some(multiplier) = resolve_low_level_multiplier() {
        percent *= multiplier;
    }

    // Preserve the represented Rust guard, including rejection of NaN. The
    // target C++ spells this as `percent <= 0.0f`; this extraction does not
    // change that inherited difference.
    (percent > 0.0).then_some(percent)
}

/// Apply a caller-resolved recruit bonus to the still-floating percentage.
/// `None` skips the multiplication, including for spell-source rewards.
pub fn apply_recruit_a_friend_reputation_bonus(percent: f32, bonus: Option<f32>) -> f32 {
    let mut percent = percent;
    if let Some(bonus) = bonus {
        percent *= 1.0 + bonus;
    }
    percent
}

/// Complete gain calculation using the shared initial percentage stage.
///
/// The initial resolver returns that stage's result without repeating aura
/// queries. Faction-rate resolution occurs only after it succeeds. Recruit
/// resolution occurs only after an absent or enabled faction rate. No integer
/// truncation occurs until the final multiply/divide/cast expression.
pub fn calculate_reputation_gain(
    reputation: i32,
    resolve_initial_percent: impl FnOnce() -> Option<f32>,
    resolve_reward_rate: impl FnOnce() -> Option<f32>,
    resolve_recruit_bonus: impl FnOnce() -> Option<f32>,
) -> i32 {
    let Some(mut percent) = resolve_initial_percent() else {
        return 0;
    };

    if let Some(rate) = resolve_reward_rate() {
        // NaN deliberately does not satisfy this guard, just as in the
        // represented caller before extraction.
        if rate <= 0.0 {
            return 0;
        }
        percent *= rate;
    }

    percent = apply_recruit_a_friend_reputation_bonus(percent, resolve_recruit_bonus());

    (reputation as f32 * percent / 100.0) as i32
}
