// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest system data structures and in-memory store.
//!
//! Loads `quest_template`, `quest_objectives`, creature quest relations,
//! and GameObject quest relations from the world database at startup.

use std::collections::{HashMap, HashSet};
use tracing::{info, warn};
pub use wow_constants::quest::QUEST_FLAGS_COMPLETION_AREA_TRIGGER_LIKE_CPP;

pub use wow_data_model::quest::{
    QuestDayCooldownBlock, QuestEligibilityRules, QuestObjective, QuestObjectiveRulesLikeCpp,
    QuestRewardRules, QuestStatusBlock,
};

// ── Constants (matching C# SharedConst) ──────────────────────────────────────
pub use wow_constants::quest::{
    QUEST_FLAGS_DAILY as QUEST_FLAGS_DAILY_LIKE_CPP,
    QUEST_FLAGS_EX_LEGENDARY as QUEST_FLAGS_EX_LEGENDARY_LIKE_CPP,
    QUEST_FLAGS_HIDE_REWARD_POI as QUEST_FLAGS_HIDE_REWARD_POI_LIKE_CPP,
    QUEST_FLAGS_WEEKLY as QUEST_FLAGS_WEEKLY_LIKE_CPP, QUEST_ITEM_DROP_COUNT,
    QUEST_OBJECTIVE_AREATRIGGER as QUEST_OBJECTIVE_AREATRIGGER_LIKE_CPP,
    QUEST_REWARD_CHOICES_COUNT, QUEST_REWARD_CURRENCY_COUNT, QUEST_REWARD_DISPLAY_SPELL_COUNT,
    QUEST_REWARD_ITEM_COUNT, QUEST_REWARD_REPUTATIONS_COUNT,
    QUEST_SPECIAL_FLAGS_DF_QUEST as QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP,
    QUEST_SPECIAL_FLAGS_MONTHLY as QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP,
    QUEST_SPECIAL_FLAGS_REPEATABLE as QUEST_SPECIAL_FLAGS_REPEATABLE_LIKE_CPP,
};
use wow_constants::quest::{
    QUEST_SORT_BREWFEST as QUEST_SORT_BREWFEST_LIKE_CPP,
    QUEST_SORT_BREWFEST_NEGATIVE as QUEST_SORT_BREWFEST_NEGATIVE_LIKE_CPP,
    QUEST_SORT_LOVE_IS_IN_THE_AIR as QUEST_SORT_LOVE_IS_IN_THE_AIR_LIKE_CPP,
    QUEST_SORT_LOVE_IS_IN_THE_AIR_NEGATIVE as QUEST_SORT_LOVE_IS_IN_THE_AIR_NEGATIVE_LIKE_CPP,
    QUEST_SORT_LUNAR_FESTIVAL as QUEST_SORT_LUNAR_FESTIVAL_LIKE_CPP,
    QUEST_SORT_LUNAR_FESTIVAL_NEGATIVE as QUEST_SORT_LUNAR_FESTIVAL_NEGATIVE_LIKE_CPP,
    QUEST_SORT_MIDSUMMER as QUEST_SORT_MIDSUMMER_LIKE_CPP,
    QUEST_SORT_MIDSUMMER_NEGATIVE as QUEST_SORT_MIDSUMMER_NEGATIVE_LIKE_CPP,
    QUEST_SORT_NOBLEGARDEN as QUEST_SORT_NOBLEGARDEN_LIKE_CPP,
    QUEST_SORT_NOBLEGARDEN_NEGATIVE as QUEST_SORT_NOBLEGARDEN_NEGATIVE_LIKE_CPP,
    QUEST_SORT_SEASONAL as QUEST_SORT_SEASONAL_LIKE_CPP,
    QUEST_SORT_SEASONAL_NEGATIVE as QUEST_SORT_SEASONAL_NEGATIVE_LIKE_CPP,
    QUEST_SORT_SPECIAL as QUEST_SORT_SPECIAL_LIKE_CPP,
    QUEST_SORT_SPECIAL_NEGATIVE as QUEST_SORT_SPECIAL_NEGATIVE_LIKE_CPP,
    QUEST_SPECIAL_FLAGS_AUTO_ACCEPT as QUEST_SPECIAL_FLAGS_AUTO_ACCEPT_LIKE_CPP,
    QUEST_SPECIAL_FLAGS_AUTO_PUSH_TO_PARTY as QUEST_SPECIAL_FLAGS_AUTO_PUSH_TO_PARTY_LIKE_CPP,
    QUEST_SPECIAL_FLAGS_DB_ALLOWED as QUEST_SPECIAL_FLAGS_DB_ALLOWED_LIKE_CPP,
    QUEST_TYPE_TURNIN as QUEST_TYPE_TURNIN_LIKE_CPP,
};

pub use wow_data_model::quest::QuestTemplate;

/// C++ `ObjectMgr::LoadQuests` post-load normalization before `Quest` helpers are observable.
fn nonzero_abs_i32_to_u32_like_cpp(value: i32) -> Option<u32> {
    let abs = value.unsigned_abs();
    (abs != 0).then_some(abs)
}

fn push_unique_sorted_like_cpp(values: &mut Vec<u32>, value: u32) {
    if !values.contains(&value) {
        values.push(value);
        values.sort_unstable();
    }
}

fn normalize_quest_flags_like_cpp(flags: u32, special_flags: u32) -> (u32, u32) {
    let mut flags = flags;
    let mut special_flags = special_flags & QUEST_SPECIAL_FLAGS_DB_ALLOWED_LIKE_CPP;

    if flags & QUEST_FLAGS_DAILY_LIKE_CPP != 0 && flags & QUEST_FLAGS_WEEKLY_LIKE_CPP != 0 {
        flags &= !QUEST_FLAGS_DAILY_LIKE_CPP;
    }

    if flags & (QUEST_FLAGS_DAILY_LIKE_CPP | QUEST_FLAGS_WEEKLY_LIKE_CPP) != 0
        || special_flags & QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP != 0
    {
        special_flags |= QUEST_SPECIAL_FLAGS_REPEATABLE_LIKE_CPP;
    }

    (flags, special_flags)
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
    /// GameObject template entry → list of quest IDs this GameObject starts.
    pub gameobject_starter_quests: HashMap<u32, Vec<u32>>,
    /// GameObject template entry → list of quest IDs this GameObject ends.
    pub gameobject_ender_quests: HashMap<u32, Vec<u32>>,
}

impl QuestStore {
    pub fn from_template_rows_like_cpp(templates: Vec<QuestTemplate>) -> Self {
        let mut store = Self::new();
        for mut quest in templates {
            (quest.flags, quest.special_flags) =
                normalize_quest_flags_like_cpp(quest.flags, quest.special_flags);
            store.quests.insert(quest.id, quest);
        }
        store.normalize_dependent_quest_metadata_like_cpp();
        info!("Loaded {} quest templates", store.quests.len());
        store
    }

    pub fn apply_special_flag_rows_like_cpp(&mut self, rows: Vec<(u32, u32)>) {
        let mut special_count = 0;
        for (id, special_flags) in rows {
            if let Some(quest) = self.quests.get_mut(&id) {
                (quest.flags, quest.special_flags) =
                    normalize_quest_flags_like_cpp(quest.flags, special_flags);
                special_count += 1;
            }
        }
        info!("Applied {special_count} quest_template_addon SpecialFlags rows like C++");
    }

    pub fn apply_seasonal_relation_rows_like_cpp(&mut self, rows: Vec<(u32, u32)>) {
        let mut seasonal_count = 0;
        for (quest_id, event_entry) in rows {
            if let Some(quest) = self.quests.get_mut(&quest_id) {
                if let Ok(event_id) = u16::try_from(event_entry) {
                    quest.event_id_for_quest = event_id;
                    seasonal_count += 1;
                } else {
                    warn!(
                        quest_id,
                        event_entry,
                        "Skipping seasonal quest relation with event id outside u16 range"
                    );
                }
            } else {
                warn!(
                    quest_id,
                    event_entry, "Skipping seasonal quest relation for missing quest template"
                );
            }
        }
        info!(
            "Loaded {seasonal_count} seasonal quest event relations (GameEvent max range guard remains with GameEvent metadata owner)"
        );
    }

    pub fn apply_objective_rows_like_cpp(&mut self, rows: Vec<QuestObjective>) {
        let mut objective_count = 0;
        for objective in rows {
            if let Some(quest) = self.quests.get_mut(&objective.quest_id) {
                quest.objectives.push(objective);
                objective_count += 1;
            }
        }
        info!("Loaded {objective_count} quest objectives");
    }

    pub fn apply_creature_starter_rows_like_cpp(&mut self, rows: Vec<(u32, u32)>) {
        for (npc, quest) in rows {
            if self.quests.contains_key(&quest) {
                self.starter_quests.entry(npc).or_default().push(quest);
            }
        }
    }

    pub fn apply_creature_ender_rows_like_cpp(&mut self, rows: Vec<(u32, u32)>) {
        for (npc, quest) in rows {
            if self.quests.contains_key(&quest) {
                self.ender_quests.entry(npc).or_default().push(quest);
            }
        }
    }

    pub fn apply_gameobject_starter_rows_like_cpp(&mut self, rows: Vec<(u32, u32)>) {
        for (entry, quest) in rows {
            self.insert_gameobject_starter_relation_like_cpp(entry, quest);
        }
    }

    pub fn apply_gameobject_ender_rows_like_cpp(&mut self, rows: Vec<(u32, u32)>) {
        for (entry, quest) in rows {
            self.insert_gameobject_ender_relation_like_cpp(entry, quest);
        }
    }

    pub fn log_relation_counts_like_cpp(&self) {
        info!(
            "Quest relations: NPC {} starters / {} enders, GameObject {} starters / {} enders",
            self.starter_quests.len(),
            self.ender_quests.len(),
            self.gameobject_starter_quests.len(),
            self.gameobject_ender_quests.len()
        );
    }

    pub fn new() -> Self {
        Self {
            quests: HashMap::new(),
            starter_quests: HashMap::new(),
            ender_quests: HashMap::new(),
            gameobject_starter_quests: HashMap::new(),
            gameobject_ender_quests: HashMap::new(),
        }
    }

    pub fn from_quests_like_cpp(quests: impl IntoIterator<Item = QuestTemplate>) -> Self {
        let mut store = Self {
            quests: quests.into_iter().map(|quest| (quest.id, quest)).collect(),
            starter_quests: HashMap::new(),
            ender_quests: HashMap::new(),
            gameobject_starter_quests: HashMap::new(),
            gameobject_ender_quests: HashMap::new(),
        };
        store.normalize_dependent_quest_metadata_like_cpp();
        store
    }

    /// C++ `ObjectMgr::LoadQuests` represented metadata normalization for quest dependencies.
    ///
    /// Ownership: `QuestStore` owns static DB quest metadata and these post-load derived vectors.
    /// Runtime handlers/sessions may read the normalized vectors in later slices, but must not
    /// write back into this store.
    pub fn normalize_dependent_quest_metadata_like_cpp(&mut self) {
        for quest in self.quests.values_mut() {
            quest.dependent_previous_quests.clear();
            quest.dependent_breadcrumb_quests.clear();
        }

        let mut quest_ids: Vec<u32> = self.quests.keys().copied().collect();
        quest_ids.sort_unstable();

        for quest_id in &quest_ids {
            let Some(quest) = self.quests.get(quest_id) else {
                continue;
            };

            let prev_quest_id = quest.prev_quest_id;
            let next_quest_id = quest.next_quest_id;
            let breadcrumb_for_quest_id = quest.breadcrumb_for_quest_id;

            if let Some(prev_id) = nonzero_abs_i32_to_u32_like_cpp(prev_quest_id) {
                if self
                    .quests
                    .get(&prev_id)
                    .is_some_and(|previous| previous.breadcrumb_for_quest_id == 0)
                    && prev_quest_id > 0
                {
                    if let Some(quest) = self.quests.get_mut(quest_id) {
                        push_unique_sorted_like_cpp(&mut quest.dependent_previous_quests, prev_id);
                    }
                }
            }

            if next_quest_id != 0 && self.quests.contains_key(&next_quest_id) {
                if let Some(next_quest) = self.quests.get_mut(&next_quest_id) {
                    push_unique_sorted_like_cpp(
                        &mut next_quest.dependent_previous_quests,
                        *quest_id,
                    );
                }
            }

            if let Some(breadcrumb_target_id) =
                nonzero_abs_i32_to_u32_like_cpp(breadcrumb_for_quest_id)
            {
                if !self.quests.contains_key(&breadcrumb_target_id) {
                    if let Some(quest) = self.quests.get_mut(quest_id) {
                        quest.breadcrumb_for_quest_id = 0;
                    }
                }
            }
        }

        for source_quest_id in quest_ids {
            let mut current_quest_id = source_quest_id;
            let mut breadcrumb_for_quest_id = self
                .quests
                .get(&current_quest_id)
                .and_then(|quest| nonzero_abs_i32_to_u32_like_cpp(quest.breadcrumb_for_quest_id));
            let mut seen = HashSet::new();

            while let Some(target_quest_id) = breadcrumb_for_quest_id {
                if !seen.insert(current_quest_id) {
                    if let Some(quest) = self.quests.get_mut(&current_quest_id) {
                        quest.breadcrumb_for_quest_id = 0;
                    }
                    break;
                }

                if !self.quests.contains_key(&target_quest_id) {
                    break;
                }

                if let Some(target_quest) = self.quests.get_mut(&target_quest_id) {
                    push_unique_sorted_like_cpp(
                        &mut target_quest.dependent_breadcrumb_quests,
                        source_quest_id,
                    );
                }

                current_quest_id = target_quest_id;
                breadcrumb_for_quest_id = self.quests.get(&current_quest_id).and_then(|quest| {
                    nonzero_abs_i32_to_u32_like_cpp(quest.breadcrumb_for_quest_id)
                });
            }
        }
    }

    /// Applies C++ source-item/source-spell metadata normalization to all loaded quest templates.
    ///
    /// Ownership: callers provide item/spell validity predicates from the future composition
    /// layer. This store owns only static quest metadata and never infers item/spell existence.
    pub fn normalize_source_item_spell_metadata_like_cpp(
        &mut self,
        item_exists: impl Fn(u32) -> bool,
        spell_valid: impl Fn(u32) -> bool,
    ) {
        for quest in self.quests.values_mut() {
            quest.normalize_source_item_spell_like_cpp(&item_exists, &spell_valid);
        }
    }

    pub fn get(&self, id: u32) -> Option<&QuestTemplate> {
        self.quests.get(&id)
    }

    /// Complete C++ `sObjectMgr->GetQuestTemplates()` projection. Callers
    /// that audit global login/update producers must inspect every template,
    /// not only quests related to the current Player.
    pub fn quests_like_cpp(&self) -> impl Iterator<Item = &QuestTemplate> {
        self.quests.values()
    }

    pub fn objective_like_cpp(&self, objective_id: u32) -> Option<&QuestObjective> {
        self.quests
            .values()
            .flat_map(|quest| quest.objectives.iter())
            .find(|objective| objective.id == objective_id)
    }

    pub fn objectives_like_cpp(&self) -> impl Iterator<Item = &QuestObjective> {
        self.quests
            .values()
            .flat_map(|quest| quest.objectives.iter())
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

    /// Get all quests a given GameObject can offer.
    pub fn quests_for_gameobject_starter(&self, go_entry: u32) -> Vec<&QuestTemplate> {
        self.gameobject_starter_quests
            .get(&go_entry)
            .map(|ids| ids.iter().filter_map(|id| self.quests.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all quests a given GameObject can complete/turn-in.
    pub fn quests_for_gameobject_ender(&self, go_entry: u32) -> Vec<&QuestTemplate> {
        self.gameobject_ender_quests
            .get(&go_entry)
            .map(|ids| ids.iter().filter_map(|id| self.quests.get(id)).collect())
            .unwrap_or_default()
    }

    /// Creature entries that have `creature_questender`/involved relation for `quest_id`.
    ///
    /// C++ uses reverse bounds over ObjectMgr relation multimaps. Rust's current store is
    /// entry → quest IDs, so this read-only reverse lookup sorts entries for deterministic
    /// represented output rather than depending on `HashMap` iteration order.
    pub fn creature_ender_entries_for_quest_like_cpp(&self, quest_id: u32) -> Vec<u32> {
        let mut entries: Vec<u32> = self
            .ender_quests
            .iter()
            .filter_map(|(&entry, quest_ids)| quest_ids.contains(&quest_id).then_some(entry))
            .collect();
        entries.sort_unstable();
        entries
    }

    /// GameObject entries that have `gameobject_questender`/involved relation for `quest_id`.
    ///
    /// C++ response callers must apply the `0x80000000` GameObject mask themselves.
    pub fn gameobject_ender_entries_for_quest_like_cpp(&self, quest_id: u32) -> Vec<u32> {
        let mut entries: Vec<u32> = self
            .gameobject_ender_quests
            .iter()
            .filter_map(|(&entry, quest_ids)| quest_ids.contains(&quest_id).then_some(entry))
            .collect();
        entries.sort_unstable();
        entries
    }

    /// C++ `ObjectMgr::LoadQuestRelationsHelper` insert guard for `gameobject_queststarter`.
    pub fn insert_gameobject_starter_relation_like_cpp(
        &mut self,
        go_entry: u32,
        quest_id: u32,
    ) -> bool {
        if !self.quests.contains_key(&quest_id) {
            return false;
        }

        self.gameobject_starter_quests
            .entry(go_entry)
            .or_default()
            .push(quest_id);
        true
    }

    /// C++ `ObjectMgr::LoadQuestRelationsHelper` insert guard for `gameobject_questender`.
    pub fn insert_gameobject_ender_relation_like_cpp(
        &mut self,
        go_entry: u32,
        quest_id: u32,
    ) -> bool {
        if !self.quests.contains_key(&quest_id) {
            return false;
        }

        self.gameobject_ender_quests
            .entry(go_entry)
            .or_default()
            .push(quest_id);
        true
    }

    /// Whether a given NPC starts a specific quest.
    pub fn creature_has_starter_relation_like_cpp(&self, npc_entry: u32, quest_id: u32) -> bool {
        self.starter_quests
            .get(&npc_entry)
            .is_some_and(|ids| ids.contains(&quest_id))
    }

    /// Whether a given NPC ends a specific quest.
    pub fn creature_has_ender_relation_like_cpp(&self, npc_entry: u32, quest_id: u32) -> bool {
        self.ender_quests
            .get(&npc_entry)
            .is_some_and(|ids| ids.contains(&quest_id))
    }

    /// Whether a given GameObject starts a specific quest.
    pub fn gameobject_has_starter_relation_like_cpp(&self, go_entry: u32, quest_id: u32) -> bool {
        self.gameobject_starter_quests
            .get(&go_entry)
            .is_some_and(|ids| ids.contains(&quest_id))
    }

    /// Whether a given GameObject ends a specific quest.
    pub fn gameobject_has_ender_relation_like_cpp(&self, go_entry: u32, quest_id: u32) -> bool {
        self.gameobject_ender_quests
            .get(&go_entry)
            .is_some_and(|ids| ids.contains(&quest_id))
    }

    /// Whether a given NPC starts any quest.
    pub fn npc_has_start_quests(&self, npc_entry: u32) -> bool {
        self.starter_quests
            .get(&npc_entry)
            .map_or(false, |v| !v.is_empty())
    }

    /// Whether a given NPC ends any quest.
    pub fn npc_has_end_quests(&self, npc_entry: u32) -> bool {
        self.ender_quests
            .get(&npc_entry)
            .map_or(false, |v| !v.is_empty())
    }

    /// Whether a given GameObject starts any quest.
    pub fn gameobject_has_start_quests(&self, go_entry: u32) -> bool {
        self.gameobject_starter_quests
            .get(&go_entry)
            .map_or(false, |v| !v.is_empty())
    }

    /// Whether a given GameObject ends any quest.
    pub fn gameobject_has_end_quests(&self, go_entry: u32) -> bool {
        self.gameobject_ender_quests
            .get(&go_entry)
            .map_or(false, |v| !v.is_empty())
    }
}

impl Default for QuestStore {
    fn default() -> Self {
        Self::new()
    }
}

// ── QuestPoolMgr represented metadata seam ───────────────────────────────────

/// Row from C++ `quest_pool_members` joined to `quest_pool_template`.
///
/// C++ anchor: `QuestPoolMgr::LoadFromDB`, `QuestPools.cpp:75-125`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestPoolMemberRowLikeCpp {
    pub quest_id: u32,
    pub pool_id: u32,
    pub pool_index: u32,
    pub num_active: Option<u32>,
}

/// Row from C++ `pool_quest_save`.
///
/// C++ anchor: `QuestPoolMgr::LoadFromDB`, `QuestPools.cpp:128-160`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestPoolSavedActiveRowLikeCpp {
    pub pool_id: u32,
    pub quest_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestPoolLikeCpp {
    pub pool_id: u32,
    pub num_active: u32,
    pub members: Vec<Vec<u32>>,
    pub active_quests: HashSet<u32>,
}

/// Read-only C++-shaped subset of `QuestPoolMgr` sufficient for `IsQuestActive`.
///
/// This deliberately does not implement C++ regeneration/RNG or DB persistence from
/// `QuestPools.cpp:163-250`; when saved rows are absent/incomplete the store remains a
/// represented snapshot of the metadata and saved active rows supplied by the caller.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestPoolStoreLikeCpp {
    pools: HashMap<u32, QuestPoolLikeCpp>,
    pool_lookup: HashMap<u32, u32>,
}

impl QuestPoolStoreLikeCpp {
    pub fn from_rows_like_cpp(
        quest_store: &QuestStore,
        member_rows: impl IntoIterator<Item = QuestPoolMemberRowLikeCpp>,
        saved_active_rows: impl IntoIterator<Item = QuestPoolSavedActiveRowLikeCpp>,
    ) -> Self {
        let mut pools: HashMap<u32, QuestPoolLikeCpp> = HashMap::new();
        let mut first_valid_pool_kind: HashMap<u32, QuestPoolKindLikeCpp> = HashMap::new();

        for row in member_rows {
            let Some(num_active) = row.num_active else {
                continue;
            };
            let Some(quest) = quest_store.get(row.quest_id) else {
                continue;
            };
            let Some(kind) = QuestPoolKindLikeCpp::from_quest_like_cpp(quest) else {
                continue;
            };

            first_valid_pool_kind.entry(row.pool_id).or_insert(kind);
            let pool = pools
                .entry(row.pool_id)
                .or_insert_with(|| QuestPoolLikeCpp {
                    pool_id: row.pool_id,
                    num_active,
                    members: Vec::new(),
                    active_quests: HashSet::new(),
                });

            let pool_index = row.pool_index as usize;
            if pool_index >= pool.members.len() {
                pool.members.resize_with(pool_index + 1, Vec::new);
            }
            pool.members[pool_index].push(row.quest_id);
        }

        let mut saved_active_by_pool: HashMap<u32, HashSet<u32>> = HashMap::new();
        for row in saved_active_rows {
            if pools.contains_key(&row.pool_id) {
                saved_active_by_pool
                    .entry(row.pool_id)
                    .or_default()
                    .insert(row.quest_id);
            }
        }

        for pool in pools.values_mut() {
            let Some(saved_active) = saved_active_by_pool.get(&pool.pool_id) else {
                continue;
            };

            for member in &pool.members {
                let Some(first_quest_id) = member.first() else {
                    continue;
                };

                if saved_active.contains(first_quest_id) {
                    pool.active_quests.extend(member.iter().copied());
                }
            }
        }

        let mut pool_lookup = HashMap::new();
        for (pool_id, pool) in &pools {
            if first_valid_pool_kind.contains_key(pool_id) {
                for quest_id in pool.members.iter().flatten().copied() {
                    pool_lookup.entry(quest_id).or_insert(*pool_id);
                }
            }
        }

        Self { pools, pool_lookup }
    }

    /// C++ `QuestPoolMgr::IsQuestActive`: non-pooled quests are active; pooled quests are
    /// active iff present in their pool's `activeQuests` set.
    ///
    /// C++ anchor: `QuestPools.cpp:286-292`.
    pub fn is_quest_active_like_cpp(&self, quest_id: u32) -> bool {
        let Some(pool_id) = self.pool_lookup.get(&quest_id) else {
            return true;
        };

        self.pools
            .get(pool_id)
            .is_none_or(|pool| pool.active_quests.contains(&quest_id))
    }

    pub fn is_quest_pooled_like_cpp(&self, quest_id: u32) -> bool {
        self.pool_lookup.contains_key(&quest_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuestPoolKindLikeCpp {
    Daily,
    Weekly,
    Monthly,
}

impl QuestPoolKindLikeCpp {
    fn from_quest_like_cpp(quest: &QuestTemplate) -> Option<Self> {
        if quest.is_daily_like_cpp() {
            Some(Self::Daily)
        } else if quest.is_weekly_like_cpp() {
            Some(Self::Weekly)
        } else if quest.is_monthly_like_cpp() {
            Some(Self::Monthly)
        } else {
            None
        }
    }
}

#[cfg(test)]
#[path = "quest/tests/mod.rs"]
mod tests;
