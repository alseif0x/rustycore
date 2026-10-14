// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the player cast, the spell handlers,
//! the spell-click plan, the respec wipe and the aura modifier moved to
//! `wow_world_spell` (#1363 slice 9e).
//!
//! The first impl block holds the `#[cfg(test)]` overloads that were nested in
//! the moved production files, verbatim except that their calls to the moved
//! entry points are free calls: they were already test-only. The second holds
//! one-call method forwards for the moved functions the scenarios call, with
//! this session as the host, so no scenario call site changes.

#![cfg(test)]

use crate::session::*;
use wow_core::ObjectGuid;

impl WorldSession {
    pub(crate) fn apply_represented_aura_modifier_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        effect: &wow_data::SpellEffectInfo,
        represented_effect: RepresentedAuraEffectLikeCpp,
        duration_ms: u32,
    ) -> Result<(), &'static str> {
        <Self as wow_world_spell::AuraHostLikeCpp>::apply_represented_aura_modifier_like_cpp(
            self,
            spell_id,
            caster_guid,
            effect,
            represented_effect,
            duration_ms,
        )
    }
}
