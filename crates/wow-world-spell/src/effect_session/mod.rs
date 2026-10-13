// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented spell-effect bodies at the session boundary (#1363 slice 9c),
//! moved from the World `session/spell_effects` files without the execution
//! entry points.
//!
//! C++ `Spell::Effect*` (`SpellEffects.cpp`) and the `Unit` damage, healing
//! and power helpers they call (`Unit::SpellDamageBonusDone`,
//! `Unit::HealBySpell`, `Unit::EnergizeBySpell`, `Spell::SendSpellExecuteLog`).
//! The bodies run over [`SpellEffectHostLikeCpp`]; each host method is the
//! same-named `WorldSession` operation, called at the exact point the body
//! called it before the move.

use std::future::Future;

use wow_core::{ObjectGuid, Position};
use wow_entities::PlayerHomebindLikeCpp as RepresentedHomebindLikeCpp;

use crate::SpellClickHostLikeCpp;

mod apply;
mod combat;
mod effects;
mod homebind;
mod player;
mod power;
mod pve_combat;
mod summon;
mod summon_contracts;

pub use apply::*;
pub use combat::*;
pub use effects::*;
pub use homebind::*;
pub use player::*;
pub use power::*;
pub use pve_combat::*;
pub use summon::*;
pub use summon_contracts::*;

/// The World session as the spell effects' host (supertrait: the spell-click
/// host, which lends the registry sync and the unit VALUES update object).
pub trait SpellEffectHostLikeCpp: SpellClickHostLikeCpp {
    fn forward_heal_threat_like_cpp(
        &mut self,
        spell_id: Option<i32>,
        healer_guid: ObjectGuid,
        target_guid: ObjectGuid,
        effective_heal: u32,
    );
    fn forward_assisting_threat_like_cpp(
        &mut self,
        spell_id: Option<i32>,
        assistant_guid: ObjectGuid,
        assisted_guid: ObjectGuid,
        base_amount: f32,
        ignore_modifiers: bool,
    );
    fn canonical_threatened_by_me_owner_guids_like_cpp(
        &self,
        target_guid: ObjectGuid,
    ) -> Vec<ObjectGuid>;
    fn scale_canonical_owner_threat_to_target_zero_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        target_guid: ObjectGuid,
    );
    /// The World spell damage to a creature with its kill cascade (loot, XP,
    /// reputation and quest credit), which stays on the session.
    #[allow(clippy::too_many_arguments)]
    fn apply_damage_from_caster_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        spell_id: Option<i32>,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        damage_amount: u32,
        cast_id: ObjectGuid,
        spell_visual_id: u32,
    ) -> impl Future<Output = Result<(), &'static str>> + Send;
    fn teleport_to(&mut self, new_map: u32, new_pos: Position) -> impl Future<Output = ()> + Send;
    fn upgrade_account_heirloom_like_cpp(
        &mut self,
        item_id: u32,
        cast_item: i32,
    ) -> Option<wow_entities::PlayerValuesUpdate>;
    /// The World homebind persistence (the lifecycle port and its ordered
    /// worker), which stays on the session.
    fn persist_player_homebind_like_cpp(&mut self, homebind: RepresentedHomebindLikeCpp);
    fn represented_visibility_source_combat_reach_like_cpp(&self) -> f32;
}
