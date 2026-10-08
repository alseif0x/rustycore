use super::*;
// The moved `LootMoney` command receiver is reached through the application loot
// owner's host trait default method (#1263 F5); the call text is unchanged.
use wow_world_application::LootHandlerHostLikeCpp;

impl WorldSession {
    /// CMSG_LOOT_MONEY — player takes money from the current loot view.
    pub async fn handle_loot_money_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let req = match LootMoney::read(&mut pkt) {
            Ok(r) => r,
            Err(e) => {
                warn!("Bad LootMoney: {e}");
                return;
            }
        };

        let player_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };

        debug!(
            account = self.core.account_id,
            is_soft_interact = req.is_soft_interact,
            "CMSG_LOOT_MONEY"
        );

        let mut active_owners: Vec<ObjectGuid> =
            self.loot.active_loot_view_owners_snapshot_like_cpp();
        if active_owners.is_empty() && !self.loot.active_loot_guid_like_cpp().is_empty() {
            active_owners.push(self.loot.active_loot_guid_like_cpp());
        }
        active_owners.sort_by_key(|guid| (guid.high_value(), guid.low_value()));

        if active_owners.is_empty() {
            return;
        }

        let money_by_loot: Vec<(ObjectGuid, ObjectGuid, u32)> = active_owners
            .into_iter()
            .filter_map(|loot_guid| {
                let loot = self.loot.cached_loot_for_owner_like_cpp(loot_guid)?;
                // C++ only places loot in Player::GetAELootView after the
                // player passed the source's loot-eligibility gate. Keep the
                // same invariant at this represented boundary so a stale or
                // forged local view cannot take another player's money.
                if !loot.allowed_looters.contains(&player_guid) {
                    return None;
                }
                Some((
                    loot_guid,
                    loot.loot_guid,
                    self.loot.represented_loot_money_for_player_like_cpp(
                        loot_guid,
                        loot,
                        player_guid,
                    ),
                ))
            })
            .collect();

        if money_by_loot.is_empty() {
            return;
        }

        let mut item_release: Vec<ObjectGuid> = Vec::new();
        let mut player_money_delta = 0u64;
        let mut legacy_money_processed = false;

        for (loot_guid, loot_obj, money) in &money_by_loot {
            let owned_authority = self
                .prepare_owned_loot_authority_for_active_request_outcome_like_cpp(
                    *loot_guid,
                    player_guid,
                );
            // An unreadable designated owner abandons this loot-money attempt
            // before any session-local fallback; genuine absence keeps the
            // historical fixture rule below.
            let owned_authority = match owned_authority {
                OwnedLootAuthorityLookupOutcomeLikeCpp::Unavailable => continue,
                OwnedLootAuthorityLookupOutcomeLikeCpp::Absent => None,
                OwnedLootAuthorityLookupOutcomeLikeCpp::Found(authority) => Some(authority),
            };
            let authority = owned_authority
                .as_ref()
                .filter(|authority| {
                    authority
                        .snapshot_for_player_like_cpp(player_guid)
                        .is_some()
                })
                .cloned();
            if authority.is_none()
                && (loot_guid.is_creature_or_vehicle() || loot_guid.is_game_object())
                && (owned_authority.is_some() || !represented_local_loot_fixture_allowed_like_cpp())
            {
                debug!(
                    owner = ?loot_guid,
                    "world-object loot money has no shared authority; refusing session-local fallback"
                );
                continue;
            }
            if let Some(authority) = authority {
                if !self.represented_active_loot_generation_matches_like_cpp(*loot_guid, &authority)
                {
                    debug!(
                        owner = ?loot_guid,
                        "delayed loot-money request does not belong to the active object generation"
                    );
                    continue;
                }
                let _ = self.reconcile_represented_loot_cache_like_cpp(*loot_guid, player_guid);
                self.loot
                    .ensure_represented_player_looting_like_cpp(*loot_guid, player_guid);

                let Some(expected_generation) = self
                    .loot
                    .active_loot_view_generation_like_cpp(*loot_guid)
                    .copied()
                else {
                    continue;
                };
                let claim = match authority
                    .reserve_money_for_generation_like_cpp(player_guid, expected_generation)
                    .await
                {
                    Ok(claim) => claim,
                    Err(_) => {
                        let _ =
                            self.reconcile_represented_loot_cache_like_cpp(*loot_guid, player_guid);
                        continue;
                    }
                };
                if !self
                    .loot
                    .represented_active_loot_claim_generation_matches_like_cpp(*loot_guid, &claim)
                {
                    claim.rollback_like_cpp();
                    continue;
                }
                let LootClaimPayload::Money(reserved_money) = claim.payload_like_cpp() else {
                    claim.rollback_like_cpp();
                    continue;
                };
                let authority_generation = claim.generation_like_cpp();
                let mut recipients = self.represented_loot_money_recipients_like_cpp(*loot_guid);
                recipients.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));
                recipients.dedup();
                if recipients.is_empty() {
                    recipients.push(player_guid);
                }
                let money_per_player = u64::from(*reserved_money) / recipients.len() as u64;
                let sole_looter = recipients.len() <= 1;
                let payouts = recipients
                    .iter()
                    .copied()
                    .map(|recipient| (recipient, money_per_player))
                    .collect::<Vec<_>>();
                let authority_committed = Arc::new(AtomicBool::new(false));
                let mut deliveries = Vec::with_capacity(recipients.len());
                let mut local_application = None;
                for recipient in recipients.iter().copied() {
                    let durable_applied_amount = Arc::new(AtomicU64::new(0));
                    let send_coin_removed = Arc::new(AtomicBool::new(false));
                    let applied = Arc::new(AtomicBool::new(false));
                    let published = Arc::new(AtomicBool::new(false));
                    let (delivery, application) = if recipient == player_guid {
                        let application = ApplyLootMoneyLikeCppCommand {
                            recipient,
                            loot_owner: *loot_guid,
                            loot_obj: *loot_obj,
                            amount: money_per_player,
                            durable_applied_amount,
                            durable_persistence_tracker: crate::session::cx_loot_ref(self)
                                .durable_loot_money_persistence_tracker_like_cpp(),
                            sole_looter,
                            authority: authority.clone(),
                            authority_generation,
                            authority_committed: Arc::clone(&authority_committed),
                            send_coin_removed,
                            applied,
                            published,
                        };
                        local_application = Some(application.clone());
                        (
                            LootMoneyDeliveryAddressLikeCpp::Source(self.session_command_tx()),
                            application,
                        )
                    } else {
                        let Some(registry) = self.player_registry().cloned() else {
                            deliveries.clear();
                            break;
                        };
                        let Some(prepared) = registry.prepare_loot_money_application(
                            PrepareLootMoneyApplicationLikeCpp {
                                recipient,
                                loot_owner: *loot_guid,
                                loot_obj: *loot_obj,
                                amount: money_per_player,
                                durable_applied_amount,
                                sole_looter,
                                authority: authority.clone(),
                                authority_generation,
                                authority_committed: Arc::clone(&authority_committed),
                                send_coin_removed,
                                applied,
                                published,
                            },
                        ) else {
                            deliveries.clear();
                            break;
                        };
                        (
                            LootMoneyDeliveryAddressLikeCpp::Directory {
                                registry,
                                registration: prepared.registration,
                            },
                            prepared.command,
                        )
                    };
                    deliveries.push((delivery, SessionCommand::ApplyLootMoneyLikeCpp(application)));
                }

                // Eligibility is chosen once, before persistence. If a
                // connected eligible member cannot be admitted, retry the
                // original pool instead of silently changing the divisor.
                if deliveries.len() != recipients.len() || deliveries.is_empty() {
                    claim.rollback_like_cpp();
                    let _ = self.reconcile_represented_loot_cache_like_cpp(*loot_guid, player_guid);
                    continue;
                }

                let current_map = self.core.player_map_id_like_cpp();
                let current_instance = self
                    .core
                    .current_canonical_player_map_key_like_cpp()
                    .map(|key| key.instance_id)
                    .unwrap_or(0);
                let viewer_fanout = LootMoneyViewerFanoutLikeCpp {
                    scope_player: player_guid,
                    source_player: player_guid,
                    source_command_tx: self.session_command_tx(),
                    player_registry: self.player_registry().cloned(),
                    map_id: current_map,
                    instance_id: current_instance,
                    loot_owner: *loot_guid,
                    loot_obj: *loot_obj,
                    authority: authority.clone(),
                    authority_generation,
                    payout_recipients: recipients.iter().copied().collect(),
                };

                let persistence = match self.spawn_group_loot_money_persistence_like_cpp(
                    payouts,
                    claim,
                    deliveries,
                    authority_committed,
                    viewer_fanout,
                ) {
                    Ok(persistence) => persistence,
                    Err(error) => {
                        warn!(
                            owner = ?loot_guid,
                            recipients = recipients.len(),
                            amount = money_per_player,
                            %error,
                            "atomic loot-money fanout could not start; pool remains available"
                        );
                        let _ =
                            self.reconcile_represented_loot_cache_like_cpp(*loot_guid, player_guid);
                        continue;
                    }
                };

                if let Err(error) = persistence.await.unwrap_or_else(|join_error| {
                    warn!(
                        owner = ?loot_guid,
                        ?join_error,
                        "atomic loot-money persistence worker terminated"
                    );
                    Err(crate::session::LootMoneyPersistenceErrorLikeCpp::WorkerTerminated)
                }) {
                    warn!(
                        owner = ?loot_guid,
                        recipients = recipients.len(),
                        amount = money_per_player,
                        %error,
                        "atomic loot-money fanout persistence failed; pool remains available"
                    );
                    let _ = self.reconcile_represented_loot_cache_like_cpp(*loot_guid, player_guid);
                    continue;
                }
                if let Some(application) = local_application {
                    self.handle_apply_loot_money_with_generator_like_cpp_command(
                        item_guid_generator,
                        application,
                    )
                    .await;
                }
                // The detached worker has already committed the authority and
                // queued the durable runtime applications. Returning to the
                // session loop lets this session drain its own command too.
                continue;
            }

            self.loot
                .ensure_represented_player_looting_like_cpp(*loot_guid, player_guid);

            if loot_guid.is_item() {
                let cached_amount = u64::from(*money);
                let Some((balance_applied, publication_applied, applied_delta, notified_amount)) =
                    self.persist_and_consume_stored_item_money_like_cpp(*loot_guid, cached_amount)
                        .await
                else {
                    continue;
                };
                let apply_balance = balance_applied
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok();
                let publish = publication_applied
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok();
                if !apply_balance && !publish {
                    continue;
                }

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
                    if applied_delta != 0 {
                        self.quest_state
                            .enqueue_represented_quest_objective_progress_like_cpp(
                                RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                                    old_money,
                                    new_money,
                                },
                            );
                    }
                }
                if publish {
                    self.represented_notify_money_removed_like_cpp(*loot_guid);
                    self.send_packet(&LootMoneyNotify {
                        money: notified_amount,
                        money_mod: 0,
                        sole_looter: true,
                    });
                    if let Some(loot) = self.loot.cached_loot_for_owner_mut_like_cpp(*loot_guid) {
                        loot.coins = 0;
                        if loot_is_looted_like_cpp(loot) {
                            item_release.push(*loot_guid);
                        }
                    }
                }
                if apply_balance || publish {
                    self.drain_represented_quest_objective_progress_with_generator_like_cpp(
                        item_guid_generator,
                    )
                    .await;
                }
                continue;
            }

            // Every live Creature/Vehicle/GameObject source must use its
            // object-owned authority, and stored Item money has the atomic
            // character/source-row transaction above. The remaining local
            // cache path exists only for pre-authority unit fixtures. Refuse
            // it in production so an unknown future owner type cannot publish
            // CoinRemoved or clear its pool before durable money succeeds.
            if !represented_local_loot_fixture_allowed_like_cpp() {
                debug!(
                    owner = ?loot_guid,
                    "non-authoritative loot-money fallback is disabled in production"
                );
                continue;
            }

            legacy_money_processed = true;
            self.represented_notify_money_removed_like_cpp(*loot_guid);

            let recipients = self.represented_loot_money_recipients_like_cpp(*loot_guid);
            let money = u64::from(*money);
            let money_per_player = money / recipients.len() as u64;
            let sole_looter = recipients.len() <= 1;

            let notify = LootMoneyNotify {
                money: money_per_player,
                money_mod: 0,
                sole_looter,
            };

            for recipient in recipients {
                if recipient == player_guid {
                    self.send_packet(&notify);
                    player_money_delta = player_money_delta.saturating_add(money_per_player);
                } else if let Some(registry) = self.player_registry() {
                    if let Some(member) = registry.loot_presence(recipient) {
                        let _ =
                            registry.send_current_packet(member.registration, notify.to_bytes());
                    }
                }
            }

            if let Some(loot) = self
                .loot
                .record_personal_loot_money_or_clear_cached_coins_for_owner_like_cpp(
                    *loot_guid,
                    player_guid,
                )
            {
                if loot_guid.is_item() && loot_is_looted_like_cpp(loot) {
                    item_release.push(*loot_guid);
                }
            }
        }

        if legacy_money_processed {
            if let Some((old_money, new_money)) = self
                .mutate_and_persist_player_gold_exclusive_like_cpp(|old_money| {
                    crate::session::loot_money_durable_outcome_like_cpp(
                        old_money,
                        player_money_delta,
                    )
                    .0
                })
                .await
            {
                if old_money != new_money {
                    self.quest_state
                        .enqueue_represented_quest_objective_progress_like_cpp(
                            RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                                old_money,
                                new_money,
                            },
                        );
                }
            }
            self.drain_represented_quest_objective_progress_with_generator_like_cpp(
                item_guid_generator,
            )
            .await;
        }

        for loot_guid in item_release {
            self.loot.remove_cached_loot_for_owner_like_cpp(loot_guid);
            self.loot.clear_active_loot_guid_if(loot_guid);
            self.send_packet(&SLootRelease {
                loot_obj: loot_guid,
                owner: player_guid,
            });
            self.destroy_fully_looted_direct_item(loot_guid).await;
        }

        let _ = player_guid;
    }

    #[cfg(test)]
    pub async fn handle_loot_money(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_loot_money_with_generator_like_cpp(generators.item.as_ref(), pkt)
            .await;
    }

    /// Test-only receiver entry point. The receiver itself now lives in the
    /// application loot owner; this shim only supplies the session generator
    /// bundle the pre-move entry point built, and has three test callers.
    #[cfg(test)]
    pub(crate) async fn handle_apply_loot_money_like_cpp_command(
        &mut self,
        command: ApplyLootMoneyLikeCppCommand,
    ) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_apply_loot_money_with_generator_like_cpp_command(
            generators.item.as_ref(),
            command,
        )
        .await;
    }
}
