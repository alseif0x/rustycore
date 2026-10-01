// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_pet_guid_like_cpp(&self) -> Option<ObjectGuid> {
        crate::session::hub_ref(self).represented_pet_guid_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_pet_movement_speed_rate_like_cpp(
        &self,
        move_type: UnitMoveTypeLikeCpp,
    ) -> f32 {
        crate::session::hub_ref(self).represented_pet_movement_speed_rate_like_cpp(move_type)
    }
    #[cfg(test)]
    pub(crate) fn represented_pet_speed_propagations_like_cpp(&self) -> u32 {
        self.fixtures
            .pets
            .represented_pet_speed_propagations_like_cpp()
    }
    pub(in crate::session) fn player_pet_lifecycle_state_snapshot_like_cpp(
        &self,
    ) -> Option<PlayerPetLifecycleStateLikeCpp> {
        crate::session::hub_ref(self).player_pet_lifecycle_state_snapshot_like_cpp()
    }
}
