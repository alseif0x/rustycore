// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use tracing::warn;

use wow_constants::ClientOpcodes;
use wow_constants::unit::NPCFlags1;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::packets::talent::{
    ConfirmRespecWipe, LearnTalent, LearnTalents, SPEC_RESET_TALENTS_LIKE_CPP,
};
use wow_packet::{ClientPacket, WorldPacket};

#[cfg(test)]
use crate::session::RepresentedTalentRespecVisualSpellCastLikeCpp;
use crate::session::{RepresentedConfirmRespecWipeLikeCpp, WorldSession};

mod state;
#[allow(unused_imports)]
pub use state::*;

#[cfg(test)]
#[path = "talent/tests/mod.rs"]
mod tests;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ConfirmRespecWipe,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_confirm_respec_wipe",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_confirm_respec_wipe_with_generator_like_cpp(
                        catalogs.id_generators.item.as_ref(),
                        catalogs.progression.no_reset_talent_cost,
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LearnTalent,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_learn_talent",
        handler: |session, catalogs, pkt| Box::pin(async move { session.handle_learn_talent(catalogs.player_bootstrap.talent_tabs.as_ref(), pkt).await }),
    }
}
