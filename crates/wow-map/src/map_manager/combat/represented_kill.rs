//! Represented creature damage and post-hook death completion.
//! Catalog reads, logs, loot/rewards and publication remain application-owned.
//! The borrowed phases keep APP kill logging/serialization before values capture.
use super::super::WorldCreature;
use wow_core::ObjectGuid;
use wow_entities::UnitValuesUpdate;
use wow_movement::MoveSplineStopResult;

pub struct RepresentedCreatureDamage<'a> {
    creature: &'a mut WorldCreature,
}

pub struct RepresentedCreatureDamageApplied<'a> {
    creature: &'a mut WorldCreature,
    died: bool,
    threat_value: Option<f32>,
    newly_engaged: bool,
    pre_hit_health: u32,
}

pub struct CreatureDamageOutcome {
    pub values_update: UnitValuesUpdate,
    pub threat_value: Option<f32>,
    pub newly_engaged: bool,
    pub pre_hit_health: u32,
}

impl WorldCreature {
    /// Admission precedes APP's successful-hit log; it performs no mutation.
    pub fn begin_represented_damage(&mut self) -> Option<RepresentedCreatureDamage<'_>> {
        if !self.is_alive() {
            return None;
        }
        Some(RepresentedCreatureDamage { creature: self })
    }

    /// Unit::Kill10457–10763: death state follows rewards/procs; corpse flags
    /// follow death state. Creature::setDeathState2193–2294 remains unchanged.
    /// The existing wrapper samples game time here, not before APP awaits.
    pub fn finalize_represented_kill(
        &mut self,
        lootable: bool,
        can_skin: bool,
    ) -> UnitValuesUpdate {
        self.complete_death_state_after_kill_hooks_like_cpp();
        self.apply_corpse_loot_flags_after_death_state_like_cpp(lootable, can_skin);
        self.creature.unit().values_update()
    }
}

impl<'a> RepresentedCreatureDamage<'a> {
    /// Facts have already been resolved at the original APP catalog read sites.
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        self,
        damage_amount: u32,
        caster_guid: ObjectGuid,
        player_tap: Option<(ObjectGuid, &[ObjectGuid])>,
        suppress_harmful_threat: bool,
        no_initial_threat: bool,
        spell_threat_pct_mod: f32,
        caster_school_threat_mod: f32,
    ) -> RepresentedCreatureDamageApplied<'a> {
        let creature = self.creature;
        if let Some((player_guid, tap_group_guids)) = player_tap {
            creature
                .creature
                .set_tapped_by_player(player_guid, tap_group_guids);
        }
        // SpellNonMeleeDamage::preHitHealth precedes DealDamage.
        let pre_hit_health = creature.current_hp();
        let died = creature.take_damage_before_death_state_like_cpp(damage_amount);
        let newly_engaged = !died
            && damage_amount > 0
            && !suppress_harmful_threat
            && !(no_initial_threat && !creature.creature.is_in_combat())
            && creature.creature.ai_ownership().combat_target.is_none();
        let threat_value = if !died
            && damage_amount > 0
            && !suppress_harmful_threat
            && !(no_initial_threat && !creature.creature.is_in_combat())
        {
            // AtTargetAttacked then DealDamage threat, preserving factor order.
            if creature.creature.ai_ownership().combat_target.is_none() {
                creature.enter_combat(caster_guid);
            }
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .add_threat(
                    caster_guid,
                    damage_amount as f32 * spell_threat_pct_mod * caster_school_threat_mod,
                );
            creature
                .creature
                .unit()
                .subsystems()
                .combat
                .threat_value(caster_guid)
        } else {
            None
        };
        RepresentedCreatureDamageApplied {
            creature,
            died,
            threat_value,
            newly_engaged,
            pre_hit_health,
        }
    }
}

impl RepresentedCreatureDamageApplied<'_> {
    pub fn died(&self) -> bool {
        self.died
    }

    pub fn entry(&self) -> u32 {
        self.creature.entry()
    }

    /// Called after APP's kill log and before its stop serialization.
    pub fn stop_after_kill(&mut self) -> Option<MoveSplineStopResult> {
        if self.died {
            self.creature.stop_move_spline_like_cpp()
        } else {
            None
        }
    }

    /// Consuming the borrowed phase captures values only after APP serialization.
    pub fn finish(self) -> CreatureDamageOutcome {
        CreatureDamageOutcome {
            values_update: self.creature.creature.unit().values_update(),
            threat_value: self.threat_value,
            newly_engaged: self.newly_engaged,
            pre_hit_health: self.pre_hit_health,
        }
    }
}

#[cfg(test)]
mod tests;
