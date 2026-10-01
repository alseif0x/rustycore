// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub fn transmog_set_items_like_cpp(
        &self,
        transmog_set_id: u32,
    ) -> Option<&[wow_data::TransmogSetItemEntry]> {
        self.catalogs.transmog_set_items_like_cpp(transmog_set_id)
    }
    pub fn transmog_set_item_modified_appearances_like_cpp(
        &self,
        transmog_set_id: u32,
    ) -> Vec<&wow_data::ItemModifiedAppearanceEntry> {
        self.catalogs
            .transmog_set_item_modified_appearances_like_cpp(transmog_set_id)
    }
    #[cfg(test)]
    pub(crate) fn represented_has_item_appearance_like_cpp(
        &self,
        item_modified_appearance_id: u32,
    ) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_has_item_appearance_like_cpp(hub, item_modified_appearance_id)
    }
    #[allow(dead_code)]
    pub(crate) fn has_transmog_illusion_like_cpp(&self, transmog_illusion_id: u32) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.has_transmog_illusion_like_cpp(hub, transmog_illusion_id)
    }
    #[cfg(test)]
    pub(crate) fn represented_favorite_item_appearance_state_like_cpp(
        &self,
        item_modified_appearance_id: u32,
    ) -> Option<FavoriteAppearanceStateLikeCpp> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_favorite_item_appearance_state_like_cpp(hub, item_modified_appearance_id)
    }
    pub fn is_transmog_set_completed_like_cpp(&self, transmog_set_id: u32) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.is_transmog_set_completed_like_cpp(hub, transmog_set_id)
    }
    pub(crate) fn account_item_appearance_save_plan_like_cpp(
        &mut self,
    ) -> Option<AccountItemAppearanceSavePlanLikeCpp> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.account_item_appearance_save_plan_like_cpp(&mut hub)
    }
    pub(crate) fn account_transmog_illusion_save_plan_like_cpp(
        &self,
    ) -> Option<AccountTransmogIllusionSavePlanLikeCpp> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.account_transmog_illusion_save_plan_like_cpp(hub)
    }
    pub(crate) fn account_transmog_active_player_rows_like_cpp(&self) -> Vec<u32> {
        crate::session::hub_ref(self).account_transmog_active_player_rows_like_cpp()
    }
}
