// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Handlers for social opcodes: AddFriend, DelFriend, SendContactList.

use std::sync::Arc;

use tracing::{info, warn};
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::packets::query::{NameCacheLookupResult, PlayerGuidLookupData, QueryPlayerNamesResponse};
use wow_packet::packets::social::{ContactInfo, ContactListPkt, FriendStatusPkt, FriendsResult};
use wow_packet::ServerPacket;

use crate::session::WorldSession;

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
        opcode: ClientOpcodes::DelFriend,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_del_friend",
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

// ── handler implementations ───────────────────────────────────────────────────

impl WorldSession {
    /// CMSG_ADD_FRIEND (0x36d8)
    ///
    /// Parse: bits(9)=name_len, bits(9)=notes_len, string(name), string(notes)
    pub async fn handle_add_friend(&mut self, mut pkt: wow_packet::WorldPacket) {
        let name_len = match pkt.read_bits(9) {
            Ok(n) => n as usize,
            Err(e) => { warn!("AddFriend: failed to read name_len: {}", e); return; }
        };
        let notes_len = match pkt.read_bits(9) {
            Ok(n) => n as usize,
            Err(e) => { warn!("AddFriend: failed to read notes_len: {}", e); return; }
        };
        let name = match pkt.read_string(name_len) {
            Ok(s) => s,
            Err(e) => { warn!("AddFriend: failed to read name: {}", e); return; }
        };
        let notes = match pkt.read_string(notes_len) {
            Ok(s) => s,
            Err(_) => String::new(),
        };

        let my_guid = match self.player_guid {
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
        let friend_class: u32 = row.try_get::<u8, _>(3).unwrap_or(0) as u32;
        let friend_level: i32 = row.try_get::<u8, _>(4).unwrap_or(0) as i32;
        let friend_zone: i32 = row.try_get::<i32, _>(5).unwrap_or(0);

        let friend_guid = ObjectGuid::create_player(0, friend_guid_raw);

        // Can't add yourself
        if friend_guid == my_guid {
            send_status!(FriendsResult::Self_, ObjectGuid::EMPTY);
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

        // Insert into character_social (flags=1 = SOCIAL_FLAG_FRIEND)
        let insert = sqlx::query(
            "INSERT IGNORE INTO character_social (guid, friend, flags, note) VALUES (?, ?, 1, ?)",
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

        // Is friend online? Check player registry
        let is_online = self
            .player_registry()
            .map(|reg| reg.contains_key(&friend_guid))
            .unwrap_or(false);

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
            status: if is_online { 1 } else { 0 },
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

    /// CMSG_DEL_FRIEND (0x36d9)
    ///
    /// Parse: QualifiedGUID = packed_guid + u32 realm
    pub async fn handle_del_friend(&mut self, mut pkt: wow_packet::WorldPacket) {
        let friend_guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(e) => { warn!("DelFriend: failed to read guid: {}", e); return; }
        };
        // VirtualRealmAddress — ignored
        let _ = pkt.read_uint32();

        let my_guid = match self.player_guid {
            Some(g) => g,
            None => return,
        };

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let _ = sqlx::query(
            "DELETE FROM character_social WHERE guid = ? AND friend = ? AND flags & 1",
        )
        .bind(my_guid.counter())
        .bind(friend_guid.counter())
        .execute(char_db.pool())
        .await;

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

    /// CMSG_SEND_CONTACT_LIST (0x36d7)
    ///
    /// Parse: u32 flags (SocialFlag bitmask)
    pub async fn handle_send_contact_list(&mut self, mut pkt: wow_packet::WorldPacket) {
        let flags = match pkt.read_uint32() {
            Ok(f) => f,
            Err(e) => { warn!("SendContactList: failed to read flags: {}", e); return; }
        };

        let my_guid = match self.player_guid {
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
            let is_online = self
                .player_registry()
                .map(|r| r.contains_key(&friend_guid))
                .unwrap_or(false);

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
                status: if is_online { 1 } else { 0 },
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
