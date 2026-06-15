// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest system packets.
//!
//! C# ref: Game/Networking/Packets/QuestPackets.cs
//! C# ref: Game/Networking/Packets/NPCPackets.cs (ClientGossipText)

use crate::{ClientPacket, PacketError, ServerPacket, WorldPacket};
use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::ObjectGuid;

// Constants matching C# SharedConst
const QUEST_REWARD_ITEM_COUNT: usize = 4;
const QUEST_REWARD_CHOICES_COUNT: usize = 6;
const QUEST_REWARD_REPUTATIONS_COUNT: usize = 5;
const QUEST_REWARD_CURRENCY_COUNT: usize = 4;
const QUEST_REWARD_DISPLAY_SPELL_COUNT: usize = 3;

/// Client request to start an Adventure Map quest.
///
/// C++ anchor: `WorldPackets::AdventureMap::AdventureMapStartQuest::Read`,
/// `AdventureMapPackets.cpp:24-27`: exactly one signed `int32 QuestID`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdventureMapStartQuest {
    pub quest_id: i32,
}

impl ClientPacket for AdventureMapStartQuest {
    const OPCODE: ClientOpcodes = ClientOpcodes::AdventureMapStartQuest;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let quest_id = packet.read_int32()?;

        Ok(Self { quest_id })
    }
}

/// Client confirmation that accepts a shared quest prompt.
///
/// C++ anchor: `WorldPackets::Quest::QuestConfirmAccept::Read`,
/// `QuestPackets.cpp:603-606`: exactly one signed `int32 QuestID`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestConfirmAccept {
    pub quest_id: i32,
}

impl ClientPacket for QuestConfirmAccept {
    const OPCODE: ClientOpcodes = ClientOpcodes::QuestConfirmAccept;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let quest_id = packet.read_int32()?;

        Ok(Self { quest_id })
    }
}

/// Client response to a shared-quest prompt.
///
/// C++ anchor: `WorldPackets::Quest::QuestPushResult::Read`,
/// `QuestPackets.cpp:621-626`: `SenderGUID`, then `uint32 QuestID`, then `uint8 Result`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestPushResult {
    pub sender_guid: ObjectGuid,
    pub quest_id: u32,
    pub result: u8,
}

/// C++ `QuestPushReason` values used by `SMSG_QUEST_PUSH_RESULT`.
///
/// C++ anchor: `QuestDef.h:75-96`.
pub mod quest_push_reason {
    pub const NOT_DAILY: u8 = 14;
    pub const NOT_IN_PARTY: u8 = 16;
    pub const NOT_ALLOWED: u8 = 19;
}

/// Server response emitted to the quest-sharing sender/receiver.
///
/// C++ anchors:
/// - `Player::SendPushToPartyResponse`, `Player.cpp:16735-16752`: sender GUID,
///   result, optional localized quest title, then direct send.
/// - `QuestPushResultResponse::Write`, `QuestPackets.cpp:608-618`: packed GUID,
///   `uint8 Result`, 9-bit title length, flush bits, raw title string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestPushResultResponse {
    pub sender_guid: ObjectGuid,
    pub result: u8,
    pub quest_title: String,
}

impl ServerPacket for QuestPushResultResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::QuestPushResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.sender_guid);
        pkt.write_uint8(self.result);
        pkt.write_bits(self.quest_title.len() as u32, 9);
        pkt.flush_bits();
        pkt.write_string(&self.quest_title);
    }
}

impl ClientPacket for QuestPushResult {
    const OPCODE: ClientOpcodes = ClientOpcodes::QuestPushResult;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let sender_guid = packet.read_packed_guid()?;
        let quest_id = packet.read_uint32()?;
        let result = packet.read_uint8()?;

        Ok(Self {
            sender_guid,
            quest_id,
            result,
        })
    }
}

/// Sender request to share one quest with the current party.
///
/// C++ anchor: `WorldPackets::Quest::PushQuestToParty::Read`,
/// `QuestPackets.cpp:658-661`: exactly one `uint32 QuestID`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PushQuestToParty {
    pub quest_id: u32,
}

impl ClientPacket for PushQuestToParty {
    const OPCODE: ClientOpcodes = ClientOpcodes::PushQuestToParty;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let quest_id = packet.read_uint32()?;

        Ok(Self { quest_id })
    }
}

// ── SMSG_QUEST_GIVER_STATUS (0x...) ──────────────────────────────────────────

/// Quest giver status for a single NPC — controls the ! ? exclamation icons.
/// C# ref: QuestGiverStatusPkt
pub struct QuestGiverStatus {
    pub guid: ObjectGuid,
    /// C++ `QuestGiverStatus` bitmask from `QuestDef.h`.
    pub status: u64,
}

impl ServerPacket for QuestGiverStatus {
    const OPCODE: ServerOpcodes = ServerOpcodes::QuestGiverStatus;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.guid);
        pkt.write_uint64(self.status);
    }
}

/// Quest giver statuses for every currently visible questgiver.
///
/// C++ anchor: `WorldPackets::Quest::QuestGiverStatusMultiple::Write`,
/// `QuestPackets.cpp:64-74`: `int32` count followed by packed GUID and
/// `uint64` status for each questgiver.
pub struct QuestGiverStatusMultiple {
    pub statuses: Vec<(ObjectGuid, u64)>,
}

impl ServerPacket for QuestGiverStatusMultiple {
    const OPCODE: ServerOpcodes = ServerOpcodes::QuestGiverStatusMultiple;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.statuses.len() as i32);
        for (guid, status) in &self.statuses {
            pkt.write_packed_guid(guid);
            pkt.write_uint64(*status);
        }
    }
}

/// One entry in `SMSG_WORLD_QUEST_UPDATE_RESPONSE`.
///
/// C++ anchor: `WorldPackets::Quest::WorldQuestUpdateInfo`,
/// `QuestPackets.h:663-673`: `Timestamp<> LastUpdate`, `uint32 QuestID`,
/// `uint32 Timer`, `int32 VariableID`, `int32 Value`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldQuestUpdateInfo {
    pub last_update: i64,
    pub quest_id: u32,
    pub timer: u32,
    pub variable_id: i32,
    pub value: i32,
}

/// Empty represented response for `CMSG_REQUEST_WORLD_QUEST_UPDATE`.
///
/// C++ anchor: `WorldQuestUpdateResponse::Write`,
/// `QuestPackets.cpp:677-690`: `uint32` update count followed by each
/// `WorldQuestUpdateInfo`. The current Trinity handler leaves this vector empty.
pub struct WorldQuestUpdateResponse {
    pub updates: Vec<WorldQuestUpdateInfo>,
}

impl ServerPacket for WorldQuestUpdateResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::WorldQuestUpdateResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.updates.len() as u32);
        for update in &self.updates {
            pkt.write_int64(update.last_update);
            pkt.write_uint32(update.quest_id);
            pkt.write_uint32(update.timer);
            pkt.write_int32(update.variable_id);
            pkt.write_int32(update.value);
        }
    }
}

// ── Quest giver status constants ──────────────────────────────────────────────
pub mod quest_giver_status {
    pub const NONE: u64 = 0;
    pub const FUTURE: u64 = 0x0000_0002;
    pub const TRIVIAL: u64 = 0x0000_0004;
    pub const TRIVIAL_REPEATABLE_TURNIN: u64 = 0x0000_0008;
    pub const TRIVIAL_DAILY_QUEST: u64 = 0x0000_0010;
    pub const REWARD: u64 = 0x0000_0020;
    pub const JOURNEY_REWARD: u64 = 0x0000_0040;
    pub const COVENANT_CALLING_REWARD: u64 = 0x0000_0080;
    pub const REPEATABLE_TURNIN: u64 = 0x0000_0100;
    pub const DAILY_QUEST: u64 = 0x0000_0200;
    pub const QUEST: u64 = 0x0000_0400;
    pub const REWARD_COMPLETE_NO_POI: u64 = 0x0000_0800;
    pub const REWARD_COMPLETE_POI: u64 = 0x0000_1000;
    pub const LEGENDARY_QUEST: u64 = 0x0000_2000;
    pub const LEGENDARY_REWARD_COMPLETE_NO_POI: u64 = 0x0000_4000;
    pub const LEGENDARY_REWARD_COMPLETE_POI: u64 = 0x0000_8000;
    pub const JOURNEY_QUEST: u64 = 0x0001_0000;
    pub const JOURNEY_REWARD_COMPLETE_NO_POI: u64 = 0x0002_0000;
    pub const JOURNEY_REWARD_COMPLETE_POI: u64 = 0x0004_0000;
    pub const COVENANT_CALLING_QUEST: u64 = 0x0008_0000;
    pub const COVENANT_CALLING_REWARD_COMPLETE_NO_POI: u64 = 0x0010_0000;
    pub const COVENANT_CALLING_REWARD_COMPLETE_POI: u64 = 0x0020_0000;
    pub const TRIVIAL_LEGENDARY_QUEST: u64 = 0x0040_0000;
    pub const FUTURE_LEGENDARY_QUEST: u64 = 0x0080_0000;
    pub const LEGENDARY_REWARD: u64 = 0x0100_0000;
    pub const IMPORTANT_QUEST: u64 = 0x0200_0000;
    pub const IMPORTANT_REWARD: u64 = 0x0400_0000;
    pub const TRIVIAL_IMPORTANT_QUEST: u64 = 0x0800_0000;
    pub const FUTURE_IMPORTANT_QUEST: u64 = 0x1000_0000;
    pub const IMPORTANT_QUEST_REWARD_COMPLETE_NO_POI: u64 = 0x2000_0000;
    pub const IMPORTANT_QUEST_REWARD_COMPLETE_POI: u64 = 0x4000_0000;
    pub const TRIVIAL_JOURNEY_QUEST: u64 = 0x8000_0000;
    pub const FUTURE_JOURNEY_QUEST: u64 = 0x1_0000_0000;

    pub const AVAILABLE: u64 = QUEST;
    pub const AVAILABLE_DAILY: u64 = DAILY_QUEST;
    pub const CAN_REWARD: u64 = REWARD_COMPLETE_POI;
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
    pub items: [(u32, u32); QUEST_REWARD_ITEM_COUNT], // (item_id, qty) — fixed rewards
    pub choice_items: [(u32, u32); QUEST_REWARD_CHOICES_COUNT], // (item_id, qty) — player picks one
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
            choice_items: [(0, 0); QUEST_REWARD_CHOICES_COUNT],
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
        let choice_count = self.choice_items.iter().filter(|i| i.0 != 0).count() as u32;
        pkt.write_uint32(choice_count); // ChoiceItemCount
        pkt.write_uint32(self.items.iter().filter(|i| i.0 != 0).count() as u32); // ItemCount
        for (item, qty) in &self.items {
            pkt.write_int32(*item as i32);
            pkt.write_int32(*qty as i32);
        }
        pkt.write_int32(self.money);
        pkt.write_int32(self.xp);
        pkt.write_int64(0i64); // ArtifactXP
        pkt.write_int32(0); // ArtifactCategoryID
        pkt.write_int32(self.honor);
        pkt.write_int32(self.title);
        pkt.write_uint32(0); // FactionFlags
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
        // ChoiceItems (6 entries, each: ItemID, Quantity, Context+Bonuses, DisplayID, Unused)
        // C# ref: QuestChoiceItem.Write / ItemInstance.Write
        for (item_id, qty) in &self.choice_items {
            pkt.write_int32(*item_id as i32); // Item.ItemID
            pkt.write_int32(*qty as i32); // Item.Quantity
            pkt.write_uint64(0u64); // Item.Mask (ItemContext bits)
            pkt.write_uint32(0); // Item.Bonuses count
            pkt.write_int32(0); // DisplayID
            pkt.write_int32(0); // Unused (LootItemType 0=Item)
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
        pkt.write_int32(0); // QuestPackageID
        pkt.write_int32(0); // PortraitGiver
        pkt.write_int32(0); // PortraitGiverMount
        pkt.write_int32(0); // PortraitGiverModelSceneID
        pkt.write_int32(0); // PortraitTurnIn
        pkt.write_uint32(self.quest_flags[0]);
        pkt.write_uint32(self.quest_flags[1]);
        pkt.write_uint32(self.quest_flags[2]);
        pkt.write_int32(self.suggested_party_members as i32);
        pkt.write_int32(0); // LearnSpells count
        pkt.write_int32(0); // DescEmotes count
        pkt.write_int32(self.objectives.len() as i32);
        pkt.write_int32(0); // QuestStartItemID
        pkt.write_int32(0); // QuestSessionBonus
        pkt.write_int32(0); // QuestGiverCreatureID
        pkt.write_int32(0); // ConditionalDescriptionText count

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
        pkt.write_bits(0u32, 8); // PortraitGiverName len
        pkt.write_bits(0u32, 10); // PortraitTurnInText len
        pkt.write_bits(0u32, 8); // PortraitTurnInName len
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
    pub skill_line_id: u32,
    pub skill_points: u32,
    pub use_quest_reward_currency: bool,
}

impl ServerPacket for QuestGiverQuestComplete {
    const OPCODE: ServerOpcodes = ServerOpcodes::QuestGiverQuestComplete;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.quest_id);
        pkt.write_uint32(self.xp);
        pkt.write_int64(self.money as i64);
        pkt.write_uint32(self.skill_line_id);
        pkt.write_uint32(self.skill_points);
        pkt.write_bit(self.use_quest_reward_currency);
        pkt.write_bit(false); // LaunchGossip
        pkt.write_bit(false); // HideChatMessage
        pkt.write_bit(false); // ShowKeybind
        pkt.flush_bits();
    }
}

// ── SMSG_QUEST_GIVER_QUEST_FAILED ───────────────────────────────────────────

/// Quest turn-in failure reason.
///
/// C++ anchor: `QuestGiverQuestFailed::Write`, `QuestPackets.cpp:650-655`:
/// `uint32 QuestID`, then `uint32 Reason`.
pub struct QuestGiverQuestFailed {
    pub quest_id: u32,
    pub reason: u32,
}

impl ServerPacket for QuestGiverQuestFailed {
    const OPCODE: ServerOpcodes = ServerOpcodes::QuestGiverQuestFailed;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.quest_id);
        pkt.write_uint32(self.reason);
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
    pub reward_faction_ids: [u32; QUEST_REWARD_REPUTATIONS_COUNT],
    pub reward_faction_values: [i32; QUEST_REWARD_REPUTATIONS_COUNT],
    pub reward_faction_overrides: [i32; QUEST_REWARD_REPUTATIONS_COUNT],
    pub reward_faction_cap_in: [i32; QUEST_REWARD_REPUTATIONS_COUNT],
    pub reward_faction_flags: u32,
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

        if !self.allow {
            return;
        }

        pkt.write_int32(self.quest_id as i32);
        pkt.write_int32(self.quest_type as i32);
        pkt.write_int32(self.quest_level);
        pkt.write_int32(0); // QuestScalingFactionGroup
        pkt.write_int32(self.quest_max_scaling_level);
        pkt.write_int32(0); // QuestPackageID
        pkt.write_int32(self.min_level);
        pkt.write_int32(self.quest_sort_id);
        pkt.write_int32(self.quest_info_id as i32);
        pkt.write_int32(self.suggested_group_num as i32);
        pkt.write_int32(self.reward_next_quest as i32);
        pkt.write_int32(self.reward_xp_difficulty as i32);
        pkt.write_float(1.0); // RewardXPMultiplier
        pkt.write_int32(0); // RewardMoney (base; not difficulty)
        pkt.write_int32(self.reward_money_difficulty as i32);
        pkt.write_float(1.0); // RewardMoneyMultiplier
        pkt.write_int32(0); // RewardBonusMoney
        // RewardDisplaySpell (3)
        for s in &self.reward_display_spell {
            pkt.write_int32(*s as i32);
        }
        pkt.write_int32(self.reward_spell as i32);
        pkt.write_int32(0); // RewardHonor
        pkt.write_float(0.0); // RewardKillHonor
        pkt.write_int32(0); // RewardArtifactXPDifficulty
        pkt.write_float(1.0); // RewardArtifactXPMultiplier
        pkt.write_int32(0); // RewardArtifactCategoryID
        pkt.write_int32(0); // StartItem
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
        for i in 0..QUEST_REWARD_REPUTATIONS_COUNT {
            pkt.write_int32(self.reward_faction_ids[i] as i32);
            pkt.write_int32(self.reward_faction_values[i]);
            pkt.write_int32(self.reward_faction_overrides[i]);
            pkt.write_int32(self.reward_faction_cap_in[i]);
        }
        pkt.write_uint32(self.reward_faction_flags); // RewardFactionFlags
        // RewardCurrency (4x2)
        for _ in 0..QUEST_REWARD_CURRENCY_COUNT {
            pkt.write_int32(0);
            pkt.write_int32(0);
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
        pkt.write_bits(0u32, 8); // PortraitGiverName
        pkt.write_bits(0u32, 10); // PortraitTurnInText
        pkt.write_bits(0u32, 8); // PortraitTurnInName
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

// ── SMSG_QUEST_GIVER_OFFER_REWARD_MESSAGE ────────────────────────────────────

/// "Quest Complete" dialog — shows rewards and a "Complete Quest" button.
/// C# ref: QuestGiverOfferRewardMessage / QuestGiverOfferReward
pub struct QuestGiverOfferReward {
    pub giver_guid: ObjectGuid,
    pub quest_id: u32,
    pub quest_flags: [u32; 3],
    pub suggested_party_members: u8,
    pub rewards: QuestRewardsBlock,
    pub title: String,
    pub reward_text: String,
    pub auto_launched: bool,
}

impl ServerPacket for QuestGiverOfferReward {
    const OPCODE: ServerOpcodes = ServerOpcodes::QuestGiverOfferRewardMessage;
    fn write(&self, pkt: &mut WorldPacket) {
        // QuestGiverOfferReward inner block
        pkt.write_packed_guid(&self.giver_guid);
        pkt.write_int32(0); // QuestGiverCreatureID
        pkt.write_uint32(self.quest_id);
        pkt.write_uint32(self.quest_flags[0]);
        pkt.write_uint32(self.quest_flags[1]);
        pkt.write_uint32(self.quest_flags[2]);
        pkt.write_int32(self.suggested_party_members as i32);
        pkt.write_int32(0); // Emotes count
        pkt.write_bit(self.auto_launched);
        pkt.write_bit(false); // unused
        pkt.flush_bits();
        self.rewards.write(pkt);

        // Outer wrapper fields
        pkt.write_int32(0); // QuestPackageID
        pkt.write_int32(0); // PortraitGiver
        pkt.write_int32(0); // PortraitGiverMount
        pkt.write_int32(0); // PortraitGiverModelSceneID
        pkt.write_int32(0); // PortraitTurnIn
        pkt.write_int32(0); // QuestGiverCreatureID (outer)
        pkt.write_int32(0); // ConditionalRewardText count

        pkt.write_bits(self.title.len() as u32, 9);
        pkt.write_bits(self.reward_text.len() as u32, 12);
        pkt.write_bits(0u32, 10); // PortraitGiverText
        pkt.write_bits(0u32, 8); // PortraitGiverName
        pkt.write_bits(0u32, 10); // PortraitTurnInText
        pkt.write_bits(0u32, 8); // PortraitTurnInName
        pkt.flush_bits();

        pkt.write_string(&self.title);
        pkt.write_string(&self.reward_text);
    }
}

// ── SMSG_QUEST_GIVER_REQUEST_ITEMS ───────────────────────────────────────────

/// One item objective row in `SMSG_QUEST_GIVER_REQUEST_ITEMS`.
///
/// C++ anchor: `QuestPackets.cpp:509-514` writes `ObjectID`, `Amount`, `Flags`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestGiverRequestItemsCollect {
    pub object_id: i32,
    pub amount: i32,
    pub flags: u32,
}

/// One currency objective row in `SMSG_QUEST_GIVER_REQUEST_ITEMS`.
///
/// C++ anchor: `QuestPackets.cpp:516-520` writes `CurrencyID`, `Amount`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestGiverRequestItemsCurrency {
    pub currency_id: i32,
    pub amount: i32,
}

/// Shown when player tries to complete a quest but hasn't finished all objectives.
///
/// C++ anchors:
/// - `GossipDef.cpp:605-689` fills title/completion text, collect/currency/money,
///   status flags and `AutoLaunched`.
/// - `QuestPackets.cpp:493-538` defines the exact wire layout.
pub struct QuestGiverRequestItems {
    pub giver_guid: ObjectGuid,
    pub giver_creature_id: i32,
    pub quest_id: u32,
    pub comp_emote_delay: i32,
    pub comp_emote_type: i32,
    pub quest_flags: [u32; 3],
    pub suggested_party_members: u8,
    pub money_to_get: i32,
    pub collect: Vec<QuestGiverRequestItemsCollect>,
    pub currency: Vec<QuestGiverRequestItemsCurrency>,
    pub status_flags: u32,
    pub title: String,
    pub completion_text: String,
    pub auto_launched: bool,
}

impl ServerPacket for QuestGiverRequestItems {
    const OPCODE: ServerOpcodes = ServerOpcodes::QuestGiverRequestItems;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.giver_guid);
        pkt.write_int32(self.giver_creature_id);
        pkt.write_int32(self.quest_id as i32);
        pkt.write_int32(self.comp_emote_delay);
        pkt.write_int32(self.comp_emote_type);
        pkt.write_uint32(self.quest_flags[0]);
        pkt.write_uint32(self.quest_flags[1]);
        pkt.write_uint32(self.quest_flags[2]);
        pkt.write_int32(self.suggested_party_members as i32);
        pkt.write_int32(self.money_to_get);
        pkt.write_int32(self.collect.len() as i32);
        pkt.write_int32(self.currency.len() as i32);
        pkt.write_int32(self.status_flags as i32);

        for objective in &self.collect {
            pkt.write_int32(objective.object_id);
            pkt.write_int32(objective.amount);
            pkt.write_uint32(objective.flags);
        }

        for currency in &self.currency {
            pkt.write_int32(currency.currency_id);
            pkt.write_int32(currency.amount);
        }

        pkt.write_bit(self.auto_launched);
        pkt.flush_bits();

        pkt.write_int32(self.giver_creature_id);
        pkt.write_uint32(0); // ConditionalCompletionText count (not represented yet)

        pkt.write_bits(self.title.len() as u32, 9);
        pkt.write_bits(self.completion_text.len() as u32, 12);
        pkt.flush_bits();

        pkt.write_string(&self.title);
        pkt.write_string(&self.completion_text);
    }
}

// ── SMSG_QUEST_UPDATE_COMPLETE ────────────────────────────────────────────────

/// Notifies client that a quest in the log is now complete (green checkmark).
pub struct QuestUpdateComplete {
    pub quest_id: u32,
}

impl ServerPacket for QuestUpdateComplete {
    const OPCODE: ServerOpcodes = ServerOpcodes::QuestUpdateComplete;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.quest_id);
    }
}

// ── SMSG_QUEST_UPDATE_ADD_CREDIT (0x2a8c) ────────────────────────────────────

/// Sent when a quest objective gets progress (kill, item loot, etc.).
/// Updates the count shown in the quest tracker.
/// C# ref: QuestUpdateAddCredit
pub struct QuestUpdateAddCredit {
    pub victim_guid: ObjectGuid,
    pub quest_id: u32,
    pub object_id: i32,
    pub count: u16,
    pub required: u16,
    pub objective_type: u8,
}

impl ServerPacket for QuestUpdateAddCredit {
    const OPCODE: ServerOpcodes = ServerOpcodes::QuestUpdateAddCredit;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.victim_guid);
        pkt.write_uint32(self.quest_id);
        pkt.write_int32(self.object_id);
        pkt.write_uint16(self.count);
        pkt.write_uint16(self.required);
        pkt.write_uint8(self.objective_type);
    }
}

// ── SMSG_QUEST_UPDATE_ADD_CREDIT_SIMPLE (0x2a8d) ────────────────────────────

/// C++ `WorldPackets::Quest::QuestUpdateAddCreditSimple`.
pub struct QuestUpdateAddCreditSimple {
    pub quest_id: u32,
    pub object_id: i32,
    pub objective_type: u8,
}

impl ServerPacket for QuestUpdateAddCreditSimple {
    const OPCODE: ServerOpcodes = ServerOpcodes::QuestUpdateAddCreditSimple;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.quest_id as i32);
        pkt.write_int32(self.object_id);
        pkt.write_uint8(self.objective_type);
    }
}

// ── SMSG_QUEST_UPDATE_ADD_PVP_CREDIT (0x2a8e) ────────────────────────────────

/// C++ `WorldPackets::Quest::QuestUpdateAddPvPCredit`.
pub struct QuestUpdateAddPvpCredit {
    pub quest_id: u32,
    pub count: u16,
}

impl ServerPacket for QuestUpdateAddPvpCredit {
    const OPCODE: ServerOpcodes = ServerOpcodes::QuestUpdateAddPvpCredit;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.quest_id as i32);
        pkt.write_uint16(self.count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_core::guid::HighGuid;

    #[test]
    fn quest_giver_status_multiple_writes_status_as_uint64_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1234, 5678);
        let status = 0x1_0000_0020_u64;
        let bytes = QuestGiverStatusMultiple {
            statuses: vec![(guid, status)],
        }
        .to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::QuestGiverStatusMultiple as u16
        );
        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_int32().unwrap(), 1);
        assert_eq!(pkt.read_packed_guid().unwrap(), guid);
        assert_eq!(pkt.read_uint64().unwrap(), status);
    }

    #[test]
    fn quest_giver_status_constants_match_cpp_quest_def_like_cpp() {
        assert_eq!(quest_giver_status::NONE, 0x0000_0000);
        assert_eq!(quest_giver_status::FUTURE, 0x0000_0002);
        assert_eq!(quest_giver_status::TRIVIAL, 0x0000_0004);
        assert_eq!(quest_giver_status::TRIVIAL_REPEATABLE_TURNIN, 0x0000_0008);
        assert_eq!(quest_giver_status::TRIVIAL_DAILY_QUEST, 0x0000_0010);
        assert_eq!(quest_giver_status::REWARD, 0x0000_0020);
        assert_eq!(quest_giver_status::JOURNEY_REWARD, 0x0000_0040);
        assert_eq!(quest_giver_status::COVENANT_CALLING_REWARD, 0x0000_0080);
        assert_eq!(quest_giver_status::REPEATABLE_TURNIN, 0x0000_0100);
        assert_eq!(quest_giver_status::DAILY_QUEST, 0x0000_0200);
        assert_eq!(quest_giver_status::QUEST, 0x0000_0400);
        assert_eq!(quest_giver_status::REWARD_COMPLETE_NO_POI, 0x0000_0800);
        assert_eq!(quest_giver_status::REWARD_COMPLETE_POI, 0x0000_1000);
        assert_eq!(quest_giver_status::LEGENDARY_QUEST, 0x0000_2000);
        assert_eq!(
            quest_giver_status::LEGENDARY_REWARD_COMPLETE_NO_POI,
            0x0000_4000
        );
        assert_eq!(
            quest_giver_status::LEGENDARY_REWARD_COMPLETE_POI,
            0x0000_8000
        );
        assert_eq!(quest_giver_status::JOURNEY_QUEST, 0x0001_0000);
        assert_eq!(
            quest_giver_status::JOURNEY_REWARD_COMPLETE_NO_POI,
            0x0002_0000
        );
        assert_eq!(quest_giver_status::JOURNEY_REWARD_COMPLETE_POI, 0x0004_0000);
        assert_eq!(quest_giver_status::COVENANT_CALLING_QUEST, 0x0008_0000);
        assert_eq!(
            quest_giver_status::COVENANT_CALLING_REWARD_COMPLETE_NO_POI,
            0x0010_0000
        );
        assert_eq!(
            quest_giver_status::COVENANT_CALLING_REWARD_COMPLETE_POI,
            0x0020_0000
        );
        assert_eq!(quest_giver_status::TRIVIAL_LEGENDARY_QUEST, 0x0040_0000);
        assert_eq!(quest_giver_status::FUTURE_LEGENDARY_QUEST, 0x0080_0000);
        assert_eq!(quest_giver_status::LEGENDARY_REWARD, 0x0100_0000);
        assert_eq!(quest_giver_status::IMPORTANT_QUEST, 0x0200_0000);
        assert_eq!(quest_giver_status::IMPORTANT_REWARD, 0x0400_0000);
        assert_eq!(quest_giver_status::TRIVIAL_IMPORTANT_QUEST, 0x0800_0000);
        assert_eq!(quest_giver_status::FUTURE_IMPORTANT_QUEST, 0x1000_0000);
        assert_eq!(
            quest_giver_status::IMPORTANT_QUEST_REWARD_COMPLETE_NO_POI,
            0x2000_0000
        );
        assert_eq!(
            quest_giver_status::IMPORTANT_QUEST_REWARD_COMPLETE_POI,
            0x4000_0000
        );
        assert_eq!(quest_giver_status::TRIVIAL_JOURNEY_QUEST, 0x8000_0000);
        assert_eq!(quest_giver_status::FUTURE_JOURNEY_QUEST, 0x1_0000_0000);
    }

    #[test]
    fn world_quest_update_response_empty_writes_zero_count_like_cpp() {
        let bytes = WorldQuestUpdateResponse {
            updates: Vec::new(),
        }
        .to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::WorldQuestUpdateResponse as u16
        );
        assert_eq!(&bytes[2..], &[0, 0, 0, 0]);
    }

    #[test]
    fn quest_giver_quest_failed_writes_quest_id_then_reason_like_cpp() {
        let bytes = QuestGiverQuestFailed {
            quest_id: 7008,
            reason: 50,
        }
        .to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::QuestGiverQuestFailed as u16
        );
        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_uint32().unwrap(), 7008);
        assert_eq!(pkt.read_uint32().unwrap(), 50);
        assert!(pkt.is_empty());
    }

    #[test]
    fn quest_update_add_pvp_credit_writes_cpp_shape() {
        let bytes = QuestUpdateAddPvpCredit {
            quest_id: 12_345,
            count: 7,
        }
        .to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::QuestUpdateAddPvpCredit as u16
        );
        assert_eq!(&bytes[2..6], &12_345_i32.to_le_bytes());
        assert_eq!(&bytes[6..8], &7_u16.to_le_bytes());
    }

    #[test]
    fn quest_update_add_credit_simple_writes_cpp_shape() {
        let bytes = QuestUpdateAddCreditSimple {
            quest_id: 12_345,
            object_id: -7,
            objective_type: 14,
        }
        .to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::QuestUpdateAddCreditSimple as u16
        );
        assert_eq!(&bytes[2..6], &12_345_i32.to_le_bytes());
        assert_eq!(&bytes[6..10], &(-7_i32).to_le_bytes());
        assert_eq!(bytes[10], 14);
    }
}

#[cfg(test)]
mod adventure_map_start_quest_tests {
    use super::*;

    #[test]
    fn adventure_map_start_quest_reads_signed_quest_id_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_int32(-123);
        pkt.reset_read();

        let parsed =
            AdventureMapStartQuest::read(&mut pkt).expect("valid AdventureMapStartQuest packet");

        assert_eq!(parsed.quest_id, -123);
        assert_eq!(
            AdventureMapStartQuest::OPCODE,
            ClientOpcodes::AdventureMapStartQuest
        );
    }

    #[test]
    fn adventure_map_start_quest_short_packet_fails_closed() {
        let mut pkt = WorldPacket::from_bytes(&[0x01, 0x02, 0x03]);

        assert!(AdventureMapStartQuest::read(&mut pkt).is_err());
    }
}

#[cfg(test)]
mod quest_confirm_accept_tests {
    use super::*;

    #[test]
    fn quest_confirm_accept_reads_signed_quest_id_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_int32(7001);

        let parsed = QuestConfirmAccept::read(&mut pkt).expect("valid QuestConfirmAccept packet");

        assert_eq!(parsed.quest_id, 7001);
    }

    #[test]
    fn quest_confirm_accept_short_packet_fails_closed() {
        let mut pkt = WorldPacket::from_bytes(&[0x59, 0x1B, 0x00]);

        assert!(QuestConfirmAccept::read(&mut pkt).is_err());
    }
}

#[cfg(test)]
mod push_quest_to_party_tests {
    use super::*;

    #[test]
    fn push_quest_to_party_reads_uint32_quest_id_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(0xA1B2_C3D4);

        let parsed = PushQuestToParty::read(&mut pkt).expect("valid PushQuestToParty packet");

        assert_eq!(parsed.quest_id, 0xA1B2_C3D4);
        assert_eq!(PushQuestToParty::OPCODE, ClientOpcodes::PushQuestToParty);
    }

    #[test]
    fn push_quest_to_party_short_packet_fails_closed() {
        let mut pkt = WorldPacket::from_bytes(&[0x9F, 0x34, 0x00]);

        assert!(PushQuestToParty::read(&mut pkt).is_err());
    }
}

#[cfg(test)]
mod quest_push_result_tests {
    use super::*;

    #[test]
    fn quest_push_result_response_writes_empty_title_like_cpp() {
        let response = QuestPushResultResponse {
            sender_guid: ObjectGuid::EMPTY,
            result: quest_push_reason::NOT_ALLOWED,
            quest_title: String::new(),
        };

        let bytes = response.to_bytes();

        assert_eq!(
            &bytes,
            &[
                0x90, 0x2A, // SMSG_QUEST_PUSH_RESULT
                0x00, 0x00, // empty packed ObjectGuid masks
                19,   // NotAllowed
                0x00, 0x00, // 9-bit empty title length + flush padding
            ]
        );
        assert_eq!(
            QuestPushResultResponse::OPCODE,
            ServerOpcodes::QuestPushResult
        );
    }

    #[test]
    fn quest_push_result_response_writes_title_length_bits_and_string_like_cpp() {
        let response = QuestPushResultResponse {
            sender_guid: ObjectGuid::EMPTY,
            result: quest_push_reason::NOT_DAILY,
            quest_title: String::from("Hi"),
        };

        let bytes = response.to_bytes();

        assert_eq!(
            &bytes,
            &[
                0x90, 0x2A, // SMSG_QUEST_PUSH_RESULT
                0x00, 0x00, // empty packed ObjectGuid masks
                14,   // NotDaily
                0x01, 0x00, // 9-bit length 2: 00000001 0xxxxxxx after flush
                b'H', b'i',
            ]
        );
    }

    #[test]
    fn quest_push_result_reads_sender_quest_id_result_in_cpp_order() {
        let sender_guid = ObjectGuid::create_player(1, 0x1234);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&sender_guid);
        pkt.write_uint32(0xA1B2_C3D4);
        pkt.write_uint8(0x2A);

        let parsed = QuestPushResult::read(&mut pkt).expect("valid QuestPushResult packet");

        assert_eq!(parsed.sender_guid, sender_guid);
        assert_eq!(parsed.quest_id, 0xA1B2_C3D4);
        assert_eq!(parsed.result, 0x2A);
    }

    #[test]
    fn quest_push_result_short_packet_fails_closed() {
        let sender_guid = ObjectGuid::create_player(1, 0x1234);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&sender_guid);
        pkt.write_uint32(0xA1B2_C3D4);
        let mut short = WorldPacket::from_bytes(&pkt.into_data());

        assert!(QuestPushResult::read(&mut short).is_err());
    }
}

#[cfg(test)]
mod quest_giver_quest_complete_tests {
    use super::*;

    #[test]
    fn quest_giver_quest_complete_writes_skill_line_then_skill_ups_like_cpp() {
        let complete = QuestGiverQuestComplete {
            quest_id: 0x0102_0304,
            xp: 0x0506_0708,
            money: 0x0102_0304,
            skill_line_id: 0x0A0B_0C0D,
            skill_points: 0x0E0F_1011,
            use_quest_reward_currency: true,
        };

        let bytes = complete.to_bytes();

        assert_eq!(
            QuestGiverQuestComplete::OPCODE,
            ServerOpcodes::QuestGiverQuestComplete
        );
        assert_eq!(&bytes[2..6], &0x0102_0304u32.to_le_bytes());
        assert_eq!(&bytes[6..10], &0x0506_0708u32.to_le_bytes());
        assert_eq!(&bytes[10..18], &0x0102_0304u64.to_le_bytes());
        assert_eq!(&bytes[18..22], &0x0A0B_0C0Du32.to_le_bytes());
        assert_eq!(&bytes[22..26], &0x0E0F_1011u32.to_le_bytes());
    }
}
