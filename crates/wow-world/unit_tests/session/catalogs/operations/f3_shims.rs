// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub fn set_import_price_stores(&mut self, stores: Arc<ImportPriceStores>) {
        self.catalogs.set_import_price_stores(stores)
    }
    #[cfg(test)]
    pub fn set_tact_key_store(&mut self, store: Arc<TactKeyStore>) {
        self.catalogs.set_tact_key_store(store)
    }
    #[cfg(test)]
    pub fn set_graveyard_store(&mut self, store: Arc<GraveyardStore>) {
        self.catalogs.set_graveyard_store(store)
    }
    #[cfg(test)]
    pub fn set_battlemaster_list_store(&mut self, store: Arc<BattlemasterListStore>) {
        self.catalogs.set_battlemaster_list_store(store)
    }
    #[cfg(test)]
    pub fn set_pet_levelup_spell_store(&mut self, store: Arc<PetLevelupSpellStoreLikeCpp>) {
        self.catalogs.set_pet_levelup_spell_store(store)
    }
    #[cfg(test)]
    pub fn set_pet_default_spell_store(&mut self, store: Arc<PetDefaultSpellStoreLikeCpp>) {
        self.catalogs.set_pet_default_spell_store(store)
    }
    #[cfg(test)]
    pub fn set_pet_family_spell_store(&mut self, store: Arc<PetFamilySpellStoreLikeCpp>) {
        self.catalogs.set_pet_family_spell_store(store)
    }
    #[cfg(test)]
    pub fn set_object_mgr_catalogs_like_cpp(&mut self, catalogs: Arc<ObjectMgrCatalogsLikeCpp>) {
        self.catalogs.set_object_mgr_catalogs_like_cpp(catalogs)
    }
    #[cfg(test)]
    pub fn set_exploration_base_xp_store_like_cpp(
        &mut self,
        store: Arc<ExplorationBaseXpStoreLikeCpp>,
    ) {
        self.catalogs.set_exploration_base_xp_store_like_cpp(store)
    }
}
