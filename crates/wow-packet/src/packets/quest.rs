// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest system packets.
//!
//! C# ref: Game/Networking/Packets/QuestPackets.cs
//! C# ref: Game/Networking/Packets/NPCPackets.cs (ClientGossipText)

use crate::{ServerPacket, WorldPacket};
use wow_core::ObjectGuid;
use wow_constants::ServerOpcodes;

// Constants matching C# SharedConst
const QUEST_REWARD_ITEM_COUNT: usize = 4;
const QUEST_REWARD_CHOICES_COUNT: usize = 6;
const QUEST_REWARD_REPUTATIONS_COUNT: usize = 5;
const QUEST_REWARD_CURRENCY_COUNT: usize = 4;
const QUEST_REWARD_DISPLAY_SPELL_COUNT: usize = 3;

// ── SMSG_QUEST_GIVER_STATUS (0x...) ──────────────────────────────────────────

/// Quest giver status for a single NPC — controls the ! ? exclamation icons.
/// C# ref: QuestGiverStatusPkt
pub struct QuestGiverStatus {
    pub guid: ObjectGuid,
    /// Status flags: 0=None, 2=Future(grey ?), 4=Trivial(grey ?),
    /// 5=TrivialRepeatableTurnIn, 6=TrivialDaily, 8=FailedTimer,
    /// 9=FailedFail, 16=CanReward, 17=CanRewardDailyMixed,
    /// 18=CanRewardRep, 32=Available(yellow !), 64=AvailableRep,
    /// 4096=AvailableDaily(blue !)
    pub status: u64,
}

impl ServerPacket for QuestGiverStatus {
    const OPCODE: ServerOpcodes = ServerOpcodes::QuestGiverStatus;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.guid);
        pkt.write_uint64(self.status);
    }
}

// ── Quest giver status constants ──────────────────────────────────────────────
pub mod quest_giver_status {
    pub const NONE: u64           = 0;
    pub const FUTURE: u64         = 2;
    pub const TRIVIAL: u64        = 4;
    pub const FAILED_TIMER: u64   = 8;
    pub const CAN_REWARD: u64     = 16;
    pub const AVAILABLE: u64      = 32;
    pub const AVAILABLE_DAILY: u64 = 4096;
}

// ── SMSG_QUEST_GIVER_QUEST_LIST_MESSAGE ──────────────────────────────────────

/// One quest entry in the NPC quest list.
/// C# ref: ClientGossipText (used in QuestGiverQuestListMessage)
pub struct QuestListEntry {
    pub quest_id: u32,
    pub quest_type: u8,
    pub quest_level: i32,
    pub quest_max_scaling_level: i32,
    pub quest_flags: u32,
    pub quest_flags_ex: u32,
    pub repeatable: bool,
    pub title: String,
}

/// List of quests offered by an NPC.
/// C# ref: QuestGiverQuestListMessage
pub struct QuestGiverQuestList {
    pub guid: ObjectGuid,
    pub greeting: String,
    pub greet_emote_delay: u32,
    pub greet_emote_type: u32,
    pub quests: Vec<QuestListEntry>,
}

impl ServerPacket for QuestGiverQuestList {
    const OPCODE: ServerOpcodes = ServerOpcodes::QuestGiverQuestListMessage;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.guid);
        pkt.write_uint32(self.greet_emote_delay);
        pkt.write_uint32(self.greet_emote_type);
        pkt.write_uint32(self.quests.len() as u32);
        pkt.write_bits(self.greeting.len() as u32, 11);
        pkt.flush_bits();

        for q in &self.quests {
            pkt.write_uint32(q.quest_id);
            pkt.write_uint32(0); // ContentTuningID
            pkt.write_int32(q.quest_type as i32);
            pkt.write_int32(q.quest_level);
            pkt.write_int32(q.quest_max_scaling_level);
            pkt.write_uint32(q.quest_flags);
            pkt.write_uint32(q.quest_flags_ex);
            pkt.write_bit(q.repeatable);
            pkt.write_bit(false); // Important
            pkt.write_bits(q.title.len() as u32, 9);
            pkt.flush_bits();
            pkt.write_string(&q.title);
        }

        pkt.write_string(&self.greeting);
    }
}

// ── SMSG_QUEST_GIVER_QUEST_DETAILS ───────────────────────────────────────────

/// One objective shown in the QuestDetails dialog.
/// C# ref: QuestObjectiveSimple
pub struct QuestObjectiveSimple {
    pub id: u32,
    pub object_id: i32,
    pub amount: i32,
    pub obj_type: u8,
}

/// Full reward block for a quest details / offer reward packet.
/// C# ref: QuestRewards
pub struct QuestRewardsBlock {
    pub items: [(u32, u32); QUEST_REWARD_ITEM_COUNT],   // (item_id, qty)
    pub money: i32,
    pub xp: i32,
    pub honor: i32,
    pub title: i32,
    pub display_spells: [u32; QUEST_REWARD_DISPLAY_SPELL_COUNT],
    pub completion_spell: i32,
}

impl Default for QuestRewardsBlock {
    fn default() -> Self {
        Self {
            items: [(0, 0); QUEST_REWARD_ITEM_COUNT],
            money: 0,
            xp: 0,
            honor: 0,
            title: 0,
            display_spells: [0; QUEST_REWARD_DISPLAY_SPELL_COUNT],
            completion_spell: 0,
        }
    }
}

impl QuestRewardsBlock {
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(0); // ChoiceItemCount
        pkt.write_uint32(self.items.iter().filter(|i| i.0 != 0).count() as u32); // ItemCount
        for (item, qty) in &self.items {
            pkt.write_int32(*item as i32);
            pkt.write_int32(*qty as i32);
        }
        pkt.write_int32(self.money);
        pkt.write_int32(self.xp);
        pkt.write_int64(0i64);  // ArtifactXP
        pkt.write_int32(0);     // ArtifactCategoryID
        pkt.write_int32(self.honor);
        pkt.write_int32(self.title);
        pkt.write_uint32(0);    // FactionFlags
        // RewardFaction (5x4 ints = 20 ints)
        for _ in 0..QUEST_REWARD_REPUTATIONS_COUNT {
            pkt.write_int32(0); // FactionID
            pkt.write_int32(0); // FactionValue
            pkt.write_int32(0); // FactionOverride
            pkt.write_int32(0); // FactionCapIn
        }
        // SpellCompletionDisplayID (3 ints)
        for spell in &self.display_spells {
            pkt.write_int32(*spell as i32);
        }
        pkt.write_int32(self.completion_spell);
        // Currency (4x2 ints)
        for _ in 0..QUEST_REWARD_CURRENCY_COUNT {
            pkt.write_int32(0); // CurrencyID
            pkt.write_int32(0); // CurrencyQty
        }
        pkt.write_int32(0); // SkillLineID
        pkt.write_int32(0); // NumSkillUps
        pkt.write_int32(0); // TreasurePickerID
        // ChoiceItems (6 entries, each: ItemID, Quantity, DisplayID)
        for _ in 0..QUEST_REWARD_CHOICES_COUNT {
            // QuestChoiceItem.Write — 5 ints + flags
            pkt.write_int32(0); // Item.ItemID
            pkt.write_int32(0); // Item.Quantity (ItemCount)
            pkt.write_uint64(0u64); // Item.Mask (ItemContext bits)
            pkt.write_uint32(0); // Item.Bonuses count
            pkt.write_int32(0); // DisplayID
            pkt.write_int32(0); // Unused
        }
        pkt.write_bit(false); // IsBoostSpell
        pkt.flush_bits();
    }
}

/// Full quest details packet shown when clicking a quest name in the list.
/// C# ref: QuestGiverQuestDetails
pub struct QuestGiverQuestDetails {
    pub giver_guid: ObjectGuid,
    pub quest_id: u32,
    pub quest_flags: [u32; 3],
    pub suggested_party_members: u8,
    pub objectives: Vec<QuestObjectiveSimple>,
    pub rewards: QuestRewardsBlock,
    pub title: String,
    pub description: String,
    pub log_description: String,
    pub auto_launched: bool,
}

impl ServerPacket for QuestGiverQuestDetails {
    const OPCODE: ServerOpcodes = ServerOpcodes::QuestGiverQuestDetails;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.giver_guid);
        pkt.write_packed_guid(&ObjectGuid::EMPTY); // InformUnit
        pkt.write_uint32(self.quest_id);
        pkt.write_int32(0);  // QuestPackageID
        pkt.write_int32(0);  // PortraitGiver
        pkt.write_int32(0);  // PortraitGiverMount
        pkt.write_int32(0);  // PortraitGiverModelSceneID
        pkt.write_int32(0);  // PortraitTurnIn
        pkt.write_uint32(self.quest_flags[0]);
        pkt.write_uint32(self.quest_flags[1]);
        pkt.write_uint32(self.quest_flags[2]);
        pkt.write_int32(self.suggested_party_members as i32);
        pkt.write_int32(0);  // LearnSpells count
        pkt.write_int32(0);  // DescEmotes count
        pkt.write_int32(self.objectives.len() as i32);
        pkt.write_int32(0);  // QuestStartItemID
        pkt.write_int32(0);  // QuestSessionBonus
        pkt.write_int32(0);  // QuestGiverCreatureID
        pkt.write_int32(0);  // ConditionalDescriptionText count

        // Objectives
        for obj in &self.objectives {
            pkt.write_uint32(obj.id);
            pkt.write_int32(obj.object_id);
            pkt.write_int32(obj.amount);
            pkt.write_uint8(obj.obj_type);
        }

        // Bit fields
        pkt.write_bits(self.title.len() as u32, 9);
        pkt.write_bits(self.description.len() as u32, 12);
        pkt.write_bits(self.log_description.len() as u32, 12);
        pkt.write_bits(0u32, 10); // PortraitGiverText len
        pkt.write_bits(0u32, 8);  // PortraitGiverName len
        pkt.write_bits(0u32, 10); // PortraitTurnInText len
        pkt.write_bits(0u32, 8);  // PortraitTurnInName len
        pkt.write_bit(self.auto_launched);
        pkt.write_bit(false); // unused
        pkt.write_bit(false); // StartCheat
        pkt.write_bit(false); // DisplayPopup
        pkt.flush_bits();

        // Rewards block
        self.rewards.write(pkt);

        // Strings
        pkt.write_string(&self.title);
        pkt.write_string(&self.description);
        pkt.write_string(&self.log_description);
        // PortraitGiverText / Name / TurnInText / Name (all empty)
    }
}

// ── SMSG_QUEST_GIVER_QUEST_COMPLETE ──────────────────────────────────────────

/// Shown after accepting a quest — "Quest Accepted" popup.
/// C# ref: QuestGiverQuestComplete
pub struct QuestGiverQuestComplete {
    pub quest_id: u32,
    pub xp: u32,
    pub money: u32,
    pub skill_points: u32,
    pub use_quest_reward_currency: bool,
}

impl ServerPacket for QuestGiverQuestComplete {
    const OPCODE: ServerOpcodes = ServerOpcodes::QuestGiverQuestComplete;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.quest_id);
        pkt.write_uint32(self.xp);
        pkt.write_int64(self.money as i64);
        pkt.write_uint32(self.skill_points);
        pkt.write_uint32(0); // SkillLineID
        pkt.write_bit(self.use_quest_reward_currency);
        pkt.write_bit(false); // LaunchGossip
        pkt.write_bit(false); // HideChatMessage
        pkt.write_bit(false); // ShowKeybind
        pkt.flush_bits();
    }
}

// ── SMSG_QUERY_QUEST_INFO_RESPONSE ───────────────────────────────────────────

/// One quest objective as sent in QueryQuestInfoResponse.
pub struct QuestObjectiveInfo {
    pub id: u32,
    pub obj_type: u8,
    pub storage_index: i8,
    pub object_id: i32,
    pub amount: i32,
    pub flags: u32,
    pub flags2: u32,
    pub progress_bar_weight: f32,
    pub description: String,
}

/// Response to CMSG_QUERY_QUEST_INFO — full quest data.
/// C# ref: QueryQuestInfoResponse
#[derive(Default)]
pub struct QueryQuestInfoResponse {
    pub quest_id: u32,
    pub allow: bool,
    // Quest info fields (only written when allow=true)
    pub quest_type: u8,
    pub quest_level: i32,
    pub quest_max_scaling_level: i32,
    pub min_level: i32,
    pub quest_sort_id: i32,
    pub quest_info_id: u16,
    pub suggested_group_num: u8,
    pub reward_next_quest: u32,
    pub reward_xp_difficulty: u32,
    pub reward_money_difficulty: u32,
    pub flags: u32,
    pub flags_ex: u32,
    pub flags_ex2: u32,
    pub reward_items: [u32; QUEST_REWARD_ITEM_COUNT],
    pub reward_amounts: [u32; QUEST_REWARD_ITEM_COUNT],
    pub reward_display_spell: [u32; QUEST_REWARD_DISPLAY_SPELL_COUNT],
    pub reward_spell: u32,
    pub objectives: Vec<QuestObjectiveInfo>,
    pub log_title: String,
    pub log_description: String,
    pub quest_description: String,
    pub area_description: String,
    pub quest_completion_log: String,
}

impl ServerPacket for QueryQuestInfoResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::QueryQuestInfoResponse;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.quest_id);
        pkt.write_bit(self.allow);
        pkt.flush_bits();

        if !self.allow { return; }

        pkt.write_int32(self.quest_id as i32);
        pkt.write_int32(self.quest_type as i32);
        pkt.write_int32(self.quest_level);
        pkt.write_int32(0);  // QuestScalingFactionGroup
        pkt.write_int32(self.quest_max_scaling_level);
        pkt.write_int32(0);  // QuestPackageID
        pkt.write_int32(self.min_level);
        pkt.write_int32(self.quest_sort_id);
        pkt.write_int32(self.quest_info_id as i32);
        pkt.write_int32(self.suggested_group_num as i32);
        pkt.write_int32(self.reward_next_quest as i32);
        pkt.write_int32(self.reward_xp_difficulty as i32);
        pkt.write_float(1.0); // RewardXPMultiplier
        pkt.write_int32(0);   // RewardMoney (base; not difficulty)
        pkt.write_int32(self.reward_money_difficulty as i32);
        pkt.write_float(1.0); // RewardMoneyMultiplier
        pkt.write_int32(0);   // RewardBonusMoney
        // RewardDisplaySpell (3)
        for s in &self.reward_display_spell { pkt.write_int32(*s as i32); }
        pkt.write_int32(self.reward_spell as i32);
        pkt.write_int32(0);   // RewardHonor
        pkt.write_float(0.0); // RewardKillHonor
        pkt.write_int32(0);   // RewardArtifactXPDifficulty
        pkt.write_float(1.0); // RewardArtifactXPMultiplier
        pkt.write_int32(0);   // RewardArtifactCategoryID
        pkt.write_int32(0);   // StartItem
        pkt.write_uint32(self.flags);
        pkt.write_uint32(self.flags_ex);
        pkt.write_uint32(self.flags_ex2);
        // RewardItems (4x: ItemID, Amount, ItemDrop, ItemDropQty)
        for i in 0..QUEST_REWARD_ITEM_COUNT {
            pkt.write_int32(self.reward_items[i] as i32);
            pkt.write_int32(self.reward_amounts[i] as i32);
            pkt.write_int32(0); // ItemDrop
            pkt.write_int32(0); // ItemDropQuantity
        }
        // RewardChoiceItems (6x: ItemID, Quantity, DisplayID)
        for _ in 0..QUEST_REWARD_CHOICES_COUNT {
            pkt.write_int32(0);
            pkt.write_int32(0);
            pkt.write_int32(0);
        }
        pkt.write_int32(0); // POIContinent
        pkt.write_float(0.0); // POIx
        pkt.write_float(0.0); // POIy
        pkt.write_int32(0); // POIPriority
        pkt.write_int32(0); // RewardTitle
        pkt.write_int32(0); // RewardArenaPoints
        pkt.write_int32(0); // RewardSkillLineID
        pkt.write_int32(0); // RewardNumSkillUps
        pkt.write_int32(0); // PortraitGiver
        pkt.write_int32(0); // PortraitGiverMount
        pkt.write_int32(0); // PortraitGiverModelSceneID
        pkt.write_int32(0); // PortraitTurnIn
        // RewardFaction (5x4)
        for _ in 0..QUEST_REWARD_REPUTATIONS_COUNT {
            pkt.write_int32(0); pkt.write_int32(0);
            pkt.write_int32(0); pkt.write_int32(0);
        }
        pkt.write_uint32(0); // RewardFactionFlags
        // RewardCurrency (4x2)
        for _ in 0..QUEST_REWARD_CURRENCY_COUNT {
            pkt.write_int32(0); pkt.write_int32(0);
        }
        pkt.write_int32(0); // AcceptedSoundKitID
        pkt.write_int32(0); // CompleteSoundKitID
        pkt.write_int32(0); // AreaGroupID
        pkt.write_int64(0i64); // TimeAllowed
        pkt.write_int32(self.objectives.len() as i32);
        pkt.write_uint64(0u64); // AllowableRaces
        pkt.write_int32(0); // TreasurePickerID
        pkt.write_int32(0); // Expansion
        pkt.write_int32(0); // ManagedWorldStateID
        pkt.write_int32(0); // QuestSessionBonus
        pkt.write_int32(0); // QuestGiverCreatureID

        // Bit string lengths
        pkt.write_bits(self.log_title.len() as u32, 9);
        pkt.write_bits(self.log_description.len() as u32, 12);
        pkt.write_bits(self.quest_description.len() as u32, 12);
        pkt.write_bits(self.area_description.len() as u32, 9);
        pkt.write_bits(0u32, 10); // PortraitGiverText
        pkt.write_bits(0u32, 8);  // PortraitGiverName
        pkt.write_bits(0u32, 10); // PortraitTurnInText
        pkt.write_bits(0u32, 8);  // PortraitTurnInName
        pkt.write_bits(self.quest_completion_log.len() as u32, 11);
        pkt.flush_bits();

        // Objectives
        for obj in &self.objectives {
            pkt.write_uint32(obj.id);
            pkt.write_uint8(obj.obj_type);
            pkt.write_int8(obj.storage_index);
            pkt.write_int32(obj.object_id);
            pkt.write_int32(obj.amount);
            pkt.write_uint32(obj.flags);
            pkt.write_uint32(obj.flags2);
            pkt.write_float(obj.progress_bar_weight);
            pkt.write_int32(0); // VisualEffects count
            pkt.write_bits(obj.description.len() as u32, 8);
            pkt.flush_bits();
            pkt.write_string(&obj.description);
        }

        // Strings
        pkt.write_string(&self.log_title);
        pkt.write_string(&self.log_description);
        pkt.write_string(&self.quest_description);
        pkt.write_string(&self.area_description);
        // PortraitGiverText / Name / TurnInText / Name (empty)
        pkt.write_string(&self.quest_completion_log);
    }
}
