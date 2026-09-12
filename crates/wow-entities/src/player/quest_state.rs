// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player quest state.
//!
//! C++ `Player` owns `QuestStatusMap m_QuestStatus` (`Player.h:2947`),
//! `RewardedQuestSet m_RewardedQuests` (`:2951`), `m_DFQuests` (`:2481`) and
//! `m_seasonalquests` (`:2838`), and performs the transitions itself:
//! `SetQuestStatus` (`Player.cpp:15557`), `RemoveActiveQuest` (`:15575`),
//! `RemoveRewardedQuest` (`:15595`), `AdjustQuestObjectiveProgress` (`:15874`),
//! `SetQuestObjectiveData` (`:16426`) and the daily/weekly/seasonal/monthly
//! setters (`:24037`, `:24061`, `:24067`, `:24077`), with `_LoadQuestStatus`
//! (`:18613`) and its objective/rewarded siblings at the persistence edges.
//!
//! Separated from the `player/mod.rs` root under #756, which also closed the
//! fields to this crate's Player module: reads keep named accessors and every
//! write is a named operation. The status-authority flag has no direct C++
//! member — it distinguishes an authoritative empty load from an owner that was
//! never hydrated — and that meaning is preserved here.

use std::collections::{BTreeMap, BTreeSet};

use wow_core::ObjectGuid;

use super::PlayerQuestStatusRecord;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerQuestGameplayState {
    pub(super) statuses: BTreeMap<u32, PlayerQuestStatusRecord>,
    pub(super) rewarded_quest_ids: BTreeSet<u32>,
    pub(super) daily_quest_ids: BTreeSet<u32>,
    pub(super) weekly_quest_ids: BTreeSet<u32>,
    pub(super) monthly_quest_ids: BTreeSet<u32>,
    pub(super) seasonal_quests: BTreeMap<u16, BTreeMap<u32, u64>>,
    pub(super) df_quest_ids: BTreeSet<u32>,
    pub(super) last_daily_quest_time_secs: i64,
    pub(super) seasonal_quest_changed: bool,
    pub(super) status_authority_complete: bool,
    pub(super) rewarded_quest_rows: BTreeSet<u32>,
    pub(super) pending_share: Option<(ObjectGuid, u32)>,
    pub(super) objective_counts_by_quest: Vec<(u32, Vec<i32>)>,
}

impl PlayerQuestGameplayState {
    // ---- reads -------------------------------------------------------------

    /// C++ `Player::m_QuestStatus`, keyed by quest id.
    #[must_use]
    pub fn statuses_like_cpp(&self) -> &BTreeMap<u32, PlayerQuestStatusRecord> {
        &self.statuses
    }

    /// C++ `Player::GetQuestStatus` reaching one quest's record.
    #[must_use]
    pub fn status_like_cpp(&self, quest_id: u32) -> Option<&PlayerQuestStatusRecord> {
        self.statuses.get(&quest_id)
    }

    /// Mutable access to one identified quest's record, as C++ writes
    /// `itr->second` after finding it in `m_QuestStatus`.
    pub fn status_mut_like_cpp(&mut self, quest_id: u32) -> Option<&mut PlayerQuestStatusRecord> {
        self.statuses.get_mut(&quest_id)
    }

    /// C++ `Player::m_RewardedQuests`.
    #[must_use]
    pub fn rewarded_quest_ids_like_cpp(&self) -> &BTreeSet<u32> {
        &self.rewarded_quest_ids
    }

    #[must_use]
    pub fn daily_quest_ids_like_cpp(&self) -> &BTreeSet<u32> {
        &self.daily_quest_ids
    }

    #[must_use]
    pub fn weekly_quest_ids_like_cpp(&self) -> &BTreeSet<u32> {
        &self.weekly_quest_ids
    }

    #[must_use]
    pub fn monthly_quest_ids_like_cpp(&self) -> &BTreeSet<u32> {
        &self.monthly_quest_ids
    }

    /// C++ `Player::m_seasonalquests`, keyed by event id then quest id.
    #[must_use]
    pub fn seasonal_quests_like_cpp(&self) -> &BTreeMap<u16, BTreeMap<u32, u64>> {
        &self.seasonal_quests
    }

    /// C++ `Player::m_DFQuests`.
    #[must_use]
    pub fn df_quest_ids_like_cpp(&self) -> &BTreeSet<u32> {
        &self.df_quest_ids
    }

    #[must_use]
    pub fn last_daily_quest_time_secs_like_cpp(&self) -> i64 {
        self.last_daily_quest_time_secs
    }

    #[must_use]
    pub fn seasonal_quest_changed_like_cpp(&self) -> bool {
        self.seasonal_quest_changed
    }

    /// Whether the quest statuses are the authoritative loaded set.
    #[must_use]
    pub fn status_authority_complete_like_cpp(&self) -> bool {
        self.status_authority_complete
    }

    /// Rewarded rows as the save projection reads them.
    #[must_use]
    pub fn rewarded_quest_rows_like_cpp(&self) -> &BTreeSet<u32> {
        &self.rewarded_quest_rows
    }

    /// The quest share this Player is currently offered.
    #[must_use]
    pub fn pending_share_like_cpp(&self) -> Option<(ObjectGuid, u32)> {
        self.pending_share
    }

    /// Represented objective counts, keyed by quest.
    #[must_use]
    pub fn objective_counts_by_quest_like_cpp(&self) -> &[(u32, Vec<i32>)] {
        &self.objective_counts_by_quest
    }

    // ---- transitions -------------------------------------------------------

    /// C++ `Player::SetQuestStatus` (`Player.cpp:15557`) installing one record.
    pub fn insert_status_like_cpp(&mut self, quest_id: u32, record: PlayerQuestStatusRecord) {
        self.statuses.insert(quest_id, record);
    }

    /// C++ `Player::RemoveActiveQuest` (`Player.cpp:15575`) erasing one record.
    pub fn remove_status_like_cpp(&mut self, quest_id: u32) -> Option<PlayerQuestStatusRecord> {
        self.statuses.remove(&quest_id)
    }

    /// Iterate every quest record for in-place progress updates, as C++ walks
    /// `m_QuestStatus` when a world event adjusts several quests at once.
    pub fn statuses_mut_like_cpp(&mut self) -> impl Iterator<Item = &mut PlayerQuestStatusRecord> {
        self.statuses.values_mut()
    }

    /// Forget every rewarded row, as the status-authority load restart does.
    pub fn clear_rewarded_quest_rows_like_cpp(&mut self) {
        self.rewarded_quest_rows.clear();
    }

    /// Copy every seasonal event's quests, for a plan that must own them.
    #[must_use]
    pub fn seasonal_quests_snapshot_like_cpp(&self) -> BTreeMap<u16, BTreeMap<u32, u64>> {
        self.seasonal_quests.clone()
    }

    /// Copy every quest record, for a plan that must own them.
    #[must_use]
    pub fn statuses_snapshot_like_cpp(&self) -> BTreeMap<u32, PlayerQuestStatusRecord> {
        self.statuses.clone()
    }

    /// Install the authoritative loaded statuses (`Player::_LoadQuestStatus`).
    pub fn replace_statuses_like_cpp(
        &mut self,
        statuses: BTreeMap<u32, PlayerQuestStatusRecord>,
        complete: bool,
    ) {
        self.statuses = statuses;
        self.status_authority_complete = complete;
    }

    /// Set the status authority on its own, so an authoritative empty load and
    /// an unhydrated owner stay distinguishable.
    pub fn set_status_authority_complete_like_cpp(&mut self, complete: bool) {
        self.status_authority_complete = complete;
    }

    /// C++ `Player::m_RewardedQuests` gaining or losing one quest
    /// (`RemoveRewardedQuest`, `Player.cpp:15595`).
    pub fn set_rewarded_like_cpp(&mut self, quest_id: u32, rewarded: bool) {
        if rewarded {
            self.rewarded_quest_ids.insert(quest_id);
        } else {
            self.rewarded_quest_ids.remove(&quest_id);
        }
    }

    pub fn replace_rewarded_quest_ids_like_cpp(&mut self, quest_ids: BTreeSet<u32>) {
        self.rewarded_quest_ids = quest_ids;
    }

    /// The rewarded rows the save projection persists.
    pub fn replace_rewarded_quest_rows_like_cpp(&mut self, rows: BTreeSet<u32>) {
        self.rewarded_quest_rows = rows;
    }

    pub fn set_rewarded_row_like_cpp(&mut self, quest_id: u32, present: bool) {
        if present {
            self.rewarded_quest_rows.insert(quest_id);
        } else {
            self.rewarded_quest_rows.remove(&quest_id);
        }
    }

    /// C++ `Player::SetDailyQuestStatus` (`Player.cpp:24037`).
    pub fn set_daily_like_cpp(&mut self, quest_id: u32, active: bool) {
        if active {
            self.daily_quest_ids.insert(quest_id);
        } else {
            self.daily_quest_ids.remove(&quest_id);
        }
    }

    pub fn replace_daily_quest_ids_like_cpp(&mut self, quest_ids: BTreeSet<u32>) {
        self.daily_quest_ids = quest_ids;
    }

    /// C++ `Player::SetWeeklyQuestStatus` (`Player.cpp:24061`).
    pub fn set_weekly_like_cpp(&mut self, quest_id: u32, active: bool) {
        if active {
            self.weekly_quest_ids.insert(quest_id);
        } else {
            self.weekly_quest_ids.remove(&quest_id);
        }
    }

    pub fn replace_weekly_quest_ids_like_cpp(&mut self, quest_ids: BTreeSet<u32>) {
        self.weekly_quest_ids = quest_ids;
    }

    /// C++ `Player::SetMonthlyQuestStatus` (`Player.cpp:24077`).
    pub fn set_monthly_like_cpp(&mut self, quest_id: u32, active: bool) {
        if active {
            self.monthly_quest_ids.insert(quest_id);
        } else {
            self.monthly_quest_ids.remove(&quest_id);
        }
    }

    pub fn replace_monthly_quest_ids_like_cpp(&mut self, quest_ids: BTreeSet<u32>) {
        self.monthly_quest_ids = quest_ids;
    }

    /// C++ `Player::SetSeasonalQuestStatus` (`Player.cpp:24067`) keys the quest
    /// under its event and records when it completed.
    pub fn set_seasonal_like_cpp(&mut self, event_id: u16, quest_id: u32, completed_at: u64) {
        self.seasonal_quests
            .entry(event_id)
            .or_default()
            .insert(quest_id, completed_at);
        self.seasonal_quest_changed = true;
    }

    /// C++ `Player::m_seasonalquests` for one event.
    #[must_use]
    pub fn seasonal_event_quests_like_cpp(&self, event_id: u16) -> Option<&BTreeMap<u32, u64>> {
        self.seasonal_quests.get(&event_id)
    }

    /// Drop the seasonal quests of one event that completed before its new
    /// start time, as `Player::ResetSeasonalQuestStatus` does, erasing the
    /// event entry once nothing is left under it.
    pub fn reset_seasonal_event_like_cpp(
        &mut self,
        event_id: u16,
        event_start_time: u64,
    ) -> ResetSeasonalEventLikeCpp {
        let Some(bucket) = self.seasonal_quests.get_mut(&event_id) else {
            return ResetSeasonalEventLikeCpp::default();
        };
        let removed_quest_ids: Vec<u32> = bucket
            .iter()
            .filter_map(|(quest_id, completed_time)| {
                (*completed_time < event_start_time).then_some(*quest_id)
            })
            .collect();
        for quest_id in &removed_quest_ids {
            bucket.remove(quest_id);
        }
        let event_bucket_erased = bucket.is_empty() && !removed_quest_ids.is_empty();
        if event_bucket_erased {
            self.seasonal_quests.remove(&event_id);
        }
        if !removed_quest_ids.is_empty() {
            self.seasonal_quest_changed = true;
        }
        ResetSeasonalEventLikeCpp {
            removed_quest_ids,
            event_bucket_erased,
        }
    }

    /// Seed one stored seasonal completion without marking the state changed.
    ///
    /// C++ `SetSeasonalQuestStatus` (`Player.cpp:24067`) sets
    /// `m_SeasonalQuestChanged` because it is a live completion. Restoring a
    /// previously stored row — a load or a fixture seed — must not, or the next
    /// save would treat untouched state as dirty.
    pub fn seed_seasonal_quest_like_cpp(
        &mut self,
        event_id: u16,
        quest_id: u32,
        completed_at: u64,
    ) {
        self.seasonal_quests
            .entry(event_id)
            .or_default()
            .insert(quest_id, completed_at);
    }

    /// Ensure one seasonal event exists without changing its quests.
    pub fn ensure_seasonal_event_like_cpp(&mut self, event_id: u16) {
        self.seasonal_quests.entry(event_id).or_default();
    }

    pub fn replace_seasonal_quests_like_cpp(
        &mut self,
        seasonal_quests: BTreeMap<u16, BTreeMap<u32, u64>>,
        changed: bool,
    ) {
        self.seasonal_quests = seasonal_quests;
        self.seasonal_quest_changed = changed;
    }

    pub fn set_seasonal_quest_changed_like_cpp(&mut self, changed: bool) {
        self.seasonal_quest_changed = changed;
    }

    /// C++ `Player::m_DFQuests`.
    pub fn set_df_quest_like_cpp(&mut self, quest_id: u32, active: bool) {
        if active {
            self.df_quest_ids.insert(quest_id);
        } else {
            self.df_quest_ids.remove(&quest_id);
        }
    }

    pub fn replace_df_quest_ids_like_cpp(&mut self, quest_ids: BTreeSet<u32>) {
        self.df_quest_ids = quest_ids;
    }

    /// C++ `Player::m_lastDailyQuestTime`.
    pub fn set_last_daily_quest_time_secs_like_cpp(&mut self, time_secs: i64) {
        self.last_daily_quest_time_secs = time_secs;
    }

    /// Record or clear the quest share currently offered to this Player.
    pub fn set_pending_share_like_cpp(&mut self, share: Option<(ObjectGuid, u32)>) {
        self.pending_share = share;
    }

    /// Record one quest's represented objective counts.
    pub fn set_objective_counts_for_quest_like_cpp(&mut self, quest_id: u32, counts: Vec<i32>) {
        if let Some(existing) = self
            .objective_counts_by_quest
            .iter_mut()
            .find(|(id, _)| *id == quest_id)
        {
            existing.1 = counts;
            return;
        }
        self.objective_counts_by_quest.push((quest_id, counts));
    }

    /// Replace the represented objective counts, keyed by quest.
    pub fn replace_objective_counts_by_quest_like_cpp(&mut self, counts: Vec<(u32, Vec<i32>)>) {
        self.objective_counts_by_quest = counts;
    }
}

/// What one seasonal-event reset removed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResetSeasonalEventLikeCpp {
    pub removed_quest_ids: Vec<u32>,
    /// C++ erases the event entry once its last quest is gone.
    pub event_bucket_erased: bool,
}
