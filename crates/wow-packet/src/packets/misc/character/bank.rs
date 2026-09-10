//! Bank packets.
//!
//! Separated from character.rs under #689.

use super::*;

/// Audited 3.4.3 C++ `WorldPackets::Bank::AutoBankItem`: `InvUpdate`, source
/// bag and slot. Retail upstream added a `BankType` byte for account-bank
/// support after this client build; consuming it here would shift real 3.4.3
/// `Bag`/`Slot` payloads.
#[derive(Debug, Clone)]
pub struct AutoBankItem {
    pub inv_update: InvUpdate,
    pub bag: u8,
    pub slot: u8,
}

impl ClientPacket for AutoBankItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::AutobankItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            inv_update: InvUpdate::read(pkt)?,
            bag: pkt.read_uint8()?,
            slot: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Bank::AutoStoreBankItem`: `InvUpdate`, source bag and slot.
#[derive(Debug, Clone)]
pub struct AutoStoreBankItem {
    pub inv_update: InvUpdate,
    pub bag: u8,
    pub slot: u8,
}

impl ClientPacket for AutoStoreBankItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::AutostoreBankItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            inv_update: InvUpdate::read(pkt)?,
            bag: pkt.read_uint8()?,
            slot: pkt.read_uint8()?,
        })
    }
}

// ── Guild Bank ─────────────────────────────────────────────────────

/// C++ `WorldPackets::Guild::GuildBankActivate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildBankActivate {
    pub banker: ObjectGuid,
    pub full_update: bool,
}

impl ClientPacket for GuildBankActivate {
    const OPCODE: ClientOpcodes = ClientOpcodes::GuildBankActivate;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            banker: pkt.read_guid()?,
            full_update: pkt.read_bit()?,
        })
    }
}

/// C++ `WorldPackets::Guild::GuildBankQueryTab`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildBankQueryTab {
    pub banker: ObjectGuid,
    pub tab: u8,
    pub full_update: bool,
}

impl ClientPacket for GuildBankQueryTab {
    const OPCODE: ClientOpcodes = ClientOpcodes::GuildBankQueryTab;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            banker: pkt.read_guid()?,
            tab: pkt.read_uint8()?,
            full_update: pkt.read_bit()?,
        })
    }
}

/// C++ `WorldPackets::Guild::GuildBankBuyTab`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildBankBuyTab {
    pub banker: ObjectGuid,
    pub bank_tab: u8,
}

impl ClientPacket for GuildBankBuyTab {
    const OPCODE: ClientOpcodes = ClientOpcodes::GuildBankBuyTab;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            banker: pkt.read_guid()?,
            bank_tab: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Guild::GuildBankUpdateTab`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildBankUpdateTab {
    pub banker: ObjectGuid,
    pub bank_tab: u8,
    pub name: String,
    pub icon: String,
}

impl ClientPacket for GuildBankUpdateTab {
    const OPCODE: ClientOpcodes = ClientOpcodes::GuildBankUpdateTab;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let banker = pkt.read_guid()?;
        let bank_tab = pkt.read_uint8()?;
        let name_len = pkt.read_bits(7)? as usize;
        let icon_len = pkt.read_bits(9)? as usize;
        let name = pkt.read_string(name_len)?;
        let icon = pkt.read_string(icon_len)?;

        Ok(Self {
            banker,
            bank_tab,
            name,
            icon,
        })
    }
}

/// C++ `WorldPackets::Guild::GuildBankDepositMoney`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildBankDepositMoney {
    pub banker: ObjectGuid,
    pub money: u64,
}

impl ClientPacket for GuildBankDepositMoney {
    const OPCODE: ClientOpcodes = ClientOpcodes::GuildBankDepositMoney;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            banker: pkt.read_guid()?,
            money: pkt.read_uint64()?,
        })
    }
}

/// C++ `WorldPackets::Guild::GuildBankWithdrawMoney`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildBankWithdrawMoney {
    pub banker: ObjectGuid,
    pub money: u64,
}

impl ClientPacket for GuildBankWithdrawMoney {
    const OPCODE: ClientOpcodes = ClientOpcodes::GuildBankWithdrawMoney;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            banker: pkt.read_guid()?,
            money: pkt.read_uint64()?,
        })
    }
}

/// C++ `WorldPackets::Guild::GuildBankLogQuery`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildBankLogQuery {
    pub tab: i32,
}

impl ClientPacket for GuildBankLogQuery {
    const OPCODE: ClientOpcodes = ClientOpcodes::GuildBankLogQuery;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            tab: pkt.read_int32()?,
        })
    }
}

/// C++ `WorldPackets::Guild::GuildBankTextQuery`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildBankTextQuery {
    pub tab: i32,
}

impl ClientPacket for GuildBankTextQuery {
    const OPCODE: ClientOpcodes = ClientOpcodes::GuildBankTextQuery;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            tab: pkt.read_int32()?,
        })
    }
}

/// C++ `WorldPackets::Guild::GuildBankSetTabText`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildBankSetTabText {
    pub tab: i32,
    pub tab_text: String,
}

impl ClientPacket for GuildBankSetTabText {
    const OPCODE: ClientOpcodes = ClientOpcodes::GuildBankSetTabText;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let tab = pkt.read_int32()?;
        let tab_text_len = pkt.read_bits(14)? as usize;
        let tab_text = pkt.read_string(tab_text_len)?;

        Ok(Self { tab, tab_text })
    }
}

/// C++ `WorldPackets::Guild::AutoGuildBankItem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoGuildBankItem {
    pub banker: ObjectGuid,
    pub bank_tab: u8,
    pub bank_slot: u8,
    pub container_item_slot: u8,
    pub container_slot: Option<u8>,
}

impl ClientPacket for AutoGuildBankItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::AutoGuildBankItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let banker = pkt.read_guid()?;
        let bank_tab = pkt.read_uint8()?;
        let bank_slot = pkt.read_uint8()?;
        let container_item_slot = pkt.read_uint8()?;
        let has_container_slot = pkt.read_bit()?;
        let container_slot = if has_container_slot {
            Some(pkt.read_uint8()?)
        } else {
            None
        };

        Ok(Self {
            banker,
            bank_tab,
            bank_slot,
            container_item_slot,
            container_slot,
        })
    }
}

/// C++ `WorldPackets::Guild::AutoStoreGuildBankItem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoStoreGuildBankItem {
    pub banker: ObjectGuid,
    pub bank_tab: u8,
    pub bank_slot: u8,
}

impl ClientPacket for AutoStoreGuildBankItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::AutoStoreGuildBankItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            banker: pkt.read_guid()?,
            bank_tab: pkt.read_uint8()?,
            bank_slot: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Bank::BuyBankSlot`: a single banker `ObjectGuid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuyBankSlot {
    pub guid: ObjectGuid,
}

impl ClientPacket for BuyBankSlot {
    const OPCODE: ClientOpcodes = ClientOpcodes::BuyBankSlot;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            guid: pkt.read_guid()?,
        })
    }
}

/// C++ `WorldPackets::Bank::ChangeBankBagSlotFlag`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeBankBagSlotFlag {
    pub slot: u32,
    pub flag: u32,
    pub enabled: bool,
}

impl ClientPacket for ChangeBankBagSlotFlag {
    const OPCODE: ClientOpcodes = ClientOpcodes::ChangeBankBagSlotFlag;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            slot: pkt.read_uint32()?,
            flag: pkt.read_uint32()?,
            enabled: pkt.read_bit()?,
        })
    }
}

/// C++ `WorldPackets::Guild::GuildBankRemainingWithdrawMoney`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GuildBankRemainingWithdrawMoney {
    pub remaining_withdraw_money: i64,
}

impl ServerPacket for GuildBankRemainingWithdrawMoney {
    const OPCODE: ServerOpcodes = ServerOpcodes::GuildBankRemainingWithdrawMoney;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int64(self.remaining_withdraw_money);
    }
}
