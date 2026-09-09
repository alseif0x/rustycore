//! Gameobject lookup, catalogs and nearby search for the represented Session.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(crate) fn record_represented_gameobject_faction_template_like_cpp(
        &mut self,
        guid: ObjectGuid,
        faction_template: u32,
    ) {
        self.represented_gameobject_use_states
            .entry(guid)
            .or_default()
            .faction_template = (faction_template != 0).then_some(faction_template);
    }
    pub(crate) fn restore_represented_gameobject_override_flags_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) {
        if let Some(state) = self.represented_gameobject_use_states.get_mut(&guid) {
            if let Some(flags) = state.gameobject_override_flags {
                state.gameobject_flags = flags;
            }
        }
    }
    pub(crate) fn update_visible_gameobjects_like_cpp(&mut self) -> usize {
        let mut sent = 0;
        let visible_guids = self
            .client_visible_guids_like_cpp
            .snapshot_like_cpp()
            .into_iter()
            .collect::<Vec<_>>();
        for guid in visible_guids {
            if !guid.is_game_object() {
                continue;
            }
            let Some(access) = self.canonical_gameobject_access_like_cpp(guid) else {
                continue;
            };
            let Some(state) = self.represented_gameobject_use_states.get(&guid).cloned() else {
                continue;
            };
            let objective_refresh =
                self.represented_has_quest_for_gameobject_like_cpp(access.entry);
            let relation_refresh =
                self.represented_gameobject_is_for_quests_like_cpp(access.entry, &state);
            if !objective_refresh && !relation_refresh {
                continue;
            }

            let dynamic_flags =
                self.represented_gameobject_dynamic_flags_for_player_like_cpp(access.entry, &state);
            let Some(update) = Self::represented_gameobject_dynamic_flags_update_like_cpp(
                guid,
                self.player_map_id_like_cpp(),
                dynamic_flags,
            ) else {
                continue;
            };
            self.send_packet(&update);
            sent += 1;
        }

        sent
    }
    pub(crate) fn update_visible_gameobjects_or_spell_clicks_like_cpp(&mut self) -> usize {
        self.update_visible_gameobjects_like_cpp() + self.update_visible_spell_clicks_like_cpp()
    }
    pub(crate) fn visible_gameobjects_from_canonical_map_like_cpp(
        &self,
        map_id: u16,
        position: &wow_core::Position,
        visibility_radius: f32,
    ) -> Option<Vec<wow_packet::packets::update::GameObjectCreateData>> {
        let requested_map_id = u32::from(map_id);
        let player_map_key = self.current_canonical_player_map_key_like_cpp();
        let source_combat_reach = self.represented_visibility_source_combat_reach_like_cpp();
        // Resolve Player-owned phase state before entering the map. The
        // canonical Player handle uses this same manager mutex, so consulting
        // it from the entity read scope would recursively lock the manager.
        let viewer_phase_shift = self.represented_player_phase_shift_like_cpp();
        let candidates = {
            let manager = self.canonical_map_manager.as_ref()?;
            let Ok(manager) = manager.lock() else {
                return None;
            };
            let map = match player_map_key {
                Some(key) if key.map_id == requested_map_id => {
                    manager.find_map(key.map_id, key.instance_id)?
                }
                Some(_) => return None,
                None => manager.find_map(requested_map_id, 0)?,
            };
            let nearby = map.map().nearby_cell_guids_like_cpp(
                position.x,
                position.y,
                visibility_radius + source_combat_reach,
            );
            nearby
                .grid
                .gameobjects
                .into_iter()
                .filter_map(|guid| {
                    map.map()
                        .with_game_object_like_cpp(guid, |gameobject| (guid, gameobject.clone()))
                })
                .collect::<Vec<_>>()
        };
        let now = Instant::now();
        let mut gameobjects = Vec::new();

        for (guid, gameobject) in candidates {
            let object = gameobject.world();
            if !object.object().is_in_world()
                || object.map_id() != u32::from(map_id)
                || !Self::visibility_distance_allows_like_cpp(
                    position,
                    source_combat_reach,
                    &object.position(),
                    object.combat_reach(),
                    visibility_radius,
                )
            {
                continue;
            }
            if let Some(phase_shift) = self.represented_gameobject_phase_shifts.get(&guid)
                && !viewer_phase_shift
                    .as_ref()
                    .is_some_and(|viewer| viewer.can_see(phase_shift))
            {
                continue;
            }
            let state = self.represented_gameobject_use_states.get(&guid);
            if state.is_some_and(|state| {
                state
                    .per_player_despawn_until
                    .is_some_and(|until| until > now)
            }) {
                continue;
            }

            let create_data = if let Some(state) = state {
                let Some(display_id) = state.display_id else {
                    continue;
                };
                let Some(go_type) = state.go_type else {
                    continue;
                };
                let state_for_viewer =
                    self.represented_gameobject_go_state_for_viewer_like_cpp(state, now);

                wow_packet::packets::update::GameObjectCreateData {
                    guid,
                    entry: object.object().entry(),
                    dynamic_flags: self.represented_gameobject_dynamic_flags_for_player_like_cpp(
                        object.object().entry(),
                        state,
                    ),
                    display_id,
                    go_type,
                    position: object.position(),
                    rotation: state.rotation,
                    anim_progress: 255,
                    state: state_for_viewer as i8,
                    art_kit: gameobject.data().art_kit,
                    created_by: state.owner_guid.unwrap_or(ObjectGuid::EMPTY),
                    faction_template: state.faction_template.unwrap_or(0) as i32,
                    gameobject_flags: state.gameobject_flags,
                    world_effect_id: 0,
                    scale: state.scale,
                    level: 0, // non-transport GameObject: Level unused (period via AnimationData)
                    // Canonical state path does not yet record per-spawn ParentRotation;
                    // identity until state population is added (#NEXT.R8.ENTITIES.1216 follow-up).
                    parent_rotation: [0.0, 0.0, 0.0, 1.0],
                }
            } else {
                let Some(create_data) =
                    self.gameobject_create_data_from_canonical_like_cpp(guid, &gameobject)
                else {
                    continue;
                };
                create_data
            };

            gameobjects.push(create_data);
        }

        Some(gameobjects)
    }
    pub fn set_gameobject_display_info_store(&mut self, store: Arc<GameObjectDisplayInfoStore>) {
        self.gameobjects.display_info_store = Some(store);
    }
    pub(crate) fn gameobject_display_info_store(&self) -> Option<&Arc<GameObjectDisplayInfoStore>> {
        self.gameobjects.display_info_store.as_ref()
    }
    pub fn set_gameobject_template_lifecycle_store(
        &mut self,
        store: Arc<GameObjectTemplateLifecycleStoreLikeCpp>,
    ) {
        self.gameobject_template_lifecycle_store_like_cpp = Some(store);
    }
    pub(crate) fn gameobject_template_lifecycle_store(
        &self,
    ) -> Option<&Arc<GameObjectTemplateLifecycleStoreLikeCpp>> {
        self.gameobject_template_lifecycle_store_like_cpp.as_ref()
    }
    pub(in crate::session) fn visible_gameobject_guids_from_last_update_summary_like_cpp<F>(
        &self,
        select_guids: F,
    ) -> Option<(u32, u32, u64, Vec<ObjectGuid>)>
    where
        F: FnOnce(&wow_map::map::GameObjectsUpdateSummaryLikeCpp) -> Vec<ObjectGuid>,
    {
        let key = self.current_canonical_player_map_key_like_cpp()?;
        let manager = self.canonical_map_manager.as_ref()?;
        let player_guid = self.player_guid()?;
        let represented_seer_guid = self.represented_seer_guid_like_cpp;
        let direct_target_seer_gate_allows_send = represented_seer_guid.is_none_or(|seer_guid| {
            seer_guid.is_empty()
                || seer_guid == player_guid
                || self.represented_player_has_active_vehicle_like_cpp()
        });

        let manager = manager.lock().ok()?;
        let managed_map = manager.find_map(key.map_id, key.instance_id)?;
        let map = managed_map.map();
        let player = map.get_typed_player(player_guid)?;
        if !player.unit().world().object().is_in_world() {
            return None;
        }
        let player_position = player.unit().world().position();
        let player_phase_shift = player.unit().world().phase_shift().clone();
        let visibility_range = map.visibility_range();
        let represented_gameobject_phase_shifts = &self.represented_gameobject_phase_shifts;
        let mut shared_vision_target_guids = map
            .typed_combat_unit_guids_like_cpp()
            .into_iter()
            .filter(|target_guid| *target_guid != player_guid)
            .filter(|target_guid| {
                represented_seer_guid.is_some_and(|seer_guid| seer_guid == *target_guid)
            })
            .filter(|target_guid| {
                map.get_typed_player(*target_guid).is_some_and(|target| {
                    target.unit().world().object().is_in_world()
                        && target
                            .unit()
                            .subsystems()
                            .control
                            .shared_vision_guids
                            .contains(&player_guid)
                }) || map
                    .with_creature_like_cpp(*target_guid, |target| {
                        target.unit().world().object().is_in_world()
                            && target
                                .unit()
                                .subsystems()
                                .control
                                .shared_vision_guids
                                .contains(&player_guid)
                    })
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        shared_vision_target_guids.sort_by_key(|guid| (guid.high_value(), guid.low_value()));
        let dynamic_object_target_guid = represented_seer_guid.filter(|seer_guid| {
            if !seer_guid.is_dynamic_object() {
                return false;
            }
            let Some(dynamic_object) = map.get_typed_dynamic_object(*seer_guid) else {
                return false;
            };
            if !dynamic_object.world().object().is_in_world() {
                return false;
            }
            let caster_guid = dynamic_object
                .bound_caster()
                .unwrap_or_else(|| dynamic_object.caster_guid());
            caster_guid == player_guid && caster_guid.is_player()
        });
        let summary = managed_map.last_game_objects_update_summary();
        let guids = select_guids(&summary)
            .into_iter()
            .filter(|guid| {
                if !guid.is_game_object() {
                    return false;
                }
                let Some(gameobject) = map.get_typed_game_object(*guid) else {
                    return self.client_visible_guids_like_cpp.contains(guid);
                };
                if !gameobject.world().object().is_in_world() {
                    return false;
                }
                let gameobject_phase_shift = represented_gameobject_phase_shifts
                    .get(guid)
                    .unwrap_or_else(|| gameobject.world().phase_shift());
                let gameobject_position = gameobject.world().position();
                let direct_target_allows = direct_target_seer_gate_allows_send
                    && player_phase_shift.can_see(gameobject_phase_shift)
                    && gameobject_position.is_within_dist_2d(&player_position, visibility_range);
                if direct_target_allows {
                    return true;
                }
                if shared_vision_target_guids.iter().any(|target_guid| {
                    if let Some(target) = map.get_typed_player(*target_guid) {
                        let target_world = target.unit().world();
                        target_world.phase_shift().can_see(gameobject_phase_shift)
                            && gameobject_position
                                .is_within_dist_2d(&target_world.position(), visibility_range)
                    } else {
                        map.with_creature_like_cpp(*target_guid, |target| {
                            let target_world = target.unit().world();
                            target_world.phase_shift().can_see(gameobject_phase_shift)
                                && gameobject_position
                                    .is_within_dist_2d(&target_world.position(), visibility_range)
                        })
                        .unwrap_or(false)
                    }
                }) {
                    return true;
                }
                dynamic_object_target_guid.is_some_and(|dynamic_object_guid| {
                    map.get_typed_dynamic_object(dynamic_object_guid)
                        .is_some_and(|dynamic_object| {
                            let dynamic_object_world = dynamic_object.world();
                            dynamic_object_world
                                .phase_shift()
                                .can_see(gameobject_phase_shift)
                                && gameobject_position.is_within_dist_2d(
                                    &dynamic_object_world.position(),
                                    visibility_range,
                                )
                        })
                })
            })
            .collect::<Vec<_>>();
        Some((
            key.map_id,
            key.instance_id,
            managed_map.update_calls().len() as u64,
            guids,
        ))
    }
}
