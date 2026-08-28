// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Handlers for social opcodes: AddFriend, AddIgnore, DelFriend, DelIgnore, SendContactList,
//! SetContactNotes, SocialContractRequest.

use wow_packet::ClientPacket;

use tracing::{info, warn};
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::packets::social::{
    AcceptSocialContract, AccountNotificationAcknowledged, AddIgnore, ContactInfo, ContactListPkt,
    DelIgnore, FriendStatusPkt, FriendsResult, SetContactNotes, SocialContractRequestResponse,
};

use crate::session::{WorldSession, player_team_for_race_cpp};
use wow_persistence::{
    PersistenceOutcomeLikeCpp, SocialAddCandidateLoadOutcomeLikeCpp,
    SocialContactListLoadOutcomeLikeCpp, SocialRelationshipKindLikeCpp,
};

const FRIEND_STATUS_OFFLINE_LIKE_CPP: u8 = 0x00;
const FRIEND_STATUS_ONLINE_LIKE_CPP: u8 = 0x01;
const FRIEND_STATUS_AFK_LIKE_CPP: u8 = 0x02;
const FRIEND_STATUS_DND_LIKE_CPP: u8 = 0x04;

fn normalize_player_name_like_cpp(name: &str) -> Option<String> {
    let mut lowered = String::new();
    for ch in name.chars() {
        lowered.extend(ch.to_lowercase());
    }

    let mut chars = lowered.chars();
    let first = chars.next()?;
    let mut normalized = String::new();
    normalized.extend(first.to_uppercase());
    normalized.extend(chars);
    Some(normalized)
}

// ── inventory registrations ───────────────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AddFriend,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_add_friend",
        handler: |session, pkt| Box::pin(async move { session.handle_add_friend(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AddIgnore,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_add_ignore",
        handler: |session, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::social::AddIgnore::read(&mut pkt) {
                    Ok(ignore) => session.handle_add_ignore(ignore).await,
                    Err(e) => tracing::warn!("Failed to read AddIgnore: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DelFriend,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_del_friend",
        handler: |session, pkt| Box::pin(async move { session.handle_del_friend(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DelIgnore,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_del_ignore",
        handler: |session, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::social::DelIgnore::read(&mut pkt) {
                    Ok(ignore) => session.handle_del_ignore(ignore).await,
                    Err(e) => tracing::warn!("Failed to read DelIgnore: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SendContactList,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_send_contact_list",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_send_contact_list(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetContactNotes,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_contact_notes",
        handler: |session, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::social::SetContactNotes::read(&mut pkt) {
                    Ok(contact) => session.handle_set_contact_notes(contact).await,
                    Err(e) => tracing::warn!("Failed to read SetContactNotes: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SocialContractRequest,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_social_contract_request",
        handler: |session, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::social::SocialContractRequest::read(&mut pkt) {
                    Ok(_) => session.handle_social_contract_request().await,
                    Err(e) => tracing::warn!("Failed to read SocialContractRequest: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AcceptSocialContract,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_accept_social_contract",
        handler: |session, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::social::AcceptSocialContract::read(&mut pkt) {
                    Ok(accept) => session.handle_accept_social_contract(accept).await,
                    Err(e) => tracing::warn!("Failed to read AcceptSocialContract: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AccountNotificationAcknowledged,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_account_notification_acknowledged",
        handler: |session, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::social::AccountNotificationAcknowledged::read(&mut pkt) {
                    Ok(packet) => session.handle_account_notification_acknowledged(packet).await,
                    Err(e) => tracing::warn!("Failed to read AccountNotificationAcknowledged: {e}"),
                }
            })
        },
    }
}

// ── handler implementations ───────────────────────────────────────────────────

impl WorldSession {
    fn friend_status_for_guid_like_cpp(&self, guid: ObjectGuid) -> u8 {
        self.player_registry()
            .and_then(|registry| {
                registry.social_recipient(guid).map(|recipient| {
                    if recipient.is_dnd {
                        FRIEND_STATUS_DND_LIKE_CPP
                    } else if recipient.is_afk {
                        FRIEND_STATUS_AFK_LIKE_CPP
                    } else {
                        FRIEND_STATUS_ONLINE_LIKE_CPP
                    }
                })
            })
            .unwrap_or(FRIEND_STATUS_OFFLINE_LIKE_CPP)
    }

    pub(crate) async fn send_contact_list_like_cpp(&mut self, flags: u32) {
        let my_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        let port = match self.social_persistence_port_like_cpp() {
            Some(port) => port,
            None => {
                // C++ sends `ContactList` even when the loaded social map is
                // empty; it never follows it with a name-query response.
                self.send_packet_realm(&ContactListPkt {
                    flags,
                    contacts: Vec::new(),
                });
                return;
            }
        };

        // C++ `PlayerSocial::SendSocialList` iterates the loaded social map and
        // writes only entries matching the requested `SocialFlag` bitmask.
        let rows = match port.load_contacts_like_cpp(my_guid.counter(), flags).await {
            SocialContactListLoadOutcomeLikeCpp::Loaded(rows) => rows,
            SocialContactListLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("SendContactList persistence error: {}", reason);
                Vec::new()
            }
        };

        let vra = self.virtual_realm_address();

        let mut contacts: Vec<ContactInfo> = Vec::new();

        for row in rows {
            let friend_guid = ObjectGuid::create_player(0, row.friend_guid);
            let friend_status = self.friend_status_for_guid_like_cpp(friend_guid);

            contacts.push(ContactInfo {
                guid: friend_guid,
                wow_account_guid: ObjectGuid::EMPTY,
                virtual_realm_address: vra,
                native_realm_address: vra,
                type_flags: row.type_flags,
                note: row.note,
                status: friend_status,
                area_id: row.zone_id,
                level: row.level,
                class_id: row.class_id,
                is_mobile: false,
            });
        }

        self.send_packet_realm(&ContactListPkt { flags, contacts });
    }

    /// CMSG_ADD_FRIEND (0x36d8)
    ///
    /// Parse: bits(9)=name_len, bits(9)=notes_len, string(name), string(notes)
    pub async fn handle_add_friend(&mut self, mut pkt: wow_packet::WorldPacket) {
        let name_len = match pkt.read_bits(9) {
            Ok(n) => n as usize,
            Err(e) => {
                warn!("AddFriend: failed to read name_len: {}", e);
                return;
            }
        };
        let notes_len = match pkt.read_bits(9) {
            Ok(n) => n as usize,
            Err(e) => {
                warn!("AddFriend: failed to read notes_len: {}", e);
                return;
            }
        };
        let name = match pkt.read_string(name_len) {
            Ok(s) => s,
            Err(e) => {
                warn!("AddFriend: failed to read name: {}", e);
                return;
            }
        };
        let Some(name) = normalize_player_name_like_cpp(&name) else {
            return;
        };
        let notes = match pkt.read_string(notes_len) {
            Ok(s) => s,
            Err(_) => String::new(),
        };

        let my_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        let port = match self.social_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };

        let vra = self.virtual_realm_address();

        macro_rules! send_status {
            ($result:expr, $guid:expr) => {
                self.send_packet(&FriendStatusPkt {
                    result: $result,
                    guid: $guid,
                    account_guid: ObjectGuid::EMPTY,
                    virtual_realm_address: vra,
                    status: 0,
                    area_id: 0,
                    level: 0,
                    class_id: 0,
                    notes: String::new(),
                });
            };
        }

        let candidate = match port
            .load_add_candidate_like_cpp(name.clone(), SocialRelationshipKindLikeCpp::Friend)
            .await
        {
            SocialAddCandidateLoadOutcomeLikeCpp::Found(candidate) => candidate,
            SocialAddCandidateLoadOutcomeLikeCpp::NotFound => {
                send_status!(FriendsResult::NotFound, ObjectGuid::EMPTY);
                return;
            }
            SocialAddCandidateLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("AddFriend DB error looking up '{}': {}", name, reason);
                return;
            }
        };
        let friend_guid = ObjectGuid::create_player(0, candidate.guid);

        // Can't add yourself
        if friend_guid == my_guid {
            send_status!(FriendsResult::Self_, friend_guid);
            return;
        }

        // C++: WorldSession::HandleAddFriendOpcode rejects enemy-faction
        // contacts unless RBAC_PERM_TWO_SIDE_ADD_FRIEND is present. RustyCore
        // does not yet have AccountMgr/RBAC runtime, so normal-player behavior
        // is represented conservatively and the GM bypass remains a tracked gap.
        let player_team = player_team_for_race_cpp(self.player_race_like_cpp());
        let friend_team = player_team_for_race_cpp(candidate.race);
        if player_team != friend_team {
            send_status!(FriendsResult::Enemy, friend_guid);
            return;
        }

        let relationship = port
            .load_relationship_state_like_cpp(
                my_guid.counter(),
                candidate.guid,
                SocialRelationshipKindLikeCpp::Friend,
            )
            .await;
        if relationship.already_present {
            send_status!(FriendsResult::Already, friend_guid);
            return;
        }
        if relationship.relationship_count >= 50 {
            send_status!(FriendsResult::ListFull, friend_guid);
            return;
        }

        // AddToSocialList ORs the flag into an existing social row; preserve
        // ignore/mute bits instead of dropping this request with INSERT IGNORE.
        match port
            .add_relationship_like_cpp(
                my_guid.counter(),
                candidate.guid,
                SocialRelationshipKindLikeCpp::Friend,
                notes.clone(),
            )
            .await
        {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason }
            | PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!("AddFriend insert error: {}", reason);
                return;
            }
        }

        // Is friend online? Check player registry.
        let friend_status = self.friend_status_for_guid_like_cpp(friend_guid);
        let is_online = friend_status != FRIEND_STATUS_OFFLINE_LIKE_CPP;

        let result = if is_online {
            FriendsResult::AddedOnline
        } else {
            FriendsResult::AddedOffline
        };

        let p = FriendStatusPkt {
            result,
            guid: friend_guid,
            account_guid: ObjectGuid::EMPTY,
            virtual_realm_address: vra,
            status: friend_status,
            area_id: candidate.zone_id,
            level: candidate.level,
            class_id: candidate.class_id,
            notes: notes.clone(),
        };
        self.send_packet(&p);
        info!(
            "Player {:?} added friend {:?} ({})",
            my_guid, friend_guid, name
        );
    }

    /// Handle CMSG_ADD_IGNORE.
    ///
    /// C++ ref: `WorldSession::HandleAddIgnoreOpcode`.
    ///
    /// This represents the per-character ignore list (`SOCIAL_FLAG_IGNORED`).
    /// Account-level ignore remains parked until Rust owns `character_social.accountGuid`
    /// and an in-memory `PlayerSocial::_ignoredAccounts` equivalent.
    pub async fn handle_add_ignore(&mut self, ignore: AddIgnore) {
        let my_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        let port = match self.social_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };

        let vra = self.virtual_realm_address();
        macro_rules! send_status {
            ($result:expr, $guid:expr) => {
                self.send_packet(&FriendStatusPkt {
                    result: $result,
                    guid: $guid,
                    account_guid: ObjectGuid::EMPTY,
                    virtual_realm_address: vra,
                    status: 0,
                    area_id: 0,
                    level: 0,
                    class_id: 0,
                    notes: String::new(),
                });
            };
        }

        let Some(name) = normalize_player_name_like_cpp(&ignore.name) else {
            return;
        };

        let candidate = match port
            .load_add_candidate_like_cpp(name.clone(), SocialRelationshipKindLikeCpp::Ignored)
            .await
        {
            SocialAddCandidateLoadOutcomeLikeCpp::Found(candidate) => candidate,
            SocialAddCandidateLoadOutcomeLikeCpp::NotFound => {
                send_status!(FriendsResult::IgnoreNotFound, ObjectGuid::EMPTY);
                return;
            }
            SocialAddCandidateLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("AddIgnore DB error looking up '{}': {}", name, reason);
                return;
            }
        };
        let ignore_guid = ObjectGuid::create_player(0, candidate.guid);

        if ignore_guid == my_guid {
            send_status!(FriendsResult::IgnoreSelf, ignore_guid);
            return;
        }

        let relationship = port
            .load_relationship_state_like_cpp(
                my_guid.counter(),
                candidate.guid,
                SocialRelationshipKindLikeCpp::Ignored,
            )
            .await;
        if relationship.already_present {
            send_status!(FriendsResult::IgnoreAlready, ignore_guid);
            return;
        }
        if relationship.relationship_count >= 50 {
            send_status!(FriendsResult::IgnoreFull, ignore_guid);
            return;
        }
        match port
            .add_relationship_like_cpp(
                my_guid.counter(),
                candidate.guid,
                SocialRelationshipKindLikeCpp::Ignored,
                String::new(),
            )
            .await
        {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason }
            | PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!("AddIgnore insert error: {}", reason);
                return;
            }
        }

        send_status!(FriendsResult::IgnoreAdded, ignore_guid);
        info!("Player {:?} ignored {:?} ({})", my_guid, ignore_guid, name);
    }

    /// CMSG_DEL_FRIEND (0x36d9)
    ///
    /// Parse: QualifiedGUID = packed_guid + u32 realm
    pub async fn handle_del_friend(&mut self, mut pkt: wow_packet::WorldPacket) {
        let friend_guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(e) => {
                warn!("DelFriend: failed to read guid: {}", e);
                return;
            }
        };
        // VirtualRealmAddress — ignored
        let _ = pkt.read_uint32();

        let my_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        let port = match self.social_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };
        match port
            .remove_relationship_like_cpp(
                my_guid.counter(),
                friend_guid.counter(),
                SocialRelationshipKindLikeCpp::Friend,
            )
            .await
        {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason }
            | PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!("DelFriend persistence error: {}", reason);
                return;
            }
        }

        let p = FriendStatusPkt {
            result: FriendsResult::Removed,
            guid: friend_guid,
            account_guid: ObjectGuid::EMPTY,
            virtual_realm_address: self.virtual_realm_address(),
            status: 0,
            area_id: 0,
            level: 0,
            class_id: 0,
            notes: String::new(),
        };
        self.send_packet(&p);
    }

    /// Handle CMSG_DEL_IGNORE.
    ///
    /// C++ ref: `WorldSession::HandleDelIgnoreOpcode` delegates to
    /// `PlayerSocial::RemoveFromSocialList(..., SOCIAL_FLAG_IGNORED)`, which
    /// clears only the ignored bit and deletes the row only when no social flags
    /// remain.
    pub async fn handle_del_ignore(&mut self, ignore: DelIgnore) {
        let my_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        let port = match self.social_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };

        let target_guid = ignore.player_guid;
        let target_counter = target_guid.counter();

        match port
            .remove_relationship_like_cpp(
                my_guid.counter(),
                target_counter,
                SocialRelationshipKindLikeCpp::Ignored,
            )
            .await
        {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason }
            | PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!("DelIgnore persistence error: {}", reason);
                return;
            }
        }

        self.send_packet(&FriendStatusPkt {
            result: FriendsResult::IgnoreRemoved,
            guid: target_guid,
            account_guid: ObjectGuid::EMPTY,
            virtual_realm_address: self.virtual_realm_address(),
            status: 0,
            area_id: 0,
            level: 0,
            class_id: 0,
            notes: String::new(),
        });
    }

    /// Handle CMSG_SET_CONTACT_NOTES.
    ///
    /// C++ ref: `WorldSession::HandleSetContactNotesOpcode` delegates to
    /// `PlayerSocial::SetFriendNote`, which silently returns if the contact is
    /// not present and truncates the stored note to 48 UTF-8 chars.
    pub async fn handle_set_contact_notes(&mut self, contact: SetContactNotes) {
        let my_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        let port = match self.social_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };

        let note: String = contact.notes.chars().take(48).collect();
        match port
            .set_contact_note_like_cpp(my_guid.counter(), contact.player_guid.counter(), note)
            .await
        {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason }
            | PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!("SetContactNotes update error: {}", reason);
            }
        }
    }

    /// Handle CMSG_SOCIAL_CONTRACT_REQUEST.
    ///
    /// C++ ref: `WorldSession::HandleSocialContractRequest` sends a
    /// `SocialContractRequestResponse` with `ShowSocialContract = false`.
    pub async fn handle_social_contract_request(&mut self) {
        self.send_packet(&SocialContractRequestResponse {
            show_social_contract: false,
        });
    }

    /// Handle CMSG_ACCEPT_SOCIAL_CONTRACT.
    ///
    /// C++ ref: `WorldSession::HandleAcceptSocialContract` currently logs the
    /// acceptance and leaves account-data persistence as a future hook.
    pub async fn handle_accept_social_contract(&mut self, _accept: AcceptSocialContract) {
        // Account-data persistence remains parked until Rust owns the account
        // data layer. Matching current C++ behavior here means no response.
    }

    /// Handle CMSG_ACCOUNT_NOTIFICATION_ACKNOWLEDGED.
    ///
    /// C++ ref: `WorldSession::HandleAccountNotificationAcknowledged` logs the
    /// notification id and leaves DB read-state persistence as a future hook.
    pub async fn handle_account_notification_acknowledged(
        &mut self,
        _packet: AccountNotificationAcknowledged,
    ) {
        // Matching current C++ behavior here means no response and no state
        // mutation; account-notification persistence is not implemented there.
    }

    /// CMSG_SEND_CONTACT_LIST (0x36d7)
    ///
    /// Parse: u32 flags (SocialFlag bitmask)
    pub async fn handle_send_contact_list(&mut self, mut pkt: wow_packet::WorldPacket) {
        let flags = match pkt.read_uint32() {
            Ok(f) => f,
            Err(e) => {
                warn!("SendContactList: failed to read flags: {}", e);
                return;
            }
        };

        self.send_contact_list_like_cpp(flags).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::ToPrimitive;
    use std::sync::{Arc, Mutex};
    use wow_constants::ServerOpcodes;
    use wow_persistence::{
        PersistenceFutureLikeCpp, SocialAddCandidateLoadOutcomeLikeCpp,
        SocialContactLoadRowLikeCpp, SocialPartyInviteLookupOutcomeLikeCpp,
        SocialPersistencePortLikeCpp, SocialRelationshipStateLikeCpp,
    };

    struct RecordingSocialPort {
        contacts: SocialContactListLoadOutcomeLikeCpp,
        mutation: PersistenceOutcomeLikeCpp,
        calls: Mutex<Vec<String>>,
    }

    impl SocialPersistencePortLikeCpp for RecordingSocialPort {
        fn load_contacts_like_cpp<'a>(
            &'a self,
            player_guid: i64,
            flags: u32,
        ) -> PersistenceFutureLikeCpp<'a, SocialContactListLoadOutcomeLikeCpp> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("load:{player_guid}:{flags}"));
            let outcome = self.contacts.clone();
            Box::pin(async move { outcome })
        }

        fn load_add_candidate_like_cpp<'a>(
            &'a self,
            _normalized_name: String,
            _kind: SocialRelationshipKindLikeCpp,
        ) -> PersistenceFutureLikeCpp<'a, SocialAddCandidateLoadOutcomeLikeCpp> {
            Box::pin(async { SocialAddCandidateLoadOutcomeLikeCpp::NotFound })
        }

        fn load_relationship_state_like_cpp<'a>(
            &'a self,
            _player_guid: i64,
            _target_guid: i64,
            _kind: SocialRelationshipKindLikeCpp,
        ) -> PersistenceFutureLikeCpp<'a, SocialRelationshipStateLikeCpp> {
            Box::pin(async {
                SocialRelationshipStateLikeCpp {
                    already_present: false,
                    relationship_count: 0,
                }
            })
        }

        fn party_invite_target_ignores_like_cpp<'a>(
            &'a self,
            _target_guid: i64,
            _inviter_guid: i64,
            _inviter_account_id: u32,
        ) -> PersistenceFutureLikeCpp<'a, SocialPartyInviteLookupOutcomeLikeCpp> {
            Box::pin(async { SocialPartyInviteLookupOutcomeLikeCpp::Resolved(false) })
        }

        fn party_invite_target_has_friend_like_cpp<'a>(
            &'a self,
            _target_guid: i64,
            _inviter_guid: i64,
        ) -> PersistenceFutureLikeCpp<'a, SocialPartyInviteLookupOutcomeLikeCpp> {
            Box::pin(async { SocialPartyInviteLookupOutcomeLikeCpp::Resolved(false) })
        }

        fn add_relationship_like_cpp<'a>(
            &'a self,
            _player_guid: i64,
            _target_guid: i64,
            _kind: SocialRelationshipKindLikeCpp,
            _note: String,
        ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
            let outcome = self.mutation.clone();
            Box::pin(async move { outcome })
        }

        fn remove_relationship_like_cpp<'a>(
            &'a self,
            player_guid: i64,
            target_guid: i64,
            kind: SocialRelationshipKindLikeCpp,
        ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("remove:{player_guid}:{target_guid}:{kind:?}"));
            let outcome = self.mutation.clone();
            Box::pin(async move { outcome })
        }

        fn set_contact_note_like_cpp<'a>(
            &'a self,
            _player_guid: i64,
            _target_guid: i64,
            _note: String,
        ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
            let outcome = self.mutation.clone();
            Box::pin(async move { outcome })
        }
    }

    fn recording_port(
        contacts: SocialContactListLoadOutcomeLikeCpp,
        mutation: PersistenceOutcomeLikeCpp,
    ) -> Arc<RecordingSocialPort> {
        Arc::new(RecordingSocialPort {
            contacts,
            mutation,
            calls: Mutex::new(Vec::new()),
        })
    }

    fn make_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
        let (_pkt_tx, pkt_rx) = flume::bounded(8);
        let (send_tx, send_rx) = flume::bounded(8);
        (
            WorldSession::new(
                1,
                "SocialTest".into(),
                0,
                2,
                9,
                54261,
                vec![0; 40],
                "enUS".into(),
                pkt_rx,
                send_tx,
            ),
            send_rx,
        )
    }

    fn opcode(bytes: &[u8]) -> u16 {
        u16::from_le_bytes([bytes[0], bytes[1]])
    }

    #[tokio::test]
    async fn social_contract_request_sends_false_response_like_cpp() {
        let (mut session, send_rx) = make_session();

        session.handle_social_contract_request().await;

        let bytes = send_rx.try_recv().expect("social contract response");
        assert_eq!(
            opcode(&bytes),
            ServerOpcodes::SocialContractRequestResponse
                .to_u16()
                .expect("opcode")
        );
        assert_eq!(bytes.last().copied(), Some(0));
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn accept_social_contract_is_no_response_like_cpp() {
        let (mut session, send_rx) = make_session();

        session
            .handle_accept_social_contract(AcceptSocialContract)
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn account_notification_acknowledged_is_no_response_like_cpp() {
        let (mut session, send_rx) = make_session();

        session
            .handle_account_notification_acknowledged(AccountNotificationAcknowledged {
                notification_id: 42,
            })
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn empty_contact_list_sends_only_contact_list_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

        session.send_contact_list_like_cpp(7).await;

        let bytes = send_rx.try_recv().expect("empty contact list");
        assert_eq!(
            opcode(&bytes),
            ServerOpcodes::ContactList.to_u16().expect("opcode")
        );
        let mut body = wow_packet::WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(body.read_uint32().unwrap(), 7);
        assert_eq!(body.read_bits(8).unwrap(), 0);
        assert_eq!(body.remaining(), 0);
        assert!(
            send_rx.try_recv().is_err(),
            "C++ PlayerSocial::SendSocialList does not inject QueryPlayerNamesResponse"
        );
    }

    #[tokio::test]
    async fn contact_list_uses_the_sqlx_free_port_and_projects_loaded_rows() {
        let (mut session, send_rx) = make_session();
        session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
        let port = recording_port(
            SocialContactListLoadOutcomeLikeCpp::Loaded(vec![SocialContactLoadRowLikeCpp {
                friend_guid: 77,
                type_flags: 1,
                note: "raid".into(),
                class_id: 8,
                level: 80,
                zone_id: 1519,
            }]),
            PersistenceOutcomeLikeCpp::Applied { rows: 1 },
        );
        session.set_social_persistence_port_like_cpp(port.clone());

        session.send_contact_list_like_cpp(1).await;

        let bytes = send_rx.try_recv().expect("contact list");
        assert_eq!(opcode(&bytes), ServerOpcodes::ContactList.to_u16().unwrap());
        assert_eq!(port.calls.lock().unwrap().as_slice(), ["load:42:1"]);
    }

    #[tokio::test]
    async fn failed_remove_does_not_publish_the_friend_status_packet() {
        let (mut session, send_rx) = make_session();
        session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
        let port = recording_port(
            SocialContactListLoadOutcomeLikeCpp::Loaded(Vec::new()),
            PersistenceOutcomeLikeCpp::Failed {
                reason: "write failed".into(),
            },
        );
        session.set_social_persistence_port_like_cpp(port.clone());

        session
            .handle_del_ignore(DelIgnore {
                player_guid: ObjectGuid::create_player(1, 77),
                virtual_realm_address: 0,
            })
            .await;

        assert!(send_rx.try_recv().is_err());
        assert_eq!(
            port.calls.lock().unwrap().as_slice(),
            ["remove:42:77:Ignored"]
        );
    }

    #[tokio::test]
    async fn committed_remove_publishes_after_the_port_returns() {
        let (mut session, send_rx) = make_session();
        session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
        let port = recording_port(
            SocialContactListLoadOutcomeLikeCpp::Loaded(Vec::new()),
            PersistenceOutcomeLikeCpp::Applied { rows: 2 },
        );
        session.set_social_persistence_port_like_cpp(port);

        session
            .handle_del_ignore(DelIgnore {
                player_guid: ObjectGuid::create_player(1, 77),
                virtual_realm_address: 0,
            })
            .await;

        let bytes = send_rx.try_recv().expect("remove status");
        assert_eq!(
            opcode(&bytes),
            ServerOpcodes::FriendStatus.to_u16().unwrap()
        );
    }

    #[test]
    fn normalize_player_name_empty_rejects_like_cpp() {
        assert_eq!(normalize_player_name_like_cpp(""), None);
    }

    #[test]
    fn normalize_player_name_capitalizes_first_and_lowers_rest_like_cpp() {
        assert_eq!(
            normalize_player_name_like_cpp("tHrAlL").as_deref(),
            Some("Thrall")
        );
        assert_eq!(
            normalize_player_name_like_cpp("jaina").as_deref(),
            Some("Jaina")
        );
    }

    #[test]
    fn normalize_player_name_handles_unicode_case_like_cpp_wide_string_path() {
        assert_eq!(
            normalize_player_name_like_cpp("éLUNE").as_deref(),
            Some("Élune")
        );
    }

    #[test]
    fn del_ignore_dispatch_metadata_matches_cpp() {
        let entry = inventory::iter::<PacketHandlerEntry>
            .into_iter()
            .find(|entry| entry.opcode == ClientOpcodes::DelIgnore)
            .expect("DelIgnore handler entry");

        assert_eq!(entry.status, SessionStatus::LoggedIn);
        assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
        assert_eq!(entry.handler_name, "handle_del_ignore");
    }

    #[test]
    fn social_handler_source_cannot_reacquire_concrete_persistence() {
        let source = include_str!("social.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("production source prefix");
        for forbidden in [
            "sqlx::",
            "CharacterDatabase",
            "CharStatements",
            ".pool()",
            "self.char_db()",
            "SELECT ",
            "INSERT INTO character_social",
            "UPDATE character_social",
            "DELETE FROM character_social",
        ] {
            assert!(
                !source.contains(forbidden),
                "social handler reacquired concrete persistence syntax: {forbidden}"
            );
        }
    }
}
