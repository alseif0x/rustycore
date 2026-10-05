// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;
use wow_entities::INVENTORY_SLOT_BAG_0;
use wow_world_core::session::{
    HubRef, OwnedInventoryAccessLikeCpp, PacketPublicationAccessLikeCpp,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtendedCostItemTurninChange {
    Update {
        slot: u8,
        item_guid: ObjectGuid,
        db_guid: u64,
        new_count: u32,
    },
    Delete {
        slot: u8,
        item_guid: ObjectGuid,
        db_guid: u64,
    },
}

impl crate::InventoryState {
    /// Apply the ordered turn-in changes and publish their inventory values.
    /// The caller owns the subsequent stats phase when equipped slots changed.
    pub fn apply_item_turnin_changes_with_access_like_cpp(
        &mut self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        publication: &PacketPublicationAccessLikeCpp<'_>,
        item_store: Option<&std::sync::Arc<wow_data::ItemStore>>,
        item_stats_store: Option<&std::sync::Arc<wow_data::ItemStatsStore>>,
        _player_guid: ObjectGuid,
        map_id: u16,
        changes: &[ExtendedCostItemTurninChange],
    ) -> bool {
        let mut cleared_slots = Vec::new();
        let mut visible_item_changes = Vec::new();
        let mut virtual_item_changes = Vec::new();
        let mut send_stat_update = false;

        for change in changes {
            match *change {
                ExtendedCostItemTurninChange::Update {
                    item_guid,
                    new_count,
                    ..
                } => {
                    let _ = self
                        .mutate_player_inventory_runtime_with_access_like_cpp(access, |inventory| {
                            inventory.apply_item_object_updates_like_cpp(
                                item_guid,
                                &[wow_entities::ItemObjectUpdateLikeCpp::SetCount(new_count)],
                            )
                        })
                        .unwrap_or(false);
                    publication.send_packet(
                        &wow_packet::packets::update::UpdateObject::item_stack_count_update(
                            item_guid, map_id, new_count,
                        ),
                    );
                }
                ExtendedCostItemTurninChange::Delete {
                    slot, item_guid, ..
                } => {
                    self.mutate_player_inventory_runtime_with_access_like_cpp(
                        access,
                        |inventory| inventory.remove_item_from_slot_like_cpp(slot),
                    )
                    .flatten();
                    self.mutate_player_inventory_runtime_with_access_like_cpp(
                        access,
                        |inventory| inventory.remove_item_object_like_cpp(item_guid),
                    )
                    .flatten();
                    cleared_slots.push((slot, ObjectGuid::EMPTY));
                    if (slot as usize) < 19 {
                        visible_item_changes.push((slot, 0i32, 0u16, 0u16));
                        send_stat_update = true;
                    }
                    if (15..=17).contains(&slot) {
                        virtual_item_changes.push((slot - 15, 0i32, 0u16, 0u16));
                    }
                }
            }
        }

        if !cleared_slots.is_empty() {
            self.send_player_values_update_from_entity_bridge_with_access_like_cpp(
                access,
                publication,
                item_store,
                item_stats_store,
                &cleared_slots,
                &visible_item_changes,
                &virtual_item_changes,
                &[],
                None,
            );
        }
        send_stat_update
    }

    pub fn inventory_container_db_guid_like_cpp(&self, hub: HubRef<'_>, bag: u8) -> Option<u64> {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.inventory_container_db_guid_with_access_like_cpp(&access, bag)
    }

    pub fn inventory_container_db_guid_with_access_like_cpp(
        &self,
        access: &wow_world_core::session::OwnedInventoryAccessLikeCpp<'_>,
        bag: u8,
    ) -> Option<u64> {
        if bag == INVENTORY_SLOT_BAG_0 {
            Some(0)
        } else {
            self.quest_reward_inventory_item_from_runtime_with_access_like_cpp(access, bag)
                .map(|item| item.db_guid)
        }
    }

    pub fn has_item_count_direct_inventory(
        &self,
        hub: HubRef<'_>,
        item_entry: u32,
        count: u32,
    ) -> bool {
        if count == 0 {
            return true;
        }

        let Some(inventory_items) = self.resolved_inventory_items_like_cpp(hub) else {
            return false;
        };
        let mut current_count = 0_u32;
        let mut slots: Vec<_> = inventory_items.iter().collect();
        slots.sort_by_key(|&(slot, _)| {
            let slot = *slot;
            if slot >= 19 {
                u16::from(slot)
            } else {
                1000 + u16::from(slot)
            }
        });

        for (_, inventory_item) in slots {
            if inventory_item.entry_id != item_entry {
                continue;
            }
            let Some(item) = self.resolved_inventory_item_object_like_cpp(hub, inventory_item.guid)
            else {
                continue;
            };
            if item.is_in_trade() {
                continue;
            }
            current_count = current_count.saturating_add(item.count());
            if current_count >= count {
                return true;
            }
        }

        false
    }

    pub fn plan_destroy_item_count_direct_inventory(
        &self,
        hub: HubRef<'_>,
        item_entry: u32,
        count: u32,
    ) -> Option<Vec<ExtendedCostItemTurninChange>> {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.plan_destroy_item_count_direct_inventory_with_access_like_cpp(
            &access, item_entry, count,
        )
    }
}
