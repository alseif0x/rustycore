// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private guild capability handlers extracted from the legacy misc owner.
//!
//! Every guild opcode is registered by its owner under #1263 F5: the guild-bank
//! family by `wow-world-application::guild_bank_handlers` and the invitation and
//! auto-decline handlers by `wow-world-social::guild_handlers`. This module only
//! lends the World session's state to the guild-bank context.

use wow_world_application::{GuildBankHandlerCxLikeCpp, GuildBankHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl GuildBankHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn guild_bank_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> GuildBankHandlerCxLikeCpp<'a> {
        let (inventory, social, world_entities, hub) = crate::session::split_guild_bank_mut(self);
        GuildBankHandlerCxLikeCpp::new(hub, inventory, social, world_entities, cfg!(test))
    }
}

#[cfg(test)]
mod test_shims;

#[cfg(test)]
#[path = "../../../unit_tests/handlers/guild/tests/mod.rs"]
mod tests;
