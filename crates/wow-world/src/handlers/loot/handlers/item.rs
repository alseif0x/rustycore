use super::*;

impl WorldSession {
    #[cfg(test)]
    pub async fn handle_loot_item(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_loot_item_with_generator_like_cpp(generators.item.as_ref(), pkt)
            .await;
    }

    /// CMSG_LOOT_ITEM — player clicks to take a specific item from the loot.
    pub async fn handle_loot_item_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        mut pkt: wow_packet::WorldPacket,
    ) {
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
                self.store_claimed_direct_loot_item_from_owner_with_generator_like_cpp(
                    item_guid_generator,
                    &entry,
                    dungeon_encounter_id,
                    owner_guid,
                    loot_req.object,
                    claim,
                )
                .await
            } else {
                self.store_direct_loot_item_from_owner_with_generator_like_cpp(
                    item_guid_generator,
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
                self.apply_pending_durable_item_loot_completions_with_generator_like_cpp(
                    item_guid_generator,
                )
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
}
