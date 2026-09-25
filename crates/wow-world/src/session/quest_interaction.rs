// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest interaction: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{ObjectGuid, RepresentedGameObjectAccessLikeCpp, RepresentedGameObjectUseState};
use super::{WorldSession, quest};

impl WorldSession {
    pub(crate) fn represented_gameobject_questgiver_can_interact_with_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectAccessLikeCpp> {
        // C++ anchor: Player::CanInteractWithQuestGiver(TYPEID_GAMEOBJECT)
        // delegates to GetGameObjectIfCanInteractWith(guid, GAMEOBJECT_TYPE_QUESTGIVER).
        // This represented guard consumes canonical map access plus locally recorded
        // template type/radius from the GO-use path. The C++ IconName == "Point"
        // rejection is represented earlier in handle_game_obj_use before runtime state
        // is registered/consumed here; standalone paths without represented type state
        // fail closed instead of treating canonical existence as interactability.
        let access = self.canonical_gameobject_access_like_cpp(guid)?;
        let state = self.represented_gameobject_use_states.get(&guid)?;
        if state.go_type.map(u32::from) != Some(wow_entities::GAMEOBJECT_TYPE_QUESTGIVER) {
            return None;
        }
        let player_position = self.player_position_like_cpp()?;
        let interaction_distance = state
            .interact_radius_override
            .filter(|value| *value != 0)
            .map_or(5.5555553, |override_hundredths| {
                override_hundredths as f32 / 100.0
            });
        access
            .position
            .is_within_dist(&player_position, interaction_distance)
            .then_some(access)
    }

    pub(in crate::session) fn represented_has_quest_for_gameobject_like_cpp(
        &self,
        gameobject_entry: u32,
    ) -> bool {
        let Some(store) = self.quests.store.as_ref() else {
            return false;
        };
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        let object_id = i32::try_from(gameobject_entry).unwrap_or(i32::MAX);
        quests.statuses_like_cpp().values().any(|status| {
            if status.status != wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                return false;
            }
            let Some(quest) = store.get(status.quest_id) else {
                return false;
            };
            quest
                .objectives
                .iter()
                .enumerate()
                .any(|(index, objective)| {
                    objective.obj_type == 2
                        && objective.object_id == object_id
                        && wow_entities::represented_quest_objective_completable_like_cpp(
                            status,
                            &quest.objective_rules_like_cpp(),
                            index,
                        )
                        && !wow_entities::represented_quest_objective_complete_like_cpp(
                            status,
                            &quest.objective_rules_like_cpp(),
                            objective,
                        )
                })
        })
    }

    pub(crate) fn record_represented_gameobject_template_quest_source_like_cpp(
        &mut self,
        guid: ObjectGuid,
        template: &wow_entities::GameObjectTemplateData,
    ) {
        let state = self
            .represented_gameobject_use_states
            .entry(guid)
            .or_default();
        if let Some(source) = template.chest_loot_source_like_cpp() {
            state.chest_loot_source = Some(source);
        }
        if let Some(source) = template.gathering_node_use_source_like_cpp() {
            state.gathering_node_loot_id = Some(source.loot_id);
        }
        state.condition_id1 = (template.get_condition_id1_like_cpp() != 0)
            .then_some(template.get_condition_id1_like_cpp());
    }

    pub(in crate::session) fn represented_gameobject_is_for_quests_like_cpp(
        &self,
        gameobject_entry: u32,
        state: &RepresentedGameObjectUseState,
    ) -> bool {
        match state.go_type.map(u32::from) {
            Some(wow_entities::GAMEOBJECT_TYPE_QUESTGIVER) => true,
            // C++ anchor: /home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:8791-8800
            Some(wow_entities::GAMEOBJECT_TYPE_CHEST) => {
                self.represented_has_quest_for_gameobject_like_cpp(gameobject_entry)
                    || state
                        .chest_loot_source
                        .is_some_and(|source| source.chest_quest_id != 0)
                    || state.chest_loot_source.is_some_and(|source| {
                        self.represented_gameobject_loot_ids_have_quest_loot_like_cpp(
                            source.loot_ids_like_cpp(),
                        )
                    })
            }
            Some(wow_entities::GAMEOBJECT_TYPE_GENERIC) => {
                self.represented_has_quest_for_gameobject_like_cpp(gameobject_entry)
            }
            Some(wow_entities::GAMEOBJECT_TYPE_GOOBER) => state
                .goober_use_source
                .is_some_and(|source| source.quest_id != 0),
            Some(wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE) => self
                .represented_gameobject_loot_ids_have_quest_loot_like_cpp(
                    state.gathering_node_loot_id,
                ),
            _ => false,
        }
    }

    pub(in crate::session) fn represented_gameobject_activate_to_quest_like_cpp(
        &self,
        gameobject_entry: u32,
        state: &RepresentedGameObjectUseState,
    ) -> bool {
        if self.represented_has_quest_for_gameobject_like_cpp(gameobject_entry) {
            return true;
        }

        if !self.represented_gameobject_is_for_quests_like_cpp(gameobject_entry, state) {
            return false;
        }

        match state.go_type.map(u32::from) {
            Some(wow_entities::GAMEOBJECT_TYPE_QUESTGIVER) => {
                let status = self.get_represented_quest_giver_status_like_cpp(
                    crate::handlers::quest::RepresentedQuestGiverStatusSourceLikeCpp::GameObject {
                        entry: gameobject_entry,
                    },
                );
                status != wow_packet::packets::quest::quest_giver_status::NONE
                    && status != wow_packet::packets::quest::quest_giver_status::FUTURE
            }
            // C++ anchor: /home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.cpp:2236-2251
            Some(wow_entities::GAMEOBJECT_TYPE_CHEST) => {
                state.loot_state != Some(wow_entities::LootState::NotReady)
                    && (state.chest_loot_source.is_some_and(|source| {
                        source.chest_quest_id != 0
                            && self
                                .represented_player_quest_status_like_cpp(source.chest_quest_id)
                                .is_some_and(|status| {
                                    status == Some(wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP)
                                })
                    }) || state.chest_loot_source.is_some_and(|source| {
                        self.represented_gameobject_loot_ids_have_quest_loot_for_player_like_cpp(
                            source.loot_ids_like_cpp(),
                        )
                    }))
            }
            Some(wow_entities::GAMEOBJECT_TYPE_GENERIC) => {
                self.represented_has_quest_for_gameobject_like_cpp(gameobject_entry)
            }
            Some(wow_entities::GAMEOBJECT_TYPE_GOOBER) => {
                state
                    .goober_use_source
                    .is_some_and(|source| source.quest_id != 0)
                    && state.goober_use_source.is_some_and(|source| {
                        self.represented_player_quest_status_like_cpp(source.quest_id)
                            .is_some_and(|status| {
                                status == Some(wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP)
                            })
                    })
            }
            Some(wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE) => self
                .represented_gameobject_loot_ids_have_quest_loot_for_player_like_cpp(
                    state.gathering_node_loot_id,
                ),
            _ => false,
        }
    }
}
