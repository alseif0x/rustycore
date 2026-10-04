// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Complete area-based item-level transition over selected owners.

use wow_world_core::session::{
    InventoryValuationAccessLikeCpp, OwnedInventoryAccessLikeCpp,
    OwnedItemModifiersAccessLikeCpp, PacketPublicationAccessLikeCpp, PlayerStatsAccessLikeCpp,
};
use wow_world_inventory::{InventoryState, ItemModsCatalogsViewLikeCpp};
use wow_world_loot::LootState;

pub struct InventoryScalingApplicationCxLikeCpp<'a> {
    inventory: &'a mut InventoryState,
    player: PlayerStatsAccessLikeCpp<'a>,
    inventory_access: OwnedInventoryAccessLikeCpp<'a>,
    valuation: InventoryValuationAccessLikeCpp<'a>,
    modifiers: OwnedItemModifiersAccessLikeCpp<'a>,
    publication: PacketPublicationAccessLikeCpp<'a>,
    map_store: Option<&'a wow_data::map::MapStore>,
    item_mods: ItemModsCatalogsViewLikeCpp<'a>,
    loot: &'a LootState,
    consumer_test: bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_level: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    shapeshift_form: &'a u32,
    #[cfg(any(test, feature = "test-fixtures"))]
    position: &'a Option<wow_core::Position>,
    #[cfg(any(test, feature = "test-fixtures"))]
    transport: &'a Option<Box<wow_world_core::session::PlayerTransportLoginStateLikeCpp>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    hydration: crate::PlayerRegistryHydrationContext<'a>,
}

impl<'a> InventoryScalingApplicationCxLikeCpp<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        inventory: &'a mut InventoryState,
        player: PlayerStatsAccessLikeCpp<'a>,
        inventory_access: OwnedInventoryAccessLikeCpp<'a>,
        valuation: InventoryValuationAccessLikeCpp<'a>,
        modifiers: OwnedItemModifiersAccessLikeCpp<'a>,
        publication: PacketPublicationAccessLikeCpp<'a>,
        map_store: Option<&'a wow_data::map::MapStore>,
        item_mods: ItemModsCatalogsViewLikeCpp<'a>,
        loot: &'a LootState,
        consumer_test: bool,
        #[cfg(any(test, feature = "test-fixtures"))] player_level: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))] shapeshift_form: &'a u32,
        #[cfg(any(test, feature = "test-fixtures"))] position: &'a Option<wow_core::Position>,
        #[cfg(any(test, feature = "test-fixtures"))]
        transport: &'a Option<Box<wow_world_core::session::PlayerTransportLoginStateLikeCpp>>,
        #[cfg(any(test, feature = "test-fixtures"))]
        hydration: crate::PlayerRegistryHydrationContext<'a>,
    ) -> Self {
        Self {
            inventory, player, inventory_access, valuation, modifiers, publication,
            map_store, item_mods, loot, consumer_test,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            shapeshift_form,
            #[cfg(any(test, feature = "test-fixtures"))]
            position,
            #[cfg(any(test, feature = "test-fixtures"))]
            transport,
            #[cfg(any(test, feature = "test-fixtures"))]
            hydration,
        }
    }

    /// C++ Player.cpp:28715–28729 supplies the phase reference. The existing
    /// Rust integer restoration and publication sequence are retained here.
    pub fn update_item_level_area_based_scaling_like_cpp(mut self, publish: bool) -> Option<bool> {
        let map_pvp_activity = self.map_store
            .and_then(|store| store.get(u32::from(self.valuation.player_map_id_like_cpp())))
            .is_some_and(|entry| {
                entry.is_battleground_or_arena() || entry.activates_pvp_item_levels_like_cpp()
            });
        let pvp_activity = map_pvp_activity || self.player.item_scaling_pvp_rules_enabled_like_cpp();
        let Some(using) = self.inventory.resolved_using_pvp_item_levels_with_access_like_cpp(&self.valuation)
        else { return None; };
        if using == pvp_activity { return Some(false); }
        let Some((health_before, max_health_before, _)) = self.player.resolved_player_vitals_like_cpp()
        else { return None; };
        let Some(targets) = self.inventory.represented_top_level_item_mod_targets_with_access_like_cpp(&self.inventory_access)
        else { return None; };
        self.record_all_item_mods_like_cpp(&targets, false);
        if !self.inventory.set_represented_using_pvp_item_levels_with_access_like_cpp(&self.valuation, pvp_activity) {
            return None;
        }
        self.record_all_item_mods_like_cpp(&targets, true);
        if let Some((_, max_health_after, _)) = self.player.resolved_player_vitals_like_cpp() {
            let restored = restored_health_like_cpp(health_before, max_health_before, max_health_after);
            let _ = self.player.sync_canonical_player_health_like_cpp(restored, max_health_after);
            if let Some((position, control)) = self.player.item_scaling_registry_access_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                self.position,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.transport,
            ) {
                let sync = crate::PlayerRegistrySyncContext::new(
                    position,
                    control,
                    self.loot,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    self.player.registry_sync_inputs_like_cpp(),
                );
                #[cfg(any(test, feature = "test-fixtures"))]
                let sync = sync.with_fixture_hydration(self.hydration);
                sync.sync();
            }
        }
        if publish && !targets.is_empty() {
            let mut stats = crate::stats::CharacterStatsApplicationCxLikeCpp::new(
                self.player, &*self.inventory, self.publication,
            );
            let sent = stats.send_stat_update_like_cpp();
            drop(stats);
            if !sent {
                let Some(guid) = self.valuation.player_guid_like_cpp() else { return Some(true); };
                if let Some(bonuses) = self.inventory.represented_item_bonus_state_for_equipment_set_use_like_cpp(&self.modifiers) {
                    let update = wow_packet::packets::update::UpdateObject::player_stat_update(
                        guid, self.valuation.player_map_id_like_cpp(),
                        wow_world_inventory::represented_player_stat_changes_like_cpp(&bonuses),
                    );
                    self.valuation.packet_publication_access_like_cpp().send_packet(&update);
                }
            }
        }
        Some(true)
    }

    fn record_all_item_mods_like_cpp(&mut self, targets: &[(u8, wow_core::ObjectGuid)], apply: bool) {
        for (slot, guid) in targets {
            self.inventory.record_represented_item_mods_with_access_like_cpp(
                &self.inventory_access, &self.modifiers, self.item_mods, *guid, *slot, apply,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.player_level,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.shapeshift_form,
                self.consumer_test,
            );
        }
    }
}

fn restored_health_like_cpp(health_before: u32, max_health_before: u32, max_health_after: u32) -> u32 {
    (u64::from(max_health_after) * u64::from(health_before)
        / u64::from(max_health_before.max(1))) as u32
}

#[cfg(test)]
mod tests {
    use super::restored_health_like_cpp;
    #[test]
    fn scaling_restoration_retains_integer_floor_and_zero_maximum_guard() {
        assert_eq!(restored_health_like_cpp(1, 3, 10), 3);
        assert_eq!(restored_health_like_cpp(0, 0, 100), 0);
        assert_eq!(restored_health_like_cpp(1, 0, 100), 100);
    }
}
