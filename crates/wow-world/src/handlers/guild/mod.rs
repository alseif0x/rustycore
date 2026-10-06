// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private guild capability handlers extracted from the legacy misc owner.
//!
//! The guild-bank family moved to `wow-world-application::guild_bank_handlers`
//! under #1263 F5; this module lends the World session's state to it and keeps
//! the auto-decline handler, which still re-publishes the registry state.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{PacketProcessing, SessionStatus};
use wow_world_application::{GuildBankHandlerCxLikeCpp, GuildBankHandlerHostLikeCpp};

use crate::session::registry::PacketHandlerEntry;
use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};
use wow_packet::ClientPacket;
use wow_packet::packets::misc::DeclineGuildInvites;

impl GuildBankHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn guild_bank_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> GuildBankHandlerCxLikeCpp<'a> {
        let (inventory, social, world_entities, hub) = crate::session::split_guild_bank_mut(self);
        GuildBankHandlerCxLikeCpp::new(hub, inventory, social, world_entities, cfg!(test))
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DeclineGuildInvites,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_decline_guild_invites",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_decline_guild_invites(pkt).await })
        },
    }
}

#[cfg(test)]
mod test_shims;

impl crate::session::WorldSession {
    pub async fn handle_decline_guild_invites(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match DeclineGuildInvites::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.core.account_id,
                    "DeclineGuildInvites parse failed: {error}"
                );
                return;
            }
        };

        self.represented_set_auto_decline_guild_invites_like_cpp(request.allow);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/handlers/guild/tests/mod.rs"]
mod tests;
