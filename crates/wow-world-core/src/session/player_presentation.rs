// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical player presentation adapters shared with World.

use wow_constants::UnitFlags;
use wow_constants::movement::MovementFlag;
use wow_core::ObjectGuid;
use wow_entities::Player;

pub const LIQUID_MAP_IN_WATER_LIKE_CPP: u32 = 0x0000_0004;
pub const LIQUID_MAP_UNDER_WATER_LIKE_CPP: u32 = 0x0000_0008;

impl crate::session::HubMut<'_> {
    pub fn represented_eject_passenger_like_cpp(&mut self, passenger_guid: ObjectGuid) -> bool {
        if !passenger_guid.is_unit() {
            return false;
        }

        self.eject_player_mount_vehicle_passenger_like_cpp(passenger_guid)
    }
}

impl crate::session::HubMut<'_> {
    pub fn set_player_mount_presentation_like_cpp(
        &mut self,
        display_id: i32,
        mounted: bool,
    ) -> bool {
        self.player_aura_removal_access_like_cpp()
            .set_player_mount_presentation_like_cpp(display_id, mounted)
    }
}

impl crate::session::HubRef<'_> {
    pub fn player_is_game_master_like_cpp(&self) -> Option<bool> {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core.player_is_game_master_with_fixture_like_cpp(
                &self.fixtures.combat.player_game_master_like_cpp,
            )
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_is_game_master_canonical_like_cpp()
        }
    }

    pub fn player_unit_presentation_snapshot_like_cpp(&self) -> Option<(UnitFlags, i32, f32)> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            (
                player.unit().unit_flags_like_cpp(),
                player.unit().data().mount_display_id,
                player.unit().world().object().scale(),
            )
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some((
                self.fixtures.presentation.player_unit_flags_like_cpp,
                self.fixtures.vehicles.player_mount_display_id_like_cpp,
                self.fixtures.presentation.player_object_scale_like_cpp,
            ));
        }
        canonical
    }

    /// C++ `Unit::GetShapeshiftForm`: the canonical `UNIT_FIELD_BYTES_2` byte
    /// owned by the Unit, with the transitional Player gameplay projection as
    /// the fallback for fixtures that only seed it.
    pub fn represented_shapeshift_form_like_cpp(&self) -> Option<u32> {
        self.core.represented_shapeshift_form_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.auras.represented_shapeshift_form_like_cpp,
        )
    }

    pub fn represented_player_mount_liquid_state_like_cpp(&self) -> Option<(bool, bool)> {
        let liquid_status = self.core.player_liquid_status_like_cpp()?;
        let is_submerged = liquid_status & LIQUID_MAP_UNDER_WATER_LIKE_CPP != 0
            || self
                .resolved_player_movement_flags_like_cpp()?
                .contains(MovementFlag::SWIMMING);
        let is_in_water =
            liquid_status & (LIQUID_MAP_IN_WATER_LIKE_CPP | LIQUID_MAP_UNDER_WATER_LIKE_CPP) != 0;
        Some((is_submerged, is_in_water))
    }
}

impl crate::session::state::SessionCore {
    pub(crate) fn represented_shapeshift_form_with_fixture_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_form: &u32,
    ) -> Option<u32> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            let form_id = u32::from(player.unit().shapeshift_form_id_like_cpp());
            if form_id != 0 {
                form_id
            } else {
                player.shapeshift_form_id_like_cpp()
            }
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(*fixture_form);
        }
        canonical
    }

    pub(crate) fn represented_shapeshift_combat_round_time_with_fixture_refs_like_cpp(
        &self,
        catalogs: &crate::session::SessionCatalogs,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_form: &u32,
    ) -> Option<f32> {
        let form_id = self.represented_shapeshift_form_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_form,
        )?;
        let store = catalogs.spell_catalogs.spell_shapeshift_form_store()?;
        let form = store.get(form_id)?;
        (form.combat_round_time > 0).then(|| f32::from(form.combat_round_time))
    }
}

impl crate::session::state::SessionCore {
    pub(crate) fn player_is_game_master_canonical_like_cpp(&self) -> Option<bool> {
        self.with_owned_player_like_cpp(Player::is_game_master_like_cpp)
    }

    /// Resolve canonical GM state, or the selected handle-less fixture value
    /// at the caller's original query point.
    pub(crate) fn player_is_game_master_with_fixture_like_cpp(
        &self,
        fixture_value: &bool,
    ) -> Option<bool> {
        let canonical = self.with_owned_player_like_cpp(Player::is_game_master_like_cpp);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(*fixture_value);
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = fixture_value;
        canonical
    }
}
