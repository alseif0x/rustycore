use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoulboundTradeableItemRef {
    pub guid: ObjectGuid,
    pub owner_guid: ObjectGuid,
    pub trade_expired: bool,
}

impl SoulboundTradeableItemRef {
    pub const fn new(guid: ObjectGuid, owner_guid: ObjectGuid, trade_expired: bool) -> Self {
        Self {
            guid,
            owner_guid,
            trade_expired,
        }
    }

    pub fn from_item(item: &Item, owner_total_played_time: u32) -> Self {
        Self {
            guid: item.object().guid(),
            owner_guid: item.owner_guid(),
            trade_expired: item.is_soulbound_trade_expired(owner_total_played_time),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerItemTimeUpdate {
    pub item_guid: ObjectGuid,
    pub expiration: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemDurationRef {
    pub guid: ObjectGuid,
    pub expiration: u32,
    pub real_duration: bool,
}

impl ItemDurationRef {
    pub const fn new(guid: ObjectGuid, expiration: u32, real_duration: bool) -> Self {
        Self {
            guid,
            expiration,
            real_duration,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateItemDurationAction {
    MissingItem {
        item_guid: ObjectGuid,
    },
    UpdateExpiration {
        item_guid: ObjectGuid,
        expiration: u32,
    },
    Expire {
        item_guid: ObjectGuid,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerEnchantDuration {
    pub item_guid: ObjectGuid,
    pub slot: EnchantmentSlot,
    pub left_duration_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerEnchantTimeUpdate {
    pub item_guid: ObjectGuid,
    pub slot: EnchantmentSlot,
    pub duration_secs: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerEnchantDurationItemRef {
    pub item_guid: ObjectGuid,
    pub slot: EnchantmentSlot,
    pub enchantment_id: i32,
}

impl PlayerEnchantDurationItemRef {
    pub const fn new(item_guid: ObjectGuid, slot: EnchantmentSlot, enchantment_id: i32) -> Self {
        Self {
            item_guid,
            slot,
            enchantment_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateEnchantTimeAction {
    RemoveMissingEnchantment {
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
    },
    ClearExpired {
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
    },
}

