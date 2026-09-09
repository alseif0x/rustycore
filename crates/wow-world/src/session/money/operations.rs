//! Represented money and currency operations.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Set the currency types store for this session.
    pub fn set_currency_types_store(&mut self, store: Arc<CurrencyTypesStore>) {
        self.currency_types_store = Some(store);
    }
    /// Get the currency types store reference.
    pub fn currency_types_store(&self) -> Option<&Arc<CurrencyTypesStore>> {
        self.currency_types_store.as_ref()
    }
    /// Set the item currency cost store for this session.
    #[cfg(test)]
    pub fn set_item_currency_cost_store(&mut self, store: Arc<ItemCurrencyCostStore>) {
        self.item_currency_cost_store = Some(store);
    }
    /// C++ `Player::GetCurrencyQuantity`.
    pub(crate) fn player_currency_quantity(&self, currency_id: u32) -> Option<u32> {
        self.player_currencies_like_cpp().map(|currencies| {
            currencies
                .get(&currency_id)
                .map(|currency| currency.quantity)
                .unwrap_or(0)
        })
    }
    /// C++ `Player::HasCurrency`.
    pub(crate) fn has_currency(&self, currency_id: u32, amount: u32) -> bool {
        self.player_currency_quantity(currency_id)
            .is_some_and(|quantity| quantity >= amount)
    }
    /// C++ `Player::SetCurrencyFlags` + `Player::SendCurrencies`.
    pub(crate) fn represented_set_currency_flags_like_cpp(
        &mut self,
        currency_id: u32,
        flags: u8,
    ) -> bool {
        let Some(store) = self.currency_types_store.as_ref() else {
            return false;
        };
        if !store.has_record(currency_id) {
            return false;
        }

        let Some(mut currencies) = self.player_currencies_like_cpp() else {
            return false;
        };
        if let Some(currency) = currencies.get_mut(&currency_id) {
            if currency.flags != flags {
                currency.flags = flags;
                if currency.state != PlayerCurrencyState::New {
                    currency.state = PlayerCurrencyState::Changed;
                }
            }
            if !self.set_player_currencies_like_cpp(currencies) {
                return false;
            }
        }

        let Some(packet) = self.setup_currencies_packet_like_cpp() else {
            return false;
        };
        self.send_packet(&packet);
        true
    }
    /// Publish the C++ vendor gain immediately for callers that do not own a
    /// wider durable transaction. Persistence-sensitive vendor handlers use
    /// [`Self::plan_add_currency_vendor_like_cpp`] and publish only after
    /// their combined item/currency transaction commits.
    pub(crate) fn add_currency_vendor(
        &mut self,
        currency_id: u32,
        amount: u32,
    ) -> Result<Option<PlayerCurrencyDelta>, ()> {
        let mut currencies = self.player_currencies_like_cpp().ok_or(())?;
        let delta = self.plan_add_currency_vendor_like_cpp(&mut currencies, currency_id, amount)?;
        if !self.set_player_currencies_like_cpp(currencies) {
            return Err(());
        }
        Ok(delta)
    }
    /// C++ `Player::AddCurrency(..., CurrencyGainSource::ItemRefund)`.
    pub(crate) fn add_currency_item_refund(
        &mut self,
        currency_id: u32,
        amount: u32,
    ) -> Result<Option<PlayerCurrencyDelta>, ()> {
        if amount == 0 {
            return Ok(None);
        }

        let Some(entry) = self
            .currency_types_store
            .as_ref()
            .and_then(|store| store.get(currency_id))
            .copied()
        else {
            return Err(());
        };

        let player_team = player_team_for_race_cpp(self.player_race_like_cpp());
        if (entry.is_alliance() && player_team != Team::Alliance)
            || (entry.is_horde() && player_team != Team::Horde)
        {
            return Ok(None);
        }

        if entry.award_condition_id != 0 {
            return Err(());
        }
        if entry.faction_id != 0 || currency_id == CurrencyTypes::Azerite as u32 {
            return Ok(None);
        }

        let mut currencies = self.player_currencies_like_cpp().ok_or(())?;
        let currency = currencies.entry(currency_id).or_insert(PlayerCurrency {
            state: PlayerCurrencyState::New,
            quantity: 0,
            weekly_quantity: 0,
            tracked_quantity: 0,
            increased_cap_quantity: 0,
            earned_quantity: 0,
            flags: 0,
        });

        if currency.state != PlayerCurrencyState::New {
            currency.state = PlayerCurrencyState::Changed;
        }
        currency.quantity = currency.quantity.saturating_add(amount);

        let scaler = entry.scaler().max(1) as u32;
        let max_quantity = currency_max_quantity_cpp(&entry, currency);
        let delta = PlayerCurrencyDelta {
            currency_id,
            quantity: currency.quantity,
            amount,
            weekly_quantity: ((currency.weekly_quantity / scaler) > 0)
                .then_some(currency.weekly_quantity),
            max_quantity: (max_quantity != 0).then_some(max_quantity),
            total_earned: entry.has_total_earned().then_some(currency.earned_quantity),
            suppress_chat_log: entry.is_suppressing_chat_log(false),
        };
        if !self.set_player_currencies_like_cpp(currencies) {
            return Err(());
        }
        Ok(Some(delta))
    }
    /// Start the complete durable half of one shared money claim in a detached
    /// task.  The task owns the lease across `COMMIT`, commits the authority in
    /// the same task immediately after SQL success, then schedules the
    /// already-durable session-local applications.
    ///
    /// Dropping or aborting the packet-handler future only drops its
    /// `JoinHandle`; Tokio keeps this worker alive.  This closes the duplicate
    /// window where SQL could commit after the handler was cancelled while the
    /// lease's `Drop` reopened the object pool.
    ///
    /// Boundary: this is an in-process guarantee, not a durable claim journal.
    /// Aborting this detached task (including runtime/process shutdown) at the
    /// database commit await boundary can still lose the continuation between
    /// durable SQL and the synchronous authority commit. Recovery across that
    /// boundary requires persisting claim identity in the same transaction and
    /// replaying it at startup; #106 does not yet provide such a journal.
    /// Delivery tasks may likewise outlive a target session, but they carry
    /// only runtime publication: durable player money reloads from SQL and the
    /// already-committed authority prevents a second in-process payout.
    pub(crate) fn spawn_group_loot_money_persistence_like_cpp(
        &self,
        mut payouts: Vec<(ObjectGuid, u64)>,
        claim: LootClaimLease,
        mut deliveries: Vec<(LootMoneyDeliveryAddressLikeCpp, SessionCommand)>,
        authority_committed: Arc<AtomicBool>,
        viewer_fanout: LootMoneyViewerFanoutLikeCpp,
    ) -> Result<
        tokio::task::JoinHandle<Result<(), LootMoneyPersistenceErrorLikeCpp>>,
        LootMoneyPersistenceErrorLikeCpp,
    > {
        if payouts.is_empty() {
            return Err(LootMoneyPersistenceErrorLikeCpp::MissingPlayer);
        }
        if self.player_guid().is_none() {
            return Err(LootMoneyPersistenceErrorLikeCpp::MissingPlayer);
        }

        #[cfg(test)]
        let test_result = self.loot_money_persistence_test_result_like_cpp;
        #[cfg(not(test))]
        let test_result: Option<bool> = None;

        payouts
            .sort_unstable_by_key(|(recipient, _)| (recipient.high_value(), recipient.low_value()));
        payouts.dedup_by_key(|(recipient, _)| *recipient);

        // Register every recipient directly before any SQL begins. Commands
        // are publication-only; waiting for target acknowledgements here would
        // create self/A↔B deadlocks between concurrent group looters.
        let payout_recipients = payouts
            .iter()
            .map(|(recipient, _)| *recipient)
            .collect::<HashSet<_>>();
        let mut money_persistence_guards = HashMap::<
            ObjectGuid,
            DurableLootMoneyPersistenceGuardLikeCpp,
        >::with_capacity(payouts.len());
        let mut money_mutation_trackers = HashMap::<
            ObjectGuid,
            Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
        >::with_capacity(payouts.len());
        for (_, command) in &deliveries {
            let SessionCommand::ApplyLootMoneyLikeCpp(command) = command else {
                continue;
            };
            if payout_recipients.contains(&command.recipient)
                && !money_persistence_guards.contains_key(&command.recipient)
            {
                let guard = command
                    .durable_persistence_tracker
                    .begin_like_cpp()
                    .map_err(|_| LootMoneyPersistenceErrorLikeCpp::MissingPlayer)?;
                money_persistence_guards.insert(command.recipient, guard);
                money_mutation_trackers.insert(
                    command.recipient,
                    Arc::clone(&command.durable_persistence_tracker),
                );
            }
        }
        if money_persistence_guards.len() != payouts.len() {
            return Err(LootMoneyPersistenceErrorLikeCpp::MissingPlayer);
        }

        let persistence_port = if test_result.is_some() {
            None
        } else {
            Some(
                self.group_loot_money_persistence_port_like_cpp()
                    .ok_or(LootMoneyPersistenceErrorLikeCpp::MissingCharacterDatabase)?,
            )
        };

        let mut persistence_guard = claim
            .begin_persistence_guard_like_cpp()
            .map_err(LootMoneyPersistenceErrorLikeCpp::Claim)?;
        drop(claim);

        Ok(tokio::spawn(async move {
            // Match the sorted character-row lock order below. Stored-item
            // money uses the same per-character lock before it takes the row,
            // so COMMIT reconciliation cannot be confused by a later local
            // payout interleaving between the failed reply and our reads.
            let mut _money_mutation_locks = Vec::with_capacity(payouts.len());
            for (recipient, _) in &payouts {
                _money_mutation_locks.push(
                    money_mutation_trackers
                        .get(recipient)
                        .expect("every payout retained its target mutation lock")
                        .lock_money_mutation_like_cpp()
                        .await,
                );
            }

            let durable_outcomes = if let Some(success) = test_result {
                // Give cancellation regressions a deterministic opportunity to
                // drop the outer waiter while this detached task owns `claim`.
                tokio::task::yield_now().await;
                if !success {
                    return Err(LootMoneyPersistenceErrorLikeCpp::MissingCharacterDatabase);
                }
                payouts
                    .iter()
                    .map(|(recipient, amount)| {
                        (
                            recipient.counter() as u64,
                            wow_persistence::GroupLootMoneyPersistenceOutcomeLikeCpp {
                                recipient_guid: recipient.counter() as u64,
                                before: 0,
                                after: *amount,
                                applied_delta: *amount,
                            },
                        )
                    })
                    .collect::<HashMap<_, _>>()
            } else {
                let persistence_port = persistence_port
                    .expect("production loot-money worker must own a persistence port");
                let request = wow_persistence::GroupLootMoneyPersistenceRequestLikeCpp {
                    payouts: payouts
                        .iter()
                        .map(
                            |(recipient, amount)| wow_persistence::GroupLootMoneyPayoutLikeCpp {
                                recipient_guid: recipient.counter() as u64,
                                requested_delta: *amount,
                            },
                        )
                        .collect(),
                    max_money: MAX_MONEY_AMOUNT,
                };
                let durable_outcomes = match persistence_port
                    .attempt_group_loot_money_like_cpp(request)
                    .await
                {
                    wow_persistence::GroupLootMoneyPersistenceAttemptLikeCpp::Applied(outcomes) => outcomes
                        .into_iter()
                        .map(|outcome| (outcome.recipient_guid, outcome))
                        .collect::<HashMap<_, _>>(),
                    wow_persistence::GroupLootMoneyPersistenceAttemptLikeCpp::DefinitelyRolledBack {
                        kind,
                        reason,
                        ..
                    } => {
                        return Err(match kind {
                            wow_persistence::GroupLootMoneyRollbackKindLikeCpp::MissingPlayer { .. } => {
                                LootMoneyPersistenceErrorLikeCpp::MissingPlayer
                            }
                            wow_persistence::GroupLootMoneyRollbackKindLikeCpp::Database => {
                                LootMoneyPersistenceErrorLikeCpp::Persistence(reason)
                            }
                        });
                    }
                    wow_persistence::GroupLootMoneyPersistenceAttemptLikeCpp::CommitOutcomeUnknown {
                        reason,
                        outcomes: durable_outcomes,
                    } => match persistence_port
                        .reconcile_group_loot_money_like_cpp(durable_outcomes.clone())
                        .await
                    {
                        wow_persistence::GroupLootMoneyReconciliationLikeCpp::RolledBack => {
                            return Err(LootMoneyPersistenceErrorLikeCpp::Persistence(
                                "loot-money COMMIT was reconciled as rolled back".to_owned(),
                            ));
                        }
                        wow_persistence::GroupLootMoneyReconciliationLikeCpp::Indeterminate { .. } => {
                            for guard in money_persistence_guards.values_mut() {
                                guard.mark_indeterminate_like_cpp();
                            }
                            let _ = persistence_guard.quarantine_commit_unknown_like_cpp();
                            for (delivery, _) in &deliveries {
                                let kick = SessionCommand::KickLikeCpp(KickLikeCppCommand {
                                    reason: "loot-money COMMIT outcome is unknown; relog required"
                                        .to_string(),
                                });
                                delivery.clone().queue_reliably_like_cpp(kick);
                            }
                            return Err(
                                LootMoneyPersistenceErrorLikeCpp::CommitOutcomeUnknownPersistence(
                                    reason,
                                ),
                            );
                        }
                        wow_persistence::GroupLootMoneyReconciliationLikeCpp::CommittedOrCapOnlyNoop => durable_outcomes
                            .into_iter()
                            .map(|outcome| (outcome.recipient_guid, outcome))
                            .collect::<HashMap<_, _>>(),
                    },
                };
                durable_outcomes
            };

            for (_, command) in &mut deliveries {
                if let SessionCommand::ApplyLootMoneyLikeCpp(command) = command {
                    let outcome = durable_outcomes
                        .get(&(command.recipient.counter() as u64))
                        .copied()
                        .expect("every admitted payout retains its locked DB outcome");
                    command
                        .durable_applied_amount
                        .store(outcome.applied_delta, Ordering::Release);
                    money_persistence_guards
                        .get_mut(&command.recipient)
                        .expect("every payout registered its target money fence")
                        .commit_like_cpp(DurableLootMoneyCompletionLikeCpp {
                            durable_money_before: outcome.before,
                            durable_money_after: outcome.after,
                            durable_applied_amount: outcome.applied_delta,
                            applied: Arc::clone(&command.applied),
                        });
                }
            }

            let committed_snapshot = match persistence_guard.commit_with_snapshot_like_cpp() {
                Ok((_, committed_snapshot)) => {
                    authority_committed.store(true, Ordering::Release);
                    committed_snapshot
                }
                Err(error) => {
                    // SQL is already durable. Never let a lifecycle race reopen
                    // this old allocation; payout publication still proceeds,
                    // but replacement loot is not touched.
                    warn!(?error, "durable loot-money authority commit failed closed");
                    let _ = persistence_guard.quarantine_commit_unknown_like_cpp();
                    None
                }
            };

            // `Loot::PlayersLooting` can grow while the detached SQL task is
            // running. The snapshot above was captured by the commit while
            // holding the same authority mutex: every included opener saw
            // non-zero money, and every later opener sees zero directly.
            let mut viewers = committed_snapshot
                .filter(|snapshot| {
                    snapshot.generation == viewer_fanout.authority_generation
                        && snapshot.loot.loot_guid == viewer_fanout.loot_obj
                })
                .map(|snapshot| {
                    snapshot
                        .loot
                        .players_looting
                        .into_iter()
                        .collect::<HashSet<_>>()
                })
                .unwrap_or_default();

            for (_, command) in &mut deliveries {
                if let SessionCommand::ApplyLootMoneyLikeCpp(command) = command {
                    command
                        .send_coin_removed
                        .store(viewers.remove(&command.recipient), Ordering::Release);
                }
            }

            for viewer in viewers {
                if viewer_fanout.payout_recipients.contains(&viewer) {
                    continue;
                }
                let delivery = if viewer == viewer_fanout.source_player {
                    Some(LootMoneyDeliveryAddressLikeCpp::Source(
                        viewer_fanout.source_command_tx.clone(),
                    ))
                } else {
                    viewer_fanout.player_registry.as_ref().and_then(|registry| {
                        let registration = registry.in_world_loot_delivery_recipient(
                            viewer,
                            viewer_fanout.map_id,
                            viewer_fanout.instance_id,
                        )?;
                        Some(LootMoneyDeliveryAddressLikeCpp::Directory {
                            registry: Arc::clone(registry),
                            registration,
                        })
                    })
                };
                let Some(delivery) = delivery else {
                    continue;
                };
                deliveries.push((
                    delivery,
                    SessionCommand::NotifyLootMoneyRemovedLikeCpp(
                        NotifyLootMoneyRemovedLikeCppCommand {
                            recipient: viewer,
                            loot_owner: viewer_fanout.loot_owner,
                            loot_obj: viewer_fanout.loot_obj,
                            authority: viewer_fanout.authority.clone(),
                            authority_generation: viewer_fanout.authority_generation,
                            authority_committed: Arc::clone(&authority_committed),
                        },
                    ),
                ));
            }

            // Do not make persistence wait for another session's bounded
            // command queue. Each delivery task owns its command until the
            // target drains capacity or disconnects.
            for (delivery, command) in deliveries {
                delivery.queue_reliably_like_cpp(command);
            }
            Ok(())
        }))
    }
    #[cfg(test)]
    pub(crate) async fn money_changed_like_cpp(&mut self, new_money: u64) {
        let Some(old_money) = self.resolved_player_money_like_cpp() else {
            return;
        };
        self.enqueue_represented_quest_objective_progress_like_cpp(
            RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                old_money,
                new_money,
            },
        );
        self.drain_represented_quest_objective_progress_like_cpp()
            .await;
    }
    #[cfg(test)]
    pub(crate) async fn apply_player_money_change_like_cpp(
        &mut self,
        old_money: u64,
        new_money: u64,
    ) {
        if !self.stage_player_money_change_like_cpp(old_money, new_money) {
            return;
        }
        self.drain_represented_quest_objective_progress_like_cpp()
            .await;
    }
    /// Publish an already-durable absolute money mutation without awaiting.
    /// Transactional callers use this while their exclusive money guard is
    /// still held, then drop the guard before draining criteria (which can
    /// re-enter money persistence through a quest reward).
    pub(crate) fn stage_player_money_change_like_cpp(
        &mut self,
        old_money: u64,
        new_money: u64,
    ) -> bool {
        if !self.set_player_gold_like_cpp(new_money) {
            return false;
        }
        if old_money != new_money {
            self.enqueue_represented_quest_objective_progress_like_cpp(
                RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                    old_money,
                    new_money,
                },
            );
        }
        true
    }
    #[cfg(test)]
    pub(crate) async fn currency_changed_like_cpp(&mut self, currency_id: u32, change: i32) {
        self.enqueue_represented_quest_objective_progress_like_cpp(
            RepresentedQuestObjectiveProgressEventLikeCpp::CurrencyChanged {
                currency_id,
                change,
            },
        );
        self.drain_represented_quest_objective_progress_like_cpp()
            .await;
    }
    pub(crate) fn resolved_player_money_like_cpp(&self) -> Option<u64> {
        let canonical = self.with_owned_player_like_cpp(Player::money);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_gold);
        }
        canonical
    }
}
