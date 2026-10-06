// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the spell cancel handler context (#1263 F5).
//!
//! The application crate owns the bodies and their context; the session only
//! lends its hub and the aura-application participants (spell, inventory,
//! loot and quest state), so no session reference crosses into the handler.

use wow_world_application::{SpellHandlerCxLikeCpp, SpellHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl SpellHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn spell_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> SpellHandlerCxLikeCpp<'a> {
        let (spell_state, inventory, loot, quest_state, hub) =
            crate::session::state::split_aura_application_mut(self);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = quest_state;
        SpellHandlerCxLikeCpp::new(
            hub,
            spell_state,
            inventory,
            loot,
            #[cfg(any(test, feature = "test-fixtures"))]
            quest_state,
            cfg!(test),
        )
    }
}
