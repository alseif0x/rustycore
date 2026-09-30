//! Roll lifetime fences, completion and durable award delivery.
use super::*;

impl WorldSession {
    /// A represented roll is scoped to one lifetime of the object-owned Loot.
    ///
    /// C++ destroys `LootRoll` together with its owning `Loot`. Rust keeps the
    /// packet-facing roll state in the session, so a recycled object GUID must
    /// not let that stale state unblock or award an item from a later lifetime.
    pub(super) fn represented_current_loot_roll_authority_like_cpp(
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

    pub(super) fn cancel_represented_loot_roll_generation_mismatch_like_cpp(
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

    pub(super) async fn finish_represented_loot_roll_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        item_valuation: &ItemValuationCatalogsLikeCpp,
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
                for (player_guid, vote) in state.ballots.votes() {
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
            item_guid_generator,
            item_valuation,
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

    pub(super) async fn store_represented_loot_roll_winner_item_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        item_valuation: &ItemValuationCatalogsLikeCpp,
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
                .store_represented_disenchant_loot_winner_with_generator_like_cpp(
                    item_guid_generator,
                    item_valuation,
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
                self.store_claimed_direct_loot_item_from_owner_with_generator_like_cpp(
                    item_guid_generator,
                    &store_entry,
                    dungeon_encounter_id,
                    owner_guid,
                    loot_obj,
                    claim,
                )
                .await
            } else {
                self.store_direct_loot_item_from_owner_with_generator_like_cpp(
                    item_guid_generator,
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

    pub(in crate::handlers::loot) async fn request_represented_remote_loot_roll_winner_store_like_cpp(
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
}
