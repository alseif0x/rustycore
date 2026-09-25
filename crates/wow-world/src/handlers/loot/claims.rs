// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Owned loot claims and their leases.

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use super::*;
use wow_entities::Item;
use wow_entities::ItemObjectUpdateLikeCpp;

mod release;

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

    #[cfg(test)]
    pub(super) async fn store_claimed_direct_loot_item_from_owner_like_cpp(
        &mut self,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        claim: &LootClaimLease,
    ) -> bool {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return false;
        };
        self.store_claimed_direct_loot_item_from_owner_with_generator_like_cpp(
            generator.as_ref(),
            loot_entry,
            dungeon_encounter_id,
            owner_guid,
            loot_obj,
            claim,
        )
        .await
    }

    pub(super) async fn store_claimed_direct_loot_item_from_owner_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        claim: &LootClaimLease,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        self.store_direct_loot_item_with_source_and_generator_like_cpp(
            item_guid_generator,
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

        let runtime_item = self.resolved_inventory_item_object_like_cpp(item_guid);
        let (bag, slot) = match runtime_item.as_ref() {
            Some(item) => (item.bag_slot(), item.slot()),
            None => return,
        };

        let Some(item) = self.get_inventory_item_by_pos(bag, slot) else {
            return;
        };

        let port = match self.stored_item_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };

        let current_count = runtime_item.as_ref().map_or(1, Item::count);
        let new_count =
            direct_item_count_after_loot_release_like_cpp(current_count, maximum_destroy_count);
        if new_count != 0 {
            match port
                .update_inventory_item_count_like_cpp(
                    wow_persistence::InventoryItemCountPersistenceRequestLikeCpp {
                        item_guid: item.db_guid,
                        count: new_count,
                    },
                )
                .await
            {
                wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
                wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
                | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                    warn!(error = %reason, "LootRelease: update partially consumed item failed");
                    return;
                }
            }
            let _ = self.apply_inventory_item_object_updates_like_cpp(
                item_guid,
                &[
                    ItemObjectUpdateLikeCpp::SetCount(new_count),
                    ItemObjectUpdateLikeCpp::SetLootGenerated(false),
                ],
            );
            self.send_packet(&UpdateObject::item_stack_count_update(
                item_guid,
                self.player_map_id_like_cpp(),
                new_count,
            ));
            return;
        }

        let should_expire_refund = runtime_item
            .as_ref()
            .is_some_and(|item_object| item_object.is_refundable());
        match port
            .destroy_inventory_item_like_cpp(
                wow_persistence::InventoryItemDestroyPersistenceRequestLikeCpp {
                    owner_guid: player_guid.counter() as u64,
                    item_guid: item.db_guid,
                    expire_refund: should_expire_refund,
                },
            )
            .await
        {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(error = %reason, "LootRelease: delete fully looted item failed");
                return;
            }
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
