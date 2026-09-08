//! Classic SpellPackets.cpp SpellFailure / SpellFailedOther. Their reason
//! widths deliberately differ: uint16 versus uint8.

use super::{ObjectGuid, ServerOpcodes, ServerPacket, SpellCastVisual, WorldPacket};

pub struct SpellFailurePkt {
    pub caster: ObjectGuid,
    pub cast_id: ObjectGuid,
    pub spell_id: i32,
    pub visual: SpellCastVisual,
    pub reason: u16,
}

impl ServerPacket for SpellFailurePkt {
    const OPCODE: ServerOpcodes = ServerOpcodes::SpellFailure;
    fn write(&self, packet: &mut WorldPacket) {
        packet.write_packed_guid(&self.caster);
        packet.write_packed_guid(&self.cast_id);
        packet.write_int32(self.spell_id);
        self.visual.write(packet);
        packet.write_uint16(self.reason);
    }
}

pub struct SpellFailedOtherPkt {
    pub caster: ObjectGuid,
    pub cast_id: ObjectGuid,
    pub spell_id: u32,
    pub visual: SpellCastVisual,
    pub reason: u8,
}

impl ServerPacket for SpellFailedOtherPkt {
    const OPCODE: ServerOpcodes = ServerOpcodes::SpellFailedOther;
    fn write(&self, packet: &mut WorldPacket) {
        packet.write_packed_guid(&self.caster);
        packet.write_packed_guid(&self.cast_id);
        packet.write_uint32(self.spell_id);
        self.visual.write(packet);
        packet.write_uint8(self.reason);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interruption_packets_keep_the_distinct_cpp_reason_widths() {
        let failure = SpellFailurePkt {
            caster: ObjectGuid::EMPTY,
            cast_id: ObjectGuid::EMPTY,
            spell_id: 133,
            visual: SpellCastVisual::default(),
            reason: 0x1234,
        }
        .to_bytes();
        let other = SpellFailedOtherPkt {
            caster: ObjectGuid::EMPTY,
            cast_id: ObjectGuid::EMPTY,
            spell_id: 133,
            visual: SpellCastVisual::default(),
            reason: 0x56,
        }
        .to_bytes();
        // Two empty PackedGUID masks, SpellID and the single visual ID.
        assert_eq!(
            &failure[2..],
            &[0, 0, 0, 0, 133, 0, 0, 0, 0, 0, 0, 0, 0x34, 0x12]
        );
        assert_eq!(&other[2..], &[0, 0, 0, 0, 133, 0, 0, 0, 0, 0, 0, 0, 0x56]);
        assert_eq!(
            &failure[..2],
            &(ServerOpcodes::SpellFailure as u16).to_le_bytes()
        );
        assert_eq!(
            &other[..2],
            &(ServerOpcodes::SpellFailedOther as u16).to_le_bytes()
        );
    }
}
