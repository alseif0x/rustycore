// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest-completion visibility refreshes using selected Core and entity access.

mod gameobject_flags;
mod dialog_status;
mod quest_eligibility;

pub use self::quest_eligibility::QuestEligibilityCx;

pub(crate) use self::gameobject_flags::represented_gameobject_go_state_for_viewer_like_cpp;
pub use self::gameobject_flags::{
    represented_gameobject_activate_to_quest_like_cpp,
    represented_meets_player_condition_id_like_cpp,
};

use std::sync::Arc;

use wow_world_core::session::{
    NpcInteractionAccessLikeCpp, PlayerConditionAccessLikeCpp, QuestObjectiveAccessLikeCpp,
    RepresentedGetReactionInputLikeCpp, SessionCatalogs,
};
use crate::PlayerConditionProjectionCxLikeCpp;
use wow_world_entities::RepresentedGameObjectUseState;
use wow_world_social::SessionSocialLimits;
use wow_world_entities::WorldEntitiesState;
use wow_core::ObjectGuid;
use wow_data::{
    AreaTableStore, ChrSpecializationStore, ConditionEntriesByTypeStore,
    NpcSpellClickStoreLikeCpp, PlayerConditionStore,
};

use super::{SessionQuestState, objective_progress::current_quest_gameplay_snapshot_like_cpp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedCanSeeSpellClickOutcomeLikeCpp {
    Visible,
    Hidden,
    ExactContextUnrepresented,
}

impl PlayerConditionProjectionCxLikeCpp<'_> {
    /// Evaluate one quest's ungrouped availability conditions at the selected
    /// condition-projection phase.
    pub fn represented_quest_available_conditions_meet_like_cpp(
        &self,
        owner: &QuestObjectiveAccessLikeCpp<'_>,
        quest_state: &SessionQuestState,
        catalogs: &SessionCatalogs,
        quest_id: u32,
        consumer_test: bool,
    ) -> bool {
        let condition_store = if let Some(store) = catalogs.condition_store() {
            Arc::clone(store)
        } else if let Some(store) = wow_conditions::condition_mgr_store_like_cpp() {
            store
        } else {
            return true;
        };

        if !wow_conditions::has_conditions_for_not_grouped_entry_like_cpp(
            condition_store.as_ref(),
            wow_constants::ConditionSourceType::QuestAvailable,
            quest_id,
        ) {
            return true;
        }

        let Some(player_object) = self.player.build_condition_player_object_like_cpp() else {
            return false;
        };
        let Some(recurrence) = current_quest_gameplay_snapshot_like_cpp(
            owner,
            quest_state,
            consumer_test,
        ) else {
            return false;
        };

        let quest_statuses: Vec<_> = recurrence
            .statuses_like_cpp()
            .iter()
            .map(|(&quest_id, status)| wow_conditions::ConditionQuestStatusSnapshot {
                quest_id,
                status: status.status,
            })
            .collect();
        let quest_store = catalogs.quests.store.as_ref();
        let quest_objective_progress: Vec<_> = quest_store
            .map(|store| {
                recurrence
                    .statuses_like_cpp()
                    .iter()
                    .filter_map(|(&quest_id, status)| {
                        store.get(quest_id).map(|quest| {
                            quest.objectives.iter().filter_map(move |objective| {
                                let storage_index =
                                    usize::try_from(objective.storage_index).ok()?;
                                let counter = status
                                    .objective_counts
                                    .get(storage_index)
                                    .copied()
                                    .unwrap_or(0);
                                Some(wow_conditions::ConditionQuestObjectiveProgressSnapshot {
                                    quest_id,
                                    objective_id: objective.id,
                                    counter,
                                })
                            })
                        })
                    })
                    .flatten()
                    .collect()
            })
            .unwrap_or_default();
        let rewarded_quest_ids: Vec<_> = recurrence
            .rewarded_quest_ids_like_cpp()
            .iter()
            .copied()
            .collect();
        let daily_quest_ids: Vec<_> = recurrence
            .daily_quest_ids_like_cpp()
            .iter()
            .copied()
            .collect();
        let quest_snapshot = wow_conditions::ConditionPlayerQuestSnapshot {
            statuses: &quest_statuses,
            objective_progress: &quest_objective_progress,
            rewarded_quest_ids: &rewarded_quest_ids,
            daily_quest_ids: &daily_quest_ids,
        };

        let Some(player_condition_context) = self.project_like_cpp() else {
            return false;
        };
        let area_table_store = self
            .valuation_catalogs
            .area_table_store_like_cpp()
            .cloned();

        let mut source_info =
            wow_conditions::ConditionSourceInfo::from_targets(Some(&player_object), None, None);
        let Some(player_unit_snapshot) = self.player.condition_player_unit_snapshot_like_cpp()
        else {
            return false;
        };
        source_info.set_unit_target_snapshot(0, player_unit_snapshot);
        source_info.set_player_target_snapshot(0, self.player.condition_player_snapshot_like_cpp());
        source_info.set_player_quest_target_snapshot(0, quest_snapshot);
        if let Some(store) = self.valuation_catalogs.player_condition_store_like_cpp() {
            source_info.set_player_condition_store(store.as_ref());
            if let Some(context) = self.condition_context_like_cpp(&player_condition_context) {
                source_info.set_player_condition_context(0, context);
            }
        }

        wow_conditions::is_object_meeting_not_grouped_conditions_like_cpp(
            condition_store.as_ref(),
            wow_constants::ConditionSourceType::QuestAvailable,
            quest_id,
            &mut source_info,
            |condition, source_info| {
                wow_conditions::condition_meets_basic_like_cpp(
                    condition,
                    source_info,
                    |area_id, required_area_id| {
                        area_table_store.as_ref().is_some_and(|store| {
                            store.is_in_area_like_cpp(area_id, required_area_id)
                        })
                    },
                )
                .value()
                .unwrap_or(false)
            },
        )
    }
}

/// Preserve the existing publication rule: only a represented Hidden result
/// clears the spell-click bit; unavailable exact context leaves it untouched.
pub fn represented_viewer_dependent_creature_npc_flags_like_cpp(
    npc_flags: u64,
    outcome: RepresentedCanSeeSpellClickOutcomeLikeCpp,
) -> u64 {
    let spell_click_flag = wow_data::spell_click::UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP;
    if (npc_flags & spell_click_flag) == 0 {
        return npc_flags;
    }
    match outcome {
        RepresentedCanSeeSpellClickOutcomeLikeCpp::Hidden => npc_flags & !spell_click_flag,
        RepresentedCanSeeSpellClickOutcomeLikeCpp::Visible
        | RepresentedCanSeeSpellClickOutcomeLikeCpp::ExactContextUnrepresented => npc_flags,
    }
}

/// Evaluate one viewer's spell-click visibility with the original selected
/// player, NPC-reaction, and condition providers.
#[allow(clippy::too_many_arguments)]
pub fn represented_can_see_spell_click_on_like_cpp(
    world_entities: &WorldEntitiesState,
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    npc_access: &NpcInteractionAccessLikeCpp<'_>,
    player_access: &PlayerConditionAccessLikeCpp<'_>,
    condition_projection: &PlayerConditionProjectionCxLikeCpp<'_>,
    social: &SessionSocialLimits,
    chr_specialization_store: Option<&ChrSpecializationStore>,
    creature_guid: ObjectGuid,
    spell_click_store: Option<&NpcSpellClickStoreLikeCpp>,
    condition_store: Option<&ConditionEntriesByTypeStore>,
    player_condition_store: Option<&Arc<PlayerConditionStore>>,
    area_table_store: Option<&Arc<AreaTableStore>>,
    consumer_test: bool,
) -> RepresentedCanSeeSpellClickOutcomeLikeCpp {
    use RepresentedCanSeeSpellClickOutcomeLikeCpp as Outcome;

    let Some(spell_click_store) = spell_click_store else {
        return Outcome::ExactContextUnrepresented;
    };
    let Some(condition_store) = condition_store else {
        return Outcome::ExactContextUnrepresented;
    };
    let Some(creature) = world_entities
        .represented_spell_click_creature_snapshot_like_cpp(owner, creature_guid)
    else {
        return Outcome::ExactContextUnrepresented;
    };
    if !creature.is_in_world {
        return Outcome::Hidden;
    }

    if (u64::from(creature.npc_flags)
        & wow_data::spell_click::UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP)
        == 0
    {
        return Outcome::Hidden;
    }

    let click_bounds = spell_click_store.spell_click_info_map_bounds_like_cpp(creature.entry);
    if click_bounds.is_empty() {
        return Outcome::Hidden;
    }

    let Some(clicker_object) = player_access.build_condition_player_object_like_cpp() else {
        return Outcome::ExactContextUnrepresented;
    };
    let mut target_object = wow_entities::WorldObject::new(
        false,
        wow_constants::TypeId::Unit,
        wow_constants::TypeMask::OBJECT | wow_constants::TypeMask::UNIT,
    );
    target_object.object_mut().create(creature.guid);
    target_object.object_mut().set_entry(creature.entry);
    let _ = target_object.set_map(creature.map_id, creature.instance_id);
    target_object.relocate(creature.position);
    *target_object.phase_shift_mut() = creature.phase_shift.clone();

    let Some(player_unit_snapshot) = player_access.condition_player_unit_snapshot_like_cpp() else {
        return Outcome::ExactContextUnrepresented;
    };
    let player_snapshot = player_access.condition_player_snapshot_like_cpp();
    let creature_unit_snapshot = wow_conditions::ConditionUnitSnapshot {
        level: creature.level,
        health: creature.health,
        max_health: creature.max_health,
        class_mask: 0,
        race: 0,
        creature_type: None,
        is_alive: creature.is_alive,
        is_charmed: false,
        in_water: false,
        unit_state: 0,
        stand_state: wow_constants::UnitStandStateType::Stand as u32,
    };
    let player_condition_store = player_condition_store.cloned();
    let Some(player_conditions) = condition_projection.project_like_cpp() else {
        return Outcome::ExactContextUnrepresented;
    };
    let area_table_store = area_table_store.cloned();

    for click_info in click_bounds {
        match click_info.user_type {
            wow_data::SPELL_CLICK_USER_FRIEND_LIKE_CPP => {
                let player_faction_template =
                    npc_access.player_faction_template_id_like_cpp();
                if creature.is_summon
                    || !npc_access.has_faction_template_store_like_cpp()
                    || player_faction_template.is_none()
                {
                    return Outcome::ExactContextUnrepresented;
                }
                let reaction = npc_access.represented_get_reaction_to_like_cpp(
                    RepresentedGetReactionInputLikeCpp {
                        self_faction_template_id: player_faction_template.unwrap_or(0),
                        target_faction_template_id: creature.faction_template_id,
                        same_object: false,
                        attackable_by_summoner: false,
                        same_charmer_or_owner_or_self: false,
                        self_has_player_owner: true,
                        target_has_player_owner: false,
                        target_player_owner_is_current_session: false,
                        target_owner_forced_rank_for_self: None,
                        same_player_owner: false,
                        duel_in_progress: false,
                        same_raid: false,
                        self_unit_player_controlled: true,
                        target_unit_player_controlled: false,
                        self_ffa_pvp: false,
                        target_ffa_pvp: false,
                        self_ignores_reputation: false,
                        target_ignores_reputation: false,
                        target_is_unit: true,
                        target_player_contested_pvp: false,
                    },
                );
                if reaction < wow_data::reputation::ReputationRankLikeCpp::Friendly {
                    return Outcome::Hidden;
                }
            }
            wow_data::SPELL_CLICK_USER_PARTY_LIKE_CPP
            | wow_data::SPELL_CLICK_USER_RAID_LIKE_CPP => {
                return Outcome::ExactContextUnrepresented;
            }
            _ => {}
        }

        if wow_conditions::is_object_meeting_spell_click_conditions_like_cpp(
            condition_store,
            creature.entry,
            click_info.spell_id,
            Some(&clicker_object),
            Some(&target_object),
            |condition, source_info| {
                source_info.set_unit_target_snapshot(0, player_unit_snapshot);
                source_info.set_player_target_snapshot(0, player_snapshot);
                source_info.set_unit_target_snapshot(1, creature_unit_snapshot);
                if let Some(store) = player_condition_store.as_ref() {
                    source_info.set_player_condition_store(store.as_ref());
                    if let Some(context) = player_conditions.as_context(
                        player_access,
                        social,
                        chr_specialization_store,
                        consumer_test,
                    ) {
                        source_info.set_player_condition_context(0, context);
                    }
                }
                wow_conditions::condition_meets_basic_like_cpp(
                    condition,
                    source_info,
                    |area_id, required_area_id| {
                        area_table_store.as_ref().is_some_and(|store| {
                            store.is_in_area_like_cpp(area_id, required_area_id)
                        })
                    },
                )
                .value()
                .unwrap_or(false)
            },
        ) {
            return Outcome::Visible;
        }
    }

    Outcome::Hidden
}

pub(crate) fn represented_has_quest_for_gameobject_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    catalogs: &SessionCatalogs,
    quest_state: &SessionQuestState,
    gameobject_entry: u32,
    fixture_fallback: bool,
) -> bool {
    let Some(store) = catalogs.quests.store.as_ref() else {
        return false;
    };
    let Some(quests) =
        current_quest_gameplay_snapshot_like_cpp(owner, quest_state, fixture_fallback)
    else {
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

pub(crate) fn represented_gameobject_is_for_quests_like_cpp(
    catalogs: &SessionCatalogs,
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    quest_state: &SessionQuestState,
    gameobject_entry: u32,
    state: &RepresentedGameObjectUseState,
    fixture_fallback: bool,
) -> bool {
    match state.go_type.map(u32::from) {
        Some(wow_entities::GAMEOBJECT_TYPE_QUESTGIVER) => true,
        Some(wow_entities::GAMEOBJECT_TYPE_CHEST) => {
            represented_has_quest_for_gameobject_like_cpp(
                owner,
                catalogs,
                quest_state,
                gameobject_entry,
                fixture_fallback,
            ) || state
                .chest_loot_source
                .is_some_and(|source| source.chest_quest_id != 0)
                || state.chest_loot_source.is_some_and(|source| {
                    catalogs.represented_gameobject_loot_ids_have_quest_loot_like_cpp(
                        source.loot_ids_like_cpp(),
                    )
                })
        }
        Some(wow_entities::GAMEOBJECT_TYPE_GENERIC) => {
            represented_has_quest_for_gameobject_like_cpp(
                owner,
                catalogs,
                quest_state,
                gameobject_entry,
                fixture_fallback,
            )
        }
        Some(wow_entities::GAMEOBJECT_TYPE_GOOBER) => state
            .goober_use_source
            .is_some_and(|source| source.quest_id != 0),
        Some(wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE) => catalogs
            .represented_gameobject_loot_ids_have_quest_loot_like_cpp(
                state.gathering_node_loot_id,
            ),
        _ => false,
    }
}
