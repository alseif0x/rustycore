// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Group loot rolls: need/greed/disenchant voting and winner selection.

use super::*;

impl WorldSession {
    pub(super) fn route_represented_remote_loot_roll_vote_to_owner_like_cpp(
        &self,
        roll: &LootRoll,
        player_guid: ObjectGuid,
    ) -> bool {
        let Some(registry) = self.player_registry() else {
            return false;
        };
        let Some(pass_on_group_loot) = self.resolved_pass_on_group_loot_like_cpp() else {
            return false;
        };

        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let Some((registration, roll_identity)) = registry.loot_roll_owner(
            player_guid,
            self.player_map_id_like_cpp(),
            instance_id,
            roll.loot_obj,
            roll.loot_list_id,
        ) else {
            return false;
        };

        registry
            .try_send_current_command(
                registration,
                SessionCommand::LootRollVote(LootRollVoteCommand {
                    voter_guid: player_guid,
                    loot_obj: roll.loot_obj,
                    loot_list_id: roll.loot_list_id,
                    roll_type: roll.roll_type,
                    pass_on_group_loot,
                    roll_identity,
                }),
            )
            .is_ok()
    }

    pub(super) async fn represented_player_vote_on_loot_roll_like_cpp(
        &mut self,
        roll: &LootRoll,
        player_guid: ObjectGuid,
    ) -> bool {
        let Some(pass_on_group_loot) = self.resolved_pass_on_group_loot_like_cpp() else {
            return false;
        };
        self.represented_player_vote_on_loot_roll_with_pass_state_like_cpp(
            roll,
            player_guid,
            pass_on_group_loot,
        )
        .await
    }

    pub(super) async fn represented_player_vote_on_loot_roll_with_pass_state_like_cpp(
        &mut self,
        roll: &LootRoll,
        player_guid: ObjectGuid,
        pass_on_group_loot: bool,
    ) -> bool {
        let roll_key = (roll.loot_obj, roll.loot_list_id);
        let Some(roll_state) = self.represented_loot_rolls.get(&roll_key).cloned() else {
            return false;
        };
        if self
            .represented_current_loot_roll_authority_like_cpp(&roll_state)
            .is_none()
        {
            self.cancel_represented_loot_roll_generation_mismatch_like_cpp(roll_key, &roll_state);
            return true;
        }

        if pass_on_group_loot {
            return false;
        }

        let owner_guid = roll_state.owner_guid;

        let Some(loot) = self.loot_table.get(&owner_guid) else {
            return false;
        };
        if !matches!(
            loot.loot_method,
            LOOT_METHOD_GROUP_LIKE_CPP | LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP
        ) {
            return false;
        }
        let loot_guid = loot.loot_guid;
        let dungeon_encounter_id = loot.dungeon_encounter_id as i32;

        let Some(entry) = loot.items.iter().find(|entry| {
            entry.loot_list_id == roll.loot_list_id
                && entry.flags.blocked
                && entry.has_allowed_looter_like_cpp(player_guid)
        }) else {
            return false;
        };
        let entry = entry.clone();

        let (roll_number, stored_roll_number) = match roll.roll_type {
            ROLL_VOTE_PASS_LIKE_CPP => (-1, None),
            ROLL_VOTE_NEED_LIKE_CPP => (0, Some(self.represented_urand_u32_like_cpp(1, 100) as u8)),
            ROLL_VOTE_GREED_LIKE_CPP | ROLL_VOTE_DISENCHANT_LIKE_CPP => {
                (-1, Some(self.represented_urand_u32_like_cpp(1, 100) as u8))
            }
            _ => return false,
        };

        let Some(state) = self
            .represented_loot_rolls
            .get_mut(&(loot_guid, roll.loot_list_id))
        else {
            return false;
        };
        let Some(voter) = state.voters.get_mut(&player_guid) else {
            return false;
        };
        voter.vote = roll.roll_type;
        if let Some(stored_roll_number) = stored_roll_number {
            voter.roll_number = stored_roll_number;
        }

        let packet = LootRollBroadcast {
            loot_obj: loot_guid,
            player: player_guid,
            roll: roll_number,
            roll_type: roll.roll_type,
            item: loot_roll_broadcast_item_like_cpp(&entry, LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP),
            autopassed: false,
            off_spec: false,
            dungeon_encounter_id,
        };

        let finish = represented_loot_roll_finish_winner_like_cpp(state);
        let finished_state = finish.as_ref().map(|_| state.clone());
        self.update_represented_loot_roll_vote_criteria_like_cpp(player_guid, roll.roll_type);
        self.broadcast_represented_loot_roll_packet_like_cpp(&packet, &entry, None);
        if let Some(winner) = finish {
            self.finish_represented_loot_roll_like_cpp(
                loot_guid,
                roll.loot_list_id,
                &entry,
                winner,
                finished_state.as_ref(),
            )
            .await;
        }
        true
    }

    /// A represented roll is scoped to one lifetime of the object-owned Loot.
    ///
    /// C++ destroys `LootRoll` together with its owning `Loot`. Rust keeps the
    /// packet-facing roll state in the session, so a recycled object GUID must
    /// not let that stale state unblock or award an item from a later lifetime.
    fn represented_current_loot_roll_authority_like_cpp(
        &mut self,
        state: &RepresentedLootRollState,
    ) -> Option<OwnedLootAuthority> {
        let Some(authority) = self.represented_owned_loot_authority_like_cpp(state.owner_guid)
        else {
            return None;
        };

        if !authority.shares_storage_like_cpp(&state.authority) {
            return None;
        }
        let player_guid = match state.authority_scope {
            wow_loot::OwnedLootScope::Shared => self.player_guid()?,
            wow_loot::OwnedLootScope::Personal(player_guid) => player_guid,
        };
        authority
            .snapshot_for_player_like_cpp(player_guid)
            .is_some_and(|snapshot| {
                snapshot.scope == state.authority_scope
                    && snapshot.generation == state.authority_generation
                    && snapshot.loot.loot_guid == state.loot_obj
            })
            .then_some(authority)
    }

    fn cancel_represented_loot_roll_generation_mismatch_like_cpp(
        &mut self,
        key: (ObjectGuid, u8),
        state: &RepresentedLootRollState,
    ) {
        debug!(
            owner = ?state.owner_guid,
            loot_obj = ?state.loot_obj,
            loot_list_id = state.loot_list_id,
            authority_generation = state.authority_generation,
            "represented loot roll cancelled after owner loot generation changed"
        );
        self.represented_loot_rolls.remove(&key);
        self.publish_represented_loot_roll_ownership_like_cpp();
    }

    async fn finish_represented_loot_roll_like_cpp(
        &mut self,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        entry: &LootEntry,
        winner: Option<(ObjectGuid, RepresentedLootRollVote)>,
        finished_state: Option<&RepresentedLootRollState>,
    ) {
        let Some(state) = finished_state else {
            return;
        };
        let roll_key = (loot_obj, loot_list_id);
        if state.loot_obj != loot_obj || state.loot_list_id != loot_list_id {
            self.cancel_represented_loot_roll_generation_mismatch_like_cpp(roll_key, state);
            return;
        }
        let Some(authority) = self.represented_current_loot_roll_authority_like_cpp(state) else {
            self.cancel_represented_loot_roll_generation_mismatch_like_cpp(roll_key, state);
            return;
        };
        let owner_guid = state.owner_guid;
        let dungeon_encounter_id = self
            .loot_table
            .get(&owner_guid)
            .map(|loot| loot.dungeon_encounter_id as i32)
            .unwrap_or(0);

        let winner_guid = winner.as_ref().map(|(guid, _)| *guid);
        let scope_player = winner_guid
            .or_else(|| self.player_guid())
            .unwrap_or(ObjectGuid::EMPTY);
        let claim = if let Some(winner_guid) = winner_guid {
            match authority.finish_item_roll_and_reserve_award_like_cpp(
                scope_player,
                state.authority_generation,
                loot_list_id,
                winner_guid,
            ) {
                Ok(claim) => Some(claim),
                Err(_) => {
                    self.cancel_represented_loot_roll_generation_mismatch_like_cpp(roll_key, state);
                    return;
                }
            }
        } else {
            if authority
                .finish_item_roll_like_cpp(
                    scope_player,
                    state.authority_generation,
                    loot_list_id,
                    false,
                    None,
                )
                .is_err()
            {
                self.cancel_represented_loot_roll_generation_mismatch_like_cpp(roll_key, state);
                return;
            }
            None
        };
        let _ = self.reconcile_represented_loot_cache_like_cpp(owner_guid, scope_player);

        if let Some(loot) = self.loot_table.get_mut(&owner_guid) {
            if let Some(loot_entry) = loot
                .items
                .iter_mut()
                .find(|loot_entry| loot_entry.loot_list_id == loot_list_id)
            {
                loot_entry.flags.blocked = false;
                if let Some((winner_guid, _)) = winner {
                    loot_entry.roll_winner = winner_guid;
                }
            }
        }

        self.represented_loot_rolls
            .remove(&(loot_obj, loot_list_id));
        self.publish_represented_loot_roll_ownership_like_cpp();

        let Some((winner_guid, winner_vote)) = winner else {
            let packet = LootAllPassed {
                loot_obj,
                item: loot_roll_broadcast_item_like_cpp(entry, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP),
                dungeon_encounter_id,
            };
            if let Some(state) = finished_state {
                for (player_guid, vote) in &state.voters {
                    if vote.vote == ROLL_VOTE_NOT_VALID_LIKE_CPP {
                        self.send_represented_loot_roll_packet_to_player_like_cpp(
                            &packet,
                            *player_guid,
                        );
                    }
                }
            }
            return;
        };

        if let Some(state) = finished_state {
            self.send_represented_loot_roll_final_values_like_cpp(
                loot_obj,
                entry,
                winner_guid,
                state,
                dungeon_encounter_id,
            );
        }

        let locked = LootRollWon {
            loot_obj,
            winner: winner_guid,
            roll: i32::from(winner_vote.roll_number),
            roll_type: winner_vote.vote,
            item: loot_roll_broadcast_item_like_cpp(entry, LOOT_SLOT_TYPE_LOCKED_LIKE_CPP),
            main_spec: true,
            dungeon_encounter_id,
        };
        self.broadcast_represented_loot_roll_packet_like_cpp(&locked, entry, Some(winner_guid));

        let allow = LootRollWon {
            item: loot_roll_broadcast_item_like_cpp(entry, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP),
            ..locked
        };
        self.send_represented_loot_roll_packet_to_player_like_cpp(&allow, winner_guid);
        self.update_represented_loot_roll_winner_criteria_like_cpp(
            winner_guid,
            entry.item_id,
            winner_vote,
        );
        self.store_represented_loot_roll_winner_item_like_cpp(
            owner_guid,
            loot_obj,
            loot_list_id,
            entry,
            winner_guid,
            winner_vote,
            claim,
        )
        .await;
    }

    fn update_represented_loot_roll_vote_criteria_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        roll_type: u8,
    ) {
        match roll_type {
            ROLL_VOTE_NEED_LIKE_CPP => {
                self.record_represented_roll_any_need_criteria_like_cpp(player_guid, 1)
            }
            ROLL_VOTE_GREED_LIKE_CPP | ROLL_VOTE_DISENCHANT_LIKE_CPP => {
                self.record_represented_roll_any_greed_criteria_like_cpp(player_guid, 1)
            }
            _ => {}
        }
    }

    fn update_represented_loot_roll_winner_criteria_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        item_id: u32,
        winner_vote: RepresentedLootRollVote,
    ) {
        match winner_vote.vote {
            ROLL_VOTE_NEED_LIKE_CPP => self.record_represented_roll_need_criteria_like_cpp(
                player_guid,
                item_id,
                winner_vote.roll_number,
            ),
            ROLL_VOTE_DISENCHANT_LIKE_CPP => self.record_represented_disenchant_criteria_like_cpp(
                player_guid,
                DISENCHANT_LOOT_ROLL_CRITERIA_SPELL_LIKE_CPP,
            ),
            ROLL_VOTE_GREED_LIKE_CPP => self.record_represented_roll_greed_criteria_like_cpp(
                player_guid,
                item_id,
                winner_vote.roll_number,
            ),
            _ => {}
        }
    }

    fn record_represented_roll_any_need_criteria_like_cpp(
        &mut self,
        _player_guid: ObjectGuid,
        _quantity: u32,
    ) {
        #[cfg(test)]
        self.represented_loot_roll_criteria_events.push(
            crate::session::RepresentedLootRollCriteriaEvent::RollAnyNeed {
                player_guid: _player_guid,
                quantity: _quantity,
            },
        );
    }

    fn record_represented_roll_any_greed_criteria_like_cpp(
        &mut self,
        _player_guid: ObjectGuid,
        _quantity: u32,
    ) {
        #[cfg(test)]
        self.represented_loot_roll_criteria_events.push(
            crate::session::RepresentedLootRollCriteriaEvent::RollAnyGreed {
                player_guid: _player_guid,
                quantity: _quantity,
            },
        );
    }

    fn record_represented_roll_need_criteria_like_cpp(
        &mut self,
        _player_guid: ObjectGuid,
        _item_id: u32,
        _roll_number: u8,
    ) {
        #[cfg(test)]
        self.represented_loot_roll_criteria_events.push(
            crate::session::RepresentedLootRollCriteriaEvent::RollNeed {
                player_guid: _player_guid,
                item_id: _item_id,
                roll_number: _roll_number,
            },
        );
    }

    fn record_represented_roll_greed_criteria_like_cpp(
        &mut self,
        _player_guid: ObjectGuid,
        _item_id: u32,
        _roll_number: u8,
    ) {
        #[cfg(test)]
        self.represented_loot_roll_criteria_events.push(
            crate::session::RepresentedLootRollCriteriaEvent::RollGreed {
                player_guid: _player_guid,
                item_id: _item_id,
                roll_number: _roll_number,
            },
        );
    }

    fn send_represented_loot_roll_final_values_like_cpp(
        &self,
        loot_obj: ObjectGuid,
        entry: &LootEntry,
        winner_guid: ObjectGuid,
        state: &RepresentedLootRollState,
        dungeon_encounter_id: i32,
    ) {
        for (player_guid, vote) in &state.voters {
            let (roll, roll_type) = match vote.vote {
                ROLL_VOTE_PASS_LIKE_CPP => continue,
                ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP | ROLL_VOTE_NOT_VALID_LIKE_CPP => {
                    (0, ROLL_VOTE_PASS_LIKE_CPP)
                }
                ROLL_VOTE_NEED_LIKE_CPP
                | ROLL_VOTE_GREED_LIKE_CPP
                | ROLL_VOTE_DISENCHANT_LIKE_CPP => (i32::from(vote.roll_number), vote.vote),
                _ => continue,
            };

            let ongoing = LootRollBroadcast {
                loot_obj,
                player: *player_guid,
                roll,
                roll_type,
                item: loot_roll_broadcast_item_like_cpp(
                    entry,
                    LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP,
                ),
                autopassed: false,
                off_spec: false,
                dungeon_encounter_id,
            };

            self.broadcast_represented_loot_roll_packet_to_voters_like_cpp(
                &ongoing,
                state,
                Some(winner_guid),
            );

            let allow = LootRollBroadcast {
                item: loot_roll_broadcast_item_like_cpp(entry, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP),
                ..ongoing
            };
            self.send_represented_loot_roll_packet_to_player_like_cpp(&allow, winner_guid);
        }
    }

    fn send_represented_loot_roll_packet_to_player_like_cpp<P: ServerPacket>(
        &self,
        packet: &P,
        target: ObjectGuid,
    ) {
        if self.player_guid() == Some(target) {
            self.send_packet(packet);
            return;
        }

        let Some(registry) = self.player_registry() else {
            return;
        };
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let Some(registration) =
            registry.loot_delivery_recipient(target, self.player_map_id_like_cpp(), instance_id)
        else {
            return;
        };

        let _ = registry.send_current_packet(registration, packet.to_bytes());
    }

    fn broadcast_represented_loot_roll_packet_like_cpp<P: ServerPacket>(
        &self,
        packet: &P,
        entry: &LootEntry,
        except: Option<ObjectGuid>,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };

        let bytes = packet.to_bytes();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        for looter in &entry.allowed_looters {
            if Some(*looter) == except {
                continue;
            }

            if *looter == player_guid {
                self.send_packet(packet);
                continue;
            }

            let Some(registry) = self.player_registry() else {
                continue;
            };
            let Some(registration) = registry.loot_delivery_recipient(
                *looter,
                self.player_map_id_like_cpp(),
                instance_id,
            ) else {
                continue;
            };

            let _ = registry.send_current_packet(registration, bytes.clone());
        }
    }

    fn broadcast_represented_loot_roll_packet_to_voters_like_cpp<P: ServerPacket>(
        &self,
        packet: &P,
        state: &RepresentedLootRollState,
        except: Option<ObjectGuid>,
    ) {
        let bytes = packet.to_bytes();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        for (player_guid, vote) in &state.voters {
            if vote.vote == ROLL_VOTE_NOT_VALID_LIKE_CPP {
                continue;
            }
            if Some(*player_guid) == except {
                continue;
            }

            if self.player_guid() == Some(*player_guid) {
                self.send_packet(packet);
                continue;
            }

            let Some(registry) = self.player_registry() else {
                continue;
            };
            let Some(registration) = registry.loot_delivery_recipient(
                *player_guid,
                self.player_map_id_like_cpp(),
                instance_id,
            ) else {
                continue;
            };

            let _ = registry.send_current_packet(registration, bytes.clone());
        }
    }

    async fn store_represented_loot_roll_winner_item_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        entry: &LootEntry,
        winner_guid: ObjectGuid,
        winner_vote: RepresentedLootRollVote,
        claim: Option<LootClaimLease>,
    ) {
        let dungeon_encounter_id = self
            .loot_table
            .get(&owner_guid)
            .map(|loot| loot.dungeon_encounter_id)
            .unwrap_or(0);
        if winner_vote.vote == ROLL_VOTE_DISENCHANT_LIKE_CPP {
            let reserved_entry = claim
                .as_ref()
                .and_then(|claim| match claim.payload_like_cpp() {
                    LootClaimPayload::Item(entry) => Some(entry),
                    LootClaimPayload::Money(_) => None,
                })
                .unwrap_or(entry);
            if self
                .store_represented_disenchant_loot_winner_like_cpp(
                    owner_guid,
                    loot_obj,
                    loot_list_id,
                    reserved_entry,
                    winner_guid,
                    dungeon_encounter_id,
                    claim.as_ref(),
                )
                .await
            {
                if self.player_guid() == Some(winner_guid) {
                    if claim.is_none() {
                        self.mark_represented_master_loot_item_removed_like_cpp(
                            owner_guid,
                            loot_obj,
                            loot_list_id,
                            winner_guid,
                        );
                    }
                } else if claim.is_none() {
                    // Object-owned claims are committed and fanned out by the
                    // remote target session.  The legacy cache-only fallback
                    // still has to be retired by the source session.
                    self.mark_represented_master_loot_item_removed_like_cpp(
                        owner_guid,
                        loot_obj,
                        loot_list_id,
                        winner_guid,
                    );
                }
            }
            return;
        }

        if self.player_inventory_persistence_port_like_cpp().is_none() {
            return;
        }

        let mut store_entry = self
            .loot_table
            .get(&owner_guid)
            .and_then(|loot| {
                loot.items
                    .iter()
                    .find(|loot_entry| loot_entry.loot_list_id == loot_list_id)
                    .cloned()
            })
            .unwrap_or_else(|| entry.clone());
        if let Some(claim) = claim.as_ref()
            && let LootClaimPayload::Item(reserved_entry) = claim.payload_like_cpp()
        {
            store_entry = reserved_entry.clone();
        }
        store_entry.roll_winner = winner_guid;

        if self.player_guid() == Some(winner_guid) {
            let stored = if let Some(claim) = claim.as_ref() {
                self.store_claimed_direct_loot_item_from_owner_like_cpp(
                    &store_entry,
                    dungeon_encounter_id,
                    owner_guid,
                    loot_obj,
                    claim,
                )
                .await
            } else {
                self.store_direct_loot_item_from_owner_like_cpp(
                    &store_entry,
                    dungeon_encounter_id,
                    owner_guid,
                )
                .await
            };
            if stored {
                if claim.is_none() {
                    self.mark_represented_master_loot_item_removed_like_cpp(
                        owner_guid,
                        loot_obj,
                        loot_list_id,
                        winner_guid,
                    );
                }
            }
            return;
        }

        let authoritative_claim = claim.is_some();
        match self
            .request_represented_remote_loot_roll_winner_store_like_cpp(
                winner_guid,
                owner_guid,
                loot_obj,
                loot_list_id,
                dungeon_encounter_id,
                vec![store_entry],
                false,
                claim,
            )
            .await
        {
            MasterLootGiveResult::Stored if !authoritative_claim => {
                self.mark_represented_master_loot_item_removed_like_cpp(
                    owner_guid,
                    loot_obj,
                    loot_list_id,
                    winner_guid,
                );
            }
            MasterLootGiveResult::Stored => {}
            MasterLootGiveResult::StoreFailed(error) => {
                debug!(
                    account = self.account_id,
                    winner = ?winner_guid,
                    loot_obj = ?loot_obj,
                    loot_list_id,
                    error,
                    "represented loot-roll winner store failed in target session"
                );
            }
            MasterLootGiveResult::TargetMismatch => {
                debug!(
                    account = self.account_id,
                    winner = ?winner_guid,
                    loot_obj = ?loot_obj,
                    loot_list_id,
                    "represented loot-roll winner store target was not connected"
                );
            }
        }
    }

    pub(super) async fn request_represented_remote_loot_roll_winner_store_like_cpp(
        &self,
        target: ObjectGuid,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        dungeon_encounter_id: u32,
        entries: Vec<LootEntry>,
        is_disenchant: bool,
        claim: Option<LootClaimLease>,
    ) -> MasterLootGiveResult {
        let Some(registry) = self.player_registry() else {
            return MasterLootGiveResult::TargetMismatch;
        };
        let Some(command_address) = registry.control_address(target) else {
            return MasterLootGiveResult::TargetMismatch;
        };

        let (result_tx, result_rx) = flume::bounded(1);
        let command = SessionCommand::LootRollStoreWinner(LootRollStoreWinnerCommand {
            loot_owner: owner_guid,
            loot_obj,
            loot_list_id,
            dungeon_encounter_id,
            entries,
            is_disenchant,
            claim,
            result_tx,
        });

        if command_address.try_send(command).is_err() {
            return MasterLootGiveResult::TargetMismatch;
        }

        timeout(REMOTE_MASTER_LOOT_COMMAND_TIMEOUT, result_rx.recv_async())
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or(MasterLootGiveResult::TargetMismatch)
    }

    pub(super) fn represented_loot_roll_vote_command_targets_identity_like_cpp(
        command: &LootRollVoteCommand,
        current_identity: &LootRollCommandIdentityLikeCpp,
    ) -> bool {
        current_identity.matches_key_like_cpp(command.loot_obj, command.loot_list_id)
            && current_identity.is_exact_roll_like_cpp(&command.roll_identity)
    }

    pub(super) fn represented_start_group_loot_rolls_on_first_open_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        let Some(authority) = self.represented_owned_loot_authority_like_cpp(owner_guid) else {
            return;
        };
        let Some(authority_snapshot) = authority.snapshot_for_player_like_cpp(player_guid) else {
            return;
        };
        let authority_generation = authority_snapshot.generation;
        let authority_scope = authority_snapshot.scope;
        let current_map_id = self.player_map_id_like_cpp();
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let player_registry = self.player_registry().cloned();
        let mut packets = Vec::new();
        let mut auto_pass_packets = Vec::new();
        let mut pending_rolls = Vec::new();
        let mut unblocked_without_roll = Vec::new();
        let item_flags2_by_item_id: HashMap<u32, (Option<u32>, Option<u16>)> = self
            .loot_table
            .get(&owner_guid)
            .map(|loot| {
                loot.items
                    .iter()
                    .map(|entry| {
                        (
                            entry.item_id,
                            (
                                self.item_template_flags2(entry.item_id),
                                self.represented_loot_roll_disenchant_skill_required_like_cpp(
                                    entry.item_id,
                                ),
                            ),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        let current_player_enchanting_skill = self.resolved_enchanting_skill_like_cpp();
        let Some(pass_on_group_loot) = self.resolved_pass_on_group_loot_like_cpp() else {
            return;
        };

        if let Some(loot) = self.loot_table.get_mut(&owner_guid) {
            for entry in &mut loot.items {
                if !entry.flags.blocked {
                    continue;
                }

                let eligible_looters = connected_roll_looters_like_cpp(
                    entry,
                    player_guid,
                    current_map_id,
                    current_instance_id,
                    player_registry.as_deref(),
                );
                if eligible_looters.len() <= 1 {
                    entry.flags.under_threshold = true;
                    entry.flags.blocked = false;
                    unblocked_without_roll.push(entry.loot_list_id);
                    continue;
                }

                let mut voters = HashMap::new();
                for looter in &entry.allowed_looters {
                    let vote = if *looter == player_guid {
                        if pass_on_group_loot {
                            ROLL_VOTE_PASS_LIKE_CPP
                        } else {
                            ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP
                        }
                    } else {
                        match player_registry.as_deref().and_then(|registry| {
                            registry.loot_pass_on_group_loot(
                                *looter,
                                current_map_id,
                                current_instance_id,
                            )
                        }) {
                            Some(pass_on_group_loot) => {
                                if pass_on_group_loot {
                                    ROLL_VOTE_PASS_LIKE_CPP
                                } else {
                                    ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP
                                }
                            }
                            _ => ROLL_VOTE_NOT_VALID_LIKE_CPP,
                        }
                    };
                    voters.insert(
                        *looter,
                        RepresentedLootRollVote {
                            vote,
                            roll_number: 0,
                        },
                    );
                }
                let command_identity = LootRollCommandIdentityLikeCpp::new_like_cpp(
                    loot.loot_guid,
                    entry.loot_list_id,
                    authority.clone(),
                    authority_generation,
                );
                let state = RepresentedLootRollState {
                    owner_guid,
                    loot_obj: loot.loot_guid,
                    loot_list_id: entry.loot_list_id,
                    authority: authority.clone(),
                    authority_generation,
                    authority_scope,
                    command_identity,
                    end_time: Instant::now()
                        + Duration::from_millis(u64::from(LOOT_ROLL_TIMEOUT_MS_LIKE_CPP)),
                    voters,
                };
                let max_enchanting_skill = represented_max_enchanting_skill_like_cpp(
                    &eligible_looters,
                    player_guid,
                    current_player_enchanting_skill,
                    player_registry.as_deref(),
                );
                let (item_flags2, disenchant_skill_required) = item_flags2_by_item_id
                    .get(&entry.item_id)
                    .copied()
                    .unwrap_or((None, None));
                let valid_rolls = Self::represented_loot_roll_valid_rolls_like_cpp(
                    item_flags2,
                    disenchant_skill_required,
                    max_enchanting_skill,
                );

                for (looter, vote) in &state.voters {
                    if vote.vote != ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP {
                        continue;
                    }

                    packets.push((
                        *looter,
                        start_loot_roll_packet_like_cpp(
                            loot.loot_guid,
                            current_map_id,
                            loot.loot_method,
                            entry,
                            valid_rolls,
                            loot.dungeon_encounter_id as i32,
                        ),
                    ));
                }

                for (looter, vote) in &state.voters {
                    if vote.vote != ROLL_VOTE_PASS_LIKE_CPP {
                        continue;
                    }

                    auto_pass_packets.push((
                        LootRollBroadcast {
                            loot_obj: loot.loot_guid,
                            player: *looter,
                            roll: -1,
                            roll_type: ROLL_VOTE_PASS_LIKE_CPP,
                            item: loot_roll_broadcast_item_like_cpp(
                                entry,
                                LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP,
                            ),
                            autopassed: false,
                            off_spec: false,
                            dungeon_encounter_id: loot.dungeon_encounter_id as i32,
                        },
                        state.clone(),
                    ));
                }

                pending_rolls.push(state);
            }
        }

        if !unblocked_without_roll.is_empty()
            && let Some(authority) = self.represented_owned_loot_authority_like_cpp(owner_guid)
        {
            for loot_list_id in unblocked_without_roll {
                let _ = authority.finish_item_roll_like_cpp(
                    player_guid,
                    authority_generation,
                    loot_list_id,
                    true,
                    None,
                );
            }
            let _ = self.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid);
        }

        for roll in pending_rolls {
            self.represented_loot_rolls
                .insert((roll.loot_obj, roll.loot_list_id), roll);
        }
        self.publish_represented_loot_roll_ownership_like_cpp();

        for (looter, packet) in packets {
            if looter == player_guid {
                self.send_packet(&packet);
                continue;
            }

            let Some(registry) = self.player_registry() else {
                continue;
            };
            let Some(registration) = registry.loot_delivery_recipient(
                looter,
                self.player_map_id_like_cpp(),
                current_instance_id,
            ) else {
                continue;
            };

            let _ = registry.send_current_packet(registration, packet.to_bytes());
        }

        for (packet, state) in auto_pass_packets {
            self.broadcast_represented_loot_roll_packet_to_voters_like_cpp(&packet, &state, None);
        }
    }

    fn publish_represented_loot_roll_ownership_like_cpp(&self) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let Some(registry) = self.player_registry() else {
            return;
        };
        let identities = self
            .represented_loot_rolls
            .values()
            .map(|state| state.command_identity.clone())
            .collect();
        let _ = registry.replace_loot_rolls_for_control_channel(
            player_guid,
            &self.session_command_tx(),
            identities,
        );
    }

    pub(crate) async fn tick_represented_loot_rolls_like_cpp(&mut self) {
        let now = Instant::now();
        let roll_keys: Vec<(ObjectGuid, u8)> =
            self.represented_loot_rolls.keys().copied().collect();

        for (loot_obj, loot_list_id) in roll_keys {
            let Some(state) = self
                .represented_loot_rolls
                .get(&(loot_obj, loot_list_id))
                .cloned()
            else {
                continue;
            };
            if self
                .represented_current_loot_roll_authority_like_cpp(&state)
                .is_none()
            {
                self.cancel_represented_loot_roll_generation_mismatch_like_cpp(
                    (loot_obj, loot_list_id),
                    &state,
                );
                continue;
            }
            if state.end_time > now {
                continue;
            }

            let owner_guid = state.owner_guid;
            let Some(entry) = self.loot_table.get(&owner_guid).and_then(|loot| {
                loot.items
                    .iter()
                    .find(|entry| entry.loot_list_id == loot_list_id)
                    .cloned()
            }) else {
                self.represented_loot_rolls
                    .remove(&(loot_obj, loot_list_id));
                self.publish_represented_loot_roll_ownership_like_cpp();
                continue;
            };

            let winner = represented_loot_roll_current_winner_like_cpp(&state);
            self.finish_represented_loot_roll_like_cpp(
                loot_obj,
                loot_list_id,
                &entry,
                winner,
                Some(&state),
            )
            .await;
        }
    }

    fn represented_loot_roll_valid_rolls_like_cpp(
        item_flags2: Option<u32>,
        disenchant_skill_required: Option<u16>,
        max_enchanting_skill: u16,
    ) -> u8 {
        let mut valid_rolls = ROLL_ALL_TYPE_MASK_LIKE_CPP;
        if item_flags2.is_some_and(|flags| (flags & ItemFlags2::CanOnlyRollGreed as u32) != 0) {
            valid_rolls &= !ROLL_FLAG_TYPE_NEED_LIKE_CPP;
        }
        if disenchant_skill_required
            .is_none_or(|skill_required| skill_required > max_enchanting_skill)
        {
            valid_rolls &= !ROLL_FLAG_TYPE_DISENCHANT_LIKE_CPP;
        }

        valid_rolls
    }

    fn represented_loot_roll_disenchant_skill_required_like_cpp(
        &self,
        item_id: u32,
    ) -> Option<u16> {
        let template = self
            .item_stats_store()
            .and_then(|store| store.random_property_template(item_id))?;
        self.item_disenchant_loot_like_cpp(
            item_id,
            template.quality as u32,
            u32::from(template.item_level),
            true,
        )
        .map(|(_, skill_required)| skill_required)
    }
}
