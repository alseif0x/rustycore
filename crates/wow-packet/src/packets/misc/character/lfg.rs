//! Lfg packets.
//!
//! Separated from character.rs under #689.

use super::*;

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
