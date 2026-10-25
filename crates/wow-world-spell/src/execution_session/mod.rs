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

use tracing::{debug, info};
use wow_core::{ObjectGuid, Position};
use wow_entities::{RepresentedAuraEffectLikeCpp, SpellCastMetadata};
use wow_packet::packets::spell::SpellTargetData;
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;

use crate::SpellEffectHostLikeCpp;
use crate::aura_session::{
    apply_represented_mounted_aura_like_cpp, apply_represented_provide_spell_focus_aura_like_cpp,
    update_represented_flight_flags_for_flight_aura_like_cpp,
};
use crate::effect_session::{
    apply_add_extra_attacks_effect_like_cpp, apply_block_effect_like_cpp,
    apply_change_raid_marker_effect_like_cpp, apply_dismiss_pet_effect_like_cpp,
    apply_distract_effect_like_cpp, apply_dual_wield_effect_like_cpp,
    apply_effect_environmental_damage_like_cpp, apply_energize_effect_like_cpp,
    apply_give_honor_effect_like_cpp, apply_heal_from_caster_like_cpp,
    apply_heal_max_health_like_cpp, apply_heal_pct_like_cpp, apply_health_leech_like_cpp,
    apply_inebriate_effect_like_cpp, apply_instakill_like_cpp, apply_kill_credit_effect_like_cpp,
    apply_learn_transmog_set_effect_like_cpp, apply_modify_cooldown_effect_like_cpp,
    apply_modify_spell_charges_effect_like_cpp, apply_modify_threat_percent_effect_like_cpp,
    apply_parry_effect_like_cpp, apply_play_movie_effect_like_cpp,
    apply_power_drain_effect_like_cpp, apply_sanctuary_effect_like_cpp,
    apply_taunt_effect_like_cpp, apply_threat_effect_like_cpp, apply_titan_grip_effect_like_cpp,
    apply_upgrade_heirloom_effect_like_cpp, power_drain_pre_scaled_damage_like_cpp,
    represented_spell_damage_bonus_done_like_cpp, represented_spell_healing_bonus_done_like_cpp,
};
use completion::complete_spell_execution_like_cpp;
use post_spell_go::{
    apply_post_spell_go_target_effects_like_cpp, apply_spell_gameobject_summon_effects_like_cpp,
};
use preparation::prepare_represented_spell_execution_like_cpp;
use primary_effect_fallback::apply_primary_spell_effect_fallback_like_cpp;
use target_resolution::{
    ResolvedSpellEffectTargetsLikeCpp, resolve_spell_effect_target_data_like_cpp,
};

mod cast_state;
mod checks;
mod completion;
mod destinations;
mod execution;
mod farsight;
mod item_effects;
mod nearby_entry;
mod overloads;
mod player_effects;
mod post_spell_go;
mod preparation;
mod primary_effect_fallback;
mod resurrection_auras;
mod resurrection_offer;
mod resurrection_sickness;
mod selected_creation;
mod target_resolution;

pub use cast_state::*;
pub use checks::*;
pub use destinations::*;
pub use execution::*;
pub use farsight::*;
pub use item_effects::*;
pub use nearby_entry::*;
pub use overloads::*;
pub use player_effects::*;

/// The World session as the spell execution's host (supertrait: the spell
/// effect host). The effect bodies that reach the World quest, social,
/// visibility, item and acquisition state are called here as host methods;
/// where such a body has moved to this crate or above it (#1363 slice 12b2),
/// the World impl names the moved free function.
pub trait SpellExecutionHostLikeCpp:
    SpellEffectHostLikeCpp + wow_world_inventory::BattlePetSessionHostLikeCpp
{
    fn mutate_cast_execution_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::CastExecutionStateLikeCpp) -> R,
    ) -> Option<R>;
    fn has_represented_battle_pet_journal_lock_like_cpp(&self) -> bool;
    fn represented_battle_pet_like_cpp(
        &self,
        pet_guid: ObjectGuid,
    ) -> Option<wow_world_core::session::RepresentedBattlePetDataLikeCpp>;
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
        send_update: bool,
    ) -> Result<(), &'static str>;
    fn apply_ghost_wisp_aura(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        spell_info: &wow_data::SpellInfo,
        duration_ms: u32,
        effect_mask: u32,
        stack_amount: u32,
        attributes1: u32,
        attributes5: u32,
        charges: u8,
        creation: Result<(u32, wow_entities::AuraCastProvenanceLikeCpp), &'static str>,
    ) -> Result<(), &'static str>;
    fn apply_resurrection_sickness_aura(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        spell_info: &wow_data::SpellInfo,
        duration_total: i32,
        stack_amount: u32,
        attributes1: u32,
        attributes5: u32,
        charges: u8,
        creation: Result<(u32, wow_entities::AuraCastProvenanceLikeCpp), &'static str>,
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
    fn apply_quest_complete_effect_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        target_guid: ObjectGuid,
        quest_id: i32,
    ) -> impl Future<Output = Result<(), &'static str>> + Send;
    fn apply_force_deselect_effect_like_cpp(&mut self) -> bool;
    /// The World resurrection snapshot, ghost flag and alive state the moved
    /// stuck effect reads and writes (#1363 slice 12b2).
    fn set_player_ghost_flag_like_cpp(&mut self, ghost: bool);
    fn set_player_alive_like_cpp(&mut self, alive: bool);
    /// The World test session's handle-less capability owner for the moved
    /// proficiency effect's fixture step; `None` for every other host.
    #[cfg(any(test, feature = "test-fixtures"))]
    fn mutate_player_persistent_capability_state_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut wow_entities::PlayerPersistentCapabilityStateLikeCpp) -> R,
    ) -> Option<R> {
        let _ = mutate;
        None
    }
    /// The World test session's represented seer fixture, which the moved
    /// farsight seer consumer writes; a no-op for every other host.
    #[cfg(any(test, feature = "test-fixtures"))]
    fn set_represented_seer_guid_fixture_like_cpp(&mut self, guid: Option<ObjectGuid>) {
        let _ = guid;
    }

    /// Whether any linked module wants `player.spell_cast` (#1444 H3).
    fn spell_cast_hook_wanted_like_cpp(&self) -> bool {
        false
    }

    /// `player.spell_cast`: TrinityCore `OnPlayerSpellCast`, raised once per
    /// player cast after `SMSG_SPELL_GO`, with no guard or await in scope.
    fn spell_cast_hook_like_cpp(&mut self, _event: &crate::SpellCastHookEventLikeCpp) {}
}
