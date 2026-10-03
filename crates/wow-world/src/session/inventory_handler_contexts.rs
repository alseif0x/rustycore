// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_world_inventory::{EquipmentSetsHandlerCxLikeCpp, InventoryHandlerHostLikeCpp};

use super::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl WorldSession {
    pub(crate) fn build_equipment_sets_handler_cx_like_cpp<'a>(
        &'a mut self,
    ) -> EquipmentSetsHandlerCxLikeCpp<'a> {
        let owner = self.core.owned_equipment_sets_access_like_cpp();
        EquipmentSetsHandlerCxLikeCpp::new(&mut self.inventory, owner)
    }
}

impl InventoryHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn equipment_sets_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> EquipmentSetsHandlerCxLikeCpp<'a> {
        self.build_equipment_sets_handler_cx_like_cpp()
    }
}
