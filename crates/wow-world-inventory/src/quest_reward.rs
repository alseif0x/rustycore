// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory planning and storage primitives used by the quest-reward owner.

use wow_world_core::session::OwnedInventoryAccessLikeCpp;

use crate::{ExtendedCostItemTurninChange, InventoryState};

pub fn make_inventory_item_object_like_cpp(
    item_guid: wow_core::ObjectGuid,
    entry_id: u32,
    owner_guid: wow_core::ObjectGuid,
    count: u32,
    durability: u32,
    max_durability: u32,
    context: wow_constants::ItemContext,
    slot: u8,
    total_played_time: u32,
) -> wow_entities::Item {
    let mut item = wow_entities::Item::new(i64::from(total_played_time));
    item.initialize_created_state(wow_entities::ItemCreateInfo {
        guid: item_guid,
        item_id: entry_id,
        context,
        owner: Some(owner_guid),
        max_durability,
        expiration: 0,
        spell_charges: [0; wow_entities::MAX_ITEM_SPELLS],
    });
    item.set_count(count.max(1));
    item.set_durability(durability);
    item.set_slot(slot);
    item.set_container_guid(wow_core::ObjectGuid::EMPTY);
    item
}

pub fn item_push_result_from_send_new_item_plan(
    plan: &wow_entities::SendNewItemPlan,
) -> wow_packet::packets::item::ItemPushResult {
    wow_packet::packets::item::ItemPushResult {
        player_guid: plan.player_guid,
        slot: plan.slot,
        slot_in_bag: i32::from(plan.slot_in_bag),
        item: wow_packet::packets::item::ItemInstance {
            item_id: plan.item_instance.item_id as i32,
            random_properties_seed: plan.item_instance.random_properties_seed,
            random_properties_id: plan.item_instance.random_properties_id,
            item_bonus: None,
            modifications: wow_packet::packets::item::ItemModList {
                values: plan
                    .item_instance
                    .modifications
                    .iter()
                    .map(|modifier| {
                        wow_packet::packets::item::ItemMod::new(
                            modifier.value,
                            modifier.modifier_type,
                        )
                    })
                    .collect(),
            },
        },
        quest_log_item_id: plan.quest_log_item_id as i32,
        quantity: plan.quantity as i32,
        quantity_in_inventory: plan.quantity_in_inventory as i32,
        dungeon_encounter_id: plan.dungeon_encounter_id as i32,
        battle_pet_species_id: plan.battle_pet_species_id as i32,
        battle_pet_breed_id: plan.battle_pet_breed_id as i32,
        battle_pet_breed_quality: u32::from(plan.battle_pet_breed_quality),
        battle_pet_level: plan.battle_pet_level as i32,
        item_guid: plan.item_guid,
        pushed: plan.pushed,
        display_text: match plan.display_text {
            wow_entities::SendNewItemDisplayText::Normal => {
                wow_packet::packets::item::ItemPushResultDisplayType::Normal
            }
            wow_entities::SendNewItemDisplayText::EncounterLoot => {
                wow_packet::packets::item::ItemPushResultDisplayType::EncounterLoot
            }
            wow_entities::SendNewItemDisplayText::QuestUpdateAddItem => {
                wow_packet::packets::item::ItemPushResultDisplayType::QuestUpdateAddItem
            }
        },
        created: plan.created,
        is_bonus_roll: false,
        is_encounter_loot: plan.is_encounter_loot,
    }
}

impl InventoryState {
    pub fn quest_reward_inventory_item_from_runtime_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        slot: u8,
    ) -> Option<wow_entities::PlayerInventoryItem> {
        self.resolved_player_inventory_runtime_with_access_like_cpp(access)?
            .inventory_items()
            .get(&slot)
            .cloned()
    }

    pub fn apply_quest_reward_item_object_updates_with_access_like_cpp(
        &mut self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        item_guid: wow_core::ObjectGuid,
        updates: &[wow_entities::ItemObjectUpdateLikeCpp],
    ) -> bool {
        self.mutate_player_inventory_runtime_with_access_like_cpp(access, |inventory| {
            inventory.apply_item_object_updates_like_cpp(item_guid, updates)
        }).unwrap_or(false)
    }

    pub fn insert_quest_reward_inventory_item_with_access_like_cpp(
        &mut self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        slot: u8,
        item: wow_entities::PlayerInventoryItem,
    ) -> Option<wow_entities::PlayerInventoryItem> {
        self.mutate_player_inventory_runtime_with_access_like_cpp(access, |inventory| {
            inventory.store_item_in_slot_like_cpp(slot, item)
        }).flatten()
    }

    pub fn insert_quest_reward_item_object_with_access_like_cpp(
        &mut self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        item: wow_entities::Item,
    ) -> Option<wow_entities::Item> {
        self.mutate_player_inventory_runtime_with_access_like_cpp(access, |inventory| {
            inventory.store_item_object_like_cpp(item)
        }).flatten()
    }

    /// Clone one runtime item through the same full inventory projection used by
    /// CanEquip after its selected slot reread.
    pub fn resolved_player_inventory_item_object_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        guid: wow_core::ObjectGuid,
    ) -> Option<wow_entities::Item> {
        self.resolved_player_inventory_runtime_with_access_like_cpp(access)?
            .item_objects()
            .get(&guid)
            .cloned()
    }

    pub fn get_inventory_item_by_pos_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        item_store: Option<&std::sync::Arc<wow_data::ItemStore>>,
        item_stats_store: Option<&std::sync::Arc<wow_data::ItemStatsStore>>,
        bag: u8,
        slot: u8,
    ) -> Option<wow_entities::PlayerInventoryItem> {
        if bag == wow_entities::INVENTORY_SLOT_BAG_0 {
            if (slot as usize) >= wow_entities::PLAYER_SLOT_END
                || wow_entities::is_buyback_slot(slot)
            {
                return None;
            }
            return self
                .resolved_player_inventory_runtime_with_access_like_cpp(access)?
                .inventory_items()
                .get(&slot)
                .cloned();
        }

        if !crate::is_represented_bag_slot(bag) {
            return None;
        }

        let bag_item = self
            .resolved_player_inventory_runtime_with_access_like_cpp(access)?
            .inventory_items()
            .get(&bag)?
            .clone();
        let bag_guid = bag_item.guid;
        let item_objects = self
            .resolved_player_inventory_runtime_with_access_like_cpp(access)?
            .item_objects()
            .clone();
        let nested = item_objects
            .values()
            .find(|item| item.container_guid() == bag_guid && item.slot() == slot)?;
        let guid = nested.object().guid();
        let entry_id = nested.object().entry();
        let inventory_type = wow_world_core::catalogs::item::item_storage_template_like_cpp(
            item_store,
            item_stats_store,
            entry_id,
        )
        .map(|template| template.inventory_type as u8)
        .filter(|&inventory_type| {
            inventory_type != wow_constants::InventoryType::NonEquip as u8
        });

        Some(wow_entities::PlayerInventoryItem {
            guid,
            entry_id,
            db_guid: guid.counter() as u64,
            inventory_type,
        })
    }

    pub fn represented_inventory_item_counts_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
    ) -> Option<std::collections::HashMap<u32, u32>> {
        let inventory_items = self.resolved_inventory_items_with_access_like_cpp(access)?;
        let item_objects = self.resolved_inventory_item_objects_with_access_like_cpp(access)?;
        Some(
            inventory_items
                .values()
                .filter_map(|inventory_item| item_objects.get(&inventory_item.guid))
                .chain(item_objects.values().filter(|item| {
                    !item.container_guid().is_empty()
                        && item_objects.contains_key(&item.container_guid())
                }))
                .filter(|item| !item.is_in_trade())
                .fold(std::collections::HashMap::new(), |mut counts, item| {
                    let entry_id = item.object().entry();
                    counts
                        .entry(entry_id)
                        .and_modify(|count| *count = count.saturating_add(item.count()))
                        .or_insert(item.count());
                    counts
                }),
        )
    }

    /// Plan C++ quest-item destruction against the shared canonical inventory
    /// projection. `u32::MAX` means remove the represented direct-inventory
    /// count before running the ordinary ordered destruction planner.
    pub fn plan_quest_reward_item_removal_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        item_entry: u32,
        count: u32,
    ) -> Option<Vec<ExtendedCostItemTurninChange>> {
        let effective_count = if count == u32::MAX {
            let inventory_items = self.resolved_inventory_items_with_access_like_cpp(access)?;
            inventory_items
                .values()
                .filter(|item| item.entry_id == item_entry)
                .filter_map(|inventory_item| {
                    self.resolved_player_inventory_runtime_with_access_like_cpp(access)
                        .and_then(|inventory| {
                            inventory
                                .item_objects()
                                .get(&inventory_item.guid)
                                .cloned()
                        })
                        .filter(|item| !item.is_in_trade())
                        .map(|item| item.count())
                })
                .fold(0u32, u32::saturating_add)
        } else {
            count
        };

        if effective_count == 0 {
            return Some(Vec::new());
        }

        self.plan_destroy_item_count_direct_inventory_with_access_like_cpp(
            access,
            item_entry,
            effective_count,
        )
    }

    pub(crate) fn plan_destroy_item_count_direct_inventory_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        item_entry: u32,
        count: u32,
    ) -> Option<Vec<ExtendedCostItemTurninChange>> {
        if count == 0 {
            return Some(Vec::new());
        }

        let inventory_items = self.resolved_inventory_items_with_access_like_cpp(access)?;
        let mut remaining = count;
        let mut changes = Vec::new();
        let mut slots: Vec<_> = inventory_items.iter().collect();
        slots.sort_by_key(|&(slot, _)| {
            let slot = *slot;
            if slot >= 19 {
                u16::from(slot)
            } else {
                1000 + u16::from(slot)
            }
        });

        for (&slot, inventory_item) in slots {
            if inventory_item.entry_id != item_entry {
                continue;
            }
            let Some(inventory) =
                self.resolved_player_inventory_runtime_with_access_like_cpp(access)
            else {
                continue;
            };
            let Some(item) = inventory.item_objects().get(&inventory_item.guid).cloned() else {
                continue;
            };
            if item.is_in_trade() {
                continue;
            }

            let item_count = item.count();
            if item_count <= remaining {
                remaining -= item_count;
                changes.push(ExtendedCostItemTurninChange::Delete {
                    slot,
                    item_guid: inventory_item.guid,
                    db_guid: inventory_item.db_guid,
                });
            } else {
                changes.push(ExtendedCostItemTurninChange::Update {
                    slot,
                    item_guid: inventory_item.guid,
                    db_guid: inventory_item.db_guid,
                    new_count: item_count - remaining,
                });
                remaining = 0;
            }

            if remaining == 0 {
                return Some(changes);
            }
        }

        None
    }
}
