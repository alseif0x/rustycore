// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Owned loot claims and their leases.

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use wow_database::{CharStatements, SqlTransaction};

use super::*;

impl WorldSession {
    pub(super) fn creature_loot_release_values_for_viewer_like_cpp(
        &self,
        creature_guid: ObjectGuid,
        viewer_guid: ObjectGuid,
        viewer_has_pending_bind: bool,
        authority: Option<&OwnedLootAuthority>,
        mut update: wow_packet::packets::update::UnitDataValuesDeltaUpdate,
    ) -> wow_packet::packets::update::UnitDataValuesDeltaUpdate {
        let Some(object_data) = update.object_data.as_mut() else {
            return update;
        };
        if object_data.dynamic_flags & UnitDynFlags::Lootable as u32 == 0 {
            return update;
        }
        let Some(authority) = authority else {
            // The authority-less path exists only for bounded unit fixtures.
            // Preserve the canonical flag rather than inventing per-viewer
            // ownership without `Creature::GetLootForPlayer` evidence.
            return update;
        };

        // C++ `ViewerDependentValue<ObjectData::DynamicFlags>` removes
        // UNIT_DYNFLAG_LOOTABLE when the complete `Player::isAllowedToLoot`
        // predicate is false. The object-owned authority is the Rust
        // equivalent of `Creature::GetLootForPlayer`; one exhausted personal
        // pool must not hide a different player's still-live pool.
        let creature_is_dead =
            self.represented_creature_is_dead_for_loot_visibility_like_cpp(creature_guid);
        let viewer_can_still_loot = authority
            .snapshot_for_player_like_cpp(viewer_guid)
            .is_some_and(|snapshot| {
                creature_loot_is_allowed_to_player_like_cpp(
                    creature_is_dead,
                    viewer_has_pending_bind,
                    &snapshot.loot,
                    viewer_guid,
                )
            });
        if !viewer_can_still_loot {
            object_data.dynamic_flags &= !(UnitDynFlags::Lootable as u32);
        }
        update
    }

    /// Publishes the dirty DynamicFlags field created by C++
    /// `WorldSession::DoLootRelease` to every same-map session that currently
    /// has the creature at the client. The canonical object mutation alone is
    /// insufficient until the global `Map::SendObjectUpdates` bridge owns
    /// normal VALUES fanout.
    pub(super) fn send_creature_loot_release_dynamic_flags_update_like_cpp(
        &self,
        creature_guid: ObjectGuid,
        values_update: &wow_entities::UnitValuesUpdate,
        authority: Option<&OwnedLootAuthority>,
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(packet_update) =
            crate::entity_update_bridge::unit_values_update_to_packet(values_update)
        else {
            return 0;
        };
        let map_id = self.player_map_id_like_cpp();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let mut sent = 0;

        if self.client_visible_guids_like_cpp.contains(&creature_guid) {
            let source_update = self.creature_loot_release_values_for_viewer_like_cpp(
                creature_guid,
                player_guid,
                self.pending_bind.is_some(),
                authority,
                packet_update.clone(),
            );
            self.send_packet(&UpdateObject::unit_values_update(
                creature_guid,
                map_id,
                source_update,
            ));
            sent += 1;
        }

        let Some(registry) = self.player_registry() else {
            return sent;
        };
        let recipients = registry.same_map_loot_recipients(player_guid, map_id, instance_id);
        for registration in recipients {
            // C++'s dirty-field pass cannot silently lose this forced update.
            // Do not retain a DashMap guard (or any map/authority lock) while
            // queueing the bounded target-session command rail.
            if registry.queue_current_command_reliably(
                registration,
                SessionCommand::SendCreatureLootReleaseValuesUpdateLikeCpp(
                    SendCreatureLootReleaseValuesUpdateLikeCppCommand {
                        creature_guid,
                        map_id,
                        instance_id,
                        unit_values_update: packet_update.clone(),
                        authority: authority.cloned(),
                    },
                ),
            ) != crate::session::directory::PlayerDirectoryReliableSendOutcome::StaleOrDisconnected
            {
                sent += 1;
            }
        }

        sent
    }

    pub(super) fn record_represented_gameobject_chest_release_metadata_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        source: GameObjectLootSource,
    ) {
        let state = self
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        state.go_type = Some(GAMEOBJECT_TYPE_CHEST as u8);
        state.chest_restock_time_secs = Some(source.chest_restock_time_secs);
        state.chest_consumable = Some(source.chest_consumable);
        state.despawn_at_action = source.chest_consumable;
        state.chest_loot_source = Some(source);
        state.chest_personal_loot_id = Some(source.personal_loot_id);
        state.linked_trap_entry =
            (source.linked_trap_entry != 0).then_some(source.linked_trap_entry);
    }

    /// Clone the object-owned authority while the map/entity lock is held,
    /// then release that lock before any reservation can await.
    pub(super) fn represented_owned_loot_authority_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        if owner_guid.is_creature_or_vehicle() {
            // The legacy and canonical maps deliberately use separate locks.
            // Reconcile optimistically with object-local compare/exchange;
            // blind rebinding can otherwise clobber a newer respawn between
            // the read and write phases.
            for _ in 0..8 {
                let canonical_player_map_key = self.current_canonical_player_map_key_like_cpp();
                let map_key = canonical_player_map_key
                    .or_else(|| {
                        self.canonical_object_lookup_map_key_like_cpp(u32::from(
                            self.player_map_id_like_cpp(),
                        ))
                    })
                    .unwrap_or_else(|| {
                        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
                        wow_map::MapKey::new(u32::from(map_id), instance_id)
                    });
                let map_key_still_valid = |session: &Self| {
                    session.loot_reconciliation_map_key_still_valid_like_cpp(
                        map_key,
                        canonical_player_map_key.is_some(),
                    )
                };
                let legacy =
                    self.read_legacy_creature_loot_authority_on_map_like_cpp(owner_guid, map_key);
                let canonical = self
                    .read_canonical_creature_loot_authority_on_map_like_cpp(owner_guid, map_key);
                let (legacy, canonical) = match (legacy, canonical) {
                    (Some(legacy), Some(canonical)) => (legacy, canonical),
                    (None, None) => return None,
                    (Some(authority), None) | (None, Some(authority)) => {
                        if !map_key_still_valid(self) {
                            continue;
                        }
                        return Some(authority);
                    }
                };
                if !map_key_still_valid(self) {
                    continue;
                }

                let legacy_stamp = legacy.stamp_like_cpp();
                let canonical_stamp = canonical.stamp_like_cpp();
                let selected = crate::session::reconcile_creature_loot_authority_mirrors_like_cpp(
                    &canonical,
                    canonical_stamp,
                    &legacy,
                    legacy_stamp,
                );
                if !map_key_still_valid(self) {
                    continue;
                }
                if self
                    .rebind_canonical_creature_loot_authority_on_map_like_cpp(
                        owner_guid,
                        map_key,
                        &canonical,
                        canonical_stamp,
                        selected.clone(),
                    )
                    .is_none()
                {
                    continue;
                }
                if !map_key_still_valid(self) {
                    continue;
                }
                if self
                    .rebind_legacy_creature_loot_authority_on_map_like_cpp(
                        owner_guid,
                        map_key,
                        &legacy,
                        legacy_stamp,
                        selected.clone(),
                    )
                    .is_none()
                {
                    continue;
                }

                if !map_key_still_valid(self) {
                    continue;
                }
                let converged_legacy = self
                    .read_legacy_creature_loot_authority_on_map_like_cpp(owner_guid, map_key)
                    .is_some_and(|authority| authority.shares_storage_like_cpp(&selected));
                let converged_canonical = self
                    .read_canonical_creature_loot_authority_on_map_like_cpp(owner_guid, map_key)
                    .is_some_and(|authority| authority.shares_storage_like_cpp(&selected));
                if converged_legacy && converged_canonical {
                    return Some(selected);
                }
            }

            // Continuous concurrent replacement is safer as a failed request
            // than as an overwrite of the newest mirror.
            return None;
        }

        if owner_guid.is_game_object() {
            let canonical_player_map_key = self.current_canonical_player_map_key_like_cpp();
            let map_key = canonical_player_map_key.or_else(|| {
                self.canonical_object_lookup_map_key_like_cpp(u32::from(
                    self.player_map_id_like_cpp(),
                ))
            })?;
            let authority =
                self.read_canonical_gameobject_loot_authority_on_map_like_cpp(owner_guid, map_key)?;
            let still_valid = self.loot_reconciliation_map_key_still_valid_like_cpp(
                map_key,
                canonical_player_map_key.is_some(),
            );
            return still_valid.then_some(authority);
        }

        None
    }

    /// Bridge pre-authority represented fixtures (and the equivalent first
    /// live generation) into the object-owned source of truth exactly once.
    /// A retired non-zero generation is never reinstalled from session cache.
    pub(super) fn prepare_owned_loot_authority_for_active_request_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        scope_player: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        let authority = self.represented_owned_loot_authority_like_cpp(owner_guid)?;
        let can_install_first_generation = represented_local_loot_fixture_allowed_like_cpp()
            && authority.is_retired_like_cpp()
            && authority.generation_like_cpp() == 0
            && self.loot_table.contains_key(&owner_guid)
            && (self.active_loot_view_owners.contains(&owner_guid)
                || self.is_active_loot_guid(owner_guid));
        if !can_install_first_generation {
            return Some(authority);
        }

        if owner_guid.is_game_object() {
            let _ = self
                .sync_represented_gameobject_loot_to_canonical_like_cpp(owner_guid, scope_player);
        } else if owner_guid.is_creature_or_vehicle() {
            let _ =
                self.sync_represented_creature_loot_to_canonical_like_cpp(owner_guid, scope_player);
        }

        let authority = self.represented_owned_loot_authority_like_cpp(owner_guid)?;
        if let Some(snapshot) = authority.snapshot_for_player_like_cpp(scope_player) {
            self.active_loot_view_generations_like_cpp
                .entry(owner_guid)
                .or_insert(snapshot.generation);
            self.active_loot_view_authorities_like_cpp
                .entry(owner_guid)
                .or_insert_with(|| authority.clone());
        }
        Some(authority)
    }

    /// Rebuild every session-local field derived from one authoritative
    /// snapshot. In particular, a reopened personal creature view must restore
    /// its personal-owner marker and per-player money mirror; restoring only
    /// `loot_table` would make the same pool behave as shared loot.
    pub(super) fn cache_represented_owned_loot_snapshot_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        _requested_player_guid: ObjectGuid,
        snapshot: OwnedLootSnapshot,
    ) {
        let OwnedLootSnapshot {
            generation,
            scope,
            loot,
        } = snapshot;
        // One WorldSession caches exactly one selected pool for an owner.
        // Generation scratch may have populated money entries for every
        // encounter tapper, but those peer pools now live in the authority;
        // retaining their session-local markers can misclassify a later
        // shared snapshot as personal loot.
        self.represented_personal_loot_money
            .retain(|(owner, _), _| *owner != owner_guid);
        self.represented_personal_loot_owners.remove(&owner_guid);
        match scope {
            OwnedLootScope::Personal(scope_player_guid) => {
                self.represented_personal_loot_owners.insert(owner_guid);
                self.represented_personal_loot_money
                    .insert((owner_guid, scope_player_guid), loot.coins);
            }
            OwnedLootScope::Shared => {}
        }
        self.loot_table.insert(owner_guid, loot);
        self.represented_loot_cache_generations_like_cpp
            .insert(owner_guid, generation);
    }

    pub(super) fn refresh_owned_loot_summary_like_cpp(&mut self, owner_guid: ObjectGuid) {
        if owner_guid.is_creature_or_vehicle() {
            if let Some(authority) = self.represented_owned_loot_authority_like_cpp(owner_guid) {
                let _ = self.rebind_legacy_creature_loot_authority_like_cpp(
                    owner_guid,
                    &authority,
                    authority.stamp_like_cpp(),
                    authority.clone(),
                );
                let authority_stamp = authority.stamp_like_cpp();
                let _ = self.rebind_canonical_creature_loot_authority_like_cpp(
                    owner_guid,
                    &authority,
                    authority_stamp,
                    authority.clone(),
                );
            }
        } else if owner_guid.is_game_object() {
            if let Some(authority) = self.represented_owned_loot_authority_like_cpp(owner_guid) {
                let _ = self.rebind_canonical_gameobject_loot_authority_like_cpp(
                    owner_guid,
                    &authority,
                    authority.stamp_like_cpp(),
                    authority.clone(),
                );
            }
        }
    }

    pub(super) fn represented_active_loot_claim_generation_matches_like_cpp(
        &self,
        owner_guid: ObjectGuid,
        claim: &LootClaimLease,
    ) -> bool {
        self.active_loot_view_authorities_like_cpp
            .get(&owner_guid)
            .is_some_and(|opened| claim.shares_authority_like_cpp(opened))
            && self
                .active_loot_view_generations_like_cpp
                .get(&owner_guid)
                .is_some_and(|opened| *opened == claim.generation_like_cpp())
    }

    pub(super) fn apply_represented_gameobject_loot_release_like_cpp(
        &mut self,
        guid: ObjectGuid,
        player_guid: ObjectGuid,
        selected_pool_looted: bool,
        mut whole_object_fully_looted: bool,
        authoritative_release: Option<&AuthoritativeLootReleaseLikeCpp>,
    ) {
        let go_type = self
            .represented_gameobject_use_states
            .get(&guid)
            .and_then(|state| state.go_type)
            .map(u32::from);
        let represented_chest_restock_time_secs = self
            .represented_gameobject_use_states
            .get(&guid)
            .and_then(|state| state.chest_restock_time_secs)
            .unwrap_or_default();
        let represented_personal_loot_uses_after_release = self
            .represented_gameobject_use_states
            .get(&guid)
            .map(|state| state.personal_loot_uses.saturating_add(1))
            .unwrap_or(1);
        // C++ `FishingHole.MaxOpens` is still template evidence from the represented
        // GO value; the use counter source-of-truth is canonical `GameObject::use_times`
        // when the canonical GameObject can be mutated.
        let represented_fishing_hole_max_opens = self
            .represented_gameobject_use_states
            .get(&guid)
            .and_then(|state| state.fishing_hole_max_opens);
        let canonical_fishing_hole_release = (go_type == Some(GAMEOBJECT_TYPE_FISHING_HOLE))
            .then(|| {
                self.release_canonical_fishing_hole_like_cpp(
                    guid,
                    represented_fishing_hole_max_opens,
                )
            })
            .flatten();
        let canonical_fishing_hole_use_count_after_release = canonical_fishing_hole_release
            .as_ref()
            .map(|(use_count, _, _)| *use_count);

        let guarded_global_transition_attempted = selected_pool_looted
            && whole_object_fully_looted
            && !matches!(
                go_type,
                Some(GAMEOBJECT_TYPE_FISHING_NODE)
                    | Some(GAMEOBJECT_TYPE_FISHING_HOLE)
                    | Some(GAMEOBJECT_TYPE_GATHERING_NODE)
            )
            && authoritative_release.is_some();
        let guarded_global_transition = authoritative_release
            .filter(|_| guarded_global_transition_attempted)
            .and_then(|release| {
                if release.require_no_viewers {
                    self.set_canonical_gameobject_loot_state_if_unviewed_fully_looted_observation_like_cpp(
                        guid,
                        &release.authority,
                        release.object_generation,
                        release.lifecycle_revision,
                        LootState::JustDeactivated,
                        None,
                        represented_chest_restock_time_secs,
                        false,
                    )
                } else {
                    self.set_canonical_gameobject_loot_state_if_fully_looted_observation_like_cpp(
                        guid,
                        &release.authority,
                        release.object_generation,
                        release.lifecycle_revision,
                        LootState::JustDeactivated,
                        None,
                        represented_chest_restock_time_secs,
                        false,
                    )
                }
            });
        if guarded_global_transition_attempted && guarded_global_transition.is_none() {
            // An upsert/install/replacement won the serialization point after
            // close. Its new pool must keep the object globally active.
            whole_object_fully_looted = false;
        }

        let canonical_loot_state_request = match go_type {
            Some(GAMEOBJECT_TYPE_FISHING_NODE) => Some((LootState::JustDeactivated, None, false)),
            Some(GAMEOBJECT_TYPE_FISHING_HOLE) if canonical_fishing_hole_release.is_some() => None,
            Some(GAMEOBJECT_TYPE_FISHING_HOLE) => {
                let use_count_after_release = canonical_fishing_hole_use_count_after_release
                    .unwrap_or(represented_personal_loot_uses_after_release);
                let state = if represented_fishing_hole_max_opens
                    .is_some_and(|max_opens| use_count_after_release >= max_opens)
                {
                    LootState::JustDeactivated
                } else {
                    LootState::Ready
                };
                Some((state, None, false))
            }
            Some(GAMEOBJECT_TYPE_GATHERING_NODE) if selected_pool_looted => None,
            _ if guarded_global_transition_attempted => None,
            _ if selected_pool_looted && whole_object_fully_looted => {
                Some((LootState::JustDeactivated, None, false))
            }
            _ if selected_pool_looted => None,
            _ => Some((LootState::Activated, Some(player_guid), true)),
        };
        let requested_loot_state_outcome = canonical_loot_state_request.and_then(
            |(loot_state, unit_guid, shared_loot_is_changed_like_cpp)| {
                self.set_canonical_gameobject_loot_state_like_cpp(
                    guid,
                    loot_state,
                    unit_guid,
                    represented_chest_restock_time_secs,
                    shared_loot_is_changed_like_cpp,
                )
            },
        );
        let canonical_applied_loot_state = if guarded_global_transition.is_some() {
            Some((LootState::JustDeactivated, None))
        } else if let Some((_, state, _)) = canonical_fishing_hole_release.as_ref() {
            Some((*state, None))
        } else {
            canonical_loot_state_request.map(|(state, unit_guid, _)| (state, unit_guid))
        };
        let canonical_loot_state_updated = guarded_global_transition
            .as_ref()
            .or_else(|| {
                canonical_fishing_hole_release
                    .as_ref()
                    .map(|(_, _, outcome)| outcome)
            })
            .or(requested_loot_state_outcome.as_ref())
            .is_some_and(|outcome| {
                outcome.status == wow_map::map::GameObjectSetLootStateStatusLikeCpp::Updated
            });

        let state = self
            .represented_gameobject_use_states
            .entry(guid)
            .or_default();
        if canonical_loot_state_updated {
            if let Some((loot_state, unit_guid)) = canonical_applied_loot_state {
                state.loot_state = Some(loot_state);
                state.loot_state_unit_guid = unit_guid.unwrap_or(ObjectGuid::EMPTY);
                if loot_state == LootState::Activated
                    && go_type == Some(GAMEOBJECT_TYPE_CHEST)
                    && state.chest_consumable == Some(false)
                    && state.chest_restock_until.is_none()
                    && state
                        .chest_restock_time_secs
                        .is_some_and(|restock_time| restock_time != 0)
                {
                    let restock_secs = state.chest_restock_time_secs.unwrap_or_default();
                    state.chest_restock_until =
                        Some(Instant::now() + Duration::from_secs(u64::from(restock_secs)));
                }
            }
        } else {
            match go_type {
                Some(GAMEOBJECT_TYPE_FISHING_NODE) => {
                    state.loot_state = Some(LootState::JustDeactivated);
                    state.loot_state_unit_guid = ObjectGuid::EMPTY;
                }
                Some(GAMEOBJECT_TYPE_FISHING_HOLE) => {
                    state.personal_loot_uses = state.personal_loot_uses.saturating_add(1);
                    state.loot_state = if state
                        .fishing_hole_max_opens
                        .is_some_and(|max_opens| state.personal_loot_uses >= max_opens)
                    {
                        Some(LootState::JustDeactivated)
                    } else {
                        Some(LootState::Ready)
                    };
                    state.loot_state_unit_guid = ObjectGuid::EMPTY;
                }
                Some(GAMEOBJECT_TYPE_GATHERING_NODE) if selected_pool_looted => {}
                Some(GAMEOBJECT_TYPE_CHEST)
                    if selected_pool_looted
                        && whole_object_fully_looted
                        && state.chest_consumable == Some(false)
                        && state
                            .chest_personal_loot_id
                            .is_none_or(|loot_id| loot_id == 0)
                        && state
                            .chest_restock_time_secs
                            .is_some_and(|restock_time| restock_time != 0) =>
                {
                    let restock_secs = state.chest_restock_time_secs.unwrap_or_default();
                    state.loot_state = Some(LootState::NotReady);
                    state.loot_state_unit_guid = ObjectGuid::EMPTY;
                    state.chest_restock_until =
                        Some(Instant::now() + Duration::from_secs(u64::from(restock_secs)));
                }
                _ if selected_pool_looted && whole_object_fully_looted => {
                    state.loot_state = Some(LootState::JustDeactivated);
                    state.loot_state_unit_guid = ObjectGuid::EMPTY;
                }
                _ if selected_pool_looted => {}
                _ => {
                    state.loot_state = Some(LootState::Activated);
                    state.loot_state_unit_guid = player_guid;
                    if go_type == Some(GAMEOBJECT_TYPE_CHEST)
                        && state.chest_consumable == Some(false)
                        && state.chest_restock_until.is_none()
                        && state
                            .chest_restock_time_secs
                            .is_some_and(|restock_time| restock_time != 0)
                    {
                        let restock_secs = state.chest_restock_time_secs.unwrap_or_default();
                        state.chest_restock_until =
                            Some(Instant::now() + Duration::from_secs(u64::from(restock_secs)));
                    }
                }
            }
        }
        if canonical_loot_state_updated && go_type == Some(GAMEOBJECT_TYPE_FISHING_HOLE) {
            state.personal_loot_uses = canonical_fishing_hole_use_count_after_release
                .unwrap_or(represented_personal_loot_uses_after_release);
        }
        if go_type == Some(GAMEOBJECT_TYPE_GATHERING_NODE) && selected_pool_looted {
            state.go_state = Some(GoState::Active);
        }
        if go_type == Some(GAMEOBJECT_TYPE_CHEST)
            && selected_pool_looted
            && state.chest_consumable == Some(false)
            && state
                .chest_personal_loot_id
                .is_some_and(|loot_id| loot_id != 0)
        {
            let delay_secs = state
                .chest_restock_time_secs
                .filter(|restock_time| *restock_time != 0)
                .unwrap_or(wow_entities::DEFAULT_GAMEOBJECT_RESPAWN_DELAY_SECS);
            state.per_player_despawn_secs = Some(delay_secs);
            state.per_player_despawn_until =
                Some(Instant::now() + Duration::from_secs(u64::from(delay_secs)));
            state.per_player_state_player_guid = Some(player_guid);
        }
    }

    pub(super) fn hide_represented_gameobject_for_player_after_loot_release_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) {
        let Some(map_id) = self
            .represented_gameobject_use_states
            .get(&guid)
            .and_then(|state| state.per_player_despawn_until.map(|_| state.map_id))
            .flatten()
        else {
            return;
        };
        if !self.client_visible_guids_like_cpp.remove(&guid) {
            return;
        }
        self.send_packet(&UpdateObject::out_of_range_objects(vec![guid], map_id));
    }

    pub(super) fn send_gathering_node_loot_release_dynamic_flags_update_like_cpp(
        &self,
        guid: ObjectGuid,
    ) {
        if !self.client_visible_guids_like_cpp.contains(&guid) {
            return;
        }
        let Some(access) = self.canonical_gameobject_access_like_cpp(guid) else {
            return;
        };
        let Some(state) = self.represented_gameobject_use_states.get(&guid) else {
            return;
        };
        if state.go_type.map(u32::from) != Some(GAMEOBJECT_TYPE_GATHERING_NODE) {
            return;
        }
        let dynamic_flags =
            self.represented_gameobject_dynamic_flags_for_player_like_cpp(access.entry, state);
        let packet_update = wow_packet::packets::update::GameObjectDataValuesUpdate {
            changed_object_type_mask: 1 << wow_entities::TYPEID_OBJECT,
            object_data: Some(wow_packet::packets::update::ObjectDataValuesUpdate {
                changed_object_type_mask: 1 << wow_entities::TYPEID_OBJECT,
                object_data_mask: 0x05,
                entry_id: 0,
                dynamic_flags,
                scale: 0.0,
            }),
            game_object_data_mask: 0,
            state_world_effect_ids: Vec::new(),
            enable_doodad_sets: Vec::new(),
            enable_doodad_sets_update_mask: None,
            world_effects: Vec::new(),
            world_effects_update_mask: None,
            display_id: 0,
            spell_visual_id: 0,
            state_spell_visual_id: 0,
            spawn_tracking_state_anim_id: 0,
            spawn_tracking_state_anim_kit_id: 0,
            created_by: ObjectGuid::EMPTY,
            guild_guid: ObjectGuid::EMPTY,
            flags: 0,
            parent_rotation: [0.0; 4],
            faction_template: 0,
            level: 0,
            state: 0,
            type_id: 0,
            percent_health: 0,
            art_kit: 0,
            custom_param: 0,
        };
        let update = UpdateObject::game_object_values_update(
            guid,
            self.player_map_id_like_cpp(),
            packet_update,
        );
        self.send_packet(&update);
    }

    pub(crate) async fn do_loot_release_all_like_cpp(&mut self, player_guid: ObjectGuid) {
        let mut active_owners: Vec<ObjectGuid> =
            self.active_loot_view_owners.iter().copied().collect();
        if active_owners.is_empty() && !self.active_loot_guid.is_empty() {
            active_owners.push(self.active_loot_guid);
        }
        active_owners.sort_by_key(|guid| (guid.high_value(), guid.low_value()));

        for owner_guid in active_owners {
            self.do_loot_release_owner_like_cpp(owner_guid, player_guid)
                .await;
        }
    }

    pub(super) async fn do_loot_release_owner_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        if !self.active_loot_view_owners.contains(&owner_guid)
            && !self.is_active_loot_guid(owner_guid)
        {
            return false;
        }

        let authoritative_release = if let Some(authority) =
            self.prepare_owned_loot_authority_for_active_request_like_cpp(owner_guid, player_guid)
        {
            if !self
                .active_loot_view_authorities_like_cpp
                .get(&owner_guid)
                .is_some_and(|opened| opened.shares_storage_like_cpp(&authority))
            {
                self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
                return true;
            }
            let Some(active_generation) = self
                .active_loot_view_generations_like_cpp
                .get(&owner_guid)
                .copied()
            else {
                self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
                return true;
            };
            let Some(close) =
                authority.close_viewer_if_generation_like_cpp(active_generation, player_guid)
            else {
                self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
                return true;
            };
            Some(AuthoritativeLootReleaseLikeCpp {
                authority,
                selected_generation: active_generation,
                loot: close.snapshot.loot,
                whole_object_fully_looted: close.whole_object_fully_looted,
                whole_object_fully_skinned: close.whole_object_fully_skinned,
                object_generation: close.object_generation,
                lifecycle_revision: close.lifecycle_revision,
                require_no_viewers: false,
            })
        } else {
            None
        };

        if authoritative_release.is_none()
            && (owner_guid.is_creature_or_vehicle() || owner_guid.is_game_object())
            && !represented_local_loot_fixture_allowed_like_cpp()
        {
            self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
            return true;
        }

        // C++ `Loot::isLooted()` requires both zero gold and zero remaining
        // player-visible item count.
        let Some(loot) = authoritative_release
            .as_ref()
            .map(|release| &release.loot)
            .or_else(|| self.loot_table.get(&owner_guid))
        else {
            return false;
        };
        let selected_pool_looted = loot_is_looted_like_cpp(loot);
        let represented_loot_type = loot.loot_type;
        let whole_object_fully_looted = if let Some(release) = authoritative_release.as_ref() {
            release.whole_object_fully_looted
        } else if owner_guid.is_game_object() {
            self.canonical_gameobject_fully_looted_after_represented_sync_like_cpp(
                owner_guid,
                player_guid,
                selected_pool_looted,
            )
        } else if owner_guid.is_creature_or_vehicle() {
            self.canonical_creature_fully_looted_after_represented_sync_like_cpp(
                owner_guid,
                player_guid,
                selected_pool_looted,
            )
        } else {
            selected_pool_looted
        };

        if let Some(loot) = self.loot_table.get_mut(&owner_guid) {
            loot.players_looting.retain(|looter| *looter != player_guid);
        }

        // Acknowledge the release to the client.
        let release = SLootRelease {
            loot_obj: owner_guid,
            owner: player_guid,
        };
        self.send_packet(&release);

        if owner_guid.is_game_object() {
            self.clear_active_loot_guid_if(owner_guid);
            if !self
                .represented_gameobject_can_autostore_loot_item_like_cpp(owner_guid, player_guid)
            {
                if authoritative_release.is_some() {
                    self.discard_represented_personal_loot_cache_for_player_like_cpp(
                        owner_guid,
                        player_guid,
                    );
                }
                return true;
            }
            self.apply_represented_gameobject_loot_release_like_cpp(
                owner_guid,
                player_guid,
                selected_pool_looted,
                whole_object_fully_looted,
                authoritative_release.as_ref(),
            );
            let _ = self.queue_chest_gameobject_state_refresh_for_same_map_like_cpp(owner_guid);
            let go_type = self
                .represented_gameobject_use_states
                .get(&owner_guid)
                .and_then(|state| state.go_type)
                .map(u32::from);
            let selected_release_branch = selected_pool_looted
                || matches!(
                    go_type,
                    Some(GAMEOBJECT_TYPE_FISHING_NODE) | Some(GAMEOBJECT_TYPE_FISHING_HOLE)
                );
            if !selected_release_branch {
                if authoritative_release.is_some() {
                    self.discard_represented_personal_loot_cache_for_player_like_cpp(
                        owner_guid,
                        player_guid,
                    );
                }
                return true;
            }

            self.hide_represented_gameobject_for_player_after_loot_release_like_cpp(owner_guid);
            if go_type == Some(GAMEOBJECT_TYPE_GATHERING_NODE) {
                self.send_gathering_node_loot_release_dynamic_flags_update_like_cpp(owner_guid);
            }
            if authoritative_release.is_some() {
                self.discard_represented_personal_loot_cache_for_player_like_cpp(
                    owner_guid,
                    player_guid,
                );
            } else {
                self.loot_table.remove(&owner_guid);
            }
            return true;
        }

        if owner_guid.is_item()
            && matches!(
                represented_loot_type,
                LOOT_TYPE_PROSPECTING_LIKE_CPP | LOOT_TYPE_MILLING_LIKE_CPP
            )
        {
            // C++ always clears the generated Loot and consumes at most five
            // source items for prospecting/milling, even if the window closes
            // before every generated entry was taken.
            self.clear_active_loot_guid_if(owner_guid);
            self.loot_table.remove(&owner_guid);
            self.update_inventory_item_object_like_cpp(owner_guid, |item| {
                item.set_loot_generated(false);
            });
            self.destroy_direct_item_count_after_loot_release_like_cpp(owner_guid, Some(5))
                .await;
            return true;
        }

        if owner_guid.is_item() && !selected_pool_looted {
            self.clear_active_loot_guid_if(owner_guid);
            let item_has_loot_flag = self
                .inventory_items_like_cpp()
                .values()
                .find(|item| item.guid == owner_guid)
                .and_then(|item| self.item_template_flags(item.entry_id))
                .map(|flags| flags.contains(wow_constants::ItemFlags::HAS_LOOT));
            if item_has_loot_flag == Some(false) {
                self.destroy_fully_looted_direct_item(owner_guid).await;
            }
            return true;
        }

        self.clear_active_loot_guid_if(owner_guid);

        if !selected_pool_looted {
            let round_robin_released = if let Some(release) = authoritative_release.as_ref() {
                release
                    .authority
                    .clear_round_robin_if_generation_like_cpp(
                        release.selected_generation,
                        player_guid,
                    )
                    .is_some_and(|outcome| {
                        self.loot_table.insert(owner_guid, outcome.snapshot.loot);
                        outcome.cleared
                    })
            } else {
                self.loot_table.get_mut(&owner_guid).is_some_and(|loot| {
                    if loot.round_robin_player == player_guid {
                        loot.round_robin_player = ObjectGuid::EMPTY;
                        true
                    } else {
                        false
                    }
                })
            };
            if round_robin_released {
                self.represented_notify_loot_list_like_cpp(owner_guid);
            }
            if owner_guid.is_creature_or_vehicle() {
                let values_update = self.mutate_world_creature(owner_guid, |creature| {
                    creature.force_dynamic_flags_update_like_cpp();
                    creature.creature.unit().values_update()
                });
                if let Some(values_update) = values_update.as_ref() {
                    self.send_creature_loot_release_dynamic_flags_update_like_cpp(
                        owner_guid,
                        values_update,
                        authoritative_release
                            .as_ref()
                            .map(|release| &release.authority),
                    );
                }
            }
            if authoritative_release.is_some() {
                self.discard_represented_personal_loot_cache_for_player_like_cpp(
                    owner_guid,
                    player_guid,
                );
            }
            return true;
        }

        // Remove loot entry from memory once the represented loot is consumed.
        self.loot_table.remove(&owner_guid);

        if owner_guid.is_item() && selected_pool_looted {
            self.destroy_fully_looted_direct_item(owner_guid).await;
            return true;
        }

        if owner_guid.is_corpse() {
            self.remove_canonical_corpse_lootable_dynamic_flag_like_cpp(owner_guid);
            return true;
        }

        // C++ forces the viewer-dependent DynamicFlags field after every
        // creature release, including a selected personal pool that completed
        // while another pool remains.
        let forced_values_update = self.mutate_world_creature(owner_guid, |creature| {
            creature.force_dynamic_flags_update_like_cpp();
            creature.creature.unit().values_update()
        });

        if !whole_object_fully_looted {
            if let Some(values_update) = forced_values_update.as_ref() {
                self.send_creature_loot_release_dynamic_flags_update_like_cpp(
                    owner_guid,
                    values_update,
                    authoritative_release
                        .as_ref()
                        .map(|release| &release.authority),
                );
            }
            if authoritative_release.is_some() {
                self.discard_represented_personal_loot_cache_for_player_like_cpp(
                    owner_guid,
                    player_guid,
                );
            }
            return true;
        }

        let corpse_decay_looted_rate = self.loot_drop_rates_like_cpp().corpse_decay_looted;

        // Start corpse despawn timer if fully looted.
        let whole_object_fully_skinned = authoritative_release.as_ref().map_or(
            represented_loot_type == LOOT_TYPE_SKINNING_LIKE_CPP,
            |release| release.whole_object_fully_skinned,
        );
        let apply_lifecycle = |creature: &mut crate::map_manager::WorldCreature| {
            creature.remove_lootable_dynamic_flag_like_cpp();
            let marked = if !creature.is_alive() {
                let corpse_decay_secs = looted_corpse_decay_secs_like_cpp(
                    whole_object_fully_skinned,
                    creature.corpse_delay_secs_like_cpp(),
                    creature.ignore_corpse_decay_ratio_like_cpp(),
                    corpse_decay_looted_rate,
                );
                if !creature.all_loot_removed_from_corpse_like_cpp(
                    corpse_decay_looted_rate,
                    whole_object_fully_skinned,
                ) {
                    // C++ returns without resetting an already-expired
                    // corpse. The lifecycle mirror must remain expired too.
                    None
                } else {
                    Some((creature.entry(), corpse_decay_secs))
                }
            } else {
                None
            };
            (marked, creature.creature.unit().values_update())
        };
        let lifecycle_update = if let Some(release) = authoritative_release.as_ref() {
            self.mutate_world_creature_if_fully_looted_observation_like_cpp(
                owner_guid,
                &release.authority,
                release.object_generation,
                release.lifecycle_revision,
                apply_lifecycle,
            )
        } else {
            self.mutate_world_creature(owner_guid, apply_lifecycle)
        };

        if let Some((_, values_update)) = lifecycle_update.as_ref() {
            self.send_creature_loot_release_dynamic_flags_update_like_cpp(
                owner_guid,
                values_update,
                authoritative_release
                    .as_ref()
                    .map(|release| &release.authority),
            );
        }
        let marked = lifecycle_update.and_then(|(marked, _)| marked);

        if let Some((entry, corpse_decay_secs)) = marked {
            info!(
                "Creature {:?} (entry {}) fully looted — despawning in {}s",
                owner_guid, entry, corpse_decay_secs
            );
        }

        if authoritative_release.is_some() {
            self.discard_represented_personal_loot_cache_for_player_like_cpp(
                owner_guid,
                player_guid,
            );
        }

        true
    }

    pub(super) async fn store_claimed_direct_loot_item_from_owner_like_cpp(
        &mut self,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        claim: &LootClaimLease,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        self.store_direct_loot_item_with_source_like_cpp(
            loot_entry,
            dungeon_encounter_id,
            owner_guid.is_item().then_some(owner_guid),
            Some(claim),
            Some(LootItemClaimCommitContextLikeCpp {
                owner_guid,
                loot_obj,
                loot_list_id: loot_entry.loot_list_id,
                player_guid,
                free_for_all: loot_entry.flags.freeforall,
            }),
        )
        .await
    }

    pub(super) async fn destroy_direct_item_count_after_loot_release_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        maximum_destroy_count: Option<u32>,
    ) {
        let player_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };

        let runtime_item = self
            .inventory_item_objects_like_cpp()
            .get(&item_guid)
            .cloned();
        let (bag, slot) = match runtime_item.as_ref() {
            Some(item) => (item.bag_slot(), item.slot()),
            None => return,
        };

        let Some(item) = self.get_inventory_item_by_pos(bag, slot) else {
            return;
        };

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let current_count = runtime_item.as_ref().map_or(1, Item::count);
        let new_count =
            direct_item_count_after_loot_release_like_cpp(current_count, maximum_destroy_count);
        if new_count != 0 {
            let mut update_count = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_COUNT);
            update_count.set_u32(0, new_count);
            update_count.set_u64(1, item.db_guid);
            if let Err(error) = char_db.execute(&update_count).await {
                warn!(?error, "LootRelease: update partially consumed item failed");
                return;
            }
            self.update_inventory_item_object_like_cpp(item_guid, |item| {
                item.set_count(new_count);
                item.set_loot_generated(false);
            });
            self.sync_object_accessor_player();
            self.send_packet(&UpdateObject::item_stack_count_update(
                item_guid,
                self.player_map_id_like_cpp(),
                new_count,
            ));
            return;
        }

        let mut tx = SqlTransaction::new();
        let should_expire_refund = runtime_item
            .as_ref()
            .is_some_and(|item_object| item_object.is_refundable());
        if should_expire_refund {
            let mut del_refund = char_db.prepare(CharStatements::DEL_ITEM_REFUND_INSTANCE);
            del_refund.set_u64(0, item.db_guid);
            tx.append(del_refund);
        }

        let mut del_inv = char_db.prepare(CharStatements::DEL_CHAR_INVENTORY_ITEM);
        del_inv.set_u64(0, player_guid.counter() as u64);
        del_inv.set_u64(1, item.db_guid);
        tx.append(del_inv);

        let mut del_item = char_db.prepare(CharStatements::DEL_ITEM_INSTANCE);
        del_item.set_u64(0, item.db_guid);
        tx.append(del_item);

        if let Err(e) = char_db.commit_transaction(tx).await {
            warn!("LootRelease: delete fully looted item failed: {e}");
            return;
        }

        self.remove_fully_looted_runtime_item(bag, slot, item.guid);

        if should_expire_refund {
            self.send_packet(&ItemExpirePurchaseRefund {
                item_guid: item.guid,
            });
        }

        // Player-values update and stat refresh only apply to top-level slots.
        if bag == INVENTORY_SLOT_BAG_0 {
            let mut visible_item_changes = Vec::new();
            let mut virtual_item_changes = Vec::new();
            if (slot as usize) < 19 {
                visible_item_changes.push((slot, 0i32, 0u16, 0u16));
            }
            if slot >= 15 && slot <= 17 {
                virtual_item_changes.push((slot - 15, 0i32, 0u16, 0u16));
            }

            self.send_player_values_update_from_entity_bridge(
                &[(slot, ObjectGuid::EMPTY)],
                &visible_item_changes,
                &virtual_item_changes,
                &[],
                None,
            );

            if slot < 19 {
                self.send_stat_update();
            }
        }
    }
}
