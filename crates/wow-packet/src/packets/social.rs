// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Social packet definitions: SMSG_FRIEND_STATUS, SMSG_CONTACT_LIST.

use crate::{ClientPacket, PacketError, ServerPacket, WorldPacket};
use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::ObjectGuid;

/// FriendsResult enum values (byte).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FriendsResult {
    DbError = 0x00,
    ListFull = 0x01,
    Online = 0x02,
    Offline = 0x03,
    NotFound = 0x04,
    Removed = 0x05,
    AddedOnline = 0x06,
    AddedOffline = 0x07,
    Already = 0x08,
    Self_ = 0x09,
    Enemy = 0x0A,
    IgnoreFull = 0x0B,
    IgnoreSelf = 0x0C,
    IgnoreNotFound = 0x0D,
    IgnoreAlready = 0x0E,
    IgnoreAdded = 0x0F,
    IgnoreRemoved = 0x10,
    IgnoreAmbiguous = 0x11,
    MuteFull = 0x12,
    MuteSelf = 0x13,
    MuteNotFound = 0x14,
    MuteAlready = 0x15,
    MuteAdded = 0x16,
    MuteRemoved = 0x17,
    MuteAmbiguous = 0x18,
    Unknown = 0x1C,
}

/// CMSG_ADD_IGNORE.
///
/// C++ `WorldPackets::Social::AddIgnore::Read` reads a 9-bit name length,
/// then an account GUID, then the name string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddIgnore {
    pub name: String,
    pub account_guid: ObjectGuid,
}

impl ClientPacket for AddIgnore {
    const OPCODE: ClientOpcodes = ClientOpcodes::AddIgnore;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let name_len = packet.read_bits(9)? as usize;
        let account_guid = packet.read_packed_guid()?;
        let name = packet.read_string(name_len)?;
        Ok(Self { name, account_guid })
    }
}

/// CMSG_DEL_IGNORE.
///
/// C++ `WorldPackets::Social::DelIgnore::Read` reads a `QualifiedGUID`
/// (`ObjectGuid` plus virtual realm address).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelIgnore {
    pub player_guid: ObjectGuid,
    pub virtual_realm_address: u32,
}

impl ClientPacket for DelIgnore {
    const OPCODE: ClientOpcodes = ClientOpcodes::DelIgnore;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let player_guid = packet.read_packed_guid()?;
        let virtual_realm_address = packet.read_uint32()?;
        Ok(Self {
            player_guid,
            virtual_realm_address,
        })
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_ignore_reads_cpp_name_length_account_guid_name_order() {
        let account_guid = ObjectGuid::create_player(1, 0xAABBCC);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bits(6, 9);
        pkt.write_packed_guid(&account_guid);
        pkt.write_string("Thrall");

        let parsed = AddIgnore::read(&mut pkt).expect("add ignore packet");

        assert_eq!(parsed.account_guid, account_guid);
        assert_eq!(parsed.name, "Thrall");
        assert!(pkt.is_empty());
    }

    #[test]
    fn friends_result_ignore_values_match_cpp_social_mgr() {
        assert_eq!(FriendsResult::IgnoreFull as u8, 0x0B);
        assert_eq!(FriendsResult::IgnoreSelf as u8, 0x0C);
        assert_eq!(FriendsResult::IgnoreNotFound as u8, 0x0D);
        assert_eq!(FriendsResult::IgnoreAlready as u8, 0x0E);
        assert_eq!(FriendsResult::IgnoreAdded as u8, 0x0F);
        assert_eq!(FriendsResult::IgnoreRemoved as u8, 0x10);
    }

    #[test]
    fn del_ignore_reads_cpp_qualified_guid_order() {
        let player_guid = ObjectGuid::create_player(1, 0x00CCBBAA);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&player_guid);
        pkt.write_uint32(0xAABBCCDD);

        let parsed = DelIgnore::read(&mut pkt).expect("del ignore packet");

        assert_eq!(parsed.player_guid, player_guid);
        assert_eq!(parsed.virtual_realm_address, 0xAABBCCDD);
        assert!(pkt.is_empty());
    }
}
