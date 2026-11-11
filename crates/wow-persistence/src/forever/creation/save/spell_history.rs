//! SQL-free Player spell-history save projection.

pub struct NewSpellHistory {
    /// Preserve observed unordered cooldown-map iteration order.
    pub cooldowns: Vec<NewSpellCooldown>,
    /// Preserve observed unordered category-map iteration order.
    pub charges: Vec<NewSpellChargeCategory>,
}

/// Times are signed epoch seconds already projected by the future canonical
/// constructor using Clock::to_time_t, not runtime millisecond timestamps.
pub struct NewSpellCooldown {
    pub spell: u32,
    pub item: u32,
    pub end_time: i64,
    pub category: u32,
    pub category_end: i64,
    pub on_hold: bool,
}

pub struct NewSpellChargeCategory {
    pub category: u32,
    /// Preserve the source consumed-charge deque order, including past values.
    pub charges: Vec<NewSpellCharge>,
}

/// Signed epoch seconds after canonical Clock::to_time_t projection.
pub struct NewSpellCharge {
    pub start_time: i64,
    pub end_time: i64,
}
