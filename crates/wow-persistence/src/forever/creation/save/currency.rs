//! Frozen unordered currency-map save projection; canonical lookup/apply is absent.

pub struct NewCurrencySave {
    pub currency_id: u32,
    /// Required frozen CurrencyTypesStore::LookupEntry outcome, not a fallback.
    pub has_currency_entry: bool,
    pub state: NewCurrencySaveState,
    pub quantity: u32,
    pub weekly_quantity: u32,
    pub tracked_quantity: u32,
    pub increased_cap_quantity: u32,
    pub earned_quantity: u32,
    pub flags: u8,
}

#[derive(Clone, Copy)]
pub enum NewCurrencySaveState {
    Unchanged,
    Changed,
    New,
    Removed,
}
