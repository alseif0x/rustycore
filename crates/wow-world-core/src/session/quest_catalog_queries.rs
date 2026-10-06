// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::sync::Arc;

impl crate::session::state::SessionCatalogs {
    /// C++ `Player::PushQuests` scans the complete global quest-template map
    /// during login and area updates. `AddQuest` can cast an AUTO_PUSH quest's
    /// SourceSpellID, so the narrow empty-source proof requires that the
    /// authoritative store contain no such template at all.
    pub fn represented_auto_push_quest_aura_source_is_empty_like_cpp(&self) -> bool {
        const QUEST_FLAGS_EX_AUTO_PUSH_LIKE_CPP: u32 = 0x0400_0000;
        self.quests.store.as_ref().is_some_and(|quests| {
            quests
                .quests_like_cpp()
                .all(|quest| quest.flags_ex & QUEST_FLAGS_EX_AUTO_PUSH_LIKE_CPP == 0)
        })
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn represented_quest_can_increase_rewarded_counters_like_cpp(
        &self,
        quest_id: u32,
    ) -> Option<bool> {
        self.quests.store.as_ref()?.get(quest_id).map(|quest| {
            !quest.is_df_quest_like_cpp()
                && !quest.is_daily_like_cpp()
                && (!quest.is_repeatable()
                    || quest.is_weekly_like_cpp()
                    || quest.is_monthly_like_cpp()
                    || quest.is_seasonal_like_cpp())
        })
    }
}

impl crate::session::state::SessionCatalogs {
    /// Set the represented QuestPoolMgr active snapshot shared reference.
    pub fn set_quest_pool_store(&mut self, store: Arc<wow_data::quest::QuestPoolStoreLikeCpp>) {
        self.quests.pool_store = Some(store);
    }
}
