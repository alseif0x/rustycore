// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;
use wow_entities::{EQUIPMENT_SLOT_END, Item, PlayerEnchantTimeUpdate, PlayerItemTimeUpdate};
use wow_world_core::session::{HubMut, HubRef};

impl crate::InventoryState {
    pub fn loaded_inventory_item_visible_update_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_guid: ObjectGuid,
    ) -> Option<(u8, i32, u16, u16)> {
        let item = self.resolved_inventory_item_object_like_cpp(hub, item_guid)?;
        let slot = item.slot();
        if slot >= EQUIPMENT_SLOT_END {
            return None;
        }
        let (item_id, appearance_mod_id, item_visual) =
            self.loaded_inventory_item_visible_fields_like_cpp(hub, &item);
        Some((slot, item_id, appearance_mod_id, item_visual))
    }

    /// Restores C++ `Player::_LoadInventory` duration tracking before the login
    /// create packet is sent. Equipped enchantments are registered later by the
    /// ordered `_ApplyAllItemMods` replay, so only non-equipped enchantments are
    /// added here.
    pub fn register_loaded_inventory_item_duration_refs_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        loaded_item_guids: &[ObjectGuid],
        loaded_equipped_item_guids: &[ObjectGuid],
    ) -> (Vec<PlayerItemTimeUpdate>, Vec<PlayerEnchantTimeUpdate>) {
        let mut item_updates = Vec::new();
        let mut enchantment_updates = Vec::new();

        for &item_guid in loaded_item_guids {
            let Some(mut item) =
                self.resolved_inventory_item_object_like_cpp(hub.shared(), item_guid)
            else {
                continue;
            };
            let is_equipped = loaded_equipped_item_guids.contains(&item_guid);
            let Some((item_update, mut item_enchantment_updates)) =
                hub.core.mutate_canonical_player_like_cpp(|player| {
                    let item_update = player.add_item_durations(&item);
                    let enchantment_updates = if is_equipped {
                        Vec::new()
                    } else {
                        player.add_enchantment_durations(&mut item)
                    };
                    (item_update, enchantment_updates)
                })
            else {
                continue;
            };

            self.insert_inventory_item_object(hub, item);
            if let Some(item_update) = item_update {
                item_updates.push(item_update);
            }
            enchantment_updates.append(&mut item_enchantment_updates);
        }

        (item_updates, enchantment_updates)
    }
}

impl crate::InventoryState {
    pub fn loaded_inventory_item_visible_fields_like_cpp(
        &self,
        hub: HubRef<'_>,
        item: &Item,
    ) -> (i32, u16, u16) {
        (
            item.object().entry() as i32,
            0,
            item.visible_item_visual(0, |enchantment_id| {
                hub.catalogs
                    .spell_item_enchantment_store()
                    .and_then(|store| store.get(enchantment_id))
                    .map(|entry| entry.item_visual)
            }),
        )
    }

    /// Begin hydrating the persisted equipment/inventory source for the active
    /// Player. The proof remains incomplete on every non-empty, early-return,
    /// or query-error path; this bounded slice authorizes only a proven-empty
    /// persisted result.
    pub fn begin_player_equipment_inventory_authority_load_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) {
        let _canonical = hub
            .core
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .inventory_runtime_mut_like_cpp()
                    .set_equipment_inventory_authority_complete_like_cpp(false);
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !_canonical && hub.core.player_handle_like_cpp.is_none() {
            self.player_equipment_inventory_authority_complete_like_cpp = false;
        }
        hub.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }

    pub fn complete_player_equipment_inventory_authority_load_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) {
        let _canonical = hub
            .core
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .inventory_runtime_mut_like_cpp()
                    .set_equipment_inventory_authority_complete_like_cpp(true);
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !_canonical && hub.core.player_handle_like_cpp.is_none() {
            self.player_equipment_inventory_authority_complete_like_cpp = true;
        }
    }

    pub fn player_equipment_inventory_authority_complete_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_like_cpp(|player| {
                player
                    .inventory_runtime_like_cpp()
                    .equipment_inventory_authority_complete_like_cpp()
            })
            .unwrap_or(false);
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            return self.player_equipment_inventory_authority_complete_like_cpp;
        }
        canonical
    }
}
