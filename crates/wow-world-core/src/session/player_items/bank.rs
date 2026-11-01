#[cfg(any(test, feature = "test-fixtures"))]
use std::sync::Arc;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_data::BankBagSlotPricesStore;

impl crate::session::state::SessionCatalogs {
    /// Set the C++ BankBagSlotPrices.db2 store for this session.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_bank_bag_slot_prices_store(&mut self, store: Arc<BankBagSlotPricesStore>) {
        self.bank_bag_slot_prices_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn bank_bag_slot_prices_store_for_test_like_cpp(
        &self,
    ) -> Option<&Arc<BankBagSlotPricesStore>> {
        self.bank_bag_slot_prices_store.as_ref()
    }
}
