//! Represented item enchantment state and its loaded effects.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    /// Set the item random enchantment template store for this session.
    pub fn set_item_random_enchantment_template_store(
        &mut self,
        store: Arc<ItemRandomEnchantmentTemplateStore>,
    ) {
        self.catalogs.items.random_enchantment_template_store = Some(store);
    }
    /// Set the spell item enchantment store for this session.
    pub fn set_spell_item_enchantment_store(&mut self, store: Arc<SpellItemEnchantmentStore>) {
        self.catalogs.spell_catalogs.spell_item_enchantment_store = Some(store);
    }
    pub fn set_spell_item_enchantment_condition_store(
        &mut self,
        store: Arc<SpellItemEnchantmentConditionStore>,
    ) {
        self.catalogs
            .spell_catalogs
            .spell_item_enchantment_condition_store = Some(store);
    }
}

impl wow_world_application::LoadedItemModsHostLikeCpp for WorldSession {
    fn apply_current_player_item_enchantment_plan_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
        args: ApplyEnchantmentArgs,
    ) -> Option<ApplyEnchantmentPlan> {
        self.build_item_enchantment_handler_cx_like_cpp()
            .apply_current_player_item_enchantment_plan_like_cpp(item_guid, slot, args)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/enchantment/f3_shims.rs"]
mod f3_shims;
