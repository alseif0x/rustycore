// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;
use wow_constants::UnitFlags;
use wow_core::ObjectGuid;

impl crate::session::WorldSession {
    pub(crate) fn represented_shapeshift_form_like_cpp(&self) -> Option<u32> {
        crate::session::hub_ref(self).represented_shapeshift_form_like_cpp()
    }
    pub(in crate::session) fn player_unit_presentation_snapshot_like_cpp(
        &self,
    ) -> Option<(UnitFlags, i32, f32)> {
        crate::session::hub_ref(self).player_unit_presentation_snapshot_like_cpp()
    }
    pub(crate) fn represented_eject_passenger_like_cpp(
        &mut self,
        passenger_guid: ObjectGuid,
    ) -> bool {
        crate::session::hub_mut(self).represented_eject_passenger_like_cpp(passenger_guid)
    }
}

impl WorldSession {
    /// C++ `Unit::SetShapeshiftForm`: write the canonical Unit field and keep
    /// the transitional Player gameplay projection in sync for the fallback
    /// readers.
    pub(crate) fn set_represented_shapeshift_form_like_cpp(&mut self, form_id: u32) -> bool {
        self.core.set_shapeshift_form_with_fixture_like_cpp(
            form_id,
            cfg!(test),
            #[cfg(any(test, feature = "test-fixtures"))]
            &mut self.fixtures.auras.represented_shapeshift_form_like_cpp,
        )
    }
    pub(crate) fn represented_primary_specialization_id_like_cpp(&self) -> Option<u32> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(Player::primary_specialization_id_like_cpp);
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(
                self.fixtures
                    .progression
                    .represented_primary_specialization_id_like_cpp,
            );
        }
        canonical
    }
    pub(crate) fn set_represented_primary_specialization_id_like_cpp(
        &mut self,
        spec_id: u32,
    ) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_primary_specialization(spec_id))
            .is_some();
        #[cfg(test)]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures
                .progression
                .represented_primary_specialization_id_like_cpp = spec_id;
            return true;
        }
        canonical
    }
    pub(crate) fn apply_far_sight_like_cpp(&mut self, enable: bool) {
        if !enable {
            #[cfg(test)]
            if let Some(player_guid) = self.player_guid() {
                self.visibility
                    .set_represented_seer_guid_fixture_like_cpp(Some(player_guid));
            }
            return;
        }

        let Some(target) = ({
            let (s, h) = crate::session::split_visibility_ref(self);
            s.current_canonical_farsight_object_like_cpp(h)
        }) else {
            debug!("CMSG_FAR_SIGHT enable requested with no current viewpoint");
            return;
        };
        if self.canonical_map_has_seer_like_object_like_cpp(target) {
            #[cfg(test)]
            {
                self.visibility
                    .set_represented_seer_guid_fixture_like_cpp(Some(target));
            }
        } else {
            debug!("CMSG_FAR_SIGHT enable target {:?} is not resoluble", target);
        }
    }
}
