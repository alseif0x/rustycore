// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The represented spell execution at the session boundary (#1363 slice 9d),
//! moved from the World `session/spell_effects/{execution*,execution/**,destinations}.rs`.
//!
//! C++ `Spell::prepare`/`Spell::cast`/`Spell::_cast` (the cast checks and
//! power, `Spell::SelectSpellTargets`, `Spell::SendSpellGo`,
//! `Spell::HandleEffects` and `Spell::finish`) for the represented player
//! cast. The bodies run over [`SpellExecutionHostLikeCpp`]; each host method
//! is the same-named `WorldSession` operation, called at the exact point the
//! body called it before the move.

use std::future::Future;

use wow_core::{ObjectGuid, Position};
use wow_entities::SpellCastMetadata;
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;

use crate::{RepresentedSpellFocusObjectLikeCpp, SpellEffectHostLikeCpp};

mod cast_state;
mod destinations;
mod execution;
mod overloads;

pub use cast_state::*;
pub use destinations::*;
pub use execution::*;
pub use overloads::*;

/// The World session as the spell execution's host (supertrait: the spell
/// effect host). The effect bodies that reach the World quest, social,
/// visibility, item and acquisition state stay on the session and are called
/// here as host methods.
pub trait SpellExecutionHostLikeCpp:
    SpellEffectHostLikeCpp + wow_world_inventory::BattlePetSessionHostLikeCpp
{
    fn mutate_cast_execution_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::CastExecutionStateLikeCpp) -> R,
    ) -> Option<R>;
    fn check_represented_cast_preparation_like_cpp(
        &mut self,
        spell_info: &wow_data::SpellInfo,
        cast_id: ObjectGuid,
        spell_visual: &wow_packet::packets::spell::SpellCastVisual,
        metadata: SpellCastMetadata,
    ) -> Result<Option<Option<RepresentedSpellFocusObjectLikeCpp>>, &'static str>;
    fn force_update_visibility_with_catalogs_like_cpp(
        &mut self,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    ) -> impl Future<Output = ()> + Send;
    fn consume_add_farsight_set_seer_outcome_like_cpp(
        &mut self,
        outcome: &wow_map::map::FarsightDynamicObjectCreateOutcomeLikeCpp,
    ) -> bool;
    fn represented_nearby_entry_destination_like_cpp(
        &self,
        effect: &wow_data::SpellEffectInfo,
        caster_position: &Position,
        range: f32,
        implicit_conditions: Option<&[wow_data::Condition]>,
    ) -> Option<Position>;
    fn apply_spell_initial_threat_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        is_positive: bool,
    );
    fn apply_creature_aura_with_provenance_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        effect_mask: u32,
        duration_ms: u32,
        provenance: wow_entities::AuraCastProvenanceLikeCpp,
    ) -> Result<(), &'static str>;
    fn apply_aura_with_effect_mask_and_provenance_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
        provenance: wow_entities::AuraCastProvenanceLikeCpp,
    ) -> Result<(), &'static str>;
    fn apply_durability_damage_effect_like_cpp(
        &mut self,
        effect: u32,
        damage: i32,
        slot: i32,
        target_guid: ObjectGuid,
    );
    fn apply_durability_damage_pct_effect_like_cpp(
        &mut self,
        damage: i32,
        slot: i32,
        target_guid: ObjectGuid,
    );
    fn apply_duel_effect_like_cpp(
        &mut self,
        spell_id: i32,
        gameobject_entry: i32,
        target_guid: ObjectGuid,
    ) -> bool;
    fn apply_learn_spell_effect_like_cpp(
        &mut self,
        spell: i32,
        target: ObjectGuid,
    ) -> impl Future<Output = bool> + Send;
    fn apply_reputation_effect_like_cpp(
        &mut self,
        damage: i32,
        misc_value: i32,
        target_guid: ObjectGuid,
    ) -> bool;
    fn apply_proficiency_effect_like_cpp(&mut self, spell_id: i32) -> Result<(), &'static str>;
    fn apply_quest_complete_effect_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        target_guid: ObjectGuid,
        quest_id: i32,
    ) -> impl Future<Output = Result<(), &'static str>> + Send;
    fn apply_stuck_effect_like_cpp(&mut self) -> impl Future<Output = ()> + Send;
    fn apply_self_resurrect_effect_like_cpp(&mut self, damage: i32, misc_value: i32);
    fn apply_force_deselect_effect_like_cpp(&mut self) -> bool;
}
