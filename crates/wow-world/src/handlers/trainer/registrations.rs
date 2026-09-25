// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Opcode registrations for this handler family (moved out of the root file to keep it
//! inside its physical budget; PacketHandlerEntry remains the single registration source).

use crate::session::registry::PacketHandlerEntry;
use wow_constants::ClientOpcodes;
use wow_handler::PacketProcessing;
use wow_handler::SessionStatus;
use wow_packet::ClientPacket;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TrainerList,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_trainer_list",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::gossip::Hello::read(&mut pkt) {
                    Ok(hello) => session.handle_trainer_list(hello).await,
                    Err(e) => tracing::warn!("Failed to read TrainerList: {e}"),
                }
            })
        },
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TrainerBuySpell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_trainer_buy_spell",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_trainer_buy_spell_with_generator_like_cpp(
                        catalogs.id_generators.item.as_ref(),
                        catalogs.battle_pet_trainer_selection.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}
