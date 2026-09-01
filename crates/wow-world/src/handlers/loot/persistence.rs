// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Durable loot persistence and its worker.

use super::*;

impl WorldSession {
    pub(super) fn prepare_durable_loot_item_fanout_like_cpp(
        &mut self,
        claim: &LootClaimLease,
        context: LootItemClaimCommitContextLikeCpp,
    ) -> Option<DurableLootItemFanoutLikeCpp> {
        let authority = self
            .represented_owned_loot_authority_like_cpp(context.owner_guid)
            .filter(|authority| claim.shares_authority_like_cpp(authority))?;
        let precommit_snapshot = authority
            .snapshot_for_player_like_cpp(context.player_guid)
            .filter(|snapshot| {
                snapshot.generation == claim.generation_like_cpp()
                    && snapshot.loot.loot_guid == context.loot_obj
                    && snapshot
                        .loot
                        .items
                        .iter()
                        .any(|entry| entry.loot_list_id == context.loot_list_id)
            })?;
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        Some(DurableLootItemFanoutLikeCpp {
            owner_guid: context.owner_guid,
            loot_obj: context.loot_obj,
            loot_list_id: context.loot_list_id,
            player_guid: context.player_guid,
            free_for_all: context.free_for_all,
            authority,
            authority_generation: claim.generation_like_cpp(),
            precommit_snapshot,
            committed_snapshot: Arc::new(std::sync::OnceLock::new()),
            source_send_tx: self.send_tx().clone(),
            player_registry: self.player_registry().cloned(),
            map_id: self.player_map_id_like_cpp(),
            instance_id,
            published: Arc::new(AtomicBool::new(false)),
        })
    }

    fn publish_durable_loot_item_fanout_like_cpp(
        &mut self,
        route: &DurableLootItemFanoutLikeCpp,
    ) -> bool {
        let Some(committed_snapshot) = route.committed_snapshot.get().filter(|snapshot| {
            snapshot.generation == route.authority_generation
                && snapshot.loot.loot_guid == route.loot_obj
        }) else {
            // Never replace the serialization cut with a later authority
            // sample. The latter may include a viewer that opened after the
            // item commit and already received a response without this slot.
            return false;
        };
        if route
            .published
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return true;
        }

        // C++ serializes StoreLootItem before a later LootRelease and notifies
        // synchronously. Preserve that already ordered cohort across Rust's
        // SQL wait, then add only viewers captured by the item mutation. A
        // post-COMMIT opener is excluded because it saw the removed slot.
        let viewers = durable_loot_item_fanout_viewers_like_cpp(
            &route.precommit_snapshot.loot.players_looting,
            &committed_snapshot.loot.players_looting,
        );
        let Some(entry) = committed_snapshot
            .loot
            .items
            .iter()
            .find(|entry| entry.loot_list_id == route.loot_list_id)
        else {
            return false;
        };
        let allowed_looters = entry
            .allowed_looters
            .iter()
            .copied()
            .collect::<HashSet<_>>();
        let packet = LootRemoved {
            owner: route.owner_guid,
            loot_obj: route.loot_obj,
            loot_list_id: route.loot_list_id,
        };
        let bytes = packet.to_bytes();
        let mut stale_viewers = Vec::new();

        if route.free_for_all {
            let _ = route.source_send_tx.send(bytes);
        } else {
            for viewer in viewers {
                if !allowed_looters.contains(&viewer) {
                    continue;
                }
                if viewer == route.player_guid {
                    let _ = route.source_send_tx.send(bytes.clone());
                    continue;
                }
                let Some(registry) = route.player_registry.as_ref() else {
                    stale_viewers.push(viewer);
                    continue;
                };
                let Some(registration) =
                    registry.loot_delivery_recipient(viewer, route.map_id, route.instance_id)
                else {
                    stale_viewers.push(viewer);
                    continue;
                };
                if registry
                    .send_current_packet(registration, bytes.clone())
                    .is_err()
                {
                    stale_viewers.push(viewer);
                }
            }
        }

        for viewer in stale_viewers {
            let _ = route
                .authority
                .remove_viewer_if_generation_like_cpp(route.authority_generation, viewer);
        }

        self.refresh_owned_loot_summary_like_cpp(route.owner_guid);
        if self.player_guid() == Some(route.player_guid) {
            let _ =
                self.reconcile_represented_loot_cache_like_cpp(route.owner_guid, route.player_guid);
        }
        self.finalize_unviewed_durable_loot_owner_like_cpp(route);
        true
    }

    fn finalize_unviewed_durable_loot_owner_like_cpp(
        &mut self,
        route: &DurableLootItemFanoutLikeCpp,
    ) {
        let same_view_still_open = self
            .active_loot_view_authorities_like_cpp
            .get(&route.owner_guid)
            .is_some_and(|authority| authority.shares_storage_like_cpp(&route.authority))
            && self
                .active_loot_view_generations_like_cpp
                .get(&route.owner_guid)
                .is_some_and(|generation| *generation == route.authority_generation);
        if same_view_still_open {
            return;
        }
        if !self
            .represented_owned_loot_authority_like_cpp(route.owner_guid)
            .is_some_and(|authority| authority.shares_storage_like_cpp(&route.authority))
        {
            return;
        }
        let Some(observation) = route
            .authority
            .fully_looted_unviewed_lifecycle_observation_like_cpp()
        else {
            return;
        };
        let Some(snapshot) = route
            .authority
            .snapshot_for_player_like_cpp(route.player_guid)
            .filter(|snapshot| snapshot.generation == route.authority_generation)
        else {
            return;
        };

        self.loot_table
            .insert(route.owner_guid, snapshot.loot.clone());
        self.represented_loot_cache_generations_like_cpp
            .insert(route.owner_guid, snapshot.generation);

        if route.owner_guid.is_game_object() {
            let release = AuthoritativeLootReleaseLikeCpp {
                authority: route.authority.clone(),
                selected_generation: route.authority_generation,
                loot: snapshot.loot,
                whole_object_fully_looted: true,
                whole_object_fully_skinned: observation.whole_object_fully_skinned,
                object_generation: observation.object_generation,
                lifecycle_revision: observation.lifecycle_revision,
                require_no_viewers: true,
            };
            self.apply_represented_gameobject_loot_release_like_cpp(
                route.owner_guid,
                route.player_guid,
                true,
                true,
                Some(&release),
            );
            let _ =
                self.queue_chest_gameobject_state_refresh_for_same_map_like_cpp(route.owner_guid);
            self.hide_represented_gameobject_for_player_after_loot_release_like_cpp(
                route.owner_guid,
            );
            if self
                .represented_gameobject_use_states
                .get(&route.owner_guid)
                .and_then(|state| state.go_type)
                .map(u32::from)
                == Some(GAMEOBJECT_TYPE_GATHERING_NODE)
            {
                self.send_gathering_node_loot_release_dynamic_flags_update_like_cpp(
                    route.owner_guid,
                );
            }
            self.loot_table.remove(&route.owner_guid);
            return;
        }

        if route.owner_guid.is_corpse() {
            self.remove_canonical_corpse_lootable_dynamic_flag_if_unviewed_fully_looted_observation_like_cpp(
                route.owner_guid,
                &route.authority,
                observation.object_generation,
                observation.lifecycle_revision,
            );
            self.loot_table.remove(&route.owner_guid);
            return;
        }

        if !route.owner_guid.is_creature_or_vehicle() {
            return;
        }

        let corpse_decay_looted_rate = self.loot_drop_rates_like_cpp().corpse_decay_looted;
        let whole_object_fully_skinned = observation.whole_object_fully_skinned;
        let lifecycle_update = self
            .mutate_world_creature_if_unviewed_fully_looted_observation_like_cpp(
                route.owner_guid,
                &route.authority,
                observation.object_generation,
                observation.lifecycle_revision,
                |creature| {
                    creature.force_dynamic_flags_update_like_cpp();
                    creature.remove_lootable_dynamic_flag_like_cpp();
                    let marked = if creature.is_alive() {
                        None
                    } else {
                        let corpse_decay_secs = looted_corpse_decay_secs_like_cpp(
                            whole_object_fully_skinned,
                            creature.corpse_delay_secs_like_cpp(),
                            creature.ignore_corpse_decay_ratio_like_cpp(),
                            corpse_decay_looted_rate,
                        );
                        creature
                            .all_loot_removed_from_corpse_like_cpp(
                                corpse_decay_looted_rate,
                                whole_object_fully_skinned,
                            )
                            .then_some((creature.entry(), corpse_decay_secs))
                    };
                    (marked, creature.creature.unit().values_update())
                },
            );
        self.loot_table.remove(&route.owner_guid);
        if let Some((_, values_update)) = lifecycle_update.as_ref() {
            self.send_creature_loot_release_dynamic_flags_update_like_cpp(
                route.owner_guid,
                values_update,
                Some(&route.authority),
            );
        }
        let marked = lifecycle_update.and_then(|(marked, _)| marked);
        if let Some((entry, corpse_decay_secs)) = marked {
            info!(
                "Creature {:?} (entry {}) fully looted after durable claim — despawning in {}s",
                route.owner_guid, entry, corpse_decay_secs
            );
        }
    }

    fn commit_represented_loot_item_claim_like_cpp(
        &mut self,
        claim: &LootClaimLease,
        context: LootItemClaimCommitContextLikeCpp,
        fanout: Option<&DurableLootItemFanoutLikeCpp>,
    ) -> bool {
        if !claim.is_committed_like_cpp() {
            match claim.commit_with_snapshot_like_cpp() {
                Ok((_, Some(snapshot))) => {
                    if let Some(fanout) = fanout {
                        let _ = fanout.committed_snapshot.set(snapshot);
                    }
                }
                Ok((_, None)) => {
                    warn!(
                        owner = ?context.owner_guid,
                        loot_list_id = context.loot_list_id,
                        "durable loot item committed without an exact authority snapshot"
                    );
                    return false;
                }
                Err(error) => {
                    warn!(
                        owner = ?context.owner_guid,
                        loot_list_id = context.loot_list_id,
                        ?error,
                        "durable loot item could not commit its object-owned claim"
                    );
                    return false;
                }
            }
        }
        let Some(fanout) = fanout.filter(|fanout| {
            fanout.owner_guid == context.owner_guid
                && fanout.loot_obj == context.loot_obj
                && fanout.loot_list_id == context.loot_list_id
                && fanout.player_guid == context.player_guid
                && fanout.free_for_all == context.free_for_all
                && claim.shares_authority_like_cpp(&fanout.authority)
                && claim.generation_like_cpp() == fanout.authority_generation
        }) else {
            warn!(
                owner = ?context.owner_guid,
                loot_list_id = context.loot_list_id,
                "durable loot item committed without its retained fanout route"
            );
            return false;
        };
        self.publish_durable_loot_item_fanout_like_cpp(fanout)
    }

    pub(super) fn publish_persisted_loot_item_removal_like_cpp(
        &mut self,
        claim: Option<&LootClaimLease>,
        context: Option<LootItemClaimCommitContextLikeCpp>,
        fanout: Option<&DurableLootItemFanoutLikeCpp>,
    ) -> bool {
        match (claim, context, fanout) {
            (None, None, None) => true,
            (Some(claim), Some(context), fanout) => {
                self.commit_represented_loot_item_claim_like_cpp(claim, context, fanout)
            }
            _ => {
                warn!("durable loot item claim/context mismatch before removal publication");
                false
            }
        }
    }

    /// Publishes successful durable loot transactions that outlived their
    /// packet waiter. This replays committed Item-owned money or item grants
    /// into runtime state before disconnect can persist a stale snapshot. C++
    /// auto-releases only when `Loot::isLooted()` (zero coins and no visible
    /// items) and the owner GUID is an Item.
    pub(crate) async fn apply_pending_durable_item_loot_completions_like_cpp(&mut self) {
        self.apply_pending_durable_item_loot_completions_with_objective_drain_like_cpp(true)
            .await;
    }

    pub(crate) async fn apply_pending_durable_item_loot_completions_with_objective_drain_like_cpp(
        &mut self,
        drain_money_objectives: bool,
    ) {
        let completions = self.take_durable_item_loot_completions_like_cpp();
        for completion in completions {
            if let Some(fanout) = completion.item_fanout.as_ref() {
                let _ = self.publish_durable_loot_item_fanout_like_cpp(fanout);
            }
            let requires_runtime_recovery =
                !completion.runtime_inventory_applied.load(Ordering::Acquire);
            let targets_current_player = self.player_guid() == Some(completion.player_guid);

            if let Some(applied_delta) = completion.durable_item_money_applied_amount {
                let apply_balance = targets_current_player
                    && completion
                        .durable_item_money_balance_applied
                        .as_ref()
                        .is_some_and(|applied| {
                            applied
                                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                                .is_ok()
                        });
                let publish = targets_current_player
                    && completion
                        .runtime_inventory_applied
                        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok();
                let Some(old_money) = self.resolved_player_money_like_cpp() else {
                    continue;
                };
                if apply_balance {
                    let new_money = old_money
                        .checked_add(applied_delta)
                        .filter(|money| *money <= MAX_MONEY_AMOUNT)
                        .unwrap_or(old_money);
                    if !self.set_player_gold_like_cpp(new_money) {
                        continue;
                    }
                    if old_money != new_money {
                        self.enqueue_represented_quest_objective_progress_like_cpp(
                            RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                                old_money,
                                new_money,
                            },
                        );
                    }
                }
                if publish {
                    self.represented_notify_money_removed_like_cpp(completion.owner_guid);
                    self.send_packet(&LootMoneyNotify {
                        money: completion
                            .durable_item_money_notified_amount
                            .expect("durable Item money completion retains notification amount"),
                        money_mod: 0,
                        sole_looter: true,
                    });

                    let fully_looted =
                        self.loot_table
                            .get_mut(&completion.owner_guid)
                            .is_some_and(|loot| {
                                loot.coins = 0;
                                loot_is_looted_like_cpp(loot)
                            });
                    if fully_looted {
                        // Source Item destruction remains owned by the normal
                        // C++ release phase; the completion only makes the
                        // already-durable money mutation visible first.
                        self.do_loot_release_owner_like_cpp(
                            completion.owner_guid,
                            completion.player_guid,
                        )
                        .await;
                    }
                }
                if drain_money_objectives && (apply_balance || publish) {
                    self.drain_represented_quest_objective_progress_like_cpp()
                        .await;
                }
                continue;
            }

            if targets_current_player && completion.item_owner_auto_release {
                debug_assert!(completion.owner_guid.is_item());
                let removal = self
                    .loot_table
                    .get_mut(&completion.owner_guid)
                    .and_then(|loot| {
                        let entry = loot
                            .items
                            .iter()
                            .find(|entry| entry.loot_list_id == completion.loot_list_id)?;
                        let free_for_all = entry.flags.freeforall;
                        let newly_removed = !loot_item_is_looted_for_player_like_cpp(
                            loot,
                            entry,
                            completion.player_guid,
                        );
                        let loot_obj = loot.loot_guid;
                        mark_loot_item_looted_for_player_like_cpp(
                            loot,
                            completion.loot_list_id,
                            completion.player_guid,
                        );
                        Some((
                            loot_obj,
                            free_for_all,
                            newly_removed,
                            loot_is_looted_like_cpp(loot),
                        ))
                    });

                if let Some((loot_obj, free_for_all, newly_removed, fully_looted)) = removal {
                    if newly_removed {
                        if free_for_all {
                            self.send_packet(&LootRemoved {
                                owner: completion.owner_guid,
                                loot_obj,
                                loot_list_id: completion.loot_list_id,
                            });
                        } else {
                            self.represented_notify_loot_item_removed_like_cpp(
                                completion.owner_guid,
                                completion.loot_list_id,
                            );
                        }
                    }

                    if fully_looted {
                        self.do_loot_release_owner_like_cpp(
                            completion.owner_guid,
                            completion.player_guid,
                        )
                        .await;
                    }
                }
            } else if targets_current_player && requires_runtime_recovery {
                // The detached worker already committed the authority. Refresh
                // the packet cache so disconnect's DoLootReleaseAll observes
                // the consumed claim rather than the pre-commit session copy.
                self.refresh_owned_loot_summary_like_cpp(completion.owner_guid);
                let _ = self.reconcile_represented_loot_cache_like_cpp(
                    completion.owner_guid,
                    completion.player_guid,
                );
            }

            if requires_runtime_recovery {
                // SQL committed after the packet waiter disappeared, before
                // its synchronous runtime inventory publication. Do not let
                // the player operate on a stale slot; the persisted grant and
                // source consumption are reconstructed on the next login.
                self.kick("durable loot item completed after handler cancellation; relog required");
            }
        }
    }

    pub(crate) async fn wait_for_active_loot_persistence_like_cpp(&mut self) {
        let mut authorities = Vec::<OwnedLootAuthority>::new();
        for authority in self.active_loot_view_authorities_like_cpp.values() {
            if authorities
                .iter()
                .any(|existing| existing.shares_storage_like_cpp(authority))
            {
                continue;
            }
            authorities.push(authority.clone());
        }
        for authority in authorities {
            authority.wait_for_persisting_claims_like_cpp().await;
        }
        self.wait_for_durable_item_loot_persistence_like_cpp().await;
        self.apply_pending_durable_item_loot_completions_like_cpp()
            .await;
    }

    /// Retire the detached Rust representation of Loot owned by an Item that
    /// a durable transaction has committed to destroy. C++ gets the same
    /// window teardown from destroying the Item and its owned `Loot`; this is
    /// deliberately narrower than `DoLootReleaseAll` and cannot consume or
    /// otherwise mutate an unrelated active loot owner.
    pub(crate) fn retire_committed_destroyed_item_loot_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        if self.active_loot_view_owners.contains(&item_guid) || self.is_active_loot_guid(item_guid)
        {
            self.close_stale_active_loot_view_like_cpp(item_guid, player_guid);
        }
        self.loot_table.remove(&item_guid);
    }
}
