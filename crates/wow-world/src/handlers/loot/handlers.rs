// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot packet entry points and their handler registrations.

use super::*;
use wow_packet::ClientPacket;

mod item;
mod money;

crate::session::registry::register_packet_handler_like_cpp! {
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

crate::session::registry::register_packet_handler_like_cpp! {
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

// The inspected TrinityCore opcode table assigns the shared unresolved 0xBADD
// placeholder to CMSG_CLEAR_RAID_MARKER (uint8 payload),
// CMSG_SET_LOOT_SPECIALIZATION (uint32), CMSG_SET_SAVED_INSTANCE_EXTEND
// (int32+uint32+bit), CMSG_CANCEL_MOD_SPEED_NO_CONTROL_AURAS (packed GUID) and
// CMSG_CLIENT_PORT_GRAVEYARD (empty). Rust keeps one enum variant and splits by
// payload length until the real opcode table is resolved, so this one
// registration carries all five payload shapes.
#[cfg(test)]
mod test_shims;

impl WorldSession {
    /// Receiver-owned half of the loot-release VALUES fanout. Applying
    /// `Player::isAllowedToLoot` here preserves session-local pending-bind
    /// state and avoids serialising one player's dynamic flags for another.
    pub(crate) fn handle_send_creature_loot_release_values_update_command_like_cpp(
        &mut self,
        command: SendCreatureLootReleaseValuesUpdateLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn
            || self.core.player_map_id_like_cpp() != command.map_id
        {
            return;
        }
        let instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if instance_id != command.instance_id
            || !self
                .core
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
            self.instances.has_pending_bind_like_cpp(),
            Some(expected_authority),
            command.unit_values_update,
        );
        self.send_packet(&UpdateObject::unit_values_update(
            command.creature_guid,
            command.map_id,
            viewer_update,
        ));
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
        if self.core.player_map_id_like_cpp() != command.map_id {
            return;
        }
        let session_instance_id = self
            .core
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
            .core
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
        crate::session::hub_ref(self).publish_self_share_health_like_cpp(&command);
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
        let Some(current_roll) = self
            .loot
            .represented_loot_roll_like_cpp(roll_key.0, roll_key.1)
        else {
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
            account = self.core.account_id,
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
            account = self.core.account_id,
            owner = ?command.loot_owner,
            loot_obj = ?command.loot_obj,
            loot_list_id = command.loot_list_id,
            ?result,
            "processed represented remote loot-roll winner store command"
        );

        let _ = command.result_tx.send(result);
    }
}
