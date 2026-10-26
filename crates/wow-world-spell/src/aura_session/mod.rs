// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented player auras at the session boundary (#1363 slice 9b).
//!
//! C++ `Unit::_AddAura`/`RemoveAura` bookkeeping on the Player, the
//! `Player::_LoadAuras` restore, `AuraEffect::HandleMount` and the
//! interrupt-flag, attribute and expiry removals. Their Rust bodies now live
//! in this crate as free functions over [`AuraHostLikeCpp`]; each host method
//! is the same-named `WorldSession` operation or split, called at the exact
//! point the body called it before the move.

use std::time::Instant;

use wow_core::ObjectGuid;
use wow_entities::{
    AuraApplicationLikeCpp as AuraApplication, RepresentedAuraEffectLikeCpp,
    legacy_aura_duration_pair,
};
use wow_world_core::session::{HubMut, HubRef};
use wow_world_entities::WorldEntitiesState;

use crate::{SpellbookHostLikeCpp, represented_aura_effect_amounts_like_cpp};

mod application;
mod aura_state;
mod duration;
mod mount;
mod publication;
mod selected_caster;
pub(crate) mod selected_catalog;
mod selected_charges;
mod selected_creation;
mod selected_flags;
mod selected_lifetime;
mod selected_load;
mod selected_metadata;
mod selected_reaction;
mod selected_visual;
mod sources;

pub use application::*;
pub use aura_state::*;
pub use duration::*;
pub use mount::*;
pub use publication::*;
pub(super) use selected_caster::selected_caster_level;
pub use selected_charges::selected_aura_max_charges;
pub(crate) use selected_charges::validate_fresh_selected_self_caster_modifiers;
pub(crate) use selected_creation::prepare_selected_aura_creation;
pub(crate) use selected_reaction::selected_caster_is_friendly;
pub(crate) use selected_flags::{
    SelectedAuraFlagsError, SelectedSpellMetadataFacts, SelectedSpellPolarity,
    SelectedSpellPolarityError, build_selected_aura_application_flags,
    classify_selected_spell_polarity,
};
pub(crate) use selected_lifetime::validate_application as validate_selected_aura_application;
pub use selected_lifetime::*;
pub(crate) use selected_metadata::{
    SelectedSpellMetadata, selected_spell_metadata,
};
pub(crate) use selected_visual::selected_aura_spell_visual_id;
pub use sources::*;

#[cfg(test)]
#[path = "../../unit_tests/aura_session/selected_caster.rs"]
mod selected_caster_tests;

#[cfg(test)]
#[path = "../../unit_tests/aura_session/selected_charges.rs"]
mod selected_charges_tests;

#[cfg(test)]
#[path = "../../unit_tests/aura_session/selected_visual.rs"]
mod selected_visual_tests;

#[cfg(test)]
#[path = "../../unit_tests/aura_session/selected_flags.rs"]
mod selected_flags_tests;

/// The World session as the auras' host (supertrait: the spellbook host).
pub trait AuraHostLikeCpp: SpellbookHostLikeCpp {
    fn split_world_entities_ref(&self) -> (&WorldEntitiesState, HubRef<'_>);
    fn split_world_entities_mut(&mut self) -> (&mut WorldEntitiesState, HubMut<'_>);
    fn remove_aura(&mut self, slot: u8) -> Result<(), &'static str>;
    fn apply_loaded_ghost_wisp_aura(&mut self, aura: &AuraApplication) -> Result<(), &'static str>;
    /// The represented player aura modifier (#1363 slice 9e), moved from the
    /// World `session/spell_state/aura_application.rs` as this provided
    /// method so its 23 spell call sites keep their method form.
    fn apply_represented_aura_modifier_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        effect: &wow_data::SpellEffectInfo,
        represented_effect: RepresentedAuraEffectLikeCpp,
        duration_ms: u32,
    ) -> Result<(), &'static str>
    where
        Self: Sized,
    {
        let Some((duration_total, duration_remaining)) =
            legacy_aura_duration_pair(duration_ms, duration_ms)
        else {
            return Err("Aura duration exceeds the signed application range");
        };
        let slot = next_player_visible_aura_slot_like_cpp(self)
            .ok_or("No free aura slots or missing Player aura owner")?;

        let aura = AuraApplication {
            spell_id,
            difficulty_id: self.core().current_map_difficulty_id_like_cpp(),
            caster_guid,
            slot,
            duration_total,
            duration_remaining,
            stack_count: 1,
            aura_flags: 0x0000_0001,
            effect_mask: 1u32 << effect.effect_index,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: Some(represented_effect),
            represented_amount: effect.effect_base_points,
            represented_effect_amounts: represented_aura_effect_amounts_like_cpp(effect),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: Instant::now(),
        };

        if !insert_player_visible_aura_like_cpp(self, aura) {
            return Err("Missing Player aura owner");
        }
        send_aura_update_applied(
            self,
            spell_id,
            slot,
            caster_guid,
            duration_ms,
            0x0000_0001,
            1u32 << effect.effect_index,
        );

        Ok(())
    }
    fn represented_mount_x_display_usable_like_cpp(&self, player_condition_id: u32) -> bool;
    fn next_represented_spell_cast_guid_like_cpp(&self, spell_id: i32) -> Option<ObjectGuid>;
    fn tick_represented_creature_auras_like_cpp(&mut self);
    fn represented_spell_area_quest_status_like_cpp(&self, quest_id: u32) -> Option<u8>;
}
