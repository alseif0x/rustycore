//! Lfg packets.
//!
//! Separated from world_state.rs under #689.

use super::*;

/// C++ `WorldPackets::LFG::LFGUpdateStatus`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LfgUpdateStatus {
    pub ticket: LfgRideTicket,
    pub sub_type: u8,
    pub reason: u8,
    pub slots: Vec<u32>,
    pub requested_roles: u8,
    pub suspended_players: Vec<ObjectGuid>,
    pub queue_map_id: u32,
    pub notify_ui: bool,
    pub is_party: bool,
    pub joined: bool,
    pub lfg_joined: bool,
    pub queued: bool,
    pub unused: bool,
}

impl LfgUpdateStatus {
    /// C++ `HandleLfgListGetStatus` branch when `sLFGMgr` has no state/ticket.
    pub fn removed_from_queue() -> Self {
        Self {
            ticket: LfgRideTicket::default(),
            sub_type: LFG_QUEUE_DUNGEON_LIKE_CPP,
            reason: LFG_UPDATE_TYPE_REMOVED_FROM_QUEUE_LIKE_CPP,
            slots: Vec::new(),
            requested_roles: 0,
            suspended_players: Vec::new(),
            queue_map_id: 0,
            notify_ui: true,
            is_party: false,
            joined: false,
            lfg_joined: false,
            queued: false,
            unused: false,
        }
    }
}

impl ServerPacket for LfgUpdateStatus {
    const OPCODE: ServerOpcodes = ServerOpcodes::LfgUpdateStatus;

    fn write(&self, pkt: &mut WorldPacket) {
        self.ticket.write_like_cpp(pkt);
        pkt.write_uint8(self.sub_type);
        pkt.write_uint8(self.reason);
        pkt.write_uint32(self.slots.len() as u32);
        pkt.write_uint8(self.requested_roles);
        pkt.write_uint32(self.suspended_players.len() as u32);
        pkt.write_uint32(self.queue_map_id);

        for slot in &self.slots {
            pkt.write_uint32(*slot);
        }

        for suspended_player in &self.suspended_players {
            pkt.write_packed_guid(suspended_player);
        }

        pkt.write_bit(self.is_party);
        pkt.write_bit(self.notify_ui);
        pkt.write_bit(self.joined);
        pkt.write_bit(self.lfg_joined);
        pkt.write_bit(self.queued);
        pkt.write_bit(self.unused);
        pkt.flush_bits();
    }
}

/// C++ `WorldPackets::LFG::LFGListBlacklist::BlacklistEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LfgListBlacklistEntry {
    pub slot: u32,
    pub reason: u32,
    pub sub_reason1: i32,
    pub sub_reason2: i32,
    pub soft_lock: u32,
}

/// C++ `WorldPackets::LFG::LFGListBlacklist`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LfgListBlacklist {
    pub entries: Vec<LfgListBlacklistEntry>,
}

impl LfgListBlacklist {
    pub fn empty() -> Self {
        Self::default()
    }
}

impl ServerPacket for LfgListBlacklist {
    const OPCODE: ServerOpcodes = ServerOpcodes::LfgListUpdateBlacklist;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.entries.len() as u32);
        for entry in &self.entries {
            pkt.write_uint32(entry.slot);
            pkt.write_uint32(entry.reason);
            pkt.write_int32(entry.sub_reason1);
            pkt.write_int32(entry.sub_reason2);
            pkt.write_uint32(entry.soft_lock);
        }
    }
}

/// C++ `WorldPackets::LFG::LFGBlackList`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LfgBlackList {
    pub player_guid: Option<ObjectGuid>,
    pub slots: Vec<LfgListBlacklistEntry>,
}

impl LfgBlackList {
    pub(in crate::packets::misc) fn write_like_cpp(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.player_guid.is_some());
        pkt.write_uint32(self.slots.len() as u32);
        if let Some(player_guid) = self.player_guid {
            pkt.write_packed_guid(&player_guid);
        }
        for slot in &self.slots {
            pkt.write_uint32(slot.slot);
            pkt.write_uint32(slot.reason);
            pkt.write_int32(slot.sub_reason1);
            pkt.write_int32(slot.sub_reason2);
            pkt.write_uint32(slot.soft_lock);
        }
    }
}
