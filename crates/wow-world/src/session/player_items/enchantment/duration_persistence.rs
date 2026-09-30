//! duration persistence for the existing enchantment owner.

use super::*;

impl WorldSession {
    pub fn send_item_enchant_time_update_plan(
        &self,
        owner_guid: ObjectGuid,
        update: &PlayerEnchantTimeUpdate,
    ) {
        self.send_packet(&ItemEnchantTimeUpdate {
            owner_guid,
            item_guid: update.item_guid,
            duration_left: update.duration_secs,
            slot: update.slot as u32,
        });
    }
    pub fn send_item_enchant_time_update_plans(
        &self,
        owner_guid: ObjectGuid,
        updates: &[PlayerEnchantTimeUpdate],
    ) {
        for update in updates {
            self.send_item_enchant_time_update_plan(owner_guid, update);
        }
    }
    /// C++ `_StoreItem` merge branch calls `AddEnchantmentDurations(pItem2)`
    /// without calling `AddItemDurations` for the existing destination stack.
    pub(crate) fn refresh_inventory_item_enchantment_duration_refs_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
    ) {
        let Some(mut item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return;
        };
        let Some((owner_guid, enchantment_updates)) =
            self.mutate_canonical_player_like_cpp(|player| {
                (player.guid(), player.add_enchantment_durations(&mut item))
            })
        else {
            return;
        };

        self.insert_inventory_item_object(item);
        self.send_item_enchant_time_update_plans(owner_guid, &enchantment_updates);
    }
    /// C++ `Player::RemoveItem` clears main-hand-only enchantments when the
    /// main-hand item leaves that slot. Return both the post-remove DB value
    /// and the runtime slots to clear, without mutating live state before the
    /// caller's transaction commits.
    pub(crate) fn inventory_remove_enchantment_persistence_like_cpp(
        &self,
        item_guid: ObjectGuid,
        clear_mainhand_only: bool,
    ) -> Option<(String, Vec<EnchantmentSlot>)> {
        let item = self.resolved_inventory_item_object_like_cpp(item_guid)?;
        let current_durations = self
            .canonical_player_snapshot_like_cpp(|player| {
                player
                    .enchant_durations()
                    .iter()
                    .filter(|duration| duration.item_guid == item_guid)
                    .copied()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut cleared = Vec::new();
        let mut persisted = String::new();

        for (index, enchantment) in item.data().enchantments.iter().enumerate() {
            let Some(slot) = <EnchantmentSlot as num_traits::FromPrimitive>::from_usize(index)
            else {
                continue;
            };
            let enchantment_entry = u32::try_from(enchantment.id)
                .ok()
                .and_then(|id| {
                    self.spell_catalogs
                        .spell_item_enchantment_store
                        .as_ref()?
                        .get(id)
                })
                .copied();
            let clear_mainhand = clear_mainhand_only
                && enchantment_entry.is_some_and(|entry| {
                    entry
                        .flags
                        .contains(SpellItemEnchantmentFlags::MAINHAND_ONLY)
                });
            if clear_mainhand {
                cleared.push(slot);
            }
            if clear_mainhand
                || enchantment_entry.is_none_or(|entry| {
                    entry
                        .flags
                        .contains(SpellItemEnchantmentFlags::DO_NOT_SAVE_TO_DB)
                })
            {
                persisted.push_str("0 0 0 ");
            } else {
                let duration = current_durations
                    .iter()
                    .find(|duration| duration.slot == slot)
                    .map_or(enchantment.duration, |duration| duration.left_duration_ms);
                persisted.push_str(&format!(
                    "{} {} {} ",
                    enchantment.id, duration, enchantment.charges
                ));
            }
        }

        Some((persisted, cleared))
    }
    pub(crate) fn send_loaded_equipped_item_enchantment_updates_like_cpp(
        &self,
        outcome: &LoadedEquippedItemEnchantmentsOutcomeLikeCpp,
    ) {
        if let Some(owner_guid) = self.player_guid() {
            self.send_item_enchant_time_update_plans(owner_guid, &outcome.duration_updates);
        }
        if !outcome.visible_item_changes.is_empty() {
            self.send_player_values_update_from_entity_bridge(
                &[],
                &outcome.visible_item_changes,
                &[],
                &[],
                None,
            );
        }
    }
}
