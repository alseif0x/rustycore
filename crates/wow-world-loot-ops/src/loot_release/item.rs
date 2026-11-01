// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;
use tracing::warn;
use wow_entities::{INVENTORY_SLOT_BAG_0, Item, ItemObjectUpdateLikeCpp};
use wow_packet::packets::item::ItemExpirePurchaseRefund;
use wow_packet::packets::update::UpdateObject;

/// C++ `Item::GetCount()` after a partial/full loot release consumption.
pub fn direct_item_count_after_loot_release_like_cpp(
    current_count: u32,
    maximum_destroy_count: Option<u32>,
) -> u32 {
    let destroy_count = maximum_destroy_count
        .unwrap_or(current_count)
        .min(current_count);
    current_count.saturating_sub(destroy_count)
}

impl LootReleaseCxLikeCpp<'_> {
    fn remove_fully_looted_runtime_item(&mut self, bag: u8, slot: u8, item_guid: ObjectGuid) {
        if bag == INVENTORY_SLOT_BAG_0
            && self
                .inventory
                .quest_reward_inventory_item_from_runtime_with_access_like_cpp(
                    &self.owner.inventory_like_cpp(),
                    slot,
                )
                .is_some_and(|item| item.guid == item_guid)
        {
            self.inventory
                .remove_inventory_item_with_access_like_cpp(&self.owner.inventory_like_cpp(), slot);
        }
        self.inventory
            .remove_inventory_item_object_with_access_like_cpp(
                &self.owner.inventory_like_cpp(),
                item_guid,
            );
        self.sync_player_registry_state_like_cpp();
    }

    pub async fn destroy_fully_looted_direct_item(&mut self, item_guid: ObjectGuid) {
        self.destroy_direct_item_count_after_loot_release_like_cpp(item_guid, None)
            .await;
    }

    pub(super) async fn destroy_direct_item_count_after_loot_release_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        maximum_destroy_count: Option<u32>,
    ) {
        let player_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };

        let runtime_item = self.resolved_inventory_item_object_like_cpp(item_guid);
        let (bag, slot) = match runtime_item.as_ref() {
            Some(item) => (item.bag_slot(), item.slot()),
            None => return,
        };

        let Some(item) = self.get_inventory_item_by_pos(bag, slot) else {
            return;
        };

        let port = match self.lifecycle.stored_item_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };

        let current_count = runtime_item.as_ref().map_or(1, Item::count);
        let new_count =
            direct_item_count_after_loot_release_like_cpp(current_count, maximum_destroy_count);
        if new_count != 0 {
            match port
                .update_inventory_item_count_like_cpp(
                    wow_persistence::InventoryItemCountPersistenceRequestLikeCpp {
                        item_guid: item.db_guid,
                        count: new_count,
                    },
                )
                .await
            {
                wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
                wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
                | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                    warn!(error = %reason, "LootRelease: update partially consumed item failed");
                    return;
                }
            }
            let _ = self.apply_inventory_item_object_updates_like_cpp(
                item_guid,
                &[
                    ItemObjectUpdateLikeCpp::SetCount(new_count),
                    ItemObjectUpdateLikeCpp::SetLootGenerated(false),
                ],
            );
            self.send_packet(&UpdateObject::item_stack_count_update(
                item_guid,
                self.owner.player_map_id_like_cpp(),
                new_count,
            ));
            return;
        }

        let should_expire_refund = runtime_item
            .as_ref()
            .is_some_and(|item_object| item_object.is_refundable());
        match port
            .destroy_inventory_item_like_cpp(
                wow_persistence::InventoryItemDestroyPersistenceRequestLikeCpp {
                    owner_guid: player_guid.counter() as u64,
                    item_guid: item.db_guid,
                    expire_refund: should_expire_refund,
                },
            )
            .await
        {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(error = %reason, "LootRelease: delete fully looted item failed");
                return;
            }
        }

        self.remove_fully_looted_runtime_item(bag, slot, item.guid);

        if should_expire_refund {
            self.send_packet(&ItemExpirePurchaseRefund {
                item_guid: item.guid,
            });
        }

        // Player-values update and stat refresh only apply to top-level slots.
        if bag == INVENTORY_SLOT_BAG_0 {
            let mut visible_item_changes = Vec::new();
            let mut virtual_item_changes = Vec::new();
            if (slot as usize) < 19 {
                visible_item_changes.push((slot, 0i32, 0u16, 0u16));
            }
            if slot >= 15 && slot <= 17 {
                virtual_item_changes.push((slot - 15, 0i32, 0u16, 0u16));
            }

            self.send_player_values_update_from_entity_bridge(
                &[(slot, ObjectGuid::EMPTY)],
                &visible_item_changes,
                &virtual_item_changes,
                &[],
                None,
            );

            if slot < 19 {
                self.send_stat_update();
            }
        }
    }
}
