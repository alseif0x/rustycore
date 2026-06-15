// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Handlers for social opcodes: AddFriend, AddIgnore, DelFriend, DelIgnore, SendContactList,
//! SetContactNotes, SocialContractRequest.

use std::sync::Arc;

use tracing::{info, warn};
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::packets::query::{
    NameCacheLookupResult, PlayerGuidLookupData, QueryPlayerNamesResponse,
};
use wow_packet::packets::social::{
    AcceptSocialContract, AccountNotificationAcknowledged, AddIgnore, ContactInfo, ContactListPkt,
    DelIgnore, FriendStatusPkt, FriendsResult, SetContactNotes, SocialContractRequestResponse,
};

use crate::session::{WorldSession, player_team_for_race_cpp};

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
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AddIgnore,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_add_ignore",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DelFriend,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_del_friend",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DelIgnore,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_del_ignore",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SendContactList,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_send_contact_list",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetContactNotes,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_contact_notes",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SocialContractRequest,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_social_contract_request",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AcceptSocialContract,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_accept_social_contract",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AccountNotificationAcknowledged,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_account_notification_acknowledged",
    }
}

// ── handler implementations ───────────────────────────────────────────────────

impl WorldSession {
    fn friend_status_for_guid_like_cpp(&self, guid: ObjectGuid) -> u8 {
        self.player_registry()
            .and_then(|reg| {
                reg.get(&guid).map(|entry| {
                    if entry.is_dnd {
                        FRIEND_STATUS_DND_LIKE_CPP
                    } else if entry.is_afk {
                        FRIEND_STATUS_AFK_LIKE_CPP
                    } else {
                        FRIEND_STATUS_ONLINE_LIKE_CPP
                    }
                })
            })
            .unwrap_or(FRIEND_STATUS_OFFLINE_LIKE_CPP)
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

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
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

        // Lookup friend by name in characters table
        // CAST guid AS SIGNED: sqlx cannot decode BIGINT UNSIGNED as i64 without explicit cast
        let row = sqlx::query(
            "SELECT CAST(guid AS SIGNED), account, race, class, level, zone FROM characters WHERE name = ? LIMIT 1",
        )
        .bind(&name)
        .fetch_optional(char_db.pool())
        .await;

        let row = match row {
            Ok(Some(r)) => r,
            Ok(None) => {
                send_status!(FriendsResult::NotFound, ObjectGuid::EMPTY);
                return;
            }
            Err(e) => {
                warn!("AddFriend DB error looking up '{}': {}", name, e);
                return;
            }
        };

        use sqlx::Row;
        let friend_guid_raw: i64 = row.try_get(0).unwrap_or(0);
        let friend_race: u8 = row.try_get(2).unwrap_or(0);
        let friend_class: u32 = row.try_get::<u8, _>(3).unwrap_or(0) as u32;
        let friend_level: i32 = row.try_get::<u8, _>(4).unwrap_or(0) as i32;
        let friend_zone: i32 = row.try_get::<i32, _>(5).unwrap_or(0);

        let friend_guid = ObjectGuid::create_player(0, friend_guid_raw);

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
        let friend_team = player_team_for_race_cpp(friend_race);
        if player_team != friend_team {
            send_status!(FriendsResult::Enemy, friend_guid);
            return;
        }

        // Check if already a friend
        let already_row = sqlx::query(
            "SELECT COUNT(*) FROM character_social WHERE guid = ? AND friend = ? AND flags & 1",
        )
        .bind(my_guid.counter())
        .bind(friend_guid_raw)
        .fetch_one(char_db.pool())
        .await;

        let already = match already_row {
            Ok(r) => {
                let count: i64 = r.try_get(0).unwrap_or(0);
                count > 0
            }
            Err(_) => false,
        };

        if already {
            send_status!(FriendsResult::Already, friend_guid);
            return;
        }

        let friend_count_row =
            sqlx::query("SELECT COUNT(*) FROM character_social WHERE guid = ? AND flags & 1")
                .bind(my_guid.counter())
                .fetch_one(char_db.pool())
                .await;

        let friend_count = match friend_count_row {
            Ok(r) => r.try_get::<i64, _>(0).unwrap_or(0),
            Err(_) => 0,
        };

        if friend_count >= 50 {
            send_status!(FriendsResult::ListFull, friend_guid);
            return;
        }

        // AddToSocialList ORs the flag into an existing social row; preserve
        // ignore/mute bits instead of dropping this request with INSERT IGNORE.
        let insert = sqlx::query(
            "INSERT INTO character_social (guid, friend, flags, note) VALUES (?, ?, 1, ?) \
             ON DUPLICATE KEY UPDATE flags = flags | 1, note = VALUES(note)",
        )
        .bind(my_guid.counter())
        .bind(friend_guid_raw)
        .bind(&notes)
        .execute(char_db.pool())
        .await;

        if let Err(e) = insert {
            warn!("AddFriend insert error: {}", e);
            return;
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
            area_id: friend_zone,
            level: friend_level,
            class_id: friend_class,
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

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
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

        let row = sqlx::query("SELECT CAST(guid AS SIGNED) FROM characters WHERE name = ? LIMIT 1")
            .bind(&name)
            .fetch_optional(char_db.pool())
            .await;

        let row = match row {
            Ok(Some(row)) => row,
            Ok(None) => {
                send_status!(FriendsResult::IgnoreNotFound, ObjectGuid::EMPTY);
                return;
            }
            Err(e) => {
                warn!("AddIgnore DB error looking up '{}': {}", name, e);
                return;
            }
        };

        use sqlx::Row;
        let ignore_guid_raw: i64 = row.try_get(0).unwrap_or(0);
        let ignore_guid = ObjectGuid::create_player(0, ignore_guid_raw);

        if ignore_guid == my_guid {
            send_status!(FriendsResult::IgnoreSelf, ignore_guid);
            return;
        }

        let already_row = sqlx::query(
            "SELECT COUNT(*) FROM character_social WHERE guid = ? AND friend = ? AND flags & 2",
        )
        .bind(my_guid.counter())
        .bind(ignore_guid_raw)
        .fetch_one(char_db.pool())
        .await;

        let already = match already_row {
            Ok(row) => row.try_get::<i64, _>(0).unwrap_or(0) > 0,
            Err(_) => false,
        };

        if already {
            send_status!(FriendsResult::IgnoreAlready, ignore_guid);
            return;
        }

        let ignore_count_row =
            sqlx::query("SELECT COUNT(*) FROM character_social WHERE guid = ? AND flags & 2")
                .bind(my_guid.counter())
                .fetch_one(char_db.pool())
                .await;

        let ignore_count = match ignore_count_row {
            Ok(row) => row.try_get::<i64, _>(0).unwrap_or(0),
            Err(_) => 0,
        };
        if ignore_count >= 50 {
            send_status!(FriendsResult::IgnoreFull, ignore_guid);
            return;
        }

        let insert = sqlx::query(
            "INSERT INTO character_social (guid, friend, flags, note) VALUES (?, ?, 2, '') \
             ON DUPLICATE KEY UPDATE flags = flags | 2",
        )
        .bind(my_guid.counter())
        .bind(ignore_guid_raw)
        .execute(char_db.pool())
        .await;

        if let Err(e) = insert {
            warn!("AddIgnore insert error: {}", e);
            return;
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

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let update = sqlx::query(
            "UPDATE character_social SET flags = flags & 254 \
             WHERE guid = ? AND friend = ? AND flags & 1",
        )
        .bind(my_guid.counter())
        .bind(friend_guid.counter())
        .execute(char_db.pool())
        .await;

        if let Err(e) = update {
            warn!("DelFriend update error: {}", e);
            return;
        }

        if let Err(e) =
            sqlx::query("DELETE FROM character_social WHERE guid = ? AND friend = ? AND flags = 0")
                .bind(my_guid.counter())
                .bind(friend_guid.counter())
                .execute(char_db.pool())
                .await
        {
            warn!("DelFriend cleanup error: {}", e);
            return;
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

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let target_guid = ignore.player_guid;
        let target_counter = target_guid.counter();

        let update = sqlx::query(
            "UPDATE character_social SET flags = flags & 253 \
             WHERE guid = ? AND friend = ? AND flags & 2",
        )
        .bind(my_guid.counter())
        .bind(target_counter)
        .execute(char_db.pool())
        .await;

        if let Err(e) = update {
            warn!("DelIgnore update error: {}", e);
            return;
        }

        if let Err(e) =
            sqlx::query("DELETE FROM character_social WHERE guid = ? AND friend = ? AND flags = 0")
                .bind(my_guid.counter())
                .bind(target_counter)
                .execute(char_db.pool())
                .await
        {
            warn!("DelIgnore cleanup error: {}", e);
            return;
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

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let note: String = contact.notes.chars().take(48).collect();
        if let Err(e) =
            sqlx::query("UPDATE character_social SET note = ? WHERE guid = ? AND friend = ?")
                .bind(note)
                .bind(my_guid.counter())
                .bind(contact.player_guid.counter())
                .execute(char_db.pool())
                .await
        {
            warn!("SetContactNotes update error: {}", e);
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

        let my_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        // Load all social entries for this character (also fetch name/gender for name cache)
        // CAST ... AS SIGNED: sqlx cannot decode BIGINT UNSIGNED as i64 without explicit cast
        let rows = sqlx::query(
            "SELECT CAST(cs.friend AS SIGNED), cs.flags, cs.note, c.race, c.class, c.level, c.zone, c.name, c.gender \
             FROM character_social cs \
             JOIN characters c ON c.guid = cs.friend \
             WHERE cs.guid = ?",
        )
        .bind(my_guid.counter())
        .fetch_all(char_db.pool())
        .await
        .unwrap_or_default();

        let vra = self.virtual_realm_address();

        struct ContactNameData {
            guid: ObjectGuid,
            name: String,
            race: u8,
            sex: u8,
            class: u8,
            level: u8,
        }

        let mut contacts: Vec<ContactInfo> = Vec::new();
        let mut name_data: Vec<ContactNameData> = Vec::new();

        for row in rows {
            use sqlx::Row;
            let friend_raw: i64 = row.try_get(0).unwrap_or(0);
            let type_flags: u32 = row.try_get::<u8, _>(1).unwrap_or(0) as u32;
            let note: String = row.try_get(2).unwrap_or_default();
            let race: u8 = row.try_get::<u8, _>(3).unwrap_or(0);
            let class_id: u32 = row.try_get::<u8, _>(4).unwrap_or(0) as u32;
            let level: u32 = row.try_get::<u8, _>(5).unwrap_or(0) as u32;
            let zone: u32 = row.try_get::<i32, _>(6).unwrap_or(0) as u32;
            let name: String = row.try_get(7).unwrap_or_default();
            let gender: u8 = row.try_get::<u8, _>(8).unwrap_or(0);

            let friend_guid = ObjectGuid::create_player(0, friend_raw);
            let friend_status = self.friend_status_for_guid_like_cpp(friend_guid);

            name_data.push(ContactNameData {
                guid: friend_guid,
                name,
                race,
                sex: gender,
                class: class_id as u8,
                level: level as u8,
            });

            contacts.push(ContactInfo {
                guid: friend_guid,
                wow_account_guid: ObjectGuid::EMPTY,
                virtual_realm_address: vra,
                native_realm_address: vra,
                type_flags,
                note,
                status: friend_status,
                area_id: zone,
                level,
                class_id,
                is_mobile: false,
            });
        }

        let p = ContactListPkt { flags, contacts };
        self.send_packet(&p);

        // Send player name cache entries so the client can display contact names
        if !name_data.is_empty() {
            let players: Vec<NameCacheLookupResult> = name_data
                .into_iter()
                .map(|nd| NameCacheLookupResult {
                    player: nd.guid,
                    result: 0,
                    data: Some(PlayerGuidLookupData {
                        guid_actual: nd.guid,
                        name: nd.name,
                        race: nd.race,
                        sex: nd.sex,
                        class: nd.class,
                        level: nd.level,
                        virtual_realm_address: vra,
                        ..Default::default()
                    }),
                })
                .collect();

            self.send_packet_realm(&QueryPlayerNamesResponse { players });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::ToPrimitive;
    use wow_constants::ServerOpcodes;

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
}
