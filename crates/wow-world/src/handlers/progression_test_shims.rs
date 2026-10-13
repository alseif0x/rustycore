// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the slice 7c progression families
//! (#1363). The battle-pet XP-percent aura moved to `wow_world_spell`; its
//! entry point forwards to the moved body with the session's spell-state
//! split, so the scenarios keep their method syntax and the session hotspot's
//! test lines do not grow.

#![cfg(test)]

use wow_core::ObjectGuid;

use crate::session::WorldSession;

impl WorldSession {
    pub(crate) fn apply_represented_battle_pet_xp_pct_aura_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        effect: &wow_data::SpellEffectInfo,
    ) -> Result<(), &'static str> {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.apply_represented_battle_pet_xp_pct_aura_like_cpp(
            &mut hub,
            spell_id,
            caster_guid,
            effect,
        )
    }
}
