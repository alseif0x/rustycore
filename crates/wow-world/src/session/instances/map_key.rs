//! Map key resolution used by the represented Session.
//!
//! Moved out of the Session root under #613. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn apply_create_map_side_effects_like_cpp(
        &mut self,
        map_id: u32,
        decision: &wow_map::CreateMapDecision,
    ) -> CreateMapSideEffectApplySummaryLikeCpp {
        let side_effects = match decision {
            wow_map::CreateMapDecision::Existing { side_effects, .. }
            | wow_map::CreateMapDecision::Create { side_effects, .. }
            | wow_map::CreateMapDecision::Reject { side_effects } => side_effects,
        };
        let decision_difficulty_id = match decision {
            wow_map::CreateMapDecision::Existing { difficulty_id, .. }
            | wow_map::CreateMapDecision::Create { difficulty_id, .. } => Some(*difficulty_id),
            wow_map::CreateMapDecision::Reject { .. } => None,
        };

        let mut summary = CreateMapSideEffectApplySummaryLikeCpp::default();
        for side_effect in side_effects {
            match *side_effect {
                wow_map::CreateMapSideEffect::SetPlayerRecentInstance { instance_id } => {
                    if self.set_represented_player_recent_instance_like_cpp(map_id, instance_id) {
                        summary.player_recent_instance_sets += 1;
                    }
                }
                wow_map::CreateMapSideEffect::SetGroupRecentInstance {
                    owner_guid_counter,
                    instance_id,
                } => {
                    let owner_guid = i64::try_from(owner_guid_counter)
                        .ok()
                        .map(|counter| ObjectGuid::create_player(1, counter));
                    let updated = self
                        .resolved_group_guid_like_cpp()
                        .zip(owner_guid)
                        .and_then(|(group_guid, owner_guid)| {
                            self.group_registry
                                .as_ref()?
                                .set_recent_instance_transition_like_cpp(
                                    group_guid,
                                    map_id,
                                    owner_guid,
                                    instance_id,
                                )
                                .ok()
                        })
                        .is_some();
                    if updated {
                        summary.group_recent_instance_sets += 1;
                    } else {
                        summary.skipped_group_recent_instance_sets += 1;
                    }
                }
                wow_map::CreateMapSideEffect::CreateInstanceLockForNewInstance {
                    owner_guid_counter,
                    instance_id,
                } => {
                    let owner_guid = i64::try_from(owner_guid_counter)
                        .ok()
                        .map(|counter| ObjectGuid::create_player(1, counter));
                    let created = decision_difficulty_id
                        .zip(owner_guid)
                        .and_then(|(difficulty_id, owner_guid)| {
                            self.create_instance_lock_for_new_instance_side_effect_like_cpp(
                                map_id,
                                difficulty_id,
                                owner_guid,
                                instance_id,
                            )
                        })
                        .is_some();
                    if created {
                        summary.instance_lock_creates += 1;
                    } else {
                        summary.skipped_instance_lock_creates += 1;
                    }
                }
                wow_map::CreateMapSideEffect::SetInstanceLockInstanceId { instance_id } => {
                    let updated = decision_difficulty_id
                        .and_then(|difficulty_id| {
                            self.set_active_instance_lock_instance_id_side_effect_like_cpp(
                                map_id,
                                difficulty_id,
                                instance_id,
                            )
                        })
                        .is_some();
                    if updated {
                        summary.instance_lock_instance_id_updates += 1;
                    } else {
                        summary.skipped_instance_lock_instance_id_updates += 1;
                    }
                }
                wow_map::CreateMapSideEffect::TeleportToBattlegroundEntryPoint => {
                    summary.pending_battleground_entry_teleports += 1;
                }
            }
        }

        summary
    }
    pub(crate) fn create_map_player_context_like_cpp(
        &self,
        map_id: u32,
        map_entry: wow_data::map::MapEntry,
        player_guid: ObjectGuid,
    ) -> Option<wow_map::CreateMapPlayerContext> {
        let player_difficulty_id =
            self.represented_player_difficulty_id_for_map_entry_like_cpp(map_id, map_entry)?;

        let group = self
            .resolved_group_guid_like_cpp()
            .and_then(|group_guid| self.group_registry.as_ref()?.get(&group_guid))
            .map(|group| {
                let difficulty_id = self.represented_group_difficulty_id_for_map_entry_like_cpp(
                    map_id, map_entry, &group,
                );
                wow_map::CreateMapGroupContext {
                    difficulty_id,
                    recent_instance_owner_guid_counter: group
                        .recent_instance_owner_like_cpp(map_id)
                        .counter() as u64,
                    recent_instance_id: group.recent_instance_id_like_cpp(map_id),
                }
            });

        Some(wow_map::CreateMapPlayerContext {
            guid_counter: player_guid.counter() as u64,
            team_id: player_team_id_for_race_cpp(self.player_race_like_cpp()),
            battleground_id: 0,
            has_battleground: false,
            player_difficulty_id,
            player_recent_instance_id: self.resolved_player_recent_instance_id_like_cpp(map_id)?,
            group,
        })
    }
    pub(crate) fn create_map_db2_entries_like_cpp(
        &self,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
    ) -> Option<wow_instances::MapDb2Entries> {
        wow_instances::MapDb2Entries::from_downscaled_stores_like_cpp(
            self.map_store()?.as_ref(),
            self.map_difficulty_store()?.as_ref(),
            self.difficulty_store()?.as_ref(),
            map_id,
            difficulty_id,
        )
    }
    pub(crate) fn player_map_visibility_range_like_cpp(&self, map_id: u16) -> f32 {
        self.legacy_creature_aggro_config_like_cpp
            .map_visibility_range_like_cpp(map_id)
    }
    pub fn set_vmap_indoor_check_like_cpp(&mut self, enabled: bool) {
        self.vmap_indoor_check_like_cpp = enabled;
    }
    pub fn set_mmap_runtime_config_like_cpp(&mut self, config: MMapRuntimeConfigLikeCpp) {
        self.mmap_runtime_config_like_cpp = config;
    }
    pub fn mmap_runtime_config_like_cpp(&self) -> &MMapRuntimeConfigLikeCpp {
        &self.mmap_runtime_config_like_cpp
    }
    /// Set the C++ AdventureMapPOI.db2 store for this session.
    #[cfg(test)]
    pub fn set_adventure_map_poi_store(&mut self, store: Arc<AdventureMapPoiStore>) {
        self.adventure_map_poi_store = Some(store);
    }
    #[cfg(test)]
    pub fn adventure_map_poi_store(&self) -> Option<&Arc<AdventureMapPoiStore>> {
        self.adventure_map_poi_store.as_ref()
    }
    pub fn set_map_store(&mut self, store: Arc<MapStore>) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.maps.store = Some(store);
    }
    pub(crate) fn map_store(&self) -> Option<&Arc<MapStore>> {
        self.maps.store.as_ref()
    }
    #[cfg(test)]
    pub(crate) fn represented_reveal_world_map_overlay_criteria_like_cpp(&self) -> &[u32] {
        &self.represented_reveal_world_map_overlay_criteria_like_cpp
    }
    pub(in crate::session) fn player_cannot_enter_target_map_like_cpp(
        &self,
        map_id: u32,
    ) -> Option<(u32, u8, i32)> {
        let Some(map_store) = self.maps.store.as_ref() else {
            return None;
        };
        let Some(map_entry) = map_store.get(map_id).copied() else {
            return Some((TRANSFER_ABORT_MAP_NOT_ALLOWED_LIKE_CPP, 0, 0));
        };
        if !map_entry.is_dungeon() {
            return None;
        }

        let player_guid = self.player_guid?;
        let player = self.create_map_player_context_like_cpp(map_id, map_entry, player_guid)?;
        let requested_difficulty = player
            .group
            .map(|group| group.difficulty_id)
            .unwrap_or(player.player_difficulty_id);

        if self
            .create_map_db2_entries_like_cpp(map_id, requested_difficulty)
            .is_none()
        {
            return Some((TRANSFER_ABORT_DIFFICULTY_LIKE_CPP, 0, 0));
        }

        if self.player_is_game_master_like_cpp() == Some(true) {
            return None;
        }

        if let Some(abort) =
            self.access_requirement_abort_like_cpp(map_id, requested_difficulty as u8)
        {
            return Some(abort);
        }

        if map_entry.instance_type == wow_data::map::MAP_RAID
            && map_entry.expansion_like_cpp() >= self.server_expansion_like_cpp
            && !self.instance_ignore_raid_like_cpp
            && !self.current_player_is_in_raid_group_like_cpp()
        {
            return Some((TRANSFER_ABORT_NEED_GROUP_LIKE_CPP, 0, 0));
        }

        let Some(canonical_map_manager) = self.canonical_map_manager.as_ref() else {
            return None;
        };
        let entry = wow_map::CreateMapEntryContext {
            map_id,
            kind: wow_map::CreateMapEntryKind::Dungeon,
            split_by_faction: map_entry.is_split_by_faction(),
            flex_locking: map_entry.is_flex_locking(),
        };
        let active_instance_lock =
            self.create_map_active_instance_lock_context_like_cpp(map_id, requested_difficulty);
        let mut manager = canonical_map_manager.lock().ok()?;
        let decision = manager.create_map_decision_like_cpp(
            Some(entry),
            Some(player),
            |candidate_map_id, difficulty_id| {
                self.create_map_difficulty_context_like_cpp(candidate_map_id, difficulty_id)
            },
            active_instance_lock,
            |_, _| None,
        );

        let existing_instance_lock_context = match &decision {
            wow_map::CreateMapDecision::Existing { key, .. } => manager
                .find_map(key.map_id, key.instance_id)
                .and_then(|map| map.instance_lock_context()),
            _ => None,
        };
        let existing_instance_player_count = match &decision {
            wow_map::CreateMapDecision::Existing { key, .. } => manager
                .find_map(key.map_id, key.instance_id)
                .map(|map| map.players_count_except_gms_like_cpp()),
            _ => None,
        };
        let existing_instance_encounter_in_progress = match &decision {
            wow_map::CreateMapDecision::Existing { key, .. } => manager
                .find_map(key.map_id, key.instance_id)
                .map(|map| map.instance_encounter_in_progress_like_cpp()),
            _ => None,
        };
        let decision_key = create_map_decision_key_like_cpp(&decision);
        drop(manager);

        if let wow_map::CreateMapDecision::Existing {
            key, difficulty_id, ..
        } = &decision
        {
            if let Some(player_count) = existing_instance_player_count
                && let Some(entries) =
                    self.create_map_db2_entries_like_cpp(key.map_id, *difficulty_id)
                && player_count >= entries.max_players
            {
                return Some((TRANSFER_ABORT_MAX_PLAYERS_LIKE_CPP, 0, 0));
            }

            if map_entry.instance_type == wow_data::map::MAP_RAID
                && self.player_loading() != Some(player_guid)
                && existing_instance_encounter_in_progress == Some(true)
            {
                return Some((TRANSFER_ABORT_ZONE_IN_COMBAT_LIKE_CPP, 0, 0));
            }

            if let Some(lock_context) = existing_instance_lock_context {
                let deny_reason = self
                    .cannot_enter_existing_instance_lock_like_cpp(
                        key.map_id,
                        *difficulty_id,
                        lock_context,
                    )
                    .unwrap_or(wow_instances::TransferAbortReason::None);
                if deny_reason != wow_instances::TransferAbortReason::None {
                    return Some((deny_reason as u32, 0, 0));
                }
            }
        }

        if !map_entry.ignores_instance_farm_limit_like_cpp()
            && let Some(key) = decision_key
            && !self.check_instance_count_probe_like_cpp(key.instance_id)
            && self.resolved_player_is_alive_like_cpp() == Some(true)
        {
            return Some((TRANSFER_ABORT_TOO_MANY_INSTANCES_LIKE_CPP, 0, 0));
        }

        None
    }
    pub(crate) fn is_disabled_map_type_for_player_like_cpp(
        &self,
        disable_type: u32,
        map_id: u32,
    ) -> bool {
        let Some(disable_mgr) = self.disable_mgr() else {
            return false;
        };
        let Some(map_store) = self.map_store() else {
            return false;
        };

        let current_map_id = u32::from(self.player_map_id_like_cpp());
        let Some((_, area_id)) = self.player_zone_area_like_cpp() else {
            return true;
        };
        let current_map_instance_type = map_store
            .get(current_map_id)
            .map(|entry| entry.instance_type);

        disable_mgr.is_disabled_for_like_cpp(
            disable_type,
            map_id,
            Some(DisableWorldObjectRefLikeCpp {
                type_id: TypeId::Player,
                map_id: current_map_id,
                area_id,
                is_pet: false,
                is_battle_arena: current_map_instance_type == Some(MAP_ARENA_LIKE_CPP),
                is_battleground: current_map_instance_type == Some(MAP_BATTLEGROUND_LIKE_CPP),
                player_map_difficulty: None,
            }),
            0,
            Some(map_store.as_ref()),
        )
    }
    pub(in crate::session) fn is_map_disabled_for_player_like_cpp(&self, map_id: u32) -> bool {
        self.is_disabled_map_type_for_player_like_cpp(DISABLE_TYPE_MAP, map_id)
    }
    pub(crate) fn player_map_id_like_cpp(&self) -> u16 {
        self.current_map_id
    }
    pub(crate) fn handle_under_map_like_cpp(
        &mut self,
        movement_info: &wow_packet::packets::movement::MovementInfo,
    ) -> Option<MovementUnderMapDamageEvent> {
        let min_height = self.player_min_height_like_cpp(movement_info.position);
        if movement_info.position.z >= min_height {
            #[cfg(test)]
            {
                self.player_out_of_bounds_like_cpp = false;
            }
            return None;
        }

        let (original_health, max_health, player_is_alive) =
            self.resolved_player_vitals_like_cpp()?;
        if !player_is_alive {
            return None;
        }

        #[cfg(test)]
        {
            self.player_out_of_bounds_like_cpp = true;
        }
        let damage = max_health;
        let (_, health_after, _, _, killed_player) =
            self.apply_owned_player_damage_like_cpp(damage, wow_constants::DeathState::JustDied)?;
        if health_after != original_health
            && let Some(player_guid) = self.player_guid()
        {
            self.send_player_health_update_like_cpp(player_guid, u64::from(health_after));
            self.send_environmental_damage_log_like_cpp(
                player_guid,
                DAMAGE_FALL_TO_VOID_LIKE_CPP,
                damage,
                0,
                0,
            );
            if killed_player {
                self.send_player_health_values_update_like_cpp(player_guid, 0);
            }
        }

        // C++ calls KillPlayer if EnvironmentalDamage did not kill due to GM/immunity.
        if self.resolved_player_is_alive_like_cpp() == Some(true) {
            self.set_player_alive_like_cpp(false);
        } else {
            self.sync_player_registry_state_like_cpp();
        }

        let event = MovementUnderMapDamageEvent {
            z: movement_info.position.z,
            min_height,
            damage,
        };
        #[cfg(test)]
        self.under_map_damage_events_like_cpp.push(event);
        Some(event)
    }
    #[cfg(test)]
    pub(crate) fn under_map_damage_events_like_cpp(&self) -> &[MovementUnderMapDamageEvent] {
        &self.under_map_damage_events_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn set_taxi_node_map_id_like_cpp(&mut self, node_id: u32, map_id: u16) {
        self.taxi_node_map_ids_like_cpp.insert(node_id, map_id);
    }
    /// The legacy map facade must follow the same map instance that owns the
    /// canonical Player. Instance `0` remains only the bootstrap fallback for
    /// tests/runtime phases where no canonical Player has been materialized.
    pub(crate) fn current_legacy_runtime_map_key_like_cpp(&self) -> (u16, u32) {
        let fallback_map_id = self.player_map_id_like_cpp();
        let Some(map_key) = self.current_canonical_player_map_key_like_cpp() else {
            return (fallback_map_id, 0);
        };
        let Ok(map_id) = u16::try_from(map_key.map_id) else {
            return (fallback_map_id, 0);
        };
        (map_id, map_key.instance_id)
    }
    /// Consume the last map-owned represented `Map::SendObjectUpdates` stable
    /// DynamicObject VALUES snapshot into this session's outbound packet stream.
    ///
    /// Source of truth remains canonical `Map::map_objects` as snapshotted by
    /// `ManagedMap::last_send_object_updates_summary_like_cpp()`. This helper
    /// gates snapshot delivery through represented direct Player,
    /// PlayerMapType/CreatureMapType shared-vision, or DynamicObjectMapType
    /// receiver-source evidence before the final session `HaveAtClient`
    /// visibility check, and never reads live DynamicObject changed masks or
    /// mutates canonical map state.
    pub(crate) fn send_represented_dynamic_object_values_updates_from_last_map_send_object_updates_like_cpp(
        &mut self,
    ) -> usize {
        let Some(key) = self.current_canonical_player_map_key_like_cpp() else {
            return 0;
        };
        let Ok(packet_map_id) = u16::try_from(key.map_id) else {
            return 0;
        };
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return 0;
        };
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let represented_seer_guid = self.represented_seer_guid_like_cpp;
        let (update_generation, updates) = {
            let Ok(manager) = manager.lock() else {
                return 0;
            };
            let Some(managed_map) = manager.find_map(key.map_id, key.instance_id) else {
                return 0;
            };
            let map = managed_map.map();
            let Some(player) = map.get_typed_player(player_guid) else {
                return 0;
            };
            if !player.unit().world().object().is_in_world() {
                return 0;
            }
            let player_world = player.unit().world();
            let player_phase_shift = player_world.phase_shift().clone();
            let player_position = player_world.position();
            let visibility_range = map.visibility_range();
            let shared_vision_source_guids = map
                .typed_combat_unit_guids_like_cpp()
                .into_iter()
                .filter(|source_guid| *source_guid != player_guid)
                .filter(|source_guid| {
                    map.get_typed_player(*source_guid).is_some_and(|source| {
                        source.unit().world().object().is_in_world()
                            && source
                                .unit()
                                .subsystems()
                                .control
                                .shared_vision_guids
                                .contains(&player_guid)
                    }) || map
                        .with_creature_like_cpp(*source_guid, |source| {
                            source.unit().world().object().is_in_world()
                                && source
                                    .unit()
                                    .subsystems()
                                    .control
                                    .shared_vision_guids
                                    .contains(&player_guid)
                        })
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>();
            let dynamic_object_seer_guid = represented_seer_guid.filter(|seer_guid| {
                if !seer_guid.is_dynamic_object() {
                    return false;
                }
                let Some(seer) = map.get_typed_dynamic_object(*seer_guid) else {
                    return false;
                };
                if !seer.world().object().is_in_world() {
                    return false;
                }
                let caster_guid = seer.bound_caster().unwrap_or_else(|| seer.caster_guid());
                caster_guid == player_guid && caster_guid.is_player()
            });

            let updates = managed_map
                .last_send_object_updates_summary_like_cpp()
                .dynamic_object_values_updates
                .into_iter()
                .filter(|represented_update| {
                    let guid = represented_update.guid;
                    if !guid.is_dynamic_object() {
                        return false;
                    }
                    let Some(updated) = map.get_typed_dynamic_object(guid) else {
                        return false;
                    };
                    if !updated.world().object().is_in_world() {
                        return false;
                    }
                    // C++ visibility distance is 2D (CanSeeOrDetect -> GetSightRange ->
                    // IsWithinDist(obj, range, is3D=false); Object.cpp:1587-1609). Mirror the
                    // already-2D entry filters and the creature/GameObject values fanout.
                    let updated_world = updated.world();
                    let direct_player_allows = player_phase_shift
                        .can_see(updated_world.phase_shift())
                        && updated_world
                            .position()
                            .is_within_dist_2d(&player_position, visibility_range);
                    if direct_player_allows {
                        return true;
                    }
                    if shared_vision_source_guids.iter().any(|source_guid| {
                        if let Some(source) = map.get_typed_player(*source_guid) {
                            let source_world = source.unit().world();
                            source_world
                                .phase_shift()
                                .can_see(updated_world.phase_shift())
                                && updated_world
                                    .position()
                                    .is_within_dist_2d(&source_world.position(), visibility_range)
                        } else {
                            map.with_creature_like_cpp(*source_guid, |source| {
                                let source_world = source.unit().world();
                                source_world
                                    .phase_shift()
                                    .can_see(updated_world.phase_shift())
                                    && updated_world.position().is_within_dist_2d(
                                        &source_world.position(),
                                        visibility_range,
                                    )
                            })
                            .unwrap_or(false)
                        }
                    }) {
                        return true;
                    }
                    dynamic_object_seer_guid.is_some_and(|seer_guid| {
                        map.get_typed_dynamic_object(seer_guid).is_some_and(|seer| {
                            let seer_world = seer.world();
                            seer_world
                                .phase_shift()
                                .can_see(updated_world.phase_shift())
                                && updated_world
                                    .position()
                                    .is_within_dist_2d(&seer_world.position(), visibility_range)
                        })
                    })
                })
                .collect::<Vec<_>>();
            (managed_map.update_calls().len() as u64, updates)
        };

        use wow_packet::ServerPacket;

        let mut sent = 0;
        for represented_update in updates {
            let guid = represented_update.guid;
            if !self.client_visible_guids_like_cpp.contains(&guid) {
                continue;
            }
            let Some(update) = dynamic_object_values_update_to_update_object(
                guid,
                packet_map_id,
                &represented_update.values_update,
            ) else {
                continue;
            };
            let bytes = update.to_bytes();
            let fingerprint =
                crate::session_rules::represented_dynamic_object_values_update_delivery_fingerprint_like_cpp(
                    guid, &bytes,
                );
            if !self
                .represented_dynamic_object_values_updates_delivered_like_cpp
                .insert((
                    key.map_id,
                    key.instance_id,
                    update_generation,
                    guid,
                    fingerprint,
                ))
            {
                continue;
            }
            self.send_packet(&update);
            sent += 1;
        }
        sent
    }
}
