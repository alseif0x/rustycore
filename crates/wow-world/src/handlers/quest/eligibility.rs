// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest availability: status classification and the `SatisfyQuest*` gates.

use super::*;

impl WorldSession {
    /// World facade for the Application-owned represented dialog-status operation.
    pub(crate) fn get_represented_quest_giver_status_like_cpp(
        &self,
        source: RepresentedQuestGiverStatusSourceLikeCpp,
    ) -> u64 {
        self.get_represented_quest_giver_status_with_catalog_like_cpp(
            self.catalogs.quests.info_store.as_deref(),
            source,
        )
    }

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

    fn satisfy_quest_skill_like_cpp(&self, quest: &wow_data::quest::QuestTemplate) -> bool {
        if quest.required_skill_id == 0 {
            return true;
        }
        let Ok(skill_u16) = u16::try_from(quest.required_skill_id) else {
            return true;
        };
        crate::session::hub_ref(self)
            .resolved_player_skill_value_like_cpp(skill_u16)
            .is_some_and(|value| u32::from(value) >= quest.required_skill_points)
    }

    fn satisfy_quest_reputation_like_cpp(&self, quest: &wow_data::quest::QuestTemplate) -> bool {
        let hub = crate::session::hub_ref(self);
        if quest.required_min_rep_faction != 0 {
            let rep = match hub
                .catalogs
                .faction_store()
                .and_then(|store| store.get(quest.required_min_rep_faction))
            {
                Some(faction_entry) => {
                    let player_race = hub.player_race_like_cpp();
                    let player_class = hub.player_class_like_cpp();
                    let Some(rep) = hub.with_reputation_mgr_like_cpp(|mgr| {
                        mgr.reputation_for_faction_like_cpp(
                            faction_entry,
                            player_race,
                            player_class,
                        )
                    }) else {
                        return false;
                    };
                    rep
                }
                None => 0,
            };
            if rep < quest.required_min_rep_value {
                return false;
            }
        }

        if quest.required_max_rep_faction != 0 {
            let rep = match hub
                .catalogs
                .faction_store()
                .and_then(|store| store.get(quest.required_max_rep_faction))
            {
                Some(faction_entry) => {
                    let player_race = hub.player_race_like_cpp();
                    let player_class = hub.player_class_like_cpp();
                    let Some(rep) = hub.with_reputation_mgr_like_cpp(|mgr| {
                        mgr.reputation_for_faction_like_cpp(
                            faction_entry,
                            player_race,
                            player_class,
                        )
                    }) else {
                        return false;
                    };
                    rep
                }
                None => 0,
            };
            if rep >= quest.required_max_rep_value {
                return false;
            }
        }

        true
    }

    // SatisfyQuestExclusiveGroup — Player.cpp:15348-15391
    //
    // Only positive exclusive_group values restrict: a positive group means "take
    // at most one quest from this set".  Non-positive (0 or negative) groups are
    // unused/unrestricted → always true (Player.cpp:15351).
    //
    // quest_store None → fail-open: without the store we cannot enumerate peers,
    // so we conservatively allow the quest rather than silently blocking it.  The
    // same fail-open pattern is used throughout can_take_quest for missing stores.
    fn satisfy_quest_exclusive_group_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        // Player.cpp:15351 — non-positive exclusive_group never restricts
        if quest.exclusive_group <= 0 {
            return true;
        }

        let Some(quest_store) = &self.catalogs.quests.store else {
            return true;
        };
        let Some(recurrence) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };

        for peer in quest_store
            .quests
            .values()
            .filter(|c| c.exclusive_group == quest.exclusive_group)
        {
            // Player.cpp:15360 — skip the quest being evaluated
            if peer.id == quest.id {
                continue;
            }

            // Player.cpp:15366 — SatisfyQuestDay: daily/DF cooldown blocks the group
            // Mirrors the daily/DF pattern from the push path (quest.rs:271-278).
            if peer.is_df_quest_like_cpp() && recurrence.df_quest_ids_like_cpp().contains(&peer.id)
            {
                return false;
            }
            if peer.is_daily_like_cpp() && recurrence.daily_quest_ids_like_cpp().contains(&peer.id)
            {
                return false;
            }

            // Player.cpp:15366 — SatisfyQuestWeek: weekly cooldown blocks the group
            if peer.is_weekly_like_cpp()
                && recurrence.weekly_quest_ids_like_cpp().contains(&peer.id)
            {
                return false;
            }

            // Player.cpp:15366 — SatisfyQuestSeasonal: seasonal cooldown blocks the group
            // Mirrors the seasonal pattern from can_take_quest (quest.rs:5948-5963).
            if peer.is_seasonal_like_cpp() && !recurrence.seasonal_quests_like_cpp().is_empty() {
                if let Some(bucket) = recurrence
                    .seasonal_quests_like_cpp()
                    .get(&peer.event_id_for_quest_like_cpp())
                {
                    if !bucket.is_empty() && bucket.contains_key(&peer.id) {
                        return false;
                    }
                }
            }

            // Player.cpp:15379 — alternative quest already active or rewarded (non-repeatable pair).
            //
            // C++: GetQuestStatus(peer) != QUEST_STATUS_NONE
            //   → in C++ GetQuestStatus returns REWARDED when rewarded, so this single
            //     term would also catch rewarded quests.  We model the two cases separately
            //     to keep the Rust representation explicit:
            //   Term 1: peer is currently active in player_quests (Incomplete/Complete/Failed).
            //   Term 2: peer was rewarded AND not both quests are repeatable (matching the
            //           C++ second OR operand: GetQuestRewardStatus + !IsRepeatable pair).
            if recurrence.statuses_like_cpp().contains_key(&peer.id) {
                return false;
            }
            if !(quest.is_repeatable() && peer.is_repeatable())
                && recurrence.rewarded_quest_ids_like_cpp().contains(&peer.id)
            {
                return false;
            }
        }

        true
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

    /// Check if the player currently has an active quest with the given ID.
    pub fn has_quest(&self, quest_id: u32) -> bool {
        self.player_quest_gameplay_snapshot_like_cpp()
            .is_some_and(|state| state.statuses_like_cpp().contains_key(&quest_id))
    }

    /// C++ anchor: `Player::CanTakeQuest` (Player.cpp:14090–14102).
    /// This keeps the current represented Rust gate order; `SatisfyQuestTimed`
    /// remains unrepresented, so this is not a full-parity claim.
    pub fn can_take_quest(&self, quest: &wow_data::quest::QuestTemplate) -> bool {
        self.with_quest_eligibility_cx_like_cpp(|operation| operation.can_take_quest_like_cpp(quest))
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
        crate::session::hub_ref(self)
            .catalogs
            .disable_mgr()
            .is_some_and(|disable_mgr| {
                disable_mgr.is_disabled_for_like_cpp(
                    DISABLE_TYPE_QUEST,
                    quest_id,
                    None,
                    0,
                    None,
                )
            })
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
