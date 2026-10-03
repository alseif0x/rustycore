// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;
use wow_entities::{
    ItemObjectUpdateLikeCpp, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END,
    INVENTORY_SLOT_ITEM_END, INVENTORY_SLOT_ITEM_START,
};
use wow_world_core::session::{
    OwnedInventoryAccessLikeCpp, OwnedItemModifiersAccessLikeCpp, OwnedItemSetAccessLikeCpp,
};

use crate::{InventoryState, ItemModsCatalogsViewLikeCpp};

impl InventoryState {
    pub fn represented_direct_inventory_slot_by_guid_for_equipment_set_like_cpp(
        &self,
        inventory_access: &OwnedInventoryAccessLikeCpp<'_>,
        item_store: Option<&std::sync::Arc<wow_data::ItemStore>>,
        item_stats_store: Option<&std::sync::Arc<wow_data::ItemStatsStore>>,
        guid: ObjectGuid,
    ) -> Option<(u8, wow_entities::PlayerInventoryItem)> {
        let (bag, slot, item) = self.get_inventory_item_by_guid_with_access_like_cpp(
            inventory_access,
            item_store,
            item_stats_store,
            guid,
        )?;
        (bag == INVENTORY_SLOT_BAG_0).then_some((slot, item))
    }

    pub fn equipment_set_inventory_item_by_pos_like_cpp(
        &self,
        inventory_access: &OwnedInventoryAccessLikeCpp<'_>,
        item_store: Option<&std::sync::Arc<wow_data::ItemStore>>,
        item_stats_store: Option<&std::sync::Arc<wow_data::ItemStatsStore>>,
        bag: u8,
        slot: u8,
    ) -> Option<wow_entities::PlayerInventoryItem> {
        self.get_inventory_item_by_pos_with_access_like_cpp(
            inventory_access,
            item_store,
            item_stats_store,
            bag,
            slot,
        )
    }

    pub fn find_free_backpack_slot_for_equipment_set_like_cpp(
        &self,
        inventory_access: &OwnedInventoryAccessLikeCpp<'_>,
    ) -> Option<u8> {
        let inventory_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(
                self.resolved_player_inventory_slot_count_with_access_like_cpp(inventory_access)?,
            )
            .min(INVENTORY_SLOT_ITEM_END);
        let inventory_items = self.resolved_inventory_items_with_access_like_cpp(inventory_access)?;
        (INVENTORY_SLOT_ITEM_START..inventory_end).find(|slot| !inventory_items.contains_key(slot))
    }

    pub fn represented_item_bonus_state_for_equipment_set_use_like_cpp(
        &self,
        modifier_access: &OwnedItemModifiersAccessLikeCpp<'_>,
    ) -> Option<wow_entities::PlayerItemBonusStateLikeCpp> {
        self.player_item_modifier_runtime_snapshot_with_access_like_cpp(modifier_access)
            .map(|runtime| runtime.bonuses_snapshot_like_cpp())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn move_direct_inventory_item_with_item_mods_for_equipment_set_like_cpp(
        &mut self,
        inventory_access: &OwnedInventoryAccessLikeCpp<'_>,
        modifier_access: &OwnedItemModifiersAccessLikeCpp<'_>,
        item_sets: &OwnedItemSetAccessLikeCpp<'_>,
        item_store: Option<&std::sync::Arc<wow_data::ItemStore>>,
        item_stats_store: Option<&std::sync::Arc<wow_data::ItemStatsStore>>,
        scaling_stat_distribution_store: Option<&std::sync::Arc<wow_data::ScalingStatDistributionStore>>,
        scaling_stat_values_store: Option<&std::sync::Arc<wow_data::ScalingStatValuesStore>>,
        shield_block_regular_game_table:
            Option<&std::sync::Arc<wow_data::ShieldBlockRegularGameTableLikeCpp>>,
        spell_shapeshift_form_store: Option<&std::sync::Arc<wow_data::SpellShapeshiftFormStore>>,
        src: u8,
        dst: u8,
        #[cfg(any(test, feature = "test-fixtures"))] player_level_fixture: &u8,
        #[cfg(any(test, feature = "test-fixtures"))] shapeshift_form_fixture: &u32,
        record_test_evidence: bool,
    ) -> Option<bool> {
        if src == dst {
            return Some(false);
        }

        let src_item = self.resolved_inventory_item_with_access_like_cpp(inventory_access, src)?;
        let dst_item = self.resolved_inventory_item_with_access_like_cpp(inventory_access, dst);
        let mut item_mods_changed = false;

        if src < INVENTORY_SLOT_BAG_END {
            let _ = self.record_represented_items_set_item_like_cpp(
                inventory_access,
                modifier_access,
                item_sets,
                src_item.guid,
                false,
                record_test_evidence,
            );
        }

        if src < INVENTORY_SLOT_BAG_END
            && self
                .resolved_inventory_item_object_with_access_like_cpp(inventory_access, src_item.guid)
                .is_some_and(|item| !item.is_broken())
        {
            self.record_item_mods_for_equipment_set_like_cpp(
                inventory_access,
                modifier_access,
                item_store,
                item_stats_store,
                scaling_stat_distribution_store,
                scaling_stat_values_store,
                shield_block_regular_game_table,
                spell_shapeshift_form_store,
                src_item.guid,
                src,
                false,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_level_fixture,
                #[cfg(any(test, feature = "test-fixtures"))]
                shapeshift_form_fixture,
                record_test_evidence,
            );
            item_mods_changed = true;
        }

        if dst < INVENTORY_SLOT_BAG_END
            && let Some(dst_item) = dst_item.as_ref()
        {
            let _ = self.record_represented_items_set_item_like_cpp(
                inventory_access,
                modifier_access,
                item_sets,
                dst_item.guid,
                false,
                record_test_evidence,
            );
        }

        if dst < INVENTORY_SLOT_BAG_END
            && dst_item.as_ref().is_some_and(|item| {
                self.resolved_inventory_item_object_with_access_like_cpp(
                    inventory_access,
                    item.guid,
                )
                .is_some_and(|item_object| !item_object.is_broken())
            })
        {
            let dst_item = dst_item.as_ref().expect("checked Some above");
            self.record_item_mods_for_equipment_set_like_cpp(
                inventory_access,
                modifier_access,
                item_store,
                item_stats_store,
                scaling_stat_distribution_store,
                scaling_stat_values_store,
                shield_block_regular_game_table,
                spell_shapeshift_form_store,
                dst_item.guid,
                dst,
                false,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_level_fixture,
                #[cfg(any(test, feature = "test-fixtures"))]
                shapeshift_form_fixture,
                record_test_evidence,
            );
            item_mods_changed = true;
        }

        if !self.move_direct_inventory_item_with_access_like_cpp(inventory_access, src, dst) {
            return None;
        }

        if dst < INVENTORY_SLOT_BAG_END {
            let _ = self.record_represented_items_set_item_like_cpp(
                inventory_access,
                modifier_access,
                item_sets,
                src_item.guid,
                true,
                record_test_evidence,
            );
        }

        if dst < INVENTORY_SLOT_BAG_END
            && self
                .resolved_inventory_item_object_with_access_like_cpp(inventory_access, src_item.guid)
                .is_some_and(|item| !item.is_broken())
        {
            self.record_item_mods_for_equipment_set_like_cpp(
                inventory_access,
                modifier_access,
                item_store,
                item_stats_store,
                scaling_stat_distribution_store,
                scaling_stat_values_store,
                shield_block_regular_game_table,
                spell_shapeshift_form_store,
                src_item.guid,
                dst,
                true,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_level_fixture,
                #[cfg(any(test, feature = "test-fixtures"))]
                shapeshift_form_fixture,
                record_test_evidence,
            );
            item_mods_changed = true;
        }

        if src < INVENTORY_SLOT_BAG_END
            && let Some(dst_item) = dst_item.as_ref()
        {
            let _ = self.record_represented_items_set_item_like_cpp(
                inventory_access,
                modifier_access,
                item_sets,
                dst_item.guid,
                true,
                record_test_evidence,
            );
        }

        if src < INVENTORY_SLOT_BAG_END
            && dst_item.as_ref().is_some_and(|item| {
                self.resolved_inventory_item_object_with_access_like_cpp(
                    inventory_access,
                    item.guid,
                )
                .is_some_and(|item_object| !item_object.is_broken())
            })
        {
            let dst_item = dst_item.as_ref().expect("checked Some above");
            self.record_item_mods_for_equipment_set_like_cpp(
                inventory_access,
                modifier_access,
                item_store,
                item_stats_store,
                scaling_stat_distribution_store,
                scaling_stat_values_store,
                shield_block_regular_game_table,
                spell_shapeshift_form_store,
                dst_item.guid,
                src,
                true,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_level_fixture,
                #[cfg(any(test, feature = "test-fixtures"))]
                shapeshift_form_fixture,
                record_test_evidence,
            );
            item_mods_changed = true;
        }

        Some(item_mods_changed)
    }

    #[allow(clippy::too_many_arguments)]
    fn record_item_mods_for_equipment_set_like_cpp(
        &mut self,
        inventory_access: &OwnedInventoryAccessLikeCpp<'_>,
        modifier_access: &OwnedItemModifiersAccessLikeCpp<'_>,
        item_store: Option<&std::sync::Arc<wow_data::ItemStore>>,
        item_stats_store: Option<&std::sync::Arc<wow_data::ItemStatsStore>>,
        scaling_stat_distribution_store: Option<&std::sync::Arc<wow_data::ScalingStatDistributionStore>>,
        scaling_stat_values_store: Option<&std::sync::Arc<wow_data::ScalingStatValuesStore>>,
        shield_block_regular_game_table:
            Option<&std::sync::Arc<wow_data::ShieldBlockRegularGameTableLikeCpp>>,
        spell_shapeshift_form_store: Option<&std::sync::Arc<wow_data::SpellShapeshiftFormStore>>,
        guid: ObjectGuid,
        slot: u8,
        apply: bool,
        #[cfg(any(test, feature = "test-fixtures"))] player_level_fixture: &u8,
        #[cfg(any(test, feature = "test-fixtures"))] shapeshift_form_fixture: &u32,
        record_test_evidence: bool,
    ) {
        self.record_represented_item_mods_with_access_like_cpp(
            inventory_access,
            modifier_access,
            ItemModsCatalogsViewLikeCpp::new(
                item_store,
                item_stats_store,
                scaling_stat_distribution_store,
                scaling_stat_values_store,
                shield_block_regular_game_table,
                spell_shapeshift_form_store,
            ),
            guid,
            slot,
            apply,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_level_fixture,
            #[cfg(any(test, feature = "test-fixtures"))]
            shapeshift_form_fixture,
            record_test_evidence,
        );
    }

    pub fn move_direct_inventory_item_with_access_like_cpp(
        &mut self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        src: u8,
        dst: u8,
    ) -> bool {
        if src == dst {
            return true;
        }

        let src_item = self.resolved_inventory_item_with_access_like_cpp(access, src);
        let dst_item = self.resolved_inventory_item_with_access_like_cpp(access, dst);
        let Some(src_item) = src_item else {
            return false;
        };

        let _ = self.mutate_player_inventory_runtime_with_access_like_cpp(access, |inventory| {
            inventory.store_item_in_slot_like_cpp(dst, src_item.clone());
        });
        let player_guid = access.player_guid_like_cpp().unwrap_or(ObjectGuid::EMPTY);
        let _ = self.apply_inventory_item_object_updates_with_access_like_cpp(
            access,
            src_item.guid,
            &[
                ItemObjectUpdateLikeCpp::SetContainedIn(player_guid),
                ItemObjectUpdateLikeCpp::SetSlot(dst),
            ],
        );

        if let Some(dst_item) = dst_item {
            let _ = self.mutate_player_inventory_runtime_with_access_like_cpp(access, |inventory| {
                inventory.store_item_in_slot_like_cpp(src, dst_item.clone());
            });
            let _ = self.apply_inventory_item_object_updates_with_access_like_cpp(
                access,
                dst_item.guid,
                &[
                    ItemObjectUpdateLikeCpp::SetContainedIn(player_guid),
                    ItemObjectUpdateLikeCpp::SetSlot(src),
                ],
            );
        } else {
            let _ = self.mutate_player_inventory_runtime_with_access_like_cpp(access, |inventory| {
                inventory.remove_item_from_slot_like_cpp(src);
            });
        }

        true
    }

    fn apply_inventory_item_object_updates_with_access_like_cpp(
        &mut self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        item_guid: ObjectGuid,
        updates: &[ItemObjectUpdateLikeCpp],
    ) -> bool {
        self.mutate_player_inventory_runtime_with_access_like_cpp(access, |inventory| {
            inventory.apply_item_object_updates_like_cpp(item_guid, updates)
        })
        .unwrap_or(false)
    }
}
