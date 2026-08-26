// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature combat: aggro, threat, melee and evade.

use super::*;

impl WorldCreature {
    /// C++ `Unit::Update` health-derived `UNIT_FIELD_AURASTATE` bits.
    ///
    /// Mirrors `Unit.cpp:469-476` `ModifyAuraState` calls for an alive unit:
    /// the WOUNDED_* and WOUND_HEALTH_* / HEALTHY_75 states are pure functions
    /// of the health percentage. AURA_STATE values are 1-based flag indices, so
    /// the wire bit is `1 << (state - 1)`. A full-HP creature yields `0x00D00000`.
    /// Shipping 0 here (the bit 0x100000 = AURA_STATE_WOUND_HEALTH_20_80 in
    /// particular) crashes the 3.4.3 client on a per-frame unit tick.
    pub fn health_aura_state_like_cpp(current_health: u64, max_health: u64, alive: bool) -> u32 {
        if !alive || max_health == 0 {
            return 0;
        }
        // C++ HealthBelowPct(p): health < max * p / 100; HealthAbovePct(p): health > max * p / 100.
        let below = |p: u64| current_health.saturating_mul(100) < max_health.saturating_mul(p);
        let above = |p: u64| current_health.saturating_mul(100) > max_health.saturating_mul(p);
        let mut state: u32 = 0;
        let mut set = |flag_index: u32, apply: bool| {
            if apply {
                state |= 1 << (flag_index - 1);
            }
        };
        set(2, below(20)); // AURA_STATE_WOUNDED_20_PERCENT
        set(6, below(25)); // AURA_STATE_WOUNDED_25_PERCENT
        set(13, below(35)); // AURA_STATE_WOUNDED_35_PERCENT
        set(21, below(20) || above(80)); // AURA_STATE_WOUND_HEALTH_20_80
        set(23, above(75)); // AURA_STATE_HEALTHY_75_PERCENT
        set(24, below(35) || above(80)); // AURA_STATE_WOUND_HEALTH_35_80
        state
    }

    pub fn enter_combat(&mut self, attacker: ObjectGuid) {
        // `enter_combat` is also used when threat selection changes the current
        // victim. C++ only resets/schedules `CombatAI::_events` for a new
        // engagement, not every victim switch.
        if self.creature.ai_state() != CreatureAiState::InCombat {
            self.reset_creature_spell_schedule_like_cpp();
        }
        self.creature.enter_ai_combat(attacker);
        self.sync_runtime_motion_master_like_cpp();
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
        if assistants.is_empty() {
            return false;
        }
        self.pending_assistance_like_cpp.push((
            victim,
            assistants,
            self.runtime_elapsed_ms_like_cpp()
                .saturating_add(u64::from(delay_ms)),
        ));
        true
    }

    pub fn set_no_call_assistance_like_cpp(&mut self) {
        self.assistance_called_like_cpp = true;
    }

    pub fn take_assistance_call_like_cpp(&mut self) -> Option<ObjectGuid> {
        if self.assistance_called_like_cpp
            || self
                .creature
                .unit()
                .subsystems()
                .control
                .charmer_or_owner_guid()
                .is_some()
        {
            return None;
        }
        let victim = self.creature.ai_ownership().combat_target?;
        self.assistance_called_like_cpp = true;
        Some(victim)
    }

    pub fn take_due_assistance_like_cpp(&mut self) -> Vec<(ObjectGuid, Vec<ObjectGuid>)> {
        let now_ms = self.runtime_elapsed_ms_like_cpp();
        let mut due = Vec::new();
        self.pending_assistance_like_cpp
            .retain(|(victim, assistants, due_at_ms)| {
                if now_ms >= *due_at_ms {
                    due.push((*victim, assistants.clone()));
                    false
                } else {
                    true
                }
            });
        due
    }

    pub fn apply_taunt_aura_like_cpp(
        &mut self,
        caster: ObjectGuid,
        spell_id: u32,
        effect_mask: u32,
        duration_ms: i32,
    ) -> Option<u8> {
        let due_at_ms = (duration_ms >= 0).then(|| {
            self.runtime_elapsed_ms_like_cpp()
                .saturating_add(duration_ms as u64)
        });
        let replaced: Vec<_> = self
            .active_taunts_like_cpp
            .iter()
            .copied()
            .filter(|active| active.caster == caster && active.spell_id == spell_id)
            .collect();
        self.active_taunts_like_cpp
            .retain(|active| active.caster != caster || active.spell_id != spell_id);
        let auras = &mut self.creature.unit_mut().subsystems_mut().auras;
        auras.remove_auras_due_to_spell_like_cpp(spell_id, caster, effect_mask);
        for active in replaced {
            auras.clear_visible(active.slot);
        }
        if !auras.add_self_cast_addon_aura_application_like_cpp(spell_id, caster, effect_mask, 0) {
            return None;
        }
        let slot = auras.visible_auras.iter().find_map(|(slot, aura)| {
            (aura.spell_id == spell_id && aura.caster_guid == caster).then_some(*slot)
        })?;
        auras.register_applied_aura_type_like_cpp(
            wow_entities::AppliedAuraRef::new(spell_id, caster, slot, effect_mask),
            wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT,
        );
        self.active_taunts_like_cpp.push(ActiveTauntLikeCpp {
            caster,
            due_at_ms,
            spell_id,
            effect_mask,
            slot,
        });
        self.refresh_active_taunt_states_like_cpp();
        Some(slot)
    }

    pub fn expire_taunt_auras_if_due_like_cpp(&mut self) -> Vec<u8> {
        let now_ms = self.runtime_elapsed_ms_like_cpp();
        if !self.active_taunts_like_cpp.iter().any(|active| {
            active
                .due_at_ms
                .is_some_and(|due_at_ms| now_ms >= due_at_ms)
        }) {
            return Vec::new();
        }
        let expired: Vec<_> = self
            .active_taunts_like_cpp
            .iter()
            .copied()
            .filter(|active| {
                active
                    .due_at_ms
                    .is_some_and(|due_at_ms| now_ms >= due_at_ms)
            })
            .collect();
        self.active_taunts_like_cpp
            .retain(|active| active.due_at_ms.is_none_or(|due_at_ms| now_ms < due_at_ms));
        for active in &expired {
            let auras = &mut self.creature.unit_mut().subsystems_mut().auras;
            auras.remove_auras_due_to_spell_like_cpp(
                active.spell_id,
                active.caster,
                active.effect_mask,
            );
            auras.clear_visible(active.slot);
        }
        self.refresh_active_taunt_states_like_cpp();
        expired.into_iter().map(|active| active.slot).collect()
    }

    fn refresh_active_taunt_states_like_cpp(&mut self) {
        let active_casters: Vec<_> = self
            .active_taunts_like_cpp
            .iter()
            .map(|active| active.caster)
            .collect();
        let combat = &mut self.creature.unit_mut().subsystems_mut().combat;
        for guid in combat.sorted_threat_guids() {
            combat.set_threat_taunt_state(guid, wow_entities::ThreatTauntState::None);
        }
        for (priority, caster) in active_casters.into_iter().enumerate() {
            combat.set_threat_taunt_state(
                caster,
                wow_entities::ThreatTauntState::Taunt(priority as u32 + 1),
            );
        }
        // C++ `ThreatManager::TauntUpdate` finishes with
        // `EvaluateSuppressed(true)`. The runtime tick owns the target aura
        // snapshots needed for `ShouldBeSuppressed`, so retain that event
        // until the next selection pass.
        combat.request_taunt_suppression_reevaluation_like_cpp();
    }

    pub fn reset_combat(&mut self) -> Vec<u8> {
        let active_taunts = std::mem::take(&mut self.active_taunts_like_cpp);
        for active in &active_taunts {
            let auras = &mut self.creature.unit_mut().subsystems_mut().auras;
            auras.remove_auras_due_to_spell_like_cpp(
                active.spell_id,
                active.caster,
                active.effect_mask,
            );
            auras.clear_visible(active.slot);
        }
        // C++ `AssistDelayEvent` is owned by the caller, not by an assistant's
        // combat state. Preserve represented pending requests across this
        // assistant's independent combat/evade reset; execution revalidates
        // `CanAssistTo` when the delay expires. A real `Unit::AttackStop`
        // resets `m_AlreadyCallAssistance` for the next engagement.
        self.assistance_called_like_cpp = false;
        self.reset_creature_spell_schedule_like_cpp();
        self.creature
            .reset_ai_combat(self.runtime_elapsed_ms_like_cpp());
        self.sync_runtime_motion_master_like_cpp();
        active_taunts
            .into_iter()
            .map(|active| active.slot)
            .collect()
    }

    pub fn take_damage(&mut self, damage: u32) -> bool {
        self.creature
            .take_ai_damage(damage, self.runtime_elapsed_ms_like_cpp())
    }

    pub fn take_damage_before_death_state_like_cpp(&mut self, damage: u32) -> bool {
        self.creature
            .apply_ai_damage_before_death_state_like_cpp(damage, self.runtime_elapsed_ms_like_cpp())
    }

    pub fn take_damage_before_death_state_at_game_time_like_cpp(
        &mut self,
        damage: u32,
        game_time_secs: i64,
    ) -> bool {
        let local_elapsed_ms = self.runtime_elapsed_ms_like_cpp();
        self.creature
            .apply_ai_damage_before_death_state_at_game_time_like_cpp(
                damage,
                local_elapsed_ms,
                game_time_secs,
            )
    }

    pub fn complete_death_state_after_kill_hooks_like_cpp(&mut self) {
        self.complete_death_state_after_kill_hooks_at_game_time_like_cpp(game_time_secs_like_cpp());
    }

    pub fn complete_death_state_after_kill_hooks_at_game_time_like_cpp(
        &mut self,
        game_time_secs: i64,
    ) {
        let local_elapsed_ms = self.runtime_elapsed_ms_like_cpp();
        self.creature
            .complete_ai_death_state_after_kill_hooks_like_cpp(local_elapsed_ms, game_time_secs);
    }

    pub fn try_aggro(&mut self, player_guid: ObjectGuid, player_pos: &Position) -> bool {
        self.creature.try_ai_aggro(player_guid, player_pos)
    }

    pub fn try_aggro_with_target_combat_reach_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        player_pos: &Position,
        player_combat_reach: f32,
    ) -> bool {
        self.creature
            .try_ai_aggro_with_target_combat_reach_like_cpp(
                player_guid,
                player_pos,
                player_combat_reach,
            )
    }

    pub(crate) fn creature_spell_schedule_initialized_like_cpp(&self) -> bool {
        self.creature_spell_schedule_initialized_like_cpp
    }

    pub(crate) fn mark_creature_spell_schedule_initialized_like_cpp(&mut self) {
        self.creature_spell_schedule_initialized_like_cpp = true;
    }

    pub(crate) fn reset_creature_spell_schedule_like_cpp(&mut self) {
        self.creature_spell_due_at_ms_like_cpp = [None; wow_entities::MAX_CREATURE_SPELLS];
        self.creature_spell_schedule_initialized_like_cpp = false;
        self.creature_spell_engagement_epoch_like_cpp = self
            .creature_spell_engagement_epoch_like_cpp
            .wrapping_add(1);
    }

    pub(crate) fn creature_spell_engagement_epoch_like_cpp(&self) -> u64 {
        self.creature_spell_engagement_epoch_like_cpp
    }

    pub(crate) fn schedule_creature_spell_slot_after_like_cpp(
        &mut self,
        slot: usize,
        delay_ms: u64,
    ) {
        let due_at_ms = self.runtime_elapsed_ms_like_cpp().saturating_add(delay_ms);
        if let Some(due_at) = self.creature_spell_due_at_ms_like_cpp.get_mut(slot) {
            *due_at = Some(due_at_ms);
        }
    }

    pub(crate) fn clear_creature_spell_slot_like_cpp(&mut self, slot: usize) {
        if let Some(due_at) = self.creature_spell_due_at_ms_like_cpp.get_mut(slot) {
            *due_at = None;
        }
    }

    pub(crate) fn first_due_creature_spell_slot_like_cpp(&self) -> Option<usize> {
        let now_ms = self.runtime_elapsed_ms_like_cpp();
        self.creature_spell_due_at_ms_like_cpp
            .iter()
            .enumerate()
            .filter_map(|(slot, due_at)| due_at.map(|due_at| (slot, due_at)))
            .filter(|(_, due_at)| now_ms >= *due_at)
            // C++ `CombatAI::UpdateAI` invokes `EventMap::ExecuteEvent` once
            // per update, so simultaneous events are consumed one at a time.
            .min_by_key(|(slot, due_at)| (*due_at, *slot))
            .map(|(slot, _)| slot)
    }

    #[cfg(test)]
    pub(crate) fn creature_spell_due_in_ms_for_test(&self, slot: usize) -> Option<u64> {
        self.creature_spell_due_at_ms_like_cpp
            .get(slot)
            .copied()
            .flatten()
            .map(|due_at| due_at.saturating_sub(self.runtime_elapsed_ms_like_cpp()))
    }

    pub(crate) fn random_creature_spell_delay_like_cpp(
        &mut self,
        minimum_ms: u64,
        maximum_ms: u64,
    ) -> Option<u64> {
        if !self.runtime_rng_authority_complete_like_cpp {
            return None;
        }
        if minimum_ms > maximum_ms {
            self.invalidate_runtime_rng_authority_like_cpp();
            return None;
        }
        if minimum_ms == maximum_ms {
            // C++ `urand(min, max)` still invokes its process-global engine
            // when both inclusive bounds are equal. Preserve that logical
            // draw in the Creature-owned represented stream.
            let _ = self.runtime_rng_like_cpp.next_u32();
            return Some(minimum_ms);
        }
        Some(self.runtime_rng_like_cpp.gen_range(minimum_ms..=maximum_ms))
    }

    pub(crate) fn random_creature_spell_hit_roll_like_cpp(&mut self) -> Option<u32> {
        self.runtime_rng_authority_complete_like_cpp
            .then(|| self.runtime_rng_like_cpp.gen_range(0..=9_999))
    }

    pub fn roll_damage(&mut self) -> Option<u32> {
        let min_dmg = self.min_dmg();
        let max_dmg = self.max_dmg();
        if min_dmg > max_dmg {
            self.invalidate_runtime_rng_authority_like_cpp();
            return None;
        }
        if min_dmg == max_dmg {
            let _ = self.runtime_rng_like_cpp.next_u32();
            return Some(min_dmg);
        }
        Some(self.runtime_rng_like_cpp.gen_range(min_dmg..=max_dmg))
    }
}
