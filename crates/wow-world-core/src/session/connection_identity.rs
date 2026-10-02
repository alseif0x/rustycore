// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Connection identity data shared with the WorldSession shell.

use std::sync::Arc;

use wow_core::ObjectGuid;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_core::{ObjectGuidGenerator, VoidStorageItemIdGeneratorLikeCpp};
use wow_network::session_mgr::SessionManager;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PacketCounterLikeCpp {
    pub last_receive_time_secs: u64,
    pub amount_counter: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PacketSpoofPendingBanTargetLikeCpp {
    Account { account_id: u32 },
    Ip { address: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketSpoofPendingBanLikeCpp {
    pub target: PacketSpoofPendingBanTargetLikeCpp,
    pub duration_secs: u32,
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

/// Current Unix timestamp (seconds since epoch).
pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

impl crate::session::state::SessionCore {
    pub fn auto_reply_msg_like_cpp(&self) -> Option<String> {
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

    /// Compute the Virtual Realm Address: `(Region << 24) | (Battlegroup << 16) | RealmId`.
    ///
    /// Region and Battlegroup come from the active `realmlist` row, matching C++
    /// `Battlenet::RealmHandle{ realm.Id.Region, realm.Id.Site, realm.Id.Realm }.GetAddress()`.
    pub fn virtual_realm_address(&self) -> u32 {
        (u32::from(self.realm_policy.realm_region) << 24)
            | (u32::from(self.realm_policy.realm_battlegroup) << 16)
            | u32::from(self.realm_id)
    }

    pub fn realm_names_for_address_like_cpp(&self, realm_address: u32) -> Option<(&str, &str)> {
        self.realm_policy
            .realm_names_like_cpp
            .get(&realm_address)
            .map(|(actual, normalized)| (actual.as_str(), normalized.as_str()))
    }

    /// Install the process-wide C++ `sObjectMgr->GenerateVoidStorageItemId()` mirror.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_void_storage_item_id_generator_like_cpp(
        &mut self,
        generator: Arc<VoidStorageItemIdGeneratorLikeCpp>,
    ) {
        self.void_storage_item_id_generator_like_cpp = Some(generator);
    }

    pub fn set_realm_list_secret_like_cpp(&mut self, secret: [u8; 32]) {
        self.realm_policy.realm_list_secret_like_cpp = secret;
    }

    pub fn realm_list_secret_like_cpp(&self) -> &[u8; 32] {
        &self.realm_policy.realm_list_secret_like_cpp
    }

    pub fn can_speak_like_cpp(&self) -> bool {
        self.account_state.mute_time_like_cpp <= unix_now()
    }

    pub fn mute_time_remaining_secs_like_cpp(&self) -> Option<u64> {
        let remaining = self
            .account_state
            .mute_time_like_cpp
            .saturating_sub(unix_now());
        (remaining > 0).then_some(remaining as u64)
    }

    pub fn recruiter_id_like_cpp(&self) -> u32 {
        self.account_state.recruiter_id_like_cpp
    }

    pub fn is_a_recruiter_like_cpp(&self) -> bool {
        self.account_state.is_a_recruiter_like_cpp
    }

    pub fn session_locale_name_like_cpp(&self) -> &str {
        &self.locale
    }

    /// Get the realm ID.
    pub fn realm_id(&self) -> u16 {
        self.realm_id
    }

    /// Get the GUID generator test fixture.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn guid_generator(&self) -> Option<&Arc<ObjectGuidGenerator>> {
        self.guid_generator.as_ref()
    }

    /// Get the session manager reference.
    pub fn session_mgr(&self) -> Option<&Arc<SessionManager>> {
        self.transport.session_mgr.as_ref()
    }

    /// Set the list of legitimate characters for this account.
    pub fn set_legit_characters(&mut self, guids: Vec<ObjectGuid>) {
        self.account_state.legit_characters = guids;
    }

    /// Check if a GUID is in the legit characters list.
    pub fn is_legit_character(&self, guid: &ObjectGuid) -> bool {
        self.account_state.legit_characters.contains(guid)
    }
}
