use std::sync::Arc;

use wow_data::CurrencyTypesStore;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_data::ItemCurrencyCostStore;

impl crate::session::state::SessionCatalogs {
    /// Set the item currency cost store for this session.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_item_currency_cost_store(&mut self, store: Arc<ItemCurrencyCostStore>) {
        self.item_currency_cost_store = Some(store);
    }
}

impl crate::session::state::SessionCatalogs {
    /// Get the currency types store reference.
    pub fn currency_types_store(&self) -> Option<&Arc<CurrencyTypesStore>> {
        self.currency_types_store.as_ref()
    }
}
