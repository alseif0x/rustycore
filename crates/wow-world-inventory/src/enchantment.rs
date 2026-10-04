// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::RepresentedItemBonusActionLikeCpp;
use wow_constants::{item::EnchantmentSlot, SpellItemEnchantmentFlags};
use wow_core::ObjectGuid;
use wow_entities::{
    ApplyEnchantmentArgs, ApplyEnchantmentPlan, EQUIPMENT_SLOT_END, PlayerEnchantTimeUpdate,
};
use wow_packet::packets::item::ItemEnchantTimeUpdate;
use wow_world_core::session::{HubMut, HubRef, SKILL_ENCHANTING_LIKE_CPP};

mod operation;
pub use operation::{ItemEnchantmentApplicationCxLikeCpp, ItemEnchantmentCatalogsLikeCpp};

#[derive(Debug, Clone, Default)]
pub struct LoadedEquippedItemEnchantmentsOutcomeLikeCpp {
    pub plans: Vec<ApplyEnchantmentPlan>,
    pub duration_updates: Vec<PlayerEnchantTimeUpdate>,
    pub send_stat_update: bool,
    pub visible_item_changes: Vec<(u8, i32, u16, u16)>,
    pub effect_actions: Vec<RepresentedItemBonusActionLikeCpp>,
    pub unrepresented_effect_actions: Vec<RepresentedItemBonusActionLikeCpp>,
}

impl LoadedEquippedItemEnchantmentsOutcomeLikeCpp {
    pub(crate) fn append(&mut self, mut other: Self) {
        self.plans.append(&mut other.plans);
        self.duration_updates.append(&mut other.duration_updates);
        self.send_stat_update |= other.send_stat_update;
        self.visible_item_changes
            .append(&mut other.visible_item_changes);
        self.effect_actions.append(&mut other.effect_actions);
        self.unrepresented_effect_actions
            .append(&mut other.unrepresented_effect_actions);
    }
}

impl crate::InventoryState {
    /// C++ `Player::ApplyEnchantment(item, slot, apply, ...)` bridge for a
    /// represented inventory item owned by the current player.
    ///
    /// The item runtime lives in the session inventory while the player state
    /// lives in the canonical map. This temporarily moves the item out, runs the
    /// entity-level plan against the canonical player when available, then puts
    /// the item back without clearing or setting the enchantment field itself.
    pub fn apply_current_player_item_enchantment_plan_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
        args: ApplyEnchantmentArgs,
    ) -> Option<ApplyEnchantmentPlan> {
        let inventory = hub.core.owned_inventory_access_like_cpp();
        let owner = hub.core.owned_item_enchantment_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &hub.fixtures.progression.player_skill_test_fixture_like_cpp.player_skill_records_like_cpp,
        );
        let catalogs = ItemEnchantmentCatalogsLikeCpp::new(
            hub.catalogs.spell_catalogs.spell_item_enchantment_store.as_deref(),
            hub.catalogs.spell_catalogs.spell_item_enchantment_condition_store.as_deref(),
            hub.catalogs.items.store.as_ref(),
            hub.catalogs.items.stats_store.as_ref(),
            hub.catalogs.gem_properties_store.as_deref(),
        );
        ItemEnchantmentApplicationCxLikeCpp::new(self, inventory, owner, catalogs)
            .apply_current_player_item_enchantment_plan_like_cpp(item_guid, slot, args)
    }

    pub(crate) fn remove_enchantment_item_object_with_access_like_cpp(
        &mut self,
        access: &wow_world_core::session::OwnedInventoryAccessLikeCpp<'_>,
        item_guid: ObjectGuid,
    ) -> Option<wow_entities::Item> {
        self.mutate_player_inventory_runtime_with_access_like_cpp(access, |inventory| {
            inventory.remove_item_object_like_cpp(item_guid)
        }).flatten()
    }

    /// C++ `_StoreItem` merge branch calls `AddEnchantmentDurations(pItem2)`
    /// without calling `AddItemDurations` for the existing destination stack.
    pub fn refresh_inventory_item_enchantment_duration_refs_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
    ) {
        let Some(mut item) = self.resolved_inventory_item_object_like_cpp(hub.shared(), item_guid)
        else {
            return;
        };
        let Some((owner_guid, enchantment_updates)) =
            hub.core.mutate_canonical_player_like_cpp(|player| {
                (player.guid(), player.add_enchantment_durations(&mut item))
            })
        else {
            return;
        };

        self.insert_inventory_item_object(hub, item);
        self.send_item_enchant_time_update_plans(hub.shared(), owner_guid, &enchantment_updates);
    }

    /// C++ `Player::RemoveItem` clears main-hand-only enchantments when the
    /// main-hand item leaves that slot. Return both the post-remove DB value
    /// and the runtime slots to clear, without mutating live state before the
    /// caller's transaction commits.
    pub fn inventory_remove_enchantment_persistence_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_guid: ObjectGuid,
        clear_mainhand_only: bool,
    ) -> Option<(String, Vec<EnchantmentSlot>)> {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.inventory_remove_enchantment_persistence_with_access_like_cpp(
            &access,
            hub.catalogs.spell_catalogs.spell_item_enchantment_store.as_deref(),
            item_guid,
            clear_mainhand_only,
        )
    }

    pub fn inventory_remove_enchantment_persistence_with_access_like_cpp(
        &self,
        access: &wow_world_core::session::OwnedInventoryAccessLikeCpp<'_>,
        enchantment_store: Option<&wow_data::SpellItemEnchantmentStore>,
        item_guid: ObjectGuid,
        clear_mainhand_only: bool,
    ) -> Option<(String, Vec<EnchantmentSlot>)> {
        let item = self.resolved_player_inventory_item_object_with_access_like_cpp(access, item_guid)?;
        let current_durations = access
            .inventory_enchantment_durations_snapshot_like_cpp(item_guid)
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
                    enchantment_store?.get(id)
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

    pub fn send_loaded_equipped_item_enchantment_updates_like_cpp(
        &self,
        hub: HubRef<'_>,
        outcome: &LoadedEquippedItemEnchantmentsOutcomeLikeCpp,
    ) {
        if let Some(owner_guid) = hub.core.player_guid() {
            self.send_item_enchant_time_update_plans(hub, owner_guid, &outcome.duration_updates);
        }
        if !outcome.visible_item_changes.is_empty() {
            self.send_player_values_update_from_entity_bridge(
                hub,
                &[],
                &outcome.visible_item_changes,
                &[],
                &[],
                None,
            );
        }
    }
}

impl crate::InventoryState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn item_disenchant_loot_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_id: u32,
        quality: u32,
        item_level: u32,
        can_disenchant_bonus: bool,
    ) -> Option<(u32, u16)> {
        hub.catalogs.item_disenchant_loot_with_catalogs_like_cpp(
            &hub.catalogs.item_valuation_catalogs_for_test_like_cpp(),
            item_id,
            quality,
            item_level,
            can_disenchant_bonus,
        )
    }

    pub fn send_item_enchant_time_update_plan(
        &self,
        hub: HubRef<'_>,
        owner_guid: ObjectGuid,
        update: &PlayerEnchantTimeUpdate,
    ) {
        hub.core.send_packet(&ItemEnchantTimeUpdate {
            owner_guid,
            item_guid: update.item_guid,
            duration_left: update.duration_secs,
            slot: update.slot as u32,
        });
    }

    pub fn send_item_enchant_time_update_plans(
        &self,
        hub: HubRef<'_>,
        owner_guid: ObjectGuid,
        updates: &[PlayerEnchantTimeUpdate],
    ) {
        for update in updates {
            self.send_item_enchant_time_update_plan(hub, owner_guid, update);
        }
    }

    pub fn resolved_enchanting_skill_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<u16> {
        let canonical = hub.core.with_owned_player_like_cpp(|player| {
            player.enchanting_skill_value_like_cpp(SKILL_ENCHANTING_LIKE_CPP)
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
            return Some(hub.fixtures.progression.represented_enchanting_skill);
        }
        canonical
    }
}
