//! Battle-pet purchase operations, part 2 of 2.
//!
//! The inherent `WorldSession` impl is divided by responsibility under
//! #656; every method keeps its original body.

use super::*;

impl WorldSession {
    /// T4+T5: record the terminal-failure decision and refund exactly once.
    /// The receipt re-check before refunding closes the residual race where
    /// a concurrent driver made the pet durable after all: a durable receipt
    /// forbids the refund and completes the command instead.
    pub(super) async fn compensate_battle_pet_purchase_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        owner: &Arc<BattlePetAccountOwnerLikeCpp>,
        lease_id: BattlePetLeaseIdLikeCpp,
        player_guid: ObjectGuid,
        store: &Arc<dyn BattlePetPurchaseStoreLikeCpp>,
        command: &BattlePetPurchaseCommandLikeCpp,
        refund_publication: BattlePetPurchaseRefundPublicationLikeCpp,
    ) -> BattlePetPurchaseExecutionLikeCpp {
        // T4: persist the decision before touching money so recovery never
        // re-applies a command whose compensation was already decided.
        let decision = retry_battle_pet_purchase_step_like_cpp(
            || {
                store.mark_compensation_pending(
                    command.request_key,
                    "battle-pet purchase apply failed terminally",
                )
            },
            store_error_is_retryable_like_cpp,
        )
        .await;
        match decision {
            Ok(
                BattlePetPurchaseMarkOutcomeLikeCpp::Applied
                | BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied,
            ) => {}
            Ok(BattlePetPurchaseMarkOutcomeLikeCpp::ConflictedCompleted) => {
                return BattlePetPurchaseExecutionLikeCpp::CompletedElsewhere;
            }
            Ok(BattlePetPurchaseMarkOutcomeLikeCpp::ConflictedCompensated) => {
                return BattlePetPurchaseExecutionLikeCpp::Compensated;
            }
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "Battle-pet purchase compensation decision could not be recorded; recovery re-derives it"
                );
                return BattlePetPurchaseExecutionLikeCpp::CompensationDeferred;
            }
        }

        // Receipt re-check: a durable pet forbids the refund; the command
        // completes instead. An account-mismatched command is probed under
        // its ORIGINAL account, the only receipt authority: a durable pet
        // there still forbids the refund (the pet travels nowhere, the
        // charge stays), but nothing is ever published into the new
        // account. When this account owns the receipt and the pet resolves
        // in-process, publish first if no publication was ever recorded;
        // when the owner cannot resolve the packet (its in-memory journal
        // lost the pet with a failed insert reply), completion proceeds
        // without the marker and login recovery finishes the publication.
        let account_mismatch = command.account_id != self.battlenet_account_id();
        let receipt_probe = if account_mismatch {
            // The unfenced snapshot could be falsified by a still-flying
            // detached insert from the original account; probe through the
            // original account's process fence instead and defer while its
            // authority is held elsewhere.
            match owner
                .receipt_probe_for_account_fenced_like_cpp(
                    command.account_id,
                    BattlePetAddRequestKeyLikeCpp::from_bytes(command.request_key),
                )
                .await
            {
                Ok(crate::battle_pet_account::BattlePetFencedReceiptProbeLikeCpp::Committed) => {
                    Ok(true)
                }
                Ok(crate::battle_pet_account::BattlePetFencedReceiptProbeLikeCpp::Absent) => {
                    Ok(false)
                }
                Ok(
                    crate::battle_pet_account::BattlePetFencedReceiptProbeLikeCpp::AuthorityUnavailable,
                ) => Err(BattlePetAddFailureLikeCpp::MissingAuthority),
                Err(error) => Err(error),
            }
        } else {
            owner
                .add_request_committed_like_cpp(BattlePetAddRequestKeyLikeCpp::from_bytes(
                    command.request_key,
                ))
                .await
        };
        match receipt_probe {
            Ok(true) => {
                if !command.published && account_mismatch {
                    // The pet is durable in the original account's journal;
                    // just close the marker so recovery stops selecting the
                    // row, without emitting anything here.
                    self.record_battle_pet_purchase_publication_like_cpp(
                        store,
                        command.request_key,
                    )
                    .await;
                }
                if !command.published
                    && !account_mismatch
                    && self.battle_pet_try_acquire_journal_lease_like_cpp().await
                {
                    let replay = owner
                        .try_add_pet_like_cpp(
                            lease_id,
                            BattlePetAddRequestLikeCpp {
                                request_key: BattlePetAddRequestKeyLikeCpp::from_bytes(
                                    command.request_key,
                                ),
                                species: command.species,
                                display_id: command.display_id,
                                breed: command.breed,
                                quality: command.quality,
                                level: command.level,
                                owner_guid: Some(player_guid),
                            },
                        )
                        .await;
                    if let Ok(outcome) = replay {
                        let pet = match outcome {
                            BattlePetAddOutcomeLikeCpp::Added(pet) => pet,
                            BattlePetAddOutcomeLikeCpp::Replayed(pet) => pet,
                        };
                        let enqueued = self
                            .publish_battle_pet_trainer_purchase_like_cpp(pet, command.spell_id);
                        if enqueued {
                            self.record_battle_pet_purchase_publication_like_cpp(
                                store,
                                command.request_key,
                            )
                            .await;
                        }
                    }
                }
                let _ = self
                    .complete_battle_pet_purchase_like_cpp(store, command.request_key)
                    .await;
                return BattlePetPurchaseExecutionLikeCpp::CompletedElsewhere;
            }
            Ok(false) => {}
            Err(error) => {
                // Absence cannot be proven; refunding blind is forbidden.
                warn!(
                    account = self.account_id,
                    ?error,
                    "Battle-pet purchase compensation cannot prove the receipt absent; refund deferred"
                );
                return BattlePetPurchaseExecutionLikeCpp::CompensationDeferred;
            }
        }

        // T5: refund and flip in one Character DB transaction, under the
        // same per-character money exclusion as the charge.
        let Some(refund_guard) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return BattlePetPurchaseExecutionLikeCpp::CompensationDeferred;
        };
        let money_tracker = self.durable_loot_money_persistence_tracker_like_cpp();
        let compensated = retry_battle_pet_purchase_step_like_cpp(
            || {
                store.compensate(
                    command.request_key,
                    wow_entities::MAX_MONEY_AMOUNT,
                    Box::new(
                        PlayerMoneyCommitCancellationFenceLikeCpp::new_disarmed_like_cpp(
                            Arc::clone(&money_tracker),
                        ),
                    ),
                )
            },
            store_error_is_retryable_like_cpp,
        )
        .await;
        match compensated {
            Ok(
                BattlePetPurchaseCompensationOutcomeLikeCpp::Compensated { durable_money }
                | BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated { durable_money },
            ) => {
                // Reconcile to the absolute durable value. Attribution based
                // on a lost COMMIT reply is racy when another driver can finish
                // compensation before the status re-read.
                let Some(current) = self.resolved_player_money_like_cpp() else {
                    drop(refund_guard);
                    return BattlePetPurchaseExecutionLikeCpp::Compensated;
                };
                let restored = durable_money.min(wow_entities::MAX_MONEY_AMOUNT);
                if !self.stage_player_money_change_like_cpp(current, restored) {
                    drop(refund_guard);
                    return BattlePetPurchaseExecutionLikeCpp::Compensated;
                }
                if refund_publication
                    == BattlePetPurchaseRefundPublicationLikeCpp::ValuesUpdatePacket
                    && current != restored
                {
                    self.send_player_values_update_from_entity_bridge(
                        &[],
                        &[],
                        &[],
                        &[],
                        Some(restored),
                    );
                }
                drop(refund_guard);
                self.drain_represented_quest_objective_progress_with_generator_like_cpp(
                    item_guid_generator,
                )
                .await;
                BattlePetPurchaseExecutionLikeCpp::Compensated
            }
            Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::ConflictedCompleted) => {
                drop(refund_guard);
                BattlePetPurchaseExecutionLikeCpp::CompletedElsewhere
            }
            Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::CharacterMissing) => {
                drop(refund_guard);
                let marked = retry_battle_pet_purchase_step_like_cpp(
                    || {
                        store.mark_terminal_failure(
                            command.request_key,
                            "battle-pet purchase refund impossible: character row missing",
                        )
                    },
                    store_error_is_retryable_like_cpp,
                )
                .await;
                if let Err(error) = marked {
                    warn!(
                        account = self.account_id,
                        ?error,
                        "Battle-pet purchase terminal-failure mark did not commit"
                    );
                    return BattlePetPurchaseExecutionLikeCpp::CompensationDeferred;
                }
                BattlePetPurchaseExecutionLikeCpp::TerminalFailure
            }
            Err(BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(error)) => {
                warn!(
                    account = self.account_id,
                    error,
                    "Battle-pet purchase refund COMMIT outcome is unknown; quarantined the session"
                );
                drop(refund_guard);
                self.quarantine_player_money_persistence_like_cpp(
                    "battle-pet purchase refund COMMIT outcome is unknown; relog required",
                );
                BattlePetPurchaseExecutionLikeCpp::CompensationDeferred
            }
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "Battle-pet purchase compensation did not converge"
                );
                drop(refund_guard);
                BattlePetPurchaseExecutionLikeCpp::CompensationDeferred
            }
        }
    }
    /// The one allowed success publication, in the C++ battle-pet
    /// `TeachSpell` order with trainer visuals suppressed: the petAdded
    /// journal update with its criteria hooks, then the dependent runtime
    /// spell learn (`Player::LearnSpell(dependent=true)` — runtime-only,
    /// never a `character_spell` row) and its `SMSG_LEARNED_SPELLS`.
    pub(super) fn publish_battle_pet_trainer_purchase_like_cpp(
        &mut self,
        pet: BattlePetJournalPet,
        spell_id: u32,
    ) -> bool {
        let species = pet.species;
        let journal_enqueued = self.publish_battle_pet_trainer_purchase_add_like_cpp(pet.clone());
        // C++ `BattlePetMgr::AddPet`: SendUpdates first, then the two set-like
        // criteria hooks. Their represented bridge derives current durable
        // state, making receipt and packet-publication recovery idempotent.
        self.record_battle_pet_trainer_purchase_criteria_like_cpp(species);
        self.learn_dependent_known_spell_like_cpp(spell_id as i32);
        let learned_enqueued = self
            .send_tx()
            .send(wow_packet::ServerPacket::to_bytes(&LearnedSpells::single(
                spell_id as i32,
            )))
            .is_ok();
        if !learned_enqueued {
            warn!("Send channel closed for account {}", self.account_id);
        }
        // Where this lands relative to the commit is the crash window, so the
        // trace has to see it -- and each packet separately, because the client
        // observes them separately. Recording one event for the pair made a
        // partial delivery look like a closed channel when it was gated on
        // both, and like a complete delivery when it was not. Recovery has to
        // know which packets the client actually saw.
        if journal_enqueued {
            record_battle_pet_purchase_publication_trace_like_cpp(
                "battle_pet_trainer_purchase.journal",
            );
        }
        if learned_enqueued {
            record_battle_pet_purchase_publication_trace_like_cpp(
                "battle_pet_trainer_purchase.learned_spell",
            );
        }
        journal_enqueued && learned_enqueued
    }
}
