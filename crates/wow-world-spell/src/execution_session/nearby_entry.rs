// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The nearby-entry destination search with its implicit-condition check
//! (#1363 slice 12b2b), moved from the World
//! `session/spell_effects/{destinations,effects}.rs`. The player-condition
//! context is the vendor host's projection, viewed through
//! [`wow_world_inventory::PlayerConditionContextSourceLikeCpp::as_context`]
//! (the World implements it with the same `as_context` the body called).

use wow_constants::UnitStandStateType;
use wow_core::Position;
use wow_entities::WorldObject;
use wow_world_inventory::{PlayerConditionContextSourceLikeCpp, VendorSessionHostLikeCpp};

use super::SpellExecutionHostLikeCpp;

pub fn represented_nearby_entry_destination_like_cpp<
    H: SpellExecutionHostLikeCpp + VendorSessionHostLikeCpp,
>(
    host: &H,
    effect: &wow_data::SpellEffectInfo,
    caster_position: &Position,
    range: f32,
    implicit_conditions: Option<&[wow_data::Condition]>,
) -> Option<Position> {
    let Ok(entry) = u32::try_from(effect.effect_misc_value_1) else {
        return None;
    };
    if entry == 0 || !range.is_finite() || range <= 0.0 {
        return None;
    }

    let Some(player_map_key) = host.core().current_canonical_player_map_key_like_cpp() else {
        return None;
    };
    let has_implicit_conditions =
        implicit_conditions.is_some_and(|conditions| !conditions.is_empty());
    let caster_object = if has_implicit_conditions {
        host.hub_ref().build_condition_player_object_like_cpp()
    } else {
        None
    };
    let player_unit_snapshot = host.hub_ref().condition_player_unit_snapshot_like_cpp()?;
    let player_snapshot = host.hub_ref().condition_player_snapshot_like_cpp();
    let player_condition_context = if has_implicit_conditions {
        host.represented_player_condition_context_like_cpp()
    } else {
        None
    };
    let player_condition_store = if has_implicit_conditions {
        host.hub_ref()
            .catalogs
            .player_condition_store
            .as_ref()
            .cloned()
    } else {
        None
    };
    let area_table_store = if has_implicit_conditions {
        host.hub_ref().catalogs.area_table_store.as_ref().cloned()
    } else {
        None
    };
    let Some(manager) = &host.core().canonical_map_manager else {
        return None;
    };
    let Ok(manager) = manager.lock() else {
        return None;
    };
    let Some(map) = manager.find_map(player_map_key.map_id, player_map_key.instance_id) else {
        return None;
    };
    let nearby = map
        .map()
        .nearby_cell_guids_like_cpp(caster_position.x, caster_position.y, range);
    let mut best: Option<(f32, Position)> = None;

    for guid in nearby
        .world
        .creatures
        .iter()
        .chain(nearby.grid.creatures.iter())
    {
        let candidate = map
            .map()
            .with_creature_like_cpp(*guid, |creature| {
                if creature.unit().world().object().entry() != entry {
                    return None;
                }
                let position = creature.unit().world().position();
                let distance = position.distance(caster_position);
                (distance < range
                    && best.is_none_or(|(best_distance, _)| distance < best_distance)
                    && represented_nearby_candidate_meets_implicit_conditions_like_cpp(
                        host,
                        creature.unit().world(),
                        Some(wow_conditions::ConditionUnitSnapshot {
                            level: u32::from(creature.level()),
                            health: creature.current_health(),
                            max_health: creature.max_health(),
                            class_mask: 0,
                            race: 0,
                            creature_type: None,
                            is_alive: creature.is_alive(),
                            is_charmed: false,
                            in_water: false,
                            unit_state: 0,
                            stand_state: UnitStandStateType::Stand as u32,
                        }),
                        implicit_conditions,
                        caster_object.as_ref(),
                        player_unit_snapshot,
                        player_snapshot,
                        player_condition_store.as_deref(),
                        player_condition_context.as_ref(),
                        area_table_store.as_deref(),
                    ))
                .then_some((distance, position))
            })
            .flatten();
        if let Some((distance, position)) = candidate {
            best = Some((distance, position));
        }
    }

    for guid in nearby.grid.gameobjects.iter() {
        let Some(gameobject) = map.map().get_typed_game_object(*guid) else {
            continue;
        };
        if gameobject.world().object().entry() != entry {
            continue;
        }
        let position = gameobject.world().position();
        let distance = position.distance(caster_position);
        if distance < range
            && best.is_none_or(|(best_distance, _)| distance < best_distance)
            && represented_nearby_candidate_meets_implicit_conditions_like_cpp(
                host,
                gameobject.world(),
                None,
                implicit_conditions,
                caster_object.as_ref(),
                player_unit_snapshot,
                player_snapshot,
                player_condition_store.as_deref(),
                player_condition_context.as_ref(),
                area_table_store.as_deref(),
            )
        {
            best = Some((distance, position));
        }
    }

    best.map(|(_, position)| position)
}

fn represented_nearby_candidate_meets_implicit_conditions_like_cpp<
    H: SpellExecutionHostLikeCpp + VendorSessionHostLikeCpp,
>(
    host: &H,
    candidate: &WorldObject,
    candidate_unit_snapshot: Option<wow_conditions::ConditionUnitSnapshot>,
    implicit_conditions: Option<&[wow_data::Condition]>,
    caster_object: Option<&WorldObject>,
    player_unit_snapshot: wow_conditions::ConditionUnitSnapshot,
    player_snapshot: wow_conditions::ConditionPlayerSnapshot,
    player_condition_store: Option<&wow_data::PlayerConditionStore>,
    player_condition_context: Option<&H::PlayerConditionContext>,
    area_table_store: Option<&wow_data::AreaTableStore>,
) -> bool {
    let Some(conditions) = implicit_conditions.filter(|conditions| !conditions.is_empty()) else {
        return true;
    };
    let Some(condition_store) = host.hub_ref().catalogs.condition_store.as_ref() else {
        return false;
    };

    // C++ `WorldObjectSpellTargetCheck` builds `ConditionSourceInfo(nullptr, caster)`
    // and then assigns the tested target to condition slot 0.
    let mut source_info =
        wow_conditions::ConditionSourceInfo::from_targets(Some(candidate), caster_object, None);
    if let Some(snapshot) = candidate_unit_snapshot {
        source_info.set_unit_target_snapshot(0, snapshot);
    }
    source_info.set_unit_target_snapshot(1, player_unit_snapshot);
    source_info.set_player_target_snapshot(1, player_snapshot);
    if let (Some(store), Some(context)) = (player_condition_store, player_condition_context) {
        source_info.set_player_condition_store(store);
        if let Some(context) = context.as_context(host) {
            source_info.set_player_condition_context(1, context);
        }
    }

    wow_conditions::is_object_meet_to_conditions_like_cpp(
        &mut source_info,
        conditions,
        condition_store.as_ref(),
        |condition, source_info| {
            wow_conditions::condition_meets_basic_like_cpp(
                condition,
                source_info,
                |area_id, required_area_id| {
                    area_table_store
                        .is_some_and(|store| store.is_in_area_like_cpp(area_id, required_area_id))
                },
            )
            .value()
            .unwrap_or(false)
        },
    )
}
