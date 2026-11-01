use std::sync::Arc;

use crate::session_policy::LootDropRatesLikeCpp;
use wow_loot::{LootStoreKind, LootStores};

impl crate::session::state::SessionWorldConfig {
    pub fn enable_ae_loot_like_cpp(&self) -> bool {
        self.enable_ae_loot_like_cpp
    }

    pub fn loot_drop_rates_like_cpp(&self) -> LootDropRatesLikeCpp {
        self.loot_drop_rates
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn represented_gameobject_loot_ids_have_quest_loot_like_cpp(
        &self,
        loot_ids: impl IntoIterator<Item = u32>,
    ) -> bool {
        let Some(stores) = self.loot_stores.as_ref() else {
            return false;
        };
        let Some(store) = stores.get(&LootStoreKind::Gameobject) else {
            return false;
        };
        loot_ids
            .into_iter()
            .filter(|id| *id != 0)
            .any(|loot_id| store.have_quest_loot_for_like_cpp(loot_id, stores.as_ref()))
    }

    /// Get the C++ LootTemplates_* foundation stores.
    pub fn loot_stores(&self) -> Option<&Arc<LootStores>> {
        self.loot_stores.as_ref()
    }
}
