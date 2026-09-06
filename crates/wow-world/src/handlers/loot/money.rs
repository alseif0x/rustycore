// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot money application and distribution.

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use super::*;

impl WorldSession {
    pub(super) fn represented_loot_money_recipients_like_cpp(
        &self,
        loot_guid: ObjectGuid,
    ) -> Vec<ObjectGuid> {
        let Some(player_guid) = self.player_guid() else {
            return Vec::new();
        };

        let Some(loot) = self.loot_table.get(&loot_guid) else {
            return vec![player_guid];
        };
        // C++ shares only LOOT_CORPSE. Pickpocket money is creature-owned but
        // personal; vehicle corpses still share even though their HighGuid is
        // not Creature (`LootHandler.cpp::HandleLootMoneyOpcode`).
        if loot.loot_type != LOOT_TYPE_CORPSE_LIKE_CPP {
            return vec![player_guid];
        }

        let (Some(group_guid), Some(group_registry), Some(player_registry)) = (
            self.resolved_group_guid_like_cpp(),
            self.group_registry(),
            self.player_registry(),
        ) else {
            return vec![player_guid];
        };

        let Some(group) = group_registry.get(&group_guid) else {
            return vec![player_guid];
        };

        let Some(source_position) = self.player_position_like_cpp() else {
            return vec![player_guid];
        };
        let mut source_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id);
        // Old packet fixtures have no canonical map resident. They must still
        // provide an explicit routing-directory placement; never invent an
        // instance identifier for them.
        #[cfg(test)]
        if source_instance_id.is_none() {
            source_instance_id = player_registry
                .loot_presence(player_guid)
                .map(|presence| presence.instance_id);
        }
        let Some(source_instance_id) = source_instance_id else {
            return vec![player_guid];
        };
        let mut recipients = Vec::new();

        for member_guid in &group.members {
            if !loot.allowed_looters.contains(member_guid) {
                continue;
            }

            if *member_guid == player_guid {
                recipients.push(*member_guid);
                continue;
            }

            let Some(member) = player_registry.loot_presence(*member_guid) else {
                continue;
            };

            if !member.is_in_world
                || member.map_id != self.player_map_id_like_cpp()
                || member.instance_id != source_instance_id
            {
                continue;
            }

            if self.current_map_is_dungeon_like_cpp()
                || source_position.is_within_dist(&member.position, 74.0)
            {
                recipients.push(*member_guid);
            }
        }

        if recipients.is_empty() {
            recipients.push(player_guid);
        }

        recipients
    }

    pub(super) fn represented_loot_money_for_player_like_cpp(
        &self,
        loot_guid: ObjectGuid,
        loot: &CreatureLoot,
        player_guid: ObjectGuid,
    ) -> u32 {
        if self.represented_personal_loot_owners.contains(&loot_guid) {
            return self
                .represented_personal_loot_money
                .get(&(loot_guid, player_guid))
                .copied()
                .unwrap_or(0);
        }

        loot.coins
    }

    pub(super) fn represented_loot_money_command_targets_active_generation_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        expected_authority: &OwnedLootAuthority,
        authority_generation: u64,
    ) -> bool {
        if !self
            .active_loot_view_authorities_like_cpp
            .get(&owner_guid)
            .is_some_and(|active| active.shares_storage_like_cpp(expected_authority))
            || !self
                .active_loot_view_generations_like_cpp
                .get(&owner_guid)
                .is_some_and(|active| *active == authority_generation)
        {
            return false;
        }
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        self.represented_owned_loot_authority_like_cpp(owner_guid)
            .is_some_and(|authority| {
                authority.shares_storage_like_cpp(expected_authority)
                    && authority
                        .snapshot_for_player_like_cpp(player_guid)
                        .is_some_and(|snapshot| snapshot.generation == authority_generation)
            })
    }

    pub(super) async fn apply_durable_represented_loot_money_payout_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        notified_amount: u64,
        durable_applied_amount: u64,
        sole_looter: bool,
        apply_money: bool,
        publish: bool,
    ) -> ApplyLootMoneyResultLikeCpp {
        if self.player_guid().is_none() {
            return ApplyLootMoneyResultLikeCpp::TargetMismatch;
        }
        let Some(old_money) = self.resolved_player_money_like_cpp() else {
            return ApplyLootMoneyResultLikeCpp::TargetMismatch;
        };
        let new_money = if apply_money {
            old_money
                .checked_add(durable_applied_amount)
                .filter(|money| *money <= MAX_MONEY_AMOUNT)
                .unwrap_or(old_money)
        } else {
            old_money
        };

        if apply_money && !self.set_player_gold_like_cpp(new_money) {
            return ApplyLootMoneyResultLikeCpp::TargetMismatch;
        }
        if apply_money && durable_applied_amount != 0 {
            self.enqueue_represented_quest_objective_progress_like_cpp(
                RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                    old_money,
                    new_money,
                },
            );
        }
        if publish {
            self.send_packet(&LootMoneyNotify {
                money: notified_amount,
                money_mod: 0,
                sole_looter,
            });
        }
        if apply_money || publish {
            self.drain_represented_quest_objective_progress_with_generator_like_cpp(
                item_guid_generator,
            )
            .await;
        }

        ApplyLootMoneyResultLikeCpp::Applied
    }

    pub(super) fn represented_notify_money_removed_like_cpp(&mut self, owner_guid: ObjectGuid) {
        if let Some(player_guid) = self.player_guid() {
            let _ = self.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid);
        }
        let Some(loot) = self.loot_table.get(&owner_guid) else {
            return;
        };

        let packet = CoinRemoved {
            loot_obj: loot.loot_guid,
        };
        let bytes = packet.to_bytes();
        let players_looting = loot.players_looting.clone();
        let current_player = self.player_guid();
        let current_map = self.player_map_id_like_cpp();
        let current_instance = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let registry = self.player_registry().cloned();
        let mut stale_looters = Vec::new();

        for looter in &players_looting {
            if Some(*looter) == current_player {
                self.send_packet(&packet);
                continue;
            }

            let Some(registry) = registry.as_ref() else {
                stale_looters.push(*looter);
                continue;
            };
            let Some(registration) =
                registry.loot_delivery_recipient(*looter, current_map, current_instance)
            else {
                stale_looters.push(*looter);
                continue;
            };
            if registry
                .send_current_packet(registration, bytes.clone())
                .is_err()
            {
                stale_looters.push(*looter);
            }
        }

        if !stale_looters.is_empty()
            && let Some(loot) = self.loot_table.get_mut(&owner_guid)
        {
            loot.players_looting
                .retain(|looter| !stale_looters.contains(looter));
        }
        if !stale_looters.is_empty()
            && let Some(authority) = self.represented_owned_loot_authority_like_cpp(owner_guid)
        {
            for looter in stale_looters {
                authority.remove_viewer_like_cpp(looter);
            }
            if let Some(player_guid) = current_player {
                let _ = self.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid);
            }
        }
    }

    pub(super) async fn persist_and_consume_stored_item_money_like_cpp(
        &self,
        item_guid: ObjectGuid,
        cached_notified_amount: u64,
    ) -> Option<(Arc<AtomicBool>, Arc<AtomicBool>, u64, u64)> {
        let (worker, balance_applied, publication_applied) = self
            .spawn_stored_item_money_persistence_worker_like_cpp(
                item_guid,
                cached_notified_amount,
            )?;
        match worker.await {
            Ok(Ok((applied_delta, notified_amount))) => Some((
                balance_applied,
                publication_applied,
                applied_delta,
                notified_amount,
            )),
            Ok(Err(error)) => {
                warn!(
                    item_guid = item_guid.counter(),
                    ?error,
                    "failed to atomically persist and consume stored item loot money"
                );
                None
            }
            Err(error) => {
                warn!(
                    item_guid = item_guid.counter(),
                    ?error,
                    "stored item loot-money persistence worker terminated"
                );
                None
            }
        }
    }

    fn spawn_stored_item_money_persistence_worker_like_cpp(
        &self,
        item_guid: ObjectGuid,
        cached_notified_amount: u64,
    ) -> Option<(
        tokio::task::JoinHandle<Result<(u64, u64), LootMoneyPersistenceErrorLikeCpp>>,
        Arc<AtomicBool>,
        Arc<AtomicBool>,
    )> {
        let Some(player_guid) = self.player_guid() else {
            return None;
        };
        let test_result = self.loot_money_persistence_test_result_for_worker_like_cpp();
        let persistence_port = if test_result.is_some() {
            None
        } else {
            Some(self.stored_item_money_persistence_port_like_cpp()?)
        };
        let test_current_money = self.resolved_player_money_like_cpp()?;
        let balance_applied = Arc::new(AtomicBool::new(false));
        let publication_applied = Arc::new(AtomicBool::new(false));
        let mut item_persistence_guard = self.begin_durable_item_loot_persistence_like_cpp();
        let money_persistence_tracker = self.durable_loot_money_persistence_tracker_like_cpp();
        let mut money_persistence_guard = money_persistence_tracker.begin_like_cpp().ok()?;
        let command_tx = self.session_command_tx();
        let worker_balance_applied = Arc::clone(&balance_applied);
        let worker_publication_applied = Arc::clone(&publication_applied);
        let worker = tokio::spawn(async move {
            let _money_mutation_lock = money_persistence_tracker
                .lock_money_mutation_like_cpp()
                .await;
            let (before, after, applied_delta, notified_amount) = if let Some(success) = test_result
            {
                tokio::task::yield_now().await;
                if !success {
                    return Err(LootMoneyPersistenceErrorLikeCpp::MissingCharacterDatabase);
                }
                let (after, applied_delta) =
                    loot_money_durable_outcome_like_cpp(test_current_money, cached_notified_amount);
                (
                    test_current_money,
                    after,
                    applied_delta,
                    cached_notified_amount,
                )
            } else {
                let persistence_port = persistence_port
                    .expect("production stored-money worker has a persistence port");
                let request = StoredItemMoneyPersistenceRequestLikeCpp {
                    player_guid: player_guid.counter() as u64,
                    item_guid: item_guid.counter() as u64,
                    cached_notified_amount,
                    max_money: MAX_MONEY_AMOUNT,
                };
                let outcome = match persistence_port
                    .attempt_stored_item_money_like_cpp(request)
                    .await
                {
                    StoredItemMoneyPersistenceAttemptLikeCpp::Applied(outcome) => outcome,
                    StoredItemMoneyPersistenceAttemptLikeCpp::DefinitelyRolledBack {
                        kind,
                        reason,
                        ..
                    } => {
                        return Err(match kind {
                            StoredItemMoneyRollbackKindLikeCpp::MissingPlayer => {
                                LootMoneyPersistenceErrorLikeCpp::MissingPlayer
                            }
                            StoredItemMoneyRollbackKindLikeCpp::SourceAlreadyConsumed
                            | StoredItemMoneyRollbackKindLikeCpp::Database => {
                                LootMoneyPersistenceErrorLikeCpp::Persistence(reason)
                            }
                        });
                    }
                    StoredItemMoneyPersistenceAttemptLikeCpp::CommitOutcomeUnknown {
                        reason,
                        outcome,
                    } => match persistence_port
                        .reconcile_stored_item_money_like_cpp(request, outcome)
                        .await
                    {
                        StoredItemMoneyReconciliationLikeCpp::Committed => outcome,
                        StoredItemMoneyReconciliationLikeCpp::RolledBack => {
                            return Err(LootMoneyPersistenceErrorLikeCpp::Persistence(
                                "stored Item money COMMIT was reconciled as rolled back".to_owned(),
                            ));
                        }
                        StoredItemMoneyReconciliationLikeCpp::Indeterminate { .. } => {
                            money_persistence_guard.mark_indeterminate_like_cpp();
                            queue_stored_item_money_indeterminate_kick_like_cpp(&command_tx);
                            return Err(
                                LootMoneyPersistenceErrorLikeCpp::CommitOutcomeUnknownPersistence(
                                    reason,
                                ),
                            );
                        }
                    },
                };
                (
                    outcome.before,
                    outcome.after,
                    outcome.applied_delta,
                    outcome.notified_amount,
                )
            };

            money_persistence_guard.commit_like_cpp(
                crate::loot_persistence::DurableLootMoneyCompletionLikeCpp {
                    durable_money_before: before,
                    durable_money_after: after,
                    durable_applied_amount: applied_delta,
                    applied: Arc::clone(&worker_balance_applied),
                },
            );
            item_persistence_guard.mark_committed_like_cpp(DurableItemLootCompletionLikeCpp {
                owner_guid: item_guid,
                loot_list_id: 0,
                player_guid,
                item_owner_auto_release: false,
                durable_item_money_applied_amount: Some(applied_delta),
                durable_item_money_notified_amount: Some(notified_amount),
                durable_item_money_balance_applied: Some(Arc::clone(&worker_balance_applied)),
                item_fanout: None,
                runtime_inventory_applied: Arc::clone(&worker_publication_applied),
            });
            Ok((applied_delta, notified_amount))
        });
        Some((worker, balance_applied, publication_applied))
    }
}
