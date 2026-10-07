// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the character handlers moved to
//! `wow-world-application` (#1263 F5). They dispatch through the registered
//! production thunks.

use wow_constants::ClientOpcodes;
use wow_packet::WorldPacket;
use wow_packet::packets::character::CharacterRenameRequest;

use crate::session::WorldSession;

async fn dispatch_registered_like_cpp(
    session: &mut WorldSession,
    opcode: ClientOpcodes,
    pkt: WorldPacket,
) {
    let entry = crate::session::registry::registered_handler_entries_like_cpp()
        .find(|entry| entry.opcode == opcode)
        .expect("registered character handler");
    let catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
    (entry.handler)(session, &catalogs, pkt).await;
}

impl WorldSession {
    pub async fn handle_get_undelete_cooldown_status(&mut self) {
        dispatch_registered_like_cpp(
            self,
            ClientOpcodes::GetUndeleteCharacterCooldownStatus,
            WorldPacket::new_empty(),
        )
        .await;
    }

    pub async fn handle_alter_appearance(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::AlterAppearance, pkt).await;
    }

    pub async fn handle_confirm_barbers_choice(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::ConfirmBarbersChoice, pkt).await;
    }

    pub async fn handle_set_player_declined_names(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::SetPlayerDeclinedNames, pkt).await;
    }

    pub async fn handle_character_rename_request(&mut self, pkt: CharacterRenameRequest) {
        dispatch_registered_like_cpp(
            self,
            ClientOpcodes::CharacterRenameRequest,
            character_rename_request_wire_like_cpp(&pkt),
        )
        .await;
    }

    pub async fn handle_opening_cinematic(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::OpeningCinematic, pkt).await;
    }
}

/// `CMSG_CHARACTER_RENAME_REQUEST` body, mirroring
/// `CharacterRenameRequest::read` so a caller that holds the parsed request can
/// still enter through the registered production thunk.
fn character_rename_request_wire_like_cpp(pkt: &CharacterRenameRequest) -> WorldPacket {
    let mut wire = WorldPacket::new_empty();
    wire.write_guid(&pkt.guid);
    wire.write_bits(pkt.new_name.len() as u32, 6);
    wire.flush_bits();
    wire.write_string(&pkt.new_name);
    wire
}
