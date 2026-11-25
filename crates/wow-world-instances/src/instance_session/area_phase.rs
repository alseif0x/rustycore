//! Player::UpdateArea evaluates ObjectMgr's area phases on the owned Player.

use std::{collections::BTreeMap, sync::atomic::Ordering};
use wow_conditions::{
    ConditionAuraEffectSnapshot, ConditionMapStateSnapshot, ConditionPlayerProgressionSnapshot,
    ConditionPlayerQuestSnapshot, ConditionQuestObjectiveProgressSnapshot,
    ConditionQuestStatusSnapshot, ConditionSkillSnapshot, ConditionSourceInfo,
};
use wow_constants::ConditionType;
use wow_entities::ObjectNotifyFlags;
use wow_world_core::session::HubRef;

/// Apply the area pass after UpdateArea has stored the destination on the Player.
pub(super) fn update_player_area_phases_like_cpp(hub: HubRef<'_>) {
    update_player_phases_like_cpp(hub, true);
}

/// Settle committed quest and aura mutations on the owning Session.
pub fn update_changed_player_phase_conditions_like_cpp(hub: HubRef<'_>) {
    if hub.catalogs.player_area_phases.is_some()
        && hub.core.with_owned_player_like_cpp(|player| {
            player
                .gameplay_state()
                .quests
                .phase_conditions_dirty_like_cpp()
                || player
                    .unit()
                    .subsystems()
                    .auras
                    .has_player_phase_changes_like_cpp()
        }) == Some(true)
    {
        update_player_phases_like_cpp(hub, false);
    }
}

fn update_player_phases_like_cpp(hub: HubRef<'_>, area_changed: bool) {
    let catalogs = hub.catalogs;
    let (Some(rules), Some(areas), Some(phases), Some(groups), Some(conditions)) = (
        catalogs.player_area_phases.as_deref(),
        catalogs.area_table_store(),
        catalogs.phase_store.as_deref(),
        catalogs.phase_group_store.as_deref(),
        catalogs.condition_store(),
    ) else {
        return;
    };
    // Read the event authority before borrowing the Player through MapManager.
    let active_events = rules.active_event_ids();
    let difficulty_id = hub
        .core
        .current_canonical_player_map_difficulty_id_like_cpp();
    let Some(unit_snapshot) = hub.condition_player_unit_snapshot_like_cpp() else {
        return;
    };
    let player_snapshot = hub.condition_player_snapshot_like_cpp();
    let changed = hub
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let quests = &mut player.gameplay_state_mut().quests;
            quests.take_phase_conditions_dirty_like_cpp();
            let statuses: Vec<_> = quests
                .statuses_like_cpp()
                .iter()
                .map(|(&quest_id, row)| ConditionQuestStatusSnapshot {
                    quest_id,
                    status: row.status,
                })
                .collect();
            let rewarded: Vec<_> = quests
                .rewarded_quest_ids_like_cpp()
                .iter()
                .copied()
                .collect();
            let daily: Vec<_> = quests.daily_quest_ids_like_cpp().iter().copied().collect();
            let quest_catalog = catalogs.quests.store.as_deref();
            let objectives: Vec<_> = quests
                .statuses_like_cpp()
                .iter()
                .filter_map(|(&quest_id, row)| Some((quest_id, row, quest_catalog?.get(quest_id)?)))
                .flat_map(|(quest_id, row, quest)| {
                    quest.objectives.iter().filter_map(move |objective| {
                        let storage_index = usize::try_from(objective.storage_index).ok()?;
                        Some(ConditionQuestObjectiveProgressSnapshot {
                            quest_id,
                            objective_id: objective.id,
                            counter: row
                                .objective_counts
                                .get(storage_index)
                                .copied()
                                .unwrap_or(0),
                        })
                    })
                })
                .collect();
            let quest_snapshot = ConditionPlayerQuestSnapshot {
                statuses: &statuses,
                objective_progress: &objectives,
                rewarded_quest_ids: &rewarded,
                daily_quest_ids: &daily,
            };
            let skills: Vec<_> = player
                .skill_records_like_cpp()
                .iter()
                .map(|skill| ConditionSkillSnapshot {
                    skill_id: skill.skill_line_id,
                    base_value: u32::from(skill.current_value),
                })
                .collect();
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            let aura_changes = if area_changed {
                auras.reset_player_phase_tracking_like_cpp();
                BTreeMap::new()
            } else {
                auras.take_player_phase_changes_like_cpp()
            };
            let aura_snapshot: Vec<_> = auras
                .runtime_applications_like_cpp()
                .values()
                .flat_map(|aura| {
                    (0..u32::BITS)
                        .filter(|&index| aura.effect_mask & (1 << index) != 0)
                        .map(|effect_index| ConditionAuraEffectSnapshot {
                            spell_id: aura.spell_id as u32,
                            effect_index,
                        })
                })
                .collect();
            let (aura_phases, aura_groups) = wow_world_spell::active_aura_phase_ids_like_cpp(
                auras,
                catalogs.spell_catalogs.spell_store.as_deref(),
                catalogs.spell_catalogs.serverside_spell_store.as_deref(),
                catalogs.difficulty_store.as_deref(),
            );
            let world = player.unit_mut().world_mut();
            let evaluate_area =
                |area: &wow_data::PhaseAreaInfo, object: &wow_entities::WorldObject| {
                    let mut source = ConditionSourceInfo::from_targets(Some(object), None, None);
                    source.set_unit_target_snapshot(0, unit_snapshot);
                    source.set_player_target_snapshot(0, player_snapshot);
                    source.set_player_quest_target_snapshot(0, quest_snapshot);
                    source.set_unit_aura_target_snapshot(0, &aura_snapshot);
                    source.set_player_progression_target_snapshot(
                        0,
                        ConditionPlayerProgressionSnapshot {
                            skills: &skills,
                            items: &[],
                            equipped_item_or_gem_ids: &[],
                            spell_ids: &[],
                            achievement_ids: &[],
                            reputations: &[],
                            title_ids: &[],
                            battle_pet_counts: &[],
                            active_scene_ids: &[],
                        },
                    );
                    source.set_map_state_snapshot(ConditionMapStateSnapshot {
                        active_event_ids: active_events.as_deref().unwrap_or_default(),
                        difficulty_id: difficulty_id.map(u32::from).unwrap_or(0),
                        world_states: &[],
                        instance_data: &[],
                        instance_data64: &[],
                        boss_states: &[],
                        scenario_step_id: None,
                    });
                    wow_conditions::is_object_meet_to_conditions_like_cpp(
                        &mut source,
                        area.conditions.conditions(),
                        conditions,
                        |condition, source| {
                            let available = match condition.condition_type {
                                ConditionType::ActiveEvent => active_events.is_some(),
                                ConditionType::DifficultyId => difficulty_id.is_some(),
                                ConditionType::QuestObjectiveProgress => quest_catalog.is_some(),
                                ConditionType::None
                                | ConditionType::Aura
                                | ConditionType::ZoneId
                                | ConditionType::Team
                                | ConditionType::Skill
                                | ConditionType::QuestRewarded
                                | ConditionType::QuestTaken
                                | ConditionType::QuestNone
                                | ConditionType::Class
                                | ConditionType::Race
                                | ConditionType::Gender
                                | ConditionType::MapId
                                | ConditionType::AreaId
                                | ConditionType::PhaseId
                                | ConditionType::Level
                                | ConditionType::QuestComplete
                                | ConditionType::Alive
                                | ConditionType::HpVal
                                | ConditionType::HpPct
                                | ConditionType::DailyQuestDone
                                | ConditionType::QuestState
                                | ConditionType::TerrainSwap => true,
                                _ => false,
                            };
                            available
                                && condition.script_id == 0
                                && wow_conditions::condition_meets_basic_like_cpp(
                                    condition,
                                    source,
                                    |area, parent| areas.is_in_area_like_cpp(area, parent),
                                )
                                .value()
                                .unwrap_or(false)
                        },
                    )
                };
            let changed = if area_changed {
                wow_world_core::phasing::on_area_change_like_cpp(
                    world,
                    areas,
                    phases,
                    groups,
                    &rules.areas,
                    evaluate_area,
                    aura_phases,
                    aura_groups,
                )
                .changed
            } else {
                let direct_changed =
                    apply_phase_aura_changes(world, aura_changes, catalogs, phases, groups);
                let unavailable_terrain = wow_data::TerrainSwapStore::default();
                let update = wow_world_core::phasing::on_condition_change_like_cpp(
                    world,
                    phases,
                    groups,
                    catalogs
                        .terrain_swap_store
                        .as_deref()
                        .unwrap_or(&unavailable_terrain),
                    true,
                    |phase, object| {
                        let area = phase.area_id_like_cpp()?;
                        let rule = rules
                            .areas
                            .phases_for_area(area)?
                            .iter()
                            .find(|rule| rule.phase_id == phase.id())?;
                        Some(evaluate_area(rule, object))
                    },
                    |_, _| None,
                    aura_phases,
                    aura_groups,
                );
                direct_changed || update.changed
            };
            if changed && world.object().is_in_world() {
                world
                    .object_mut()
                    .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
                Some(world.guid())
            } else {
                None
            }
        })
        .flatten();

    if let Some(guid) = changed {
        hub.core.send_current_player_phase_like_cpp(guid);
        hub.core
            .flags
            .visibility_refresh_pending_like_cpp
            .store(true, Ordering::Release);
    }
}

/// SpellAuraEffects.cpp::HandlePhase and ::HandlePhaseGroup add/remove one
/// reference per real effect. Preserve area references on overlapping phases.
fn apply_phase_aura_changes(
    world: &mut wow_entities::WorldObject,
    auras: BTreeMap<(i32, u8, u32), i32>,
    catalogs: &wow_world_core::session::SessionCatalogs,
    phases: &wow_data::PhaseStore,
    groups: &wow_data::PhaseGroupStore,
) -> bool {
    let mut changes = BTreeMap::<u32, i32>::new();
    for ((spell_id, difficulty, mask), change) in auras {
        let (phase_ids, group_ids) = wow_world_spell::aura_phase_ids_like_cpp(
            spell_id,
            difficulty,
            mask,
            catalogs.spell_catalogs.spell_store.as_deref(),
            catalogs.spell_catalogs.serverside_spell_store.as_deref(),
            catalogs.difficulty_store.as_deref(),
        );
        let group_phases = group_ids
            .into_iter()
            .filter_map(|group| groups.phases_for_group(group))
            .flatten()
            .copied();
        for phase_id in phase_ids.into_iter().chain(group_phases) {
            *changes.entry(phase_id).or_default() += change;
        }
    }
    let shift = world.phase_shift_mut();
    let mut changed = false;
    for (id, change) in changes {
        if change > 0 {
            let flags = wow_world_core::phasing::phase_flags_for_id_like_cpp(phases, id);
            changed |= shift.add_phase_like_cpp(id, flags, change);
        } else {
            for _ in change..0 {
                changed |= shift.remove_phase_like_cpp(id);
            }
        }
    }
    changed
}
