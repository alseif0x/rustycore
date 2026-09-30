//! Unit casting state, attacker relations and attack admission transitions.

use super::*;

impl Unit {
    pub const fn unit_state(&self) -> u32 {
        self.unit_state
    }
    pub fn add_unit_state(&mut self, flags: u32) {
        self.unit_state |= flags;
    }
    pub fn clear_unit_state(&mut self, flags: u32) {
        self.unit_state &= !flags;
    }
    pub fn has_unit_state(&self, flags: u32) -> bool {
        (self.unit_state & flags) != 0
    }
    pub fn set_current_cast_spell(
        &mut self,
        slot: CurrentSpellSlot,
        spell: CurrentSpellRef,
    ) -> Option<CurrentSpellRef> {
        if self.subsystems.spells.current_spell(slot) == Some(spell) {
            return None;
        }

        match slot {
            CurrentSpellSlot::Generic => {
                self.interrupt_spell(CurrentSpellSlot::Generic, false, true);
                if self
                    .current_spell(CurrentSpellSlot::Channeled)
                    .is_some_and(|current| !current.allow_actions_during_channel)
                {
                    self.interrupt_spell(CurrentSpellSlot::Channeled, false, true);
                }
                if self
                    .current_spell(CurrentSpellSlot::Autorepeat)
                    .is_some_and(|current| current.spell_id != AUTO_SHOT_SPELL_ID)
                {
                    self.interrupt_spell(CurrentSpellSlot::Autorepeat, true, true);
                }
                if spell.cast_time_ms > 0 {
                    self.add_unit_state(UnitState::CASTING.bits());
                }
            }
            CurrentSpellSlot::Channeled => {
                self.interrupt_spell(CurrentSpellSlot::Generic, false, true);
                self.interrupt_spell(CurrentSpellSlot::Channeled, true, true);
                if self
                    .current_spell(CurrentSpellSlot::Autorepeat)
                    .is_some_and(|current| current.spell_id != AUTO_SHOT_SPELL_ID)
                {
                    self.interrupt_spell(CurrentSpellSlot::Autorepeat, true, true);
                }
                self.add_unit_state(UnitState::CASTING.bits());
            }
            CurrentSpellSlot::Autorepeat => {
                if spell.spell_id != AUTO_SHOT_SPELL_ID {
                    self.interrupt_spell(CurrentSpellSlot::Generic, false, true);
                    self.interrupt_spell(CurrentSpellSlot::Channeled, false, true);
                }
            }
            CurrentSpellSlot::Melee => {}
        }

        self.subsystems.spells.current_spells.insert(slot, spell)
    }
    pub fn current_spell(&self, slot: CurrentSpellSlot) -> Option<CurrentSpellRef> {
        self.subsystems.spells.current_spell(slot)
    }
    pub fn interrupt_spell(
        &mut self,
        slot: CurrentSpellSlot,
        with_delayed: bool,
        with_instant: bool,
    ) -> Option<CurrentSpellRef> {
        let spell = self.current_spell(slot)?;
        if !with_delayed && spell.state == SpellState::Delayed {
            return None;
        }
        if !with_instant && spell.cast_time_ms == 0 && spell.state != SpellState::Casting {
            return None;
        }
        if !spell.interruptible {
            return None;
        }

        let removed = self.subsystems.spells.clear_current_spell(slot);
        self.sync_casting_unit_state();
        removed
    }
    pub fn finish_spell(&mut self, slot: CurrentSpellSlot) -> Option<CurrentSpellRef> {
        let removed = self.subsystems.spells.clear_current_spell(slot);
        self.sync_casting_unit_state();
        removed
    }
    pub fn interrupt_non_melee_spells(
        &mut self,
        spell_id: Option<u32>,
        with_delayed: bool,
        with_instant: bool,
    ) -> Vec<(CurrentSpellSlot, CurrentSpellRef)> {
        let mut removed = Vec::new();
        for slot in [
            CurrentSpellSlot::Generic,
            CurrentSpellSlot::Autorepeat,
            CurrentSpellSlot::Channeled,
        ] {
            let Some(spell) = self.current_spell(slot) else {
                continue;
            };
            if spell_id.is_some_and(|wanted| wanted != spell.spell_id) {
                continue;
            }
            let slot_with_delayed = with_delayed || slot == CurrentSpellSlot::Channeled;
            let slot_with_instant = with_instant || slot == CurrentSpellSlot::Channeled;
            if let Some(interrupted) =
                self.interrupt_spell(slot, slot_with_delayed, slot_with_instant)
            {
                removed.push((slot, interrupted));
            }
        }
        removed
    }
    pub fn is_non_melee_spell_cast_like_cpp(
        &self,
        with_delayed: bool,
        skip_channeled: bool,
        skip_autorepeat: bool,
        skip_instant: bool,
    ) -> bool {
        if let Some(spell) = self.current_spell(CurrentSpellSlot::Generic) {
            if spell.state != SpellState::Finished
                && (with_delayed || spell.state != SpellState::Delayed)
                && (!skip_instant || spell.cast_time_ms > 0)
            {
                return true;
            }
        }

        if !skip_channeled {
            if let Some(spell) = self.current_spell(CurrentSpellSlot::Channeled) {
                if spell.state != SpellState::Finished {
                    return true;
                }
            }
        }

        !skip_autorepeat && self.current_spell(CurrentSpellSlot::Autorepeat).is_some()
    }
    pub fn find_current_spell_by_spell_id(&self, spell_id: u32) -> Option<CurrentSpellRef> {
        self.subsystems
            .spells
            .find_current_spell_by_spell_id(spell_id)
    }
    pub(in crate::unit) fn sync_casting_unit_state(&mut self) {
        if self.current_spell(CurrentSpellSlot::Generic).is_none()
            && self.current_spell(CurrentSpellSlot::Channeled).is_none()
        {
            self.clear_unit_state(UnitState::CASTING.bits());
        }
    }
    pub const fn attacking(&self) -> Option<ObjectGuid> {
        self.subsystems.combat.attacking_guid
    }
    pub fn set_attacking(&mut self, victim: Option<ObjectGuid>) {
        self.subsystems.combat.set_attacking(victim);
    }
    pub fn add_attacker_like_cpp(&mut self, attacker: ObjectGuid) -> bool {
        self.subsystems.combat.add_attacker(attacker)
    }
    pub fn remove_attacker_like_cpp(&mut self, attacker: ObjectGuid) -> bool {
        self.subsystems.combat.remove_attacker(attacker)
    }
    pub fn has_attacker_like_cpp(&self, attacker: ObjectGuid) -> bool {
        self.subsystems.combat.attackers.contains(&attacker)
    }
    pub const fn last_damaged_target_like_cpp(&self) -> Option<ObjectGuid> {
        self.subsystems.combat.last_damaged_target_guid
    }
    pub fn set_last_damaged_target_like_cpp(&mut self, target: Option<ObjectGuid>) {
        self.subsystems
            .combat
            .set_last_damaged_target_like_cpp(target);
    }
    /// C++ `Unit::AddExtraAttacks`.
    ///
    /// The target bucket is `_lastDamagedTargetGuid` first, then current
    /// selection (`UNIT_FIELD_TARGET`), otherwise the call is a no-op.
    pub fn add_extra_attacks_like_cpp(&mut self, count: u32) -> Option<ObjectGuid> {
        let target = self.last_damaged_target_like_cpp().unwrap_or_else(|| {
            let selected = self.data().target;
            if selected.is_empty() {
                ObjectGuid::EMPTY
            } else {
                selected
            }
        });
        if target.is_empty() {
            return None;
        }
        self.subsystems
            .combat
            .add_extra_attacks_for_like_cpp(target, count);
        Some(target)
    }
    pub fn extra_attacks_for_like_cpp(&self, target: ObjectGuid) -> u32 {
        self.subsystems.combat.extra_attacks_for_like_cpp(target)
    }
    pub fn attack_like_cpp(
        &mut self,
        victim_guid: ObjectGuid,
        victim_alive: bool,
        victim_in_world: bool,
        melee_attack: bool,
    ) -> UnitAttackStartOutcome {
        self.attack_with_context_like_cpp(
            victim_guid,
            victim_alive,
            victim_in_world,
            melee_attack,
            UnitAttackContextLikeCpp::default(),
        )
    }
    pub fn attack_with_context_like_cpp(
        &mut self,
        victim_guid: ObjectGuid,
        victim_alive: bool,
        victim_in_world: bool,
        melee_attack: bool,
        context: UnitAttackContextLikeCpp,
    ) -> UnitAttackStartOutcome {
        let self_guid = self.world().object().guid();
        if victim_guid.is_empty() || victim_guid == self_guid {
            return UnitAttackStartOutcome::InvalidSelfTarget;
        }
        if !self.is_alive() {
            return UnitAttackStartOutcome::InvalidDeadAttacker;
        }
        if !victim_in_world {
            return UnitAttackStartOutcome::InvalidVictimNotInWorld;
        }
        if !victim_alive {
            return UnitAttackStartOutcome::InvalidDeadVictim;
        }
        if context.attacker_is_mounted_player {
            return UnitAttackStartOutcome::InvalidMountedAttacker;
        }
        if context.attacker_is_evading_creature {
            return UnitAttackStartOutcome::InvalidAttackerEvading;
        }
        if context.victim_is_game_master_player {
            return UnitAttackStartOutcome::InvalidVictimGameMaster;
        }
        if context.victim_is_evading_creature {
            return UnitAttackStartOutcome::InvalidVictimEvading;
        }
        if !Self::is_valid_attack_target_represented_like_cpp(&context) {
            return UnitAttackStartOutcome::InvalidAttackTarget;
        }

        if self
            .subsystems
            .auras
            .has_aura_type_like_cpp(SPELL_AURA_MOD_UNATTACKABLE_LIKE_CPP)
        {
            self.subsystems
                .auras
                .remove_auras_by_type_like_cpp(SPELL_AURA_MOD_UNATTACKABLE_LIKE_CPP);
        }

        if self.attacking() == Some(victim_guid) {
            if melee_attack {
                if !self.has_unit_state(UnitState::MELEE_ATTACKING.bits()) {
                    self.add_unit_state(UnitState::MELEE_ATTACKING.bits());
                    return UnitAttackStartOutcome::MeleeStartedSameTarget;
                }
            } else if self.has_unit_state(UnitState::MELEE_ATTACKING.bits()) {
                self.clear_unit_state(UnitState::MELEE_ATTACKING.bits());
                return UnitAttackStartOutcome::MeleeStoppedSameTarget;
            }
            return UnitAttackStartOutcome::NoChangeSameTarget;
        }

        let previous = self.attacking();
        if previous.is_some() {
            self.interrupt_spell(CurrentSpellSlot::Melee, true, true);
            if !melee_attack {
                self.clear_unit_state(UnitState::MELEE_ATTACKING.bits());
            }
        }

        self.set_attacking(Some(victim_guid));
        self.set_target(victim_guid);
        if melee_attack {
            self.add_unit_state(UnitState::MELEE_ATTACKING.bits());
        }
        self.apply_creature_attack_ai_side_effects_like_cpp(victim_guid);
        self.delay_offhand_attack_like_cpp();
        self.apply_player_controlled_owner_attacked_like_cpp(
            victim_guid,
            &context.controlled_creatures_with_ai,
        );

        UnitAttackStartOutcome::NewTarget { previous }
    }
}
