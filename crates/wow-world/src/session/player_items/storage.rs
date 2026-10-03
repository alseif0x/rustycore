//! Represented inventory storage: the complete store, remove and swap operations over the canonical Player item slots.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;
use wow_entities::ItemObjectUpdateLikeCpp;

impl WorldSession {
    pub(crate) fn move_represented_direct_inventory_item_like_cpp(
        &mut self,
        src: u8,
        dst: u8,
    ) -> bool {
        if src == dst {
            return true;
        }

        let src_item = self.resolved_inventory_item_like_cpp(src);
        let dst_item = self.resolved_inventory_item_like_cpp(dst);
        let Some(src_item) = src_item else {
            return false;
        };

        self.insert_inventory_item_like_cpp(dst, src_item.clone());
        let player_guid = self.player_guid().unwrap_or(ObjectGuid::EMPTY);
        let _ = self.apply_inventory_item_object_updates_like_cpp(
            src_item.guid,
            &[
                ItemObjectUpdateLikeCpp::SetContainedIn(player_guid),
                ItemObjectUpdateLikeCpp::SetSlot(dst),
            ],
        );

        if let Some(dst_item) = dst_item {
            self.insert_inventory_item_like_cpp(src, dst_item.clone());
            let _ = self.apply_inventory_item_object_updates_like_cpp(
                dst_item.guid,
                &[
                    ItemObjectUpdateLikeCpp::SetContainedIn(player_guid),
                    ItemObjectUpdateLikeCpp::SetSlot(src),
                ],
            );
        } else {
            self.remove_inventory_item_like_cpp(src);
        }

        true
    }
    pub(crate) fn move_represented_direct_inventory_item_with_item_mods_like_cpp(
        &mut self,
        src: u8,
        dst: u8,
    ) -> Option<bool> {
        if src == dst {
            return Some(false);
        }

        let src_item = self.resolved_inventory_item_like_cpp(src)?;
        let dst_item = self.resolved_inventory_item_like_cpp(dst);
        let mut item_mods_changed = false;

        if src < INVENTORY_SLOT_BAG_END {
            let _ = self.record_direct_inventory_item_set_remove_like_cpp(
                INVENTORY_SLOT_BAG_0,
                src,
                src_item.guid,
            );
        }

        if src < INVENTORY_SLOT_BAG_END
            && self
                .resolved_inventory_item_object_like_cpp(src_item.guid)
                .is_some_and(|item| !item.is_broken())
        {
            self.record_represented_item_mods_like_cpp(src_item.guid, src, false);
            item_mods_changed = true;
        }

        if dst < INVENTORY_SLOT_BAG_END
            && let Some(dst_item) = dst_item.as_ref()
        {
            let _ = self.record_direct_inventory_item_set_remove_like_cpp(
                INVENTORY_SLOT_BAG_0,
                dst,
                dst_item.guid,
            );
        }

        if dst < INVENTORY_SLOT_BAG_END
            && dst_item.as_ref().is_some_and(|item| {
                self.resolved_inventory_item_object_like_cpp(item.guid)
                    .is_some_and(|item_object| !item_object.is_broken())
            })
        {
            let dst_item = dst_item.as_ref().expect("checked Some above");
            self.record_represented_item_mods_like_cpp(dst_item.guid, dst, false);
            item_mods_changed = true;
        }

        if !self.move_represented_direct_inventory_item_like_cpp(src, dst) {
            return None;
        }

        if dst < INVENTORY_SLOT_BAG_END {
            let _ = self.record_represented_items_set_item_like_cpp(src_item.guid, true);
        }

        if dst < INVENTORY_SLOT_BAG_END
            && self
                .resolved_inventory_item_object_like_cpp(src_item.guid)
                .is_some_and(|item| !item.is_broken())
        {
            self.record_represented_item_mods_like_cpp(src_item.guid, dst, true);
            item_mods_changed = true;
        }

        if src < INVENTORY_SLOT_BAG_END
            && let Some(dst_item) = dst_item.as_ref()
        {
            let _ = self.record_represented_items_set_item_like_cpp(dst_item.guid, true);
        }

        if src < INVENTORY_SLOT_BAG_END
            && dst_item.as_ref().is_some_and(|item| {
                self.resolved_inventory_item_object_like_cpp(item.guid)
                    .is_some_and(|item_object| !item_object.is_broken())
            })
        {
            let dst_item = dst_item.as_ref().expect("checked Some above");
            self.record_represented_item_mods_like_cpp(dst_item.guid, src, true);
            item_mods_changed = true;
        }

        Some(item_mods_changed)
    }
    pub(crate) fn record_destroyed_inventory_item_mod_remove_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) -> bool {
        if bag != INVENTORY_SLOT_BAG_0 || slot >= INVENTORY_SLOT_BAG_END {
            return false;
        }
        if self
            .resolved_inventory_item_object_like_cpp(item_guid)
            .is_some_and(|item| item.is_broken())
        {
            return false;
        }

        self.record_represented_item_mods_like_cpp(item_guid, slot, false) != 0
    }
    pub(crate) fn record_direct_inventory_item_set_remove_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) -> bool {
        if bag != INVENTORY_SLOT_BAG_0 || slot >= INVENTORY_SLOT_BAG_END {
            return false;
        }

        self.record_represented_items_set_item_like_cpp(item_guid, false)
    }
    pub(in crate::session) fn represented_item_inventory_type_like_cpp(
        &self,
        item_entry: u32,
        item_guid: ObjectGuid,
    ) -> Option<InventoryType> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_item_inventory_type_like_cpp(hub, item_entry, item_guid)
    }
    pub fn item_template_inventory_type(&self, item_id: u32) -> Option<u8> {
        crate::session::hub_ref(self).item_template_inventory_type(item_id)
    }
    pub(crate) fn insert_inventory_item_object(&mut self, item: Item) -> Option<Item> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.insert_inventory_item_object(&mut hub, item)
    }
    pub(crate) fn apply_inventory_item_object_updates_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        updates: &[ItemObjectUpdateLikeCpp],
    ) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.apply_inventory_item_object_updates_like_cpp(&mut hub, item_guid, updates)
    }
    pub(crate) fn remove_inventory_item_object(&mut self, item_guid: ObjectGuid) -> Option<Item> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.remove_inventory_item_object(&mut hub, item_guid)
    }
    pub(crate) fn clear_inventory_items_and_objects_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.clear_inventory_items_and_objects_like_cpp(&mut hub)
    }
    pub(crate) fn insert_inventory_item_like_cpp(
        &mut self,
        slot: u8,
        item: InventoryItem,
    ) -> Option<InventoryItem> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.insert_inventory_item_like_cpp(&mut hub, slot, item)
    }
    pub(crate) fn send_inventory_item_pending_values_update_like_cpp(&self, item_guid: ObjectGuid) {
        let Some(item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return;
        };
        if let Some(packet) = item_values_update_to_update_object(
            item_guid,
            self.core.player_map_id_like_cpp(),
            &item.values_update(),
        ) {
            self.send_packet(&packet);
        }
    }
    pub(crate) fn remove_inventory_item_like_cpp(&mut self, slot: u8) -> Option<InventoryItem> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.remove_inventory_item_like_cpp(&mut hub, slot)
    }
    pub(crate) fn represented_inventory_item_counts_like_cpp(&self) -> Option<HashMap<u32, u32>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_inventory_item_counts_like_cpp(hub)
    }
    pub(crate) fn get_inventory_item_by_guid_like_cpp(
        &self,
        item_guid: ObjectGuid,
    ) -> Option<(u8, u8, InventoryItem)> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.get_inventory_item_by_guid_like_cpp(hub, item_guid)
    }
    pub(crate) fn can_use_inventory_item_represented_like_cpp(
        &self,
        item: &InventoryItem,
        runtime_item: Option<&Item>,
    ) -> InventoryResult {
        self.can_use_inventory_item_represented_with_loading_like_cpp(item, runtime_item, true)
    }
    pub(in crate::session) fn direct_inventory_player_snapshot(
        &self,
    ) -> Option<wow_world_core::session::InventoryPlayerProjectionLikeCpp> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.direct_inventory_player_snapshot(hub)
    }
    pub(crate) fn remove_inventory_item_duration_refs_like_cpp(&mut self, item_guid: ObjectGuid) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.remove_inventory_item_duration_refs_like_cpp(&mut hub, item_guid)
    }
    pub(crate) fn remove_inventory_tradeable_item_like_cpp(&mut self, item_guid: ObjectGuid) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.remove_inventory_tradeable_item_like_cpp(&mut hub, item_guid)
    }
    pub(crate) fn add_inventory_item_duration_refs_like_cpp(&mut self, item_guid: ObjectGuid) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.add_inventory_item_duration_refs_like_cpp(&mut hub, item_guid)
    }
    /// C++ `Player::ApplyItemObtainSpells(item, true)` after `_StoreItem`.
    pub(crate) async fn apply_inventory_item_obtain_spells_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        item_entry: u32,
    ) -> Vec<i32> {
        if self
            .item_template_flags(item_entry)
            .is_some_and(|flags| flags.contains(ItemFlags::LEGACY))
        {
            return Vec::new();
        }
        let spell_ids = self
            .catalogs
            .items
            .effect_store
            .as_ref()
            .map(|store| {
                store
                    .item_effects_for_item_id_like_cpp(item_entry)
                    .into_iter()
                    .filter(|effect| {
                        effect.trigger_type == ItemSpelltriggerType::OnPickup as i8
                            && effect.spell_id > 0
                    })
                    .map(|effect| effect.spell_id)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let Some(player_guid) = self.player_guid() else {
            return Vec::new();
        };
        let Some(visible_auras) =
            crate::session::hub_ref(self).resolved_player_visible_auras_like_cpp()
        else {
            return Vec::new();
        };
        let mut applied = Vec::new();
        for spell_id in spell_ids {
            if visible_auras.values().any(|aura| aura.spell_id == spell_id) {
                continue;
            }
            // C++ `Player::ApplyItemObtainSpells` uses
            // `CastSpellExtraArgs().SetCastItem(item)`, whose default trigger
            // flags are TRIGGERED_NONE: not a triggered cast, and the global
            // cooldown applies.
            if self
                .execute_server_triggered_spell_like_cpp(
                    item_guid_generator,
                    creature_spawn_catalogs,
                    spell_id,
                    player_guid,
                    SpellCastMetadata::default(),
                )
                .await
                .is_ok()
            {
                applied.push(spell_id);
            }
        }
        applied
    }
    #[cfg(test)]
    pub(crate) async fn apply_inventory_item_obtain_spells_like_cpp(
        &mut self,
        item_entry: u32,
    ) -> Vec<i32> {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.apply_inventory_item_obtain_spells_with_generator_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            item_entry,
        )
        .await
    }
    pub(crate) fn apply_inventory_item_remove_side_effects_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
        cleared_mainhand_enchantments: &[EnchantmentSlot],
    ) -> bool {
        self.remove_inventory_item_duration_refs_like_cpp(item_guid);
        self.remove_inventory_tradeable_item_like_cpp(item_guid);

        if bag != INVENTORY_SLOT_BAG_0 || slot >= INVENTORY_SLOT_BAG_END {
            return false;
        }

        let _ = self.record_direct_inventory_item_set_remove_like_cpp(bag, slot, item_guid);
        let item_mods_changed =
            self.record_destroyed_inventory_item_mod_remove_like_cpp(bag, slot, item_guid);
        let _ = {
            let (s, mut h) = crate::session::split_inventory_mut(self);
            s.clear_inventory_item_equipped_state_like_cpp(
                &mut h,
                item_guid,
                cleared_mainhand_enchantments,
            )
        };

        if slot < PROFESSION_SLOT_END {
            self.inventory
                .record_inventory_item_combat_stat_recalculations_like_cpp(slot);
        }
        item_mods_changed
    }
    /// C++ `StoreItem`/`BankItem`/`EquipItem` post-placement side effects for
    /// a runtime item whose persistence and position have already committed.
    pub(crate) fn apply_inventory_item_store_side_effects_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) -> bool {
        self.add_inventory_item_duration_refs_like_cpp(item_guid);
        if bag != INVENTORY_SLOT_BAG_0 || slot >= INVENTORY_SLOT_BAG_END {
            return false;
        }

        let _ = {
            let (s, mut h) = crate::session::split_inventory_mut(self);
            s.set_inventory_item_equipped_like_cpp(&mut h, item_guid, true)
        };
        let _ = self.record_represented_items_set_item_like_cpp(item_guid, true);
        let item_mods_changed = if self
            .resolved_inventory_item_object_like_cpp(item_guid)
            .is_some_and(|item| !item.is_broken())
        {
            self.record_represented_item_mods_like_cpp(item_guid, slot, true) != 0
        } else {
            false
        };
        if slot < PROFESSION_SLOT_END {
            self.inventory
                .record_inventory_item_combat_stat_recalculations_like_cpp(slot);
        }
        item_mods_changed
    }
    pub(crate) fn resolved_inventory_items_like_cpp(&self) -> Option<HashMap<u8, InventoryItem>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_inventory_items_like_cpp(hub)
    }
    pub(crate) fn resolved_inventory_item_objects_like_cpp(
        &self,
    ) -> Option<HashMap<ObjectGuid, Item>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_inventory_item_objects_like_cpp(hub)
    }
    pub(crate) fn resolved_inventory_item_like_cpp(&self, slot: u8) -> Option<InventoryItem> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_inventory_item_like_cpp(hub, slot)
    }
    pub(crate) fn resolved_inventory_item_object_like_cpp(&self, guid: ObjectGuid) -> Option<Item> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_inventory_item_object_like_cpp(hub, guid)
    }
}

impl crate::session::InventoryCxRef<'_> {
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
        let max_durability = self
            .hub
            .catalogs
            .item_template_max_durability(entry_id)
            .max(durability);
        let mut item = Item::new(i64::from(self.lifecycle.total_played_time_like_cpp()));
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
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/storage/f3_shims.rs"]
mod f3_shims;
