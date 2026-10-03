// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::item::CurrencyTypesFlags;
use wow_data::CurrencyTypesEntry;
use wow_entities::PlayerCurrency;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCurrencyDelta {
    pub currency_id: u32,
    pub quantity: u32,
    pub amount: u32,
    pub weekly_quantity: Option<u32>,
    pub max_quantity: Option<u32>,
    pub total_earned: Option<u32>,
    pub suppress_chat_log: bool,
}

pub fn currency_max_quantity_cpp(
    entry: &CurrencyTypesEntry,
    currency: &PlayerCurrency,
) -> u32 {
    if !entry.has_max_quantity(false, false) {
        return 0;
    }

    let mut max_quantity = entry.max_qty;
    if entry.flags.contains(CurrencyTypesFlags::DYNAMIC_MAXIMUM) {
        max_quantity = max_quantity.saturating_add(currency.increased_cap_quantity);
    }
    max_quantity
}
