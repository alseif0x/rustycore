// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented spell-effect bodies at the session boundary (#1363 slice 9c),
//! moved from the World `session/spell_effects` files without the execution
//! entry points, and the threat, taunt, transmog-set, kill-credit,
//! spell-charge, raid-marker and summon-slot bodies (#1363 slice 9d).
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
mod initial_threat;
mod player;
mod power;
mod progress;
mod pve_combat;
mod summon;
mod summon_contracts;
mod summon_slot;
mod taunt;
mod threat;

pub use apply::*;
pub use combat::*;
pub use effects::*;
pub use homebind::*;
pub use initial_threat::*;
pub use player::*;
pub use power::*;
pub use progress::*;
pub use pve_combat::*;
pub use summon::*;
#[cfg(any(test, feature = "test-fixtures"))]
pub use summon_contracts::*;
pub use summon_slot::*;
pub use taunt::*;
pub use threat::*;

/// The World session as the spell effects' host (supertrait: the spell-click
/// host, which lends the registry sync and the unit VALUES update object).
pub trait SpellEffectHostLikeCpp: SpellClickHostLikeCpp {
    fn set_player_drunk_value_like_cpp(
        &mut self,
        target_guid: ObjectGuid,
        effect_amount: i32,
        cast_item_guid: Option<ObjectGuid>,
        cast_item_entry: Option<u32>,
    ) -> Result<bool, &'static str>;
    fn publish_spell_pull_attack_start_like_cpp(
        &mut self,
        attacker: ObjectGuid,
        victim: ObjectGuid,
    );
    fn broadcast_creature_packet_to_visible_set_like_cpp(
        &self,
        source_guid: ObjectGuid,
        bytes: Vec<u8>,
    );
    fn add_transmog_set_like_cpp(
        &mut self,
        transmog_set_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate>;
    fn resolved_group_guid_like_cpp(&self) -> Option<u64>;
    fn on_creature_killed_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_entry: u32,
        creature_guid: ObjectGuid,
    ) -> impl Future<Output = ()> + Send;
    /// The World spell damage to a creature with its kill cascade (loot, XP,
    /// reputation and quest credit); the World implements it with
    /// `wow_world_application::apply_damage_from_caster_like_cpp` (#1363 slice 12b2b).
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
    /// The homebind persistence (the lifecycle port and its ordered worker);
    /// the World impl names the free function of the same name (#1363 slice
    /// 12b2), which also needs the battle-pet host's lifecycle split.
    fn persist_player_homebind_like_cpp(&mut self, homebind: RepresentedHomebindLikeCpp);
    /// The homebind write, which the login coordinator also calls; the World
    /// impl names the free function of the same name (#1363 slice 12b2).
    fn set_represented_homebind_like_cpp(&mut self, homebind: RepresentedHomebindLikeCpp) -> bool;
    fn represented_visibility_source_combat_reach_like_cpp(&self) -> f32;
}
