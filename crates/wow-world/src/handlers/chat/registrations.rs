// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Opcode registrations for this handler family (moved out of the root file to keep it
//! inside its physical budget; PacketHandlerEntry remains the single registration source).

use crate::session::registry::PacketHandlerEntry;
use wow_constants::ClientOpcodes;
use wow_handler::PacketProcessing;
use wow_handler::SessionStatus;

crate::session::registry::register_packet_handler_like_cpp {
    PacketHandlerEntry {
        opcode: ClientOpcodes::Emote,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_emote",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_emote(pkt).await }),
    }
}
crate::session::registry::register_packet_handler_like_cpp {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SendTextEmote,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_text_emote",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_text_emote_with_catalogs_like_cpp(
                        catalogs.emotes_text.as_ref(),
                        catalogs.emotes.as_ref(),
                        catalogs.chat_policy.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}
