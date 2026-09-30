//! Master-loot request admission and recipient storage.
use super::*;

impl WorldSession {
    /// CMSG_MASTER_LOOT_ITEM — master looter assigns loot to a target.
    ///
    /// C++ first rejects players that are not in a group or are not the group's
    /// master looter with `LOOT_ERROR_DIDNT_KILL`. Current Rust group state has
    /// loot method `MASTER_LOOT` and the stored master-looter GUID matching the
    /// current player.
    pub async fn handle_master_loot_item_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        master_loot_item: MasterLootItem,
    ) {
        self.handle_master_loot_operation(item_guid_generator, master_loot_item, LootOperationPolicy::Production).await;
    }

    pub(in crate::handlers::loot) async fn handle_master_loot_operation(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        master_loot_item: MasterLootItem,
        policy: LootOperationPolicy,
    ) {
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
                .prepare_loot_authority_operation(owner_guid, player_guid, policy);
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
                && (owned_authority.is_some() || !policy.permits_local_cache(owned_authority.is_some()))
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

            let item = match select_master_item(
                loot,
                req.loot_list_id,
                master_loot_item.target,
            ) {
                Ok(item) => item,
                Err(MasterItemRejection::WrongMethod | MasterItemRejection::InvalidSlot) => return,
                Err(
                    MasterItemRejection::TargetNotAllowed
                    | MasterItemRejection::ItemTargetNotAllowed,
                ) => {
                    self.send_loot_error_like_cpp(
                        req.object,
                        owner_guid,
                        LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                    );
                    return;
                }
            };

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
                let Some(expected_generation) = self.loot_views.generation(&owner_guid)
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
                    self.store_claimed_direct_loot_item_from_owner_with_generator_like_cpp(
                        item_guid_generator,
                        &entry,
                        dungeon_encounter_id,
                        owner_guid,
                        req.object,
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
}
