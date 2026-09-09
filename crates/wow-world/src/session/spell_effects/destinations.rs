//! Implicit destination and nearby-entry target resolution for effects.
//!
//! Moved out of the Session root under #621. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_focus_destination_target_data_like_cpp(
        &self,
        spell_id: i32,
        effect: &wow_data::SpellEffectInfo,
        target_data: &SpellTargetData,
        focus_object: Option<RepresentedSpellFocusObjectLikeCpp>,
    ) -> Option<SpellTargetData> {
        let focus_object = focus_object?;
        if target_data.dst_location.is_some()
            || !effect.has_focus_destination_implicit_target_like_cpp()
        {
            return None;
        }

        if self.has_implicit_target_conditions_like_cpp(spell_id, effect.effect_index) {
            return None;
        }

        let focus_position = self.apply_spell_destination_facing_override_like_cpp(
            spell_id,
            effect,
            focus_object.position,
        );

        let mut target_data = target_data.clone();
        target_data.flags |= 0x0000_0040; // C++ TARGET_FLAG_DEST_LOCATION
        target_data.dst_location = Some(wow_packet::packets::spell::TargetLocation {
            transport: ObjectGuid::EMPTY,
            position: focus_position,
        });
        Some(target_data)
    }
    fn apply_spell_destination_facing_override_like_cpp(
        &self,
        spell_id: i32,
        effect: &wow_data::SpellEffectInfo,
        mut position: Position,
    ) -> Position {
        if self
            .spell_catalogs
            .spell_misc_store
            .as_deref()
            .and_then(|store| store.get_by_spell_id(u32::try_from(spell_id).ok()?))
            .is_some_and(|misc| {
                (misc.attributes[4] as u32
                    & wow_data::spell::attributes::SPELL_ATTR4_USE_FACING_FROM_SPELL)
                    != 0
            })
        {
            position.orientation = effect.position_facing;
        }
        position
    }
    pub(in crate::session) fn represented_home_destination_target_data_like_cpp(
        &self,
        caster_guid: ObjectGuid,
        effect: &wow_data::SpellEffectInfo,
        target_data: &SpellTargetData,
    ) -> Option<SpellTargetData> {
        if !matches!(
            effect.implicit_target_1,
            wow_data::spell::implicit_targets::TARGET_DEST_HOME
        ) && !matches!(
            effect.implicit_target_2,
            wow_data::spell::implicit_targets::TARGET_DEST_HOME
        ) {
            return None;
        }

        // C++ starts `SelectImplicitCasterDestTargets` from
        // `SpellDestination(*m_caster)`, then replaces it with `m_homebind`
        // only when the effective caster is a Player. A triggered creature
        // cast must therefore keep the creature's own position rather than
        // borrowing the logged-in session player's homebind.
        let (map_id, transport, position) = if Some(caster_guid) == self.player_guid() {
            let homebind = self.represented_homebind_like_cpp()?;
            (homebind.map_id, ObjectGuid::EMPTY, homebind.position)
        } else {
            self.represented_effective_caster_destination_like_cpp(caster_guid)?
        };
        let mut target_data = target_data.clone();
        target_data.flags |= 0x0000_0040; // C++ TARGET_FLAG_DEST_LOCATION
        target_data.dst_location = Some(wow_packet::packets::spell::TargetLocation {
            transport,
            position,
        });
        target_data.map_id = Some(i32::try_from(map_id).unwrap_or(i32::MAX));
        Some(target_data)
    }
    fn represented_effective_caster_destination_like_cpp(
        &self,
        caster_guid: ObjectGuid,
    ) -> Option<(u32, ObjectGuid, Position)> {
        let map_key = self
            .current_canonical_player_map_key_like_cpp()
            .unwrap_or_else(|| wow_map::MapKey::new(u32::from(self.player_map_id_like_cpp()), 0));
        let legacy_map_id = u16::try_from(map_key.map_id).ok()?;
        let canonical_destination = self.canonical_map_manager.as_ref().and_then(|manager| {
            let manager = manager.lock().ok()?;
            let map = manager.find_map(map_key.map_id, map_key.instance_id)?;
            let map = map.map();
            let (world_position, vehicle_base_guid) = if Some(caster_guid) == self.player_guid() {
                let player = map.get_typed_player(caster_guid)?;
                (
                    player.unit().world().position(),
                    player.unit().subsystems().vehicle.vehicle_guid,
                )
            } else {
                map.with_creature_like_cpp(caster_guid, |creature| {
                    (
                        creature.unit().world().position(),
                        creature.unit().subsystems().vehicle.vehicle_guid,
                    )
                })?
            };
            if let Some(vehicle_base_guid) = vehicle_base_guid {
                let vehicle_base_position = map
                    .get_typed_player(vehicle_base_guid)
                    .map(|player| player.unit().world().position())
                    .or_else(|| {
                        map.creature_transform_vitals_snapshot_like_cpp(vehicle_base_guid)
                            .map(|creature| creature.position)
                    })?;
                Some((
                    vehicle_base_guid,
                    wow_entities::calculate_passenger_offset(world_position, vehicle_base_position),
                ))
            } else if let Some(transport) =
                map.get_typed_transport_for_passenger_like_cpp(caster_guid)
            {
                Some((
                    transport.world().guid(),
                    transport.calculate_passenger_offset(world_position),
                ))
            } else {
                Some((ObjectGuid::EMPTY, world_position))
            }
        });
        let (transport, position) = canonical_destination.or_else(|| {
            if Some(caster_guid) == self.player_guid() {
                self.player_position_like_cpp()
                    .map(|position| (ObjectGuid::EMPTY, position))
            } else {
                self.map_manager.as_ref().and_then(|manager| {
                    manager
                        .read()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .find_creature(legacy_map_id, map_key.instance_id, caster_guid)
                        .map(|creature| (ObjectGuid::EMPTY, creature.position()))
                })
            }
        })?;
        Some((map_key.map_id, transport, position))
    }
    pub(in crate::session) fn represented_transport_destination_world_position_like_cpp(
        &self,
        transport_guid: ObjectGuid,
        transport_offset: Position,
    ) -> Option<Position> {
        let map_key = self.current_canonical_player_map_key_like_cpp()?;
        let manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
        let map = manager.find_map(map_key.map_id, map_key.instance_id)?;
        let map = map.map();
        if let Some(transport) = map.get_typed_transport_like_cpp(transport_guid) {
            return Some(transport.calculate_passenger_position(transport_offset));
        }
        let vehicle_base_position = map
            .get_typed_player(transport_guid)
            .map(|player| player.unit().world().position())
            .or_else(|| {
                map.creature_transform_vitals_snapshot_like_cpp(transport_guid)
                    .map(|creature| creature.position)
            })?;
        Some(wow_entities::calculate_passenger_position(
            transport_offset,
            vehicle_base_position,
        ))
    }
    pub(in crate::session) fn represented_db_caster_destination_target_data_like_cpp(
        &self,
        caster_guid: ObjectGuid,
        spell_info: &wow_data::SpellInfo,
        effect: &wow_data::SpellEffectInfo,
        target_data: &SpellTargetData,
    ) -> Option<SpellTargetData> {
        if target_data.dst_location.is_some()
            || !matches!(
                effect.implicit_target_1,
                wow_data::spell::implicit_targets::TARGET_DEST_DB
            ) && !matches!(
                effect.implicit_target_2,
                wow_data::spell::implicit_targets::TARGET_DEST_DB
            )
        {
            return None;
        }

        let spell_id_u32 = u32::try_from(spell_info.spell_id).ok()?;
        let target_position = self
            .spell_catalogs
            .spell_target_position_store
            .as_deref()
            .and_then(|store| store.get(spell_id_u32, effect.effect_index));
        let cross_map_allowed = spell_info.effects().iter().any(|candidate| {
            matches!(
                candidate.effect,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS
                    | wow_data::spell::spell_effect_types::SPELL_EFFECT_BIND
            )
        });
        // C++ initializes SpellDestination from *m_caster before applying the
        // optional DB/object overrides. This must use the effective caster,
        // including triggered creature casts and transport offsets.
        let (caster_map_id, caster_transport, caster_position) =
            self.represented_effective_caster_destination_like_cpp(caster_guid)?;
        let (position, map_id, transport) = if let Some(target_position) = target_position {
            if cross_map_allowed {
                (
                    target_position.position,
                    Some(i32::from(target_position.target_map_id)),
                    ObjectGuid::EMPTY,
                )
            } else if u32::from(target_position.target_map_id) == caster_map_id {
                (target_position.position, None, ObjectGuid::EMPTY)
            } else {
                (caster_position, None, caster_transport)
            }
        } else if let Some(target_position) =
            self.represented_object_target_position_like_cpp(target_data)
        {
            (target_position, None, ObjectGuid::EMPTY)
        } else {
            (caster_position, None, caster_transport)
        };
        let position = self.apply_spell_destination_facing_override_like_cpp(
            spell_info.spell_id,
            effect,
            position,
        );

        let mut target_data = target_data.clone();
        target_data.flags |= 0x0000_0040; // C++ TARGET_FLAG_DEST_LOCATION
        target_data.dst_location = Some(wow_packet::packets::spell::TargetLocation {
            transport,
            position,
        });
        target_data.map_id = map_id;
        Some(target_data)
    }
    pub(in crate::session) fn represented_nearby_entry_destination_target_data_like_cpp(
        &mut self,
        spell_id: i32,
        effect: &wow_data::SpellEffectInfo,
        target_data: &SpellTargetData,
    ) -> Option<SpellTargetData> {
        if target_data.dst_location.is_some()
            || !effect.has_focus_destination_implicit_target_like_cpp()
        {
            return None;
        }
        let is_or_db = matches!(
            effect.implicit_target_1,
            wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB
        ) || matches!(
            effect.implicit_target_2,
            wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB
        );

        let spell_id_u32 = u32::try_from(spell_id).ok()?;
        let implicit_conditions =
            self.implicit_target_conditions_like_cpp(spell_id, effect.effect_index);
        let has_implicit_conditions = implicit_conditions
            .as_ref()
            .is_some_and(|conditions| !conditions.is_empty());
        let target_position = if is_or_db && !has_implicit_conditions {
            self.spell_catalogs
                .spell_target_position_store
                .as_deref()
                .and_then(|store| store.get(spell_id_u32, effect.effect_index))
        } else {
            None
        };
        let caster_position = self.player_position_like_cpp()?;
        let range = self
            .spell_catalogs
            .spell_misc_store
            .as_deref()
            .and_then(|store| store.get_by_spell_id(spell_id_u32))
            .and_then(|misc| {
                self.spell_catalogs
                    .spell_range_store
                    .as_deref()
                    .and_then(|store| store.get(u32::from(misc.range_index)))
            })
            .map(|range| range.range_max[0].max(range.range_max[1]))?;
        let radius = spell_effect_radius_like_cpp(
            effect.effect_radius_index_1,
            self.spell_catalogs.spell_radius_store.as_deref(),
        );
        let position = if let Some(target_position) = target_position {
            if target_position.target_map_id == self.player_map_id_like_cpp()
                && caster_position.distance(&target_position.position) <= range
            {
                target_position.position
            } else if radius == 0.0 {
                caster_position
            } else {
                self.randomized_or_db_caster_fallback_destination_like_cpp(caster_position, radius)
            }
        } else if let Some(position) = self.represented_nearby_entry_destination_like_cpp(
            effect,
            &caster_position,
            range,
            implicit_conditions.as_deref().map(Vec::as_slice),
        ) {
            self.apply_spell_destination_facing_override_like_cpp(spell_id, effect, position)
        } else if !is_or_db {
            return None;
        } else if radius == 0.0 {
            self.apply_spell_destination_facing_override_like_cpp(spell_id, effect, caster_position)
        } else {
            let randomized =
                self.randomized_or_db_caster_fallback_destination_like_cpp(caster_position, radius);
            self.apply_spell_destination_facing_override_like_cpp(spell_id, effect, randomized)
        };
        let mut target_data = target_data.clone();
        target_data.flags |= 0x0000_0040; // C++ TARGET_FLAG_DEST_LOCATION
        target_data.dst_location = Some(wow_packet::packets::spell::TargetLocation {
            transport: ObjectGuid::EMPTY,
            position,
        });
        Some(target_data)
    }
    pub(in crate::session) fn represented_gameobject_summon_missing_nearby_entry_destination_like_cpp(
        &mut self,
        spell_id: i32,
        spell_info: &wow_data::SpellInfo,
        target_data: &SpellTargetData,
        focus_object: Option<RepresentedSpellFocusObjectLikeCpp>,
    ) -> bool {
        if target_data.dst_location.is_some() {
            return false;
        }

        spell_info.effects().iter().any(|effect| {
            if effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_SUMMON_OBJECT_WILD
                && !spell_effect_is_represented_summon_object_slot_like_cpp(effect.effect)
            {
                return false;
            }
            if !spell_effect_has_non_or_db_nearby_entry_destination_like_cpp(effect) {
                return false;
            }
            if self
                .represented_focus_destination_target_data_like_cpp(
                    spell_id,
                    effect,
                    target_data,
                    focus_object,
                )
                .is_some()
            {
                return false;
            }

            self.represented_nearby_entry_destination_target_data_like_cpp(
                spell_id,
                effect,
                target_data,
            )
            .is_none()
        })
    }
    fn randomized_or_db_caster_fallback_destination_like_cpp(
        &mut self,
        caster_position: Position,
        radius: f32,
    ) -> Position {
        if !radius.is_finite() || radius <= 0.0 {
            return caster_position;
        }

        // C++ target 142 is TARGET_DIR_FRONT_RIGHT
        // (`SpellInfo.cpp:385`), so `MovePositionToFirstCollision` uses
        // caster orientation plus -PI/4. The represented seam keeps the
        // radius/direction behavior; terrain raycast/first-collision remains
        // part of the full map/path runtime work.
        let distance = self
            .represented_runtime_rng_like_cpp
            .gen_range(0.0..=radius);
        let angle = caster_position.orientation - std::f32::consts::FRAC_PI_4;
        Position::new(
            caster_position.x + distance * angle.cos(),
            caster_position.y + distance * angle.sin(),
            caster_position.z,
            caster_position.orientation,
        )
    }
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

        let Some(player_map_key) = self.current_canonical_player_map_key_like_cpp() else {
            return None;
        };
        let has_implicit_conditions =
            implicit_conditions.is_some_and(|conditions| !conditions.is_empty());
        let caster_object = if has_implicit_conditions {
            self.build_condition_player_object_like_cpp()
        } else {
            None
        };
        let player_unit_snapshot = self.condition_player_unit_snapshot_like_cpp()?;
        let player_snapshot = self.condition_player_snapshot_like_cpp();
        let player_condition_context = if has_implicit_conditions {
            self.represented_player_condition_context_like_cpp()
        } else {
            None
        };
        let player_condition_store = if has_implicit_conditions {
            self.player_condition_store.as_ref().cloned()
        } else {
            None
        };
        let area_table_store = if has_implicit_conditions {
            self.area_table_store.as_ref().cloned()
        } else {
            None
        };
        let Some(manager) = &self.canonical_map_manager else {
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
                            Some(crate::conditions::ConditionUnitSnapshot {
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
