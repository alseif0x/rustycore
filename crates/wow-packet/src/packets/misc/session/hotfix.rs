//! Hotfix packets.
//!
//! Separated from session.rs under #689.

use super::*;

// ── AvailableHotfixes (SMSG 0x290f) ────────────────────────────────

/// C++ `DB2Manager::HotfixId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HotfixId {
    pub push_id: i32,
    pub unique_id: u32,
}

/// Available hotfixes sent during session init.
pub struct AvailableHotfixes {
    pub virtual_realm_address: u32,
    pub hotfixes: Vec<HotfixId>,
}

impl ServerPacket for AvailableHotfixes {
    const OPCODE: ServerOpcodes = ServerOpcodes::AvailableHotfixes;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.virtual_realm_address);
        pkt.write_uint32(self.hotfixes.len() as u32);
        for hotfix_id in &self.hotfixes {
            pkt.write_int32(hotfix_id.push_id);
            pkt.write_uint32(hotfix_id.unique_id);
        }
    }
}

// ── AuraUpdate (SMSG 0x2c1f) ─────────────────────────────────────────

/// Client request for hotfix data after receiving [`AvailableHotfixes`].
pub struct HotfixRequest {
    pub client_build: u32,
    pub data_build: u32,
    pub hotfixes: Vec<i32>,
}

impl ClientPacket for HotfixRequest {
    const OPCODE: ClientOpcodes = ClientOpcodes::HotfixRequest;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let client_build = packet.read_uint32()?;
        let data_build = packet.read_uint32()?;
        let count = packet.read_uint32()? as usize;
        let mut hotfixes = Vec::with_capacity(count.min(8192));
        for _ in 0..count {
            hotfixes.push(packet.read_int32()?);
        }
        Ok(Self {
            client_build,
            data_build,
            hotfixes,
        })
    }
}

// ── HotfixConnect (SMSG 0x2911) ───────────────────────────────────

/// One C++ `WorldPackets::Hotfix::HotfixConnect::HotfixData` header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotfixConnectData {
    pub id: HotfixId,
    pub table_hash: u32,
    pub record_id: i32,
    pub size: u32,
    pub status: u8,
}

/// Response to [`HotfixRequest`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HotfixConnect {
    pub hotfixes: Vec<HotfixConnectData>,
    pub content: Vec<u8>,
}

impl HotfixConnect {
    pub fn empty() -> Self {
        Self::default()
    }
}

impl ServerPacket for HotfixConnect {
    const OPCODE: ServerOpcodes = ServerOpcodes::HotfixConnect;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.hotfixes.len() as u32);
        for hotfix in &self.hotfixes {
            pkt.write_int32(hotfix.id.push_id);
            pkt.write_uint32(hotfix.id.unique_id);
            pkt.write_uint32(hotfix.table_hash);
            pkt.write_int32(hotfix.record_id);
            pkt.write_uint32(hotfix.size);
            pkt.write_bits(u32::from(hotfix.status), 3);
            pkt.flush_bits();
        }

        pkt.write_uint32(self.content.len() as u32);
        if !self.content.is_empty() {
            pkt.write_bytes(&self.content);
        }
    }
}
