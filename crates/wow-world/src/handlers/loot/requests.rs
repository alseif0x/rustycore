// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot window open/close requests and the represented loot cache.

use super::*;

impl WorldSession {
    /// Refresh the session-local window from the object-owned source of truth.
    /// The local table remains a packet-building cache only.
    pub(super) fn reconcile_represented_loot_cache_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        let Some(authority) = self.represented_owned_loot_authority_like_cpp(owner_guid) else {
            return false;
        };
        let Some(snapshot) = authority.snapshot_for_player_like_cpp(player_guid) else {
            self.discard_represented_personal_loot_cache_for_player_like_cpp(
                owner_guid,
                player_guid,
            );
            return false;
        };
        self.cache_represented_owned_loot_snapshot_like_cpp(owner_guid, player_guid, snapshot);
        true
    }

    /// Drops only this session/player's packet-building mirror. The canonical
    /// object-owned authority remains the source of truth and rehydrates a
    /// later open. C++ has no session-owned `Loot` clone after a window closes.
    pub(super) fn discard_represented_personal_loot_cache_for_player_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        _player_guid: ObjectGuid,
    ) {
        self.loot_table.remove(&owner_guid);
        self.represented_loot_cache_generations_like_cpp
            .remove(&owner_guid);
        self.represented_personal_loot_money
            .retain(|(owner, _), _| *owner != owner_guid);
        self.represented_personal_loot_owners.remove(&owner_guid);
    }

    /// Runtime-facing allocator. Production has no fallback: the `cfg(test)`
    /// branch exists only so older packet-cache fixtures can retain their
    /// deterministic owner-derived identity while they are migrated to typed
    /// canonical map objects.
    pub(super) fn next_represented_loot_object_guid_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        let canonical = self.next_canonical_loot_object_guid_like_cpp(owner_guid);
        #[cfg(test)]
        {
            canonical.or_else(|| {
                (!owner_guid.is_empty()).then(|| represented_loot_object_guid_like_cpp(owner_guid))
            })
        }
        #[cfg(not(test))]
        {
            canonical
        }
    }

    /// Mirrors `Loot::Loot(Map*)`: every concrete pool receives a fresh
    /// map-owned `HighGuid::LootObject` low GUID. This strict helper always
    /// fails closed when the owner's exact canonical map is unavailable.
    pub(super) fn next_canonical_loot_object_guid_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        (|| {
            // Map 0 (Eastern Kingdoms) is a real map, not an unspecified
            // sentinel. C++ allocates from the owner's exact `GetMap()`.
            let owner_map_id = u32::from(owner_guid.map_id());
            let key = self.canonical_object_lookup_map_key_like_cpp(owner_map_id)?;
            if key.map_id != owner_map_id {
                return None;
            }
            let manager = self.canonical_map_manager.as_ref()?;
            let mut manager = manager.lock().ok()?;
            let map = manager.find_map_mut(key.map_id, key.instance_id)?.map_mut();
            let counter = map.generate_low_guid_like_cpp(HighGuid::LootObject).ok()?;
            let map_id = u16::try_from(key.map_id).ok()?;
            // C++ passes realm id 0 to ObjectGuidFactory, where
            // `GetRealmIdForObjectGuid(0)` substitutes the active realm.
            // Rust's factory is explicit, so pass the session realm here.
            Some(ObjectGuid::create_world_object(
                HighGuid::LootObject,
                0,
                self.realm_id(),
                map_id,
                0,
                0,
                counter,
            ))
        })()
    }

    pub(super) fn refresh_represented_loot_owner_canonical_summary_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        if owner_guid.is_game_object() {
            let _ = self
                .sync_represented_gameobject_loot_to_canonical_like_cpp(owner_guid, player_guid);
        } else if owner_guid.is_creature_or_vehicle() {
            if self
                .sync_represented_creature_loot_to_canonical_like_cpp(owner_guid, player_guid)
                .is_none()
            {
                self.loot_table.remove(&owner_guid);
                return;
            }
        }
    }

    pub(super) fn record_represented_disenchant_criteria_like_cpp(
        &mut self,
        _player_guid: ObjectGuid,
        _spell_id: u32,
    ) {
        #[cfg(test)]
        self.represented_loot_roll_criteria_events.push(
            crate::session::RepresentedLootRollCriteriaEvent::Disenchant {
                player_guid: _player_guid,
                spell_id: _spell_id,
            },
        );
    }

    pub(super) async fn store_represented_disenchant_loot_winner_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        entry: &LootEntry,
        winner_guid: ObjectGuid,
        dungeon_encounter_id: u32,
        claim: Option<&LootClaimLease>,
    ) -> bool {
        let Some(template) = self
            .item_stats_store()
            .and_then(|store| store.random_property_template(entry.item_id))
        else {
            return false;
        };
        let Some((disenchant_id, _)) = self.item_disenchant_loot_like_cpp(
            entry.item_id,
            template.quality as u32,
            u32::from(template.item_level),
            true,
        ) else {
            return false;
        };

        let disenchant_entries = self
            .generate_represented_disenchant_loot_template_entries_like_cpp(
                disenchant_id,
                winner_guid,
            )
            .await;
        if disenchant_entries.is_empty() {
            return false;
        }

        if self.player_guid() == Some(winner_guid) {
            return self
                .store_direct_disenchant_batch_like_cpp(
                    &disenchant_entries,
                    dungeon_encounter_id,
                    claim,
                    claim.map(|_| LootItemClaimCommitContextLikeCpp {
                        owner_guid,
                        loot_obj,
                        loot_list_id,
                        player_guid: winner_guid,
                        free_for_all: entry.flags.freeforall,
                    }),
                )
                .await;
        }

        match self
            .request_represented_remote_loot_roll_winner_store_like_cpp(
                winner_guid,
                owner_guid,
                loot_obj,
                loot_list_id,
                dungeon_encounter_id,
                disenchant_entries,
                true,
                claim.cloned(),
            )
            .await
        {
            MasterLootGiveResult::Stored => true,
            MasterLootGiveResult::StoreFailed(error) => {
                debug!(
                    account = self.account_id,
                    winner = ?winner_guid,
                    loot_obj = ?loot_obj,
                    loot_list_id,
                    error,
                    "represented disenchant loot winner batch failed in target session"
                );
                false
            }
            MasterLootGiveResult::TargetMismatch => {
                debug!(
                    account = self.account_id,
                    winner = ?winner_guid,
                    loot_obj = ?loot_obj,
                    loot_list_id,
                    "represented disenchant loot winner target was not connected"
                );
                false
            }
        }
    }

    pub(super) async fn represented_loot_response_for_owner_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        ae_looting: bool,
    ) -> Option<LootResponse> {
        let creature = self.represented_creature_loot_state_like_cpp(owner_guid)?;
        if !creature.tappers.is_empty() && !creature.tappers.contains(&player_guid) {
            return None;
        }
        // `Player::isAllowedToLoot` reads `Creature::GetLootForPlayer`; the
        // client request is not a generation trigger. A retired/missing
        // authority therefore means there is no loot response.
        if !self.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid) {
            self.loot_table.remove(&owner_guid);
            return None;
        }

        let loot = self.loot_table.get(&owner_guid)?;
        if !self.represented_loot_can_be_opened_by_player_like_cpp(owner_guid, loot, player_guid) {
            return None;
        }

        Some(LootResponse {
            owner: owner_guid,
            loot_obj: loot.loot_guid,
            failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
            acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
            loot_method: loot.loot_method,
            threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
            coins: self.represented_loot_money_for_player_like_cpp(owner_guid, loot, player_guid),
            items: represented_loot_response_items_like_cpp(loot, player_guid),
            currencies: vec![],
            acquired: true,
            ae_looting,
        })
    }

    pub(super) fn represented_on_loot_opened_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        mut response: LootResponse,
    ) {
        let authority = self
            .prepare_owned_loot_authority_for_active_request_like_cpp(owner_guid, player_guid)
            .filter(|authority| {
                authority
                    .snapshot_for_player_like_cpp(player_guid)
                    .is_some()
            });
        let authoritative_open = if let Some(authority) = authority.as_ref() {
            match authority.try_open_view_with_snapshot_like_cpp(
                player_guid,
                |snapshot, outcome| {
                    // Enqueue the response while the authority mutex still
                    // excludes item/money commit. Any later commit therefore
                    // observes this viewer and its removal packet is ordered
                    // after the response on this session's send queue.
                    response.loot_obj = snapshot.loot.loot_guid;
                    response.acquire_reason =
                        loot_type_for_client_like_cpp(snapshot.loot.loot_type);
                    response.loot_method = snapshot.loot.loot_method;
                    response.coins = snapshot.loot.coins;
                    response.items =
                        represented_loot_response_items_like_cpp(&snapshot.loot, player_guid);

                    // `flume::Sender::send` may wait indefinitely while this
                    // lock is held. Reject a saturated/disconnected socket
                    // queue immediately; the authority method rolls back its
                    // tentative viewer and first-open mutations before unlock.
                    if !self.try_send_packet(&response) {
                        return None;
                    }

                    // Session mirrors become observable only after the client
                    // response was accepted by its ordered send queue.
                    self.loot_table.insert(owner_guid, snapshot.loot.clone());
                    self.represented_loot_cache_generations_like_cpp
                        .insert(owner_guid, snapshot.generation);
                    self.active_loot_view_generations_like_cpp
                        .insert(owner_guid, outcome.generation);
                    self.active_loot_view_authorities_like_cpp
                        .insert(owner_guid, authority.clone());
                    Some(())
                },
            ) {
                Ok((outcome, ())) => Some(outcome),
                Err(LootClaimError::ResponseEnqueueFailed) => {
                    // Do not attempt a blocking release on the same saturated
                    // queue. The client never observed this view, so dropping
                    // every local mirror is the closed state.
                    self.discard_represented_personal_loot_cache_for_player_like_cpp(
                        owner_guid,
                        player_guid,
                    );
                    self.clear_active_loot_guid_if(owner_guid);
                    return;
                }
                Err(_) => None,
            }
        } else {
            None
        };
        if authoritative_open.is_none() {
            if (owner_guid.is_creature_or_vehicle() || owner_guid.is_game_object())
                && !represented_local_loot_fixture_allowed_like_cpp()
            {
                self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
                return;
            }
            self.send_packet(&response);
            self.ensure_represented_player_looting_like_cpp(owner_guid, player_guid);
        } else if let Some(authority) = authority.as_ref() {
            if !self
                .active_loot_view_authorities_like_cpp
                .get(&owner_guid)
                .is_some_and(|opened| opened.shares_storage_like_cpp(authority))
            {
                self.active_loot_view_authorities_like_cpp
                    .insert(owner_guid, authority.clone());
            }
        }

        self.represented_notify_loot_list_like_cpp(owner_guid);

        let first_open = match authoritative_open {
            Some(outcome) => outcome.first_viewer,
            None => match self.loot_table.get_mut(&owner_guid) {
                Some(loot) if !loot.looted_by_player => {
                    loot.looted_by_player = true;
                    true
                }
                _ => false,
            },
        };
        if !first_open {
            return;
        }

        let loot_method = self
            .loot_table
            .get(&owner_guid)
            .map(|loot| loot.loot_method)
            .unwrap_or_default();
        match loot_method {
            LOOT_METHOD_GROUP_LIKE_CPP | LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP => {
                self.represented_start_group_loot_rolls_on_first_open_like_cpp(
                    owner_guid,
                    player_guid,
                );
            }
            LOOT_METHOD_MASTER_LIKE_CPP => {
                if let Some(packet) =
                    self.represented_master_loot_candidate_list_like_cpp(owner_guid, player_guid)
                {
                    self.send_packet(&packet);
                }
            }
            _ => {}
        }
    }

    /// True only while a request still belongs to the exact object lifetime
    /// whose loot window this session opened.
    pub(super) fn represented_active_loot_generation_matches_like_cpp(
        &self,
        owner_guid: ObjectGuid,
        authority: &OwnedLootAuthority,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let current_generation = authority
            .snapshot_for_player_like_cpp(player_guid)
            .map(|snapshot| snapshot.generation);
        self.active_loot_view_authorities_like_cpp
            .get(&owner_guid)
            .is_some_and(|opened| opened.shares_storage_like_cpp(authority))
            && self
                .active_loot_view_generations_like_cpp
                .get(&owner_guid)
                .is_some_and(|opened| Some(*opened) == current_generation)
    }

    pub(super) fn ensure_represented_player_looting_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        if let Some(loot) = self.loot_table.get_mut(&owner_guid)
            && !loot.players_looting.contains(&player_guid)
        {
            loot.players_looting.push(player_guid);
        }
    }

    pub(super) fn represented_player_unlocked_for_dungeon_encounter_like_cpp(
        &self,
        player_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> Option<ObjectGuid> {
        self.represented_player_is_unlocked_for_dungeon_encounter_like_cpp(
            player_guid,
            dungeon_encounter_id,
        )
        .then_some(player_guid)
    }

    pub(super) fn represented_player_is_unlocked_for_dungeon_encounter_like_cpp(
        &self,
        player_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> bool {
        !self
            .represented_locked_dungeon_encounters
            .contains(&(player_guid, dungeon_encounter_id))
    }

    /// C++ `Loot::FillLoot` calls `FillNotNormalLootFor` for every connected
    /// group member at reward distance from the opening player before the
    /// chest's shared `Loot` becomes visible.
    pub(super) fn represented_group_looters_at_reward_distance_like_cpp(
        &self,
        player_guid: ObjectGuid,
    ) -> Vec<ObjectGuid> {
        let Some(group_guid) = self.group_guid else {
            return vec![player_guid];
        };
        let Some(group_registry) = self.group_registry() else {
            return vec![player_guid];
        };
        let Some(group) = group_registry.get(&group_guid) else {
            return vec![player_guid];
        };
        let Some(source_position) = self.player_position_like_cpp() else {
            return vec![player_guid];
        };
        let map_id = self.player_map_id_like_cpp();
        let Some(instance_id) = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
        else {
            return vec![player_guid];
        };
        let registry = self.player_registry();
        let mut looters = Vec::new();

        for member_guid in &group.members {
            if *member_guid == player_guid {
                looters.push(*member_guid);
                continue;
            }
            let Some(member) = registry.and_then(|registry| registry.loot_presence(*member_guid))
            else {
                continue;
            };
            if member.is_in_world
                && member.map_id == map_id
                && member.instance_id == instance_id
                && (self.current_map_is_dungeon_like_cpp()
                    || source_position.is_within_dist(&member.position, 74.0))
            {
                looters.push(*member_guid);
            }
        }

        if looters.is_empty() {
            looters.push(player_guid);
        }
        looters.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));
        looters.dedup();
        looters
    }

    pub(super) fn represented_dungeon_trash_looter_like_cpp(
        &self,
        connected_tappers: &[ObjectGuid],
    ) -> ObjectGuid {
        let selected =
            if let (Some(group_guid), Some(registry)) = (self.group_guid, self.group_registry()) {
                registry
                    .get(&group_guid)
                    .map(|group| group.looter_guid_like_cpp())
                    .filter(|looter| connected_tappers.contains(looter))
            } else {
                None
            };
        selected.unwrap_or(connected_tappers[0])
    }

    pub(super) fn advance_represented_dungeon_trash_looter_like_cpp(
        &self,
        connected_tappers: &[ObjectGuid],
    ) {
        let (Some(group_guid), Some(registry)) = (self.group_guid, self.group_registry()) else {
            return;
        };
        let _ = registry
            .advance_looter_transition_like_cpp(group_guid, connected_tappers.iter().copied());
    }

    pub(super) fn item_loot_quest_status_allows_for_player_like_cpp(
        &self,
        item_id: u32,
        needs_quest: bool,
        addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
        player_context: &RepresentedLootPlayerContext,
    ) -> bool {
        if player_context.is_current {
            return self.item_loot_quest_status_allows_like_cpp(
                item_id,
                needs_quest,
                addon_metadata,
            );
        }

        if addon_metadata.ignores_quest_status() {
            return true;
        }

        let start_quest_id = self.item_template_start_quest_id(item_id).unwrap_or(0);
        let has_non_none_start_quest_status = u32::try_from(start_quest_id)
            .ok()
            .is_some_and(|quest_id| quest_id != 0 && player_context.quest_status(quest_id) != 0);
        let has_quest_for_item =
            self.represented_has_quest_for_item_like_cpp(item_id, addon_metadata, player_context);

        (!needs_quest && !has_non_none_start_quest_status) || has_quest_for_item
    }

    pub(super) fn represented_loot_player_context_like_cpp(
        &self,
        player_guid: ObjectGuid,
    ) -> Option<RepresentedLootPlayerContext> {
        if Some(player_guid) == self.player_guid() {
            let quests = self.player_quest_gameplay_snapshot_like_cpp()?;
            return Some(RepresentedLootPlayerContext {
                race: self.player_race_like_cpp(),
                class: self.player_class_like_cpp(),
                gender: self.player_gender_like_cpp(),
                level: self.player_level_like_cpp(),
                known_spells: self.known_spells_like_cpp().to_vec(),
                active_quest_statuses: quests
                    .statuses
                    .iter()
                    .map(|(quest_id, status)| (*quest_id, status.status))
                    .collect(),
                active_quest_objective_counts: quests
                    .statuses
                    .iter()
                    .map(|(quest_id, status)| (*quest_id, status.objective_counts.clone()))
                    .collect(),
                rewarded_quests: quests.rewarded_quest_ids.into_iter().collect(),
                inventory_item_counts: self.represented_inventory_item_counts_like_cpp()?,
                is_current: true,
            });
        }

        let player = self.player_registry()?.loot_player_context(player_guid)?;
        Some(RepresentedLootPlayerContext {
            race: player.race,
            class: player.class,
            gender: player.sex,
            level: player.level,
            known_spells: player.known_spells.clone(),
            active_quest_statuses: player.active_quest_statuses.clone(),
            active_quest_objective_counts: player.active_quest_objective_counts.clone(),
            rewarded_quests: player.rewarded_quests.clone(),
            inventory_item_counts: player.inventory_item_counts.clone(),
            is_current: false,
        })
    }

    pub(super) fn item_template_flags2_like_cpp(&self, item_id: u32) -> Option<u32> {
        self.item_stats_store()
            .and_then(|store| store.sparse_template(item_id))
            .map(|template| template.flags[1])
    }

    pub(super) fn item_loot_quest_status_allows_like_cpp(
        &self,
        item_id: u32,
        needs_quest: bool,
        addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
    ) -> bool {
        let start_quest_id = self.item_template_start_quest_id(item_id).unwrap_or(0);
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        let has_non_none_start_quest_status =
            u32::try_from(start_quest_id).ok().is_some_and(|quest_id| {
                quest_id != 0
                    && (quests.statuses.contains_key(&quest_id)
                        || quests.rewarded_quest_ids.contains(&quest_id))
            });
        let has_quest_for_item = self.has_incomplete_quest_objective_for_item_like_cpp(item_id)
            || (addon_metadata.quest_log_item_id != 0
                && self.has_incomplete_quest_objective_for_object_id_like_cpp(
                    addon_metadata.quest_log_item_id,
                ))
            || self.has_incomplete_quest_item_drop_for_item_like_cpp(item_id);

        addon_metadata.ignores_quest_status()
            || ((!needs_quest && !has_non_none_start_quest_status) || has_quest_for_item)
    }

    pub(super) fn has_incomplete_quest_objective_for_item_like_cpp(&self, item_id: u32) -> bool {
        let Ok(item_object_id) = i32::try_from(item_id) else {
            return false;
        };
        self.has_incomplete_quest_objective_for_object_id_like_cpp(item_object_id)
    }

    fn has_incomplete_quest_objective_for_object_id_like_cpp(&self, item_object_id: i32) -> bool {
        let Some(quest_store) = &self.quest_store else {
            return false;
        };

        self.player_quest_gameplay_snapshot_like_cpp()
            .is_some_and(|state| {
                state.statuses.into_values().any(|status| {
                    if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                        return false;
                    }

                    let Some(quest) = quest_store.get(status.quest_id) else {
                        return false;
                    };

                    quest
                        .objectives
                        .iter()
                        .enumerate()
                        .any(|(fallback_index, objective)| {
                            if objective.obj_type != 1 || objective.object_id != item_object_id {
                                return false;
                            }

                            let storage_index = usize::try_from(objective.storage_index)
                                .ok()
                                .unwrap_or(fallback_index);
                            let current = status
                                .objective_counts
                                .get(storage_index)
                                .copied()
                                .unwrap_or(0);
                            current < objective.amount.max(1)
                        })
                })
            })
    }

    fn represented_has_quest_for_item_like_cpp(
        &self,
        item_id: u32,
        addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
        player_context: &RepresentedLootPlayerContext,
    ) -> bool {
        if player_context.is_current {
            return self.has_incomplete_quest_objective_for_item_like_cpp(item_id)
                || (addon_metadata.quest_log_item_id != 0
                    && self.has_incomplete_quest_objective_for_object_id_like_cpp(
                        addon_metadata.quest_log_item_id,
                    ))
                || self.has_incomplete_quest_item_drop_for_item_like_cpp(item_id);
        }

        let Ok(item_object_id) = i32::try_from(item_id) else {
            return false;
        };
        self.remote_has_incomplete_quest_objective_for_object_id_like_cpp(
            item_object_id,
            player_context,
        ) || (addon_metadata.quest_log_item_id != 0
            && self.remote_has_incomplete_quest_objective_for_object_id_like_cpp(
                addon_metadata.quest_log_item_id,
                player_context,
            ))
            || self.remote_has_incomplete_quest_item_drop_for_item_like_cpp(item_id, player_context)
    }

    fn remote_has_incomplete_quest_objective_for_object_id_like_cpp(
        &self,
        item_object_id: i32,
        player_context: &RepresentedLootPlayerContext,
    ) -> bool {
        let Some(quest_store) = &self.quest_store else {
            return false;
        };

        player_context
            .active_quest_objective_counts
            .iter()
            .any(|(quest_id, objective_counts)| {
                if player_context.quest_status(*quest_id) != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                    return false;
                }

                let Some(quest) = quest_store.get(*quest_id) else {
                    return false;
                };

                quest
                    .objectives
                    .iter()
                    .enumerate()
                    .any(|(fallback_index, objective)| {
                        if objective.obj_type != 1 || objective.object_id != item_object_id {
                            return false;
                        }

                        let storage_index = usize::try_from(objective.storage_index)
                            .ok()
                            .unwrap_or(fallback_index);
                        let current = objective_counts.get(storage_index).copied().unwrap_or(0);
                        current < objective.amount.max(1)
                    })
            })
    }

    pub(super) fn direct_inventory_item_count_like_cpp(&self, item_id: u32) -> Option<u32> {
        Some(
            self.represented_inventory_item_counts_like_cpp()?
                .get(&item_id)
                .copied()
                .unwrap_or(0),
        )
    }

    pub(super) fn player_quest_objective_progress_like_cpp(
        &self,
        objective_id: u32,
    ) -> Option<i32> {
        let quest_store = self.quest_store.as_ref()?;

        for status in self
            .player_quest_gameplay_snapshot_like_cpp()?
            .statuses
            .into_values()
        {
            let Some(quest) = quest_store.get(status.quest_id) else {
                continue;
            };
            let Some((_, objective)) = quest
                .objectives
                .iter()
                .enumerate()
                .find(|(_, objective)| objective.id == objective_id)
            else {
                continue;
            };
            let objective_index = objective.storage_index.max(0) as usize;
            return Some(
                status
                    .objective_counts
                    .get(objective_index)
                    .copied()
                    .unwrap_or(0),
            );
        }

        None
    }

    pub(super) fn remote_player_quest_objective_progress_like_cpp(
        &self,
        objective_id: u32,
        player_context: &RepresentedLootPlayerContext,
    ) -> Option<i32> {
        let quest_store = self.quest_store.as_ref()?;

        for (quest_id, objective_counts) in &player_context.active_quest_objective_counts {
            let Some(quest) = quest_store.get(*quest_id) else {
                continue;
            };
            let Some((_, objective)) = quest
                .objectives
                .iter()
                .enumerate()
                .find(|(_, objective)| objective.id == objective_id)
            else {
                continue;
            };
            let objective_index = objective.storage_index.max(0) as usize;
            return Some(objective_counts.get(objective_index).copied().unwrap_or(0));
        }

        None
    }

    pub(super) async fn load_item_template_addon_loot_metadata_for_item_ids_like_cpp<I>(
        &self,
        item_ids: I,
    ) -> HashMap<u32, ItemTemplateAddonLootMetadataLikeCpp>
    where
        I: IntoIterator<Item = u32>,
    {
        let mut item_ids: Vec<u32> = item_ids.into_iter().collect();
        item_ids.sort_unstable();
        item_ids.dedup();

        let mut metadata = HashMap::with_capacity(item_ids.len());
        for item_id in item_ids {
            metadata.insert(
                item_id,
                self.load_creature_item_template_addon_loot_metadata_like_cpp(item_id)
                    .await,
            );
        }
        metadata
    }

    pub(super) fn active_loot_owner_for_loot_object_like_cpp(
        &self,
        loot_object: ObjectGuid,
    ) -> Option<ObjectGuid> {
        let active_owners: Vec<ObjectGuid> = if self.active_loot_view_owners.is_empty() {
            vec![self.active_loot_guid]
        } else {
            self.active_loot_view_owners.iter().copied().collect()
        };

        active_owners.into_iter().find(|owner_guid| {
            !owner_guid.is_empty()
                && self
                    .loot_table
                    .get(owner_guid)
                    .is_some_and(|loot| loot.loot_guid == loot_object)
        })
    }

    pub(super) fn canonical_map_object_position_for_loot_like_cpp(
        &self,
        guid: ObjectGuid,
        allowed: &[AccessorObjectKind],
    ) -> Option<wow_core::Position> {
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        let manager = self.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        let map = manager.find_map(map_key.map_id, map_key.instance_id)?.map();
        map.map_object_by_kind(guid, allowed)
            .map(|object| object.position())
    }

    pub(super) fn represented_spell_max_range_like_cpp(&self, spell_id: i32) -> Option<f32> {
        let spell_store = self.spell_store()?;
        let spell_misc_store = self.spell_misc_store()?;
        let spell_range_store = self.spell_range_store()?;
        spell_store.get(spell_id)?;
        let spell_id = u32::try_from(spell_id).ok()?;
        let range_index = spell_misc_store.get(spell_id)?.range_index;
        let range = spell_range_store.get(u32::from(range_index))?;
        Some(range.range_max[1].max(range.range_max[0]))
    }

    /// Mirrors the observable side of C++ `Loot::~Loot`: once the exact
    /// object-owned allocation behind an open view is retired, detached, or
    /// replaced, the next session tick releases that stale client window.
    /// Each session owns its socket, so global object destruction is fanned
    /// out cooperatively without holding a map lock across network work.
    pub(crate) fn close_retired_active_loot_windows_like_cpp(&mut self, player_guid: ObjectGuid) {
        let mut stale_owners = self
            .active_loot_view_authorities_like_cpp
            .iter()
            .filter_map(|(owner_guid, authority)| {
                let generation = self
                    .active_loot_view_generations_like_cpp
                    .get(owner_guid)
                    .copied();
                let still_open = generation.is_some_and(|generation| {
                    authority
                        .snapshot_for_player_like_cpp(player_guid)
                        .is_some_and(|snapshot| snapshot.generation == generation)
                });
                (!still_open).then_some(*owner_guid)
            })
            .collect::<Vec<_>>();
        stale_owners.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));

        for owner_guid in stale_owners {
            self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
        }
    }

    pub(super) fn close_stale_active_loot_view_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        self.discard_represented_personal_loot_cache_for_player_like_cpp(owner_guid, player_guid);
        self.send_packet(&SLootRelease {
            loot_obj: owner_guid,
            owner: player_guid,
        });
        self.clear_active_loot_guid_if(owner_guid);
    }

    pub(super) async fn store_direct_loot_item_like_cpp(
        &mut self,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
    ) -> bool {
        self.store_direct_loot_item_with_source_like_cpp(
            loot_entry,
            dungeon_encounter_id,
            None,
            None,
            None,
        )
        .await
    }

    /// Apply the item state established by C++ `Player::StoreNewItem` and
    /// `_StoreItem` before the item is persisted or sent to the client.
    fn apply_stored_new_item_flags_like_cpp(&self, item_id: u32, slot: u8, item: &mut Item) {
        if let Some(template) = self.item_storage_template(item_id) {
            item.set_bonding(template.bonding);
        }
        item.set_item_flag(ItemFieldFlags::NEW_ITEM);
        item.bind_if_stored(is_bag_pos(make_item_pos(INVENTORY_SLOT_BAG_0, slot)));
    }

    pub(super) fn stored_new_item_dynamic_flags_like_cpp(&self, item_id: u32, slot: u8) -> u32 {
        let mut item = Item::new(0);
        self.apply_stored_new_item_flags_like_cpp(item_id, slot, &mut item);
        item.item_flags_bits()
    }

    /// C++ `_StoreItem` binds the destination object before incrementing an
    /// existing stack. Unlike `StoreNewItem`, that historical object must not
    /// acquire `ITEM_FIELD_FLAG_NEW_ITEM` merely because more items arrived.
    pub(super) fn stored_existing_item_dynamic_flags_like_cpp(
        &self,
        item_id: u32,
        slot: u8,
        existing: &Item,
    ) -> u32 {
        let mut planned = existing.clone();
        if let Some(template) = self.item_storage_template(item_id) {
            planned.set_bonding(template.bonding);
        }
        planned.bind_if_stored(is_bag_pos(make_item_pos(INVENTORY_SLOT_BAG_0, slot)));
        planned.item_flags_bits()
    }

    /// Persist the complete result of one group-roll disenchant as a single
    /// durable award.
    ///
    /// C++ `LootRoll::Finish` first materializes a temporary
    /// `LOOT_DISENCHANTING` loot and then calls `Loot::AutoStore`.  The C++
    /// loop stores each generated material independently, which is unsafe for
    /// Rust's concurrently shared object authority: a later failure could
    /// reopen the original roll slot after an earlier material was durable.
    /// This bounded divergence keeps C++ generation/inventory rules but plans
    /// every material before creating one SQL transaction.  The detached
    /// transaction worker owns the original roll claim through COMMIT.
    pub(super) async fn store_direct_disenchant_batch_like_cpp(
        &mut self,
        loot_entries: &[LootEntry],
        dungeon_encounter_id: u32,
        claim: Option<&LootClaimLease>,
        claim_commit_context: Option<LootItemClaimCommitContextLikeCpp>,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        if loot_entries.is_empty()
            || loot_entries
                .iter()
                .any(|entry| entry.item_id == 0 || entry.quantity == 0)
        {
            return false;
        }
        let durable_item_fanout = match (claim, claim_commit_context) {
            (Some(claim), Some(context)) => {
                let Some(fanout) = self.prepare_durable_loot_item_fanout_like_cpp(claim, context)
                else {
                    return false;
                };
                Some(fanout)
            }
            (None, None) => None,
            _ => return false,
        };

        #[cfg(test)]
        if let Some(grants) = self.loot_item_store_test_grants_like_cpp.clone() {
            let success = self.loot_item_store_test_success_like_cpp;
            let commit_gate = self.loot_item_store_test_commit_gate_like_cpp.clone();
            let grant_count = loot_entries.len();
            let runtime_inventory_applied =
                claim_commit_context.map(|_| Arc::new(AtomicBool::new(false)));
            let durable_item_completion = claim_commit_context
                .zip(runtime_inventory_applied.as_ref().map(Arc::clone))
                .map(|(context, runtime_inventory_applied)| {
                    (
                        self.begin_durable_item_loot_persistence_like_cpp(),
                        DurableItemLootCompletionLikeCpp {
                            owner_guid: context.owner_guid,
                            loot_list_id: context.loot_list_id,
                            player_guid: context.player_guid,
                            item_owner_auto_release: false,
                            durable_item_money_applied_amount: None,
                            durable_item_money_notified_amount: None,
                            durable_item_money_balance_applied: None,
                            item_fanout: durable_item_fanout.clone(),
                            runtime_inventory_applied,
                        },
                    )
                });
            let Ok(persistence) = spawn_loot_claim_persistence_worker_like_cpp(
                async move {
                    // Model the asynchronous commit boundary so cancellation
                    // regressions exercise the same ownership shape as SQL.
                    tokio::task::yield_now().await;
                    if let Some(gate) = commit_gate {
                        gate.notified().await;
                    }
                    if !success {
                        return Err(());
                    }
                    grants.fetch_add(grant_count, Ordering::SeqCst);
                    Ok(())
                },
                claim.cloned(),
                durable_item_completion,
            ) else {
                return false;
            };
            if !matches!(persistence.await, Ok(Ok(()))) {
                return false;
            }
            for (slot, entry) in loot_entries.iter().enumerate() {
                self.send_loot_item_push_result(
                    player_guid,
                    ObjectGuid::EMPTY,
                    entry,
                    0,
                    0,
                    u8::try_from(slot).unwrap_or(0),
                    entry.quantity,
                    entry.quantity,
                    false,
                    dungeon_encounter_id,
                );
            }
            if !self.publish_persisted_loot_item_removal_like_cpp(
                claim,
                claim_commit_context,
                durable_item_fanout.as_ref(),
            ) {
                return false;
            }
            if let Some(runtime_inventory_applied) = runtime_inventory_applied {
                runtime_inventory_applied.store(true, Ordering::Release);
            }
            return true;
        }

        let Some(inventory_persistence) = self.player_inventory_persistence_port_like_cpp() else {
            return false;
        };

        // `CanStoreNewItem`'s max-count checks must see all generated stacks
        // of the same material, not each temporary LootItem in isolation.
        let mut quantity_by_item = HashMap::<u32, u32>::new();
        for entry in loot_entries {
            let Some(total) = quantity_by_item
                .get(&entry.item_id)
                .copied()
                .unwrap_or(0)
                .checked_add(entry.quantity)
            else {
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            };
            quantity_by_item.insert(entry.item_id, total);
        }
        for (item_id, count) in quantity_by_item {
            let Some((store_result, _, _)) =
                self.plan_store_new_direct_inventory_item(item_id, count)
            else {
                self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
                return false;
            };
            if store_result != InventoryResult::Ok {
                self.send_equip_error(store_result, None, None, 0, 0);
                return false;
            }
        }

        let backpack_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(INVENTORY_DEFAULT_SIZE)
            .min(INVENTORY_SLOT_ITEM_END);
        let mut planned_existing_stacks = Vec::<PlannedDisenchantExistingStack>::new();
        let mut planned_new_stacks = Vec::<PlannedLootNewStack>::new();
        let mut planned_grants = Vec::<PlannedDisenchantGrant>::new();

        for loot_entry in loot_entries {
            let random_properties = {
                let mut rng = self.represented_runtime_subrng_like_cpp();
                self.generate_loot_store_random_properties_with_rng_like_cpp(
                    loot_entry.item_id,
                    &mut rng,
                )
            };
            let max_stack = self
                .item_storage_template(loot_entry.item_id)
                .map(|template| template.max_stack_size)
                .unwrap_or(1)
                .max(1);
            let mut remaining = loot_entry.quantity;
            let mut existing_pushes = Vec::new();
            let mut new_pushes = Vec::new();

            // Existing backpack stacks are consumed first, matching the
            // direct StoreNewItem path represented in this server.
            for slot in INVENTORY_SLOT_ITEM_START..backpack_end {
                if remaining == 0 {
                    break;
                }
                let Some(existing) = self.resolved_inventory_item_like_cpp(slot) else {
                    continue;
                };
                if existing.entry_id != loot_entry.item_id {
                    continue;
                }
                let Some(existing_object) =
                    self.resolved_inventory_item_object_like_cpp(existing.guid)
                else {
                    self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
                    return false;
                };
                if !loot_store_data_can_stack_with_item(
                    loot_entry,
                    random_properties,
                    &existing_object,
                ) {
                    continue;
                }

                let current_count = planned_existing_stacks
                    .iter()
                    .find(|planned| planned.slot == slot)
                    .map(|planned| planned.new_count)
                    .unwrap_or_else(|| existing_object.count());
                let added_count = max_stack.saturating_sub(current_count).min(remaining);
                if added_count == 0 {
                    continue;
                }
                let new_count = current_count.saturating_add(added_count);
                if let Some(planned) = planned_existing_stacks
                    .iter_mut()
                    .find(|planned| planned.slot == slot)
                {
                    planned.new_count = new_count;
                } else {
                    let dynamic_flags = self.stored_existing_item_dynamic_flags_like_cpp(
                        loot_entry.item_id,
                        slot,
                        &existing_object,
                    );
                    planned_existing_stacks.push(PlannedDisenchantExistingStack {
                        slot,
                        item_guid: existing.guid,
                        db_guid: existing.db_guid,
                        new_count,
                        dynamic_flags,
                        flags_changed: dynamic_flags != existing_object.item_flags_bits(),
                    });
                }
                existing_pushes.push(PlannedDisenchantExistingPush {
                    slot,
                    item_guid: existing.guid,
                    added_count,
                    new_count,
                });
                remaining = remaining.saturating_sub(added_count);
            }

            // A second generated LootItem for the same material may continue
            // a new stack already planned earlier in this same transaction.
            for (stack_index, stack) in planned_new_stacks.iter_mut().enumerate() {
                if remaining == 0 {
                    break;
                }
                if stack.entry_id != loot_entry.item_id
                    || stack.random_properties_id != random_properties.id
                    || stack.random_properties_seed != random_properties.seed
                    || stack.item_context != loot_entry.item_context
                {
                    continue;
                }
                let added_count = max_stack.saturating_sub(stack.count).min(remaining);
                if added_count == 0 {
                    continue;
                }
                stack.count = stack.count.saturating_add(added_count);
                new_pushes.push(PlannedDisenchantNewPush {
                    stack_index,
                    added_count,
                    new_count: stack.count,
                });
                remaining = remaining.saturating_sub(added_count);
            }

            while remaining > 0 {
                let Some(slot) = (INVENTORY_SLOT_ITEM_START..backpack_end).find(|slot| {
                    self.resolved_inventory_items_like_cpp()
                        .is_some_and(|items| !items.contains_key(slot))
                        && !planned_new_stacks.iter().any(|stack| stack.slot == *slot)
                }) else {
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return false;
                };
                let count = remaining.min(max_stack);
                let stack_index = planned_new_stacks.len();
                planned_new_stacks.push(PlannedLootNewStack {
                    slot,
                    entry_id: loot_entry.item_id,
                    count,
                    max_durability: self.item_template_max_durability(loot_entry.item_id),
                    dynamic_flags: self
                        .stored_new_item_dynamic_flags_like_cpp(loot_entry.item_id, slot),
                    random_properties_id: random_properties.id,
                    random_properties_seed: random_properties.seed,
                    item_context: loot_entry.item_context,
                });
                new_pushes.push(PlannedDisenchantNewPush {
                    stack_index,
                    added_count: count,
                    new_count: count,
                });
                remaining = remaining.saturating_sub(count);
            }

            planned_grants.push(PlannedDisenchantGrant {
                entry: loot_entry.clone(),
                random_properties,
                existing_pushes,
                new_pushes,
            });
        }

        let mut created_new_stacks = Vec::with_capacity(planned_new_stacks.len());
        if !planned_new_stacks.is_empty() {
            let Some(allocated_guids) =
                self.allocate_item_instance_guids_like_cpp(planned_new_stacks.len())
            else {
                warn!(
                    count = planned_new_stacks.len(),
                    "disenchant item grant has no process-wide item GUID allocator"
                );
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            };

            for (stack, (db_guid, item_guid)) in planned_new_stacks.iter().zip(allocated_guids) {
                created_new_stacks.push((stack.clone(), db_guid, item_guid));
            }
        }

        let persistence_request =
            wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::LootDisenchantBatch(
                wow_persistence::LootDisenchantBatchPersistenceLikeCpp {
                    existing_stacks: planned_existing_stacks
                        .iter()
                        .map(
                            |stack| wow_persistence::LootExistingStackPersistenceLikeCpp {
                                item_guid: stack.db_guid,
                                new_count: stack.new_count,
                                dynamic_flags: stack.flags_changed.then_some(stack.dynamic_flags),
                            },
                        )
                        .collect(),
                    new_stacks: created_new_stacks
                        .iter()
                        .map(|(stack, db_guid, _)| {
                            wow_persistence::LootNewStackPersistenceLikeCpp {
                                item_guid: *db_guid,
                                entry_id: stack.entry_id,
                                owner_guid: player_guid.counter() as u64,
                                count: stack.count,
                                max_durability: stack.max_durability,
                                dynamic_flags: stack.dynamic_flags,
                                random_properties_id: stack.random_properties_id,
                                random_properties_seed: stack.random_properties_seed,
                                item_context: stack.item_context,
                                slot: stack.slot,
                            }
                        })
                        .collect(),
                },
            );

        let runtime_inventory_applied =
            claim_commit_context.map(|_| Arc::new(AtomicBool::new(false)));
        let durable_item_completion = claim_commit_context
            .zip(runtime_inventory_applied.as_ref().map(Arc::clone))
            .map(|(context, runtime_inventory_applied)| {
                (
                    self.begin_durable_item_loot_persistence_like_cpp(),
                    DurableItemLootCompletionLikeCpp {
                        owner_guid: context.owner_guid,
                        loot_list_id: context.loot_list_id,
                        player_guid: context.player_guid,
                        item_owner_auto_release: false,
                        durable_item_money_applied_amount: None,
                        durable_item_money_notified_amount: None,
                        durable_item_money_balance_applied: None,
                        item_fanout: durable_item_fanout.clone(),
                        runtime_inventory_applied,
                    },
                )
            });
        let persistence = match spawn_loot_item_persistence_worker_like_cpp(
            async move {
                inventory_persistence
                    .persist_inventory_mutation_like_cpp(persistence_request)
                    .await
            },
            claim.cloned(),
            durable_item_completion,
            self.session_command_tx(),
        ) {
            Ok(persistence) => persistence,
            Err(error) => {
                warn!(?error, "disenchant claim closed before persistence started");
                return false;
            }
        };
        match persistence.await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                warn!(?error, "disenchant material batch transaction failed");
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            }
            Err(error) => {
                warn!(?error, "disenchant material batch worker terminated");
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            }
        }

        for stack in &planned_existing_stacks {
            self.update_inventory_item_object_like_cpp(stack.item_guid, |item| {
                item.set_count(stack.new_count);
                if stack.flags_changed {
                    item.replace_all_item_flags(ItemFieldFlags::from_bits_retain(
                        stack.dynamic_flags,
                    ));
                }
            });
        }

        let mut collection_updates = Vec::new();
        for (stack, db_guid, item_guid) in &created_new_stacks {
            self.insert_inventory_item_like_cpp(
                stack.slot,
                InventoryItem {
                    guid: *item_guid,
                    entry_id: stack.entry_id,
                    db_guid: *db_guid,
                    inventory_type: self.item_template_inventory_type(stack.entry_id),
                },
            );
            let mut item_object = self.make_inventory_item_object(
                *item_guid,
                stack.entry_id,
                player_guid,
                stack.count,
                stack.max_durability,
                loot_item_context(stack.item_context),
                stack.slot,
            );
            self.apply_stored_new_item_flags_like_cpp(stack.entry_id, stack.slot, &mut item_object);
            if stack.random_properties_id != 0 {
                item_object.set_random_properties_id(stack.random_properties_id);
            }
            if stack.random_properties_seed != 0 {
                item_object.set_property_seed(stack.random_properties_seed);
            }
            collection_updates.extend(self.on_item_added_to_collection_like_cpp(&item_object));
            self.insert_inventory_item_object(item_object);
        }
        if let Some(runtime_inventory_applied) = runtime_inventory_applied {
            runtime_inventory_applied.store(true, Ordering::Release);
        }

        for grant in &planned_grants {
            let quest_log_item_id = self
                .load_creature_item_template_addon_loot_metadata_like_cpp(grant.entry.item_id)
                .await
                .quest_log_item_id
                .try_into()
                .unwrap_or(0);
            let mut changed_quest_ids = self
                .apply_quest_source_item_added_non_bound_objective_progress_like_cpp(
                    grant.entry.item_id,
                    quest_log_item_id,
                    grant.entry.quantity,
                )
                .await;
            self.save_changed_represented_quest_statuses_like_cpp(&mut changed_quest_ids)
                .await;
        }

        let map_id = self.player_map_id_like_cpp();
        if !created_new_stacks.is_empty() {
            let item_creates = created_new_stacks
                .iter()
                .map(|(stack, _, item_guid)| ItemCreateData {
                    item_guid: *item_guid,
                    entry_id: stack.entry_id as i32,
                    owner_guid: player_guid,
                    contained_in: player_guid,
                    stack_count: stack.count,
                    dynamic_flags: stack.dynamic_flags,
                    durability: stack.max_durability,
                    max_durability: stack.max_durability,
                    random_properties_seed: stack.random_properties_seed,
                    random_properties_id: stack.random_properties_id,
                    enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
                    gems: Vec::new(),
                    context: stack.item_context,
                    container_slots: 0,
                    container_item_guids: [ObjectGuid::EMPTY; 36],
                })
                .collect();
            self.send_packet(&UpdateObject::create_stored_items(item_creates, map_id));
        }
        for stack in &planned_existing_stacks {
            let update = if stack.flags_changed {
                UpdateObject::item_stack_count_and_flags_update(
                    stack.item_guid,
                    map_id,
                    stack.new_count,
                    stack.dynamic_flags,
                )
            } else {
                UpdateObject::item_stack_count_update(stack.item_guid, map_id, stack.new_count)
            };
            self.send_packet(&update);
        }

        // C++ writes each material's item update on the instance connection
        // before `SendNewItem` routes its push result to the realm connection.
        if !self
            .wait_for_instance_send_before_realm_send_like_cpp()
            .await
        {
            let _ = self.publish_persisted_loot_item_removal_like_cpp(
                claim,
                claim_commit_context,
                durable_item_fanout.as_ref(),
            );
            self.sync_player_registry_state_like_cpp();
            self.kick("loot socket ordering fence failed after durable disenchant claim");
            return true;
        }

        for grant in &planned_grants {
            for push in &grant.existing_pushes {
                self.send_loot_item_push_result(
                    player_guid,
                    push.item_guid,
                    &grant.entry,
                    grant.random_properties.id,
                    grant.random_properties.seed,
                    push.slot,
                    push.added_count,
                    push.new_count,
                    false,
                    dungeon_encounter_id,
                );
            }
            for push in &grant.new_pushes {
                let (stack, _, item_guid) = &created_new_stacks[push.stack_index];
                self.send_loot_item_push_result(
                    player_guid,
                    *item_guid,
                    &grant.entry,
                    stack.random_properties_id,
                    stack.random_properties_seed,
                    stack.slot,
                    push.added_count,
                    push.new_count,
                    false,
                    dungeon_encounter_id,
                );
            }
        }

        // `Loot::AutoStore` completes every realm-routed `SendNewItem` before
        // `LootRoll::Finish` emits the original slot removal on the instance
        // connection. Do not publish the later instance packets without the
        // writer acknowledgement; reconnect will reload the durable grant.
        if !self
            .wait_for_realm_send_before_instance_update_like_cpp()
            .await
        {
            self.sync_player_registry_state_like_cpp();
            self.kick("loot socket ordering fence failed after durable disenchant claim");
            return true;
        }

        // C++ `Loot::AutoStore` performs `StoreNewItem` and `SendNewItem` for
        // every generated material. Only after `AutoStore` returns does
        // `LootRoll::Finish` call `NotifyItemRemoved` for the original loot.
        // SQL and the claim were already committed by the detached worker.
        if !self.publish_persisted_loot_item_removal_like_cpp(
            claim,
            claim_commit_context,
            durable_item_fanout.as_ref(),
        ) {
            return false;
        }

        if !created_new_stacks.is_empty() {
            let changed_slots = created_new_stacks
                .iter()
                .map(|(stack, _, item_guid)| (stack.slot, *item_guid))
                .collect::<Vec<_>>();
            self.send_player_values_update_from_entity_bridge(&changed_slots, &[], &[], &[], None);
        }
        for update in &collection_updates {
            self.send_player_values_update_like_cpp(update);
        }
        self.sync_player_registry_state_like_cpp();
        true
    }

    pub(super) async fn store_direct_loot_item_from_owner_like_cpp(
        &mut self,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
        owner_guid: ObjectGuid,
    ) -> bool {
        self.store_direct_loot_item_with_source_like_cpp(
            loot_entry,
            dungeon_encounter_id,
            owner_guid.is_item().then_some(owner_guid),
            None,
            None,
        )
        .await
    }

    pub(super) async fn store_direct_loot_item_with_source_like_cpp(
        &mut self,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
        stored_item_loot_source: Option<ObjectGuid>,
        claim: Option<&LootClaimLease>,
        claim_commit_context: Option<LootItemClaimCommitContextLikeCpp>,
    ) -> bool {
        let item_id = loot_entry.item_id;
        let count = loot_entry.quantity;
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let durable_item_fanout = match (claim, claim_commit_context) {
            (Some(claim), Some(context)) => {
                let Some(fanout) = self.prepare_durable_loot_item_fanout_like_cpp(claim, context)
                else {
                    return false;
                };
                Some(fanout)
            }
            (None, None) => None,
            _ => return false,
        };
        // C++ Loot::AutoStore validates CanStoreNewItem before StoreNewItem.
        // That ordering still applies when StoreNewItem later converts a
        // quest-bound Item into objective credit and returns nullptr.
        let Some((store_result, mut store_dest, _)) =
            self.plan_store_new_direct_inventory_item(item_id, count)
        else {
            self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
            return false;
        };
        if store_result != InventoryResult::Ok {
            self.send_equip_error(store_result, None, None, 0, 0);
            return false;
        }
        let quest_log_item_id = self
            .load_creature_item_template_addon_loot_metadata_like_cpp(item_id)
            .await
            .quest_log_item_id
            .try_into()
            .unwrap_or(0);
        let bound_objective_plan = self
            .plan_quest_source_item_bound_objective_persistence_like_cpp(
                item_id,
                quest_log_item_id,
                count,
            );
        #[cfg(test)]
        if let Some(grants) = self.loot_item_store_test_grants_like_cpp.clone() {
            let success = self.loot_item_store_test_success_like_cpp;
            let commit_gate = self.loot_item_store_test_commit_gate_like_cpp.clone();
            let materializes_inventory_item = bound_objective_plan.is_none();
            let durable_completion_context = stored_item_loot_source
                .map(|owner_guid| (owner_guid, loot_entry.loot_list_id, player_guid, true))
                .or_else(|| {
                    claim_commit_context.map(|context| {
                        (
                            context.owner_guid,
                            context.loot_list_id,
                            context.player_guid,
                            false,
                        )
                    })
                });
            let runtime_inventory_applied =
                durable_completion_context.map(|_| Arc::new(AtomicBool::new(false)));
            let durable_item_completion = durable_completion_context
                .zip(runtime_inventory_applied.as_ref().map(Arc::clone))
                .map(
                    |(
                        (owner_guid, loot_list_id, player_guid, item_owner_auto_release),
                        runtime_inventory_applied,
                    )| {
                        (
                            self.begin_durable_item_loot_persistence_like_cpp(),
                            DurableItemLootCompletionLikeCpp {
                                owner_guid,
                                loot_list_id,
                                player_guid,
                                item_owner_auto_release,
                                durable_item_money_applied_amount: None,
                                durable_item_money_notified_amount: None,
                                durable_item_money_balance_applied: None,
                                item_fanout: durable_item_fanout.clone(),
                                runtime_inventory_applied,
                            },
                        )
                    },
                );
            let Ok(worker) = spawn_loot_claim_persistence_worker_like_cpp(
                async move {
                    tokio::task::yield_now().await;
                    if let Some(gate) = commit_gate {
                        gate.notified().await;
                    }
                    if !success {
                        return Err(());
                    }
                    if materializes_inventory_item {
                        grants.fetch_add(1, Ordering::SeqCst);
                    }
                    Ok(())
                },
                claim.cloned(),
                durable_item_completion,
            ) else {
                return false;
            };
            if !matches!(worker.await, Ok(Ok(()))) {
                return false;
            }
            if let Some(plan) = bound_objective_plan.as_ref() {
                let applied = self
                    .apply_quest_source_item_bound_objective_preflight_like_cpp(
                        item_id,
                        quest_log_item_id,
                        count,
                    )
                    .await;
                debug_assert!(applied.as_ref().is_some_and(|result| result.no_grant));
                debug_assert!(plan.statuses.iter().all(|planned| {
                    self.player_quests
                        .get(&planned.quest_id)
                        .is_some_and(|actual| {
                            actual.status == planned.status
                                && actual.objective_counts == planned.objective_counts
                        })
                }));
            }
            if !self.publish_persisted_loot_item_removal_like_cpp(
                claim,
                claim_commit_context,
                durable_item_fanout.as_ref(),
            ) {
                return false;
            }
            if bound_objective_plan.is_none() {
                self.send_loot_item_push_result(
                    player_guid,
                    ObjectGuid::EMPTY,
                    loot_entry,
                    0,
                    0,
                    0,
                    count,
                    count,
                    false,
                    dungeon_encounter_id,
                );
            }
            if let Some(runtime_inventory_applied) = runtime_inventory_applied {
                runtime_inventory_applied.store(true, Ordering::Release);
            }
            return true;
        }
        let Some(inventory_persistence) = self.player_inventory_persistence_port_like_cpp() else {
            return false;
        };
        if let Some(bound_objective_plan) = bound_objective_plan {
            let persistence_request =
                wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::LootQuestBoundProgress(
                    wow_persistence::LootQuestBoundProgressPersistenceLikeCpp {
                        owner_guid: player_guid.counter() as u64,
                        quest_statuses: self.represented_quest_status_persistence_rows_like_cpp(
                            &bound_objective_plan.statuses,
                        ),
                        stored_item_source: stored_item_loot_source.map(|item_guid| {
                            wow_persistence::StoredItemLootSourcePersistenceLikeCpp {
                                item_guid: item_guid.counter() as u64,
                                item_id,
                                count,
                                loot_list_id: u32::from(loot_entry.loot_list_id),
                            }
                        }),
                    },
                );

            let durable_completion_context = stored_item_loot_source
                .map(|owner_guid| (owner_guid, loot_entry.loot_list_id, player_guid, true))
                .or_else(|| {
                    claim_commit_context.map(|context| {
                        (
                            context.owner_guid,
                            context.loot_list_id,
                            context.player_guid,
                            false,
                        )
                    })
                });
            let runtime_inventory_applied =
                durable_completion_context.map(|_| Arc::new(AtomicBool::new(false)));
            let durable_item_completion = durable_completion_context
                .zip(runtime_inventory_applied.as_ref().map(Arc::clone))
                .map(
                    |(
                        (owner_guid, loot_list_id, player_guid, item_owner_auto_release),
                        runtime_inventory_applied,
                    )| {
                        (
                            self.begin_durable_item_loot_persistence_like_cpp(),
                            DurableItemLootCompletionLikeCpp {
                                owner_guid,
                                loot_list_id,
                                player_guid,
                                item_owner_auto_release,
                                durable_item_money_applied_amount: None,
                                durable_item_money_notified_amount: None,
                                durable_item_money_balance_applied: None,
                                item_fanout: durable_item_fanout.clone(),
                                runtime_inventory_applied,
                            },
                        )
                    },
                );
            let persistence = match spawn_loot_item_persistence_worker_like_cpp(
                async move {
                    inventory_persistence
                        .persist_inventory_mutation_like_cpp(persistence_request)
                        .await
                },
                claim.cloned(),
                durable_item_completion,
                self.session_command_tx(),
            ) {
                Ok(persistence) => persistence,
                Err(error) => {
                    warn!(
                        ?error,
                        "LootItem: quest-bound claim closed before persistence started"
                    );
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return false;
                }
            };
            match persistence.await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    warn!(?error, "LootItem: quest-bound objective transaction failed");
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return false;
                }
                Err(error) => {
                    warn!(
                        ?error,
                        "LootItem: detached quest-bound transaction worker terminated"
                    );
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return false;
                }
            }

            let applied = self
                .apply_quest_source_item_bound_objective_preflight_like_cpp(
                    item_id,
                    quest_log_item_id,
                    count,
                )
                .await;
            if !applied.as_ref().is_some_and(|result| result.no_grant)
                || !self
                    .player_quest_gameplay_snapshot_like_cpp()
                    .is_some_and(|state| {
                        bound_objective_plan.statuses.iter().all(|planned| {
                            state.statuses.get(&planned.quest_id).is_some_and(|actual| {
                                actual.status == planned.status
                                    && actual.objective_counts == planned.objective_counts
                            })
                        })
                    })
            {
                self.kick("durable quest-bound loot state diverged; relog required");
                return true;
            }
            if let Some(runtime_inventory_applied) = runtime_inventory_applied {
                runtime_inventory_applied.store(true, Ordering::Release);
            }
            if !self.publish_persisted_loot_item_removal_like_cpp(
                claim,
                claim_commit_context,
                durable_item_fanout.as_ref(),
            ) {
                return false;
            }
            self.sync_player_registry_state_like_cpp();
            return true;
        }
        let store_random_properties = {
            let mut rng = self.represented_runtime_subrng_like_cpp();
            self.generate_loot_store_random_properties_with_rng_like_cpp(item_id, &mut rng)
        };

        if store_dest.iter().any(|dest| {
            let bag = (dest.pos >> 8) as u8;
            let slot = (dest.pos & 0x00FF) as u8;
            bag == u8::from(INVENTORY_SLOT_BAG_0)
                && self
                    .resolved_inventory_item_like_cpp(slot)
                    .is_some_and(|existing| {
                        self.resolved_inventory_item_object_like_cpp(existing.guid)
                            .is_some_and(|item| {
                                !loot_store_data_can_stack_with_item(
                                    loot_entry,
                                    store_random_properties,
                                    &item,
                                )
                            })
                    })
        }) {
            let Some(compatible_dest) = self.plan_direct_loot_item_preserving_cpp_store_metadata(
                loot_entry,
                store_random_properties,
            ) else {
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            };
            store_dest = compatible_dest;
        }

        let mut planned_existing_counts = Vec::<PlannedDirectLootExistingStack>::new();
        let mut planned_new_stacks = Vec::<PlannedLootNewStack>::new();

        for dest in store_dest {
            let bag = (dest.pos >> 8) as u8;
            let slot = (dest.pos & 0x00FF) as u8;
            if bag != u8::from(INVENTORY_SLOT_BAG_0) {
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            }

            let max_stack = self
                .item_storage_template(item_id)
                .map(|template| template.max_stack_size)
                .unwrap_or(1)
                .max(1);

            if let Some(existing) = self.resolved_inventory_item_like_cpp(slot) {
                let Some(existing_object) =
                    self.resolved_inventory_item_object_like_cpp(existing.guid)
                else {
                    self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
                    return false;
                };
                let base_count = planned_existing_counts
                    .iter()
                    .find(|planned| planned.slot == slot)
                    .map(|planned| planned.new_count)
                    .unwrap_or_else(|| existing_object.count());
                let new_count = base_count.saturating_add(dest.count);
                if existing.entry_id != item_id
                    || new_count > max_stack
                    || !loot_store_data_can_stack_with_item(
                        loot_entry,
                        store_random_properties,
                        &existing_object,
                    )
                {
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return false;
                }
                if let Some(existing_plan) = planned_existing_counts
                    .iter_mut()
                    .find(|planned| planned.slot == slot)
                {
                    existing_plan.new_count = new_count;
                    existing_plan.added_count =
                        existing_plan.added_count.saturating_add(dest.count);
                } else {
                    let dynamic_flags = self.stored_existing_item_dynamic_flags_like_cpp(
                        item_id,
                        slot,
                        &existing_object,
                    );
                    planned_existing_counts.push(PlannedDirectLootExistingStack {
                        slot,
                        item_guid: existing.guid,
                        db_guid: existing.db_guid,
                        new_count,
                        added_count: dest.count,
                        dynamic_flags,
                        flags_changed: dynamic_flags != existing_object.item_flags_bits(),
                    });
                }
                continue;
            }

            if let Some(stack) = planned_new_stacks
                .iter_mut()
                .find(|stack| stack.slot == slot)
            {
                if stack.entry_id == item_id
                    && stack.random_properties_id == store_random_properties.id
                    && stack.random_properties_seed == store_random_properties.seed
                    && stack.item_context == loot_entry.item_context
                    && stack.count.saturating_add(dest.count) <= max_stack
                {
                    stack.count = stack.count.saturating_add(dest.count);
                    continue;
                }
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            }

            planned_new_stacks.push(PlannedLootNewStack {
                slot,
                entry_id: item_id,
                count: dest.count,
                max_durability: self.item_template_max_durability(item_id),
                dynamic_flags: self.stored_new_item_dynamic_flags_like_cpp(item_id, slot),
                random_properties_id: store_random_properties.id,
                random_properties_seed: store_random_properties.seed,
                item_context: loot_entry.item_context,
            });
        }

        let mut created_new_stacks = Vec::new();
        if !planned_new_stacks.is_empty() {
            let Some(allocated_guids) =
                self.allocate_item_instance_guids_like_cpp(planned_new_stacks.len())
            else {
                warn!(
                    count = planned_new_stacks.len(),
                    "loot item grant has no process-wide item GUID allocator"
                );
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            };

            for (stack, (db_guid, item_guid)) in planned_new_stacks.iter().zip(allocated_guids) {
                created_new_stacks.push((stack.clone(), db_guid, item_guid));
            }
        }

        // Item-container loot is a move between two durable stores. The
        // semantic request keeps source deletion in the same transaction as
        // every destination stack, so a crash cannot duplicate or lose it.
        let persistence_request =
            wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::LootDirectItemGrant(
                wow_persistence::LootDirectItemGrantPersistenceLikeCpp {
                    existing_stacks: planned_existing_counts
                        .iter()
                        .map(
                            |stack| wow_persistence::LootExistingStackPersistenceLikeCpp {
                                item_guid: stack.db_guid,
                                new_count: stack.new_count,
                                dynamic_flags: stack.flags_changed.then_some(stack.dynamic_flags),
                            },
                        )
                        .collect(),
                    new_stacks: created_new_stacks
                        .iter()
                        .map(|(stack, db_guid, _)| {
                            wow_persistence::LootNewStackPersistenceLikeCpp {
                                item_guid: *db_guid,
                                entry_id: stack.entry_id,
                                owner_guid: player_guid.counter() as u64,
                                count: stack.count,
                                max_durability: stack.max_durability,
                                dynamic_flags: stack.dynamic_flags,
                                random_properties_id: stack.random_properties_id,
                                random_properties_seed: stack.random_properties_seed,
                                item_context: stack.item_context,
                                slot: stack.slot,
                            }
                        })
                        .collect(),
                    stored_item_source: stored_item_loot_source.map(|item_guid| {
                        wow_persistence::StoredItemLootSourcePersistenceLikeCpp {
                            item_guid: item_guid.counter() as u64,
                            item_id,
                            count,
                            loot_list_id: u32::from(loot_entry.loot_list_id),
                        }
                    }),
                },
            );

        let durable_claim = claim.cloned();
        let durable_completion_context = stored_item_loot_source
            .map(|owner_guid| (owner_guid, loot_entry.loot_list_id, player_guid, true))
            .or_else(|| {
                claim_commit_context.map(|context| {
                    (
                        context.owner_guid,
                        context.loot_list_id,
                        context.player_guid,
                        false,
                    )
                })
            });
        let runtime_inventory_applied =
            durable_completion_context.map(|_| Arc::new(AtomicBool::new(false)));
        let durable_item_completion = durable_completion_context
            .zip(runtime_inventory_applied.as_ref().map(Arc::clone))
            .map(
                |(
                    (owner_guid, loot_list_id, player_guid, item_owner_auto_release),
                    runtime_inventory_applied,
                )| {
                    (
                        self.begin_durable_item_loot_persistence_like_cpp(),
                        DurableItemLootCompletionLikeCpp {
                            owner_guid,
                            loot_list_id,
                            player_guid,
                            item_owner_auto_release,
                            durable_item_money_applied_amount: None,
                            durable_item_money_notified_amount: None,
                            durable_item_money_balance_applied: None,
                            item_fanout: durable_item_fanout.clone(),
                            runtime_inventory_applied,
                        },
                    )
                },
            );
        let persistence = match spawn_loot_item_persistence_worker_like_cpp(
            async move {
                inventory_persistence
                    .persist_inventory_mutation_like_cpp(persistence_request)
                    .await
            },
            durable_claim,
            durable_item_completion,
            self.session_command_tx(),
        ) {
            Ok(persistence) => persistence,
            Err(error) => {
                warn!(?error, "LootItem: claim closed before persistence started");
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            }
        };
        let persistence_result = match persistence.await {
            Ok(result) => result,
            Err(error) => {
                warn!(
                    ?error,
                    "LootItem: detached store transaction worker terminated"
                );
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            }
        };
        if let Err(e) = persistence_result {
            warn!("LootItem: store transaction failed: {e:?}");
            self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
            return false;
        }

        for stack in &planned_existing_counts {
            self.update_inventory_item_object_like_cpp(stack.item_guid, |item| {
                item.set_count(stack.new_count);
                if stack.flags_changed {
                    item.replace_all_item_flags(ItemFieldFlags::from_bits_retain(
                        stack.dynamic_flags,
                    ));
                }
            });
        }

        let mut collection_updates = Vec::new();
        for (stack, db_guid, item_guid) in &created_new_stacks {
            self.insert_inventory_item_like_cpp(
                stack.slot,
                InventoryItem {
                    guid: *item_guid,
                    entry_id: stack.entry_id,
                    db_guid: *db_guid,
                    inventory_type: self.item_template_inventory_type(stack.entry_id),
                },
            );
            let mut item_object = self.make_inventory_item_object(
                *item_guid,
                stack.entry_id,
                player_guid,
                stack.count,
                stack.max_durability,
                loot_item_context(stack.item_context),
                stack.slot,
            );
            self.apply_stored_new_item_flags_like_cpp(stack.entry_id, stack.slot, &mut item_object);
            if stack.random_properties_id != 0 {
                item_object.set_random_properties_id(stack.random_properties_id);
            }
            if stack.random_properties_seed != 0 {
                item_object.set_property_seed(stack.random_properties_seed);
            }
            collection_updates.extend(self.on_item_added_to_collection_like_cpp(&item_object));
            self.insert_inventory_item_object(item_object);
        }
        if let Some(runtime_inventory_applied) = runtime_inventory_applied {
            runtime_inventory_applied.store(true, Ordering::Release);
        }

        let mut changed_quest_ids = self
            .apply_quest_source_item_added_non_bound_objective_progress_like_cpp(
                item_id,
                quest_log_item_id,
                count,
            )
            .await;
        self.save_changed_represented_quest_statuses_like_cpp(&mut changed_quest_ids)
            .await;

        let map_id = self.player_map_id_like_cpp();
        if !created_new_stacks.is_empty() {
            let item_creates = created_new_stacks
                .iter()
                .map(|(stack, _, item_guid)| ItemCreateData {
                    item_guid: *item_guid,
                    entry_id: stack.entry_id as i32,
                    owner_guid: player_guid,
                    contained_in: player_guid,
                    stack_count: stack.count,
                    dynamic_flags: stack.dynamic_flags,
                    durability: stack.max_durability,
                    max_durability: stack.max_durability,
                    random_properties_seed: stack.random_properties_seed,
                    random_properties_id: stack.random_properties_id,
                    enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
                    gems: Vec::new(),
                    context: stack.item_context,
                    container_slots: 0,
                    container_item_guids: [ObjectGuid::EMPTY; 36],
                })
                .collect();
            self.send_packet(&UpdateObject::create_stored_items(item_creates, map_id));
        }

        for stack in &planned_existing_counts {
            let update = if stack.flags_changed {
                UpdateObject::item_stack_count_and_flags_update(
                    stack.item_guid,
                    map_id,
                    stack.new_count,
                    stack.dynamic_flags,
                )
            } else {
                UpdateObject::item_stack_count_update(stack.item_guid, map_id, stack.new_count)
            };
            self.send_packet(&update);
        }

        // The worker committed SQL and the authority claim before runtime
        // publication. C++ `StoreNewItem` sends the stored item's update,
        // `Player::StoreLootItem` then notifies removal, and only afterwards
        // does `SendNewItem` emit `SMSG_ITEM_PUSH_RESULT`.
        if !self.publish_persisted_loot_item_removal_like_cpp(
            claim,
            claim_commit_context,
            durable_item_fanout.as_ref(),
        ) {
            return false;
        }

        if !self
            .wait_for_instance_send_before_realm_send_like_cpp()
            .await
        {
            self.sync_player_registry_state_like_cpp();
            self.kick("loot socket ordering fence failed after durable item claim");
            return true;
        }

        for stack in &planned_existing_counts {
            self.send_loot_item_push_result(
                player_guid,
                stack.item_guid,
                loot_entry,
                store_random_properties.id,
                store_random_properties.seed,
                stack.slot,
                stack.added_count,
                stack.new_count,
                false,
                dungeon_encounter_id,
            );
        }

        for (stack, _, item_guid) in &created_new_stacks {
            self.send_loot_item_push_result(
                player_guid,
                *item_guid,
                loot_entry,
                stack.random_properties_id,
                stack.random_properties_seed,
                stack.slot,
                stack.count,
                stack.count,
                false,
                dungeon_encounter_id,
            );
        }

        if (!created_new_stacks.is_empty() || !collection_updates.is_empty())
            && !self
                .wait_for_realm_send_before_instance_update_like_cpp()
                .await
        {
            self.sync_player_registry_state_like_cpp();
            self.kick("loot socket ordering fence failed after durable item claim");
            return true;
        }

        if !created_new_stacks.is_empty() {
            let changed_slots: Vec<_> = created_new_stacks
                .iter()
                .map(|(stack, _, item_guid)| (stack.slot, *item_guid))
                .collect();
            self.send_player_values_update_from_entity_bridge(&changed_slots, &[], &[], &[], None);
        }
        for update in &collection_updates {
            self.send_player_values_update_like_cpp(update);
        }

        self.sync_player_registry_state_like_cpp();
        true
    }

    fn plan_direct_loot_item_preserving_cpp_store_metadata(
        &self,
        loot_entry: &LootEntry,
        random_properties: LootStoreRandomProperties,
    ) -> Option<Vec<ItemPosCount>> {
        let max_stack = self
            .item_storage_template(loot_entry.item_id)
            .map(|template| template.max_stack_size)
            .unwrap_or(1)
            .max(1);
        let mut remaining = loot_entry.quantity;
        let mut dest = Vec::new();

        let mut existing_slots: Vec<u8> = self
            .resolved_inventory_items_like_cpp()?
            .keys()
            .copied()
            .collect();
        existing_slots.sort_unstable();
        for slot in existing_slots {
            if remaining == 0 {
                break;
            }
            let Some(existing) = self.resolved_inventory_item_like_cpp(slot) else {
                continue;
            };
            let Some(existing_object) = self.resolved_inventory_item_object_like_cpp(existing.guid)
            else {
                continue;
            };
            if existing.entry_id != loot_entry.item_id
                || !loot_store_data_can_stack_with_item(
                    loot_entry,
                    random_properties,
                    &existing_object,
                )
                || existing_object.count() >= max_stack
            {
                continue;
            }
            let can_add = max_stack
                .saturating_sub(existing_object.count())
                .min(remaining);
            if can_add > 0 {
                dest.push(ItemPosCount::new(
                    make_item_pos(INVENTORY_SLOT_BAG_0, slot),
                    can_add,
                ));
                remaining = remaining.saturating_sub(can_add);
            }
        }

        let backpack_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(INVENTORY_DEFAULT_SIZE)
            .min(INVENTORY_SLOT_ITEM_END);
        for slot in INVENTORY_SLOT_ITEM_START..backpack_end {
            if remaining == 0 {
                break;
            }
            if self
                .resolved_inventory_items_like_cpp()
                .is_none_or(|items| items.contains_key(&slot))
            {
                continue;
            }
            let quantity = max_stack.min(remaining);
            dest.push(ItemPosCount::new(
                make_item_pos(INVENTORY_SLOT_BAG_0, slot),
                quantity,
            ));
            remaining = remaining.saturating_sub(quantity);
        }

        (remaining == 0).then_some(dest)
    }

    pub(super) async fn destroy_fully_looted_direct_item(&mut self, item_guid: ObjectGuid) {
        self.destroy_direct_item_count_after_loot_release_like_cpp(item_guid, None)
            .await;
    }
}
