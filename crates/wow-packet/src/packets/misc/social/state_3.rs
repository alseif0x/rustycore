//! Duel and social packets state definitions, part 3 of 3.
//!
//! Separated from the social.rs root under #650. Behaviour is preserved.

use super::*;

/// C++ `WorldPackets::Trade::UnacceptTrade`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UnacceptTrade;

impl ClientPacket for UnacceptTrade {
    const OPCODE: ClientOpcodes = ClientOpcodes::UnacceptTrade;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::Trade::BeginTrade`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BeginTrade;

impl ClientPacket for BeginTrade {
    const OPCODE: ClientOpcodes = ClientOpcodes::BeginTrade;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::Trade::IgnoreTrade`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IgnoreTrade;

impl ClientPacket for IgnoreTrade {
    const OPCODE: ClientOpcodes = ClientOpcodes::IgnoreTrade;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// Bounded C++ `WorldPackets::Trade::TradeStatus` writer.
///
/// C++ writes `PartnerIsSameBnetAccount`, then five status bits. The bounded
/// Rust writer currently represents the `TRADE_STATUS_INITIATED` payload and
/// cancel-like statuses that only flush bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeStatus {
    pub status: u8,
    pub partner_is_same_bnet_account: bool,
    pub id: u32,
    pub failure_for_you: bool,
    pub bag_result: i32,
    pub item_id: i32,
}

impl TradeStatus {
    pub fn cancel_like_cpp(status: u8) -> Self {
        Self {
            status,
            partner_is_same_bnet_account: false,
            id: 0,
            failure_for_you: false,
            bag_result: 0,
            item_id: 0,
        }
    }

    pub fn initiated_like_cpp(id: u32) -> Self {
        Self {
            status: TRADE_STATUS_INITIATED_LIKE_CPP,
            partner_is_same_bnet_account: false,
            id,
            failure_for_you: false,
            bag_result: 0,
            item_id: 0,
        }
    }

    pub fn status_only_like_cpp(status: u8) -> Self {
        Self {
            status,
            partner_is_same_bnet_account: false,
            id: 0,
            failure_for_you: false,
            bag_result: 0,
            item_id: 0,
        }
    }

    pub fn failed_like_cpp(bag_result: i32, item_id: i32) -> Self {
        Self {
            status: TRADE_STATUS_FAILED_LIKE_CPP,
            partner_is_same_bnet_account: false,
            id: 0,
            failure_for_you: false,
            bag_result,
            item_id,
        }
    }
}

impl ServerPacket for TradeStatus {
    const OPCODE: ServerOpcodes = ServerOpcodes::TradeStatus;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.partner_is_same_bnet_account);
        pkt.write_bits(u32::from(self.status), 5);
        match self.status {
            TRADE_STATUS_FAILED_LIKE_CPP => {
                pkt.write_bit(self.failure_for_you);
                pkt.write_int32(self.bag_result);
                pkt.write_int32(self.item_id);
            }
            TRADE_STATUS_INITIATED_LIKE_CPP => {
                pkt.write_uint32(self.id);
            }
            _ => {
                pkt.flush_bits();
            }
        }
    }
}
