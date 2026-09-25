// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot packet entry points and their handler registrations.

use super::*;
use wow_packet::ClientPacket;

mod item;
mod money;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LootUnit,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_unit",
        handler: |session, catalogs, pkt| Box::pin(async move {
            session
                .handle_loot_unit_with_catalogs_like_cpp(catalogs.item_valuation.as_ref(), pkt)
                .await
        }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LootItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_item",
        handler: |session, catalogs, pkt| Box::pin(async move {
            session
                .handle_loot_item_with_generator_like_cpp(
                    catalogs.id_generators.item.as_ref(),
                    pkt,
                )
                .await
        }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LootMoney,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_money",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_loot_money_with_generator_like_cpp(
                        catalogs.id_generators.item.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LootRelease,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_release",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_loot_release(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LootRoll,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_roll",
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::loot::LootRoll::read(&mut pkt) {
                    Ok(roll) => {
                        session
                            .handle_loot_roll_with_generator_like_cpp(
                                catalogs.id_generators.item.as_ref(),
                                catalogs.item_valuation.as_ref(),
                                roll,
                            )
                            .await
                    }
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
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::loot::MasterLootItem::read(&mut pkt) {
                    Ok(master_loot_item) => {
                        session
                            .handle_master_loot_item_with_generator_like_cpp(
                                catalogs.id_generators.item.as_ref(),
                                master_loot_item,
                            )
                            .await
                    }
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
        handler: |session, _catalogs, mut pkt| {
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
    #[cfg(test)]
    pub async fn handle_loot_roll(&mut self, roll: LootRoll) {
        let generators = self.id_generators_for_test_like_cpp();
        let item_valuation = self.item_valuation_catalogs_for_test_like_cpp();
        self.handle_loot_roll_with_generator_like_cpp(
            generators.item.as_ref(),
            &item_valuation,
            roll,
        )
        .await;
    }

    #[cfg(test)]
    pub async fn handle_master_loot_item(&mut self, master_loot_item: MasterLootItem) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_master_loot_item_with_generator_like_cpp(
            generators.item.as_ref(),
            master_loot_item,
        )
        .await;
    }

    /// CMSG_LOOT_UNIT — player right-clicks a dead creature to loot it.
    #[cfg(test)]
    pub async fn handle_loot_unit(&mut self, pkt: wow_packet::WorldPacket) {
        let item_valuation = self.item_valuation_catalogs_for_test_like_cpp();
        self.handle_loot_unit_with_catalogs_like_cpp(&item_valuation, pkt)
            .await;
    }

    pub async fn handle_loot_unit_with_catalogs_like_cpp(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        mut pkt: wow_packet::WorldPacket,
    ) {
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
        self.represented_on_loot_opened_with_catalogs_like_cpp(
            item_valuation,
            req.unit,
            player_guid,
            response,
        );

        if !ae_owner_guids.is_empty() {
            self.send_packet(&AELootTargetsAck);

            for owner_guid in ae_owner_guids {
                if let Some(response) = self
                    .represented_loot_response_for_owner_like_cpp(owner_guid, player_guid, true)
                    .await
                {
                    self.add_active_loot_view_owner_like_cpp(owner_guid);
                    self.represented_on_loot_opened_with_catalogs_like_cpp(
                        item_valuation,
                        owner_guid,
                        player_guid,
                        response,
                    );
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

    /// CMSG_LOOT_ROLL — vote on a pending group loot roll.
    ///
    /// C++ `HandleLootRoll` silently returns when `GetLootRoll` finds no
    /// canonical roll state. Rust does not yet port that state machine, so this
    /// represented handler preserves the current wire behavior without emitting
    /// synthetic errors.
    pub async fn handle_loot_roll_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        roll: LootRoll,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };

        if self
            .represented_player_vote_on_loot_roll_with_generator_like_cpp(
                item_guid_generator,
                item_valuation,
                &roll,
                player_guid,
            )
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
    pub async fn handle_master_loot_item_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        master_loot_item: MasterLootItem,
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
        // C++ `Unit::Kill` applies the creature-killer durability loss while
        // the lethal damage is applied, before the melee result presentation.
        self.publish_creature_melee_death_durability_loss_like_cpp(command.over_damage);

        // C++ `Unit::CalcAbsorbResist`'s absorb log and shield removal (`Unit.cpp:1876-1889`).
        self.publish_melee_absorb_consumption_like_cpp(
            command.attacker_guid,
            command.victim_guid,
            command.original_damage.min(i32::MAX as u32) as i32,
            command.mana_spent,
            &command.absorb_consumptions,
            &command.split_combat_log_packets,
        );
        use wow_packet::packets::combat::{AttackerStateUpdate, HealthUpdate};
        // Visibility gates only the attacker-facing combat packet, never the
        // authoritative victim health/death reconciliation.
        if self
            .client_visible_guids_like_cpp
            .contains(&command.attacker_guid)
        {
            self.send_packet(&AttackerStateUpdate {
                attacker: command.attacker_guid,
                victim: command.victim_guid,
                hit_info: command.hit_info,
                damage: command.damage.min(i32::MAX as u32) as i32,
                original_damage: command.original_damage.min(i32::MAX as u32) as i32,
                over_damage: command.over_damage,
                blocked: 0,
                absorbed: command.absorbed.min(i32::MAX as u32) as i32,
                victim_state: command.victim_state,
                school_mask: 1,
                target_level: command.target_level,
                expansion: 2,
            });
        }
        self.publish_self_share_health_like_cpp(&command);
        // An avoided swing commits no health transition.
        if command.damage > 0 {
            self.send_packet(&HealthUpdate {
                guid: command.victim_guid,
                health: canonical_health.min(i64::MAX as u64) as i64,
            });
        }
    }

    pub(crate) async fn handle_represented_loot_roll_vote_command_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        command: LootRollVoteCommand,
    ) {
        let roll_key = (command.loot_obj, command.loot_list_id);
        let Some(current_roll) = self.represented_loot_rolls.get(&roll_key) else {
            return;
        };
        if !command.targets_identity_like_cpp(&current_roll.command_identity) {
            return;
        }

        let roll = LootRoll {
            loot_obj: command.loot_obj,
            loot_list_id: command.loot_list_id,
            roll_type: command.roll_type,
        };

        let _ = self
            .represented_player_vote_on_loot_roll_with_pass_state_and_generator_like_cpp(
                item_guid_generator,
                item_valuation,
                &roll,
                command.voter_guid,
                command.pass_on_group_loot,
            )
            .await;
    }

    pub(crate) async fn handle_represented_master_loot_give_command_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
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
            self.store_claimed_direct_loot_item_from_owner_with_generator_like_cpp(
                item_guid_generator,
                &command.entry,
                command.dungeon_encounter_id,
                command.loot_owner,
                command.loot_obj,
                claim,
            )
            .await
        } else {
            self.store_direct_loot_item_from_owner_with_generator_like_cpp(
                item_guid_generator,
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

    pub(crate) async fn handle_represented_loot_roll_store_winner_command_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
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
            self.store_direct_disenchant_batch_with_generator_like_cpp(
                item_guid_generator,
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
            self.store_claimed_direct_loot_item_from_owner_with_generator_like_cpp(
                item_guid_generator,
                &command.entries[0],
                command.dungeon_encounter_id,
                command.loot_owner,
                command.loot_obj,
                claim,
            )
            .await
        } else {
            self.store_direct_loot_item_from_owner_with_generator_like_cpp(
                item_guid_generator,
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
