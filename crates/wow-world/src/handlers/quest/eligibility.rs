// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest availability: status classification and the `SatisfyQuest*` gates.

use super::*;

impl WorldSession {
    pub(crate) fn get_represented_quest_giver_status_with_catalog_like_cpp(
        &self,
        quest_info: Option<&wow_data::progression_rewards::QuestInfoStore>,
        source: RepresentedQuestGiverStatusSourceLikeCpp,
    ) -> u64 {
        let player = self.core.quest_eligibility_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_race,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_class,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self
                .fixtures
                .progression
                .player_skill_test_fixture_like_cpp
                .player_skill_records_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.progression.reputation_state_like_cpp,
        );
        let conditions = self.player_condition_projection_cx_like_cpp();
        wow_world_application::QuestEligibilityCx::new(
            player,
            &self.quest_state,
            &self.catalogs,
            &conditions,
            cfg!(test),
        )
        .get_represented_quest_giver_status_with_catalog_like_cpp(quest_info, source)
    }

    pub(crate) fn represented_quest_is_important_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        WorldSession::represented_quest_dialog_classification_like_cpp(
            quest,
            self.catalogs.quests.info_store.as_deref(),
        )
        .is_important()
    }

    /// C++ anchor: `Player::CanTakeQuest` (Player.cpp:14090–14102).
    /// This keeps the current represented Rust gate order; `SatisfyQuestTimed`
    /// remains unrepresented, so this is not a full-parity claim.
    pub(crate) fn can_take_quest(&self, quest: &wow_data::quest::QuestTemplate) -> bool {
        self.with_quest_eligibility_cx_like_cpp(|operation| {
            operation.can_take_quest_like_cpp(quest)
        })
    }

    /// C++ anchor: `Player::CanSeeStartQuest` (Player.cpp:14073–14085).
    ///
    /// `HandleQuestgiverCompleteQuest` asks this before it answers a dialog
    /// request, so the represented bounded projection is exposed here rather
    /// than duplicated in the handler.
    pub(crate) fn can_see_start_quest_represented_bounded_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        self.with_quest_eligibility_cx_like_cpp(|operation| {
            operation.can_see_start_quest_like_cpp(quest)
        })
    }

    /// Build the represented eligibility operation over the canonical Player
    /// access, the quest state and the condition projection.
    fn with_quest_eligibility_cx_like_cpp<R>(
        &self,
        run: impl FnOnce(&wow_world_application::QuestEligibilityCx<'_>) -> R,
    ) -> R {
        let player = self.core.quest_eligibility_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_race,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_class,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self
                .fixtures
                .progression
                .player_skill_test_fixture_like_cpp
                .player_skill_records_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.progression.reputation_state_like_cpp,
        );
        let conditions = self.player_condition_projection_cx_like_cpp();
        run(&wow_world_application::QuestEligibilityCx::new(
            player,
            &self.quest_state,
            &self.catalogs,
            &conditions,
            cfg!(test),
        ))
    }

    pub(crate) fn is_quest_disabled_like_cpp(&self, quest_id: u32) -> bool {
        wow_world_application::quest_is_disabled_like_cpp(crate::session::hub_ref(self), quest_id)
    }
}

impl crate::session::QuestStateCxRef<'_> {
    /// Resolves CMSG_QUEST_GIVER_STATUS_QUERY through the represented equivalent of
    /// C++ `ObjectAccessor::GetObjectByTypeMask(*_player, guid, TYPEMASK_UNIT | TYPEMASK_GAMEOBJECT)`.
    /// Missing canonical objects and unsupported Player/Item/other GUID types fail closed with no packet.
    pub(crate) fn represented_quest_giver_status_query_source_like_cpp(
        &self,
        guid: wow_core::ObjectGuid,
    ) -> Option<RepresentedQuestGiverStatusSourceLikeCpp> {
        if guid.is_any_type_creature() {
            // C++ TYPEID_UNIT branch also checks Creature::IsHostileTo before computing
            // dialog status. Exact faction/hostility is not represented here yet; a
            // resolved canonical Creature is treated as non-hostile only for this
            // bounded represented status calculation.
            let access = self
                .world_entities
                .canonical_creature_access_like_cpp(self.hub, guid)?;
            return Some(RepresentedQuestGiverStatusSourceLikeCpp::Creature {
                entry: access.entry,
            });
        }

        if guid.is_game_object() {
            let access = self.hub.core.canonical_gameobject_access_like_cpp(guid)?;
            return Some(RepresentedQuestGiverStatusSourceLikeCpp::GameObject {
                entry: access.entry,
            });
        }

        None
    }
}
