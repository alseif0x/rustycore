// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Battle-pet purchase operations, part 1 of 2 (#1363 slice 7a; moved from
//! the World `battle_pet_purchase/ops_1.rs`). Every function keeps its
//! original body over the purchase host.

use super::*;

/// Issue #161 live purchase: revalidates the offer admission under the
/// exclusive money guard, commits charge + durable command, applies the
/// pet once through the #160 account owner, publishes, records delivery,
/// then completes in
/// the C++ `Trainer::TeachSpell` battle-pet order (money update,
/// `SMSG_BATTLE_PET_UPDATES`, dependent `SMSG_LEARNED_SPELLS`) with the
/// trainer visual kits suppressed (`Trainer.cpp:108,121-125`).
pub async fn execute_battle_pet_trainer_purchase_with_generator_like_cpp<
    H: BattlePetPurchaseHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    battle_pet_selection_store: &wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp,
    money_persistence: ExclusivePlayerMoneyPersistenceLikeCpp,
    trainer_guid: ObjectGuid,
    trainer_id: u32,
    offer: PreparedBattlePetTrainerOfferLikeCpp,
) -> BattlePetPurchaseExecutionLikeCpp {
    let Some((owner, lease_id)) = host.battle_pet_account_owner_lease_like_cpp() else {
        return BattlePetPurchaseExecutionLikeCpp::Unavailable(
            BattlePetPurchaseAdmissionFailureLikeCpp::NoJournalAuthority,
        );
    };
    let Some(store) = host.battle_pet_purchase_store_like_cpp() else {
        return BattlePetPurchaseExecutionLikeCpp::Unavailable(
            BattlePetPurchaseAdmissionFailureLikeCpp::StoreUnavailable,
        );
    };
    let Some(player_guid) = host.player_guid() else {
        return BattlePetPurchaseExecutionLikeCpp::Unavailable(
            BattlePetPurchaseAdmissionFailureLikeCpp::StoreUnavailable,
        );
    };
    let Some(species_entry) = host.battle_pet_species_entry_like_cpp(offer.species_id) else {
        return BattlePetPurchaseExecutionLikeCpp::Unavailable(
            BattlePetPurchaseAdmissionFailureLikeCpp::SelectionUnavailable,
        );
    };

    // C++ checks the per-species account cap before charging and stays
    // silent (`Trainer.cpp:102-106`). The #160 owner rechecks it inside
    // its own transaction; this admission check only avoids creating a
    // command that would have to be compensated.
    if owner.has_max_pet_count_like_cpp(offer.species_id, Some(player_guid)) {
        return BattlePetPurchaseExecutionLikeCpp::Unavailable(
            BattlePetPurchaseAdmissionFailureLikeCpp::Capacity,
        );
    }
    if !host.battle_pet_try_acquire_journal_lease_like_cpp().await {
        return BattlePetPurchaseExecutionLikeCpp::Unavailable(
            BattlePetPurchaseAdmissionFailureLikeCpp::JournalLocked,
        );
    }
    let Some(selection) = host
        .hub_ref()
        .battle_pet_trainer_selection_like_cpp(battle_pet_selection_store, &species_entry)
    else {
        return BattlePetPurchaseExecutionLikeCpp::Unavailable(
            BattlePetPurchaseAdmissionFailureLikeCpp::SelectionUnavailable,
        );
    };

    let Some(old_money) = host.resolved_player_money_like_cpp() else {
        return BattlePetPurchaseExecutionLikeCpp::Unavailable(
            BattlePetPurchaseAdmissionFailureLikeCpp::StoreUnavailable,
        );
    };
    let price = u64::from(offer.effective_price);
    if old_money < price {
        // C++ `FailReason::NotEnoughMoney` (`Trainer.cpp:113-117`).
        host.core().send_packet_realm(&TrainerBuyFailed {
            trainer_guid,
            spell_id: offer.source_spell_id as i32,
            reason: 1,
        });
        return BattlePetPurchaseExecutionLikeCpp::InsufficientMoney;
    }
    let new_money = old_money - price;

    // T1: the guarded money deduction and the pending command commit in
    // one Character DB transaction; the request key is also the #160
    // Login DB receipt identity.
    let mut request_key = [0_u8; 16];
    rand::thread_rng().fill_bytes(&mut request_key);
    // The receipt/owner authority is the Battle.net account (#160), not
    // the game account; persist that identity for recovery binding.
    let command = BattlePetPurchaseCommandLikeCpp {
        request_key,
        character_guid: player_guid.counter() as u64,
        account_id: host.core().battlenet_account_id(),
        trainer_id,
        spell_id: offer.source_spell_id,
        species: selection.species,
        breed: selection.breed,
        quality: selection.quality,
        display_id: selection.display_id,
        level: selection.level,
        price: offer.effective_price,
        money_before: old_money,
        money_after: new_money,
        status: BattlePetPurchaseStatusLikeCpp::PendingApplication,
        published: false,
        failure_reason: None,
    };
    let money_tracker = Arc::clone(
        host.lifecycle()
            .durable_loot_money_persistence_tracker_like_cpp(),
    );
    let charge = retry_battle_pet_purchase_step_like_cpp(
        || {
            store.charge_and_insert_command(
                command.clone(),
                Box::new(
                    PlayerMoneyCommitCancellationFenceLikeCpp::new_disarmed_like_cpp(Arc::clone(
                        &money_tracker,
                    )),
                ),
            )
        },
        store_error_is_retryable_like_cpp,
    )
    .await;
    match charge {
        Ok(BattlePetPurchaseChargeOutcomeLikeCpp::Charged) => {}
        Ok(BattlePetPurchaseChargeOutcomeLikeCpp::RolledBack) => {
            return BattlePetPurchaseExecutionLikeCpp::ChargeDeclined;
        }
        Err(BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(error)) => {
            warn!(
                account = host.core().account_id,
                error,
                "Battle-pet purchase charge COMMIT outcome is unknown; quarantined the session"
            );
            {
                let (s, mut h) = host.split_lifecycle_mut();
                s.quarantine_player_money_persistence_like_cpp(
                    &mut h,
                    "battle-pet purchase charge COMMIT outcome is unknown; relog required",
                )
            };
            return BattlePetPurchaseExecutionLikeCpp::ChargeIndeterminate;
        }
        Err(error) => {
            warn!(
                account = host.core().account_id,
                ?error,
                "Battle-pet purchase charge did not commit"
            );
            return BattlePetPurchaseExecutionLikeCpp::ChargeDeclined;
        }
    }

    // Publish the durable charge under the guard (C++ `ModifyMoney`),
    // then release it before draining criteria, matching the #159 order.
    if !host.stage_player_money_change_like_cpp(old_money, new_money) {
        host.core_mut()
            .kick("canonical Player money owner became unavailable after battle-pet charge COMMIT");
        return BattlePetPurchaseExecutionLikeCpp::ChargeIndeterminate;
    }
    if old_money != new_money {
        // The client sees the charge before the pet, and this packet goes
        // out *after* the Character transaction commits -- the charge is
        // awaited and matched before reaching here. Leaving it out made
        // moving it across the commit -- or after the pet packets --
        // invisible, which is the ordering the crash window is defined by.
        // Recorded only when it was actually enqueued: the bridge returns
        // early with no player GUID or snapshot and sends nothing.
        if host.send_player_values_update_from_entity_bridge(&[], &[], &[], &[], Some(new_money)) {
            record_battle_pet_purchase_publication_trace_like_cpp(
                "battle_pet_trainer_purchase.money",
            );
        }
    }
    drop(money_persistence);
    host.drain_represented_quest_objective_progress_with_generator_like_cpp(item_guid_generator)
        .await;

    // T2: apply once through the #160 owner; its Login DB transaction
    // revalidates fence, lease and per-species capacity and writes pet +
    // receipt together, so a replay returns the original pet.
    let request = BattlePetAddRequestLikeCpp {
        request_key: BattlePetAddRequestKeyLikeCpp::from_bytes(request_key),
        species: selection.species,
        display_id: selection.display_id,
        breed: selection.breed,
        quality: selection.quality,
        level: selection.level,
        owner_guid: Some(player_guid),
    };
    let applied = retry_battle_pet_purchase_step_like_cpp(
        || owner.try_add_pet_like_cpp(lease_id, request.clone()),
        battle_pet_add_failure_is_retryable_like_cpp,
    )
    .await;
    match applied {
        Ok(outcome) => {
            let pet = match outcome {
                BattlePetAddOutcomeLikeCpp::Added(pet) => pet,
                BattlePetAddOutcomeLikeCpp::Replayed(pet) => pet,
            };
            let pet_guid = pet.guid;
            // The durable pet exists before any success packet. Emit first
            // and record delivery second so a crash can cause an idempotent
            // recovery re-send but can never lose the only notification.
            // Exactly-once transport is impossible without a client ACK;
            // pet creation, charge and compensation remain exactly-once.
            let enqueued =
                publish_battle_pet_trainer_purchase_like_cpp(host, pet, offer.source_spell_id);
            let published = if enqueued {
                record_battle_pet_purchase_publication_like_cpp(host, &store, request_key).await
            } else {
                false
            };
            if !complete_battle_pet_purchase_like_cpp(host, &store, request_key).await {
                // The pet is durable; recovery completes the command and
                // emits only if the claim never committed.
                return BattlePetPurchaseExecutionLikeCpp::RetryableDeferred;
            }
            BattlePetPurchaseExecutionLikeCpp::Purchased {
                pet_guid,
                published,
            }
        }
        Err(error) if battle_pet_add_failure_is_terminal_like_cpp(&error) => {
            compensate_battle_pet_purchase_like_cpp(
                host,
                item_guid_generator,
                &owner,
                lease_id,
                player_guid,
                &store,
                &command,
                BattlePetPurchaseRefundPublicationLikeCpp::ValuesUpdatePacket,
            )
            .await
        }
        Err(error) => {
            warn!(
                account = host.core().account_id,
                ?error,
                "Battle-pet purchase apply deferred after bounded retries"
            );
            BattlePetPurchaseExecutionLikeCpp::RetryableDeferred
        }
    }
}
/// Login recovery: resume every unconverged durable command of this
/// character, oldest first, bounded by
/// `BATTLE_PET_PURCHASE_RECOVERY_BATCH_LIMIT_LIKE_CPP`. Runs inline
/// during the login burst (no background task) and is cancellation-safe:
/// every step is either a committed transition or leaves a resumable
/// durable state for the next login.
pub async fn recover_battle_pet_trainer_purchases_with_generator_like_cpp<
    H: BattlePetPurchaseHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
) -> Option<BattlePetPurchaseRecoveryLikeCpp> {
    let (owner, lease_id) = host.battle_pet_account_owner_lease_like_cpp()?;
    let store = host.battle_pet_purchase_store_like_cpp()?;
    let player_guid = host.player_guid()?;
    let commands = match store
        .load_pending_commands(
            player_guid.counter() as u64,
            BATTLE_PET_PURCHASE_RECOVERY_BATCH_LIMIT_LIKE_CPP,
        )
        .await
    {
        Ok(commands) => commands,
        Err(error) => {
            warn!(
                account = host.core().account_id,
                ?error,
                "Battle-pet purchase recovery scan failed; will retry on next login"
            );
            return None;
        }
    };
    if commands.is_empty() {
        return Some(BattlePetPurchaseRecoveryLikeCpp {
            applied: 0,
            compensated: 0,
            deferred: 0,
            terminal_failures: 0,
        });
    }

    let mut summary = BattlePetPurchaseRecoveryLikeCpp {
        applied: 0,
        compensated: 0,
        deferred: 0,
        terminal_failures: 0,
    };
    let full_batch = commands.len() == BATTLE_PET_PURCHASE_RECOVERY_BATCH_LIMIT_LIKE_CPP as usize;
    for command in commands {
        // A character transferred to another Battle.net account mid-
        // purchase must never apply or publish into the new account:
        // the pet/receipt authority is the original account, so the
        // command is compensated (money travels with the character) or
        // its marker is closed without emitting.
        let account_mismatch = command.account_id != host.core().battlenet_account_id();
        if account_mismatch && command.status == BattlePetPurchaseStatusLikeCpp::Completed {
            if !command.published {
                warn!(
                    account = host.core().account_id,
                    command_account = command.account_id,
                    "Closing the publication marker of a battle-pet purchase owned by another account"
                );
                record_battle_pet_purchase_publication_like_cpp(host, &store, command.request_key)
                    .await;
                summary.applied += 1;
            }
            continue;
        }
        if account_mismatch && command.status == BattlePetPurchaseStatusLikeCpp::PendingApplication
        {
            let decision = retry_battle_pet_purchase_step_like_cpp(
                || {
                    store.mark_compensation_pending(
                        command.request_key,
                        "battle-pet purchase account changed before application",
                    )
                },
                store_error_is_retryable_like_cpp,
            )
            .await;
            if matches!(
                decision,
                Ok(BattlePetPurchaseMarkOutcomeLikeCpp::Applied
                    | BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied)
            ) {
                match compensate_battle_pet_purchase_like_cpp(
                    host,
                    item_guid_generator,
                    &owner,
                    lease_id,
                    player_guid,
                    &store,
                    &command,
                    BattlePetPurchaseRefundPublicationLikeCpp::RuntimeOnly,
                )
                .await
                {
                    BattlePetPurchaseExecutionLikeCpp::Compensated => {
                        summary.compensated += 1;
                    }
                    BattlePetPurchaseExecutionLikeCpp::TerminalFailure => {
                        summary.terminal_failures += 1;
                    }
                    BattlePetPurchaseExecutionLikeCpp::CompletedElsewhere => {
                        // The original account already owns the durable
                        // pet: completed, no refund.
                        summary.applied += 1;
                    }
                    _ => {
                        summary.deferred += 1;
                        break;
                    }
                }
            } else {
                summary.deferred += 1;
                break;
            }
            continue;
        }
        match command.status {
            BattlePetPurchaseStatusLikeCpp::CompensationPending => {
                match compensate_battle_pet_purchase_like_cpp(
                    host,
                    item_guid_generator,
                    &owner,
                    lease_id,
                    player_guid,
                    &store,
                    &command,
                    BattlePetPurchaseRefundPublicationLikeCpp::RuntimeOnly,
                )
                .await
                {
                    BattlePetPurchaseExecutionLikeCpp::Compensated => {
                        summary.compensated += 1;
                    }
                    BattlePetPurchaseExecutionLikeCpp::TerminalFailure => {
                        summary.terminal_failures += 1;
                    }
                    BattlePetPurchaseExecutionLikeCpp::CompletedElsewhere => {
                        // The receipt re-check proved the pet durable:
                        // the command completed instead of refunding.
                        summary.applied += 1;
                    }
                    _ => {
                        summary.deferred += 1;
                        break;
                    }
                }
            }
            BattlePetPurchaseStatusLikeCpp::PendingApplication => {
                if !host.battle_pet_try_acquire_journal_lease_like_cpp().await {
                    summary.deferred += 1;
                    break;
                }
                let request = BattlePetAddRequestLikeCpp {
                    request_key: BattlePetAddRequestKeyLikeCpp::from_bytes(command.request_key),
                    species: command.species,
                    display_id: command.display_id,
                    breed: command.breed,
                    quality: command.quality,
                    level: command.level,
                    owner_guid: Some(player_guid),
                };
                // One attempt per login per command: a retryable failure
                // stops the batch and resumes on the next login, keeping
                // recovery bounded.
                match owner.try_add_pet_like_cpp(lease_id, request).await {
                    Ok(outcome) => {
                        let pet = match outcome {
                            BattlePetAddOutcomeLikeCpp::Added(pet) => pet,
                            BattlePetAddOutcomeLikeCpp::Replayed(pet) => pet,
                        };
                        if !command.published {
                            let enqueued = publish_battle_pet_trainer_purchase_like_cpp(
                                host,
                                pet,
                                command.spell_id,
                            );
                            if enqueued {
                                record_battle_pet_purchase_publication_like_cpp(
                                    host,
                                    &store,
                                    command.request_key,
                                )
                                .await;
                            }
                        }
                        if !complete_battle_pet_purchase_like_cpp(host, &store, command.request_key)
                            .await
                        {
                            summary.deferred += 1;
                            continue;
                        }
                        summary.applied += 1;
                    }
                    Err(error) if battle_pet_add_failure_is_terminal_like_cpp(&error) => {
                        match compensate_battle_pet_purchase_like_cpp(
                            host,
                            item_guid_generator,
                            &owner,
                            lease_id,
                            player_guid,
                            &store,
                            &command,
                            BattlePetPurchaseRefundPublicationLikeCpp::RuntimeOnly,
                        )
                        .await
                        {
                            BattlePetPurchaseExecutionLikeCpp::Compensated => {
                                summary.compensated += 1;
                            }
                            BattlePetPurchaseExecutionLikeCpp::TerminalFailure => {
                                summary.terminal_failures += 1;
                            }
                            BattlePetPurchaseExecutionLikeCpp::CompletedElsewhere => {
                                // The receipt re-check proved the pet
                                // durable: completed instead of refunding.
                                summary.applied += 1;
                            }
                            _ => {
                                summary.deferred += 1;
                                break;
                            }
                        }
                    }
                    Err(error) => {
                        warn!(
                            account = host.core().account_id,
                            ?error,
                            "Battle-pet purchase recovery deferred a command"
                        );
                        summary.deferred += 1;
                        break;
                    }
                }
            }
            BattlePetPurchaseStatusLikeCpp::Completed => {
                // Completed but never recorded as published: replay the
                // receipt and enqueue its packets before recording the
                // marker. The recovery scan selects these rows deliberately;
                // a prior enqueue followed by a crash may be re-sent.
                if command.published {
                    continue;
                }
                if !host.battle_pet_try_acquire_journal_lease_like_cpp().await {
                    summary.deferred += 1;
                    break;
                }
                let request = BattlePetAddRequestLikeCpp {
                    request_key: BattlePetAddRequestKeyLikeCpp::from_bytes(command.request_key),
                    species: command.species,
                    display_id: command.display_id,
                    breed: command.breed,
                    quality: command.quality,
                    level: command.level,
                    owner_guid: Some(player_guid),
                };
                match owner.try_add_pet_like_cpp(lease_id, request).await {
                    Ok(outcome) => {
                        let pet = match outcome {
                            BattlePetAddOutcomeLikeCpp::Added(pet) => pet,
                            BattlePetAddOutcomeLikeCpp::Replayed(pet) => pet,
                        };
                        let enqueued = publish_battle_pet_trainer_purchase_like_cpp(
                            host,
                            pet,
                            command.spell_id,
                        );
                        if enqueued {
                            record_battle_pet_purchase_publication_like_cpp(
                                host,
                                &store,
                                command.request_key,
                            )
                            .await;
                        }
                        summary.applied += 1;
                    }
                    Err(BattlePetAddFailureLikeCpp::DuplicateRequest) => {
                        // The receipt exists but its pet is gone (deleted
                        // by another session before recovery): the charge
                        // stands and nothing can be published, so close
                        // the marker instead of blocking the batch behind
                        // an unresolvable row on every login.
                        record_battle_pet_purchase_publication_like_cpp(
                            host,
                            &store,
                            command.request_key,
                        )
                        .await;
                        summary.applied += 1;
                    }
                    Err(error) => {
                        warn!(
                            account = host.core().account_id,
                            ?error,
                            "Battle-pet purchase recovery could not resolve the publication packet"
                        );
                        summary.deferred += 1;
                        break;
                    }
                }
            }
            status => {
                warn!(
                    account = host.core().account_id,
                    ?status,
                    "Battle-pet purchase recovery scanned an unexpected terminal command"
                );
            }
        }
    }
    if full_batch {
        warn!(
            account = host.core().account_id,
            limit = BATTLE_PET_PURCHASE_RECOVERY_BATCH_LIMIT_LIKE_CPP,
            "Battle-pet purchase recovery batch is full; remaining commands resume on later logins"
        );
    }
    Some(summary)
}
/// Record a success publication after the packets were queued. A failed
/// marker leaves the command selected for a recovery re-send. Enqueue
/// attempts may therefore repeat and avoid consuming the only recovery
/// signal before emission; actual delivery remains best-effort without a
/// client acknowledgement.
pub(super) async fn record_battle_pet_purchase_publication_like_cpp<
    H: BattlePetPurchaseHostLikeCpp,
>(
    host: &mut H,
    store: &Arc<dyn BattlePetPurchaseStoreLikeCpp>,
    request_key: [u8; 16],
) -> bool {
    match retry_battle_pet_purchase_step_like_cpp(
        || store.mark_published(request_key),
        store_error_is_retryable_like_cpp,
    )
    .await
    {
        Ok(
            BattlePetPurchaseMarkOutcomeLikeCpp::Applied
            | BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied,
        ) => true,
        Ok(conflict) => {
            warn!(
                account = host.core().account_id,
                ?conflict,
                "Battle-pet purchase publication marker observed a conflicting terminal state"
            );
            false
        }
        Err(error) => {
            warn!(
                account = host.core().account_id,
                ?error,
                "Battle-pet purchase publication marker did not commit"
            );
            false
        }
    }
}
/// T3 with bounded retries. False means the pet is durable but the
/// completion did not converge; recovery finishes it silently.
pub(super) async fn complete_battle_pet_purchase_like_cpp<H: BattlePetPurchaseHostLikeCpp>(
    host: &mut H,
    store: &Arc<dyn BattlePetPurchaseStoreLikeCpp>,
    request_key: [u8; 16],
) -> bool {
    match retry_battle_pet_purchase_step_like_cpp(
        || store.mark_completed(request_key),
        store_error_is_retryable_like_cpp,
    )
    .await
    {
        Ok(
            BattlePetPurchaseMarkOutcomeLikeCpp::Applied
            | BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied,
        ) => true,
        Ok(conflict) => {
            warn!(
                account = host.core().account_id,
                ?conflict,
                "Battle-pet purchase completion observed a conflicting terminal state"
            );
            false
        }
        Err(error) => {
            warn!(
                account = host.core().account_id,
                ?error,
                "Battle-pet purchase completion deferred"
            );
            false
        }
    }
}
