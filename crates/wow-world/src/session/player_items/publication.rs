//! Values updates and packets published for represented item state.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    pub(in crate::session) fn broadcast_item_push_result_to_group(&self, bytes: Vec<u8>) -> bool {
        let (Some(group_guid), Some(group_registry), Some(player_registry)) = (
            self.resolved_group_guid_like_cpp(),
            &self.group_registry,
            &self.player_registry,
        ) else {
            return false;
        };

        let Some(group) = group_registry.get(&group_guid) else {
            return false;
        };

        let mut delivered = false;
        for member_guid in &group.members {
            if let Some(member) = player_registry.loot_presence(*member_guid) {
                delivered |= player_registry
                    .send_current_realm_packet(member.registration, bytes.clone())
                    .is_ok();
            }
        }

        delivered
    }
    pub(crate) fn send_item_contained_in_values_update_like_cpp(&self, item_guid: ObjectGuid) {
        self.send_item_storage_fields_values_update_like_cpp(item_guid, true, false, &[]);
    }
    pub(crate) fn send_item_relocation_values_update_like_cpp(
        &self,
        item_guid: ObjectGuid,
        dynamic_flags2_changed: bool,
        cleared_enchantments: &[EnchantmentSlot],
    ) {
        self.send_item_storage_fields_values_update_like_cpp(
            item_guid,
            true,
            dynamic_flags2_changed,
            cleared_enchantments,
        );
    }
    fn send_item_storage_fields_values_update_like_cpp(
        &self,
        item_guid: ObjectGuid,
        contained_in_changed: bool,
        dynamic_flags2_changed: bool,
        changed_enchantments: &[EnchantmentSlot],
    ) {
        let Some(item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return;
        };
        let update = Self::item_storage_fields_values_update_like_cpp(
            &item,
            contained_in_changed,
            dynamic_flags2_changed,
            changed_enchantments,
        );
        if let Some(packet) =
            item_values_update_to_update_object(item_guid, self.player_map_id_like_cpp(), &update)
        {
            self.send_packet(&packet);
        }
    }
    pub(crate) fn item_storage_fields_values_update_like_cpp(
        item: &Item,
        contained_in_changed: bool,
        dynamic_flags2_changed: bool,
        changed_enchantments: &[EnchantmentSlot],
    ) -> ItemValuesUpdate {
        let mut item_data_mask = UpdateMask::new(ITEM_DATA_BITS);
        if contained_in_changed || dynamic_flags2_changed {
            item_data_mask.set(ITEM_DATA_PARENT_BIT);
        }
        if contained_in_changed {
            item_data_mask.set(ITEM_DATA_CONTAINED_IN_BIT);
        }
        if dynamic_flags2_changed {
            item_data_mask.set(ITEM_DATA_DYNAMIC_FLAGS2_BIT);
        }
        if !changed_enchantments.is_empty() {
            item_data_mask.set(ITEM_DATA_ENCHANTMENT_PARENT_BIT);
            for slot in changed_enchantments {
                item_data_mask.set(ITEM_DATA_ENCHANTMENT_FIRST_BIT + *slot as usize);
            }
        }
        ItemValuesUpdate {
            changed_object_type_mask: 1 << TYPEID_ITEM,
            object_data: None,
            item_data: Some(ItemDataUpdate {
                mask: item_data_mask,
                values: item.data().clone(),
            }),
        }
    }
    pub(crate) fn send_item_dynamic_flags_values_update_like_cpp(&self, item_guid: ObjectGuid) {
        let Some(item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return;
        };
        let mut item_data_mask = UpdateMask::new(ITEM_DATA_BITS);
        item_data_mask.set(ITEM_DATA_PARENT_BIT);
        item_data_mask.set(ITEM_DATA_DYNAMIC_FLAGS_BIT);
        let update = ItemValuesUpdate {
            changed_object_type_mask: 1 << TYPEID_ITEM,
            object_data: None,
            item_data: Some(ItemDataUpdate {
                mask: item_data_mask,
                values: item.data().clone(),
            }),
        };
        if let Some(packet) =
            item_values_update_to_update_object(item_guid, self.player_map_id_like_cpp(), &update)
        {
            self.send_packet(&packet);
        }
    }
    pub(crate) fn send_repeatable_turn_in_request_items_like_cpp(
        &mut self,
        sender_guid: ObjectGuid,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        let can_complete = self.can_complete_repeatable_quest_represented_bounded_like_cpp(quest);
        if can_complete && !quest_has_represented_item_objective_like_cpp(quest) {
            self.send_represented_quest_giver_offer_reward_like_cpp(sender_guid, quest, true);
            return;
        }

        self.send_represented_quest_giver_request_items_with_completion_like_cpp(
            sender_guid,
            quest,
            can_complete,
            true,
        );
    }
}
