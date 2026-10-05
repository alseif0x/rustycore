// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_entities::{INVENTORY_SLOT_BAG_0, PlayerInventoryRuntime};

use crate::session::{InventoryPlayerProjectionLikeCpp, SessionCore};

mod enchantment;
mod equip;
mod relocation;

/// Borrowed access to the session's canonical inventory owner.
pub struct OwnedInventoryAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    /// Build a borrowed capability for the canonical inventory owner.
    pub fn owned_inventory_access_like_cpp(&self) -> OwnedInventoryAccessLikeCpp<'_> {
        OwnedInventoryAccessLikeCpp { core: self }
    }
}

impl OwnedInventoryAccessLikeCpp<'_> {
    pub fn canonical_inventory_bag_slots_snapshot_like_cpp(
        &self,
        bag_slot: u8,
    ) -> Option<(
        wow_core::ObjectGuid,
        u8,
        [wow_core::ObjectGuid; wow_entities::MAX_BAG_SIZE],
    )> {
        self.core
            .canonical_player_snapshot_like_cpp(|player| {
                let bag = player
                    .inventory()
                    .bags
                    .get(bag_slot as usize)
                    .and_then(Option::as_ref)?;
                let mut slots = [wow_core::ObjectGuid::EMPTY; wow_entities::MAX_BAG_SIZE];
                for (index, slot) in bag.slots.iter().enumerate() {
                    slots[index] = slot.unwrap_or(wow_core::ObjectGuid::EMPTY);
                }
                Some((bag.bag_guid, bag.bag_size, slots))
            })
            .flatten()
    }

    /// Final native placement phase after the independent runtime writes.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_committed_inventory_swap_native_placement_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        source_guid: wow_core::ObjectGuid,
        destination_guid: wow_core::ObjectGuid,
        source_bag_size: Option<u8>,
        destination_bag_size: Option<u8>,
        source_children: &[(u8, wow_core::ObjectGuid)],
        destination_children: &[(u8, wow_core::ObjectGuid)],
    ) {
        let _ = self.core.mutate_canonical_player_like_cpp(|player| {
            if source_bag == INVENTORY_SLOT_BAG_0 {
                let _ = player.remove_top_level_item(source_slot);
            } else {
                let _ = player.remove_bag_item(source_bag, source_slot);
            }
            if destination_bag == INVENTORY_SLOT_BAG_0 {
                let _ = player.remove_top_level_item(destination_slot);
            } else {
                let _ = player.remove_bag_item(destination_bag, destination_slot);
            }

            if destination_bag == INVENTORY_SLOT_BAG_0 {
                let _ = player.store_top_level_item(destination_slot, source_guid);
                if wow_entities::is_bag_pos(wow_entities::make_item_pos(
                    INVENTORY_SLOT_BAG_0,
                    destination_slot,
                )) && let Some(size) = source_bag_size
                    && player
                        .register_bag_storage(destination_slot, source_guid, size)
                        .is_ok()
                {
                    for &(slot, guid) in source_children {
                        let _ = player.store_bag_item(destination_slot, slot, guid);
                    }
                }
            } else {
                let _ = player.store_bag_item(destination_bag, destination_slot, source_guid);
            }

            if source_bag == INVENTORY_SLOT_BAG_0 {
                let _ = player.store_top_level_item(source_slot, destination_guid);
                if wow_entities::is_bag_pos(wow_entities::make_item_pos(
                    INVENTORY_SLOT_BAG_0,
                    source_slot,
                )) && let Some(size) = destination_bag_size
                    && player
                        .register_bag_storage(source_slot, destination_guid, size)
                        .is_ok()
                {
                    for &(slot, guid) in destination_children {
                        let _ = player.store_bag_item(source_slot, slot, guid);
                    }
                }
            } else {
                let _ = player.store_bag_item(source_bag, source_slot, destination_guid);
            }
        });
    }
    pub fn remove_item_duration_refs_like_cpp(
        &self,
        item: &mut wow_entities::Item,
    ) -> Option<Vec<wow_entities::PlayerEnchantDuration>> {
        self.core.mutate_canonical_player_like_cpp(|player| {
            let removed = player.remove_enchantment_durations(item);
            let _ = player.remove_item_durations(item);
            removed
        })
    }

    pub fn remove_tradeable_item_like_cpp(&self, item: &wow_entities::Item) {
        let _ = self.core.mutate_canonical_player_like_cpp(|player| {
            player.remove_tradeable_item(item);
        });
    }

    pub fn add_item_duration_refs_like_cpp(
        &self,
        item: &mut wow_entities::Item,
    ) -> Option<(
        wow_core::ObjectGuid,
        Option<wow_entities::PlayerItemTimeUpdate>,
        Vec<wow_entities::PlayerEnchantTimeUpdate>,
    )> {
        self.core.mutate_canonical_player_like_cpp(|player| {
            let item_update = player.add_item_durations(item);
            let enchantment_updates = player.add_enchantment_durations(item);
            (player.guid(), item_update, enchantment_updates)
        })
    }

    /// Return the current canonical Player as a detached inventory projection.
    pub fn inventory_player_projection_snapshot_like_cpp(
        &self,
    ) -> Option<InventoryPlayerProjectionLikeCpp> {
        self.core
            .with_owned_player_like_cpp(Clone::clone)
            .map(InventoryPlayerProjectionLikeCpp::from_player)
    }

    /// Read the bound player GUID for fixture reconstruction and publication.
    pub fn player_guid_like_cpp(&self) -> Option<wow_core::ObjectGuid> {
        self.core.player_guid()
    }

    /// Update visible item values through the existing GUID/map mutation path.
    pub fn set_player_visible_item_values_by_guid_like_cpp(
        &self,
        guid: wow_core::ObjectGuid,
        changes: &[(u8, i32, u16, u16)],
    ) -> bool {
        self.core
            .mutate_canonical_player_by_guid_like_cpp(guid, |player| {
                for &(slot, item_id, appearance_mod_id, visual) in changes {
                    crate::canonical_player_access::set_player_visible_item_values_like_cpp(
                        player,
                        slot,
                        (item_id, appearance_mod_id, visual),
                    );
                }
            })
            .is_some()
    }

    /// Read canonical player money without applying an Inventory fixture fallback.
    pub fn player_money_like_cpp(&self) -> Option<u64> {
        self.core
            .with_owned_player_like_cpp(|player| player.money())
    }

    /// Set canonical player money and report whether the owned player resolved.
    pub fn set_player_money_like_cpp(&self, gold: u64) -> bool {
        self.core
            .with_owned_player_mut_like_cpp(|player| player.set_money(gold))
            .is_some()
    }

    /// Read canonical bank-bag slot count without applying an Inventory fixture fallback.
    pub fn player_bank_bag_slot_count_like_cpp(&self) -> Option<u8> {
        self.core
            .with_owned_player_like_cpp(|player| player.bank_bag_slot_count())
    }

    /// Set canonical bank-bag slot count and report whether the owned player resolved.
    pub fn set_player_bank_bag_slot_count_like_cpp(&self, count: u8) -> bool {
        self.core
            .with_owned_player_mut_like_cpp(|player| player.set_bank_bag_slot_count(count))
            .is_some()
    }

    /// Read one canonical bank-bag slot flag; an absent Player or slot is `None`.
    pub fn player_bank_bag_slot_flag_like_cpp(&self, slot: usize) -> Option<u32> {
        self.core
            .with_owned_player_like_cpp(|player| player.bank_bag_slot_flag_value_like_cpp(slot))
            .flatten()
    }

    /// Set one canonical bank-bag slot flag and preserve the Player setter's bounds result.
    pub fn set_player_bank_bag_slot_flag_like_cpp(&self, slot: usize, value: u32) -> bool {
        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_bank_bag_slot_flag_value_like_cpp(slot, value)
            })
            .unwrap_or(false)
    }

    /// Read canonical inventory slot count without applying an Inventory fixture fallback.
    pub fn player_inventory_slot_count_like_cpp(&self) -> Option<u8> {
        self.core
            .with_owned_player_like_cpp(|player| player.inventory_slot_count())
    }

    /// Clone the current canonical inventory runtime while its owner is held.
    pub fn inventory_runtime_snapshot_like_cpp(&self) -> Option<PlayerInventoryRuntime> {
        self.core
            .with_owned_player_like_cpp(|player| player.inventory_runtime_like_cpp().clone())
    }

    /// Clone only the selected canonical inventory slot while its Player owner is held.
    pub fn inventory_item_snapshot_like_cpp(
        &self,
        slot: u8,
    ) -> Option<wow_entities::PlayerInventoryItem> {
        self.core
            .with_owned_player_like_cpp(|player| {
                player
                    .inventory_runtime_like_cpp()
                    .inventory_items()
                    .get(&slot)
                    .cloned()
            })
            .flatten()
    }

    /// Clone one live item object while the canonical Player owner is held.
    pub fn inventory_item_object_snapshot_like_cpp(
        &self,
        guid: wow_core::ObjectGuid,
    ) -> Option<wow_entities::Item> {
        self.core
            .with_owned_player_like_cpp(|player| {
                player
                    .inventory_runtime_like_cpp()
                    .item_objects()
                    .get(&guid)
                    .cloned()
            })
            .flatten()
    }

    /// Invalidate aura authority through the existing GUID/map mutation path.
    /// A missing manager or matching Player is ignored; the manager guard is
    /// released before this call returns.
    pub fn invalidate_spell_hit_aura_authority_for_inventory_mutation_like_cpp(&self) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }

    /// Mutate inventory only through this session's generation-checked Player
    /// handle. A missing or stale owner returns `None` without GUID/map fallback.
    /// The synchronous callback runs under the manager guard, which is released
    /// before this method returns.
    pub fn with_inventory_runtime_mut_like_cpp<R>(
        &self,
        update: impl FnOnce(&mut PlayerInventoryRuntime) -> R,
    ) -> Option<R> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            update(player.inventory_runtime_mut_like_cpp())
        })
    }

    /// Whether this fixture session has no canonical Player handle installed.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn owner_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }
}
