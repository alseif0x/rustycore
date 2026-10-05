// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::sync::Arc;
use wow_core::ObjectGuid;
use wow_data::{ItemStatsStore, ItemStore};
use wow_entities::{INVENTORY_SLOT_BAG_0, PlayerInventoryItem};
use wow_world_core::session::{
    OwnedInventoryAccessLikeCpp, PacketPublicationAccessLikeCpp, PlayerStatsAccessLikeCpp,
};
use wow_world_inventory::InventoryState;

pub struct InventoryPositionPublicationCxLikeCpp<'a> {
    inventory: &'a InventoryState,
    access: OwnedInventoryAccessLikeCpp<'a>,
    publication: PacketPublicationAccessLikeCpp<'a>,
    item_store: Option<&'a Arc<ItemStore>>,
    item_stats_store: Option<&'a Arc<ItemStatsStore>>,
    stats_player: PlayerStatsAccessLikeCpp<'a>,
}

impl<'a> InventoryPositionPublicationCxLikeCpp<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        inventory: &'a InventoryState,
        access: OwnedInventoryAccessLikeCpp<'a>,
        publication: PacketPublicationAccessLikeCpp<'a>,
        item_store: Option<&'a Arc<ItemStore>>,
        item_stats_store: Option<&'a Arc<ItemStatsStore>>,
        stats_player: PlayerStatsAccessLikeCpp<'a>,
    ) -> Self {
        Self {
            inventory,
            access,
            publication,
            item_store,
            item_stats_store,
            stats_player,
        }
    }

    fn get_inventory_item_by_pos(&self, bag: u8, slot: u8) -> Option<PlayerInventoryItem> {
        self.inventory
            .get_inventory_item_by_pos_with_access_like_cpp(
                &self.access,
                self.item_store,
                self.item_stats_store,
                bag,
                slot,
            )
    }

    fn send_bag_slot_values_update_like_cpp(&self, bag: u8, slot: u8) {
        self.inventory
            .send_bag_slot_values_update_with_access_like_cpp(
                &self.access,
                &self.publication,
                bag,
                slot,
            );
    }

    fn send_player_values_update_from_entity_bridge(
        &self,
        top: &[(u8, ObjectGuid)],
        visible: &[(u8, i32, u16, u16)],
        virtual_items: &[(u8, i32, u16, u16)],
        buyback: &[(u8, u32, i64)],
        coinage: Option<u64>,
    ) {
        let _ = self
            .inventory
            .send_player_values_update_from_entity_bridge_with_access_like_cpp(
                &self.access,
                &self.publication,
                self.item_store,
                self.item_stats_store,
                top,
                visible,
                virtual_items,
                buyback,
                coinage,
            );
    }

    fn send_stat_update(&mut self) {
        let player = self.stats_player.reborrow_like_cpp();
        let _ = crate::stats::CharacterStatsApplicationCxLikeCpp::new(
            player,
            self.inventory,
            self.publication.reborrow_like_cpp(),
        )
        .send_stat_update_like_cpp();
    }

    pub fn publish_inventory_position_changes_like_cpp(&mut self, positions: &[(u8, u8)]) {
        let mut unique_positions = positions.to_vec();
        unique_positions.sort_unstable();
        unique_positions.dedup();
        let mut top_level_changes = Vec::new();
        let mut visible_item_changes = Vec::new();
        let mut virtual_item_changes = Vec::new();
        let mut gear_changed = false;

        for (bag, slot) in unique_positions {
            if bag != INVENTORY_SLOT_BAG_0 {
                self.send_bag_slot_values_update_like_cpp(bag, slot);
                continue;
            }
            let item = self.get_inventory_item_by_pos(bag, slot);
            top_level_changes.push((
                slot,
                item.as_ref().map_or(ObjectGuid::EMPTY, |item| item.guid),
            ));
            if slot < 19 {
                gear_changed = true;
                visible_item_changes.push((
                    slot,
                    item.as_ref().map_or(0, |item| item.entry_id as i32),
                    0,
                    0,
                ));
            }
            if (15..=17).contains(&slot) {
                virtual_item_changes.push((
                    slot - 15,
                    item.as_ref().map_or(0, |item| item.entry_id as i32),
                    0,
                    0,
                ));
            }
        }
        if !top_level_changes.is_empty() {
            self.send_player_values_update_from_entity_bridge(
                &top_level_changes,
                &visible_item_changes,
                &virtual_item_changes,
                &[],
                None,
            );
        }
        if gear_changed {
            self.send_stat_update();
        }
    }
}
