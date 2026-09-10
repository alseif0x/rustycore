//! Publication operations of void_storage.
//!
//! Divided out of the single inherent impl under #707; every method keeps
//! its name, signature and body.

use super::*;

impl WorldSession {
    pub(super) fn void_storage_withdrawal_container_db_guid_like_cpp(
        &self,
        bag: u8,
        planned_container_db_guids: &HashMap<u8, u64>,
    ) -> Option<u64> {
        planned_container_db_guids
            .get(&bag)
            .copied()
            .or_else(|| self.inventory_container_db_guid_like_cpp(bag))
    }

    pub(super) fn void_storage_withdrawal_container_item_guid_like_cpp(
        &self,
        bag: u8,
        planned_container_item_guids: &HashMap<u8, wow_core::ObjectGuid>,
    ) -> Option<wow_core::ObjectGuid> {
        if bag == INVENTORY_SLOT_BAG_0 {
            return self.player_guid();
        }

        planned_container_item_guids.get(&bag).copied().or_else(|| {
            self.resolved_inventory_item_like_cpp(bag)
                .map(|item| item.guid)
        })
    }

    pub(super) fn new_void_withdrawal_item_publication_like_cpp(
        create_item: &wow_entities::Item,
        post_store_item: &wow_entities::Item,
        create_dynamic_flags: u32,
        container_slots: u32,
        map_id: u16,
    ) -> VoidWithdrawalItemPublicationLikeCpp {
        let post_store = crate::entity_update_bridge::item_values_update_to_update_object(
            post_store_item.object().guid(),
            map_id,
            &post_store_item.values_update(),
        );
        VoidWithdrawalItemPublicationLikeCpp::New {
            create: void_withdrawal_item_create_data_like_cpp(
                create_item,
                create_dynamic_flags,
                container_slots,
            ),
            post_store,
        }
    }

    pub(super) fn merged_void_withdrawal_item_publications_like_cpp(
        item_guid: wow_core::ObjectGuid,
        store_stack_count: u32,
        store_dynamic_flags: Option<u32>,
        post_store_item: &wow_entities::Item,
        map_id: u16,
    ) -> Vec<VoidWithdrawalItemPublicationLikeCpp> {
        let store_update = if let Some(dynamic_flags) = store_dynamic_flags {
            UpdateObject::item_stack_count_and_flags_update(
                item_guid,
                map_id,
                store_stack_count,
                dynamic_flags,
            )
        } else {
            UpdateObject::item_stack_count_update(item_guid, map_id, store_stack_count)
        };
        let mut publications = vec![VoidWithdrawalItemPublicationLikeCpp::Values(store_update)];
        publications.extend(
            crate::entity_update_bridge::item_values_update_to_update_object(
                item_guid,
                map_id,
                &post_store_item.values_update(),
            )
            .map(VoidWithdrawalItemPublicationLikeCpp::Values),
        );
        publications
    }

    pub(super) fn apply_void_withdrawal_post_store_state_like_cpp(
        item: &mut wow_entities::Item,
        creator_guid: wow_core::ObjectGuid,
        random_properties: &EffectiveVoidStorageRandomPropertiesLikeCpp,
    ) {
        item.set_creator(creator_guid);
        // C++ `StoreNewItem` calls `SetItemRandomProperties` after
        // `StoreItem` and applies it to the item returned by `_StoreItem`.
        // On a merge that return value is the destination stack, so the later
        // void item's random properties intentionally replace it.
        Self::apply_effective_void_storage_random_properties_like_cpp(item, random_properties);
        item.set_binding(true);
    }

    pub(super) fn send_void_storage_transfer_result_like_cpp(
        &self,
        result: VoidTransferErrorLikeCpp,
    ) {
        self.send_packet(&VoidTransferResult { result });
    }

    /// Publish the committed item-object lifecycle in the same order as C++
    /// `HandleVoidStorageTransfer`: every deposit `DestroyItem(..., true)` and
    /// its cleared inventory slot precede withdrawal `StoreNewItem(..., true)`
    /// creates, including when a withdrawal reuses a deposited slot.
    pub(super) fn publish_void_storage_item_lifecycle_like_cpp(
        &self,
        map_id: u16,
        destroyed_deposit_items: Vec<((u8, u8), Vec<wow_core::ObjectGuid>)>,
        withdrawal_item_publications: Vec<VoidWithdrawalItemPublicationLikeCpp>,
    ) {
        for ((bag, slot), destroyed_guids) in destroyed_deposit_items {
            self.send_packet(&UpdateObject::destroy_objects(destroyed_guids, map_id));
            if bag == INVENTORY_SLOT_BAG_0 {
                let visible = (slot < 19)
                    .then_some((slot, 0, 0, 0))
                    .into_iter()
                    .collect::<Vec<_>>();
                let virtual_item = ((15..=17).contains(&slot))
                    .then_some((slot - 15, 0, 0, 0))
                    .into_iter()
                    .collect::<Vec<_>>();
                self.send_player_values_update_from_entity_bridge(
                    &[(slot, wow_core::ObjectGuid::EMPTY)],
                    &visible,
                    &virtual_item,
                    &[],
                    None,
                );
            } else {
                self.send_bag_slot_values_update_like_cpp(bag, slot);
            }
        }

        // C++ calls `StoreNewItem(..., true)` once per withdrawal. Preserve
        // that request order rather than batching creates: a first unit can
        // create a new object and a later unit can then publish a VALUES
        // update when it merges into that just-created stack.
        for publication in withdrawal_item_publications {
            match publication {
                VoidWithdrawalItemPublicationLikeCpp::New { create, post_store } => {
                    self.send_packet(&UpdateObject::create_stored_items(vec![create], map_id));
                    if let Some(post_store) = post_store {
                        self.send_packet(&post_store);
                    }
                }
                VoidWithdrawalItemPublicationLikeCpp::Values(update) => {
                    self.send_packet(&update);
                }
            }
        }
    }
}
