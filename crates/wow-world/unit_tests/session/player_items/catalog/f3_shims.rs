// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub fn set_item_price_base_store(&mut self, store: Arc<ItemPriceBaseStore>) {
        self.catalogs.set_item_price_base_store(store)
    }
    #[cfg(test)]
    pub fn set_item_class_store(&mut self, store: Arc<ItemClassStore>) {
        self.catalogs.set_item_class_store(store)
    }
}
