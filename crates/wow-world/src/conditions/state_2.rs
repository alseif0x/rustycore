//! Condition snapshot borrows state definitions, part 2 of 3.
//!
//! Separated from the conditions.rs root under #656. Behaviour is preserved.

use super::*;

/// Ported subset of C++ `Condition::Meets`.
///
/// Returns `Unsupported` for condition types whose exact C++ state is not yet represented by
/// the Rust entity/DB2/runtime layers.
pub fn condition_meets_basic_like_cpp<'a>(
    condition: &'a Condition,
    source_info: &mut ConditionSourceInfo<'a>,
    mut is_in_area_like_cpp: impl FnMut(u32, u32) -> bool,
) -> ConditionMeetResult {
    if usize::from(condition.condition_target) >= MAX_CONDITION_TARGETS {
        return ConditionMeetResult::Evaluated(false);
    }

    let mut cond_meets = false;
    let mut needs_object = false;

    match condition.condition_type {
        ConditionType::None => cond_meets = true,
        ConditionType::MapId => {
            cond_meets = source_info
                .condition_map
                .is_some_and(|map| map.map_id == condition.condition_value1);
        }
        ConditionType::RealmAchievement => {
            cond_meets = source_info
                .realm_achievement_ids
                .contains(&condition.condition_value1);
        }
        ConditionType::ActiveEvent => {
            cond_meets = source_info.map_state.is_some_and(|map_state| {
                map_state
                    .active_event_ids
                    .contains(&condition.condition_value1)
            });
        }
        ConditionType::WorldState => {
            cond_meets = source_info.map_state.is_some_and(|map_state| {
                map_state.world_state_value_like_cpp(condition.condition_value1)
                    == condition.condition_value2 as i32
            });
        }
        ConditionType::DifficultyId => {
            cond_meets = source_info
                .map_state
                .is_some_and(|map_state| map_state.difficulty_id == condition.condition_value1);
        }
        ConditionType::InstanceInfo => {
            let Some(instance_info) = ConditionInstanceInfo::from_u32(condition.condition_value3)
            else {
                return ConditionMeetResult::Evaluated(false);
            };
            cond_meets = source_info.map_state.is_some_and(|map_state| {
                map_state
                    .instance_data_value_like_cpp(instance_info, condition.condition_value1)
                    .is_some_and(|value| value == u64::from(condition.condition_value2))
            });
        }
        ConditionType::ScenarioStep => {
            cond_meets = source_info.map_state.is_some_and(|map_state| {
                map_state.scenario_step_id == Some(condition.condition_value1)
            });
        }
        _ => needs_object = true,
    }

    let target_index = usize::from(condition.condition_target);
    let object = source_info.condition_targets[target_index];
    if needs_object && object.is_none() {
        return ConditionMeetResult::Evaluated(false);
    }

    if let Some(object) = object {
        let unit = source_info.unit_targets[target_index];
        let unit_auras =
            source_info.unit_aura_targets[target_index].filter(|_| is_unit_object_like_cpp(object));
        let unit_relations = source_info.unit_relation_targets[target_index]
            .filter(|_| is_unit_object_like_cpp(object));
        let is_player = is_player_object_like_cpp(object);
        let player = source_info.player_targets[target_index].filter(|_| is_player);
        let player_quests = source_info.player_quest_targets[target_index].filter(|_| is_player);
        let player_progression =
            source_info.player_progression_targets[target_index].filter(|_| is_player);
        let player_condition_context =
            source_info.player_condition_contexts[target_index].filter(|_| is_player);
        match condition.condition_type {
            ConditionType::Aura => {
                if let Some(aura_effects) = unit_auras {
                    cond_meets = aura_effects.iter().any(|aura| {
                        aura.spell_id == condition.condition_value1
                            && aura.effect_index == condition.condition_value2
                    });
                }
            }
            ConditionType::ZoneId => cond_meets = object.zone_id() == condition.condition_value1,
            ConditionType::AreaId => {
                cond_meets = is_in_area_like_cpp(object.area_id(), condition.condition_value1);
            }
            ConditionType::Class => {
                if let Some(unit) = unit {
                    cond_meets = (unit.class_mask & condition.condition_value1) != 0;
                }
            }
            ConditionType::Team => {
                if let Some(player) = player {
                    cond_meets = player.team == condition.condition_value1;
                }
            }
            ConditionType::Race => {
                if let Some(unit) = unit {
                    cond_meets = unit.race != 0
                        && condition.condition_value1 & (1_u32 << u32::from(unit.race - 1)) != 0;
                }
            }
            ConditionType::Gender => {
                if let Some(player) = player {
                    cond_meets = player.native_gender == condition.condition_value1;
                }
            }
            ConditionType::Item => {
                if let Some(progression) = player_progression {
                    cond_meets = progression.has_item_count_like_cpp(
                        condition.condition_value1,
                        condition.condition_value2,
                        condition.condition_value3 != 0,
                    );
                }
            }
            ConditionType::ItemEquipped => {
                if let Some(progression) = player_progression {
                    cond_meets =
                        progression.has_item_or_gem_equipped_like_cpp(condition.condition_value1);
                }
            }
            ConditionType::Achievement => {
                if let Some(progression) = player_progression {
                    cond_meets = progression.has_achievement_like_cpp(condition.condition_value1);
                }
            }
            ConditionType::ReputationRank => {
                if let Some(progression) = player_progression {
                    cond_meets = progression.has_reputation_rank_like_cpp(
                        condition.condition_value1,
                        condition.condition_value2,
                    );
                }
            }
            ConditionType::Skill => {
                if let Some(progression) = player_progression {
                    cond_meets = progression.has_skill_base_value_like_cpp(
                        condition.condition_value1,
                        condition.condition_value2,
                    );
                }
            }
            ConditionType::QuestRewarded => {
                if let Some(quests) = player_quests {
                    cond_meets = quests.is_quest_rewarded_like_cpp(condition.condition_value1);
                }
            }
            ConditionType::QuestTaken => {
                if let Some(quests) = player_quests {
                    cond_meets = quests.quest_status_like_cpp(condition.condition_value1)
                        == QUEST_STATUS_INCOMPLETE_LIKE_CPP;
                }
            }
            ConditionType::QuestComplete => {
                if let Some(quests) = player_quests {
                    cond_meets = quests.quest_status_like_cpp(condition.condition_value1)
                        == QUEST_STATUS_COMPLETE_LIKE_CPP
                        && !quests.is_quest_rewarded_like_cpp(condition.condition_value1);
                }
            }
            ConditionType::QuestNone => {
                if let Some(quests) = player_quests {
                    cond_meets = quests.quest_status_like_cpp(condition.condition_value1)
                        == QUEST_STATUS_NONE_LIKE_CPP;
                }
            }
            ConditionType::Spell => {
                if let Some(progression) = player_progression {
                    cond_meets = progression.has_spell_like_cpp(condition.condition_value1);
                }
            }
            ConditionType::Level => {
                if let Some(unit) = unit {
                    cond_meets = compare_values_u64_like_cpp(
                        condition.condition_value2,
                        u64::from(unit.level),
                        u64::from(condition.condition_value1),
                    );
                }
            }
            ConditionType::ObjectEntryGuid | ConditionType::ObjectEntryGuidLegacy => {
                let type_id = object.object().type_id();
                if type_id as u32 == condition.condition_value1 {
                    cond_meets = condition.condition_value2 == 0
                        || object.object().entry() == condition.condition_value2;

                    if condition.condition_value3 != 0
                        && matches!(type_id, TypeId::Unit | TypeId::GameObject)
                    {
                        cond_meets &=
                            source_info.spawn_id_targets[target_index].is_some_and(|spawn_id| {
                                spawn_id == u64::from(condition.condition_value3)
                            });
                    }
                }
            }
            ConditionType::TypeMask | ConditionType::TypeMaskLegacy => {
                cond_meets = object
                    .object()
                    .is_type(TypeMask::from_bits_truncate(condition.condition_value1));
            }
            ConditionType::DistanceTo => {
                if let Some(to_object) = usize::try_from(condition.condition_value1)
                    .ok()
                    .filter(|target| *target < MAX_CONDITION_TARGETS)
                    .and_then(|target| source_info.condition_targets[target])
                {
                    cond_meets = compare_values_f32_like_cpp(
                        condition.condition_value3,
                        object.distance(to_object),
                        condition.condition_value2 as f32,
                    );
                }
            }
            ConditionType::NearCreature => {
                if let Some(creatures) = source_info.nearby_creature_targets[target_index] {
                    let alive_required = condition.condition_value3 == 0;
                    cond_meets = creatures.iter().any(|creature| {
                        creature.entry == condition.condition_value1
                            && creature.distance <= condition.condition_value2 as f32
                            && (!alive_required || creature.is_alive)
                    });
                }
            }
            ConditionType::NearGameObject => {
                if let Some(gameobjects) = source_info.nearby_gameobject_targets[target_index] {
                    cond_meets = gameobjects.iter().any(|gameobject| {
                        gameobject.entry == condition.condition_value1
                            && gameobject.distance <= condition.condition_value2 as f32
                    });
                }
            }
            ConditionType::RelationTo => {
                let Some(relation_type) = RelationType::from_u32(condition.condition_value2) else {
                    return ConditionMeetResult::Unsupported;
                };

                if let Some(to_target_index) = usize::try_from(condition.condition_value1)
                    .ok()
                    .filter(|target| *target < MAX_CONDITION_TARGETS)
                {
                    if let Some(to_object) = source_info.condition_targets[to_target_index]
                        && is_unit_object_like_cpp(object)
                        && is_unit_object_like_cpp(to_object)
                    {
                        cond_meets = match relation_type {
                            RelationType::SelfRelation => std::ptr::eq(object, to_object),
                            RelationType::InParty => unit_relations
                                .and_then(|relations| {
                                    relations.iter().find(|relation| {
                                        relation.to_target_index == to_target_index
                                    })
                                })
                                .is_some_and(|relation| relation.in_party),
                            RelationType::InRaidOrParty => unit_relations
                                .and_then(|relations| {
                                    relations.iter().find(|relation| {
                                        relation.to_target_index == to_target_index
                                    })
                                })
                                .is_some_and(|relation| relation.in_raid_or_party),
                            RelationType::OwnedBy => unit_relations
                                .and_then(|relations| {
                                    relations.iter().find(|relation| {
                                        relation.to_target_index == to_target_index
                                    })
                                })
                                .is_some_and(|relation| relation.owned_by),
                            RelationType::PassengerOf => unit_relations
                                .and_then(|relations| {
                                    relations.iter().find(|relation| {
                                        relation.to_target_index == to_target_index
                                    })
                                })
                                .is_some_and(|relation| relation.passenger_of),
                            RelationType::CreatedBy => unit_relations
                                .and_then(|relations| {
                                    relations.iter().find(|relation| {
                                        relation.to_target_index == to_target_index
                                    })
                                })
                                .is_some_and(|relation| relation.created_by),
                            RelationType::Max => false,
                        };
                    }
                }
            }
            ConditionType::ReactionTo => {
                if let Some(to_target_index) = usize::try_from(condition.condition_value1)
                    .ok()
                    .filter(|target| *target < MAX_CONDITION_TARGETS)
                {
                    if let Some(to_object) = source_info.condition_targets[to_target_index]
                        && is_unit_object_like_cpp(object)
                        && is_unit_object_like_cpp(to_object)
                        && let Some(relation) = unit_relations.and_then(|relations| {
                            relations
                                .iter()
                                .find(|relation| relation.to_target_index == to_target_index)
                        })
                    {
                        cond_meets =
                            ((1_u32 << relation.reaction) & condition.condition_value2) != 0;
                    }
                }
            }
            ConditionType::PhaseId => {
                cond_meets = object
                    .phase_shift()
                    .has_phase_like_cpp(condition.condition_value1);
            }
            ConditionType::TerrainSwap => {
                cond_meets = object
                    .phase_shift()
                    .has_visible_map_id_like_cpp(condition.condition_value1);
            }
            ConditionType::Alive => {
                if let Some(unit) = unit {
                    cond_meets = unit.is_alive;
                }
            }
            ConditionType::HpVal => {
                if let Some(unit) = unit {
                    cond_meets = compare_values_u64_like_cpp(
                        condition.condition_value2,
                        unit.health,
                        u64::from(condition.condition_value1),
                    );
                }
            }
            ConditionType::HpPct => {
                if let Some(unit) = unit {
                    let health_pct = if unit.max_health == 0 {
                        0.0
                    } else {
                        (unit.health as f32 / unit.max_health as f32) * 100.0
                    };
                    cond_meets = compare_values_f32_like_cpp(
                        condition.condition_value2,
                        health_pct,
                        condition.condition_value1 as f32,
                    );
                }
            }
            ConditionType::UnitState => {
                if let Some(unit) = unit {
                    cond_meets = (unit.unit_state & condition.condition_value1) != 0;
                }
            }
            ConditionType::InWater => {
                if let Some(unit) = unit {
                    cond_meets = unit.in_water;
                }
            }
            ConditionType::CreatureType => {
                if let Some(unit) = unit
                    && object.object().type_id() == TypeId::Unit
                    && let Some(creature_type) = unit.creature_type
                {
                    cond_meets = creature_type == condition.condition_value1;
                }
            }
            ConditionType::StandState => {
                if let Some(unit) = unit {
                    cond_meets = if condition.condition_value1 == 0 {
                        unit.stand_state == condition.condition_value2
                    } else if condition.condition_value2 == 0 {
                        unit_stand_state_is_stand_like_cpp(unit.stand_state)
                    } else if condition.condition_value2 == 1 {
                        unit_stand_state_is_sit_like_cpp(unit.stand_state)
                    } else {
                        false
                    };
                }
            }
            ConditionType::DrunkenState => {
                if let Some(player) = player {
                    cond_meets = player.drunken_state >= condition.condition_value1;
                }
            }
            ConditionType::GameMaster => {
                if let Some(player) = player {
                    cond_meets = if condition.condition_value1 == 1 {
                        player.can_be_game_master
                    } else {
                        player.is_game_master
                    };
                }
            }
            ConditionType::PetType => {
                if let Some(player) = player
                    && let Some(pet_type) = player.pet_type
                {
                    cond_meets = ((1_u32 << pet_type) & condition.condition_value1) != 0;
                }
            }
            ConditionType::Taxi => {
                if let Some(player) = player {
                    cond_meets = player.is_in_flight;
                }
            }
            ConditionType::Title => {
                if let Some(progression) = player_progression {
                    cond_meets = progression.has_title_like_cpp(condition.condition_value1);
                }
            }
            ConditionType::BattlePetCount => {
                if let Some(progression) = player_progression {
                    cond_meets = compare_values_u64_like_cpp(
                        condition.condition_value3,
                        u64::from(
                            progression.battle_pet_count_like_cpp(condition.condition_value1),
                        ),
                        u64::from(condition.condition_value2),
                    );
                }
            }
            ConditionType::SceneInProgress => {
                if let Some(progression) = player_progression {
                    cond_meets = progression.has_active_scene_like_cpp(condition.condition_value1);
                }
            }
            ConditionType::PlayerCondition => {
                let Some(store) = source_info.player_condition_store else {
                    return ConditionMeetResult::Unsupported;
                };
                let Some(context) = player_condition_context else {
                    return ConditionMeetResult::Unsupported;
                };
                if let Some(player_condition) = store.get(condition.condition_value1) {
                    cond_meets = is_player_meeting_condition_like_cpp(player_condition, &context);
                }
            }
            ConditionType::DailyQuestDone => {
                if let Some(quests) = player_quests {
                    cond_meets = quests.is_daily_quest_done_like_cpp(condition.condition_value1);
                }
            }
            ConditionType::QuestState => {
                if let Some(quests) = player_quests {
                    let quest_status = quests.quest_status_like_cpp(condition.condition_value1);
                    cond_meets = ((condition.condition_value2 & (1 << QUEST_STATUS_NONE_LIKE_CPP))
                        != 0
                        && quest_status == QUEST_STATUS_NONE_LIKE_CPP)
                        || ((condition.condition_value2 & (1 << QUEST_STATUS_COMPLETE_LIKE_CPP))
                            != 0
                            && quest_status == QUEST_STATUS_COMPLETE_LIKE_CPP)
                        || ((condition.condition_value2 & (1 << QUEST_STATUS_INCOMPLETE_LIKE_CPP))
                            != 0
                            && quest_status == QUEST_STATUS_INCOMPLETE_LIKE_CPP)
                        || ((condition.condition_value2 & (1 << QUEST_STATUS_FAILED_LIKE_CPP))
                            != 0
                            && quest_status == QUEST_STATUS_FAILED_LIKE_CPP)
                        || ((condition.condition_value2 & (1 << QUEST_STATUS_REWARDED_LIKE_CPP))
                            != 0
                            && quests.is_quest_rewarded_like_cpp(condition.condition_value1));
                }
            }
            ConditionType::QuestObjectiveProgress => {
                if let Some(quests) = player_quests
                    && let Some(progress) = quests.objective_progress.iter().find(|progress| {
                        progress.objective_id == condition.condition_value1
                            && quests.quest_is_in_log_like_cpp(progress.quest_id)
                    })
                {
                    cond_meets = progress.counter == condition.condition_value3 as i32;
                }
            }
            ConditionType::Charmed => {
                if let Some(unit) = unit {
                    cond_meets = unit.is_charmed;
                }
            }
            ConditionType::PrivateObject => {
                cond_meets = source_info.private_object_targets[target_index];
            }
            ConditionType::StringId => {
                if matches!(object.object().type_id(), TypeId::Unit | TypeId::GameObject)
                    && let Some(string_ids) = source_info.string_id_targets[target_index]
                {
                    cond_meets = string_ids
                        .iter()
                        .any(|string_id| *string_id == condition.condition_string_value1.as_str());
                }
            }
            ConditionType::None | ConditionType::MapId | ConditionType::RealmAchievement => {}
            ConditionType::ActiveEvent
            | ConditionType::InstanceInfo
            | ConditionType::WorldState
            | ConditionType::DifficultyId
            | ConditionType::ScenarioStep => {}
            _ => return ConditionMeetResult::Unsupported,
        }
    }

    if condition.negative_condition {
        cond_meets = !cond_meets;
    }

    if !cond_meets {
        source_info.mark_failed_like_cpp(condition);
    }

    ConditionMeetResult::Evaluated(cond_meets)
}

/// C++ `ConditionMgr::IsObjectMeetToConditions`.
pub fn is_object_meet_to_conditions_like_cpp<'a>(
    source_info: &mut ConditionSourceInfo<'a>,
    conditions: &'a [Condition],
    condition_store: &'a ConditionEntriesByTypeStore,
    mut meets: impl FnMut(&'a Condition, &mut ConditionSourceInfo<'a>) -> bool,
) -> bool {
    if conditions.is_empty() {
        return true;
    }

    is_object_meet_to_condition_list_like_cpp(source_info, conditions, condition_store, &mut meets)
}

pub(super) fn is_object_meet_to_condition_list_like_cpp<'a, F>(
    source_info: &mut ConditionSourceInfo<'a>,
    conditions: &'a [Condition],
    condition_store: &'a ConditionEntriesByTypeStore,
    meets: &mut F,
) -> bool
where
    F: FnMut(&'a Condition, &mut ConditionSourceInfo<'a>) -> bool,
{
    let mut else_group_store = std::collections::BTreeMap::<u32, bool>::new();

    for condition in conditions {
        if !condition.is_loaded_like_cpp() {
            continue;
        }

        let group_passed = else_group_store.entry(condition.else_group).or_insert(true);
        if !*group_passed {
            continue;
        }

        if condition.reference_id != 0 {
            if let Some(reference_conditions) = condition_store.conditions_for_like_cpp(
                ConditionSourceType::ReferenceCondition,
                ConditionId::new(condition.reference_id, 0, 0),
            ) && !is_object_meet_to_condition_list_like_cpp(
                source_info,
                reference_conditions.as_slice(),
                condition_store,
                meets,
            ) {
                *group_passed = false;
            }
        } else if !meets(condition, source_info) {
            *group_passed = false;
        }
    }

    else_group_store.values().any(|passed| *passed)
}

/// C++ `ConditionMgr::IsObjectMeetingNotGroupedConditions`.
pub fn is_object_meeting_not_grouped_conditions_like_cpp<'a>(
    condition_store: &'a ConditionEntriesByTypeStore,
    source_type: ConditionSourceType,
    entry: u32,
    source_info: &mut ConditionSourceInfo<'a>,
    meets: impl FnMut(&'a Condition, &mut ConditionSourceInfo<'a>) -> bool,
) -> bool {
    if (source_type as u32) > ConditionSourceType::None as u32
        && (source_type as u32) < ConditionSourceType::Max as u32
    {
        if let Some(conditions) = condition_store
            .conditions_for_like_cpp(source_type, ConditionId::new(0, entry as i32, 0))
        {
            return is_object_meet_to_conditions_like_cpp(
                source_info,
                conditions.as_slice(),
                condition_store,
                meets,
            );
        }
    }

    true
}

/// C++ `ConditionMgr::IsMapMeetingNotGroupedConditions` for spawn-group map conditions.
///
/// This is a map-only evaluation helper for `CONDITION_SOURCE_TYPE_SPAWN_GROUP` buckets keyed as
/// `{ source_group = 0, source_entry = spawn_group_id, source_id = 0 }`. It deliberately does not
/// mutate `wow_map::Map` or execute `SpawnGroupSpawn`/`SpawnGroupDespawn`; future live callers must
/// pass the returned bool into the map-side planner/executor.
pub fn is_spawn_group_meeting_map_conditions_like_cpp<'a>(
    condition_store: &'a ConditionEntriesByTypeStore,
    spawn_group_id: u32,
    condition_map: ConditionMapRef,
    map_state: Option<ConditionMapStateSnapshot<'a>>,
    realm_achievement_ids: &'a [u32],
) -> bool {
    let mut source_info = ConditionSourceInfo::from_map(condition_map);
    if let Some(map_state) = map_state {
        source_info.set_map_state_snapshot(map_state);
    }
    source_info.set_realm_achievement_ids(realm_achievement_ids);

    is_object_meeting_not_grouped_conditions_like_cpp(
        condition_store,
        ConditionSourceType::SpawnGroup,
        spawn_group_id,
        &mut source_info,
        |condition, source_info| {
            condition_meets_basic_like_cpp(condition, source_info, |_area, _zone| false)
                .value()
                .unwrap_or(false)
        },
    )
}

/// C++ `ConditionMgr::HasConditionsForNotGroupedEntry`.
pub fn has_conditions_for_not_grouped_entry_like_cpp(
    condition_store: &ConditionEntriesByTypeStore,
    source_type: ConditionSourceType,
    entry: u32,
) -> bool {
    (source_type as u32) > ConditionSourceType::None as u32
        && (source_type as u32) < ConditionSourceType::Max as u32
        && condition_store
            .conditions_for_like_cpp(source_type, ConditionId::new(0, entry as i32, 0))
            .is_some()
}

/// C++ `ConditionMgr::IsObjectMeetingSpellClickConditions`.
pub fn is_object_meeting_spell_click_conditions_like_cpp<'a>(
    condition_store: &'a ConditionEntriesByTypeStore,
    creature_id: u32,
    spell_id: u32,
    clicker: Option<&'a WorldObject>,
    target: Option<&'a WorldObject>,
    meets: impl FnMut(&'a Condition, &mut ConditionSourceInfo<'a>) -> bool,
) -> bool {
    if let Some(conditions) = condition_store.conditions_for_like_cpp(
        ConditionSourceType::SpellClickEvent,
        ConditionId::new(creature_id, spell_id as i32, 0),
    ) {
        let mut source_info = ConditionSourceInfo::from_targets(clicker, target, None);
        return is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions.as_slice(),
            condition_store,
            meets,
        );
    }

    true
}

/// C++ `ConditionMgr::HasConditionsForSpellClickEvent`.
pub fn has_conditions_for_spell_click_event_like_cpp(
    condition_store: &ConditionEntriesByTypeStore,
    creature_id: u32,
    spell_id: u32,
) -> bool {
    condition_store
        .conditions_for_like_cpp(
            ConditionSourceType::SpellClickEvent,
            ConditionId::new(creature_id, spell_id as i32, 0),
        )
        .is_some()
}

/// C++ `SpellClickInfo::IsFitToRequirements`.
pub fn spell_click_info_is_fit_to_requirements_like_cpp(
    info: &SpellClickInfoLikeCpp,
    context: SpellClickRequirementContextLikeCpp,
) -> bool {
    if !context.clicker_is_player {
        return true;
    }

    match info.user_type {
        SPELL_CLICK_USER_FRIEND_LIKE_CPP => context.clicker_is_friendly_to_summoner,
        SPELL_CLICK_USER_RAID_LIKE_CPP => context.clicker_is_in_raid_with_summoner,
        SPELL_CLICK_USER_PARTY_LIKE_CPP => context.clicker_is_in_party_with_summoner,
        _ => true,
    }
}

/// C++ `Player::CanSeeSpellClickOn`.
pub fn can_see_spell_click_on_like_cpp<'a>(
    spell_click_store: &NpcSpellClickStoreLikeCpp,
    condition_store: &'a ConditionEntriesByTypeStore,
    creature_entry: u32,
    creature_npc_flags: u64,
    clicker: Option<&'a WorldObject>,
    target: Option<&'a WorldObject>,
    requirement_context: SpellClickRequirementContextLikeCpp,
    mut meets: impl FnMut(&'a Condition, &mut ConditionSourceInfo<'a>) -> bool,
) -> bool {
    if (creature_npc_flags & UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP) == 0 {
        return false;
    }

    let click_bounds = spell_click_store.spell_click_info_map_bounds_like_cpp(creature_entry);
    if click_bounds.is_empty() {
        return false;
    }

    for click_info in click_bounds {
        if !spell_click_info_is_fit_to_requirements_like_cpp(click_info, requirement_context) {
            return false;
        }

        if is_object_meeting_spell_click_conditions_like_cpp(
            condition_store,
            creature_entry,
            click_info.spell_id,
            clicker,
            target,
            &mut meets,
        ) {
            return true;
        }
    }

    false
}

/// C++ `ConditionMgr::IsObjectMeetingVehicleSpellConditions`.
pub fn is_object_meeting_vehicle_spell_conditions_like_cpp<'a>(
    condition_store: &'a ConditionEntriesByTypeStore,
    creature_id: u32,
    spell_id: u32,
    player: Option<&'a WorldObject>,
    vehicle: Option<&'a WorldObject>,
    meets: impl FnMut(&'a Condition, &mut ConditionSourceInfo<'a>) -> bool,
) -> bool {
    if let Some(conditions) = condition_store.conditions_for_like_cpp(
        ConditionSourceType::VehicleSpell,
        ConditionId::new(creature_id, spell_id as i32, 0),
    ) {
        let mut source_info = ConditionSourceInfo::from_targets(player, vehicle, None);
        return is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions.as_slice(),
            condition_store,
            meets,
        );
    }

    true
}

/// C++ `ConditionMgr::IsObjectMeetingSmartEventConditions`.
pub fn is_object_meeting_smart_event_conditions_like_cpp<'a>(
    condition_store: &'a ConditionEntriesByTypeStore,
    entry_or_guid: i64,
    event_id: u32,
    source_type: u32,
    unit: Option<&'a WorldObject>,
    base_object: Option<&'a WorldObject>,
    meets: impl FnMut(&'a Condition, &mut ConditionSourceInfo<'a>) -> bool,
) -> bool {
    if let Some(conditions) = condition_store.conditions_for_like_cpp(
        ConditionSourceType::SmartEvent,
        ConditionId::new(event_id + 1, entry_or_guid as i32, source_type),
    ) {
        let mut source_info = ConditionSourceInfo::from_targets(unit, base_object, None);
        return is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions.as_slice(),
            condition_store,
            meets,
        );
    }

    true
}

/// C++ `ConditionMgr::IsObjectMeetingVendorItemConditions`.
pub fn is_object_meeting_vendor_item_conditions_like_cpp<'a>(
    condition_store: &'a ConditionEntriesByTypeStore,
    creature_id: u32,
    item_id: u32,
    player: Option<&'a WorldObject>,
    vendor: Option<&'a WorldObject>,
    meets: impl FnMut(&'a Condition, &mut ConditionSourceInfo<'a>) -> bool,
) -> bool {
    if let Some(conditions) = condition_store.conditions_for_like_cpp(
        ConditionSourceType::NpcVendor,
        ConditionId::new(creature_id, item_id as i32, 0),
    ) {
        let mut source_info = ConditionSourceInfo::from_targets(player, vendor, None);
        return is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions.as_slice(),
            condition_store,
            meets,
        );
    }

    true
}

/// C++ `ConditionMgr::GetConditionsForAreaTrigger`.
pub fn conditions_for_area_trigger_like_cpp(
    condition_store: &ConditionEntriesByTypeStore,
    area_trigger_id: u32,
    is_server_side: bool,
) -> Option<&[Condition]> {
    condition_store
        .conditions_for_like_cpp(
            ConditionSourceType::AreaTrigger,
            ConditionId::new(area_trigger_id, i32::from(is_server_side), 0),
        )
        .map(|conditions| conditions.as_slice())
}

/// C++ `ConditionMgr::IsObjectMeetingTrainerSpellConditions`.
pub fn is_object_meeting_trainer_spell_conditions_like_cpp<'a>(
    condition_store: &'a ConditionEntriesByTypeStore,
    trainer_id: u32,
    spell_id: u32,
    player: Option<&'a WorldObject>,
    meets: impl FnMut(&'a Condition, &mut ConditionSourceInfo<'a>) -> bool,
) -> bool {
    if let Some(conditions) = condition_store.conditions_for_like_cpp(
        ConditionSourceType::TrainerSpell,
        ConditionId::new(trainer_id, spell_id as i32, 0),
    ) {
        let mut source_info = ConditionSourceInfo::from_targets(player, None, None);
        return is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions.as_slice(),
            condition_store,
            meets,
        );
    }

    true
}

/// C++ `ConditionMgr::IsObjectMeetingVisibilityByObjectIdConditions`.
pub fn is_object_meeting_visibility_by_object_id_conditions_like_cpp<'a>(
    condition_store: &'a ConditionEntriesByTypeStore,
    object_type: u32,
    entry: u32,
    seer: Option<&'a WorldObject>,
    meets: impl FnMut(&'a Condition, &mut ConditionSourceInfo<'a>) -> bool,
) -> bool {
    if let Some(conditions) = condition_store.conditions_for_like_cpp(
        ConditionSourceType::ObjectIdVisibility,
        ConditionId::new(object_type, entry as i32, 0),
    ) {
        let mut source_info = ConditionSourceInfo::from_targets(seer, None, None);
        return is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions.as_slice(),
            condition_store,
            meets,
        );
    }

    true
}
