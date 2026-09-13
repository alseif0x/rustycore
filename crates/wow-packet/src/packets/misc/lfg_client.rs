// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! C++ Server/Packets/LFGPackets.cpp:20-62 and LFGPackets.h:36-106.
//! Wire decoding only: role/slot eligibility and ticket ownership belong to handlers.

use super::{ClientOpcodes, ClientPacket, LfgRideTicket, PacketError, WorldPacket};

/// C++ `DFJoin`, including `Array<uint32, 50> Slots`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DfJoin {
    pub queue_as_group: bool,
    pub unknown: bool,
    pub party_index: Option<u8>,
    pub roles: u8,
    pub slots: Vec<u32>,
}

impl ClientPacket for DfJoin {
    const OPCODE: ClientOpcodes = ClientOpcodes::DfJoin;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let queue_as_group = pkt.read_bit()?;
        let has_party_index = pkt.read_bit()?;
        let unknown = pkt.read_bit()?;
        let roles = pkt.read_uint8()?;
        let count = pkt.read_uint32()? as usize;
        // PacketUtilities.h Array::resize rejects capacity before reading PartyIndex/slots.
        if count > 50 {
            return Err(PacketError::InvalidArrayCapacity {
                requested: count,
                max: 50,
            });
        }
        let party_index = if has_party_index {
            Some(pkt.read_uint8()?)
        } else {
            None
        };
        let mut slots = Vec::with_capacity(count);
        for _ in 0..count {
            slots.push(pkt.read_uint32()?);
        }
        Ok(Self {
            queue_as_group,
            unknown,
            party_index,
            roles,
            slots,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DfLeave {
    pub ticket: LfgRideTicket,
}

impl ClientPacket for DfLeave {
    const OPCODE: ClientOpcodes = ClientOpcodes::DfLeave;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            ticket: LfgRideTicket::read_like_cpp(pkt)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DfProposalResponse {
    pub ticket: LfgRideTicket,
    pub instance_id: u64,
    pub proposal_id: u32,
    pub accepted: bool,
}

impl ClientPacket for DfProposalResponse {
    const OPCODE: ClientOpcodes = ClientOpcodes::DfProposalResponse;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            ticket: LfgRideTicket::read_like_cpp(pkt)?,
            instance_id: pkt.read_uint64()?,
            proposal_id: pkt.read_uint32()?,
            accepted: pkt.read_bit()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DfSetRoles {
    pub roles_desired: u8,
    pub party_index: Option<u8>,
}

impl ClientPacket for DfSetRoles {
    const OPCODE: ClientOpcodes = ClientOpcodes::DfSetRoles;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let has_party_index = pkt.read_bit()?;
        let roles_desired = pkt.read_uint8()?;
        let party_index = if has_party_index {
            Some(pkt.read_uint8()?)
        } else {
            None
        };
        Ok(Self {
            roles_desired,
            party_index,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DfBootPlayerVote {
    pub vote: bool,
}

impl ClientPacket for DfBootPlayerVote {
    const OPCODE: ClientOpcodes = ClientOpcodes::DfBootPlayerVote;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            vote: pkt.read_bit()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DfTeleport {
    pub teleport_out: bool,
}

impl ClientPacket for DfTeleport {
    const OPCODE: ClientOpcodes = ClientOpcodes::DfTeleport;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            teleport_out: pkt.read_bit()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read<T: ClientPacket>(bytes: &[u8]) -> T {
        let mut pkt = WorldPacket::from_bytes(bytes);
        let value = T::read(&mut pkt).unwrap();
        assert_eq!(pkt.remaining(), 0);
        value
    }

    fn rejects_every_truncation<T: ClientPacket>(bytes: &[u8]) {
        for len in 0..bytes.len() {
            assert!(
                T::read(&mut WorldPacket::from_bytes(&bytes[..len])).is_err(),
                "length {len}"
            );
        }
    }

    #[test]
    fn df_join_reads_optional_party_after_count_and_preserves_raw_slots() {
        // All three flags, raw roles, count=2, party=7, typed entries (not masked IDs).
        let bytes = [0xe0, 0xff, 2, 0, 0, 0, 7, 6, 1, 0, 6, 7, 1, 0, 6];
        let join = read::<DfJoin>(&bytes);
        assert!(join.queue_as_group && join.unknown);
        assert_eq!(join.roles, 0xff);
        assert_eq!(join.party_index, Some(7));
        assert_eq!(join.slots, [0x06000106, 0x06000107]);
        rejects_every_truncation::<DfJoin>(&bytes);
    }

    #[test]
    fn df_join_flags_are_independent_and_padding_is_not_a_flag() {
        for flags in 0u8..8 {
            let mut bytes = vec![(flags << 5) | 0x1f, 8, 0, 0, 0, 0];
            if flags & 2 != 0 {
                bytes.push(9);
            }
            let join = read::<DfJoin>(&bytes);
            assert_eq!(join.queue_as_group, flags & 4 != 0);
            assert_eq!(join.unknown, flags & 1 != 0);
            assert_eq!(join.party_index, (flags & 2 != 0).then_some(9));
        }
    }

    #[test]
    fn df_join_accepts_zero_and_fifty_slots_without_party() {
        let empty = read::<DfJoin>(&[0, 0, 0, 0, 0, 0]);
        assert!(!empty.queue_as_group && !empty.unknown);
        assert_eq!(empty.party_index, None);
        assert!(empty.slots.is_empty());
        let mut bytes = vec![0, 8, 50, 0, 0, 0];
        for id in 0u32..50 {
            bytes.extend_from_slice(&id.to_le_bytes());
        }
        let join = read::<DfJoin>(&bytes);
        assert_eq!(join.slots, (0..50).collect::<Vec<_>>());
        rejects_every_truncation::<DfJoin>(&bytes);
    }

    #[test]
    fn df_join_rejects_capacity_before_optional_party_or_allocation() {
        for count in [51u32, u32::MAX] {
            let mut bytes = vec![0x40, 8];
            bytes.extend_from_slice(&count.to_le_bytes());
            assert!(matches!(DfJoin::read(&mut WorldPacket::from_bytes(&bytes)),
                Err(PacketError::InvalidArrayCapacity { requested, max: 50 }) if requested == count as usize));
        }
    }

    // LFGPacketsCommon.cpp: packed GUID masks+bytes, uint32 ID/type, int64 time, one bit.
    fn ticket_bytes() -> Vec<u8> {
        let mut bytes = vec![1, 0x80, 0x12, 0x34];
        bytes.extend_from_slice(&0x12345678u32.to_le_bytes());
        bytes.extend_from_slice(&3u32.to_le_bytes());
        bytes.extend_from_slice(&(-0x123456789i64).to_le_bytes());
        bytes.push(0x80);
        bytes
    }

    #[test]
    fn df_leave_accepts_empty_ticket_without_applying_ownership_rules() {
        // Two zero packed-GUID masks, ID/type/time zero, Unknown925=false.
        let bytes = [0; 19];
        assert_eq!(read::<DfLeave>(&bytes).ticket, LfgRideTicket::default());
        rejects_every_truncation::<DfLeave>(&bytes);
    }

    #[test]
    fn df_leave_reads_packed_ticket_and_signed_64_bit_time() {
        let bytes = ticket_bytes();
        let ticket = read::<DfLeave>(&bytes).ticket;
        let mut guid = [0; 16];
        guid[0] = 0x12;
        guid[15] = 0x34;
        assert_eq!(
            ticket.requester_guid,
            wow_core::ObjectGuid::from_raw_bytes(&guid)
        );
        assert_eq!(ticket.id, 0x12345678);
        assert_eq!(ticket.ride_type, 3);
        assert_eq!(ticket.time, -0x123456789);
        assert!(ticket.unknown925);
        rejects_every_truncation::<DfLeave>(&bytes);
    }

    #[test]
    fn df_proposal_response_aligns_after_ticket_and_reads_acceptance_msb() {
        for (bit_byte, expected) in [(0x80, true), (0x01, false)] {
            let mut bytes = ticket_bytes();
            bytes.extend_from_slice(&0x1122334455667788u64.to_le_bytes());
            bytes.extend_from_slice(&0x99aabbccu32.to_le_bytes());
            bytes.push(bit_byte);
            let proposal = read::<DfProposalResponse>(&bytes);
            assert_eq!(proposal.instance_id, 0x1122334455667788);
            assert_eq!(proposal.proposal_id, 0x99aabbcc);
            assert_eq!(proposal.accepted, expected);
            assert!(proposal.ticket.unknown925);
            rejects_every_truncation::<DfProposalResponse>(&bytes);
        }
    }

    #[test]
    fn df_set_roles_decodes_both_optional_party_branches() {
        let roles = read::<DfSetRoles>(&[0, 0xff]);
        assert_eq!(roles.roles_desired, 0xff);
        assert_eq!(roles.party_index, None);
        let roles = read::<DfSetRoles>(&[0x80, 2, 7]);
        assert_eq!(roles.roles_desired, 2);
        assert_eq!(roles.party_index, Some(7));
        rejects_every_truncation::<DfSetRoles>(&[0, 0xff]);
        rejects_every_truncation::<DfSetRoles>(&[0x80, 2, 7]);
    }

    #[test]
    fn df_vote_and_teleport_read_msb_not_boolean_byte() {
        for byte in 0..=u8::MAX {
            assert_eq!(read::<DfBootPlayerVote>(&[byte]).vote, byte & 0x80 != 0);
            assert_eq!(read::<DfTeleport>(&[byte]).teleport_out, byte & 0x80 != 0);
        }
        rejects_every_truncation::<DfBootPlayerVote>(&[0]);
        rejects_every_truncation::<DfTeleport>(&[0]);
    }

    #[test]
    fn df_client_opcodes_match_cpp() {
        assert_eq!(DfJoin::OPCODE as u32, 0x360b);
        assert_eq!(DfLeave::OPCODE as u32, 0x3614);
        assert_eq!(DfProposalResponse::OPCODE as u32, 0x3609);
        assert_eq!(DfSetRoles::OPCODE as u32, 0x3617);
        assert_eq!(DfBootPlayerVote::OPCODE as u32, 0x3618);
        assert_eq!(DfTeleport::OPCODE as u32, 0x3619);
    }
}
