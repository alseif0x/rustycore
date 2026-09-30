//! Remaining represented item operations owned by the inventory responsibility.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

mod loot_queries;
mod guids;
mod item_sets;
mod buyback;
mod spell_requirements;
mod economy_and_pet_adapters;

impl WorldSession {
    #[cfg(test)]
    pub(crate) fn set_vendor_buy_item_test_override_like_cpp(
        &mut self,
        item: VendorBuyItemTestOverrideLikeCpp,
    ) {
        self.vendor_buy_item_test_override_like_cpp = Some(item);
    }
    #[cfg(test)]
    pub(crate) fn vendor_buy_item_test_override_like_cpp(
        &self,
    ) -> Option<VendorBuyItemTestOverrideLikeCpp> {
        self.vendor_buy_item_test_override_like_cpp
    }
    /// Bounded C++ `CollectionMgr::OnItemAdded`.
    pub(crate) fn on_item_added_to_collection_like_cpp(
        &mut self,
        item: &wow_entities::Item,
    ) -> Vec<wow_entities::PlayerValuesUpdate> {
        let item_id = item.object().entry();
        let mut updates = Vec::new();

        if self
            .heirloom_store
            .as_ref()
            .and_then(|store| store.get_by_item_id_like_cpp(item_id))
            .is_some()
            && self.add_account_heirloom_like_cpp(item_id, 0)
            && let Some(update) = self.add_player_heirloom_dynamic_fields_like_cpp(item_id, 0)
        {
            updates.push(update);
        }

        if let Some(update) = self.add_item_appearance_for_runtime_item_like_cpp(item) {
            updates.push(update);
        }

        updates
    }
    pub(in crate::session) fn item_spec_class_mask_from_overrides_like_cpp(
        &self,
        item_id: u32,
    ) -> Option<u32> {
        let overrides = self
            .items
            .spec_override_store
            .as_ref()?
            .overrides_for_item_like_cpp(item_id)?;
        let chr_specializations = self.chr.specialization_store.as_ref()?;

        let mut mask = 0_u32;
        for item_spec_override in overrides {
            if let Some(specialization) =
                chr_specializations.get(u32::from(item_spec_override.spec_id))
            {
                mask |= player_class_mask_for_transmog_like_cpp(specialization.class_id);
            }
        }

        Some(mask)
    }
    /// C++ `DB2Manager::GetItemDisplayId`.
    pub fn item_display_id(&self, item_id: u32, appearance_mod_id: u32) -> Option<u32> {
        let modified = self
            .items
            .modified_appearance_store
            .as_ref()
            .and_then(|store| store.get_for_item(item_id, appearance_mod_id))?;
        let appearance_id = u32::try_from(modified.item_appearance_id).ok()?;
        self.items
            .appearance_store
            .as_ref()
            .and_then(|store| store.item_display_info_id(appearance_id))
    }
    pub(in crate::session) fn represented_top_level_item_mod_targets_like_cpp(
        &self,
    ) -> Option<Vec<(u8, ObjectGuid)>> {
        let mut targets = self
            .resolved_inventory_item_objects_like_cpp()?
            .values()
            .filter(|item| {
                item.container_guid().is_empty()
                    && item.slot() < INVENTORY_SLOT_BAG_END
                    && !item.is_broken()
            })
            .map(|item| (item.slot(), item.object().guid()))
            .collect::<Vec<_>>();
        targets.sort_by_key(|(slot, guid)| (*slot, guid.counter()));
        Some(targets)
    }
    /// C++ `Item::IsBoundAccountWide` template-flag predicate.
    pub fn is_item_bound_account_wide(&self, item_id: u32) -> bool {
        self.item_template_flags(item_id)
            .is_some_and(|flags| flags.contains(ItemFlags::IS_BOUND_TO_ACCOUNT))
    }
    pub(in crate::session) fn item_shield_block_value_like_cpp(&self, item_id: u32) -> Option<i16> {
        let basic = self.items.store.as_ref()?.get(item_id)?;
        if basic.class_id != ItemClass::Armor as u8
            || basic.subclass_id != ItemSubClassArmor::Shield as u8
        {
            return None;
        }

        let template = self.item_random_property_template(item_id)?;
        let item_level = u32::from(template.item_level);
        let quality = u32::try_from(template.quality).ok()?;
        self.shield_block_regular_game_table
            .as_ref()?
            .shield_block_for_quality_like_cpp(item_level, quality)
            .filter(|value| *value != 0)
    }
}
