// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the aura bodies moved to
//! `wow_world_spell` (#1363 slice 9b).
//!
//! The first impl block holds the `#[cfg(test)]` method of
//! `session/spell_state/mount_aura.rs`, verbatim: it was already test-only.
//! The second holds one-call method forwards for the moved functions the
//! scenarios and these fixtures call, with this session as the host, so no
//! scenario call site changes.

#![cfg(test)]

use crate::session::*;
use wow_core::ObjectGuid;

impl WorldSession {
    pub(crate) fn apply_represented_mounted_aura_for_test_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        effect: &wow_data::SpellEffectInfo,
    ) -> Result<(), &'static str> {
        self.apply_represented_mounted_aura_like_cpp(spell_id, caster_guid, effect)
    }
}

impl WorldSession {
    pub(crate) fn represented_player_aura_state_mask_like_cpp(&self) -> Option<u32> {
        wow_world_spell::represented_player_aura_state_mask_like_cpp(self)
    }
    pub(crate) fn load_represented_character_auras_like_cpp(
        &mut self,
        aura_rows: impl IntoIterator<Item = CharacterAuraRowLikeCpp>,
        effect_rows: impl IntoIterator<Item = CharacterAuraEffectRowLikeCpp>,
        timediff_secs: u32,
    ) -> usize {
        wow_world_spell::load_represented_character_auras_like_cpp(
            self,
            aura_rows,
            effect_rows,
            timediff_secs,
        )
    }
    pub(crate) fn represented_mount_aura_display_candidates_like_cpp(
        &self,
        spell_id: u32,
    ) -> Vec<i32> {
        wow_world_spell::represented_mount_aura_display_candidates_like_cpp(self, spell_id)
    }
    pub(crate) fn apply_represented_mounted_aura_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        effect: &wow_data::SpellEffectInfo,
    ) -> Result<(), &'static str> {
        wow_world_spell::apply_represented_mounted_aura_like_cpp(
            self,
            spell_id,
            caster_guid,
            effect,
        )
    }
    pub(crate) fn tick_auras(&mut self) {
        wow_world_spell::tick_auras(self)
    }
    pub(crate) fn insert_player_visible_aura_like_cpp(&mut self, aura: AuraApplication) -> bool {
        wow_world_spell::insert_player_visible_aura_like_cpp(self, aura)
    }
    pub(crate) fn represented_spell_area_autocast_source_is_empty_like_cpp(&self) -> bool {
        wow_world_spell::represented_spell_area_autocast_source_is_empty_like_cpp(self)
    }
}
