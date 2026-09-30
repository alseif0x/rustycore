//! catalog operations at the existing Quest application boundary.

use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_quest_login_aura_sources_are_hit_inert_like_cpp(
        &self,
        difficulty_id: u8,
    ) -> bool {
        let Some(state) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        if !state.status_authority_complete_like_cpp() {
            return false;
        }
        let Some(quests) = self.quests.store.as_ref() else {
            return state.rewarded_quest_rows_like_cpp().is_empty()
                && state.statuses_like_cpp().is_empty();
        };

        // C++ `_LoadQuestStatusRewarded` calls `LearnQuestRewardedSpells`
        // before filtering special quests out of `m_RewardedQuests`. Keep the
        // narrow TESTBOT proof to rows whose static reward spell is absent.
        if state.rewarded_quest_rows_like_cpp().iter().any(|quest_id| {
            quests
                .get(*quest_id)
                .is_none_or(|quest| quest.reward_spell != 0)
        }) {
            return false;
        }

        state.statuses_like_cpp().values().all(|status| {
            let Some(quest) = quests.get(status.quest_id) else {
                return false;
            };
            let recasts_accept_spell = quest.flags & QUEST_FLAGS_PLAYER_CAST_ACCEPT_LIKE_CPP != 0
                && quest.flags_ex & QUEST_FLAGS_EX_RECAST_ACCEPT_SPELL_ON_LOGIN_LIKE_CPP != 0
                && quest.source_spell_id != 0;
            !recasts_accept_spell
                || self
                    .player_target_spell_is_hit_inert_like_cpp(quest.source_spell_id, difficulty_id)
        })
    }
    /// C++ `Player::PushQuests` scans the complete global quest-template map
    /// during login and area updates. `AddQuest` can cast an AUTO_PUSH quest's
    /// SourceSpellID, so the narrow empty-source proof requires that the
    /// authoritative store contain no such template at all.
    pub(in crate::session) fn represented_auto_push_quest_aura_source_is_empty_like_cpp(
        &self,
    ) -> bool {
        const QUEST_FLAGS_EX_AUTO_PUSH_LIKE_CPP: u32 = 0x0400_0000;
        self.quests.store.as_ref().is_some_and(|quests| {
            quests
                .quests_like_cpp()
                .all(|quest| quest.flags_ex & QUEST_FLAGS_EX_AUTO_PUSH_LIKE_CPP == 0)
        })
    }
    pub(crate) fn spell_area_for_quest_map_bounds_like_cpp(
        &self,
        quest_id: u32,
    ) -> Vec<&SpellAreaLikeCpp> {
        self.spell_catalogs
            .spell_area_store
            .as_ref()
            .map(|store| store.spell_area_for_quest_map_bounds_like_cpp(quest_id))
            .unwrap_or_default()
    }
    pub(crate) fn spell_area_for_quest_end_map_bounds_like_cpp(
        &self,
        quest_id: u32,
    ) -> Vec<&SpellAreaLikeCpp> {
        self.spell_catalogs
            .spell_area_store
            .as_ref()
            .map(|store| store.spell_area_for_quest_end_map_bounds_like_cpp(quest_id))
            .unwrap_or_default()
    }
    /// Set the quest store shared reference.
    pub fn set_quest_store(&mut self, store: Arc<wow_data::quest::QuestStore>) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.quests.store = Some(store);
    }
    /// Set the QuestV2 store shared reference used for C++ quest unique-bit lookups.
    pub fn set_quest_v2_store(&mut self, store: Arc<QuestV2Store>) {
        self.quests.v2_store = Some(store);
    }
    /// Set the QuestInfo store used by C++ Quest::GetQuestTag/IsImportant.
    pub fn set_quest_info_store(&mut self, store: Arc<QuestInfoStore>) {
        self.quests.info_store = Some(store);
    }
}
