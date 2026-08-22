// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character, inventory, vendor and currency packets.

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

// ── BugReport (CMSG 0x3687) ───────────────────────────────────────

/// C++ `WorldPackets::Misc::SetCurrencyFlags`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetCurrencyFlags {
    pub currency_id: u32,
    pub flags: u8,
}

impl ClientPacket for SetCurrencyFlags {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetCurrencyFlags;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            currency_id: pkt.read_uint32()?,
            flags: pkt.read_uint8()?,
        })
    }
}

/// Opens the barber shop/customization UI for the requested customization scope.
pub struct EnableBarberShop {
    pub customization_scope: u8,
}

impl ServerPacket for EnableBarberShop {
    const OPCODE: ServerOpcodes = ServerOpcodes::EnableBarberShop;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.customization_scope);
    }
}

// ── GameObjectInteraction (SMSG 0x288b) ─────────────────────────────

/// One C++ `WorldPackets::Misc::SetupCurrency::Record`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetupCurrencyRecord {
    pub type_id: i32,
    pub quantity: i32,
    pub weekly_quantity: Option<u32>,
    pub max_weekly_quantity: Option<u32>,
    pub tracked_quantity: Option<u32>,
    pub max_quantity: Option<i32>,
    pub total_earned: Option<i32>,
    pub next_recharge_time: Option<u64>,
    pub recharge_cycle_start_time: Option<u64>,
    pub flags: u8,
}

/// C++ `WorldPackets::Misc::SetupCurrency`.
pub struct SetupCurrency {
    pub data: Vec<SetupCurrencyRecord>,
}

impl SetupCurrency {
    pub fn empty() -> Self {
        Self { data: Vec::new() }
    }

    pub fn from_records(data: Vec<SetupCurrencyRecord>) -> Self {
        Self { data }
    }
}

impl ServerPacket for SetupCurrency {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetupCurrency;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.data.len() as u32);

        for record in &self.data {
            pkt.write_int32(record.type_id);
            pkt.write_int32(record.quantity);

            pkt.write_bit(record.weekly_quantity.is_some());
            pkt.write_bit(record.max_weekly_quantity.is_some());
            pkt.write_bit(record.tracked_quantity.is_some());
            pkt.write_bit(record.max_quantity.is_some());
            pkt.write_bit(record.total_earned.is_some());
            pkt.write_bit(record.next_recharge_time.is_some());
            pkt.write_bit(record.recharge_cycle_start_time.is_some());
            pkt.write_bits(u32::from(record.flags), 5);
            pkt.flush_bits();

            if let Some(value) = record.weekly_quantity {
                pkt.write_uint32(value);
            }
            if let Some(value) = record.max_weekly_quantity {
                pkt.write_uint32(value);
            }
            if let Some(value) = record.tracked_quantity {
                pkt.write_uint32(value);
            }
            if let Some(value) = record.max_quantity {
                pkt.write_int32(value);
            }
            if let Some(value) = record.total_earned {
                pkt.write_int32(value);
            }
            if let Some(value) = record.next_recharge_time {
                pkt.write_uint64(value);
            }
            if let Some(value) = record.recharge_cycle_start_time {
                pkt.write_uint64(value);
            }
        }
    }
}

// ── SetCurrency (SMSG 0x2574) ───────────────────────────────────────

/// Currency delta update.
///
/// Mirrors C++ `WorldPackets::Misc::SetCurrency::Write`.
pub struct SetCurrency {
    pub type_id: i32,
    pub quantity: i32,
    pub flags: u32,
    pub weekly_quantity: Option<i32>,
    pub tracked_quantity: Option<i32>,
    pub max_quantity: Option<i32>,
    pub total_earned: Option<i32>,
    pub suppress_chat_log: bool,
    pub quantity_change: Option<i32>,
    pub quantity_gain_source: Option<i32>,
    pub quantity_lost_source: Option<i32>,
    pub first_craft_operation_id: Option<u32>,
    pub next_recharge_time: Option<u64>,
    pub recharge_cycle_start_time: Option<u64>,
    pub overflown_currency_id: Option<i32>,
}

impl SetCurrency {
    pub fn vendor_gain(type_id: i32, quantity: i32, amount: i32) -> Self {
        Self {
            type_id,
            quantity,
            flags: 0,
            weekly_quantity: None,
            tracked_quantity: None,
            max_quantity: None,
            total_earned: None,
            suppress_chat_log: false,
            quantity_change: Some(amount),
            quantity_gain_source: Some(5),
            quantity_lost_source: None,
            first_craft_operation_id: None,
            next_recharge_time: None,
            recharge_cycle_start_time: None,
            overflown_currency_id: None,
        }
    }

    pub fn item_refund_gain(
        type_id: i32,
        quantity: i32,
        amount: i32,
        weekly_quantity: Option<i32>,
        max_quantity: Option<i32>,
        total_earned: Option<i32>,
        suppress_chat_log: bool,
    ) -> Self {
        Self {
            type_id,
            quantity,
            flags: 0,
            weekly_quantity,
            tracked_quantity: None,
            max_quantity,
            total_earned,
            suppress_chat_log,
            quantity_change: Some(amount),
            quantity_gain_source: Some(2),
            quantity_lost_source: None,
            first_craft_operation_id: None,
            next_recharge_time: None,
            recharge_cycle_start_time: None,
            overflown_currency_id: None,
        }
    }

    pub fn vendor_loss(type_id: i32, quantity: i32, amount: i32) -> Self {
        Self {
            type_id,
            quantity,
            flags: 0,
            weekly_quantity: None,
            tracked_quantity: None,
            max_quantity: None,
            total_earned: None,
            suppress_chat_log: false,
            quantity_change: Some(-amount),
            quantity_gain_source: None,
            quantity_lost_source: Some(4),
            first_craft_operation_id: None,
            next_recharge_time: None,
            recharge_cycle_start_time: None,
            overflown_currency_id: None,
        }
    }
}

impl ServerPacket for SetCurrency {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetCurrency;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.type_id);
        pkt.write_int32(self.quantity);
        pkt.write_uint32(self.flags);
        pkt.write_uint32(0);

        pkt.write_bit(self.weekly_quantity.is_some());
        pkt.write_bit(self.tracked_quantity.is_some());
        pkt.write_bit(self.max_quantity.is_some());
        pkt.write_bit(self.total_earned.is_some());
        pkt.write_bit(self.suppress_chat_log);
        pkt.write_bit(self.quantity_change.is_some());
        pkt.write_bit(self.quantity_gain_source.is_some());
        pkt.write_bit(self.quantity_lost_source.is_some());
        pkt.write_bit(self.first_craft_operation_id.is_some());
        pkt.write_bit(self.next_recharge_time.is_some());
        pkt.write_bit(self.recharge_cycle_start_time.is_some());
        pkt.write_bit(self.overflown_currency_id.is_some());
        pkt.flush_bits();

        if let Some(value) = self.weekly_quantity {
            pkt.write_int32(value);
        }
        if let Some(value) = self.tracked_quantity {
            pkt.write_int32(value);
        }
        if let Some(value) = self.max_quantity {
            pkt.write_int32(value);
        }
        if let Some(value) = self.total_earned {
            pkt.write_int32(value);
        }
        if let Some(value) = self.quantity_change {
            pkt.write_int32(value);
        }
        if let Some(value) = self.quantity_gain_source {
            pkt.write_int32(value);
        }
        if let Some(value) = self.quantity_lost_source {
            pkt.write_int32(value);
        }
        if let Some(value) = self.first_craft_operation_id {
            pkt.write_uint32(value);
        }
        if let Some(value) = self.next_recharge_time {
            pkt.write_uint64(value);
        }
        if let Some(value) = self.recharge_cycle_start_time {
            pkt.write_uint64(value);
        }
        if let Some(value) = self.overflown_currency_id {
            pkt.write_int32(value);
        }
    }
}

// ── UndeleteCooldownStatusResponse (SMSG 0x27ce) ────────────────────

/// Sent after the player's home bind has changed.
///
/// C++ `WorldPackets::Misc::PlayerBound::Write`: ObjectGuid stream + uint32 AreaID.
pub struct PlayerBound {
    pub binder_id: wow_core::ObjectGuid,
    pub area_id: u32,
}

impl ServerPacket for PlayerBound {
    const OPCODE: ServerOpcodes = ServerOpcodes::PlayerBound;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.binder_id);
        pkt.write_uint32(self.area_id);
    }
}

// ── WorldServerInfo (SMSG 0x25ad) ───────────────────────────────────

/// C++ `WorldPackets::EquipmentSet::LoadEquipmentSet`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadEquipmentSet {
    pub sets: Vec<EquipmentSetDataLikeCpp>,
}

impl ServerPacket for LoadEquipmentSet {
    const OPCODE: ServerOpcodes = ServerOpcodes::LoadEquipmentSet;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.sets.len() as u32);

        for set in &self.sets {
            pkt.write_int32(set.set_type);
            pkt.write_uint64(set.guid);
            pkt.write_uint32(set.set_id);
            pkt.write_uint32(set.ignore_mask);

            for i in 0..EQUIPMENT_SET_SLOTS_LIKE_CPP {
                pkt.write_guid(&set.pieces[i]);
                pkt.write_int32(set.appearances[i]);
            }

            pkt.write_int32(set.enchants[0]);
            pkt.write_int32(set.enchants[1]);
            pkt.write_int32(set.secondary_shoulder_appearance_id);
            pkt.write_int32(set.secondary_shoulder_slot);
            pkt.write_int32(set.secondary_weapon_appearance_id);
            pkt.write_int32(set.secondary_weapon_slot);

            let has_spec_index = set.assigned_spec_index != -1;
            pkt.write_bit(has_spec_index);
            pkt.write_bits(set.set_name.len() as u32, 8);
            pkt.write_bits(set.set_icon.len() as u32, 9);

            if has_spec_index {
                pkt.write_int32(set.assigned_spec_index);
            }

            pkt.write_string(&set.set_name);
            pkt.write_string(&set.set_icon);
        }
    }
}

/// C++ `WorldPackets::EquipmentSet::EquipmentSetID`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipmentSetId {
    pub guid: u64,
    pub set_type: i32,
    pub set_id: u32,
}

impl ServerPacket for EquipmentSetId {
    const OPCODE: ServerOpcodes = ServerOpcodes::EquipmentSetId;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint64(self.guid);
        pkt.write_int32(self.set_type);
        pkt.write_uint32(self.set_id);
    }
}

// ── SaveEquipmentSet (CMSG 0x3509) ───────────────────────────────────

/// C++ `EquipmentSetInfo::EquipmentSetData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipmentSetDataLikeCpp {
    pub set_type: i32,
    pub guid: u64,
    pub set_id: u32,
    pub ignore_mask: u32,
    pub pieces: [ObjectGuid; EQUIPMENT_SET_SLOTS_LIKE_CPP],
    pub appearances: [i32; EQUIPMENT_SET_SLOTS_LIKE_CPP],
    pub enchants: [i32; 2],
    pub secondary_shoulder_appearance_id: i32,
    pub secondary_shoulder_slot: i32,
    pub secondary_weapon_appearance_id: i32,
    pub secondary_weapon_slot: i32,
    pub assigned_spec_index: i32,
    pub set_name: String,
    pub set_icon: String,
}

/// C++ `WorldPackets::EquipmentSet::SaveEquipmentSet`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveEquipmentSet {
    pub set: EquipmentSetDataLikeCpp,
}

impl ClientPacket for SaveEquipmentSet {
    const OPCODE: ClientOpcodes = ClientOpcodes::SaveEquipmentSet;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let set_type = pkt.read_int32()?;
        let guid = pkt.read_uint64()?;
        let set_id = pkt.read_uint32()?;
        let ignore_mask = pkt.read_uint32()?;

        let mut pieces = [ObjectGuid::EMPTY; EQUIPMENT_SET_SLOTS_LIKE_CPP];
        let mut appearances = [0_i32; EQUIPMENT_SET_SLOTS_LIKE_CPP];
        for i in 0..EQUIPMENT_SET_SLOTS_LIKE_CPP {
            pieces[i] = pkt.read_guid()?;
            appearances[i] = pkt.read_int32()?;
        }

        let enchants = [pkt.read_int32()?, pkt.read_int32()?];
        let secondary_shoulder_appearance_id = pkt.read_int32()?;
        let secondary_shoulder_slot = pkt.read_int32()?;
        let secondary_weapon_appearance_id = pkt.read_int32()?;
        let secondary_weapon_slot = pkt.read_int32()?;

        let has_spec_index = pkt.read_bit()?;
        let set_name_len = pkt.read_bits(8)? as usize;
        let set_icon_len = pkt.read_bits(9)? as usize;
        let assigned_spec_index = if has_spec_index {
            pkt.read_int32()?
        } else {
            -1
        };

        let set_name = pkt.read_string(set_name_len)?;
        let set_icon = pkt.read_string(set_icon_len)?;

        Ok(Self {
            set: EquipmentSetDataLikeCpp {
                set_type,
                guid,
                set_id,
                ignore_mask,
                pieces,
                appearances,
                enchants,
                secondary_shoulder_appearance_id,
                secondary_shoulder_slot,
                secondary_weapon_appearance_id,
                secondary_weapon_slot,
                assigned_spec_index,
                set_name,
                set_icon,
            },
        })
    }
}

// ── AssignEquipmentSetSpec (CMSG 0x3207) ─────────────────────────────

/// C++ `WorldPackets::EquipmentSet::AssignEquipmentSetSpec`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssignEquipmentSetSpec {
    pub set_id: u32,
    pub spec_index: u32,
}

impl ClientPacket for AssignEquipmentSetSpec {
    const OPCODE: ClientOpcodes = ClientOpcodes::AssignEquipmentSetSpec;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            set_id: pkt.read_uint32()?,
            spec_index: pkt.read_uint32()?,
        })
    }
}

// ── DeleteEquipmentSet (CMSG 0x350a) ─────────────────────────────────

/// C++ `WorldPackets::EquipmentSet::DeleteEquipmentSet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteEquipmentSet {
    pub id: u64,
}

impl ClientPacket for DeleteEquipmentSet {
    const OPCODE: ClientOpcodes = ClientOpcodes::DeleteEquipmentSet;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            id: pkt.read_uint64()?,
        })
    }
}

// ── UseEquipmentSet (CMSG 0x3995 / SMSG 0x274f) ──────────────────────

/// C++ `WorldPackets::EquipmentSet::UseEquipmentSet::EquipmentSetItem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UseEquipmentSetItemLikeCpp {
    pub item: ObjectGuid,
    pub container_slot: u8,
    pub slot: u8,
}

/// C++ `WorldPackets::EquipmentSet::UseEquipmentSet`.
#[derive(Debug, Clone)]
pub struct UseEquipmentSet {
    pub inv_update: InvUpdate,
    pub items: [UseEquipmentSetItemLikeCpp; EQUIPMENT_SET_SLOTS_LIKE_CPP],
    pub guid: u64,
}

impl ClientPacket for UseEquipmentSet {
    const OPCODE: ClientOpcodes = ClientOpcodes::UseEquipmentSet;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let inv_update = InvUpdate::read(pkt)?;
        let mut items = [UseEquipmentSetItemLikeCpp {
            item: ObjectGuid::EMPTY,
            container_slot: 0,
            slot: 0,
        }; EQUIPMENT_SET_SLOTS_LIKE_CPP];
        for item in &mut items {
            item.item = pkt.read_guid()?;
            item.container_slot = pkt.read_uint8()?;
            item.slot = pkt.read_uint8()?;
        }
        let guid = pkt.read_uint64()?;

        Ok(Self {
            inv_update,
            items,
            guid,
        })
    }
}

/// C++ `WorldPackets::EquipmentSet::UseEquipmentSetResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UseEquipmentSetResult {
    pub guid: u64,
    pub reason: u8,
}

impl ServerPacket for UseEquipmentSetResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::UseEquipmentSetResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint64(self.guid);
        pkt.write_uint8(self.reason);
    }
}

// ── AllAccountCriteria (SMSG 0x2571) ─────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountMount {
    pub spell_id: i32,
    pub flags: u8,
}

/// Account-wide mount collection. Sent with IsFullUpdate=true on login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountMountUpdate {
    pub is_full_update: bool,
    pub mounts: Vec<AccountMount>,
}

impl AccountMountUpdate {
    pub fn full(mounts: Vec<AccountMount>) -> Self {
        Self {
            is_full_update: true,
            mounts,
        }
    }

    pub fn partial(mounts: Vec<AccountMount>) -> Self {
        Self {
            is_full_update: false,
            mounts,
        }
    }

    pub fn empty_full() -> Self {
        Self::full(Vec::new())
    }
}

impl ServerPacket for AccountMountUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::AccountMountUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.is_full_update);
        pkt.write_int32(self.mounts.len() as i32);
        for mount in &self.mounts {
            pkt.write_int32(mount.spell_id);
            pkt.write_bits(u32::from(mount.flags & 0x0f), 4);
        }
        pkt.flush_bits();
    }
}

// ── MountResult (SMSG 0x257b) ───────────────────────────────────────

/// C++ `WorldPackets::Spells::MountResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MountResult {
    pub result: i32,
}

impl ServerPacket for MountResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::MountResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.result);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountSpecial {
    pub spell_visual_kit_ids: Vec<i32>,
    pub sequence_variation: i32,
}

impl ClientPacket for MountSpecial {
    const OPCODE: ClientOpcodes = ClientOpcodes::MountSpecialAnim;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        let count = pkt.read_uint32()? as usize;
        let sequence_variation = pkt.read_int32()?;
        let mut spell_visual_kit_ids = Vec::with_capacity(count);
        for _ in 0..count {
            spell_visual_kit_ids.push(pkt.read_int32()?);
        }
        Ok(Self {
            spell_visual_kit_ids,
            sequence_variation,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecialMountAnim {
    pub unit_guid: ObjectGuid,
    pub spell_visual_kit_ids: Vec<i32>,
    pub sequence_variation: i32,
}

impl ServerPacket for SpecialMountAnim {
    const OPCODE: ServerOpcodes = ServerOpcodes::SpecialMountAnim;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.unit_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_uint32(self.spell_visual_kit_ids.len() as u32);
        pkt.write_int32(self.sequence_variation);
        for spell_visual_kit_id in &self.spell_visual_kit_ids {
            pkt.write_int32(*spell_visual_kit_id);
        }
    }
}

// ── AccountHeirloomUpdate (SMSG 0xBADD placeholder) ─────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountHeirloom {
    pub item_id: i32,
    pub flags: u32,
}

/// C++ `WorldPackets::Misc::AccountHeirloomUpdate`.
///
/// The archived C++ opcode table uses the shared `0xBADD` placeholder for this
/// packet, so Rust reuses the existing `UpdateCapturePoint` discriminant while
/// keeping a distinct packet type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountHeirloomUpdate {
    pub is_full_update: bool,
    pub unk: i32,
    pub heirlooms: Vec<AccountHeirloom>,
}

impl AccountHeirloomUpdate {
    pub fn full(heirlooms: Vec<AccountHeirloom>) -> Self {
        Self {
            is_full_update: true,
            unk: 0,
            heirlooms,
        }
    }
}

impl ServerPacket for AccountHeirloomUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateCapturePoint;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.is_full_update);
        pkt.flush_bits();
        pkt.write_int32(self.unk);
        pkt.write_uint32(self.heirlooms.len() as u32);
        pkt.write_uint32(self.heirlooms.len() as u32);
        for heirloom in &self.heirlooms {
            pkt.write_int32(heirloom.item_id);
        }
        for heirloom in &self.heirlooms {
            pkt.write_uint32(heirloom.flags);
        }
    }
}

// ── MountSetFavorite (CMSG 0x3633) ─────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MountSetFavorite {
    pub mount_spell_id: u32,
    pub is_favorite: bool,
}

impl ClientPacket for MountSetFavorite {
    const OPCODE: ClientOpcodes = ClientOpcodes::MountSetFavorite;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        let mount_spell_id = pkt.read_uint32()?;
        let is_favorite = pkt.read_bit()?;
        Ok(Self {
            mount_spell_id,
            is_favorite,
        })
    }
}

// ── AccountToyUpdate (SMSG 0x25b0) ───────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountToy {
    pub item_id: u32,
    pub is_favorite: bool,
    pub has_fanfare: bool,
}

/// Account-wide toy collection. Sent with IsFullUpdate=true on login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountToyUpdate {
    pub is_full_update: bool,
    pub toys: Vec<AccountToy>,
}

impl AccountToyUpdate {
    pub fn full(toys: Vec<AccountToy>) -> Self {
        Self {
            is_full_update: true,
            toys,
        }
    }
}

impl ServerPacket for AccountToyUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::AccountToyUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.is_full_update);
        pkt.flush_bits();
        pkt.write_int32(self.toys.len() as i32);
        pkt.write_int32(self.toys.len() as i32);
        pkt.write_int32(self.toys.len() as i32);
        for toy in &self.toys {
            pkt.write_uint32(toy.item_id);
        }
        for toy in &self.toys {
            pkt.write_bit(toy.is_favorite);
        }
        for toy in &self.toys {
            pkt.write_bit(toy.has_fanfare);
        }
        pkt.flush_bits();
    }
}

// ── AddToy (CMSG 0x3299) ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddToy {
    pub item_guid: ObjectGuid,
}

impl ClientPacket for AddToy {
    const OPCODE: ClientOpcodes = ClientOpcodes::AddToy;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            item_guid: pkt.read_packed_guid()?,
        })
    }
}

// ── ToyClearFanfare (CMSG 0x3128) ────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToyClearFanfare {
    pub item_id: u32,
}

impl ClientPacket for ToyClearFanfare {
    const OPCODE: ClientOpcodes = ClientOpcodes::ToyClearFanfare;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            item_id: pkt.read_uint32()?,
        })
    }
}

// ── UseToy (CMSG 0x329a) ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct UseToy {
    pub cast: CastSpellRequest,
}

impl ClientPacket for UseToy {
    const OPCODE: ClientOpcodes = ClientOpcodes::UseToy;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            cast: CastSpellRequest::read(pkt)?,
        })
    }
}

// ── Compact Unit Frame profiles ──────────────────────────────────────

/// Tells the client what weapon/armor types the player can use.
///
/// C++ `WorldPackets::Item::SetProficiency` format:
/// ```text
/// [i32] ProficiencyMask  (bitmask of sub-classes)
/// [u8]  ProficiencyClass (ItemClass enum: 2=Weapon, 4=Armor)
/// ```
pub struct SetProficiency {
    pub proficiency_mask: u32,
    pub proficiency_class: u8,
}

impl ServerPacket for SetProficiency {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetProficiency;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.proficiency_mask);
        pkt.write_uint8(self.proficiency_class);
    }
}

impl SetProficiency {
    /// Default weapon proficiency for a given class.
    ///
    /// Compatibility masks derived from C++ proficiency spell effects.
    /// Class 2 = Weapon (ItemClass.Weapon).
    pub fn default_weapons(class_id: u8) -> Self {
        // Weapon subclass bit positions (1 << subclass):
        //  0=Axe1H     0x00001   7=Sword1H   0x00080   15=Dagger    0x08000
        //  1=Axe2H     0x00002   8=Sword2H   0x00100   16=Thrown    0x10000
        //  2=Bow       0x00004  10=Staff     0x00400   18=Crossbow  0x40000
        //  3=Gun       0x00008  13=Fist      0x02000   19=Wand      0x80000
        //  4=Mace1H    0x00010
        //  5=Mace2H    0x00020
        //  6=Polearm   0x00040
        let mask = match class_id {
            1 => 0x0005_A5FF, // Warrior: Axe12,Bow,Gun,Mace12,Polearm,Sword12,Staff,Fist,Dagger,Thrown,Xbow
            2 => 0x0000_01F3, // Paladin: Axe12,Mace12,Polearm,Sword12
            3 => 0x0005_A5CF, // Hunter: Axe12,Bow,Gun,Polearm,Sword12,Staff,Fist,Dagger,Thrown,Xbow
            4 => 0x0005_A09C, // Rogue: Bow,Gun,Mace1H,Sword1H,Fist,Dagger,Thrown,Xbow
            5 => 0x0008_8410, // Priest: Mace1H,Staff,Dagger,Wand
            6 => 0x0000_01F3, // DK: Axe12,Mace12,Polearm,Sword12
            7 => 0x0000_A433, // Shaman: Axe12,Mace12,Staff,Fist,Dagger
            8 => 0x0008_8480, // Mage: Sword1H,Staff,Dagger,Wand
            9 => 0x0008_8480, // Warlock: Sword1H,Staff,Dagger,Wand
            11 => 0x0000_A470, // Druid: Mace12,Polearm,Staff,Fist,Dagger
            _ => 0x0000_2000, // Fists only
        };
        Self {
            proficiency_mask: mask,
            proficiency_class: 2, // Weapon
        }
    }

    /// Default armor proficiency for a given class.
    ///
    /// Class 4 = Armor (ItemClass.Armor).
    /// Subclass bit positions: Cloth=1(0x02), Leather=2(0x04), Mail=3(0x08),
    /// Plate=4(0x10), Shield=6(0x40).
    pub fn default_armor(class_id: u8) -> Self {
        let mask = match class_id {
            1 => 0x5E,  // Warrior: Cloth+Leather+Mail+Plate+Shield
            2 => 0x5E,  // Paladin: Cloth+Leather+Mail+Plate+Shield
            3 => 0x0E,  // Hunter: Cloth+Leather+Mail
            4 => 0x06,  // Rogue: Cloth+Leather
            5 => 0x02,  // Priest: Cloth
            6 => 0x1E,  // DK: Cloth+Leather+Mail+Plate
            7 => 0x4E,  // Shaman: Cloth+Leather+Mail+Shield
            8 => 0x02,  // Mage: Cloth
            9 => 0x02,  // Warlock: Cloth
            11 => 0x06, // Druid: Cloth+Leather
            _ => 0x02,  // Cloth
        };
        Self {
            proficiency_mask: mask,
            proficiency_class: 4, // Armor
        }
    }
}

// ── SuspendToken (SMSG 0x25a8) ───────────────────────────────────────

/// One item in the vendor's inventory list.
/// C++ `WorldPackets::NPC::VendorItem` (`Server/Packets/NPCPackets.cpp:132-150`).
#[derive(Debug, Clone)]
pub struct VendorItem {
    pub muid: i32, // slot/muid index
    pub item_id: i32,
    pub item_type: i32, // 1 = item, 2 = currency
    pub quantity: i32,  // max stack on vendor (-1 = unlimited)
    pub price: u64,     // buy price (copper)
    pub durability: i32,
    pub stack_count: i32, // VendorStackCount from item_sparse
    pub extended_cost: i32,
    pub player_condition_failed: i32,
    pub locked: bool,
    pub do_not_filter: bool,
    pub refundable: bool,
}

/// SMSG_VENDOR_INVENTORY — list of items a vendor is selling.
/// C++ `WorldPackets::NPC::VendorInventory::Write`.
pub struct VendorInventory {
    pub vendor_guid: ObjectGuid,
    pub reason: u8, // 0 = ok, non-0 = error (no items etc)
    pub items: Vec<VendorItem>,
}

impl ServerPacket for VendorInventory {
    const OPCODE: ServerOpcodes = ServerOpcodes::VendorInventory;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        pkt.write_packed_guid(&self.vendor_guid);
        pkt.write_uint8(self.reason);
        pkt.write_int32(self.items.len() as i32);

        for (i, item) in self.items.iter().enumerate() {
            pkt.write_uint64(item.price);
            pkt.write_int32(item.muid);
            pkt.write_int32(item.item_type);
            pkt.write_int32(item.durability);
            pkt.write_int32(item.stack_count);
            pkt.write_int32(item.quantity);
            pkt.write_int32(item.extended_cost);
            pkt.write_int32(item.player_condition_failed);
            // 3 bits: Locked, DoNotFilterOnVendor, Refundable
            pkt.write_bit(item.locked);
            pkt.write_bit(item.do_not_filter);
            pkt.write_bit(item.refundable);
            pkt.flush_bits();
            // ItemInstance inline:
            //   ItemID (i32), RandomPropertiesSeed (i32), RandomPropertiesID (i32)
            //   bit(ItemBonus != null) = false, FlushBits
            //   ItemModList: WriteBits(0, 6) + FlushBits  (no mods)
            pkt.write_int32(item.item_id);
            pkt.write_int32(0i32); // RandomPropertiesSeed
            pkt.write_int32(0i32); // RandomPropertiesID
            pkt.write_bit(false); // has ItemBonus = false
            pkt.flush_bits();
            pkt.write_bits(0u32, 6); // ItemModList count = 0
            pkt.flush_bits();
            // no ItemMod entries, no ItemBonus
            let _ = i; // suppress unused
        }
    }
}

/// CMSG_BUY_ITEM — client wants to buy an item from a vendor.
/// C++ `WorldPackets::Item::BuyItem::Read`.
#[derive(Debug)]
pub struct BuyItem {
    pub vendor_guid: ObjectGuid,
    pub container_guid: ObjectGuid,
    pub quantity: i32,
    pub muid: i32,
    pub slot: i32,
    pub item_type: i32,
    pub item_id: i32,
}

impl ClientPacket for BuyItem {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::BuyItem;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        let vendor_guid = pkt.read_packed_guid()?;
        let container_guid = pkt.read_packed_guid()?;
        let quantity = pkt.read_int32()?;
        let muid = pkt.read_int32()?;
        let slot = pkt.read_int32()?;
        let item_type = pkt.read_int32()?;
        // ItemInstance.Read: ItemID, RandomPropertiesSeed, RandomPropertiesID, bit(hasBonus), FlushBits, ItemModList
        let item_id = pkt.read_int32()?;
        let _seed = pkt.read_int32()?;
        let _rand_prop = pkt.read_int32()?;
        let has_bonus = pkt.read_bit()?;
        pkt.reset_bits();
        let mod_count = pkt.read_bits(6)? as u32;
        for _ in 0..mod_count {
            let _val = pkt.read_int32()?;
            let _ty = pkt.read_uint8()?;
        }
        if has_bonus {
            // ItemBonuses: Context (u8) + BonusListIDs count + entries
            let _ctx = pkt.read_uint8()?;
            let bonus_count = pkt.read_uint32()?;
            for _ in 0..bonus_count {
                let _bid = pkt.read_uint16()?;
            }
        }
        Ok(Self {
            vendor_guid,
            container_guid,
            quantity,
            muid,
            slot,
            item_type,
            item_id,
        })
    }
}

/// CMSG_BUY_BACK_ITEM — client buys back an item from a vendor buyback slot.
/// C++: WorldPackets::Item::BuyBackItem
#[derive(Debug)]
pub struct BuyBackItem {
    pub vendor_guid: ObjectGuid,
    pub slot: u32,
}

impl ClientPacket for BuyBackItem {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::BuyBackItem;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        let vendor_guid = pkt.read_packed_guid()?;
        let slot = pkt.read_uint32()?;
        Ok(Self { vendor_guid, slot })
    }
}

/// SMSG_BUY_SUCCEEDED — item bought successfully.
/// C++ `WorldPackets::Item::BuySucceeded::Write`.
pub struct BuySucceeded {
    pub vendor_guid: ObjectGuid,
    pub muid: i32,
    pub new_quantity: i32,
    pub quantity_bought: i32,
}

impl ServerPacket for BuySucceeded {
    const OPCODE: ServerOpcodes = ServerOpcodes::BuySucceeded;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        pkt.write_packed_guid(&self.vendor_guid);
        pkt.write_int32(self.muid);
        pkt.write_int32(self.new_quantity);
        pkt.write_int32(self.quantity_bought);
    }
}

/// SMSG_BUY_FAILED — buy failed with reason code.
/// C++ `WorldPackets::Item::BuyFailed::Write`.
pub struct BuyFailed {
    pub vendor_guid: ObjectGuid,
    pub muid: i32,
    pub reason: BuyResult,
}

impl ServerPacket for BuyFailed {
    const OPCODE: ServerOpcodes = ServerOpcodes::BuyFailed;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        pkt.write_packed_guid(&self.vendor_guid);
        pkt.write_int32(self.muid);
        pkt.write_uint8(self.reason as u8);
    }
}

/// CMSG_SELL_ITEM — client wants to sell an item to a vendor.
/// C++ `WorldPackets::Item::SellItem::Read`.
#[derive(Debug)]
pub struct SellItem {
    pub vendor_guid: ObjectGuid,
    pub item_guid: ObjectGuid,
    pub amount: i32,
}

impl ClientPacket for SellItem {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::SellItem;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        let vendor_guid = pkt.read_packed_guid()?;
        let item_guid = pkt.read_packed_guid()?;
        let amount = pkt.read_int32()?;
        Ok(Self {
            vendor_guid,
            item_guid,
            amount,
        })
    }
}

/// CMSG_REPAIR_ITEM — client repairs one item or all items at a repair NPC.
/// C++: WorldPackets::Item::RepairItem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairItem {
    pub npc_guid: ObjectGuid,
    pub item_guid: ObjectGuid,
    pub use_guild_bank: bool,
}

impl ClientPacket for RepairItem {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::RepairItem;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        let npc_guid = pkt.read_packed_guid()?;
        let item_guid = pkt.read_packed_guid()?;
        let use_guild_bank = pkt.read_bit()?;
        Ok(Self {
            npc_guid,
            item_guid,
            use_guild_bank,
        })
    }
}

/// SMSG_SELL_RESPONSE — result of a sell operation.
/// C++ `WorldPackets::Item::SellResponse::Write`
/// (`Server/Packets/ItemPackets.cpp:238-247`).
pub struct SellResponse {
    pub vendor_guid: ObjectGuid,
    pub item_guids: Vec<ObjectGuid>,
    pub reason: i32,
}

impl ServerPacket for SellResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::SellResponse;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        pkt.write_packed_guid(&self.vendor_guid);
        pkt.write_uint32(self.item_guids.len() as u32);
        pkt.write_int32(self.reason);
        for item_guid in &self.item_guids {
            pkt.write_packed_guid(item_guid);
        }
    }
}

impl SellResponse {
    pub fn error(vendor_guid: ObjectGuid, item_guid: ObjectGuid, reason: SellResult) -> Self {
        Self {
            vendor_guid,
            item_guids: vec![item_guid],
            reason: reason as i32,
        }
    }

    pub fn success(vendor_guid: ObjectGuid, item_guid: ObjectGuid) -> Self {
        Self {
            vendor_guid,
            item_guids: vec![item_guid],
            reason: 0,
        }
    }
}

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

/// C++ `WorldPackets::LFG::LfgPlayerInfo`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LfgPlayerInfo {
    pub blacklist: LfgBlackList,
    pub dungeons: Vec<LfgPlayerDungeonInfo>,
}

impl LfgPlayerInfo {
    pub fn empty() -> Self {
        Self::default()
    }
}

impl ServerPacket for LfgPlayerInfo {
    const OPCODE: ServerOpcodes = ServerOpcodes::LfgPlayerInfo;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.dungeons.len() as u32);
        self.blacklist.write_like_cpp(pkt);
        for dungeon in &self.dungeons {
            dungeon.write_like_cpp(pkt);
        }
    }
}

/// C++ `WorldPackets::LFG::LfgPlayerQuestRewardItem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LfgPlayerQuestRewardItem {
    pub item_id: i32,
    pub quantity: i32,
}

impl LfgPlayerQuestRewardItem {
    fn write_like_cpp(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.item_id);
        pkt.write_int32(self.quantity);
    }
}

/// C++ `WorldPackets::LFG::LfgPlayerQuestRewardCurrency`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LfgPlayerQuestRewardCurrency {
    pub currency_id: i32,
    pub quantity: i32,
}

impl LfgPlayerQuestRewardCurrency {
    fn write_like_cpp(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.currency_id);
        pkt.write_int32(self.quantity);
    }
}

/// C++ `WorldPackets::LFG::LfgPlayerQuestReward`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LfgPlayerQuestReward {
    pub mask: u8,
    pub reward_money: i32,
    pub reward_xp: i32,
    pub items: Vec<LfgPlayerQuestRewardItem>,
    pub currency: Vec<LfgPlayerQuestRewardCurrency>,
    pub bonus_currency: Vec<LfgPlayerQuestRewardCurrency>,
    pub reward_spell_id: Option<i32>,
    pub unused1: Option<i32>,
    pub unused2: Option<u64>,
    pub honor: Option<i32>,
}

impl LfgPlayerQuestReward {
    fn write_like_cpp(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.mask);
        pkt.write_int32(self.reward_money);
        pkt.write_int32(self.reward_xp);
        pkt.write_uint32(self.items.len() as u32);
        pkt.write_uint32(self.currency.len() as u32);
        pkt.write_uint32(self.bonus_currency.len() as u32);

        for item in &self.items {
            item.write_like_cpp(pkt);
        }

        for currency in &self.currency {
            currency.write_like_cpp(pkt);
        }

        for bonus_currency in &self.bonus_currency {
            bonus_currency.write_like_cpp(pkt);
        }

        pkt.write_bit(self.reward_spell_id.is_some());
        pkt.write_bit(self.unused1.is_some());
        pkt.write_bit(self.unused2.is_some());
        pkt.write_bit(self.honor.is_some());
        pkt.flush_bits();

        if let Some(reward_spell_id) = self.reward_spell_id {
            pkt.write_int32(reward_spell_id);
        }
        if let Some(unused1) = self.unused1 {
            pkt.write_int32(unused1);
        }
        if let Some(unused2) = self.unused2 {
            pkt.write_uint64(unused2);
        }
        if let Some(honor) = self.honor {
            pkt.write_int32(honor);
        }
    }
}

/// C++ `WorldPackets::LFG::LfgPlayerDungeonInfo`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LfgPlayerDungeonInfo {
    pub slot: u32,
    pub completion_quantity: i32,
    pub completion_limit: i32,
    pub completion_currency_id: i32,
    pub specific_quantity: i32,
    pub specific_limit: i32,
    pub overall_quantity: i32,
    pub overall_limit: i32,
    pub purse_weekly_quantity: i32,
    pub purse_weekly_limit: i32,
    pub purse_quantity: i32,
    pub purse_limit: i32,
    pub quantity: i32,
    pub completed_mask: u32,
    pub encounter_mask: u32,
    pub first_reward: bool,
    pub shortage_eligible: bool,
    pub rewards: LfgPlayerQuestReward,
    pub shortage_reward: Vec<LfgPlayerQuestReward>,
}

impl LfgPlayerDungeonInfo {
    pub fn random_dungeon_like_cpp(slot: u32) -> Self {
        Self {
            slot,
            completion_quantity: 1,
            completion_limit: 1,
            completion_currency_id: 0,
            specific_quantity: 0,
            specific_limit: 1,
            overall_quantity: 0,
            overall_limit: 1,
            purse_weekly_quantity: 0,
            purse_weekly_limit: 0,
            purse_quantity: 0,
            purse_limit: 0,
            quantity: 1,
            completed_mask: 0,
            encounter_mask: 0,
            first_reward: false,
            shortage_eligible: false,
            rewards: LfgPlayerQuestReward::default(),
            shortage_reward: Vec::new(),
        }
    }

    fn write_like_cpp(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.slot);
        pkt.write_int32(self.completion_quantity);
        pkt.write_int32(self.completion_limit);
        pkt.write_int32(self.completion_currency_id);
        pkt.write_int32(self.specific_quantity);
        pkt.write_int32(self.specific_limit);
        pkt.write_int32(self.overall_quantity);
        pkt.write_int32(self.overall_limit);
        pkt.write_int32(self.purse_weekly_quantity);
        pkt.write_int32(self.purse_weekly_limit);
        pkt.write_int32(self.purse_quantity);
        pkt.write_int32(self.purse_limit);
        pkt.write_int32(self.quantity);
        pkt.write_uint32(self.completed_mask);
        pkt.write_uint32(self.encounter_mask);
        pkt.write_uint32(self.shortage_reward.len() as u32);
        pkt.write_bit(self.first_reward);
        pkt.write_bit(self.shortage_eligible);
        pkt.flush_bits();

        self.rewards.write_like_cpp(pkt);
        for reward in &self.shortage_reward {
            reward.write_like_cpp(pkt);
        }
    }
}

/// C++ `WorldPackets::Trade::ClearTradeItem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClearTradeItem {
    pub trade_slot: u8,
}

impl ClientPacket for ClearTradeItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::ClearTradeItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            trade_slot: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Trade::SetTradeItem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SetTradeItem {
    pub trade_slot: u8,
    pub pack_slot: u8,
    pub item_slot_in_pack: u8,
}

impl ClientPacket for SetTradeItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetTradeItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            trade_slot: pkt.read_uint8()?,
            pack_slot: pkt.read_uint8()?,
            item_slot_in_pack: pkt.read_uint8()?,
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
