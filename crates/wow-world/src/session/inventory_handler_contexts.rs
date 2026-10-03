// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_world_inventory::{
    EquipmentSetsHandlerCxLikeCpp, EquipmentSetsSaveCxLikeCpp, InventoryHandlerHostLikeCpp,
};
use wow_core::EquipmentSetGuidGeneratorLikeCpp;

use super::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl WorldSession {
    pub(crate) fn build_equipment_sets_handler_cx_like_cpp<'a>(
        &'a mut self,
    ) -> EquipmentSetsHandlerCxLikeCpp<'a> {
        let owner = self.core.owned_equipment_sets_access_like_cpp();
        EquipmentSetsHandlerCxLikeCpp::new(&mut self.inventory, owner)
    }

    pub(crate) fn build_equipment_sets_save_handler_cx_like_cpp<'a>(
        &'a mut self,
        generator: &'a EquipmentSetGuidGeneratorLikeCpp,
    ) -> EquipmentSetsSaveCxLikeCpp<'a> {
        let equipment_sets = self.core.owned_equipment_sets_access_like_cpp();
        let inventory = self.core.owned_inventory_access_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        let collections = self
            .core
            .owned_collections_access_like_cpp()
            .with_fixture_collections(&self.fixtures.collections);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let collections = self.core.owned_collections_access_like_cpp();
        let item_modified_appearance_store = self
            .catalogs
            .item_modified_appearance_store()
            .map(|store| store.as_ref());
        let spell_item_enchantment_store = self
            .catalogs
            .spell_item_enchantment_store()
            .map(|store| store.as_ref());
        let publication = self.core.packet_publication_access_like_cpp();
        EquipmentSetsSaveCxLikeCpp::new(
            &mut self.inventory,
            equipment_sets,
            inventory,
            collections,
            item_modified_appearance_store,
            spell_item_enchantment_store,
            generator,
            publication,
        )
    }
}

impl InventoryHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn equipment_sets_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> EquipmentSetsHandlerCxLikeCpp<'a> {
        self.build_equipment_sets_handler_cx_like_cpp()
    }

    fn equipment_sets_save_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> EquipmentSetsSaveCxLikeCpp<'a> {
        self.build_equipment_sets_save_handler_cx_like_cpp(
            catalogs.id_generators.equipment_set.as_ref(),
        )
    }
}
