// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature combat: aggro, threat, melee and evade.
//!
//! #1263 F6-8D1 moved the complete operation onto canonical `Creature`
//! ownership (`wow_entities::creature::phase_ops`). What remains here is the
//! transitional seam the legacy scheduling and publication bridges still call:
//! every entry point delegates to the canonical owner and this module keeps no
//! combat state of its own. The one value that cannot cross is
//! `SPELL_AURA_MOD_TAUNT`, which lives in the static-data layer the
//! domain-runtime entity crate must not depend on; the taunt entries supply it.

use super::*;

impl WorldCreature {
    /// C++ `Unit::Update` health-derived `UNIT_FIELD_AURASTATE` bits.
    ///
    /// Moved to canonical ownership under #1263 F6-8D1; this entry point keeps
    /// the legacy call site's name.
    pub fn health_aura_state_like_cpp(current_health: u64, max_health: u64, alive: bool) -> u32 {
        Creature::health_aura_state_like_cpp(current_health, max_health, alive)
    }

    pub fn enter_combat(&mut self, attacker: ObjectGuid) {
        self.creature.enter_combat(attacker);
        debug!(
            "Creature {:?} entered combat with {:?}",
            self.guid(),
            attacker
        );
    }

    pub fn schedule_assistance_like_cpp(
        &mut self,
        victim: ObjectGuid,
        assistants: Vec<ObjectGuid>,
        delay_ms: u32,
    ) -> bool {
        self.creature
            .schedule_assistance_like_cpp(victim, assistants, delay_ms)
    }

    pub fn set_no_call_assistance_like_cpp(&mut self) {
        self.creature.set_no_call_assistance_like_cpp();
    }

    pub fn take_assistance_call_like_cpp(&mut self) -> Option<ObjectGuid> {
        self.creature.take_assistance_call_like_cpp()
    }

    pub fn take_due_assistance_like_cpp(&mut self) -> Vec<(ObjectGuid, Vec<ObjectGuid>)> {
        self.creature.take_due_assistance_like_cpp()
    }

    pub fn apply_taunt_aura_like_cpp(
        &mut self,
        caster: ObjectGuid,
        spell_id: u32,
        effect_mask: u32,
        duration_ms: i32,
    ) -> Option<u8> {
        self.apply_taunt_aura_with_provenance_like_cpp(
            caster,
            spell_id,
            effect_mask,
            duration_ms,
            wow_entities::AuraCastProvenanceLikeCpp::default(),
        )
    }

    /// Apply C++'s taunt Aura using the parent `Spell` object's retained cast
    /// identity. `EffectTaunt` must not allocate a second Cast GUID for the
    /// Aura base created by the same cast.
    pub fn apply_taunt_aura_with_provenance_like_cpp(
        &mut self,
        caster: ObjectGuid,
        spell_id: u32,
        effect_mask: u32,
        duration_ms: i32,
        provenance: wow_entities::AuraCastProvenanceLikeCpp,
    ) -> Option<u8> {
        self.creature.apply_taunt_aura_with_provenance_like_cpp(
            caster,
            spell_id,
            effect_mask,
            duration_ms,
            provenance,
            wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT,
        )
    }

    pub fn expire_taunt_auras_if_due_like_cpp(&mut self) -> Vec<u8> {
        self.creature.expire_taunt_auras_if_due_like_cpp()
    }

    pub fn reset_combat(&mut self) -> Vec<u8> {
        self.creature.reset_combat()
    }

    pub fn take_damage(&mut self, damage: u32) -> bool {
        self.creature.take_damage(damage)
    }

    pub fn take_damage_before_death_state_like_cpp(&mut self, damage: u32) -> bool {
        self.creature
            .take_damage_before_death_state_like_cpp(damage)
    }

    pub fn take_damage_before_death_state_at_game_time_like_cpp(
        &mut self,
        damage: u32,
        game_time_secs: i64,
    ) -> bool {
        self.creature
            .take_damage_before_death_state_at_game_time_like_cpp(damage, game_time_secs)
    }

    pub fn complete_death_state_after_kill_hooks_like_cpp(&mut self) {
        self.creature
            .complete_death_state_after_kill_hooks_like_cpp();
    }

    pub fn complete_death_state_after_kill_hooks_at_game_time_like_cpp(
        &mut self,
        game_time_secs: i64,
    ) {
        self.creature
            .complete_death_state_after_kill_hooks_at_game_time_like_cpp(game_time_secs);
    }

    pub fn try_aggro(&mut self, player_guid: ObjectGuid, player_pos: &Position) -> bool {
        self.creature.try_aggro(player_guid, player_pos)
    }

    pub fn try_aggro_with_target_combat_reach_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        player_pos: &Position,
        player_combat_reach: f32,
    ) -> bool {
        self.creature.try_aggro_with_target_combat_reach_like_cpp(
            player_guid,
            player_pos,
            player_combat_reach,
        )
    }

    pub fn creature_spell_schedule_initialized_like_cpp(&self) -> bool {
        self.creature.creature_spell_schedule_initialized_like_cpp()
    }

    pub fn mark_creature_spell_schedule_initialized_like_cpp(&mut self) {
        self.creature
            .mark_creature_spell_schedule_initialized_like_cpp();
    }

    pub(crate) fn reset_creature_spell_schedule_like_cpp(&mut self) {
        self.creature.reset_creature_spell_schedule_like_cpp();
    }

    pub fn creature_spell_engagement_epoch_like_cpp(&self) -> u64 {
        self.creature.creature_spell_engagement_epoch_like_cpp()
    }

    pub fn schedule_creature_spell_slot_after_like_cpp(&mut self, slot: usize, delay_ms: u64) {
        self.creature
            .schedule_creature_spell_slot_after_like_cpp(slot, delay_ms);
    }

    pub fn clear_creature_spell_slot_like_cpp(&mut self, slot: usize) {
        self.creature.clear_creature_spell_slot_like_cpp(slot);
    }

    pub fn first_due_creature_spell_slot_like_cpp(&self) -> Option<usize> {
        self.creature.first_due_creature_spell_slot_like_cpp()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn creature_spell_due_in_ms_for_test(&self, slot: usize) -> Option<u64> {
        self.creature
            .runtime_like_cpp()
            .creature_spell_due_at_ms_like_cpp
            .get(slot)
            .copied()
            .flatten()
            .map(|due_at| due_at.saturating_sub(self.creature.runtime_elapsed_ms_like_cpp()))
    }

    pub fn random_creature_spell_delay_like_cpp(
        &mut self,
        minimum_ms: u64,
        maximum_ms: u64,
    ) -> Option<u64> {
        self.creature
            .random_creature_spell_delay_like_cpp(minimum_ms, maximum_ms)
    }

    pub fn random_creature_spell_hit_roll_like_cpp(&mut self) -> Option<u32> {
        self.creature.random_creature_spell_hit_roll_like_cpp()
    }

    pub fn roll_damage(&mut self) -> Option<u32> {
        self.creature.roll_damage()
    }
}
