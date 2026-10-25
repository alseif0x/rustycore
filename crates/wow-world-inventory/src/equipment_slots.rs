// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_data::SpellEquippedItemsEntry;
use wow_world_core::session::HubRef;

impl crate::InventoryState {
    /// C++ `Player::HasItemFitToSpellRequirements` (`Player.cpp:24641-24708`)
    /// includes ignore/shield handling that this represented check does not
    /// model. There is no direct World caller: Inventory's
    /// `items::represented_has_item_fit_to_spell_requirements_like_cpp` invokes
    /// this helper from the login passive-equipment path. This does not assert
    /// that the absent branch rejects that caller.
    pub(crate) fn represented_equipped_item_in_slot_fits_spell_requirements_like_cpp(
        &self,
        hub: HubRef<'_>,
        slot: u8,
        equipped: &SpellEquippedItemsEntry,
    ) -> bool {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.resolved_inventory_item_with_access_like_cpp(&access, slot)
            .and_then(|item| {
                self.resolved_inventory_item_object_with_access_like_cpp(&access, item.guid)
            })
            .is_some_and(|item| {
                item.container_guid().is_empty()
                    && item.slot() == slot
                    && hub
                        .catalogs
                        .represented_item_fits_spell_requirements_like_cpp(
                            item.object().entry(),
                            equipped,
                        )
            })
    }
}
