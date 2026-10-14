//! The nearby-entry destination search with its implicit-condition check.
//!
//! Moved out of the Session root under #621. Behaviour is preserved; the
//! canonical owner of this state is unchanged. The other implicit destination
//! resolvers moved to `wow_world_spell` (#1363 slice 9d).

use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_nearby_entry_destination_like_cpp(
        &self,
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

        let Some(player_map_key) = self.core.current_canonical_player_map_key_like_cpp() else {
            return None;
        };
        let has_implicit_conditions =
            implicit_conditions.is_some_and(|conditions| !conditions.is_empty());
        let caster_object = if has_implicit_conditions {
            crate::session::hub_ref(self).build_condition_player_object_like_cpp()
        } else {
            None
        };
        let player_unit_snapshot =
            crate::session::hub_ref(self).condition_player_unit_snapshot_like_cpp()?;
        let player_snapshot = crate::session::hub_ref(self).condition_player_snapshot_like_cpp();
        let player_condition_context = if has_implicit_conditions {
            self.represented_player_condition_context_like_cpp()
        } else {
            None
        };
        let player_condition_store = if has_implicit_conditions {
            self.catalogs.player_condition_store.as_ref().cloned()
        } else {
            None
        };
        let area_table_store = if has_implicit_conditions {
            self.catalogs.area_table_store.as_ref().cloned()
        } else {
            None
        };
        let Some(manager) = &self.core.canonical_map_manager else {
            return None;
        };
        let Ok(manager) = manager.lock() else {
            return None;
        };
        let Some(map) = manager.find_map(player_map_key.map_id, player_map_key.instance_id) else {
            return None;
        };
        let nearby =
            map.map()
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
                        && self.represented_nearby_candidate_meets_implicit_conditions_like_cpp(
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
                && self.represented_nearby_candidate_meets_implicit_conditions_like_cpp(
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
}
