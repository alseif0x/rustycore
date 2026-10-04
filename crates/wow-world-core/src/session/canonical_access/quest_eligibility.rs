// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Read-only canonical Player inputs selected for quest admission.

#[cfg(any(test, feature = "test-fixtures"))]
use std::collections::HashMap;

use wow_data::progression_rewards::FactionEntry;
use wow_entities::PlayerQuestGameplayState;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_entities::PlayerReputationStateLikeCpp;

use crate::session::{QuestObjectiveAccessLikeCpp, SessionCore};

/// Selected, shared-borrow access for the complete quest-admission read path.
pub struct QuestEligibilityAccessLikeCpp<'a> {
    core: &'a SessionCore,
    #[cfg(any(test, feature = "test-fixtures"))]
    race: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    class: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    level: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    skill_records: &'a HashMap<u16, crate::session::RepresentedPlayerSkillLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    reputation: &'a PlayerReputationStateLikeCpp,
}

impl SessionCore {
    /// Select the Core owner and optional read-only fixture inputs without taking snapshots.
    pub fn quest_eligibility_access_like_cpp<'a>(
        &'a self,
        #[cfg(any(test, feature = "test-fixtures"))]
        race: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))]
        class: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))]
        level: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))]
        skill_records: &'a HashMap<u16, crate::session::RepresentedPlayerSkillLikeCpp>,
        #[cfg(any(test, feature = "test-fixtures"))]
        reputation: &'a PlayerReputationStateLikeCpp,
    ) -> QuestEligibilityAccessLikeCpp<'a> {
        QuestEligibilityAccessLikeCpp {
            core: self,
            #[cfg(any(test, feature = "test-fixtures"))]
            race,
            #[cfg(any(test, feature = "test-fixtures"))]
            class,
            #[cfg(any(test, feature = "test-fixtures"))]
            level,
            #[cfg(any(test, feature = "test-fixtures"))]
            skill_records,
            #[cfg(any(test, feature = "test-fixtures"))]
            reputation,
        }
    }
}

impl QuestEligibilityAccessLikeCpp<'_> {
    pub fn account_id_like_cpp(&self) -> u32 {
        self.core.account_id
    }

    pub fn expansion_like_cpp(&self) -> u8 {
        self.core.expansion
    }

    pub fn player_race_like_cpp(&self) -> u8 {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core.player_race_with_fixture_like_cpp(self.race)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_race_with_fixture_like_cpp()
        }
    }

    pub fn player_class_like_cpp(&self) -> u8 {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core.player_class_with_fixture_like_cpp(self.class)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_class_with_fixture_like_cpp()
        }
    }

    pub fn player_level_like_cpp(&self) -> u8 {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core.player_level_with_fixture_like_cpp(self.level)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_level_with_fixture_like_cpp()
        }
    }

    pub fn resolved_quest_skill_value_like_cpp(&self, skill_id: u16) -> Option<u16> {
        let records = self.core.resolved_player_skill_records_for_publication_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            self.skill_records,
        )?;
        Some(
            crate::session::represented_skill_values_from_records_like_cpp(&records)
                .get(&skill_id)
                .copied()
                .unwrap_or(0),
        )
    }

    pub fn quest_reputation_for_faction_like_cpp(
        &self,
        faction: &FactionEntry,
    ) -> Option<i32> {
        super::quest_reward_owner::quest_reputation_for_faction_like_cpp(
            self.core,
            faction,
            self.player_race_like_cpp(),
            self.player_class_like_cpp(),
            #[cfg(any(test, feature = "test-fixtures"))]
            self.reputation,
        )
    }

    pub fn player_quest_gameplay_snapshot_like_cpp(&self) -> Option<PlayerQuestGameplayState> {
        self.core
            .player_registry_hydration_access_like_cpp()
            .owned_player_quest_gameplay_snapshot_like_cpp()
    }

    pub fn quest_objective_access_like_cpp(&self) -> QuestObjectiveAccessLikeCpp<'_> {
        self.core.quest_objective_access_like_cpp()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn owner_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }

}
