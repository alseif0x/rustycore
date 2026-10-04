//! Represented item level and price valuation.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    pub(crate) fn set_represented_item_level_caps_like_cpp(
        &mut self,
        caps: RepresentedItemLevelCapsLikeCpp,
    ) -> bool {
        self.set_player_item_level_caps_like_cpp(caps)
    }
    pub(crate) fn set_represented_using_pvp_item_levels_like_cpp(&mut self, active: bool) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_represented_using_pvp_item_levels_like_cpp(&mut hub, active)
    }
    pub(in crate::session) fn resolved_using_pvp_item_levels_like_cpp(&self) -> Option<bool> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_using_pvp_item_levels_like_cpp(hub)
    }
    pub(crate) fn update_represented_item_level_area_based_scaling_like_cpp(&mut self) -> bool {
        self.update_represented_item_level_area_based_scaling_with_publication_like_cpp(true)
            .unwrap_or(false)
    }
    pub(crate) fn update_represented_item_level_area_based_scaling_with_publication_like_cpp(
        &mut self,
        publish: bool,
    ) -> Option<bool> {
        #[cfg(any(test, feature = "test-fixtures"))]
        let player = self.core.player_stats_access_with_fixture_refs_like_cpp(
            &self.catalogs, &self.config,
            &self.fixtures.identity.player_race,
            &self.fixtures.identity.player_class,
            &self.fixtures.identity.player_level,
            wow_world_core::session::StatsFixtureRefs::new_like_cpp(
                wow_world_core::session::StatsCombatFixtureRefs::new_like_cpp(
                    &mut self.fixtures.combat.player_health_like_cpp,
                    &mut self.fixtures.combat.player_max_health_like_cpp,
                    &mut self.fixtures.combat.player_alive_like_cpp,
                    &mut self.fixtures.combat.represented_player_powers_like_cpp[0],
                    &mut self.fixtures.combat.represented_player_max_powers_like_cpp[0],
                    &mut self.fixtures.combat.represented_player_base_mana_like_cpp,
                ),
                wow_world_core::session::StatsAuraFixtureRefs::new_like_cpp(
                    &self.fixtures.auras.represented_shapeshift_form_like_cpp,
                    &self.fixtures.auras.player_aura_authority_complete_like_cpp,
                    &self.fixtures.auras.player_spell_hit_aura_authority_tombstoned_like_cpp,
                    &self.fixtures.auras.visible_auras,
                    &self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
                ),
            ),
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let player = self.core.player_stats_access_like_cpp(&self.catalogs, &self.config);
        wow_world_application::InventoryScalingApplicationCxLikeCpp::new(
            &mut self.inventory,
            player,
            self.core.owned_inventory_access_like_cpp(),
            self.core.inventory_valuation_access_like_cpp(),
            self.core.owned_item_modifiers_access_like_cpp(),
            self.core.packet_publication_access_like_cpp(),
            self.catalogs.map_store().map(AsRef::as_ref),
            wow_world_inventory::ItemModsCatalogsViewLikeCpp::new(
                self.catalogs.items.store.as_ref(),
                self.catalogs.items.stats_store.as_ref(),
                self.catalogs.scaling_stat_distribution_store.as_ref(),
                self.catalogs.scaling_stat_values_store.as_ref(),
                self.catalogs.shield_block_regular_game_table.as_ref(),
                self.catalogs.spell_catalogs.spell_shapeshift_form_store(),
            ),
            &self.loot,
            cfg!(test),
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.auras.represented_shapeshift_form_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.movement.player_position,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.vehicles.player_transport_login_state_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_application::PlayerRegistryHydrationContext::new(
                self.core.player_registry_hydration_access_like_cpp(),
                &self.spell_state,
                &self.quest_state,
                (
                    &self.fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
                    &self.fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
                    &self.fixtures.vehicles.player_vehicle_seat_id_like_cpp,
                    &self.fixtures.pets.represented_pet_guid_like_cpp,
                ),
                cfg!(test),
            ),
        ).update_item_level_area_based_scaling_like_cpp(publish)
    }
    /// C++ `Player::GetAverageItemLevel`.
    pub(crate) fn represented_average_item_level_like_cpp(&self) -> Option<f32> {
        self.inventory.represented_average_item_level_with_access_like_cpp(
            &self.core.owned_inventory_access_like_cpp(),
            &self.core.inventory_valuation_access_like_cpp(),
            &self.core.owned_item_modifiers_access_like_cpp(),
            &self.catalogs.inventory_valuation_catalog_view_like_cpp(),
            Self::MIN_ITEM_LEVEL_LIKE_CPP,
            Self::MAX_ITEM_LEVEL_LIKE_CPP,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_level,
        )
    }
    pub(in crate::session) fn represented_item_level_like_cpp(
        &self,
        entry_id: u32,
        runtime_item: Option<&Item>,
    ) -> Option<u32> {
        let valuation_access = self.core.inventory_valuation_access_like_cpp();
        let modifier_access = self.core.owned_item_modifiers_access_like_cpp();
        let catalogs = self.catalogs.inventory_valuation_catalog_view_like_cpp();
        self.inventory.represented_item_level_with_access_like_cpp(
            &valuation_access,
            &modifier_access,
            &catalogs,
            entry_id,
            runtime_item,
            Self::MIN_ITEM_LEVEL_LIKE_CPP,
            Self::MAX_ITEM_LEVEL_LIKE_CPP,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_level,
        )
    }
}



#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/valuation/f3_shims.rs"]
mod f3_shims;
