//! Client state packets.
//!
//! Separated from session.rs under #689.

use super::*;

/// C++ `WorldPackets::Misc::AddonList`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonList {
    pub addons: Vec<String>,
}

impl ClientPacket for AddonList {
    const OPCODE: ClientOpcodes = ClientOpcodes::AddonList;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let count = pkt.read_uint32()?;
        let mut addons = Vec::new();

        for _ in 0..count {
            if pkt.remaining() == 0 {
                break;
            }

            let name_len = pkt.read_bits(10)? as usize;
            pkt.flush_bits();
            addons.push(pkt.read_string(name_len)?);
        }

        Ok(Self { addons })
    }
}

/// C++ `WorldPackets::Character::LoadingScreenNotify`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadingScreenNotify {
    pub map_id: u32,
    pub showing: bool,
}

impl ClientPacket for LoadingScreenNotify {
    const OPCODE: ClientOpcodes = ClientOpcodes::LoadingScreenNotify;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            map_id: pkt.read_uint32()?,
            showing: pkt.read_bit()?,
        })
    }
}

/// C++ `WorldPackets::Misc::RandomRollClient`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RandomRollClient {
    pub min: i32,
    pub max: i32,
    pub party_index: Option<u8>,
}

impl ClientPacket for RandomRollClient {
    const OPCODE: ClientOpcodes = ClientOpcodes::RandomRoll;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let has_party_index = pkt.read_bit()?;
        let min = pkt.read_int32()?;
        let max = pkt.read_int32()?;
        let party_index = if has_party_index {
            Some(pkt.read_uint8()?)
        } else {
            None
        };
        Ok(Self {
            min,
            max,
            party_index,
        })
    }
}

/// C++ `WorldPackets::Misc::CloseInteraction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloseInteraction {
    pub source_guid: ObjectGuid,
}

impl ClientPacket for CloseInteraction {
    const OPCODE: ClientOpcodes = ClientOpcodes::CloseInteraction;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            source_guid: pkt.read_packed_guid()?,
        })
    }
}

// ── UpdateWorldState (SMSG 0x2748) ──────────────────────────────────

/// C++ `WorldPackets::Misc::SetAIAnimKit`: ObjectGuid + uint16 AnimKitID.
pub struct SetAiAnimKit {
    pub unit: ObjectGuid,
    pub anim_kit_id: u16,
}

impl ServerPacket for SetAiAnimKit {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetAiAnimKit;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.unit.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_uint16(self.anim_kit_id);
    }
}

/// C++ `WorldPackets::Misc::SetMeleeAnimKit`: ObjectGuid + uint16 AnimKitID.
pub struct SetMeleeAnimKit {
    pub unit: ObjectGuid,
    pub anim_kit_id: u16,
}

impl ServerPacket for SetMeleeAnimKit {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetMeleeAnimKit;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.unit.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_uint16(self.anim_kit_id);
    }
}

// ── UpdateCapturePoint (SMSG 0xbadd) ───────────────────────────────

/// Starts a cinematic sequence for the player.
pub struct TriggerCinematic {
    pub cinematic_id: u32,
    pub conversation_guid: ObjectGuid,
}

impl ServerPacket for TriggerCinematic {
    const OPCODE: ServerOpcodes = ServerOpcodes::TriggerCinematic;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.cinematic_id);
        for byte in self.conversation_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
    }
}

/// SMSG_NPC_INTERACTION_OPEN_RESULT — opens an NPC interaction UI on client.
/// C++ `WorldPackets::NPC::NPCInteractionOpenResult::Write`
/// (`Server/Packets/NPCPackets.cpp:96-104`).
/// PlayerInteractionType values: Banker=8, Binder=20, Auctioneer=21,
/// StableMaster=22, GuildTabardVendor=14, TaxiNode=6, Merchant=5, Trainer=7.
pub struct NpcInteractionOpenResult {
    pub npc: wow_core::ObjectGuid,
    pub interaction_type: i32,
    pub success: bool,
}

impl NpcInteractionOpenResult {
    pub fn new(npc: wow_core::ObjectGuid, interaction_type: i32) -> Self {
        Self {
            npc,
            interaction_type,
            success: true,
        }
    }
}

impl ServerPacket for NpcInteractionOpenResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::NpcInteractionOpenResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.npc);
        pkt.write_int32(self.interaction_type);
        pkt.write_bit(self.success);
        pkt.flush_bits();
    }
}

// ── MailQueryNextTimeResult ──────────────────────────────────────────────────

/// C++ `WorldPackets::Token::CommerceTokenGetLog`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommerceTokenGetLog {
    pub unk_int: u32,
}

impl ClientPacket for CommerceTokenGetLog {
    const OPCODE: ClientOpcodes = ClientOpcodes::CommerceTokenGetLog;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            unk_int: pkt.read_uint32()?,
        })
    }
}

/// C++ `WorldPackets::Token::CommerceTokenGetLogResponse`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommerceTokenGetLogResponse {
    pub unk_int: u32,
    pub result: u32,
    /// Auctionable token rows are unimplemented in this C++ branch too; the
    /// handler sends a success response with an empty list.
    pub auctionable_token_count: u32,
}

impl CommerceTokenGetLogResponse {
    pub fn success_empty(unk_int: u32) -> Self {
        Self {
            unk_int,
            result: TOKEN_RESULT_SUCCESS_LIKE_CPP,
            auctionable_token_count: 0,
        }
    }
}

impl ServerPacket for CommerceTokenGetLogResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::CommerceTokenGetLogResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.unk_int);
        pkt.write_uint32(self.result);
        pkt.write_uint32(self.auctionable_token_count);
    }
}
