// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Connection identity: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Arc, Item, NUM_ACCOUNT_DATA_TYPES, ObjectGuid, ObjectGuidGenerator, SessionManager};
use super::{VoidStorageItemIdGeneratorLikeCpp, WorldSession};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerAwayModeLikeCpp {
    Afk,
    Dnd,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::session) struct PacketCounterLikeCpp {
    pub(in crate::session) last_receive_time_secs: u64,
    pub(in crate::session) amount_counter: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::session) enum PacketSpoofPendingBanTargetLikeCpp {
    Account { account_id: u32 },
    Ip { address: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::session) struct PacketSpoofPendingBanLikeCpp {
    pub(in crate::session) target: PacketSpoofPendingBanTargetLikeCpp,
    pub(in crate::session) duration_secs: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub(in crate::session) struct ChatFloodThrottleDataLikeCpp {
    pub(in crate::session) time: i64,
    pub(in crate::session) count: u32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ChatFloodThrottleIndexLikeCpp {
    Regular = 0,
    Addon = 1,
}

/// Current state of the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Authenticated but no character selected.
    Authed,
    /// Character is logged into the world.
    LoggedIn,
    /// Character is transferring between maps.
    Transfer,
    /// Session is being disconnected.
    Disconnecting,
}

/// C++ `WorldSession::_accountData[NUM_ACCOUNT_DATA_TYPES]` entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AccountDataLikeCpp {
    pub time: i64,
    pub data: String,
}

pub(crate) const ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP: u32 = 0x7FFF;
pub(crate) const GLOBAL_CACHE_MASK_LIKE_CPP: u32 = 0x2515;
pub(crate) const PER_CHARACTER_CACHE_MASK_LIKE_CPP: u32 = 0x5AEA;

pub(in crate::session) fn default_account_data_like_cpp()
-> [AccountDataLikeCpp; NUM_ACCOUNT_DATA_TYPES] {
    std::array::from_fn(|_| AccountDataLikeCpp::default())
}

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

/// Current Unix timestamp (seconds since epoch).
pub(in crate::session) fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

impl WorldSession {
    pub(crate) fn auto_reply_msg_like_cpp(&self) -> Option<String> {
        self.canonical_player_snapshot_like_cpp(|player| {
            player
                .gameplay_state()
                .social
                .auto_reply_msg_like_cpp
                .clone()
        })
    }

    pub fn set_realm_id(&mut self, realm_id: u16) {
        self.realm_id = realm_id;
    }

    pub fn set_realm_handle_like_cpp(&mut self, region: u8, battlegroup: u8, realm_id: u16) {
        self.realm_region = region;
        self.realm_battlegroup = battlegroup;
        self.realm_id = realm_id;
    }

    pub fn set_realm_names_like_cpp(
        &mut self,
        names: impl IntoIterator<Item = (u32, String, String)>,
    ) {
        self.realm_names_like_cpp = names
            .into_iter()
            .map(|(address, actual, normalized)| (address, (actual, normalized)))
            .collect();
    }

    /// Compute the Virtual Realm Address: `(Region << 24) | (Battlegroup << 16) | RealmId`.
    ///
    /// Region and Battlegroup come from the active `realmlist` row, matching C++
    /// `Battlenet::RealmHandle{ realm.Id.Region, realm.Id.Site, realm.Id.Realm }.GetAddress()`.
    pub(crate) fn virtual_realm_address(&self) -> u32 {
        (u32::from(self.realm_region) << 24)
            | (u32::from(self.realm_battlegroup) << 16)
            | u32::from(self.realm_id)
    }

    pub(crate) fn realm_names_for_address_like_cpp(
        &self,
        realm_address: u32,
    ) -> Option<(&str, &str)> {
        self.realm_names_like_cpp
            .get(&realm_address)
            .map(|(actual, normalized)| (actual.as_str(), normalized.as_str()))
    }

    /// Set the GUID generator for new characters.
    #[cfg(test)]
    pub fn set_guid_generator(&mut self, generator: Arc<ObjectGuidGenerator>) {
        self.guid_generator = Some(generator);
    }

    /// Install the process-wide C++ `sObjectMgr->GenerateVoidStorageItemId()` mirror.
    #[cfg(test)]
    pub fn set_void_storage_item_id_generator_like_cpp(
        &mut self,
        generator: Arc<VoidStorageItemIdGeneratorLikeCpp>,
    ) {
        self.void_storage_item_id_generator_like_cpp = Some(generator);
    }

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

    pub(crate) fn set_realm_list_secret_like_cpp(&mut self, secret: [u8; 32]) {
        self.realm_list_secret_like_cpp = secret;
    }

    pub(crate) fn realm_list_secret_like_cpp(&self) -> &[u8; 32] {
        &self.realm_list_secret_like_cpp
    }

    pub fn set_mute_time_like_cpp(&mut self, mute_time: i64) {
        self.mute_time_like_cpp = mute_time;
    }

    pub(crate) fn can_speak_like_cpp(&self) -> bool {
        self.mute_time_like_cpp <= unix_now()
    }

    pub(crate) fn mute_time_remaining_secs_like_cpp(&self) -> Option<u64> {
        let remaining = self.mute_time_like_cpp.saturating_sub(unix_now());
        (remaining > 0).then_some(remaining as u64)
    }

    pub fn set_recruiter_id_like_cpp(&mut self, recruiter_id: u32) {
        self.recruiter_id_like_cpp = recruiter_id;
    }

    pub(crate) fn recruiter_id_like_cpp(&self) -> u32 {
        self.recruiter_id_like_cpp
    }

    pub fn set_is_a_recruiter_like_cpp(&mut self, is_a_recruiter: bool) {
        self.is_a_recruiter_like_cpp = is_a_recruiter;
    }

    pub(crate) fn is_a_recruiter_like_cpp(&self) -> bool {
        self.is_a_recruiter_like_cpp
    }

    pub(crate) fn session_locale_name_like_cpp(&self) -> &str {
        &self.locale
    }

    /// Get the realm ID.
    pub fn realm_id(&self) -> u16 {
        self.realm_id
    }

    /// Get the GUID generator test fixture.
    #[cfg(test)]
    pub fn guid_generator(&self) -> Option<&Arc<ObjectGuidGenerator>> {
        self.guid_generator.as_ref()
    }

    /// Set the session manager for ConnectTo flow.
    pub fn set_session_mgr(&mut self, mgr: Arc<SessionManager>) {
        self.session_mgr = Some(mgr);
    }

    /// Get the session manager reference.
    pub fn session_mgr(&self) -> Option<&Arc<SessionManager>> {
        self.session_mgr.as_ref()
    }

    pub(crate) fn is_addon_registered_like_cpp(&self, prefix: &str) -> bool {
        // C++ WorldSession::IsAddonRegistered: if the registration filter is
        // disabled (initial state or softcap exceeded), all prefixes pass.
        if !self.filter_addon_messages {
            return true;
        }

        !self.registered_addon_prefixes.is_empty()
            && self
                .registered_addon_prefixes
                .iter()
                .any(|registered| registered == prefix)
    }

    #[cfg(test)]
    pub(crate) fn seed_empty_seasonal_event_bucket_like_cpp(&mut self, event_id: u16) {
        let _ = self.ensure_represented_seasonal_event_like_cpp(event_id);
    }

    /// Set the list of legitimate characters for this account.
    pub fn set_legit_characters(&mut self, guids: Vec<ObjectGuid>) {
        self.legit_characters = guids;
    }

    /// Check if a GUID is in the legit characters list.
    pub fn is_legit_character(&self, guid: &ObjectGuid) -> bool {
        self.legit_characters.contains(guid)
    }

    /// Get the current session state.
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Set the session state (e.g., after character login).
    pub fn set_state(&mut self, state: SessionState) {
        let entered_world = state == SessionState::LoggedIn && self.state != SessionState::LoggedIn;
        self.state = state;
        if entered_world {
            self.apply_represented_ffa_pvp_login_state_like_cpp();
        }
    }

    /// Time since the last packet was received.
    pub fn idle_time(&self) -> std::time::Duration {
        self.last_packet_time.elapsed()
    }

    /// Whether the session is disconnecting.
    pub fn is_disconnecting(&self) -> bool {
        self.state == SessionState::Disconnecting
    }
}
