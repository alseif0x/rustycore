// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Selected World-session inputs for the admitted trainer application.

use super::WorldSession;
use wow_world_application::{AppTrainerCx, TrainerAcquisitionCatalogsLikeCpp};

impl WorldSession {
    /// Borrow the existing disjoint owners and selected catalogs for one
    /// trainer operation. Fixture mode follows this World consumer's
    /// `cfg(test)` status, even when the shared fixture feature is enabled.
    pub(crate) fn trainer_acquisition_context_like_cpp(&mut self) -> AppTrainerCx<'_> {
        let catalogs = TrainerAcquisitionCatalogsLikeCpp::new(
            self.catalogs.skill_store(),
            self.catalogs.skill_line_store(),
            self.catalogs.skill_tiers_store(),
            self.catalogs.item_store(),
            self.catalogs.item_stats_store(),
        );
        let owner = self.core.player_acquisition_owner_access_like_cpp();

        AppTrainerCx::new(
            owner,
            &mut self.lifecycle,
            &mut self.inventory,
            &mut self.spell_state,
            &mut self.quest_state,
            &self.loot,
            catalogs,
            cfg!(test),
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_application::TrainerAcquisitionFixturesLikeCpp::new(
                &self.fixtures.identity.player_race,
                &self.fixtures.identity.player_class,
                &self.fixtures.identity.player_level,
                &mut self.fixtures.progression.player_skill_test_fixture_like_cpp,
                &mut self.fixtures.progression.represented_enchanting_skill,
                &self.fixtures.movement.player_position,
                &self.fixtures.combat.player_health_like_cpp,
                &self.fixtures.combat.player_max_health_like_cpp,
                &self.fixtures.combat.player_alive_like_cpp,
                &self.fixtures.identity.player_level,
                &self.fixtures.vehicles.player_transport_login_state_like_cpp,
                &self.fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
                &self.fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
                &self.fixtures.vehicles.player_vehicle_seat_id_like_cpp,
                &self.fixtures.pets.represented_pet_guid_like_cpp,
            ),
        )
    }
}
