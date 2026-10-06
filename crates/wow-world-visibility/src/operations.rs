//! Represented visibility and phase state at the Session boundary.

use std::sync::atomic::Ordering;

use wow_core::ObjectGuid;
use wow_entities::PhaseShift;
use wow_packet::packets::update::{ActivePlayerDataValuesUpdate, UpdateObject};
use wow_world_core::session::set_active_player_update_bit_like_cpp;
use wow_world_core::session::{HubMut, HubRef};

use crate::VisibilityState;

impl VisibilityState {
    pub fn set_represented_player_phase_shift_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        phase_shift: PhaseShift,
    ) -> bool {
        let mut phase_shift = Some(phase_shift);
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                *player.unit_mut().world_mut().phase_shift_mut() =
                    phase_shift.take().expect("phase mutation runs once");
            })
            .is_some();
        if canonical {
            return true;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            self.visibility_test_fixture_like_cpp
                .represented_player_phase_shift =
                phase_shift.take().expect("fixture phase remains available");
            return true;
        }
        false
    }

    /// Send represented `ActivePlayerData::FarsightObject` VALUES update after
    /// the canonical AddFarsight `Player::SetViewpoint(..., true)` success.
    ///
    /// C++ anchors: `Player::SetViewpoint` writes
    /// `UF::ActivePlayerData::FarsightObject`; `ActivePlayerData::WriteUpdate`
    /// emits the field under parent block `changesMask[0]` and field bit 26.
    pub fn send_active_player_farsight_object_values_update_like_cpp(
        &self,
        hub: HubRef<'_>,
        player_guid: ObjectGuid,
        farsight_guid: ObjectGuid,
    ) {
        let mut data = ActivePlayerDataValuesUpdate::default();
        set_active_player_update_bit_like_cpp(&mut data.active_player_data_mask, 0);
        set_active_player_update_bit_like_cpp(&mut data.active_player_data_mask, 26);
        data.farsight_object = farsight_guid;
        hub.core
            .send_packet(&UpdateObject::full_active_player_values_update(
                player_guid,
                hub.core.player_map_id_like_cpp(),
                data,
            ));
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_seer_guid_like_cpp(&self) -> Option<ObjectGuid> {
        self.visibility_test_fixture_like_cpp
            .represented_seer_guid_like_cpp
    }

    pub fn current_canonical_farsight_object_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<ObjectGuid> {
        let value = hub
            .core
            .current_canonical_player_farsight_object_value_like_cpp()?;
        (!value.is_empty()).then_some(value)
    }

    /// Consume the represented `Player::SetViewpoint(target, false)`/`SetSeer(this)`
    /// side effect after canonical DynamicObject viewpoint removal has already
    /// cleared the map-owned Player `ActivePlayerData::FarsightObject`.
    ///
    /// Ownership remains one-way: canonical map Player state is the source of
    /// truth. The session keeps only a publication fence so a clear VALUES
    /// packet is emitted once when the map-owned viewpoint disappears.
    pub fn sync_represented_farsight_clear_from_canonical_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) -> bool {
        let Some(player_guid) = hub.core.player_guid() else {
            return false;
        };
        let Some(canonical_farsight_object) = hub
            .core
            .current_canonical_player_farsight_object_value_like_cpp()
        else {
            return false;
        };
        if !canonical_farsight_object.is_empty() {
            self.last_observed_farsight_object_like_cpp = canonical_farsight_object;
            return false;
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        let had_non_player_seer = self
            .represented_seer_guid_like_cpp()
            .is_some_and(|seer_guid| !seer_guid.is_empty() && seer_guid != player_guid);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let had_non_player_seer = !self.last_observed_farsight_object_like_cpp.is_empty();
        if !had_non_player_seer {
            return false;
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.visibility_test_fixture_like_cpp
                .represented_seer_guid_like_cpp = Some(player_guid);
        }
        self.last_observed_farsight_object_like_cpp = ObjectGuid::EMPTY;
        self.send_active_player_farsight_object_values_update_like_cpp(
            hub.shared(),
            player_guid,
            ObjectGuid::EMPTY,
        );
        self.last_visibility_pos = None;
        true
    }

    pub fn clear_pending_visibility_refresh_like_cpp(&self, hub: HubRef<'_>) {
        hub.core
            .flags
            .visibility_refresh_pending_like_cpp
            .store(false, Ordering::Release);
    }
}
