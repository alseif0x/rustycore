//! Durable projection for the 3.4.3 MoneyLootedFromCreatures criterion.

/// Criteria used by the five supported individual loot-money achievements.
pub const LOOT_MONEY_CRITERION: u32 = 4093;

/// (achievement id, cumulative copper threshold), ordered by threshold.
pub const LOOT_MONEY_ACHIEVEMENTS: [(u32, u64); 5] = [
    (1176, 1_000_000),
    (1177, 10_000_000),
    (1178, 50_000_000),
    (1180, 100_000_000),
    (1181, 250_000_000),
];

/// Durable state produced when one eligible loot claim advances criterion 4093.
/// Newly completed achievement dates all equal `date`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootMoneyCriteriaProgress {
    pub counter: u64,
    /// Timestamp of the durable criterion progress row. Saturated progress
    /// keeps its prior date even if this update completes a missing award.
    pub date: u64,
    pub new_achievements: Vec<u32>,
    /// Shared timestamp for `new_achievements`; normally equal to `date`.
    /// Present only when at least one achievement was newly completed.
    pub achievement_date: Option<u64>,
}
