// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Connection identity: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;
use super::{Arc, ObjectGuid, SessionManager};
#[cfg(test)]
use wow_core::ObjectGuidGenerator;
#[cfg(test)]
pub(in crate::session) use wow_world_core::session::PacketSpoofPendingBanLikeCpp;
pub(in crate::session) use wow_world_core::session::PacketSpoofPendingBanTargetLikeCpp;
pub use wow_world_core::session::SessionState;

#[cfg(test)]
pub(crate) use wow_world_social::{ChatFloodThrottleIndexLikeCpp, PlayerAwayModeLikeCpp};

pub(crate) use wow_world_lifecycle::{
    AccountDataLikeCpp, GLOBAL_CACHE_MASK_LIKE_CPP, PER_CHARACTER_CACHE_MASK_LIKE_CPP,
};

impl WorldSession {
    pub fn set_realm_handle_like_cpp(&mut self, region: u8, battlegroup: u8, realm_id: u16) {
        self.core.realm_policy.realm_region = region;
        self.core.realm_policy.realm_battlegroup = battlegroup;
        self.core.realm_id = realm_id;
    }

    pub fn set_realm_names_like_cpp(
        &mut self,
        names: impl IntoIterator<Item = (u32, String, String)>,
    ) {
        self.core.realm_policy.realm_names_like_cpp = names
            .into_iter()
            .map(|(address, actual, normalized)| (address, (actual, normalized)))
            .collect();
    }

    pub fn set_player_lifecycle_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
    ) {
        self.lifecycle.set_player_lifecycle_port_like_cpp(port)
    }

    pub fn set_mute_time_like_cpp(&mut self, mute_time: i64) {
        self.core.account_state.mute_time_like_cpp = mute_time;
    }

    pub fn set_recruiter_id_like_cpp(&mut self, recruiter_id: u32) {
        self.core.account_state.recruiter_id_like_cpp = recruiter_id;
    }

    pub fn set_is_a_recruiter_like_cpp(&mut self, is_a_recruiter: bool) {
        self.core.account_state.is_a_recruiter_like_cpp = is_a_recruiter;
    }

    /// Set the session manager for ConnectTo flow.
    pub fn set_session_mgr(&mut self, mgr: Arc<SessionManager>) {
        self.core.transport.session_mgr = Some(mgr);
    }

    pub(crate) fn session_mgr(&self) -> Option<&Arc<SessionManager>> {
        self.core.session_mgr()
    }

    #[cfg(test)]
    pub(crate) fn seed_empty_seasonal_event_bucket_like_cpp(&mut self, event_id: u16) {
        let _ = self.ensure_represented_seasonal_event_like_cpp(event_id);
    }

    pub fn set_legit_characters(&mut self, guids: Vec<ObjectGuid>) {
        self.core.set_legit_characters(guids)
    }

    /// Get the current session state.
    pub fn state(&self) -> SessionState {
        self.core.state
    }

    /// Set the session state (e.g., after character login).
    pub fn set_state(&mut self, state: SessionState) {
        let entered_world =
            state == SessionState::LoggedIn && self.core.state != SessionState::LoggedIn;
        self.core.state = state;
        if entered_world {
            wow_world_entities::apply_represented_ffa_pvp_login_state_like_cpp(self);
        }
    }

    /// Whether the session is disconnecting.
    pub fn is_disconnecting(&self) -> bool {
        self.core.state == SessionState::Disconnecting
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/connection_identity/f3_shims.rs"]
mod f3_shims;
