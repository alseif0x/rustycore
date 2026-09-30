//! Shared `CurrencyTypes.db2` schema and pure helper behavior.

use wow_constants::{CurrencyTypesFlags, CurrencyTypesFlagsB};

/// C++ `CurrencyTypesEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrencyTypesEntry {
    pub id: u32,
    pub category_id: u8,
    pub inventory_icon_file_id: i32,
    pub spell_weight: u32,
    pub spell_category: u8,
    pub max_qty: u32,
    pub max_earnable_per_week: u32,
    pub quality: i8,
    pub faction_id: i32,
    pub award_condition_id: i32,
    pub flags: CurrencyTypesFlags,
    pub flags_b: CurrencyTypesFlagsB,
}

/// Immutable result of a gain applied to the canonical Player currency record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrencyGainDelta {
    pub currency_id: u32,
    pub quantity: u32,
    pub amount: u32,
    pub weekly_quantity: Option<u32>,
    pub max_quantity: Option<u32>,
    pub total_earned: Option<u32>,
    pub suppress_chat_log: bool,
}

impl CurrencyTypesEntry {
    pub fn scaler(&self) -> i32 {
        if self.flags.contains(CurrencyTypesFlags::SCALER_100) {
            100
        } else {
            1
        }
    }

    pub fn has_max_earnable_per_week(&self) -> bool {
        self.max_earnable_per_week != 0
            || self
                .flags
                .contains(CurrencyTypesFlags::COMPUTED_WEEKLY_MAXIMUM)
    }

    pub fn has_max_quantity(&self, on_load: bool, on_update_version: bool) -> bool {
        if on_load
            && self
                .flags
                .contains(CurrencyTypesFlags::IGNORE_MAX_QTY_ON_LOAD)
        {
            return false;
        }

        if on_update_version
            && self
                .flags
                .contains(CurrencyTypesFlags::UPDATE_VERSION_IGNORE_MAX)
        {
            return false;
        }

        self.max_qty != 0 || self.flags.contains(CurrencyTypesFlags::DYNAMIC_MAXIMUM)
    }

    pub fn has_total_earned(&self) -> bool {
        self.flags_b
            .contains(CurrencyTypesFlagsB::USE_TOTAL_EARNED_FOR_EARNED)
    }

    pub fn is_alliance(&self) -> bool {
        self.flags.contains(CurrencyTypesFlags::IS_ALLIANCE_ONLY)
    }

    pub fn is_horde(&self) -> bool {
        self.flags.contains(CurrencyTypesFlags::IS_HORDE_ONLY)
    }

    pub fn is_suppressing_chat_log(&self, on_update_version: bool) -> bool {
        (on_update_version
            && self
                .flags
                .contains(CurrencyTypesFlags::SUPPRESS_CHAT_MESSAGE_ON_VERSION_CHANGE))
            || self
                .flags
                .contains(CurrencyTypesFlags::SUPPRESS_CHAT_MESSAGES)
    }

    pub fn is_tracking_quantity(&self) -> bool {
        self.flags.contains(CurrencyTypesFlags::TRACK_QUANTITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn currency_type_helpers_match_cpp_flags() {
        let entry = CurrencyTypesEntry {
            id: 1,
            category_id: 0,
            inventory_icon_file_id: 0,
            spell_weight: 0,
            spell_category: 0,
            max_qty: 0,
            max_earnable_per_week: 0,
            quality: 0,
            faction_id: 0,
            award_condition_id: 0,
            flags: CurrencyTypesFlags::SCALER_100
                | CurrencyTypesFlags::DYNAMIC_MAXIMUM
                | CurrencyTypesFlags::IS_ALLIANCE_ONLY
                | CurrencyTypesFlags::TRACK_QUANTITY,
            flags_b: CurrencyTypesFlagsB::USE_TOTAL_EARNED_FOR_EARNED,
        };

        assert_eq!(entry.scaler(), 100);
        assert!(entry.has_max_quantity(false, false));
        assert!(entry.has_total_earned());
        assert!(entry.is_alliance());
        assert!(!entry.is_horde());
        assert!(entry.is_tracking_quantity());
    }

    #[test]
    fn currency_type_max_quantity_ignore_flags_match_cpp() {
        let base = CurrencyTypesEntry {
            id: 1,
            category_id: 0,
            inventory_icon_file_id: 0,
            spell_weight: 0,
            spell_category: 0,
            max_qty: 10,
            max_earnable_per_week: 0,
            quality: 0,
            faction_id: 0,
            award_condition_id: 0,
            flags: CurrencyTypesFlags::empty(),
            flags_b: CurrencyTypesFlagsB::empty(),
        };
        let ignored_on_load = CurrencyTypesEntry {
            flags: CurrencyTypesFlags::IGNORE_MAX_QTY_ON_LOAD,
            ..base
        };
        let ignored_on_update = CurrencyTypesEntry {
            flags: CurrencyTypesFlags::UPDATE_VERSION_IGNORE_MAX,
            ..base
        };

        assert!(!ignored_on_load.has_max_quantity(true, false));
        assert!(ignored_on_load.has_max_quantity(false, false));
        assert!(!ignored_on_update.has_max_quantity(false, true));
        assert!(ignored_on_update.has_max_quantity(false, false));
    }
}
