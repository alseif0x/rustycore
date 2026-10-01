// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub fn item_disenchant_loot_like_cpp(
        &self,
        item_id: u32,
        quality: u32,
        item_level: u32,
        can_disenchant_bonus: bool,
    ) -> Option<(u32, u16)> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.item_disenchant_loot_like_cpp(hub, item_id, quality, item_level, can_disenchant_bonus)
    }
    #[cfg(test)]
    pub fn set_item_disenchant_loot_store(&mut self, store: Arc<ItemDisenchantLootStore>) {
        self.catalogs.set_item_disenchant_loot_store(store)
    }
    #[cfg(test)]
    pub fn set_spell_enchant_proc_store(&mut self, store: Arc<SpellEnchantProcStoreLikeCpp>) {
        self.catalogs.set_spell_enchant_proc_store(store)
    }
    #[cfg(test)]
    pub(crate) fn spell_enchant_proc_event_like_cpp(
        &self,
        enchantment_id: u32,
    ) -> Option<&SpellEnchantProcEntryLikeCpp> {
        self.catalogs
            .spell_enchant_proc_event_like_cpp(enchantment_id)
    }
    pub fn send_item_enchant_time_update_plan(
        &self,
        owner_guid: ObjectGuid,
        update: &PlayerEnchantTimeUpdate,
    ) {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.send_item_enchant_time_update_plan(hub, owner_guid, update)
    }
}
