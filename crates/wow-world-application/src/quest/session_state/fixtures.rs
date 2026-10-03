// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use wow_entities::PlayerQuestStatusRecord;

use super::contracts::{
    RepresentedPendingQuestSharingLikeCpp, RepresentedQuestConfirmAcceptLikeCpp,
    RepresentedQuestPushResultResponseLikeCpp, RepresentedQuestRewardMailLikeCpp,
    RepresentedQuestRewardReputationLikeCpp, RepresentedQuestRewardSpellCastLikeCpp,
    RepresentedQuestRewardTalentPointsLikeCpp, RepresentedQuestRewardTitleLikeCpp,
    RepresentedPushQuestToPartyOutcomeLikeCpp,
};

pub(super) struct QuestTestFixtureLikeCpp {
    /// Active quests for this player: quest_id → status.
    pub(super) player_quests: HashMap<u32, PlayerQuestStatusRecord>,
    /// Quests the player has already been rewarded for (non-repeatable quests cannot be re-taken).
    /// C++ `Player::m_RewardedQuests`.
    pub(super) rewarded_quests: HashSet<u32>,
    /// Exact C++ GetQuestStatus source proof. Both active and rewarded status
    /// queries must succeed for the active Player before absence is evidence.
    pub(super) player_quest_status_authority_complete_like_cpp: bool,
    /// Every row read from `character_queststatus_rewarded`, including special
    /// quests that C++ deliberately omits from `m_RewardedQuests` after
    /// applying their login reward-spell side effects.
    pub(super) represented_rewarded_quest_rows_like_cpp: BTreeSet<u32>,
    /// C++ `ActivePlayerData::DailyQuestsCompleted`, represented per-session until full Player runtime owns it.
    pub(super) daily_quests_completed_like_cpp: HashSet<u32>,
    /// C++ `Player::m_DFQuests`, represented per-session until full Player runtime owns it.
    pub(super) df_quests_like_cpp: HashSet<u32>,
    /// C++ `Player::m_weeklyquests`, represented per-session until full Player runtime owns it.
    pub(super) weekly_quests_completed_like_cpp: HashSet<u32>,
    /// C++ `Player::m_monthlyquests`, represented per-session until full Player runtime owns it.
    pub(super) monthly_quests_completed_like_cpp: HashSet<u32>,
    /// C++ `Player::m_lastDailyQuestTime`, represented for daily/DF quest status persistence.
    pub(super) last_daily_quest_time_like_cpp: i64,
    /// C++ `Player::m_seasonalquests`, represented per-session until full Player runtime owns it.
    pub(super) seasonal_quests_like_cpp: BTreeMap<u16, BTreeMap<u32, u64>>,
    /// C++ `Player::m_SeasonalQuestChanged` represented flag.
    pub(super) seasonal_quest_changed_like_cpp: bool,
    /// Session-local evidence for represented `Player::RemoveTimedQuest` calls.
    pub(super) represented_timed_quest_removals_like_cpp: Vec<u32>,
    /// Session-local evidence for represented quest reward `Player::UpdateSkillPro` calls.
    pub(super) represented_quest_reward_skill_updates_like_cpp: Vec<(u32, u32)>,
    /// Session-local evidence for represented quest reward triggered spell casts.
    pub(super) represented_quest_reward_spell_casts_like_cpp:
        Vec<RepresentedQuestRewardSpellCastLikeCpp>,
    /// Session-local evidence for represented quest reward `SetTitle` calls.
    pub(super) represented_quest_reward_titles_like_cpp: Vec<RepresentedQuestRewardTitleLikeCpp>,
    /// C++ `ActivePlayerData::KnownTitles` represented as title bit indexes.
    pub(super) represented_known_titles_like_cpp: HashSet<u32>,
    /// C++ `PlayerData::PlayerTitle` represented chosen title id.
    pub(super) represented_chosen_title_like_cpp: i32,
    /// Session-local evidence for represented quest reward talent point grants.
    pub(super) represented_quest_reward_talent_points_like_cpp:
        Vec<RepresentedQuestRewardTalentPointsLikeCpp>,
    /// Session-local evidence for represented quest reward mail.
    pub(super) represented_quest_reward_mails_like_cpp: Vec<RepresentedQuestRewardMailLikeCpp>,
    /// Session-local evidence for represented quest reward reputation.
    pub(super) represented_quest_reward_reputations_like_cpp:
        Vec<RepresentedQuestRewardReputationLikeCpp>,
    /// Bridge for quest completed unique-bit state loaded before the canonical Player snapshot exists.
    pub(super) represented_quest_completed_bits_like_cpp: BTreeSet<u32>,
    /// Session-local evidence for represented `ScriptMgr::OnQuestAcknowledgeAutoAccept` calls.
    pub(super) represented_auto_accept_acknowledged_quests_like_cpp: Vec<u32>,
    /// Session-local representation of C++ pending shared quest sender + quest id.
    pub(super) represented_pending_quest_sharing_like_cpp:
        Option<RepresentedPendingQuestSharingLikeCpp>,
    /// Evidence-only replacement for sender `SendPushToPartyResponse` until safe cross-session fanout exists.
    pub(super) represented_quest_push_result_responses_like_cpp:
        Vec<RepresentedQuestPushResultResponseLikeCpp>,
    /// Explicit mismatch counter: C++ still clears pending sharing when sender GUID differs.
    pub(super) represented_quest_push_result_sender_mismatch_count_like_cpp: u32,
    /// Evidence for represented `HandleQuestConfirmAccept` after clear + template lookup.
    pub(super) represented_quest_confirm_accepts_like_cpp:
        Vec<RepresentedQuestConfirmAcceptLikeCpp>,
    /// Session-local evidence for represented sender-side `HandlePushQuestToParty` preflight.
    pub(super) represented_push_quest_to_party_outcomes_like_cpp:
        Vec<RepresentedPushQuestToPartyOutcomeLikeCpp>,
}

impl Default for QuestTestFixtureLikeCpp {
    fn default() -> Self {
        Self {
            player_quests: HashMap::new(),
            rewarded_quests: HashSet::new(),
            player_quest_status_authority_complete_like_cpp: false,
            represented_rewarded_quest_rows_like_cpp: BTreeSet::new(),
            daily_quests_completed_like_cpp: HashSet::new(),
            df_quests_like_cpp: HashSet::new(),
            weekly_quests_completed_like_cpp: HashSet::new(),
            monthly_quests_completed_like_cpp: HashSet::new(),
            last_daily_quest_time_like_cpp: 0,
            seasonal_quests_like_cpp: BTreeMap::new(),
            seasonal_quest_changed_like_cpp: false,
            represented_timed_quest_removals_like_cpp: Vec::new(),
            represented_quest_reward_skill_updates_like_cpp: Vec::new(),
            represented_quest_reward_spell_casts_like_cpp: Vec::new(),
            represented_quest_reward_titles_like_cpp: Vec::new(),
            represented_known_titles_like_cpp: HashSet::new(),
            represented_chosen_title_like_cpp: 0,
            represented_quest_reward_talent_points_like_cpp: Vec::new(),
            represented_quest_reward_mails_like_cpp: Vec::new(),
            represented_quest_reward_reputations_like_cpp: Vec::new(),
            represented_quest_completed_bits_like_cpp: BTreeSet::new(),
            represented_auto_accept_acknowledged_quests_like_cpp: Vec::new(),
            represented_pending_quest_sharing_like_cpp: None,
            represented_quest_push_result_responses_like_cpp: Vec::new(),
            represented_quest_push_result_sender_mismatch_count_like_cpp: 0,
            represented_quest_confirm_accepts_like_cpp: Vec::new(),
            represented_push_quest_to_party_outcomes_like_cpp: Vec::new(),
        }
    }
}
#[cfg(any(test, feature = "test-fixtures"))]
mod quest_status;
#[cfg(any(test, feature = "test-fixtures"))]
mod rewards;
#[cfg(any(test, feature = "test-fixtures"))]
mod sharing;
