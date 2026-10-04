// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Selected GameObject viewer-state rules used during quest refresh.

use std::time::Instant;

use wow_core::ObjectGuid;
use wow_data::{PlayerConditionStore, is_player_meeting_condition_like_cpp};
use wow_entities::GoState;
use wow_world_entities::RepresentedGameObjectUseState;

use super::QuestEligibilityCx;
use crate::PlayerConditionProjectionCxLikeCpp;
use crate::quest::dialog_status::RepresentedQuestGiverStatusSourceLikeCpp;
use crate::quest::represented_gameobject_loot_ids_have_quest_loot_for_player_like_cpp;

use super::super::objectives::QuestObjectiveProgressCx;

/// Resolve per-player state with the original viewer identity and expiry checks.
pub(crate) fn represented_gameobject_go_state_for_viewer_like_cpp(
    state: &RepresentedGameObjectUseState,
    player_guid: Option<ObjectGuid>,
    now: Instant,
) -> GoState {
    if state.per_player_state_player_guid == player_guid
        && state
            .per_player_go_state_until
            .is_none_or(|until| until > now)
        && let Some(per_player_go_state) = state.per_player_go_state
    {
        return per_player_go_state;
    }

    state.go_state.unwrap_or(GoState::Ready)
}

/// C++ `Player::SatisfyPlayerCondition` for a raw GameObject condition id.
///
/// The owned projection is built at this read point; the caller lends the
/// inert condition inputs so no mutable owner is retained here.
fn represented_meets_player_condition_id_like_cpp(
    conditions: &PlayerConditionProjectionCxLikeCpp<'_>,
    store: Option<&PlayerConditionStore>,
    player_condition_id: u32,
) -> bool {
    if player_condition_id == 0 {
        return true;
    }
    let Some(store) = store else {
        return false;
    };
    let Some(condition) = store.get(player_condition_id) else {
        return true;
    };
    let Some(values) = conditions.project_like_cpp() else {
        return false;
    };
    conditions
        .condition_context_like_cpp(&values)
        .is_some_and(|context| is_player_meeting_condition_like_cpp(condition, &context))
}

impl QuestObjectiveProgressCx<'_, '_> {
    fn player_is_game_master_like_cpp(&self) -> bool {
        let access = self.reward.player.quest_objective_access_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            access.player_is_game_master_like_cpp(self.player_game_master_fixture) == Some(true)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            access.player_is_game_master_like_cpp() == Some(true)
        }
    }

    /// C++ `GameObject::ActivateToQuest` with its native per-type gates.
    ///
    /// The caller lends the read-only eligibility context (dialog conditions and
    /// quest eligibility) so this reader never constructs or stores a World
    /// callback.
    pub(super) fn represented_gameobject_activate_to_quest_like_cpp(
        &self,
        eligibility: &QuestEligibilityCx<'_>,
        gameobject_entry: u32,
        state: &RepresentedGameObjectUseState,
    ) -> bool {
        let owner = self.reward.player.quest_objective_access_like_cpp();
        let fixture_fallback = self.reward.world_test_consumer;
        if super::represented_has_quest_for_gameobject_like_cpp(
            &owner,
            self.reward.catalogs,
            self.reward.quest_state,
            gameobject_entry,
            fixture_fallback,
        ) {
            return true;
        }

        if !super::represented_gameobject_is_for_quests_like_cpp(
            self.reward.catalogs,
            &owner,
            self.reward.quest_state,
            gameobject_entry,
            state,
            fixture_fallback,
        ) {
            return false;
        }

        let loot_ids_have_quest_loot = |loot_ids: Vec<u32>| {
            represented_gameobject_loot_ids_have_quest_loot_for_player_like_cpp(
                &owner,
                &self.reward.player.inventory_like_cpp(),
                self.reward.catalogs,
                self.reward.inventory,
                self.reward.quest_state,
                loot_ids,
                self.reward.world_test_consumer,
            )
        };

        match state.go_type.map(u32::from) {
            Some(wow_entities::GAMEOBJECT_TYPE_QUESTGIVER) => {
                let status = eligibility
                    .get_represented_quest_giver_status_with_catalog_like_cpp(
                        self.reward.catalogs.quests.info_store.as_deref(),
                        RepresentedQuestGiverStatusSourceLikeCpp::GameObject {
                            entry: gameobject_entry,
                        },
                    );
                status != wow_packet::packets::quest::quest_giver_status::NONE
                    && status != wow_packet::packets::quest::quest_giver_status::FUTURE
            }
            Some(wow_entities::GAMEOBJECT_TYPE_CHEST) => {
                state.loot_state != Some(wow_entities::LootState::NotReady)
                    && (state.chest_loot_source.is_some_and(|source| {
                        source.chest_quest_id != 0
                            && self.current_quest_status_like_cpp(source.chest_quest_id)
                                == Some(wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP)
                    }) || state.chest_loot_source.is_some_and(|source| {
                        loot_ids_have_quest_loot(source.loot_ids_like_cpp().to_vec())
                    }))
            }
            Some(wow_entities::GAMEOBJECT_TYPE_GENERIC) => {
                super::represented_has_quest_for_gameobject_like_cpp(
                    &owner,
                    self.reward.catalogs,
                    self.reward.quest_state,
                    gameobject_entry,
                    fixture_fallback,
                )
            }
            Some(wow_entities::GAMEOBJECT_TYPE_GOOBER) => state.goober_use_source.is_some_and(
                |source| {
                    source.quest_id != 0
                        && self.current_quest_status_like_cpp(source.quest_id)
                            == Some(wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP)
                },
            ),
            Some(wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE) => state
                .gathering_node_loot_id
                .is_some_and(|loot_id| loot_ids_have_quest_loot(vec![loot_id])),
            _ => false,
        }
    }

    pub(super) fn represented_gameobject_dynamic_flags_for_player_like_cpp(
        &self,
        eligibility: &QuestEligibilityCx<'_>,
        gameobject_entry: u32,
        state: &RepresentedGameObjectUseState,
    ) -> u32 {
        let mut dyn_flags = 0_u32;
        let path_progress = (state.dynamic_flags >> 16) & 0xFFFF;
        let activate_to_quest = self.represented_gameobject_activate_to_quest_like_cpp(
            eligibility,
            gameobject_entry,
            state,
        );

        match state.go_type.map(u32::from) {
            Some(wow_entities::GAMEOBJECT_TYPE_QUESTGIVER) => {
                if activate_to_quest {
                    dyn_flags |= wow_entities::GO_DYNFLAG_LO_ACTIVATE;
                }
            }
            Some(wow_entities::GAMEOBJECT_TYPE_CHEST) => {
                if activate_to_quest {
                    dyn_flags |= wow_entities::GO_DYNFLAG_LO_ACTIVATE
                        | wow_entities::GO_DYNFLAG_LO_SPARKLE
                        | wow_entities::GO_DYNFLAG_LO_HIGHLIGHT;
                } else if self.player_is_game_master_like_cpp() {
                    dyn_flags |= wow_entities::GO_DYNFLAG_LO_ACTIVATE;
                }
            }
            Some(wow_entities::GAMEOBJECT_TYPE_GOOBER) => {
                if activate_to_quest {
                    dyn_flags |= wow_entities::GO_DYNFLAG_LO_HIGHLIGHT;
                    let state_for_player = represented_gameobject_go_state_for_viewer_like_cpp(
                        state,
                        self.reward.player.player_guid_like_cpp(),
                        Instant::now(),
                    );
                    if state_for_player != GoState::Active {
                        dyn_flags |= wow_entities::GO_DYNFLAG_LO_ACTIVATE;
                    }
                } else if self.player_is_game_master_like_cpp() {
                    dyn_flags |= wow_entities::GO_DYNFLAG_LO_ACTIVATE;
                }
            }
            Some(wow_entities::GAMEOBJECT_TYPE_GENERIC) => {
                if activate_to_quest {
                    dyn_flags |=
                        wow_entities::GO_DYNFLAG_LO_SPARKLE | wow_entities::GO_DYNFLAG_LO_HIGHLIGHT;
                }
            }
            Some(wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE) => {
                if activate_to_quest {
                    dyn_flags |= wow_entities::GO_DYNFLAG_LO_ACTIVATE
                        | wow_entities::GO_DYNFLAG_LO_SPARKLE
                        | wow_entities::GO_DYNFLAG_LO_HIGHLIGHT;
                }
                let state_for_player = represented_gameobject_go_state_for_viewer_like_cpp(
                    state,
                    self.reward.player.player_guid_like_cpp(),
                    Instant::now(),
                );
                if state_for_player == GoState::Active {
                    dyn_flags |= wow_entities::GO_DYNFLAG_LO_DEPLETED;
                }
            }
            _ => {
                dyn_flags = state.dynamic_flags & 0xFFFF;
            }
        }

        if state.condition_id1.is_some_and(|id| {
            !represented_meets_player_condition_id_like_cpp(
                eligibility.conditions,
                self.reward.catalogs.player_condition_store.as_deref(),
                id,
            )
        }) {
            dyn_flags |= wow_entities::GO_DYNFLAG_LO_NO_INTERACT;
        }

        (path_progress << 16) | dyn_flags
    }
}
