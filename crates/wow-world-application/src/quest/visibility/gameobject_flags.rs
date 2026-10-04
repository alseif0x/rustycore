// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Selected GameObject viewer-state rules used during quest refresh.

use std::time::Instant;

use wow_core::ObjectGuid;
use wow_entities::{GoState, RepresentedGameObjectUseState};

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

    pub(super) fn represented_gameobject_dynamic_flags_for_player_like_cpp(
        &self,
        gameobject_entry: u32,
        state: &RepresentedGameObjectUseState,
    ) -> u32 {
        let mut dyn_flags = 0_u32;
        let path_progress = (state.dynamic_flags >> 16) & 0xFFFF;
        let activate_to_quest =
            self.represented_gameobject_activate_to_quest_like_cpp(gameobject_entry, state);

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

        if state
            .condition_id1
            .is_some_and(|id| !self.represented_meets_player_condition_id_like_cpp(id))
        {
            dyn_flags |= wow_entities::GO_DYNFLAG_LO_NO_INTERACT;
        }

        (path_progress << 16) | dyn_flags
    }
}
