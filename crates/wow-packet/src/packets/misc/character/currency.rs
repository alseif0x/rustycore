//! Currency packets.
//!
//! Separated from character.rs under #689.

use super::*;

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
