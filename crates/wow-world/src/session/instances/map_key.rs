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
                            self.core
                                .directory
                                .group_registry
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
                            let (s, h) = crate::session::split_instances_ref(self);
                            s.create_instance_lock_for_new_instance_side_effect_like_cpp(
                                h,
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
            .and_then(|group_guid| {
                self.core
                    .directory
                    .group_registry
                    .as_ref()?
                    .get(&group_guid)
            })
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
            team_id: player_team_id_for_race_cpp(
                crate::session::hub_ref(self).player_race_like_cpp(),
            ),
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
        let (state, hub) = crate::session::split_instances_ref(self);
        state.create_map_db2_entries_like_cpp(hub, map_id, difficulty_id)
    }
    pub fn set_vmap_indoor_check_like_cpp(&mut self, enabled: bool) {
        self.config.vmap_indoor_check_like_cpp = enabled;
    }
    pub fn set_mmap_runtime_config_like_cpp(&mut self, config: MMapRuntimeConfigLikeCpp) {
        self.config.mmap_runtime_config_like_cpp = config;
    }
    pub fn set_map_store(&mut self, store: Arc<MapStore>) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.catalogs.maps.store = Some(store);
    }
    /// Set the immutable C++ `sDungeonEncounterStore` catalog used by
    /// `Player::IsLockedToDungeonEncounter`.
    pub fn set_dungeon_encounter_store(&mut self, store: Arc<DungeonEncounterStore>) {
        self.catalogs.dungeon_encounter_store = Some(store);
    }
    pub(in crate::session) fn player_cannot_enter_target_map_like_cpp(
        &self,
        map_id: u32,
    ) -> Option<(u32, u8, i32)> {
        let Some(map_store) = self.catalogs.maps.store.as_ref() else {
            return None;
        };
        let Some(map_entry) = map_store.get(map_id).copied() else {
            return Some((TRANSFER_ABORT_MAP_NOT_ALLOWED_LIKE_CPP, 0, 0));
        };
        if !map_entry.is_dungeon() {
            return None;
        }

        let player_guid = self.core.player_guid?;
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

        if crate::session::hub_ref(self).player_is_game_master_like_cpp() == Some(true) {
            return None;
        }

        if let Some(abort) =
            self.access_requirement_abort_like_cpp(map_id, requested_difficulty as u8)
        {
            return Some(abort);
        }

        if map_entry.instance_type == wow_data::map::MAP_RAID
            && map_entry.expansion_like_cpp() >= self.core.realm_policy.server_expansion_like_cpp
            && !self.core.realm_policy.instance_ignore_raid_like_cpp
            && !self.current_player_is_in_raid_group_like_cpp()
        {
            return Some((TRANSFER_ABORT_NEED_GROUP_LIKE_CPP, 0, 0));
        }

        let Some(canonical_map_manager) = self.core.canonical_map_manager.as_ref() else {
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
            && crate::session::hub_ref(self).resolved_player_is_alive_like_cpp() == Some(true)
        {
            return Some((TRANSFER_ABORT_TOO_MANY_INSTANCES_LIKE_CPP, 0, 0));
        }

        None
    }
    pub(in crate::session) fn is_map_disabled_for_player_like_cpp(&self, map_id: u32) -> bool {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.is_map_disabled_for_player_like_cpp(hub, map_id)
    }
    #[cfg(test)]
    pub(crate) fn under_map_damage_events_like_cpp(&self) -> &[MovementUnderMapDamageEvent] {
        &self.fixtures.movement.under_map_damage_events_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn set_taxi_node_map_id_like_cpp(&mut self, node_id: u32, map_id: u16) {
        self.view.taxi_node_map_ids_like_cpp.insert(node_id, map_id);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/instances/map_key/f3_shims.rs"]
mod f3_shims;
