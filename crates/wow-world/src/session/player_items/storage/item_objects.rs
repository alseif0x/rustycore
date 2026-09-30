//! item objects for the existing storage owner.

use super::*;

impl WorldSession {
    /// Resolve C++ `ItemTemplate::GetInventoryType()` for equipment-slot mapping.
    pub fn item_template_inventory_type(&self, item_id: u32) -> Option<u8> {
        self.item_storage_template(item_id)
            .map(|template| template.inventory_type as u8)
            .filter(|&inventory_type| inventory_type != InventoryType::NonEquip as u8)
    }
    pub(crate) fn make_inventory_item_object(
        &self,
        item_guid: ObjectGuid,
        entry_id: u32,
        owner_guid: ObjectGuid,
        count: u32,
        durability: u32,
        context: ItemContext,
        slot: u8,
    ) -> Item {
        let max_durability = self.item_template_max_durability(entry_id).max(durability);
        let mut item = Item::new(i64::from(self.lifecycle.total_played_time));
        item.initialize_created_state(ItemCreateInfo {
            guid: item_guid,
            item_id: entry_id,
            context,
            owner: Some(owner_guid),
            max_durability,
            expiration: 0,
            spell_charges: [0; MAX_ITEM_SPELLS],
        });
        item.set_count(count.max(1));
        item.set_durability(durability);
        item.set_slot(slot);
        item.set_container_guid(ObjectGuid::EMPTY);
        item
    }
    pub(crate) fn insert_inventory_item_object(&mut self, item: Item) -> Option<Item> {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.store_item_object_like_cpp(item)
        })
        .flatten()
    }
    pub(crate) fn apply_inventory_item_object_updates_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        updates: &[ItemObjectUpdateLikeCpp],
    ) -> bool {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.apply_item_object_updates_like_cpp(item_guid, updates)
        })
        .unwrap_or(false)
    }
    pub(crate) fn transform_inventory_wrapped_gift_item_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        entry: u32,
        flags: u32,
        max_durability: u32,
    ) -> Option<u32> {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.transform_wrapped_gift_item_like_cpp(item_guid, entry, flags, max_durability)
        })
        .flatten()
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn update_inventory_item_object_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        update: impl FnOnce(&mut Item),
    ) -> bool {
        let mut update = Some(update);
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            let Some(item) = inventory.item_objects_mut().get_mut(&item_guid) else {
                return false;
            };
            if let Some(update) = update.take() {
                update(item);
            }
            true
        })
        .unwrap_or(false)
    }
    pub(crate) fn restore_inventory_item_enchantment_durations_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        durations: &[wow_entities::PlayerEnchantDuration],
    ) -> bool {
        let updates = durations
            .iter()
            .map(
                |duration| wow_entities::ItemObjectUpdateLikeCpp::SetEnchantmentDuration {
                    slot: duration.slot,
                    duration: duration.left_duration_ms,
                },
            )
            .collect::<Vec<_>>();
        self.apply_inventory_item_object_updates_like_cpp(item_guid, &updates)
    }
    pub(crate) fn clear_inventory_item_equipped_state_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        cleared_enchantments: &[EnchantmentSlot],
    ) -> bool {
        let mut updates = vec![ItemObjectUpdateLikeCpp::SetEquipped(false)];
        updates.extend(
            cleared_enchantments
                .iter()
                .copied()
                .map(ItemObjectUpdateLikeCpp::ClearEnchantment),
        );
        self.apply_inventory_item_object_updates_like_cpp(item_guid, &updates)
    }
    pub(crate) fn set_inventory_item_equipped_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        equipped: bool,
    ) -> bool {
        self.apply_inventory_item_object_updates_like_cpp(
            item_guid,
            &[ItemObjectUpdateLikeCpp::SetEquipped(equipped)],
        )
    }
    pub(crate) fn remove_inventory_item_object(&mut self, item_guid: ObjectGuid) -> Option<Item> {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.remove_item_object_like_cpp(item_guid)
        })
        .flatten()
    }
    pub(crate) fn clear_inventory_items_and_objects_like_cpp(&mut self) {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.clear_items_and_objects_like_cpp();
        });
    }
    pub(crate) fn clear_all_inventory_runtime_like_cpp(&mut self) {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            *inventory = PlayerInventoryRuntime::default();
        });
        self.reset_represented_item_bonus_runtime_like_cpp();
    }
    pub(crate) fn insert_inventory_item_like_cpp(
        &mut self,
        slot: u8,
        item: InventoryItem,
    ) -> Option<InventoryItem> {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.store_item_in_slot_like_cpp(slot, item)
        })
        .flatten()
    }
    pub(crate) fn send_inventory_item_pending_values_update_like_cpp(&self, item_guid: ObjectGuid) {
        let Some(item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return;
        };
        if let Some(packet) = item_values_update_to_update_object(
            item_guid,
            self.player_map_id_like_cpp(),
            &item.values_update(),
        ) {
            self.send_packet(&packet);
        }
    }
    pub(crate) fn remove_inventory_item_like_cpp(&mut self, slot: u8) -> Option<InventoryItem> {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.remove_item_from_slot_like_cpp(slot)
        })
        .flatten()
    }
    pub(crate) fn update_inventory_item_metadata_like_cpp(
        &mut self,
        slot: u8,
        item_guid: ObjectGuid,
        entry_id: u32,
        inventory_type: Option<u8>,
    ) -> bool {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.update_slot_item_metadata_like_cpp(slot, item_guid, entry_id, inventory_type)
        })
        .unwrap_or(false)
    }
    pub(crate) fn represented_inventory_item_counts_like_cpp(&self) -> Option<HashMap<u32, u32>> {
        let inventory_items = self.resolved_inventory_items_like_cpp()?;
        let item_objects = self.resolved_inventory_item_objects_like_cpp()?;
        Some(
            inventory_items
                .values()
                .filter_map(|inventory_item| item_objects.get(&inventory_item.guid))
                .chain(item_objects.values().filter(|item| {
                    !item.container_guid().is_empty()
                        && item_objects.contains_key(&item.container_guid())
                }))
                .filter(|item| !item.is_in_trade())
                .fold(HashMap::new(), |mut counts, item| {
                    let entry_id = item.object().entry();
                    counts
                        .entry(entry_id)
                        .and_modify(|count| *count = count.saturating_add(item.count()))
                        .or_insert(item.count());
                    counts
                }),
        )
    }
    /// Resolve an inventory item by GUID following C++ `Player::GetItemByGuid`.
    ///
    /// C++ iterates direct inventory and represented bags. Rust still models
    /// nested bag contents through runtime `Item` objects, so this returns the
    /// effective `(bag, slot, item)` tuple needed by `DestroyItem`.
    pub(crate) fn get_inventory_item_by_guid_like_cpp(
        &self,
        item_guid: ObjectGuid,
    ) -> Option<(u8, u8, InventoryItem)> {
        if item_guid.is_empty() {
            return None;
        }

        if let Some((&slot, item)) = self
            .resolved_inventory_items_like_cpp()?
            .iter()
            .find(|(_, item)| item.guid == item_guid)
        {
            if (slot as usize) < PLAYER_SLOT_END && !wow_entities::is_buyback_slot(slot) {
                return Some((INVENTORY_SLOT_BAG_0, slot, item.clone()));
            }
        }

        let runtime_item = self.resolved_inventory_item_object_like_cpp(item_guid)?;
        if !runtime_item.is_in_bag() {
            return None;
        }

        let bag = runtime_item.bag_slot();
        let slot = runtime_item.slot();
        self.get_inventory_item_by_pos(bag, slot)
            .filter(|item| item.guid == item_guid)
            .map(|item| (bag, slot, item))
    }
    pub(crate) fn can_use_inventory_item_represented_like_cpp(
        &self,
        item: &InventoryItem,
        runtime_item: Option<&Item>,
    ) -> InventoryResult {
        self.can_use_inventory_item_represented_with_loading_like_cpp(item, runtime_item, true)
    }
}
