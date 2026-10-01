// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn set_near_teleport_pending_like_cpp(
        &mut self,
        pending: bool,
        destination: Option<(u16, wow_core::Position)>,
        zone_area: Option<(u32, u32)>,
    ) -> bool {
        crate::session::hub_mut(self).set_near_teleport_pending_like_cpp(
            pending,
            destination,
            zone_area,
        )
    }
    pub(crate) fn represented_can_delay_teleport_like_cpp(&self) -> bool {
        crate::session::hub_ref(self).represented_can_delay_teleport_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_has_delayed_teleport_like_cpp(&self) -> bool {
        crate::session::hub_ref(self).represented_has_delayed_teleport_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_delayed_teleport_like_cpp(
        &self,
    ) -> Option<(u32, wow_core::Position, TeleportToOptionsLikeCpp)> {
        crate::session::hub_ref(self).represented_delayed_teleport_like_cpp()
    }
    pub(in crate::session) fn player_teleport_state_snapshot_like_cpp(
        &self,
    ) -> Option<PlayerTeleportStateLikeCpp> {
        crate::session::hub_ref(self).player_teleport_state_snapshot_like_cpp()
    }
    pub(in crate::session) fn update_player_teleport_state_like_cpp(
        &mut self,
        update: impl FnOnce(&mut PlayerTeleportStateLikeCpp),
    ) -> bool {
        crate::session::hub_mut(self).update_player_teleport_state_like_cpp(update)
    }
    pub(crate) fn set_represented_can_delay_teleport_like_cpp(&mut self, can_delay: bool) -> bool {
        crate::session::hub_mut(self).set_represented_can_delay_teleport_like_cpp(can_delay)
    }
}
