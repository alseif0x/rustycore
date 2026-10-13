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

use wow_core::ObjectGuid;
use wow_entities::{RepresentedAuraEffectAmountLikeCpp, RepresentedAuraEffectLikeCpp};
use wow_world_core::session::HubMut;
use wow_world_entities::WorldEntitiesState;

use crate::SpellbookHostLikeCpp;

mod application;
mod aura_state;
mod mount;
mod publication;

pub use application::*;
pub use aura_state::*;
pub use mount::*;
pub use publication::*;

/// The World session as the auras' host. The supertrait lends the spellbook
/// host and, through it, the hub.
pub trait AuraHostLikeCpp: SpellbookHostLikeCpp {
    /// The World `split_world_entities_mut` split.
    fn split_world_entities_mut(&mut self) -> (&mut WorldEntitiesState, HubMut<'_>);
    /// The World aura removal by slot, through the application aura context.
    fn remove_aura(&mut self, slot: u8) -> Result<(), &'static str>;
    fn apply_represented_aura_modifier_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        effect: &wow_data::SpellEffectInfo,
        represented_effect: RepresentedAuraEffectLikeCpp,
        duration_ms: u32,
    ) -> Result<(), &'static str>;
    fn represented_mount_capability_selection_for_type_like_cpp(
        &self,
        mount_type_id: u16,
        riding_skill: u32,
        mount_restriction_flags: Option<u8>,
        is_submerged: bool,
        is_in_water: bool,
    ) -> Result<wow_data::MountCapabilityEntry, wow_data::MountCapabilityRejectLikeCpp>;
    fn represented_mount_x_display_usable_like_cpp(&self, player_condition_id: u32) -> bool;
    fn update_player_collision_height_like_cpp(&mut self);
    fn next_represented_spell_cast_guid_like_cpp(&self, spell_id: i32) -> Option<ObjectGuid>;
    fn canonical_threat_aura_snapshot_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        difficulty: u8,
        effect_mask: u32,
        represented_effect_amounts: &[RepresentedAuraEffectAmountLikeCpp],
    ) -> wow_entities::AuraThreatSnapshotLikeCpp;
    fn tick_represented_creature_auras_like_cpp(&mut self);
}
