// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Social packet definitions: SMSG_FRIEND_STATUS, SMSG_CONTACT_LIST.

use crate::{ServerPacket, WorldPacket};
use wow_constants::ServerOpcodes;
use wow_core::ObjectGuid;

/// FriendsResult enum values (byte).
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum FriendsResult {
    DbError      = 0x00,
    ListFull     = 0x01,
    Online       = 0x02,
    Offline      = 0x03,
    NotFound     = 0x04,
    Removed      = 0x05,
    AddedOnline  = 0x06,
    AddedOffline = 0x07,
    Already      = 0x08,
    Self_        = 0x09,
    Enemy        = 0x0A,
}

/// SMSG_FRIEND_STATUS (0x278d)
pub struct FriendStatusPkt {
    pub result: FriendsResult,
    pub guid: ObjectGuid,
    pub account_guid: ObjectGuid,
    pub virtual_realm_address: u32,
    /// 0=offline 1=online 2=AFK 3=DND
    pub status: u8,
    pub area_id: i32,
    pub level: i32,
    pub class_id: u32,
    pub notes: String,
}

impl ServerPacket for FriendStatusPkt {
    const OPCODE: ServerOpcodes = ServerOpcodes::FriendStatus;

    fn write(&self, w: &mut WorldPacket) {
        w.write_uint8(self.result as u8);
        w.write_packed_guid(&self.guid);
        w.write_packed_guid(&self.account_guid);
        w.write_uint32(self.virtual_realm_address);
        w.write_uint8(self.status);
        w.write_int32(self.area_id);
        w.write_int32(self.level);
        w.write_uint32(self.class_id);
        let note_bytes = self.notes.as_bytes();
        w.write_bits(note_bytes.len() as u32, 10);
        w.write_bit(false); // Mobile = false
        w.flush_bits();
        w.write_bytes(note_bytes);
    }
}

/// A single contact entry for SMSG_CONTACT_LIST.
pub struct ContactInfo {
    pub guid: ObjectGuid,
    pub wow_account_guid: ObjectGuid,
    pub virtual_realm_address: u32,
    pub native_realm_address: u32,
    /// SocialFlag: 1=friend, 2=ignored, 4=muted
    pub type_flags: u32,
    pub note: String,
    /// Friend status: 0=offline, 1=online, 2=AFK, 3=DND
    pub status: u8,
    pub area_id: u32,
    pub level: u32,
    pub class_id: u32,
    pub is_mobile: bool,
}

impl ContactInfo {
    pub fn write(&self, w: &mut WorldPacket) {
        w.write_packed_guid(&self.guid);
        w.write_packed_guid(&self.wow_account_guid);
        w.write_uint32(self.virtual_realm_address);
        w.write_uint32(self.native_realm_address);
        w.write_uint32(self.type_flags);
        w.write_uint8(self.status);
        w.write_int32(self.area_id as i32);
        w.write_int32(self.level as i32);
        w.write_uint32(self.class_id);
        let note_bytes = self.note.as_bytes();
        w.write_bits(note_bytes.len() as u32, 10);
        w.write_bit(self.is_mobile);
        w.flush_bits();
        w.write_bytes(note_bytes);
    }
}

/// SMSG_CONTACT_LIST (0x278c)
pub struct ContactListPkt {
    /// SocialFlag bitmask requested
    pub flags: u32,
    pub contacts: Vec<ContactInfo>,
}

impl ServerPacket for ContactListPkt {
    const OPCODE: ServerOpcodes = ServerOpcodes::ContactList;

    fn write(&self, w: &mut WorldPacket) {
        w.write_uint32(self.flags);
        w.write_bits(self.contacts.len() as u32, 8);
        w.flush_bits();
        for c in &self.contacts {
            c.write(w);
        }
    }
}
