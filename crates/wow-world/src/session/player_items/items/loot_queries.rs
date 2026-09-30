//! loot queries for the existing items owner.

use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_player_has_quest_for_loot_item_like_cpp(
        &self,
        item_id: u32,
    ) -> bool {
        self.represented_current_player_has_incomplete_quest_objective_for_item_like_cpp(item_id)
            || self
                .item_template_addon_quest_log_item_id_like_cpp(item_id)
                .is_some_and(|quest_log_item_id| {
                    quest_log_item_id != 0
                        && self
                            .represented_current_player_has_incomplete_quest_objective_for_item_like_cpp(
                                quest_log_item_id,
                            )
                })
            || self.represented_current_player_has_incomplete_quest_item_drop_for_item_like_cpp(item_id)
    }
    fn represented_current_player_has_incomplete_quest_objective_for_item_like_cpp(
        &self,
        item_id: u32,
    ) -> bool {
        let Ok(item_object_id) = i32::try_from(item_id) else {
            return false;
        };
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        let Some(store) = self.quests.store.as_ref() else {
            return false;
        };
        wow_entities::player_has_incomplete_quest_objective_for_object_id_like_cpp(
            quests.statuses_like_cpp(),
            |id| store.get(id).map(|quest| quest.objective_rules_like_cpp()),
            item_object_id,
        )
    }
    fn represented_current_player_has_incomplete_quest_item_drop_for_item_like_cpp(
        &self,
        item_id: u32,
    ) -> bool {
        let Some(quest_store) = self.quests.store.as_ref() else {
            return false;
        };
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        quests.statuses_like_cpp().values().any(|status| {
            if status.status != wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                return false;
            }
            let Some(quest) = quest_store.get(status.quest_id) else {
                return false;
            };
            quest
                .item_drop
                .iter()
                .enumerate()
                .any(|(index, drop_item_id)| {
                    if *drop_item_id != item_id {
                        return false;
                    }
                    let Some(template) = self.item_storage_template(item_id) else {
                        return false;
                    };
                    let quantity = quest.item_drop_quantity[index];
                    let mut max_allowed_count = if quantity != 0 {
                        quantity
                    } else {
                        template.max_stack_size
                    };
                    if template.max_count > 0 {
                        max_allowed_count = max_allowed_count.min(template.max_count as u32);
                    }
                    self.represented_inventory_item_counts_like_cpp()
                        .is_some_and(|counts| {
                            counts.get(&item_id).copied().unwrap_or(0) < max_allowed_count
                        })
                })
        })
    }
    pub(crate) fn item_drop_rate_like_cpp(&self, item_id: u32) -> f32 {
        let quality = self
            .item_template_quality(item_id)
            .and_then(<ItemQuality as num_traits::FromPrimitive>::from_i8);
        match quality {
            Some(ItemQuality::Poor) => self.loot_drop_rates.item_poor,
            Some(ItemQuality::Normal) => self.loot_drop_rates.item_normal,
            Some(ItemQuality::Uncommon) => self.loot_drop_rates.item_uncommon,
            Some(ItemQuality::Rare) => self.loot_drop_rates.item_rare,
            Some(ItemQuality::Epic) => self.loot_drop_rates.item_epic,
            Some(ItemQuality::Legendary) => self.loot_drop_rates.item_legendary,
            Some(ItemQuality::Artifact) => self.loot_drop_rates.item_artifact,
            _ => 1.0,
        }
    }
    pub(crate) fn item_effect_count_like_cpp(&self, item_entry: u32) -> usize {
        self.items
            .effect_store
            .as_ref()
            .map(|store| {
                store
                    .item_effects_for_item_id_like_cpp(item_entry)
                    .len()
                    .min(MAX_ITEM_SPELLS)
            })
            .unwrap_or(0)
    }
    /// Remove a fully-looted runtime item after its DB rows were deleted.
    pub(crate) fn remove_fully_looted_runtime_item(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) {
        if bag == INVENTORY_SLOT_BAG_0
            && self
                .resolved_inventory_item_like_cpp(slot)
                .is_some_and(|item| item.guid == item_guid)
        {
            self.remove_inventory_item_like_cpp(slot);
        }
        self.remove_inventory_item_object(item_guid);
        self.sync_player_registry_state_like_cpp();
    }
    pub(crate) fn represented_has_item_count_like_cpp(&self, item_entry: u32, count: u32) -> bool {
        if count == 0 {
            return true;
        }

        self.represented_inventory_item_counts_like_cpp()
            .is_some_and(|counts| counts.get(&item_entry).copied().unwrap_or(0) >= count)
    }
    pub(crate) fn can_destroy_direct_item_like_cpp(
        &self,
        slot: u8,
        source_item: Option<&Item>,
        proto: Option<&ItemStorageTemplate>,
        source_is_not_empty_bag: bool,
    ) -> InventoryResult {
        self.can_unequip_inventory_item_at_like_cpp(
            INVENTORY_SLOT_BAG_0,
            slot,
            false,
            source_item,
            proto,
            source_is_not_empty_bag,
        )
    }
    pub(crate) fn direct_item_contains_items(&self, item_guid: ObjectGuid) -> bool {
        self.resolved_inventory_item_objects_like_cpp()
            .is_some_and(|items| {
                items
                    .values()
                    .any(|item| item.container_guid() == item_guid)
            })
    }
    pub(crate) fn has_active_non_item_loot_views_like_cpp(&self) -> bool {
        self.loot_views.has_non_item_views()
    }
}
