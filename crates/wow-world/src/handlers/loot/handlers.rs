// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot packet entry points and their handler registrations.

use super::*;
use wow_packet::ClientPacket;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LootUnit,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_unit",
        handler: |session, pkt| Box::pin(async move { session.handle_loot_unit(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LootItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_item",
        handler: |session, pkt| Box::pin(async move { session.handle_loot_item(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LootMoney,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_money",
        handler: |session, pkt| Box::pin(async move { session.handle_loot_money(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LootRelease,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_release",
        handler: |session, pkt| Box::pin(async move { session.handle_loot_release(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LootRoll,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_roll",
        handler: |session, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::loot::LootRoll::read(&mut pkt) {
                    Ok(roll) => session.handle_loot_roll(roll).await,
                    Err(e) => tracing::warn!("Failed to read LootRoll: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MasterLootItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_master_loot_item",
        handler: |session, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::loot::MasterLootItem::read(&mut pkt) {
                    Ok(master_loot_item) => session.handle_master_loot_item(master_loot_item).await,
                    Err(e) => tracing::warn!("Failed to read MasterLootItem: {e}"),
                }
            })
        },
    }
}

// The inspected TrinityCore opcode table assigns the shared unresolved 0xBADD
// placeholder to CMSG_CLEAR_RAID_MARKER (uint8 payload),
// CMSG_SET_LOOT_SPECIALIZATION (uint32), CMSG_SET_SAVED_INSTANCE_EXTEND
// (int32+uint32+bit), CMSG_CANCEL_MOD_SPEED_NO_CONTROL_AURAS (packed GUID) and
// CMSG_CLIENT_PORT_GRAVEYARD (empty). Rust keeps one enum variant and splits by
// payload length until the real opcode table is resolved, so this one
// registration carries all five payload shapes.
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetLootSpecialization,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_loot_specialization",
        handler: |session, mut pkt| {
            Box::pin(async move {
                if session
                    .try_handle_cancel_mod_speed_no_control_auras_like_cpp(pkt.clone())
                    .await
                {
                    return;
                }
                if session
                    .try_handle_client_port_graveyard_like_cpp(pkt.clone())
                    .await
                {
                    return;
                }
                if pkt.remaining() == 1 {
                    session.handle_clear_raid_marker(pkt).await;
                } else if pkt.remaining() == 4 {
                    match wow_packet::packets::loot::SetLootSpecialization::read(&mut pkt) {
                        Ok(set_loot_specialization) => {
                            session
                                .handle_set_loot_specialization(set_loot_specialization)
                                .await;
                        }
                        Err(e) => tracing::warn!("Failed to read SetLootSpecialization: {e}"),
                    }
                } else if pkt.remaining() == 9 {
                    match wow_packet::packets::misc::SetSavedInstanceExtend::read(&mut pkt) {
                        Ok(query) => session.handle_set_saved_instance_extend(query).await,
                        Err(e) => tracing::warn!("Failed to read SetSavedInstanceExtend: {e}"),
                    }
                } else {
                    tracing::warn!(
                        opcode = ?ClientOpcodes::SetLootSpecialization,
                        remaining = pkt.remaining(),
                        "unresolved 0xBADD payload shape"
                    );
                }
            })
        },
    }
}

impl WorldSession {
    /// CMSG_LOOT_UNIT — player right-clicks a dead creature to loot it.
    pub async fn handle_loot_unit(&mut self, mut pkt: wow_packet::WorldPacket) {
        let req = match LootUnit::read(&mut pkt) {
            Ok(r) => r,
            Err(e) => {
                warn!("Bad LootUnit: {e}");
                return;
            }
        };

        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        debug!(account = self.account_id, target = ?req.unit, "CMSG_LOOT_UNIT");

        if self.resolved_player_is_alive_like_cpp() != Some(true) {
            return;
        }

        if !req.unit.is_creature_or_vehicle() {
            return;
        }

        // Check creature exists and is dead.
        let creature_state = match self.represented_creature_loot_state_like_cpp(req.unit) {
            Some(state) => state,
            None => {
                warn!("LootUnit: creature {:?} not found", req.unit);
                return;
            }
        };

        if creature_state.is_alive {
            return;
        }

        if self
            .player_position_like_cpp()
            .is_some_and(|player| !player.is_within_dist(&creature_state.position, 30.0))
        {
            return;
        }

        self.interrupt_non_melee_spell_cast_for_loot_like_cpp();
        self.remove_auras_with_looting_interrupt_flags_like_cpp();

        let ae_owner_guids = if self.enable_ae_loot_like_cpp() {
            self.represented_ae_loot_creature_targets_like_cpp(req.unit, player_guid)
                .await
        } else {
            Vec::new()
        };

        if !ae_owner_guids.is_empty() {
            self.send_packet(&AELootTargets {
                count: ae_owner_guids.len() as u32 + 1,
            });
        }

        let Some(response) = self
            .represented_loot_response_for_owner_like_cpp(req.unit, player_guid, false)
            .await
        else {
            return;
        };
        if self.has_active_non_item_loot_views_like_cpp() {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }
        self.set_active_loot_guid(req.unit);
        self.represented_on_loot_opened_like_cpp(req.unit, player_guid, response);

        if !ae_owner_guids.is_empty() {
            self.send_packet(&AELootTargetsAck);

            for owner_guid in ae_owner_guids {
                if let Some(response) = self
                    .represented_loot_response_for_owner_like_cpp(owner_guid, player_guid, true)
                    .await
                {
                    self.add_active_loot_view_owner_like_cpp(owner_guid);
                    self.represented_on_loot_opened_like_cpp(owner_guid, player_guid, response);
                    self.send_packet(&AELootTargetsAck);
                }
            }
        }
    }

    /// Receiver-owned half of the loot-release VALUES fanout. Applying
    /// `Player::isAllowedToLoot` here preserves session-local pending-bind
    /// state and avoids serialising one player's dynamic flags for another.
    pub(crate) fn handle_send_creature_loot_release_values_update_command_like_cpp(
        &mut self,
        command: SendCreatureLootReleaseValuesUpdateLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn
            || self.player_map_id_like_cpp() != command.map_id
        {
            return;
        }
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if instance_id != command.instance_id
            || !self
                .client_visible_guids_like_cpp
                .contains(&command.creature_guid)
        {
            return;
        }
        if self.represented_can_receive_creature_message_to_set_by_guid_like_cpp(
            command.creature_guid,
            command.map_id,
            command.instance_id,
            false,
        ) != Some(true)
        {
            return;
        }
        let Some(expected_authority) = command.authority.as_ref() else {
            return;
        };
        let Some(current_authority) =
            self.represented_owned_loot_authority_like_cpp(command.creature_guid)
        else {
            return;
        };
        if !current_authority.shares_storage_like_cpp(expected_authority) {
            // The queued update belongs to an older corpse generation. C++
            // publishes synchronously before respawn; Rust must not apply the
            // delayed VALUES delta to a replacement creature with the same GUID.
            return;
        }
        let Some(viewer_guid) = self.player_guid() else {
            return;
        };
        let viewer_update = self.creature_loot_release_values_for_viewer_like_cpp(
            command.creature_guid,
            viewer_guid,
            self.pending_bind.is_some(),
            Some(expected_authority),
            command.unit_values_update,
        );
        self.send_packet(&UpdateObject::unit_values_update(
            command.creature_guid,
            command.map_id,
            viewer_update,
        ));
    }

    /// CMSG_LOOT_ITEM — player clicks to take a specific item from the loot.
    pub async fn handle_loot_item(&mut self, mut pkt: wow_packet::WorldPacket) {
        let req = match LootItemPkt::read(&mut pkt) {
            Ok(r) => r,
            Err(e) => {
                warn!("Bad LootItem: {e}");
                return;
            }
        };

        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        let mut taken_items: Vec<(ObjectGuid, ObjectGuid, u8, u32, u32, bool)> = Vec::new();
        let mut canonical_loot_sync: Vec<ObjectGuid> = Vec::new();

        for loot_req in &req.requests {
            let Some(owner_guid) = self.active_loot_owner_for_loot_object_like_cpp(loot_req.object)
            else {
                self.send_packet(&SLootRelease {
                    loot_obj: ObjectGuid::EMPTY,
                    owner: player_guid,
                });
                continue;
            };

            if owner_guid.is_game_object()
                && !self.represented_gameobject_can_autostore_loot_item_like_cpp(
                    owner_guid,
                    player_guid,
                )
            {
                self.send_packet(&SLootRelease {
                    loot_obj: owner_guid,
                    owner: player_guid,
                });
                continue;
            }

            if owner_guid.is_creature_or_vehicle() {
                let Some(creature_position) =
                    self.represented_creature_position_for_loot_like_cpp(owner_guid)
                else {
                    self.send_loot_error_like_cpp(
                        loot_req.object,
                        owner_guid,
                        LOOT_ERROR_NO_LOOT_LIKE_CPP,
                    );
                    continue;
                };

                if self
                    .player_position_like_cpp()
                    .is_some_and(|player| !player.is_within_dist(&creature_position, 30.0))
                {
                    self.send_loot_error_like_cpp(
                        loot_req.object,
                        owner_guid,
                        LOOT_ERROR_TOO_FAR_LIKE_CPP,
                    );
                    continue;
                }
            }

            let owned_authority = self
                .prepare_owned_loot_authority_for_active_request_like_cpp(owner_guid, player_guid);
            let authority = owned_authority
                .as_ref()
                .filter(|authority| {
                    authority
                        .snapshot_for_player_like_cpp(player_guid)
                        .is_some()
                })
                .cloned();
            if authority.is_none()
                && (owner_guid.is_creature_or_vehicle() || owner_guid.is_game_object())
                && (owned_authority.is_some() || !represented_local_loot_fixture_allowed_like_cpp())
            {
                self.send_equip_error(InventoryResult::LootGone, None, None, 0, 0);
                continue;
            }
            if let Some(authority) = authority.as_ref() {
                if !self.represented_active_loot_generation_matches_like_cpp(owner_guid, authority)
                {
                    self.send_equip_error(InventoryResult::LootGone, None, None, 0, 0);
                    continue;
                }
                let _ = self.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid);
            }
            self.ensure_represented_player_looting_like_cpp(owner_guid, player_guid);

            let Some((cached_entry, dungeon_encounter_id)) =
                self.loot_table.get(&owner_guid).and_then(|loot| {
                    loot.items
                        .iter()
                        .find(|entry| {
                            entry.loot_list_id == loot_req.loot_list_id
                                && !loot_item_is_looted_for_player_like_cpp(
                                    loot,
                                    entry,
                                    player_guid,
                                )
                        })
                        .cloned()
                        .map(|entry| (entry, loot.dungeon_encounter_id))
                })
            else {
                self.send_equip_error(InventoryResult::LootGone, None, None, 0, 0);
                continue;
            };

            if !cached_entry.has_allowed_looter_like_cpp(player_guid) {
                self.send_packet(&LootReleaseAll);
                continue;
            }

            if cached_entry.flags.blocked {
                self.send_packet(&LootReleaseAll);
                continue;
            }

            if !cached_entry.roll_winner_allows_like_cpp(player_guid) {
                self.send_packet(&LootReleaseAll);
                continue;
            }

            let (entry, claim) = if let Some(authority) = authority {
                let Some(expected_generation) = self
                    .active_loot_view_generations_like_cpp
                    .get(&owner_guid)
                    .copied()
                else {
                    self.send_equip_error(InventoryResult::LootGone, None, None, 0, 0);
                    continue;
                };
                let claim = match authority
                    .reserve_item_for_generation_like_cpp(
                        player_guid,
                        loot_req.loot_list_id,
                        expected_generation,
                    )
                    .await
                {
                    Ok(claim) => claim,
                    Err(_) => {
                        let _ =
                            self.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid);
                        self.send_equip_error(InventoryResult::LootGone, None, None, 0, 0);
                        continue;
                    }
                };
                if !self
                    .represented_active_loot_claim_generation_matches_like_cpp(owner_guid, &claim)
                {
                    claim.rollback_like_cpp();
                    self.send_equip_error(InventoryResult::LootGone, None, None, 0, 0);
                    continue;
                }
                let LootClaimPayload::Item(entry) = claim.payload_like_cpp() else {
                    claim.rollback_like_cpp();
                    self.send_equip_error(InventoryResult::LootGone, None, None, 0, 0);
                    continue;
                };
                (entry.clone(), Some(claim))
            } else {
                (cached_entry, None)
            };

            let stored = if let Some(claim) = claim.as_ref() {
                self.store_claimed_direct_loot_item_from_owner_like_cpp(
                    &entry,
                    dungeon_encounter_id,
                    owner_guid,
                    loot_req.object,
                    claim,
                )
                .await
            } else {
                self.store_direct_loot_item_from_owner_like_cpp(
                    &entry,
                    dungeon_encounter_id,
                    owner_guid,
                )
                .await
            };
            if !stored {
                continue;
            }

            if owner_guid.is_item() {
                // The detached worker published this exact durable removal to
                // the session tracker before its JoinHandle completed. Apply
                // it here on the normal path; logout/disconnect and the
                // session tick drain the same completion after cancellation.
                self.apply_pending_durable_item_loot_completions_like_cpp()
                    .await;
                debug!(
                    account = self.account_id,
                    item = entry.item_id,
                    quantity = entry.quantity,
                    "Looted item"
                );
                continue;
            }

            if claim.is_some() {
                debug!(
                    account = self.account_id,
                    item = entry.item_id,
                    quantity = entry.quantity,
                    "Looted item"
                );
                continue;
            }

            if let Some(loot) = self.loot_table.get_mut(&owner_guid) {
                if let Some(entry) = loot
                    .items
                    .iter()
                    .find(|entry| entry.loot_list_id == loot_req.loot_list_id)
                    .cloned()
                {
                    mark_loot_item_looted_for_player_like_cpp(
                        loot,
                        loot_req.loot_list_id,
                        player_guid,
                    );
                    taken_items.push((
                        owner_guid,
                        loot_req.object,
                        entry.loot_list_id,
                        entry.item_id,
                        entry.quantity,
                        entry.flags.freeforall,
                    ));
                    canonical_loot_sync.push(owner_guid);
                }
            }
        }

        canonical_loot_sync.sort_by_key(|guid| (guid.high_value(), guid.low_value()));
        canonical_loot_sync.dedup();
        for owner_guid in canonical_loot_sync {
            self.refresh_represented_loot_owner_canonical_summary_like_cpp(owner_guid, player_guid);
        }

        for (owner_guid, loot_obj, list_id, item_id, quantity, freeforall) in taken_items {
            if freeforall {
                let removed = LootRemoved {
                    owner: owner_guid,
                    loot_obj,
                    loot_list_id: list_id,
                };
                self.send_packet(&removed);
            } else {
                self.represented_notify_loot_item_removed_like_cpp(owner_guid, list_id);
            }
            debug!(
                account = self.account_id,
                item = item_id,
                quantity,
                "Looted item"
            );
        }
    }

    /// CMSG_LOOT_MONEY — player takes money from the current loot view.
    pub async fn handle_loot_money(&mut self, mut pkt: wow_packet::WorldPacket) {
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
            account = self.account_id,
            is_soft_interact = req.is_soft_interact,
            "CMSG_LOOT_MONEY"
        );

        let mut active_owners: Vec<ObjectGuid> =
            self.active_loot_view_owners.iter().copied().collect();
        if active_owners.is_empty() && !self.active_loot_guid.is_empty() {
            active_owners.push(self.active_loot_guid);
        }
        active_owners.sort_by_key(|guid| (guid.high_value(), guid.low_value()));

        if active_owners.is_empty() {
            return;
        }

        let money_by_loot: Vec<(ObjectGuid, ObjectGuid, u32)> = active_owners
            .into_iter()
            .filter_map(|loot_guid| {
                let loot = self.loot_table.get(&loot_guid)?;
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
                    self.represented_loot_money_for_player_like_cpp(loot_guid, loot, player_guid),
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
                .prepare_owned_loot_authority_for_active_request_like_cpp(*loot_guid, player_guid);
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
                self.ensure_represented_player_looting_like_cpp(*loot_guid, player_guid);

                let Some(expected_generation) = self
                    .active_loot_view_generations_like_cpp
                    .get(loot_guid)
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
                            durable_persistence_tracker: self
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

                let current_map = self.player_map_id_like_cpp();
                let current_instance = self
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
                    self.handle_apply_loot_money_like_cpp_command(application)
                        .await;
                }
                // The detached worker has already committed the authority and
                // queued the durable runtime applications.  Returning to the
                // session loop lets this session drain its own command too.
                continue;
            }

            self.ensure_represented_player_looting_like_cpp(*loot_guid, player_guid);

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
                        self.enqueue_represented_quest_objective_progress_like_cpp(
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
                    if let Some(loot) = self.loot_table.get_mut(loot_guid) {
                        loot.coins = 0;
                        if loot_is_looted_like_cpp(loot) {
                            item_release.push(*loot_guid);
                        }
                    }
                }
                if apply_balance || publish {
                    self.drain_represented_quest_objective_progress_like_cpp()
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

            let personal_money_owner = self.represented_personal_loot_owners.contains(loot_guid);
            if let Some(loot) = self.loot_table.get_mut(loot_guid) {
                if personal_money_owner {
                    self.represented_personal_loot_money
                        .insert((*loot_guid, player_guid), 0);
                } else {
                    loot.coins = 0;
                }

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
                    self.enqueue_represented_quest_objective_progress_like_cpp(
                        RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                            old_money,
                            new_money,
                        },
                    );
                }
            }
            self.drain_represented_quest_objective_progress_like_cpp()
                .await;
        }

        for loot_guid in item_release {
            self.loot_table.remove(&loot_guid);
            self.clear_active_loot_guid_if(loot_guid);
            self.send_packet(&SLootRelease {
                loot_obj: loot_guid,
                owner: player_guid,
            });
            self.destroy_fully_looted_direct_item(loot_guid).await;
        }

        let _ = player_guid;
    }

    /// CMSG_LOOT_RELEASE — player closes the loot window.
    ///
    /// C++ `WorldSession::DoLootRelease` creature branch:
    /// `loot->isLooted() && creature->IsFullyLooted()` removes the lootable
    /// dynamic flag and calls `Creature::AllLootRemovedFromCorpse` for a corpse.
    pub async fn handle_loot_release(&mut self, mut pkt: wow_packet::WorldPacket) {
        let req = match LootRelease::read(&mut pkt) {
            Ok(r) => r,
            Err(e) => {
                warn!("Bad LootRelease: {e}");
                return;
            }
        };

        debug!(account = self.account_id, unit = ?req.unit, "CMSG_LOOT_RELEASE");

        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        self.do_loot_release_owner_like_cpp(req.unit, player_guid)
            .await;
    }

    /// CMSG_LOOT_ROLL — vote on a pending group loot roll.
    ///
    /// C++ `HandleLootRoll` silently returns when `GetLootRoll` finds no
    /// canonical roll state. Rust does not yet port that state machine, so this
    /// represented handler preserves the current wire behavior without emitting
    /// synthetic errors.
    pub async fn handle_loot_roll(&mut self, roll: LootRoll) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };

        if self
            .represented_player_vote_on_loot_roll_like_cpp(&roll, player_guid)
            .await
        {
            return;
        }

        if self.route_represented_remote_loot_roll_vote_to_owner_like_cpp(&roll, player_guid) {
            return;
        }

        debug!(
            account = self.account_id,
            loot_obj = ?roll.loot_obj,
            loot_list_id = roll.loot_list_id,
            roll_type = roll.roll_type,
            "CMSG_LOOT_ROLL ignored: canonical LootRoll state is not ported yet"
        );
    }

    /// CMSG_MASTER_LOOT_ITEM — master looter assigns loot to a target.
    ///
    /// C++ first rejects players that are not in a group or are not the group's
    /// master looter with `LOOT_ERROR_DIDNT_KILL`. Current Rust group state has
    /// loot method `MASTER_LOOT` and the stored master-looter GUID matching the
    /// current player.
    pub async fn handle_master_loot_item(&mut self, master_loot_item: MasterLootItem) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };

        let is_represented_master_looter = if let (Some(group_guid), Some(registry)) =
            (self.resolved_group_guid_like_cpp(), self.group_registry())
        {
            registry.get(&group_guid).is_some_and(|group| {
                group.loot_method == LOOT_METHOD_MASTER_LIKE_CPP
                    && group.master_looter_guid == player_guid
            })
        } else {
            false
        };

        if !is_represented_master_looter {
            self.send_loot_error_like_cpp(
                ObjectGuid::EMPTY,
                ObjectGuid::EMPTY,
                LOOT_ERROR_DIDNT_KILL_LIKE_CPP,
            );
            return;
        }

        if !self.represented_master_loot_target_exists_like_cpp(master_loot_item.target) {
            self.send_loot_error_like_cpp(
                ObjectGuid::EMPTY,
                ObjectGuid::EMPTY,
                LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP,
            );
            return;
        }

        let mut current_session_assignments = 0_u32;

        for req in &master_loot_item.loot {
            let Some(owner_guid) = self.active_loot_owner_for_loot_object_like_cpp(req.object)
            else {
                return;
            };

            if !self.represented_master_loot_target_eligible_like_cpp(master_loot_item.target) {
                self.send_loot_error_like_cpp(
                    req.object,
                    owner_guid,
                    LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                );
                return;
            }

            let owned_authority = self
                .prepare_owned_loot_authority_for_active_request_like_cpp(owner_guid, player_guid);
            let authority = owned_authority
                .as_ref()
                .filter(|authority| {
                    authority
                        .snapshot_for_player_like_cpp(master_loot_item.target)
                        .is_some()
                })
                .cloned();
            if authority.is_none()
                && (owner_guid.is_creature_or_vehicle() || owner_guid.is_game_object())
                && (owned_authority.is_some() || !represented_local_loot_fixture_allowed_like_cpp())
            {
                self.send_loot_error_like_cpp(
                    req.object,
                    owner_guid,
                    LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                );
                return;
            }
            if let Some(authority) = authority.as_ref() {
                if !self.represented_active_loot_generation_matches_like_cpp(owner_guid, authority)
                {
                    self.send_loot_error_like_cpp(
                        req.object,
                        owner_guid,
                        LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                    );
                    return;
                }
                let _ = self
                    .reconcile_represented_loot_cache_like_cpp(owner_guid, master_loot_item.target);
            }

            let Some(loot) = self.loot_table.get(&owner_guid) else {
                return;
            };
            let dungeon_encounter_id = loot.dungeon_encounter_id;

            if loot.loot_method != LOOT_METHOD_MASTER_LIKE_CPP {
                return;
            }

            if !loot.allowed_looters.contains(&master_loot_item.target) {
                self.send_loot_error_like_cpp(
                    req.object,
                    owner_guid,
                    LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                );
                return;
            }

            if req.loot_list_id as usize >= loot.items.len() {
                return;
            }

            let item = &loot.items[req.loot_list_id as usize];
            if !item.allowed_looters.is_empty()
                && !item.allowed_looters.contains(&master_loot_item.target)
            {
                self.send_loot_error_like_cpp(
                    req.object,
                    owner_guid,
                    LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                );
                return;
            }

            if let Some(error) = self.represented_master_loot_can_store_error_like_cpp(
                master_loot_item.target,
                item.item_id,
                item.quantity,
            ) {
                self.send_loot_error_like_cpp(req.object, owner_guid, error);
                return;
            }

            let mut entry = item.clone();
            let claim = if let Some(authority) = authority {
                let Some(expected_generation) = self
                    .active_loot_view_generations_like_cpp
                    .get(&owner_guid)
                    .copied()
                else {
                    self.send_loot_error_like_cpp(
                        req.object,
                        owner_guid,
                        LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                    );
                    return;
                };
                let claim = match authority
                    .reserve_item_for_award_generation_like_cpp(
                        master_loot_item.target,
                        req.loot_list_id,
                        expected_generation,
                    )
                    .await
                {
                    Ok(claim) => claim,
                    Err(_) => {
                        self.send_loot_error_like_cpp(
                            req.object,
                            owner_guid,
                            LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                        );
                        return;
                    }
                };
                if !self
                    .represented_active_loot_claim_generation_matches_like_cpp(owner_guid, &claim)
                {
                    claim.rollback_like_cpp();
                    self.send_loot_error_like_cpp(
                        req.object,
                        owner_guid,
                        LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                    );
                    return;
                }
                if let LootClaimPayload::Item(reserved_entry) = claim.payload_like_cpp() {
                    entry = reserved_entry.clone();
                }
                Some(claim)
            } else {
                None
            };
            if master_loot_item.target == player_guid {
                let stored = if let Some(claim) = claim.as_ref() {
                    self.store_claimed_direct_loot_item_from_owner_like_cpp(
                        &entry,
                        dungeon_encounter_id,
                        owner_guid,
                        req.object,
                        claim,
                    )
                    .await
                } else {
                    self.store_direct_loot_item_from_owner_like_cpp(
                        &entry,
                        dungeon_encounter_id,
                        owner_guid,
                    )
                    .await
                };
                if !stored {
                    return;
                }
                if claim.is_none() {
                    self.mark_represented_master_loot_item_removed_like_cpp(
                        owner_guid,
                        req.object,
                        req.loot_list_id,
                        master_loot_item.target,
                    );
                }
                current_session_assignments = current_session_assignments.saturating_add(1);
            } else {
                let authoritative_claim = claim.is_some();
                match self
                    .request_represented_remote_master_loot_give_like_cpp(
                        master_loot_item.target,
                        owner_guid,
                        req.object,
                        req.loot_list_id,
                        dungeon_encounter_id,
                        entry,
                        claim,
                    )
                    .await
                {
                    MasterLootGiveResult::Stored if !authoritative_claim => {
                        self.mark_represented_master_loot_item_removed_like_cpp(
                            owner_guid,
                            req.object,
                            req.loot_list_id,
                            master_loot_item.target,
                        );
                    }
                    MasterLootGiveResult::Stored => {}
                    MasterLootGiveResult::StoreFailed(error) => {
                        self.send_loot_error_like_cpp(req.object, owner_guid, error);
                        return;
                    }
                    MasterLootGiveResult::TargetMismatch => {
                        self.send_loot_error_like_cpp(
                            ObjectGuid::EMPTY,
                            ObjectGuid::EMPTY,
                            LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP,
                        );
                        return;
                    }
                }
            }
        }

        debug!(
            account = self.account_id,
            target = ?master_loot_item.target,
            request_count = master_loot_item.loot.len(),
            current_session_assignments,
            "CMSG_MASTER_LOOT_ITEM accepted; represented self and connected remote target assignments route through target session state"
        );
    }

    pub(crate) fn handle_apply_group_removal_command_like_cpp(
        &mut self,
        command: ApplyGroupRemovalLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        if self.resolved_group_guid_like_cpp() != Some(command.group_guid) {
            return;
        }

        let _ = self.set_owned_player_group_like_cpp(None);
        self.clear_represented_group_subgroup_like_cpp();
        self.send_player_party_type_update_like_cpp(command.category, command.party_type);
        self.sync_player_registry_state_like_cpp();

        if command.refresh_visible_gameobjects_or_spellclicks {
            let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
        }
        if command.send_group_destroyed {
            self.send_packet_realm(&wow_packet::packets::party::GroupDestroyed);
        }
        if command.send_group_uninvite {
            self.send_packet_realm(&wow_packet::packets::party::GroupUninvite);
        }
        // C++ `Group::RemoveMember` (`Group.cpp:654-655`) and `Group::Disband`
        // (`Group.cpp:746`) both finish by sending the removed player the
        // destroyed `PartyUpdate` so its client tears down the party frames.
        if command.send_group_destroyed || command.send_group_uninvite {
            self.send_destroyed_group_party_update_like_cpp(command.group_guid, command.category);
        }
    }

    pub(crate) fn handle_apply_group_join_command_like_cpp(
        &mut self,
        command: ApplyGroupJoinLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }

        self.apply_group_join_like_cpp(command.group_guid, command.subgroup);
        self.send_player_party_type_update_like_cpp(command.category, command.party_type);

        if command.refresh_visible_gameobjects_or_spellclicks {
            let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
        }
    }

    pub(crate) fn handle_send_party_update_command_like_cpp(
        &mut self,
        mut command: SendPartyUpdateLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        if self.player_guid() != Some(command.recipient) {
            return;
        }

        let Some(sequence_num) =
            self.next_group_update_sequence_number_like_cpp(command.party_update.party_index)
        else {
            return;
        };
        command.party_update.sequence_num = sequence_num;
        // `SMSG_PARTY_UPDATE` and `SMSG_PARTY_MEMBER_FULL_STATE` are both
        // CONNECTION_TYPE_REALM in legacy C++ Opcodes.cpp:1829/1832.
        self.send_packet_realm(&command.party_update);
        for packet in command.member_full_state_packets {
            self.send_raw_packet_realm(&packet);
        }
    }

    pub(crate) fn handle_apply_group_difficulty_command_like_cpp(
        &mut self,
        command: crate::session::mailbox::ApplyGroupDifficultyLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        self.apply_group_difficulty_like_cpp(
            command.group_guid,
            command.difficulty_id,
            command.kind,
        );
    }

    pub(crate) fn handle_apply_group_subgroup_command_like_cpp(
        &mut self,
        command: crate::session::mailbox::ApplyGroupSubgroupLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        self.apply_group_subgroup_like_cpp(command.group_guid, command.subgroup);
    }

    /// Mirrors the small gathering-node state subset that C++ keeps on the
    /// shared GameObject before asking this session to recompute its visible
    /// GameObject dynamic-flag deltas.
    pub(crate) fn handle_sync_gathering_node_gameobject_state_and_refresh_like_cpp(
        &mut self,
        command: SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        if command.map_id != self.player_map_id_like_cpp() {
            return;
        }
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if command.instance_id != current_instance_id {
            return;
        }
        if u32::from(command.go_type) != GAMEOBJECT_TYPE_GATHERING_NODE {
            return;
        }
        let loot_state = match command.loot_state {
            Some(0) => Some(LootState::NotReady),
            Some(1) => Some(LootState::Ready),
            Some(2) => Some(LootState::Activated),
            Some(3) => Some(LootState::JustDeactivated),
            Some(_) => return,
            None => None,
        };
        let go_state = match command.go_state {
            Some(0) => Some(GoState::Active),
            Some(1) => Some(GoState::Ready),
            Some(2) => Some(GoState::Destroyed),
            Some(24) => Some(GoState::TransportActive),
            Some(25) => Some(GoState::TransportStopped),
            Some(_) => return,
            None => None,
        };

        {
            let state = self
                .represented_gameobject_use_states
                .entry(command.gameobject_guid)
                .or_default();
            state.map_id = Some(command.map_id);
            state.go_type = Some(command.go_type);
            state.loot_state = loot_state;
            state.loot_state_unit_guid = command.loot_state_unit_guid;
            state.go_state = go_state;
            state.dynamic_flags = command.dynamic_flags;
            state.gathering_node_loot_id = command.gathering_node_loot_id;
            state.personal_loot_uses = command.personal_loot_uses;
            state.linked_trap_entry = command.linked_trap_entry;
            state.linked_trap_guid = command.linked_trap_guid;
        }

        let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
    }

    /// Mirrors the small chest state subset that C++ keeps on the shared
    /// GameObject before asking this session to recompute visible GameObject
    /// dynamic-flag deltas.
    pub(crate) fn handle_sync_chest_gameobject_state_and_refresh_like_cpp(
        &mut self,
        command: SyncChestGameobjectStateAndRefreshLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        if command.map_id != self.player_map_id_like_cpp() {
            return;
        }
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if command.instance_id != current_instance_id {
            return;
        }
        if u32::from(command.go_type) != GAMEOBJECT_TYPE_CHEST {
            return;
        }
        let loot_state = match command.loot_state {
            Some(0) => Some(LootState::NotReady),
            Some(1) => Some(LootState::Ready),
            Some(2) => Some(LootState::Activated),
            Some(3) => Some(LootState::JustDeactivated),
            Some(_) => return,
            None => None,
        };

        {
            let state = self
                .represented_gameobject_use_states
                .entry(command.gameobject_guid)
                .or_default();
            state.map_id = Some(command.map_id);
            state.go_type = Some(command.go_type);
            state.loot_state = loot_state;
            state.loot_state_unit_guid = command.loot_state_unit_guid;
            state.chest_loot_source = Some(GameObjectLootSource {
                loot_id: command.chest_loot_id,
                use_group_loot_rules: false,
                dungeon_encounter_id: 0,
                personal_loot_id: command.chest_personal_loot_id,
                push_loot_id: command.chest_push_loot_id,
                triggered_event_id: 0,
                linked_trap_entry: command.linked_trap_entry.unwrap_or_default(),
                chest_restock_time_secs: command.chest_restock_time_secs,
                chest_consumable: command.chest_consumable,
                chest_quest_id: command.chest_quest_id,
            });
            state.chest_restock_time_secs = Some(command.chest_restock_time_secs);
            state.chest_consumable = Some(command.chest_consumable);
            state.chest_personal_loot_id = Some(command.chest_personal_loot_id);
            state.linked_trap_entry = command.linked_trap_entry;
            state.linked_trap_guid = command.linked_trap_guid;
        }

        let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
    }

    /// Mirrors the small shared goober state subset that C++ keeps on the
    /// shared GameObject before asking this session to recompute visible
    /// GameObject dynamic-flag deltas. This intentionally does not import the
    /// cooldown/source ownership fields; the map-owned close/despawn path is a
    /// later runtime slice.
    pub(crate) fn handle_sync_goober_gameobject_state_and_refresh_like_cpp(
        &mut self,
        command: SyncGooberGameobjectStateAndRefreshLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        if command.map_id != self.player_map_id_like_cpp() {
            return;
        }
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if command.instance_id != current_instance_id {
            return;
        }
        if u32::from(command.go_type) != GAMEOBJECT_TYPE_GOOBER {
            return;
        }
        let loot_state = match command.loot_state {
            Some(0) => Some(LootState::NotReady),
            Some(1) => Some(LootState::Ready),
            Some(2) => Some(LootState::Activated),
            Some(3) => Some(LootState::JustDeactivated),
            Some(_) => return,
            None => None,
        };
        let go_state = match command.go_state {
            Some(0) => Some(GoState::Active),
            Some(1) => Some(GoState::Ready),
            Some(2) => Some(GoState::Destroyed),
            Some(24) => Some(GoState::TransportActive),
            Some(25) => Some(GoState::TransportStopped),
            Some(_) => return,
            None => None,
        };

        {
            let state = self
                .represented_gameobject_use_states
                .entry(command.gameobject_guid)
                .or_default();
            state.map_id = Some(command.map_id);
            state.go_type = Some(command.go_type);
            state.gameobject_flags = command.gameobject_flags;
            state.loot_state = loot_state;
            state.loot_state_unit_guid = command.loot_state_unit_guid;
            state.go_state = go_state;
            state.dynamic_flags = command.dynamic_flags;
            state.linked_trap_entry = command.linked_trap_entry;
            state.linked_trap_guid = command.linked_trap_guid;
        }

        let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
    }

    /// Apply a transitional map-owned creature melee compatibility hit to this
    /// player session.
    ///
    /// C++ contrast: `Creature::Update` calls `DoMeleeAttackIfReady()`, which
    /// eventually emits `AttackerStateUpdate` from the map update tick and
    /// then applies damage to the victim. This driver preserves the earlier
    /// normal-hit bridge; it does not claim full `CalculateMeleeDamage` parity.
    /// It owns the swing timer/damage/canonical health mutation once, and this
    /// command is only the victim-session delivery rail. Delivery rereads the
    /// current canonical health/death tuple and advances a presentation-only
    /// revision, so neither retries nor a delayed command can write an older
    /// value over a newer heal, hit, death, or resurrection.
    pub(crate) fn handle_apply_creature_melee_damage_like_cpp_command_like_cpp(
        &mut self,
        command: ApplyCreatureMeleeDamageLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        if self.player_guid() != Some(command.victim_guid) {
            return;
        }
        if self.player_map_id_like_cpp() != command.map_id {
            return;
        }
        let session_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if session_instance_id != command.instance_id {
            return;
        }
        let Some(canonical_health) = self.present_committed_creature_melee_health_like_cpp(
            command.victim_health_state_revision_after,
        ) else {
            return;
        };

        use wow_packet::packets::combat::{
            AttackerStateUpdate, HIT_INFO_NORMAL_SWING, HealthUpdate, VICTIM_STATE_HIT,
        };
        // Visibility can change after the map-owned swing commits. It gates
        // only the attacker-facing combat packet, never authoritative victim
        // health/death reconciliation.
        if self
            .client_visible_guids_like_cpp
            .contains(&command.attacker_guid)
        {
            self.send_packet(&AttackerStateUpdate {
                attacker: command.attacker_guid,
                victim: command.victim_guid,
                hit_info: HIT_INFO_NORMAL_SWING,
                damage: command.damage.min(i32::MAX as u32) as i32,
                over_damage: command.over_damage,
                victim_state: VICTIM_STATE_HIT,
                school_mask: 1,
                target_level: command.target_level,
                expansion: 2,
            });
        }
        self.send_packet(&HealthUpdate {
            guid: command.victim_guid,
            health: canonical_health.min(i64::MAX as u64) as i64,
        });
    }

    /// Mirror one map-owned creature aggro transition into this victim session.
    ///
    /// C++ contrast: `CreatureAI::MoveInLineOfSight` calls
    /// `Creature::CanStartAttack` and then engages the target; the combat start
    /// is visible to the client through `Unit::SendMeleeAttackStart`. The map
    /// runtime owns the aggro decision; this handler only gates the victim
    /// session and sends one `AttackStart` packet.
    pub(crate) fn handle_creature_attack_start_like_cpp_command_like_cpp(
        &mut self,
        command: CreatureAttackStartLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        if self.player_guid() != Some(command.victim_guid) {
            return;
        }
        if self.resolved_player_is_alive_like_cpp() != Some(true) {
            return;
        }
        if self.player_map_id_like_cpp() != command.map_id {
            return;
        }
        let session_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if session_instance_id != command.instance_id {
            return;
        }
        let attacker_is_visible = self
            .client_visible_guids_like_cpp
            .contains(&command.attacker_guid);

        if let Some(manager) = self.canonical_map_manager.as_ref().cloned()
            && let Ok(mut manager) = manager.lock()
            && let Some(managed) =
                manager.find_map_mut(u32::from(command.map_id), command.instance_id)
        {
            let map = managed.map_mut();
            if let Some(previous_victim) = command.previous_victim_guid {
                if let Some(player) = map.get_typed_player_mut(previous_victim) {
                    player
                        .unit_mut()
                        .remove_attacker_like_cpp(command.attacker_guid);
                } else if let Some(creature) = map.get_typed_creature_mut(previous_victim) {
                    creature
                        .unit_mut()
                        .remove_attacker_like_cpp(command.attacker_guid);
                }
            }
            if let Some(player) = map.get_typed_player_mut(command.victim_guid) {
                player
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .set_in_combat_with(command.attacker_guid, false, false);
                player
                    .unit_mut()
                    .add_attacker_like_cpp(command.attacker_guid);
            }
            if let Some(creature) = map.get_typed_creature_mut(command.attacker_guid) {
                let combat = &mut creature.unit_mut().subsystems_mut().combat;
                combat.set_in_combat_with(command.victim_guid, false, false);
                if combat.threat_ref(command.victim_guid).is_none() {
                    combat.set_threat(command.victim_guid, 0.0);
                }
                let threat_ref = combat.threat_ref(command.victim_guid).copied();
                if let Some(threat_ref) = threat_ref
                    && let Some(player) = map.get_typed_player_mut(command.victim_guid)
                {
                    player
                        .unit_mut()
                        .subsystems_mut()
                        .combat
                        .put_threatened_by_me_ref(command.attacker_guid, threat_ref);
                }
            }
        }

        // Incoming attackers do not become the player's own melee target.
        // C++ keeps that direction solely in `m_attackers`/combat references.
        self.set_in_combat_like_cpp(true);

        if attacker_is_visible && !command.packet_already_broadcast {
            use wow_packet::packets::combat::AttackStart;
            self.send_packet(&AttackStart {
                attacker: command.attacker_guid,
                victim: command.victim_guid,
            });
        }
    }

    pub(crate) fn handle_creature_attack_stop_like_cpp_command_like_cpp(
        &mut self,
        command: CreatureAttackStopLikeCppCommand,
    ) {
        // This cleanup command is emitted only by the full
        // `LegacyCreatureThreatUpdateLikeCpp::Evade` path. Ordinary victim
        // switches fan out `SMSG_ATTACKSTOP` directly but deliberately do not
        // enqueue this command, matching C++ `Unit::AttackStop()` preserving
        // threat and combat references.
        if self.state() != crate::session::SessionState::LoggedIn
            || self.player_guid() != Some(command.victim_guid)
            || self.player_map_id_like_cpp() != command.map_id
        {
            return;
        }
        let Some(map_key) = self.current_canonical_player_map_key_like_cpp() else {
            return;
        };
        if map_key.instance_id != command.instance_id {
            return;
        }

        let Some(manager) = self.canonical_map_manager.as_ref().cloned() else {
            return;
        };
        let Ok(mut manager) = manager.lock() else {
            return;
        };
        let Some(managed) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return;
        };
        let map = managed.map_mut();
        let still_in_combat = if let Some(player) = map.get_typed_player_mut(command.victim_guid) {
            player
                .unit_mut()
                .subsystems_mut()
                .combat
                .purge_combat_ref_like_cpp(command.attacker_guid);
            player
                .unit_mut()
                .subsystems_mut()
                .combat
                .purge_threatened_by_me_ref(command.attacker_guid);
            player
                .unit_mut()
                .remove_attacker_like_cpp(command.attacker_guid);
            player.unit().subsystems().combat.has_combat()
        } else {
            false
        };
        if let Some(creature) = map.get_typed_creature_mut(command.attacker_guid) {
            creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .purge_combat_ref_like_cpp(command.victim_guid);
            creature
                .unit_mut()
                .remove_attacker_like_cpp(command.victim_guid);
        }
        if self.resolved_combat_target_like_cpp().flatten() == Some(command.attacker_guid) {
            self.set_combat_target_like_cpp(None);
        }
        self.set_in_combat_like_cpp(still_in_combat);
    }

    pub(crate) fn handle_reconcile_pvp_combat_expiry_like_cpp(
        &mut self,
        command: ReconcilePvpCombatExpiryLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn
            || self.player_guid() != Some(command.player_guid)
            || self.player_map_id_like_cpp() != command.map_id
        {
            return;
        }
        let Some(map_key) = self.current_canonical_player_map_key_like_cpp() else {
            return;
        };
        if map_key.instance_id != command.instance_id {
            return;
        }
        let still_in_combat = self
            .canonical_map_manager
            .as_ref()
            .and_then(|manager| manager.lock().ok())
            .and_then(|manager| {
                manager
                    .find_map(map_key.map_id, map_key.instance_id)
                    .and_then(|managed| managed.map().get_typed_player(command.player_guid))
                    .map(|player| player.unit().subsystems().combat.has_combat())
            })
            .unwrap_or(false);
        self.set_in_combat_like_cpp(still_in_combat);
    }

    pub(crate) fn handle_send_visible_object_values_update_command_like_cpp(
        &mut self,
        command: crate::session::mailbox::SendVisibleObjectValuesUpdateCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        if self.player_map_id_like_cpp() != command.map_id {
            return;
        }
        if !self
            .client_visible_guids_like_cpp
            .contains(&command.object_guid)
        {
            return;
        }

        if let Some(unit_values_update) = command.unit_values_update {
            let update = self.represented_unit_packet_update_to_update_object_like_cpp(
                command.object_guid,
                command.map_id,
                unit_values_update,
            );
            self.send_packet(&update);
        } else {
            self.send_raw_packet(&command.packet_bytes);
        }
    }

    pub(crate) fn handle_send_if_visible_like_cpp_command_like_cpp(
        &mut self,
        command: SendIfVisibleLikeCppCommand,
        realm_connection: bool,
        allow_legacy_creature_source: bool,
    ) {
        if !self.send_if_visible_like_cpp_gate_passes_like_cpp(
            command.queued_at,
            command.source_guid,
            command.map_id,
            command.instance_id,
            &command.packet_bytes,
            allow_legacy_creature_source,
        ) {
            return;
        }
        // All gates passed — deliver the already-serialised packet as-is.
        if command
            .packet_bytes
            .get(0..2)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u16::from_le_bytes)
            == Some(wow_constants::ServerOpcodes::OnMonsterMove as u16)
        {
            tracing::info!(
                account = self.account_id,
                source_guid = ?command.source_guid,
                "RUST_MONSTER_MOVE_DELIVERY sent"
            );
        }
        if realm_connection {
            self.send_raw_packet_realm(&command.packet_bytes);
        } else {
            self.send_raw_packet(&command.packet_bytes);
        }
    }

    /// Deliver one map-owned creature START+GO pair after one visibility gate.
    ///
    /// C++ `WorldObject::SendCombatLogMessage` selects the committed full GO
    /// frame for advanced-combat-log viewers and the basic frame otherwise.
    /// Both viewers receive START and their selected GO consecutively with no
    /// command drain or visibility revalidation between them. The two
    /// frame-oriented socket sends are not transactional against other cloned
    /// producers or a receiver closing after START; absolute writer adjacency
    /// needs a future batch-aware socket envelope.
    pub(crate) fn handle_send_creature_spell_cast_if_visible_like_cpp_command_like_cpp(
        &mut self,
        command: SendCreatureSpellCastIfVisibleLikeCppCommand,
    ) {
        let opcode = |packet_bytes: &[u8]| {
            packet_bytes
                .get(0..2)
                .and_then(|bytes| bytes.try_into().ok())
                .map(u16::from_le_bytes)
        };
        if opcode(&command.start_packet_bytes)
            != Some(wow_constants::ServerOpcodes::SpellStart as u16)
            || opcode(&command.go_packet_bytes)
                != Some(wow_constants::ServerOpcodes::SpellGo as u16)
        {
            return;
        }
        // Recipient selection already happened where C++ performs it: inside the
        // synchronous `SendSpellGo` fan-out, against this session's
        // `HaveAtClient` set. Re-deriving it here from the drain-time set would
        // drop a correctly committed pair after a visibility exit and deliver a
        // stale cast to a viewer that only became visible afterwards. Validate
        // that the command belongs to this session incarnation and that the
        // session is still on the map it was committed for, then honor it.
        if !self
            .client_visible_guids_like_cpp
            .shares_storage_like_cpp(&command.committed_visibility_like_cpp)
        {
            return;
        }
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        if self.player_map_id_like_cpp() != command.map_id {
            return;
        }
        let session_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if session_instance_id != command.instance_id {
            return;
        }

        // The basic/full combat-log representation was already chosen for this
        // recipient when the cast resolved, so a preference the client toggled
        // since then cannot retroactively change an earlier cast's frame.
        self.send_raw_packet(&command.start_packet_bytes);
        self.send_raw_packet(&command.go_packet_bytes);
    }

    /// Per-session gate for addon chat delivery.
    ///
    /// Mirrors C++ `WorldSession::IsAddonRegistered(prefix)`: when
    /// `_filterAddonMessages` is false, all prefixes are accepted; otherwise
    /// the prefix must be in the session-local registered list.
    pub(crate) fn handle_send_addon_if_registered_like_cpp_command_like_cpp(
        &mut self,
        command: SendAddonIfRegisteredLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        if self.is_addon_registered_like_cpp(&command.prefix) {
            self.send_raw_packet(&command.packet_bytes);
        }
    }

    pub(crate) fn handle_cancel_represented_trade_command_like_cpp(
        &mut self,
        command: CancelRepresentedTradeLikeCppCommand,
    ) {
        if !matches!(
            self.resolved_represented_active_trade_partner_like_cpp(),
            Some(Some(_))
        ) {
            return;
        }

        self.record_represented_trade_cancel_like_cpp(command.status);
        if !self.clear_represented_active_trade_partner_like_cpp() {
            return;
        }
        self.send_raw_packet(&command.packet_bytes);
    }

    pub(crate) fn handle_send_represented_trade_status_command_like_cpp(
        &mut self,
        command: SendRepresentedTradeStatusLikeCppCommand,
    ) {
        if !matches!(
            self.resolved_represented_active_trade_partner_like_cpp(),
            Some(Some(_))
        ) {
            return;
        }

        self.send_raw_packet(&command.packet_bytes);
    }

    pub(crate) fn handle_unaccept_represented_trade_command_like_cpp(
        &mut self,
        command: UnacceptRepresentedTradeLikeCppCommand,
    ) {
        if !matches!(
            self.resolved_represented_active_trade_partner_like_cpp(),
            Some(Some(_))
        ) {
            return;
        }

        if !self.set_represented_trade_accepted_like_cpp_for_command(false) {
            return;
        }
        self.send_raw_packet(&command.packet_bytes);
    }

    pub(crate) fn handle_send_represented_duel_countdown_command_like_cpp(
        &mut self,
        command: SendRepresentedDuelCountdownLikeCppCommand,
    ) {
        self.send_raw_packet(&command.packet_bytes);
    }

    pub(crate) fn handle_send_represented_duel_requested_command_like_cpp(
        &mut self,
        command: SendRepresentedDuelRequestedLikeCppCommand,
    ) {
        self.set_represented_duel_arbiter_guid_like_cpp(Some(command.arbiter_guid));
        self.send_raw_packet(&command.packet_bytes);
    }

    /// Recompute this session's map-owned creature visibility.
    ///
    /// This is the session-local side of future global creature CREATE/DESTROY
    /// work. C++ performs creature create/out-of-range decisions in
    /// `Player::UpdateVisibilityOf`; this command reuses Rust's represented
    /// `update_visibility` pass instead of sending raw bytes that cannot update
    /// `client_visible_guids_like_cpp`.
    pub(crate) async fn handle_refresh_visible_world_creatures_like_cpp_command_like_cpp(
        &mut self,
        command: RefreshVisibleWorldCreaturesLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        if self.player_map_id_like_cpp() != command.map_id {
            return;
        }
        let session_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|k| k.instance_id)
            .unwrap_or(0);
        if session_instance_id != command.instance_id {
            return;
        }
        self.clear_pending_visibility_refresh_like_cpp();
        self.force_update_visibility_like_cpp().await;
    }

    pub(crate) fn handle_send_repeatable_turn_in_request_items_command_like_cpp(
        &mut self,
        command: SendRepeatableTurnInRequestItemsLikeCppCommand,
    ) {
        self.send_repeatable_turn_in_request_items_like_cpp(command.sender_guid, &command.quest);
    }

    pub(crate) fn handle_set_quest_sharing_info_and_send_details_command_like_cpp(
        &mut self,
        command: SetQuestSharingInfoAndSendDetailsCommand,
    ) {
        let Some(receiver_guid) = self.player_guid() else {
            return;
        };

        self.set_represented_pending_quest_sharing_like_cpp(command.sender_guid, command.quest.id);
        self.send_represented_quest_giver_quest_details_like_cpp(
            receiver_guid,
            &command.quest,
            false,
        );
    }

    pub(crate) async fn handle_represented_loot_roll_vote_command_like_cpp(
        &mut self,
        command: LootRollVoteCommand,
    ) {
        let roll_key = (command.loot_obj, command.loot_list_id);
        let Some(current_roll) = self.represented_loot_rolls.get(&roll_key) else {
            return;
        };
        if !Self::represented_loot_roll_vote_command_targets_identity_like_cpp(
            &command,
            &current_roll.command_identity,
        ) {
            return;
        }

        let roll = LootRoll {
            loot_obj: command.loot_obj,
            loot_list_id: command.loot_list_id,
            roll_type: command.roll_type,
        };

        let _ = self
            .represented_player_vote_on_loot_roll_with_pass_state_like_cpp(
                &roll,
                command.voter_guid,
                command.pass_on_group_loot,
            )
            .await;
    }

    pub(crate) async fn handle_apply_loot_money_like_cpp_command(
        &mut self,
        command: ApplyLootMoneyLikeCppCommand,
    ) {
        if self.player_guid() != Some(command.recipient) {
            return;
        }
        let apply_money = !command.applied.swap(true, Ordering::SeqCst);
        let publish = !command.published.swap(true, Ordering::SeqCst);
        if !apply_money && !publish {
            return;
        }

        if publish
            && command.send_coin_removed.load(Ordering::Acquire)
            && command.authority_committed.load(Ordering::Acquire)
            && self.represented_loot_money_command_targets_active_generation_like_cpp(
                command.loot_owner,
                &command.authority,
                command.authority_generation,
            )
        {
            self.send_packet(&CoinRemoved {
                loot_obj: command.loot_obj,
            });
            self.refresh_owned_loot_summary_like_cpp(command.loot_owner);
            if let Some(player_guid) = self.player_guid() {
                let _ =
                    self.reconcile_represented_loot_cache_like_cpp(command.loot_owner, player_guid);
            }
        }
        let durable_applied_amount = command.durable_applied_amount.load(Ordering::Acquire);
        let _ = self
            .apply_durable_represented_loot_money_payout_like_cpp(
                command.amount,
                durable_applied_amount,
                command.sole_looter,
                apply_money,
                publish,
            )
            .await;
    }

    pub(crate) fn handle_notify_loot_money_removed_like_cpp_command(
        &mut self,
        command: NotifyLootMoneyRemovedLikeCppCommand,
    ) {
        if self.player_guid() != Some(command.recipient)
            || !command.authority_committed.load(Ordering::Acquire)
            || !self.represented_loot_money_command_targets_active_generation_like_cpp(
                command.loot_owner,
                &command.authority,
                command.authority_generation,
            )
        {
            return;
        }

        self.send_packet(&CoinRemoved {
            loot_obj: command.loot_obj,
        });
        self.refresh_owned_loot_summary_like_cpp(command.loot_owner);
        if let Some(player_guid) = self.player_guid() {
            let _ = self.reconcile_represented_loot_cache_like_cpp(command.loot_owner, player_guid);
        }
    }

    pub(crate) async fn handle_represented_master_loot_give_command_like_cpp(
        &mut self,
        command: MasterLootGiveCommand,
    ) {
        let Some(player_guid) = self.player_guid() else {
            let _ = command.result_tx.send(MasterLootGiveResult::TargetMismatch);
            return;
        };

        if command.entry.allowed_looters.is_empty()
            || !command.entry.allowed_looters.contains(&player_guid)
        {
            let _ = command.result_tx.send(MasterLootGiveResult::StoreFailed(
                LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
            ));
            return;
        }

        if let Some(error) = self.represented_master_loot_can_store_error_like_cpp(
            player_guid,
            command.entry.item_id,
            command.entry.quantity,
        ) {
            let _ = command
                .result_tx
                .send(MasterLootGiveResult::StoreFailed(error));
            return;
        }

        let stored = if let Some(claim) = command.claim.as_ref() {
            self.store_claimed_direct_loot_item_from_owner_like_cpp(
                &command.entry,
                command.dungeon_encounter_id,
                command.loot_owner,
                command.loot_obj,
                claim,
            )
            .await
        } else {
            self.store_direct_loot_item_from_owner_like_cpp(
                &command.entry,
                command.dungeon_encounter_id,
                command.loot_owner,
            )
            .await
        };
        let result = if stored {
            MasterLootGiveResult::Stored
        } else {
            MasterLootGiveResult::StoreFailed(LOOT_ERROR_MASTER_OTHER_LIKE_CPP)
        };

        debug!(
            account = self.account_id,
            master = ?command.master_guid,
            owner = ?command.loot_owner,
            loot_obj = ?command.loot_obj,
            loot_list_id = command.loot_list_id,
            ?result,
            "processed represented remote master-loot give command"
        );

        let _ = command.result_tx.send(result);
    }

    pub(crate) async fn handle_represented_loot_roll_store_winner_command_like_cpp(
        &mut self,
        command: LootRollStoreWinnerCommand,
    ) {
        let Some(player_guid) = self.player_guid() else {
            let _ = command.result_tx.send(MasterLootGiveResult::TargetMismatch);
            return;
        };

        if command.entries.is_empty()
            || (!command.is_disenchant && command.entries.len() != 1)
            || command.entries.iter().any(|entry| {
                entry.allowed_looters.is_empty()
                    || !entry.allowed_looters.contains(&player_guid)
                    || !entry.roll_winner_allows_like_cpp(player_guid)
            })
        {
            let _ = command.result_tx.send(MasterLootGiveResult::StoreFailed(
                LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
            ));
            return;
        }

        if let Some(error) = command.entries.iter().find_map(|entry| {
            self.represented_master_loot_can_store_error_like_cpp(
                player_guid,
                entry.item_id,
                entry.quantity,
            )
        }) {
            let _ = command
                .result_tx
                .send(MasterLootGiveResult::StoreFailed(error));
            return;
        }

        let stored = if command.is_disenchant {
            self.store_direct_disenchant_batch_like_cpp(
                &command.entries,
                command.dungeon_encounter_id,
                command.claim.as_ref(),
                command
                    .claim
                    .as_ref()
                    .map(|claim| LootItemClaimCommitContextLikeCpp {
                        owner_guid: command.loot_owner,
                        loot_obj: command.loot_obj,
                        loot_list_id: command.loot_list_id,
                        player_guid,
                        free_for_all: match claim.payload_like_cpp() {
                            LootClaimPayload::Item(entry) => entry.flags.freeforall,
                            LootClaimPayload::Money(_) => false,
                        },
                    }),
            )
            .await
        } else if let Some(claim) = command.claim.as_ref() {
            self.store_claimed_direct_loot_item_from_owner_like_cpp(
                &command.entries[0],
                command.dungeon_encounter_id,
                command.loot_owner,
                command.loot_obj,
                claim,
            )
            .await
        } else {
            self.store_direct_loot_item_from_owner_like_cpp(
                &command.entries[0],
                command.dungeon_encounter_id,
                command.loot_owner,
            )
            .await
        };
        let result = if stored {
            MasterLootGiveResult::Stored
        } else {
            MasterLootGiveResult::StoreFailed(LOOT_ERROR_MASTER_OTHER_LIKE_CPP)
        };

        debug!(
            account = self.account_id,
            owner = ?command.loot_owner,
            loot_obj = ?command.loot_obj,
            loot_list_id = command.loot_list_id,
            ?result,
            "processed represented remote loot-roll winner store command"
        );

        let _ = command.result_tx.send(result);
    }

    /// CMSG_SET_LOOT_SPECIALIZATION — select or clear the loot specialization.
    ///
    /// C++ accepts non-zero values only when `sChrSpecializationStore` has the
    /// row and its `ClassID` matches the player's class; `SpecID == 0` clears.
    pub async fn handle_set_loot_specialization(&mut self, packet: SetLootSpecialization) {
        if self.player_guid().is_none() {
            return;
        }

        if packet.spec_id == 0 {
            self.set_loot_specialization_id_like_cpp(0);
            return;
        }

        let Some(store) = self.chr_specialization_store() else {
            return;
        };
        let Some(spec) = store.get(packet.spec_id) else {
            return;
        };
        if spec.class_id != self.player_class_like_cpp() {
            return;
        }

        self.set_loot_specialization_id_like_cpp(packet.spec_id);
    }
}
