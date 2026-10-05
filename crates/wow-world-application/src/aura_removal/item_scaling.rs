// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::AuraRemovalCxLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
pub(super) struct AuraScalingFixtureRefsLikeCpp<'a> {
    pub(super) quests: &'a crate::SessionQuestState,
    pub(super) vehicle_seat_flags: &'a Option<i32>,
    pub(super) vehicle_seat_id: &'a Option<u32>,
    pub(super) transport:
        &'a Option<Box<wow_world_core::session::PlayerTransportLoginStateLikeCpp>>,
}

impl AuraRemovalCxLikeCpp<'_> {
    pub(super) fn item_scaling_phase_like_cpp(&mut self) -> bool {
        let player = self.stats.reborrow_like_cpp(
            &self.player,
            #[cfg(any(test, feature = "test-fixtures"))]
            &*self.shapeshift_form,
        );
        crate::InventoryScalingApplicationCxLikeCpp::new(
            &mut *self.inventory,
            player,
            self.player.inventory_access_like_cpp(),
            self.player.inventory_valuation_access_like_cpp(),
            self.player.item_modifiers_access_like_cpp(),
            self.player.packet_publication_like_cpp(),
            self.map_store,
            self.item_mod_catalogs,
            self.loot,
            self.consumer_test,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            &*self.shapeshift_form,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.mount.position_fixture_for_scaling_like_cpp(),
            #[cfg(any(test, feature = "test-fixtures"))]
            self.scaling_fixture.transport,
            #[cfg(any(test, feature = "test-fixtures"))]
            crate::PlayerRegistryHydrationContext::new(
                self.player.registry_hydration_access_like_cpp(),
                &*self.spell,
                self.scaling_fixture.quests,
                (
                    self.mount.mount_kit_fixture_for_hydration_like_cpp(),
                    self.scaling_fixture.vehicle_seat_flags,
                    self.scaling_fixture.vehicle_seat_id,
                    self.mount.pet_guid_fixture_for_hydration_like_cpp(),
                ),
                self.consumer_test,
            ),
        )
        .update_item_level_area_based_scaling_like_cpp(true)
        .unwrap_or(false)
    }
}
