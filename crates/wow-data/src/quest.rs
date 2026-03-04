// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest system data structures and in-memory store.
//!
//! Loads `quest_template`, `quest_objectives`, `creature_queststarter`
//! and `creature_questender` from the world database at startup.

use std::collections::HashMap;
use wow_database::{WorldDatabase, WorldStatements};
use anyhow::Result;
use tracing::info;

// ── Constants (matching C# SharedConst) ──────────────────────────────────────
pub const QUEST_REWARD_ITEM_COUNT: usize = 4;
pub const QUEST_REWARD_CHOICES_COUNT: usize = 6;
pub const QUEST_REWARD_REPUTATIONS_COUNT: usize = 5;
pub const QUEST_REWARD_CURRENCY_COUNT: usize = 4;
pub const QUEST_REWARD_DISPLAY_SPELL_COUNT: usize = 3;

// ── QuestObjective ────────────────────────────────────────────────────────────

/// A single objective for a quest (kill X, loot Y, explore Z, etc.)
/// C# ref: QuestObjective struct / quest_objectives table
#[derive(Debug, Clone)]
pub struct QuestObjective {
    pub id: u32,
    pub quest_id: u32,
    /// 0=Monster, 1=Item, 2=GameObject, 3=TalkTo, 4=Currency,
    /// 5=LearnSpell, 6=MinReputation, 7=MaxReputation, 8=Money,
    /// 9=PlayerKills, 10=AreaTrigger, ...
    pub obj_type: u8,
    pub order: u8,
    pub storage_index: i8,
    pub object_id: i32,
    pub amount: i32,
    pub flags: u32,
    pub flags2: u32,
    pub progress_bar_weight: f32,
    pub description: String,
}

// ── QuestTemplate ─────────────────────────────────────────────────────────────

/// Full quest data loaded from the world database.
/// C# ref: Quest class / quest_template table
#[derive(Debug, Clone)]
pub struct QuestTemplate {
    pub id: u32,
    pub quest_type: u8,
    pub quest_level: i32,
    pub quest_max_scaling_level: i32,
    pub min_level: i32,
    pub quest_sort_id: i32,
    pub quest_info_id: u16,
    pub suggested_group_num: u8,
    pub reward_next_quest: u32,
    pub reward_xp_difficulty: u32,
    pub reward_xp_multiplier: f32,
    pub reward_money_difficulty: u32,
    pub reward_money_multiplier: f32,
    pub reward_bonus_money: u32,
    pub reward_display_spell: [u32; QUEST_REWARD_DISPLAY_SPELL_COUNT],
    pub reward_spell: u32,
    pub reward_honor: u32,
    pub flags: u32,
    pub flags_ex: u32,
    pub flags_ex2: u32,
    pub reward_items: [u32; QUEST_REWARD_ITEM_COUNT],
    pub reward_amounts: [u32; QUEST_REWARD_ITEM_COUNT],
    // Strings
    pub log_title: String,
    pub log_description: String,
    pub quest_description: String,
    pub area_description: String,
    pub quest_completion_log: String,
    // Objectives
    pub objectives: Vec<QuestObjective>,

    // ── Eligibility filters ──────────────────────────────────────────────────
    /// Bitmask of allowed races: bit (race-1) set = allowed.
    /// 0 = all races allowed (RaceMask::Playable default).
    pub allowable_races: u64,
    /// Bitmask of allowed classes: bit (class-1) set = allowed.
    /// 0 = all classes allowed.
    pub allowable_classes: u32,
    /// Maximum player level to take this quest. 0 = no limit.
    pub max_level: u8,
    /// Previous quest that must be completed first. 0 = none.
    pub prev_quest_id: i32,
}

impl QuestTemplate {
    /// Returns true if this is a repeatable (daily/weekly) quest.
    pub fn is_repeatable(&self) -> bool {
        // Flags & QUEST_FLAGS_REPEATABLE (0x1) or daily (0x4000)
        self.flags & 0x1 != 0 || self.flags & 0x4000 != 0
    }

    /// Returns true if the given player (race, class, level) can take this quest.
    /// C# ref: SatisfyQuestRace + SatisfyQuestClass + SatisfyQuestLevel
    pub fn is_available_for(&self, race: u8, class: u8, level: u8) -> bool {
        // Race check: 0 means all races allowed
        if self.allowable_races != 0 {
            let race_bit = 1u64 << (race.saturating_sub(1) as u64);
            if self.allowable_races & race_bit == 0 {
                return false;
            }
        }

        // Class check: 0 means all classes allowed
        if self.allowable_classes != 0 {
            let class_bit = 1u32 << (class.saturating_sub(1) as u32);
            if self.allowable_classes & class_bit == 0 {
                return false;
            }
        }

        // Min level check
        if self.min_level > 0 && (level as i32) < self.min_level {
            return false;
        }

        // Max level check
        if self.max_level > 0 && level > self.max_level {
            return false;
        }

        true
    }
}

// ── QuestStore ────────────────────────────────────────────────────────────────

/// In-memory store of all quest templates and NPC relations.
pub struct QuestStore {
    /// Quest templates by ID.
    pub quests: HashMap<u32, QuestTemplate>,
    /// NPC entry → list of quest IDs this NPC starts.
    pub starter_quests: HashMap<u32, Vec<u32>>,
    /// NPC entry → list of quest IDs this NPC ends.
    pub ender_quests: HashMap<u32, Vec<u32>>,
}

impl QuestStore {
    pub fn new() -> Self {
        Self {
            quests: HashMap::new(),
            starter_quests: HashMap::new(),
            ender_quests: HashMap::new(),
        }
    }

    pub fn get(&self, id: u32) -> Option<&QuestTemplate> {
        self.quests.get(&id)
    }

    /// Get all quests a given NPC can offer.
    pub fn quests_for_starter(&self, npc_entry: u32) -> Vec<&QuestTemplate> {
        self.starter_quests
            .get(&npc_entry)
            .map(|ids| ids.iter().filter_map(|id| self.quests.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all quests a given NPC can complete/turn-in.
    pub fn quests_for_ender(&self, npc_entry: u32) -> Vec<&QuestTemplate> {
        self.ender_quests
            .get(&npc_entry)
            .map(|ids| ids.iter().filter_map(|id| self.quests.get(id)).collect())
            .unwrap_or_default()
    }

    /// Whether a given NPC starts any quest.
    pub fn npc_has_start_quests(&self, npc_entry: u32) -> bool {
        self.starter_quests.get(&npc_entry).map_or(false, |v| !v.is_empty())
    }

    /// Whether a given NPC ends any quest.
    pub fn npc_has_end_quests(&self, npc_entry: u32) -> bool {
        self.ender_quests.get(&npc_entry).map_or(false, |v| !v.is_empty())
    }
}

impl Default for QuestStore {
    fn default() -> Self { Self::new() }
}

// ── DB loading ────────────────────────────────────────────────────────────────

/// Load all quest data from the world database into a QuestStore.
pub async fn load_quests(db: &WorldDatabase) -> Result<QuestStore> {
    let mut store = QuestStore::new();

    // ── Load quest templates ──────────────────────────────────────────────
    let stmt = db.prepare(WorldStatements::SEL_QUEST_TEMPLATE);
    let result = db.query(&stmt).await?;

    if !result.is_empty() {
        let mut result = result;
        loop {
            let id: u32 = result.read(0);
            let quest = QuestTemplate {
                id,
                quest_type:               result.try_read::<u8>(1).unwrap_or(2),
                quest_level:              result.try_read::<i32>(2).unwrap_or(0),
                quest_max_scaling_level:  result.try_read::<i32>(3).unwrap_or(0),
                min_level:                result.try_read::<i32>(4).unwrap_or(0),
                quest_sort_id:            result.try_read::<i32>(5).unwrap_or(0),
                quest_info_id:            result.try_read::<u16>(6).unwrap_or(0),
                suggested_group_num:      result.try_read::<u8>(7).unwrap_or(0),
                reward_next_quest:        result.try_read::<u32>(8).unwrap_or(0),
                reward_xp_difficulty:     result.try_read::<u32>(9).unwrap_or(0),
                reward_xp_multiplier:     result.try_read::<f32>(10).unwrap_or(1.0),
                reward_money_difficulty:  result.try_read::<u32>(11).unwrap_or(0),
                reward_money_multiplier:  result.try_read::<f32>(12).unwrap_or(1.0),
                reward_bonus_money:       result.try_read::<u32>(13).unwrap_or(0),
                reward_display_spell: [
                    result.try_read::<u32>(14).unwrap_or(0),
                    result.try_read::<u32>(15).unwrap_or(0),
                    result.try_read::<u32>(16).unwrap_or(0),
                ],
                reward_spell:             result.try_read::<u32>(17).unwrap_or(0),
                reward_honor:             result.try_read::<u32>(18).unwrap_or(0),
                flags:                    result.try_read::<u32>(19).unwrap_or(0),
                flags_ex:                 result.try_read::<u32>(20).unwrap_or(0),
                flags_ex2:                result.try_read::<u32>(21).unwrap_or(0),
                reward_items: [
                    result.try_read::<u32>(22).unwrap_or(0),
                    result.try_read::<u32>(24).unwrap_or(0),
                    result.try_read::<u32>(26).unwrap_or(0),
                    result.try_read::<u32>(28).unwrap_or(0),
                ],
                reward_amounts: [
                    result.try_read::<u32>(23).unwrap_or(0),
                    result.try_read::<u32>(25).unwrap_or(0),
                    result.try_read::<u32>(27).unwrap_or(0),
                    result.try_read::<u32>(29).unwrap_or(0),
                ],
                log_title:           result.try_read::<String>(30).unwrap_or_default(),
                log_description:     result.try_read::<String>(31).unwrap_or_default(),
                quest_description:   result.try_read::<String>(32).unwrap_or_default(),
                area_description:    result.try_read::<String>(33).unwrap_or_default(),
                quest_completion_log:result.try_read::<String>(34).unwrap_or_default(),
                allowable_races:     result.try_read::<i64>(35).map(|v| v as u64).unwrap_or(0),
                allowable_classes:   result.try_read::<u32>(36).unwrap_or(0),
                max_level:           result.try_read::<u8>(37).unwrap_or(0),
                prev_quest_id:       result.try_read::<i32>(38).unwrap_or(0),
                objectives: Vec::new(), // filled next
            };
            store.quests.insert(id, quest);
            if !result.next_row() { break; }
        }
    }
    info!("Loaded {} quest templates", store.quests.len());

    // ── Load quest objectives ─────────────────────────────────────────────
    let stmt = db.prepare(WorldStatements::SEL_QUEST_OBJECTIVES);
    let result = db.query(&stmt).await?;
    if !result.is_empty() {
        let mut result = result;
        let mut count = 0u32;
        loop {
            let obj = QuestObjective {
                id:                   result.try_read::<u32>(0).unwrap_or(0),
                quest_id:             result.try_read::<u32>(1).unwrap_or(0),
                obj_type:             result.try_read::<u8>(2).unwrap_or(0),
                order:                result.try_read::<u8>(3).unwrap_or(0),
                storage_index:        result.try_read::<i8>(4).unwrap_or(0),
                object_id:            result.try_read::<i32>(5).unwrap_or(0),
                amount:               result.try_read::<i32>(6).unwrap_or(0),
                flags:                result.try_read::<u32>(7).unwrap_or(0),
                flags2:               result.try_read::<u32>(8).unwrap_or(0),
                progress_bar_weight:  result.try_read::<f32>(9).unwrap_or(0.0),
                description:          result.try_read::<String>(10).unwrap_or_default(),
            };
            if let Some(quest) = store.quests.get_mut(&obj.quest_id) {
                quest.objectives.push(obj);
                count += 1;
            }
            if !result.next_row() { break; }
        }
        info!("Loaded {} quest objectives", count);
    }

    // ── Load creature quest starters ──────────────────────────────────────
    let stmt = db.prepare(WorldStatements::SEL_QUEST_STARTERS);
    let result = db.query(&stmt).await?;
    if !result.is_empty() {
        let mut result = result;
        loop {
            let npc: u32  = result.try_read::<u32>(0).unwrap_or(0);
            let quest: u32 = result.try_read::<u32>(1).unwrap_or(0);
            if store.quests.contains_key(&quest) {
                store.starter_quests.entry(npc).or_default().push(quest);
            }
            if !result.next_row() { break; }
        }
    }

    // ── Load creature quest enders ────────────────────────────────────────
    let stmt = db.prepare(WorldStatements::SEL_QUEST_ENDERS);
    let result = db.query(&stmt).await?;
    if !result.is_empty() {
        let mut result = result;
        loop {
            let npc: u32   = result.try_read::<u32>(0).unwrap_or(0);
            let quest: u32 = result.try_read::<u32>(1).unwrap_or(0);
            if store.quests.contains_key(&quest) {
                store.ender_quests.entry(npc).or_default().push(quest);
            }
            if !result.next_row() { break; }
        }
    }

    info!(
        "Quest NPC relations: {} starters, {} enders",
        store.starter_quests.len(),
        store.ender_quests.len()
    );

    Ok(store)
}
