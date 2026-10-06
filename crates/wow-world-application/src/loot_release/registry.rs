// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

impl LootReleaseCxLikeCpp<'_> {
    /// Re-publish the owner's registry placement after a release transition.
    /// Fixture values come from the context's own `SessionFixtures`, the single
    /// mutable fixture owner the release already holds, so no second bundle of
    /// the same fields is borrowed.
    pub(super) fn sync_player_registry_state_like_cpp(&self) {
        let Some(control) = self.owner.registry_control_binding_like_cpp() else {
            return;
        };
        let position = self.owner.registry_sync_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.movement.player_position,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.vehicles.player_transport_login_state_like_cpp,
        );
        let sync = crate::PlayerRegistrySyncContext::new(
            position,
            control,
            self.loot,
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_core::session::RegistrySyncInputs::new_like_cpp(
                &self.fixtures.combat.player_health_like_cpp,
                &self.fixtures.combat.player_max_health_like_cpp,
                &self.fixtures.combat.player_alive_like_cpp,
            ),
        );
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.consumer_test {
            sync.with_fixture_hydration(crate::PlayerRegistryHydrationContext::new(
                self.owner.registry_hydration_like_cpp(),
                self.spell_state,
                self.quest_state,
                (
                    &self.fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
                    &self.fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
                    &self.fixtures.vehicles.player_vehicle_seat_id_like_cpp,
                    &self.fixtures.pets.represented_pet_guid_like_cpp,
                ),
                true,
            ))
            .sync();
            return;
        }
        sync.sync();
    }
}
