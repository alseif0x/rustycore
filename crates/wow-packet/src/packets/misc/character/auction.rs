//! Auction packets.
//!
//! Separated from character.rs under #689.

use super::*;

// ── PlayedTime (SMSG 0x26d5) ─────────────────────────────────────────────────

/// C++ `WorldPackets::AuctionHouse::AuctionListItems`.
///
/// This legacy opcode's `Read()` intentionally consumes no fields in the
/// current C++ source; the handler is also a legacy no-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuctionListItems;

impl ClientPacket for AuctionListItems {
    const OPCODE: ClientOpcodes = ClientOpcodes::AuctionListItems;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::AuctionHouse::AuctionReplicateItems`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuctionReplicateItems {
    pub auctioneer: ObjectGuid,
    pub change_number_global: u32,
    pub change_number_cursor: u32,
    pub change_number_tombstone: u32,
    pub count: u32,
    pub tainted_by: Option<AuctionAddonInfo>,
}

impl ClientPacket for AuctionReplicateItems {
    const OPCODE: ClientOpcodes = ClientOpcodes::AuctionReplicateItems;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let auctioneer = pkt.read_guid()?;
        let change_number_global = pkt.read_uint32()?;
        let change_number_cursor = pkt.read_uint32()?;
        let change_number_tombstone = pkt.read_uint32()?;
        let count = pkt.read_uint32()?;
        let tainted_by = if pkt.read_bit()? {
            Some(AuctionAddonInfo::read(pkt)?)
        } else {
            None
        };

        Ok(Self {
            auctioneer,
            change_number_global,
            change_number_cursor,
            change_number_tombstone,
            count,
            tainted_by,
        })
    }
}

/// C++ `WorldPackets::AuctionHouse::AuctionRemoveItem`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuctionRemoveItem {
    pub auctioneer: ObjectGuid,
    pub auction_id: i32,
    pub item_id: i32,
    pub tainted_by: Option<AuctionAddonInfo>,
}

impl ClientPacket for AuctionRemoveItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::AuctionRemoveItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let auctioneer = pkt.read_guid()?;
        let auction_id = pkt.read_int32()?;
        let item_id = pkt.read_int32()?;
        let tainted_by = if pkt.read_bit()? {
            Some(AuctionAddonInfo::read(pkt)?)
        } else {
            None
        };

        Ok(Self {
            auctioneer,
            auction_id,
            item_id,
            tainted_by,
        })
    }
}

/// C++ `WorldPackets::AuctionHouse::AuctionItemForSale`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuctionItemForSale {
    pub guid: ObjectGuid,
    pub use_count: u32,
}

impl AuctionItemForSale {
    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            guid: pkt.read_guid()?,
            use_count: pkt.read_uint32()?,
        })
    }
}

/// C++ `WorldPackets::AuctionHouse::AuctionSellItem`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuctionSellItem {
    pub auctioneer: ObjectGuid,
    pub min_bid: u64,
    pub buyout_price: u64,
    pub runtime: u32,
    pub tainted_by: Option<AuctionAddonInfo>,
    pub items: Vec<AuctionItemForSale>,
}

impl ClientPacket for AuctionSellItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::AuctionSellItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let auctioneer = pkt.read_guid()?;
        let min_bid = pkt.read_uint64()?;
        let buyout_price = pkt.read_uint64()?;
        let runtime = pkt.read_uint32()?;
        let tainted_by_present = pkt.read_bit()?;
        let item_count = pkt.read_bits(6)? as usize;
        let tainted_by = if tainted_by_present {
            Some(AuctionAddonInfo::read(pkt)?)
        } else {
            None
        };
        let mut items = Vec::with_capacity(item_count);
        for _ in 0..item_count {
            items.push(AuctionItemForSale::read(pkt)?);
        }

        Ok(Self {
            auctioneer,
            min_bid,
            buyout_price,
            runtime,
            tainted_by,
            items,
        })
    }
}

/// SMSG_AUCTION_LIST_BIDDER_ITEMS_RESULT — empty bidder list.
pub struct AuctionListBidderItemsResult;

impl ServerPacket for AuctionListBidderItemsResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::AuctionListBidderItemsResult;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Items.Count
        pkt.write_int32(0); // TotalCount
        pkt.write_int32(0); // DesiredDelay (ms)
    }
}

/// SMSG_AUCTION_LIST_OWNER_ITEMS_RESULT — empty owner list.
pub struct AuctionListOwnerItemsResult;

impl ServerPacket for AuctionListOwnerItemsResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::AuctionListOwnerItemsResult;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Items.Count
        pkt.write_int32(0); // TotalCount
        pkt.write_int32(0); // DesiredDelay
    }
}

/// C++ `WorldPackets::AuctionHouse::AuctionableTokenSell`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AuctionableTokenSell;

impl ClientPacket for AuctionableTokenSell {
    const OPCODE: ClientOpcodes = ClientOpcodes::AuctionableTokenSell;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::AuctionHouse::AuctionableTokenSellAtMarketPrice`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AuctionableTokenSellAtMarketPrice;

impl ClientPacket for AuctionableTokenSellAtMarketPrice {
    const OPCODE: ClientOpcodes = ClientOpcodes::AuctionableTokenSellAtMarketPrice;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}
