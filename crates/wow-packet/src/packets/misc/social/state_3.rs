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

/// C++ `WorldPackets::Item::ItemGemData`: slot, then the gem's `ItemInstance`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TradeItemGemLikeCpp {
    pub slot: u8,
    pub item: crate::packets::item::ItemInstance,
}

/// C++ `WorldPackets::Trade::UnwrappedTradeItem` (`TradePackets.cpp`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnwrappedTradeItemLikeCpp {
    pub enchant_id: i32,
    pub on_use_enchantment_id: i32,
    pub creator: ObjectGuid,
    pub charges: i32,
    pub lock: bool,
    pub max_durability: u32,
    pub durability: u32,
    pub gems: Vec<TradeItemGemLikeCpp>,
}

impl UnwrappedTradeItemLikeCpp {
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.enchant_id);
        pkt.write_int32(self.on_use_enchantment_id);
        pkt.write_packed_guid(&self.creator);
        pkt.write_int32(self.charges);
        pkt.write_uint32(self.max_durability);
        pkt.write_uint32(self.durability);
        pkt.write_bits(self.gems.len() as u32, 2);
        pkt.write_bit(self.lock);
        pkt.flush_bits();
        for gem in &self.gems {
            pkt.write_uint8(gem.slot);
            gem.item.write(pkt);
        }
    }
}

/// C++ `WorldPackets::Trade::TradeItem`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TradeItemLikeCpp {
    pub slot: u8,
    pub item: crate::packets::item::ItemInstance,
    pub stack_count: i32,
    pub gift_creator: ObjectGuid,
    pub unwrapped: Option<UnwrappedTradeItemLikeCpp>,
}

impl TradeItemLikeCpp {
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.slot);
        pkt.write_uint32(self.stack_count as u32);
        pkt.write_packed_guid(&self.gift_creator);
        self.item.write(pkt);
        pkt.write_bit(self.unwrapped.is_some());
        pkt.flush_bits();
        if let Some(unwrapped) = &self.unwrapped {
            unwrapped.write(pkt);
        }
    }
}

/// C++ `WorldPackets::Trade::TradeUpdated` (`SMSG_TRADE_UPDATED`), built by
/// `WorldSession::SendUpdateTrade` (`TradeHandler.cpp:54-105`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TradeUpdated {
    pub which_player: u8,
    pub id: u32,
    pub client_state_index: u32,
    pub current_state_index: u32,
    pub gold: u64,
    pub currency_type: i32,
    pub currency_quantity: i32,
    pub proposed_enchantment: i32,
    pub items: Vec<TradeItemLikeCpp>,
}

impl ServerPacket for TradeUpdated {
    const OPCODE: ServerOpcodes = ServerOpcodes::TradeUpdated;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.which_player);
        pkt.write_uint32(self.id);
        pkt.write_uint32(self.client_state_index);
        pkt.write_uint32(self.current_state_index);
        pkt.write_uint64(self.gold);
        pkt.write_int32(self.currency_type);
        pkt.write_int32(self.currency_quantity);
        pkt.write_int32(self.proposed_enchantment);
        pkt.write_uint32(self.items.len() as u32);
        for item in &self.items {
            item.write(pkt);
        }
    }
}

#[cfg(test)]
mod trade_packet_tests {
    use super::*;

    #[test]
    fn trade_status_payload_branches_match_cpp_writer() {
        let proposed = TradeStatus::proposed_like_cpp(ObjectGuid::create_player(1, 0x22))
            .with_partner_is_same_bnet_account(true)
            .to_bytes();
        assert_eq!(proposed[2], 0x80 | (TRADE_STATUS_PROPOSED_LIKE_CPP << 2));
        let mut payload = WorldPacket::from_bytes(&proposed[3..]);
        assert_eq!(
            payload.read_packed_guid().unwrap(),
            ObjectGuid::create_player(1, 0x22)
        );
        assert_eq!(payload.read_packed_guid().unwrap(), ObjectGuid::EMPTY);

        let taplist = TradeStatus::not_on_taplist_like_cpp(5).to_bytes();
        assert_eq!(
            &taplist[2..],
            &[TRADE_STATUS_NOT_ON_TAPLIST_LIKE_CPP << 2, 5]
        );

        let failed = TradeStatus::failed_like_cpp(30, 0).to_bytes();
        assert_eq!(failed[2], TRADE_STATUS_FAILED_LIKE_CPP << 2);
        assert_eq!(&failed[3..], &[30, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn trade_updated_writes_cpp_field_order() {
        let packet = TradeUpdated {
            which_player: 1,
            id: 0,
            client_state_index: 2,
            current_state_index: 3,
            gold: 4,
            currency_type: 0,
            currency_quantity: 0,
            proposed_enchantment: 7,
            items: vec![TradeItemLikeCpp {
                slot: 6,
                item: crate::packets::item::ItemInstance {
                    item_id: 25,
                    ..Default::default()
                },
                stack_count: 1,
                gift_creator: ObjectGuid::EMPTY,
                unwrapped: None,
            }],
        };
        let bytes = packet.to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::TradeUpdated as u16
        );
        let body = &bytes[2..];
        assert_eq!(body[0], 1);
        assert_eq!(&body[1..5], &0u32.to_le_bytes());
        assert_eq!(&body[5..9], &2u32.to_le_bytes());
        assert_eq!(&body[9..13], &3u32.to_le_bytes());
        assert_eq!(&body[13..21], &4u64.to_le_bytes());
        assert_eq!(&body[29..33], &7i32.to_le_bytes());
        assert_eq!(&body[33..37], &1u32.to_le_bytes());
        // slot, stack count, empty packed gift creator.
        assert_eq!(&body[37..44], &[6, 1, 0, 0, 0, 0, 0]);
        // ItemInstance: entry, seed, properties id, bonus bit, mods count.
        assert_eq!(&body[44..48], &25i32.to_le_bytes());
        // ...followed by the `Unwrapped` presence bit, clear.
        assert_eq!(*body.last().unwrap(), 0);
    }
}
