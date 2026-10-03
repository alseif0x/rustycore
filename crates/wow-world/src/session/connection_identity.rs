// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Connection identity: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

#[cfg(test)]
use super::ObjectGuidGenerator;
use super::WorldSession;
use super::{Arc, ObjectGuid, SessionManager};
#[cfg(test)]
pub(in crate::session) use wow_world_core::session::PacketSpoofPendingBanLikeCpp;
pub(in crate::session) use wow_world_core::session::PacketSpoofPendingBanTargetLikeCpp;
pub use wow_world_core::session::SessionState;

pub(crate) use wow_world_social::{ChatFloodThrottleIndexLikeCpp, PlayerAwayModeLikeCpp};

pub(crate) use wow_world_lifecycle::{
    AccountDataLikeCpp, ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP, GLOBAL_CACHE_MASK_LIKE_CPP,
    PER_CHARACTER_CACHE_MASK_LIKE_CPP,
};
pub(in crate::session) use wow_world_lifecycle::default_account_data_like_cpp;

pub(in crate::session) fn trinity_sprintf_like_cpp(format: &str, args: &[&str]) -> String {
    let mut output = String::with_capacity(format.len());
    let mut chars = format.chars().peekable();
    let mut arg_idx = 0usize;

    while let Some(ch) = chars.next() {
        if ch != '%' {
            output.push(ch);
            continue;
        }

        match chars.next() {
            Some('%') => output.push('%'),
            Some('u' | 'd' | 's') => {
                if let Some(arg) = args.get(arg_idx) {
                    output.push_str(arg);
                    arg_idx += 1;
                }
            }
            Some(other) => {
                output.push('%');
                output.push(other);
            }
            None => output.push('%'),
        }
    }

    output
}

pub(in crate::session) use wow_world_core::session::connection_identity::unix_now;

impl WorldSession {
    pub fn set_realm_id(&mut self, realm_id: u16) {
        self.core.set_realm_id(realm_id)
    }

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

    /// Set the GUID generator for new characters.
    #[cfg(test)]
    pub fn set_guid_generator(&mut self, generator: Arc<ObjectGuidGenerator>) {
        self.core.guid_generator = Some(generator);
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

    pub fn realm_id(&self) -> u16 {
        self.core.realm_id()
    }

    /// Set the session manager for ConnectTo flow.
    pub fn set_session_mgr(&mut self, mgr: Arc<SessionManager>) {
        self.core.transport.session_mgr = Some(mgr);
    }

    pub fn session_mgr(&self) -> Option<&Arc<SessionManager>> {
        self.core.session_mgr()
    }

    #[cfg(test)]
    pub(crate) fn seed_empty_seasonal_event_bucket_like_cpp(&mut self, event_id: u16) {
        let _ = self.ensure_represented_seasonal_event_like_cpp(event_id);
    }

    pub fn set_legit_characters(&mut self, guids: Vec<ObjectGuid>) {
        self.core.set_legit_characters(guids)
    }

    pub fn is_legit_character(&self, guid: &ObjectGuid) -> bool {
        self.core.is_legit_character(guid)
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
            self.apply_represented_ffa_pvp_login_state_like_cpp();
        }
    }

    /// Time since the last packet was received.
    pub fn idle_time(&self) -> std::time::Duration {
        self.core.admission.last_packet_time.elapsed()
    }

    /// Whether the session is disconnecting.
    pub fn is_disconnecting(&self) -> bool {
        self.core.state == SessionState::Disconnecting
    }
}

impl crate::session::state::SessionLifecycleState {
    /// Install the Player lifecycle persistence port. Composition supplies the
    /// MariaDB adapter; unit sessions leave it empty and skip durable writes.
    pub fn set_player_lifecycle_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.player_lifecycle = Some(port);
    }

    pub(crate) fn player_lifecycle_port_like_cpp(
        &self,
    ) -> Option<&Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .player
            .player_lifecycle
            .as_ref()
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/connection_identity/f3_shims.rs"]
mod f3_shims;
