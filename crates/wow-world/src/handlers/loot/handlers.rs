// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot packet entry points and their handler registrations.

use super::*;

mod master;
mod command_delivery;

use wow_loot::{
    MasterItemRejection, master_award_recipient_allowed, roll_award_batch_allowed,
    select_master_item,
};
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

        if creature_state.is_alive() {
            return;
        }

        if self
            .player_position_like_cpp()
            .is_some_and(|player| !player.is_within_dist(&creature_state.position(), 30.0))
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
