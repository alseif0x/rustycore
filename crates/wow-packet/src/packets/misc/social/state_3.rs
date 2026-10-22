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

/// C++ `WorldPackets::Trade::InitiateTrade` (`TradePackets.cpp`): one packed
/// GUID, the player the client wants to trade with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InitiateTrade {
    pub guid: ObjectGuid,
}

impl ClientPacket for InitiateTrade {
    const OPCODE: ClientOpcodes = ClientOpcodes::InitiateTrade;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            guid: pkt.read_packed_guid()?,
        })
    }
}

/// C++ `WorldPackets::Trade::TradeStatus` (`TradePackets.cpp`).
///
/// C++ writes `PartnerIsSameBnetAccount`, five status bits, then the
/// status-specific payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeStatus {
    pub status: u8,
    pub partner_is_same_bnet_account: bool,
    pub id: u32,
    pub failure_for_you: bool,
    pub bag_result: i32,
    pub item_id: i32,
    pub partner: ObjectGuid,
    pub partner_account: ObjectGuid,
    pub trade_slot: u8,
    pub currency_type: i32,
    pub currency_quantity: i32,
}

impl TradeStatus {
    /// C++ `WorldPackets::Trade::TradeStatus` default members with `Status`.
    pub fn status_only_like_cpp(status: u8) -> Self {
        Self {
            status,
            partner_is_same_bnet_account: false,
            id: 0,
            failure_for_you: false,
            bag_result: 0,
            item_id: 0,
            partner: ObjectGuid::EMPTY,
            partner_account: ObjectGuid::EMPTY,
            trade_slot: 0,
            currency_type: 0,
            currency_quantity: 0,
        }
    }

    pub fn cancel_like_cpp(status: u8) -> Self {
        Self::status_only_like_cpp(status)
    }

    pub fn initiated_like_cpp(id: u32) -> Self {
        Self {
            id,
            ..Self::status_only_like_cpp(TRADE_STATUS_INITIATED_LIKE_CPP)
        }
    }

    pub fn failed_like_cpp(bag_result: i32, item_id: i32) -> Self {
        Self {
            bag_result,
            item_id,
            ..Self::status_only_like_cpp(TRADE_STATUS_FAILED_LIKE_CPP)
        }
    }

    /// C++ `HandleInitiateTradeOpcode`: `Status = PROPOSED`, `Partner` is the
    /// initiator; `PartnerAccount` is left empty.
    pub fn proposed_like_cpp(partner: ObjectGuid) -> Self {
        Self {
            partner,
            ..Self::status_only_like_cpp(TRADE_STATUS_PROPOSED_LIKE_CPP)
        }
    }

    /// C++ `HandleSetTradeItemOpcode` bound-item refusal.
    pub fn not_on_taplist_like_cpp(trade_slot: u8) -> Self {
        Self {
            trade_slot,
            ..Self::status_only_like_cpp(TRADE_STATUS_NOT_ON_TAPLIST_LIKE_CPP)
        }
    }

    /// C++ `WorldSession::SendTradeStatus` fills `PartnerIsSameBnetAccount`.
    #[must_use]
    pub fn with_partner_is_same_bnet_account(mut self, same: bool) -> Self {
        self.partner_is_same_bnet_account = same;
        self
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
            TRADE_STATUS_PROPOSED_LIKE_CPP => {
                pkt.write_packed_guid(&self.partner);
                pkt.write_packed_guid(&self.partner_account);
            }
            TRADE_STATUS_WRONG_REALM_LIKE_CPP | TRADE_STATUS_NOT_ON_TAPLIST_LIKE_CPP => {
                pkt.write_uint8(self.trade_slot);
            }
            TRADE_STATUS_NOT_ENOUGH_CURRENCY_LIKE_CPP
            | TRADE_STATUS_CURRENCY_NOT_TRADABLE_LIKE_CPP => {
                pkt.write_int32(self.currency_type);
                pkt.write_int32(self.currency_quantity);
            }
            _ => {
                pkt.flush_bits();
            }
        }
    }
}
